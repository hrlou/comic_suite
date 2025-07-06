//! LRU cache for decoded images and async image loading.

use crate::prelude::*;

/// Represents a decoded page image (static or animated).
#[derive(Clone)]
pub enum PageImage {
    Static(DynamicImage),
    AnimatedGif {
        frames: Vec<eframe::egui::ColorImage>,
        delays: Vec<u16>,
        start_time: Instant,
    },
}

impl PageImage {
    /// Returns the dimensions of the image.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            PageImage::Static(img) => img.dimensions(),
            PageImage::AnimatedGif { frames, .. } => {
                if let Some(frame) = frames.first() {
                    (frame.size[0] as u32, frame.size[1] as u32)
                } else {
                    (0, 0)
                }
            }
        }
    }
}

/// A loaded page, ready for display.
#[derive(Clone)]
pub struct LoadedPage {
    pub image: PageImage,
    pub index: usize,
    pub filename: String,
}

/// Shared LRU cache for images.
pub type SharedImageCache = Arc<Mutex<LruCache<usize, LoadedPage>>>;

/// Create a new shared LRU cache for images.
pub fn new_image_cache(size: usize) -> SharedImageCache {
    Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(size).unwrap())))
}

/// Asynchronously load an image from the archive and insert into the cache.
pub fn load_image_async(
    page: usize,
    filenames: Arc<Vec<String>>,
    archive: Arc<Mutex<ImageArchive>>,
    image_lru: SharedImageCache,
    loading_pages: Arc<Mutex<std::collections::HashSet<usize>>>,
) -> Result<(), AppError> {
    {
        let mut loading = loading_pages.lock().unwrap();
        if loading.contains(&page) {
            debug!("Image page {} is already being loaded (skipping)", page);
            return Ok(());
        }
        loading.insert(page);
    }

    if image_lru.lock().unwrap().get(&page).is_some() {
        debug!("Image page {} is already in LRU cache (hit)", page);
        loading_pages.lock().unwrap().remove(&page);
        return Ok(());
    }

    let filenames = Arc::clone(&filenames);
    let archive = archive.clone();
    let image_lru = image_lru.clone();
    let loading_pages = loading_pages.clone();

    std::thread::spawn(move || {
        let filename = &filenames[page];
        let mut archive = archive.lock().unwrap();
        let buf = archive.read_image_by_index(page).unwrap();

        let loaded_page = if filename.to_lowercase().ends_with(".gif") {
            // Decode GIF frames
            let cursor = Cursor::new(&buf);
            let decoder = GifDecoder::new(cursor).unwrap();
            let frames = decoder
                .into_frames()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            // Convert frames to egui::ColorImage and collect delays
            let mut egui_frames = Vec::with_capacity(frames.len());
            let mut delays = Vec::with_capacity(frames.len());

            for frame in frames {
                let delay = frame.delay().numer_denom_ms().0; // delay numerator (ms)
                delays.push(delay as u16);

                // Convert the frame to RGBA8 for egui
                let buffer = frame.buffer();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [buffer.width() as usize, buffer.height() as usize],
                    buffer.as_raw(),
                );
                egui_frames.push(color_image);
            }

            PageImage::AnimatedGif {
                frames: egui_frames,
                delays,
                start_time: Instant::now(),
            }
        } else {
            // Static image fallback
            let img = image::load_from_memory(&buf).unwrap();
            PageImage::Static(img)
        };

        let loaded_page = LoadedPage {
            image: loaded_page,
            index: page,
            filename: filename.clone(),
        };

        image_lru.lock().unwrap().put(page, loaded_page);
        loading_pages.lock().unwrap().remove(&page);
        debug!("Loaded image page {} into LRU cache", page);
    });

    Ok(())
}
