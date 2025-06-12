use crate::archive::ImageArchive;
use crate::error::AppError;
use image::{DynamicImage, GenericImageView};
use eframe::egui;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;
use log::{info, warn};
use std::time::Instant;

#[derive(Clone)]
pub enum PageImage {
    Static(DynamicImage),
    AnimatedGif {
        frames: Vec<egui::ColorImage>,
        delays: Vec<u16>, // in hundredths of a second
        start_time: Instant,
    },
}

impl PageImage {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            PageImage::Static(img) => img.dimensions(),
            PageImage::AnimatedGif { frames, .. } if !frames.is_empty() => {
                (frames[0].size[0] as u32, frames[0].size[1] as u32)
            }
            _ => (1, 1),
        }
    }
}

#[derive(Clone)]
pub struct LoadedPage {
    pub index: usize,
    pub filename: String,
    pub image: PageImage,
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
    archive: Arc<Mutex<ImageArchive>>,
    image_lru: SharedImageCache,
    loading_pages: Arc<Mutex<std::collections::HashSet<usize>>>,
) -> Result<(), AppError> {
    {
        let mut loading = loading_pages.lock().unwrap();
        if loading.contains(&page) {
            log::debug!("Image page {} is already being loaded (skipping)", page);
            return Ok(());
        }
        loading.insert(page);
    }

    if image_lru.lock().unwrap().get(&page).is_some() {
        log::debug!("Image page {} is already in LRU cache (hit)", page);
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
        let buf = match archive.read_image(filename) {
            Ok(data) => data,
            Err(e) => {
                log::warn!("Failed to read image: {e}");
                loading_pages.lock().unwrap().remove(&page);
                return;
            }
        };

        let lower = filenames_clone[page].to_lowercase();
        let loaded_page = if lower.ends_with(".gif") {
            info!("Decoding GIF for page {}", page);
            match gif::DecodeOptions::new().read_info(&*buf) {
                Ok(mut reader) => {
                    let global_palette = reader.global_palette().map(|p| p.to_vec());
                    let width = reader.width() as usize;
                    let height = reader.height() as usize;

                    // Get GIF background color from palette (never transparent)
                    let bg_color = if let Some(bg_idx) = reader.bg_color() {
                        if let Some(ref pal) = global_palette {
                            let i = bg_idx as usize * 3;
                            if i + 2 < pal.len() {
                                [pal[i], pal[i + 1], pal[i + 2], 255]
                            } else {
                                [0, 0, 0, 255]
                            }
                        } else {
                            [0, 0, 0, 255]
                        }
                    } else {
                        [0, 0, 0, 255]
                    };

                    // Initialize canvas to background color
                    let mut canvas = vec![0u8; width * height * 4];
                    for px in canvas.chunks_exact_mut(4) {
                        px.copy_from_slice(&bg_color);
                    }
                    let mut frames = Vec::new();
                    let mut delays = Vec::new();

                    use gif::DisposalMethod;

                    loop {
                        match reader.read_next_frame() {
                            Ok(Some(frame)) => {
                                let pal = frame.palette.as_ref().map(|p| p.to_vec()).or_else(|| global_palette.clone());
                                if let Some(pal) = pal {
                                    let transparent = frame.transparent;
                                    let left = frame.left as usize;
                                    let top = frame.top as usize;
                                    let w = frame.width as usize;
                                    let h = frame.height as usize;

                                    // Save previous canvas if needed for "Previous"
                                    let prev_canvas = if frame.dispose == DisposalMethod::Previous {
                                        Some(canvas.clone())
                                    } else {
                                        None
                                    };

                                    // Draw frame patch onto canvas
                                    for y in 0..h {
                                        for x in 0..w {
                                            let frame_idx = y * w + x;
                                            let canvas_x = left + x;
                                            let canvas_y = top + y;
                                            if canvas_x >= width || canvas_y >= height { continue; }
                                            let canvas_idx = (canvas_y * width + canvas_x) * 4;
                                            let idx = frame.buffer[frame_idx];
                                            let i = idx as usize * 3;
                                            if i + 2 < pal.len() {
                                                let r = pal[i];
                                                let g = pal[i + 1];
                                                let b = pal[i + 2];
                                                let a = if let Some(t) = transparent {
                                                    if idx == t { 0 } else { 255 }
                                                } else { 255 };
                                                // Only update pixel if not transparent
                                                if a > 0 {
                                                    canvas[canvas_idx..canvas_idx+4].copy_from_slice(&[r, g, b, a]);
                                                }
                                            }
                                        }
                                    }

                                    // Save a copy of the canvas as a frame
                                    frames.push(egui::ColorImage::from_rgba_unmultiplied([width, height], &canvas));
                                    delays.push(frame.delay);

                                    // Handle disposal method
                                    match frame.dispose {
                                        DisposalMethod::Background => {
                                            // Clear the patch area to bg_color (never transparent)
                                            for y in 0..h {
                                                for x in 0..w {
                                                    let canvas_x = left + x;
                                                    let canvas_y = top + y;
                                                    if canvas_x >= width || canvas_y >= height { continue; }
                                                    let canvas_idx = (canvas_y * width + canvas_x) * 4;
                                                    canvas[canvas_idx..canvas_idx+4].copy_from_slice(&bg_color);
                                                }
                                            }
                                        }
                                        DisposalMethod::Previous => {
                                            if let Some(prev) = prev_canvas {
                                                canvas = prev;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                log::warn!("GIF decode error: {e}");
                                break;
                            }
                        }
                    }
                    if !frames.is_empty() {
                        LoadedPage {
                            index: page,
                            filename: filenames_clone[page].clone(),
                            image: PageImage::AnimatedGif {
                                frames,
                                delays,
                                start_time: Instant::now(),
                            },
                        }
                    } else {
                        warn!("GIF decode failed, falling back to static image for page {}", page);
                        let img = image::load_from_memory(&buf).unwrap();
                        LoadedPage {
                            index: page,
                            filename: filenames_clone[page].clone(),
                            image: PageImage::Static(img),
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to decode GIF: {e}");
                    let img = image::load_from_memory(&buf).unwrap();
                    LoadedPage {
                        index: page,
                        filename: filenames_clone[page].clone(),
                        image: PageImage::Static(img),
                    }
                }
            }
        } else {
            let img = image::load_from_memory(&buf).unwrap();
            LoadedPage {
                index: page,
                filename: filenames_clone[page].clone(),
                image: PageImage::Static(img),
            }
        };
        // Insert into LRU cache and log
        let mut lru = image_lru.lock().unwrap();
        let old = lru.put(page, loaded_page);
        if old.is_some() {
            log::debug!("Evicted image page {} from LRU cache", page);
        }
        log::debug!("Loaded image page {} into LRU cache", page);
        loading_pages.lock().unwrap().remove(&page);
    });

    Ok(())
}