use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, channel, Receiver},
    thread,
    time::Duration,
};

use anyhow::Result;
use image::{imageops::FilterType, DynamicImage};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use windows_service::{
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
use zip::ZipArchive;

const WATCH_PATH: &str = "C:\\Users\\hrl\\Gaming";

fn save_thumbnail(original: &Path, img: &DynamicImage) -> Result<PathBuf> {
    eprintln!("Resizing image for thumbnail...");
    let thumb = img.resize(512, 512, FilterType::Lanczos3);
    let thumb_path = original.with_extension("jpg");

    eprintln!("Creating thumbnail file: {}", thumb_path.display());
    let file = File::create(&thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 85);

    encoder.encode_image(&thumb)?;
    eprintln!("Thumbnail encoded successfully.");

    // Check file size
    let metadata = std::fs::metadata(&thumb_path)?;
    eprintln!("Thumbnail file size: {} bytes", metadata.len());

    Ok(thumb_path)
}

fn generate_thumb_cbz(path: &Path) -> Result<PathBuf> {
    eprintln!("Opening archive: {}", path.display());
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_ascii_lowercase();
        if name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".jpeg") {
            eprintln!("Found image in archive: {}", file.name());
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            eprintln!("Read image buffer size: {} bytes", buf.len());

            let img = image::load_from_memory(&buf)?;
            eprintln!("Image loaded successfully from memory.");

            return save_thumbnail(path, &img);
        }
    }
    anyhow::bail!("No image found in {:?}", path);
}

fn thumbnail_worker(path: PathBuf) {
    eprintln!("Generating thumbnail for {:?}", path);

    // Retry delay to let file write/rename complete
    const MAX_RETRIES: u8 = 5;
    const RETRY_DELAY_MS: u64 = 500;

    for attempt in 1..=MAX_RETRIES {
        match generate_thumb_cbz(&path) {
            Ok(out) => {
                eprintln!("Thumbnail saved: {}", out.display());
                return;
            }
            Err(e) => {
                eprintln!(
                    "Attempt {}: Failed to generate thumbnail for {:?}: {}",
                    attempt, path, e
                );
                if attempt == MAX_RETRIES {
                    eprintln!("Giving up on {:?}", path);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
}

fn watch_folder(path: &Path, stop_rx: Receiver<()>) -> Result<()> {
    eprintln!("Starting folder watcher on {}", path.display());

    let (tx, rx) = channel();

    let watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.send(res) {
                eprintln!("Failed to send watcher event: {}", e);
            }
        },
        Config::default(),
    );

    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create watcher: {:?}", e);
            return Err(e.into());
        }
    };

    if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
        eprintln!("Failed to watch folder '{}': {:?}", path.display(), e);
        return Err(e.into());
    }

    eprintln!("Watcher waiting for events or stop signal...");

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                eprintln!("Filesystem event: {:?}", event);
                // Process paths in event
                for path in event.paths.iter() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("cbz") {
                            eprintln!("Detected CBZ file: {:?}", path);
                            let path = path.to_owned();
                            std::thread::spawn(move || {
                                thumbnail_worker(path);
                            });
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // timeout, continue checking for stop signal
            }
            Err(e) => {
                eprintln!("Channel receive error: {:?}", e);
                break;
            }
        }

        // Check stop signal
        if let Ok(_) = stop_rx.try_recv() {
            eprintln!("Stop signal received, exiting watcher");
            break;
        }
    }

    Ok(())
}

fn run_service() -> windows_service::Result<()> {
    eprintln!("Service starting...");
    let watch_path = PathBuf::from(WATCH_PATH);
    if !watch_path.exists() {
        eprintln!("Watch path does not exist: {}", watch_path.display());
        return Ok(());
    }
    eprintln!("Watch path exists: {}", watch_path.display());

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();

    eprintln!("Registering service control handler...");
    let status_handle = service_control_handler::register("comic_thumbgen", {
        let stop_tx = stop_tx.clone();
        move |control_event| match control_event {
            ServiceControl::Stop => {
                eprintln!("Service stop requested");
                let _ = stop_tx.send(()); // tell watcher to stop
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    eprintln!("Setting service status to Running");
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    eprintln!("Spawning watcher thread");
    let watcher_thread = thread::spawn(move || {
        if let Err(e) = watch_folder(&watch_path, stop_rx) {
            eprintln!("Watcher thread error: {:?}", e);
        }
        let _ = done_tx.send(()); // signal main thread that watcher exited
    });

    eprintln!("Waiting for watcher thread to finish");
    match done_rx.recv() {
        Ok(_) => eprintln!("Watcher thread exited"),
        Err(e) => eprintln!("Error receiving done signal: {:?}", e),
    }

    eprintln!("Setting service status to Stopped");
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    eprintln!("Joining watcher thread");
    watcher_thread.join().unwrap();

    eprintln!("Service stopped cleanly");
    Ok(())
}

fn run_watcher_only() -> anyhow::Result<()> {
    eprintln!("Running watcher-only mode (debug)...");
    let watch_path = PathBuf::from(WATCH_PATH);
    if !watch_path.exists() {
        eprintln!("Watch path does not exist: {}", watch_path.display());
        return Ok(());
    }

    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // No service control handler registration here!

    watch_folder(&watch_path, stop_rx)?;

    Ok(())
}

// Full service logic but without calling service_control_handler or dispatcher
fn run_service_no_service_handler() -> Result<(), anyhow::Error> {
    eprintln!("Starting service logic without service handler...");
    let watch_path = PathBuf::from(WATCH_PATH);

    if !watch_path.exists() {
        eprintln!("Watch path does not exist: {}", watch_path.display());
        return Ok(());
    }

    // Use a stop channel you can trigger on Ctrl+C or never
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // Spawn watcher thread
    let watcher_thread = std::thread::spawn(move || {
        if let Err(e) = watch_folder(&watch_path, stop_rx) {
            eprintln!("Watcher thread error: {:?}", e);
        }
    });

    // Wait for Ctrl+C or indefinitely for testing
    eprintln!("Press Ctrl+C to stop...");
    ctrlc::set_handler(move || {
        let _ = stop_tx.send(());
    })
    .expect("Error setting Ctrl+C handler");

    watcher_thread.join().unwrap();
    Ok(())
}

extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    if let Err(e) = run_service() {
        eprintln!("Service failed: {:?}", e);
    }
}

fn main() -> windows_service::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--debug".to_string()) {
        eprintln!("Running in debug mode (no service)...");
        // Run your full service logic, but no service registration
        if let Err(e) = run_service_no_service_handler() {
            eprintln!("Error in debug mode: {:?}", e);
        }
        Ok(())
    } else if args.contains(&"--wonly".to_string()) {
        eprintln!("Running watcher-only mode...");
        // Run only watcher without service logic
        if let Err(e) = run_watcher_only() {
            eprintln!("Watcher error: {:?}", e);
        }
        Ok(())
    } else {
        // Normal service mode - connect to service dispatcher
        service_dispatcher::start("comic_thumbgen", service_main)
    }
}
