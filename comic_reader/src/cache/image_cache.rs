//! LRU cache for decoded images and async image loading.

use crate::prelude::*;
use std::io::Cursor;

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
            PageImage::AnimatedGif { frames, .. }
            | PageImage::AnimatedWebP { frames, .. } => {
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
fn try_decode_animated_webp(buf: &[u8], ctx: &egui::Context) -> Option<(Vec<egui::TextureHandle>, Vec<u16>)> {
    let mut decoder = WebpAnimDecoder::new(buf).ok()?;
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    for frame in decoder {
        use std::hash;

        let delay = frame.timestamp() as u16; // ms
        let (width, height) = frame.dimensions();
        delays.push(delay);
        let img = image::RgbaImage::from_raw(
            width, height,
            frame.data().to_vec(),
        )?;
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
pub fn load_image_async(
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
    let ctx = ctx.clone();

    std::thread::spawn(move || {
        let filename = &filenames[page];
        let mut archive = archive.lock().unwrap();
        let buf = archive.read_image_by_index(page).unwrap();

        let loaded_page = if filename.to_lowercase().ends_with(".gif") {
            if let Some((frames, delays)) = decode_gif(&buf, &ctx) {
                PageImage::AnimatedGif {
                    frames,
                    delays,
                    start_time: Instant::now(),
                }
            } else {
                let img = image::load_from_memory(&buf).unwrap();
                PageImage::Static(img)
            }
        } else if filename.to_lowercase().ends_with(".webp") {
            #[cfg(feature = "webp_animation")]
            {
                if let Some((frames, delays)) = try_decode_animated_webp(&buf, &ctx) {
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
            filename: filename.clone(),
        };

        image_lru.lock().unwrap().put(page, loaded_page);
        loading_pages.lock().unwrap().remove(&page);
        debug!("Loaded image page {} into LRU cache", page);
    });

    Ok(())
}
