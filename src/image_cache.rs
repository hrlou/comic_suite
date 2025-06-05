use image::DynamicImage;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

#[derive(Clone)]
pub struct LoadedPage {
    pub index: usize,
    pub filename: String,
    pub image: DynamicImage,
}

pub type SharedImageCache = Arc<Mutex<LruCache<usize, LoadedPage>>>;

/// Create a new shared LRU cache for images
pub fn new_image_cache(size: usize) -> SharedImageCache {
    Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(size).unwrap())))
}

/// Asynchronously load an image from the archive and insert into the cache
pub fn load_image_async(
    page: usize,
    filenames: Vec<String>,
    zip_path: std::path::PathBuf,
    image_lru: SharedImageCache,
    loading_pages: Arc<Mutex<std::collections::HashSet<usize>>>,
) {
    {
        let mut loading = loading_pages.lock().unwrap();
        if loading.contains(&page) {
            return;
        }
        loading.insert(page);
    }

    if image_lru.lock().unwrap().get(&page).is_some() {
        loading_pages.lock().unwrap().remove(&page);
        return;
    }

    std::thread::spawn(move || {
        let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
        let mut file = archive.by_name(&filenames[page]).unwrap();
        let size = file.size();

        let mut buf = Vec::with_capacity(size as usize);
        let mut tmp = [0u8; 8192];

        while let Ok(n) = file.read(&mut tmp) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }

        let img = image::load_from_memory(&buf).unwrap();
        let loaded_page = LoadedPage {
            index: page,
            filename: filenames[page].clone(),
            image: img,
        };
        image_lru.lock().unwrap().put(page, loaded_page);
        loading_pages.lock().unwrap().remove(&page);
    });
}