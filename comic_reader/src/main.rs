// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cache;
mod config;
mod error;
mod macros;
mod prelude;
mod ui;
mod archive_view;

use crate::prelude::*;

fn check_bin(bin: &str, msg: &str) -> bool {
    log::info!("Checking for '{}' in PATH...", bin);
    if which::which(bin).is_err() {
        rfd::MessageDialog::new()
            .set_title(&format!("Missing {}", bin))
            .set_description(&format!(
                "The '{}' executable was not found in your PATH.\n{}",
                bin, msg
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();
        log::warn!("'{}' not found in PATH. {}", bin, msg);
        return false;
    } else {
        log::info!("'{}' found in PATH.", bin);
    }
    true
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    #[cfg(feature = "rar")]
    {
        check_bin("unrar", "RAR archives will not open.");
        check_bin("rar", "RAR archives will save.");
    }
    #[cfg(feature = "7z")]
    {
        #[cfg(target_os = "windows")]
        {
            let path = PathBuf::from("C:\\Program Files\\7-Zip");
            if path.exists() {
                log::info!("Found 7-Zip installation at {}", path.display());
                let path_var = std::env::var("PATH").unwrap_or_default();
                let path_var = format!("{};{}", path_var, path.to_string_lossy());
                log::info!("Setting PATH to: {}", path_var);
                unsafe {
                    std::env::set_var("PATH", path_var);
                }
            } else {
                check_bin("7z", "7z archives will not open.");
            }
            // check_bin("7z", "7z archives will not open.");
        }
        #[cfg(not(target_os = "windows"))]
        {
            check_bin("7z", "7z archives will not open.");
        }
    }

    let path = std::env::args().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        NAME,
        native_options,
        Box::new(move |cc| {
            // Pass CreationContext to CBZViewerApp::new
            Ok(Box::new(
                CBZViewerApp::new(cc, path).expect("Failed to load comic"),
            ))
        }),
    );
    Ok(())
}
