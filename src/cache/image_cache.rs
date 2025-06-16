//! LRU cache for decoded images and async image loading.

use crate::archive::ImageArchive;
use crate::error::AppError;
use image::{DynamicImage, GenericImageView};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use log::{debug, warn};
use eframe::egui::ColorImage;
use std::time::Instant;

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
    filenames: Vec<String>,
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

    let filenames_clone = filenames.clone();
    let archive = archive.clone();
    let image_lru = image_lru.clone();
    let loading_pages = loading_pages.clone();

    std::thread::spawn(move || {
        let filename = &filenames_clone[page];
        let mut archive = archive.lock().unwrap();
        let buf = archive.read_image(filename).unwrap(); // You must implement this!
        let img = image::load_from_memory(&buf).unwrap();
        let loaded_page = LoadedPage {
            image: PageImage::Static(img),
            index: page,
            filename: filename.clone(),
        };
        image_lru.lock().unwrap().put(page, loaded_page);
        loading_pages.lock().unwrap().remove(&page);
        debug!("Loaded image page {} into LRU cache", page);
    });

    Ok(())
}