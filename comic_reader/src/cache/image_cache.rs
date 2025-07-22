//! LRU cache for decoded images and async image loading.

use crate::prelude::*;
use std::io::Cursor;

use futures::executor::block_on;
#[cfg(feature = "webp_animation")]
use webp_animation::Decoder as WebpAnimDecoder;

/// Represents a decoded page image (static or animated).
#[derive(Clone)]
pub enum PageImage {
    Static(DynamicImage),
    AnimatedGif {
        frames: Vec<egui::TextureHandle>,
        delays: Vec<u16>,
        start_time: Instant,
    },
    AnimatedWebP {
        frames: Vec<egui::TextureHandle>,
        delays: Vec<u16>,
        start_time: Instant,
    },
}

impl PageImage {
    /// Returns the dimensions of the image.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            PageImage::Static(img) => img.dimensions(),
            PageImage::AnimatedGif { frames, .. } | PageImage::AnimatedWebP { frames, .. } => {
                if let Some(frame) = frames.first() {
                    (frame.size()[0] as u32, frame.size()[1] as u32)
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

// Macro to extract frames and delays and upload as egui textures
macro_rules! extract_animation_frames {
    ($frames:expr, $delays:expr, $ctx:expr) => {{
        let mut textures = Vec::with_capacity($frames.len());
        for (i, color_image) in $frames.into_iter().enumerate() {
            let handle = $ctx.load_texture(
                format!("anim_frame_{}", i),
                color_image,
                egui::TextureOptions::default(),
            );
            textures.push(handle);
        }
        (textures, $delays)
    }};
}

#[cfg(feature = "webp_animation")]
fn try_decode_animated_webp(
    buf: &[u8],
    ctx: &egui::Context,
) -> Option<(Vec<egui::TextureHandle>, Vec<u16>)> {
    let decoder = WebpAnimDecoder::new(buf).ok()?;
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    let mut prev_timestamp = 0u32;

    for frame in decoder {
        let timestamp = frame.timestamp();
        let mut delay = timestamp.saturating_sub(prev_timestamp as i32) as u16;
        if delay < 20 {
            delay = 20;
        } // Clamp to at least 20ms (50 FPS max)
        delays.push(delay);
        prev_timestamp = timestamp as u32;

        let (width, height) = frame.dimensions();
        let img = image::RgbaImage::from_raw(width, height, frame.data().to_vec())?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [img.width() as usize, img.height() as usize],
            img.as_raw(),
        );
        frames.push(color_image);
    }
    if frames.len() > 1 {
        let (textures, delays) = extract_animation_frames!(frames, delays, ctx);
        Some((textures, delays))
    } else {
        None
    }
}
fn decode_gif(buf: &[u8], ctx: &egui::Context) -> Option<(Vec<egui::TextureHandle>, Vec<u16>)> {
    let cursor = Cursor::new(buf);
    let decoder = GifDecoder::new(cursor).ok()?;
    let frames = decoder.into_frames().collect::<Result<Vec<_>, _>>().ok()?;

    let mut color_frames = Vec::with_capacity(frames.len());
    let mut delays = Vec::with_capacity(frames.len());

    for frame in frames {
        let delay = frame.delay().numer_denom_ms().0; // delay numerator (ms)
        delays.push(delay as u16);
        let buffer = frame.buffer();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [buffer.width() as usize, buffer.height() as usize],
            buffer.as_raw(),
        );
        color_frames.push(color_image);
    }
    if color_frames.len() > 1 {
        let (textures, delays) = extract_animation_frames!(color_frames, delays, ctx);
        Some((textures, delays))
    } else {
        None
    }
}

/// Asynchronously load an image from the archive and insert into the cache.
pub async fn load_image_async(
    page: usize,
    filenames: Arc<Vec<String>>,
    archive: Arc<Mutex<ImageArchive>>,
    image_lru: SharedImageCache,
    loading_pages: Arc<Mutex<std::collections::HashSet<usize>>>,
    ctx: egui::Context,
) -> Result<(), AppError> {
    {
        let mut loading = loading_pages.lock().unwrap();
        if loading.contains(&page) {
            return Ok(());
        }
        loading.insert(page);
    }

    if image_lru.lock().unwrap().get(&page).is_some() {
        loading_pages.lock().unwrap().remove(&page);
        return Ok(());
    }

    let filename = filenames[page].clone();

    // Read the image buffer in a blocking task to avoid holding the lock across .await
    let archive_clone = archive.clone();
    let buf: Vec<u8> = match tokio::task::spawn_blocking(move || {
        let mut archive = archive_clone.lock().unwrap();
        // Use block_on to run the async function synchronously in the blocking thread
        // This must return a Vec<u8>
        block_on(archive.read_image_by_index(page))
    })
    .await
    {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            loading_pages.lock().unwrap().remove(&page);
            debug!("Failed to read image: {:?}", e);
            return Ok(());
        }
        Err(e) => {
            loading_pages.lock().unwrap().remove(&page);
            debug!("Failed to join blocking task: {:?}", e);
            return Ok(());
        }
    };

    let filename_clone = filename.clone();
    let ctx_clone = ctx.clone();
    let image_lru_clone = image_lru.clone();
    let loading_pages_clone = loading_pages.clone();

    tokio::task::spawn_blocking(move || {
        let loaded_page = if filename_clone.to_lowercase().ends_with(".gif") {
            if let Some((frames, delays)) = decode_gif(&buf, &ctx_clone) {
                PageImage::AnimatedGif {
                    frames,
                    delays,
                    start_time: Instant::now(),
                }
            } else {
                let img = image::load_from_memory(&buf).unwrap();
                PageImage::Static(img)
            }
        } else if filename_clone.to_lowercase().ends_with(".webp") {
            #[cfg(feature = "webp_animation")]
            {
                if let Some((frames, delays)) = try_decode_animated_webp(&buf, &ctx_clone) {
                    PageImage::AnimatedWebP {
                        frames,
                        delays,
                        start_time: Instant::now(),
                    }
                } else {
                    let img = image::load_from_memory(&buf).unwrap();
                    PageImage::Static(img)
                }
            }
            #[cfg(not(feature = "webp_animation"))]
            {
                let img = image::load_from_memory(&buf).unwrap();
                PageImage::Static(img)
            }
        } else {
            let img = image::load_from_memory(&buf).unwrap();
            PageImage::Static(img)
        };

        let loaded_page = LoadedPage {
            image: loaded_page,
            index: page,
            filename: filename_clone,
        };

        image_lru_clone.lock().unwrap().put(page, loaded_page);
        loading_pages_clone.lock().unwrap().remove(&page);
        debug!("Loaded image page {} into LRU cache", page);
    })
    .await
    .unwrap();

    Ok(())
}
