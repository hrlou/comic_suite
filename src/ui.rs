use eframe::egui::{self, pos2, Ui, Vec2, Rect, Image, Spinner, Color32};
use image::GenericImageView;
use crate::image_cache::{LoadedPage, PageImage};
use log::{info, debug, warn};

/// Draw a centered spinner in the given area
pub fn draw_spinner(ui: &mut Ui, area: Rect) {
    let spinner_size = 48.0;
    let spinner_rect = Rect::from_center_size(area.center(), Vec2::splat(spinner_size));
    ui.allocate_ui_at_rect(spinner_rect, |ui| {
        ui.add(Spinner::new().size(spinner_size).color(Color32::WHITE));
    });
}

/// Draw a single page image, using the texture cache for efficiency.
/// If the image is a GIF, use the GIF loader.
pub fn draw_single_page(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut crate::texture_cache::TextureCache,
) {
    match &loaded.image {
        PageImage::AnimatedGif { .. } => draw_gif(ui, loaded, area, zoom, pan),
        PageImage::Static(_) => draw_static_image(ui, loaded, area, zoom, pan, cache),
    }
}

/// Draw a static (non-animated) image, using the cache.
/// Texture is cached by page index and zoom for fast zooming.
fn draw_static_image(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut crate::texture_cache::TextureCache,
) {
    let (w, h) = match &loaded.image {
        PageImage::Static(img) => img.dimensions(),
        _ => return,
    };
    let disp_size = Vec2::new(w as f32 * zoom, h as f32 * zoom);

    let ctx = ui.ctx().clone();
    // Use cached texture if available, otherwise upload and cache
    let handle = if let Some(handle) = cache.get_single(loaded.index, zoom) {
        handle.clone()
    } else {
        info!("Uploading texture for page {} at zoom {}", loaded.index, zoom);
        let color_img = match &loaded.image {
            PageImage::Static(img) => egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &img.to_rgba8(),
            ),
            _ => return,
        };
        let handle = ctx.load_texture(
            format!("tex{}_{}", loaded.index, zoom),
            color_img,
            egui::TextureOptions::default(),
        );
        cache.set_single(loaded.index, zoom, handle.clone());
        handle
    };

    let rect = Rect::from_center_size(area.center() + pan, disp_size);
    ui.allocate_ui_at_rect(rect, |ui| {
        ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size));
    });
}

/// Draw an animated GIF using all frames and correct timing.
fn draw_gif(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
) {
    if let PageImage::AnimatedGif { frames, delays, start_time } = &loaded.image {
        if frames.is_empty() {
            warn!("GIF has no frames: {}", loaded.filename);
            return;
        }
        // Calculate which frame to show
        let elapsed = start_time.elapsed().as_millis() as u64;
        let mut total = 0u64;
        let mut idx = 0;
        let mut acc = 0u64;
        let total_duration: u64 = delays.iter().map(|d| *d as u64 * 10).sum();
        let mut t = elapsed % total_duration;
        for (i, delay) in delays.iter().enumerate() {
            let frame_time = *delay as u64 * 10;
            if t < acc + frame_time {
                idx = i;
                break;
            }
            acc += frame_time;
        }
        let frame = &frames[idx];
        let w = frame.size[0] as f32;
        let h = frame.size[1] as f32;
        let disp_size = Vec2::new(w * zoom, h * zoom);

        let ctx = ui.ctx().clone();
        let handle = ctx.load_texture(
            format!("gif{}_{}", loaded.index, idx),
            frame.clone(),
            egui::TextureOptions::default(),
        );
        let rect = Rect::from_center_size(area.center() + pan, disp_size);
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size));
        });
        ui.ctx().request_repaint(); // ensure animation updates
    } else {
        warn!("draw_gif called on non-gif image");
    }
}

/// Draw two pages side by side, using the texture cache for efficiency.
/// If either page is a GIF, use the GIF loader for that page.
pub fn draw_dual_page(
    ui: &mut Ui,
    loaded_left: &LoadedPage,
    loaded_right: Option<&LoadedPage>,
    area: Rect,
    zoom: f32,
    margin: f32,
    left_first: bool,
    pan: Vec2,
    cache: &mut crate::texture_cache::TextureCache,
) {
    // Left page
    let (w1, h1) = match &loaded_left.image {
        PageImage::Static(img) => img.dimensions(),
        PageImage::AnimatedGif { frames, .. } if !frames.is_empty() => {
            (frames[0].size[0] as u32, frames[0].size[1] as u32)
        }
        _ => return,
    };
    let disp_size1 = Vec2::new(w1 as f32 * zoom, h1 as f32 * zoom);

    let ctx = ui.ctx().clone();
    let handle1 = if let PageImage::Static(_) = &loaded_left.image {
        if let Some(handle) = cache.get_single(loaded_left.index, zoom) {
            Some(handle.clone())
        } else {
            info!("Uploading texture for left page {} at zoom {}", loaded_left.index, zoom);
            let color_img1 = match &loaded_left.image {
                PageImage::Static(img) => egui::ColorImage::from_rgba_unmultiplied(
                    [w1 as usize, h1 as usize],
                    &img.to_rgba8(),
                ),
                _ => return,
            };
            let handle = ctx.load_texture(
                format!("tex{}_{}", loaded_left.index, zoom),
                color_img1,
                egui::TextureOptions::default(),
            );
            cache.set_single(loaded_left.index, zoom, handle.clone());
            Some(handle)
        }
    } else {
        None
    };

    let (handle2, disp_size2) = if let Some(loaded2) = loaded_right {
        let (w2, h2) = match &loaded2.image {
            PageImage::Static(img) => img.dimensions(),
            PageImage::AnimatedGif { frames, .. } if !frames.is_empty() => {
                (frames[0].size[0] as u32, frames[0].size[1] as u32)
            }
            _ => return,
        };
        let disp_size2 = Vec2::new(w2 as f32 * zoom, h2 as f32 * zoom);
        let handle2 = if let PageImage::Static(_) = &loaded2.image {
            if let Some(handle) = cache.get_single(loaded2.index, zoom) {
                Some(handle.clone())
            } else {
                info!("Uploading texture for right page {} at zoom {}", loaded2.index, zoom);
                let color_img2 = match &loaded2.image {
                    PageImage::Static(img) => egui::ColorImage::from_rgba_unmultiplied(
                        [w2 as usize, h2 as usize],
                        &img.to_rgba8(),
                    ),
                    _ => return,
                };
                let handle = ctx.load_texture(
                    format!("tex{}_{}", loaded2.index, zoom),
                    color_img2,
                    egui::TextureOptions::default(),
                );
                cache.set_single(loaded2.index, zoom, handle.clone());
                Some(handle)
            }
        } else {
            None
        };
        (handle2, Some(disp_size2))
    } else {
        (None, None)
    };

    let center = area.center() + pan;
    if let (Some(disp_size2), Some(handle2)) = (disp_size2, handle2) {
        let total_width = disp_size1.x + disp_size2.x + margin;
        let left_start = center.x - total_width / 2.0;

        let (rect_left, rect_right) = (
            Rect::from_min_size(
                pos2(left_start, center.y - disp_size1.y / 2.0),
                disp_size1,
            ),
            Rect::from_min_size(
                pos2(left_start + disp_size1.x + margin, center.y - disp_size2.y / 2.0),
                disp_size2,
            ),
        );

        if left_first {
            // Left page
            match &loaded_left.image {
                PageImage::AnimatedGif { .. } => draw_gif_at_rect(ui, loaded_left, rect_left, zoom, pan),
                PageImage::Static(_) => {
                    if let Some(handle1) = &handle1 {
                        ui.allocate_ui_at_rect(rect_left, |ui| {
                            ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            }
            // Right page
            if let Some(loaded2) = loaded_right {
                match &loaded2.image {
                    PageImage::AnimatedGif { .. } => draw_gif_at_rect(ui, loaded2, rect_right, zoom, pan),
                    PageImage::Static(_) => {
                        ui.allocate_ui_at_rect(rect_right, |ui| {
                            ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
                        });
                    }
                }
            }
        } else {
            // Right page first
            if let Some(loaded2) = loaded_right {
                match &loaded2.image {
                    PageImage::AnimatedGif { .. } => draw_gif_at_rect(ui, loaded2, rect_left, zoom, pan),
                    PageImage::Static(_) => {
                        ui.allocate_ui_at_rect(rect_left, |ui| {
                            ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
                        });
                    }
                }
            }
            match &loaded_left.image {
                PageImage::AnimatedGif { .. } => draw_gif_at_rect(ui, loaded_left, rect_right, zoom, pan),
                PageImage::Static(_) => {
                    if let Some(handle1) = &handle1 {
                        ui.allocate_ui_at_rect(rect_right, |ui| {
                            ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            }
        }
    } else {
        let rect = Rect::from_center_size(area.center() + pan, disp_size1);
        match &loaded_left.image {
            PageImage::AnimatedGif { .. } => draw_gif_at_rect(ui, loaded_left, rect, zoom, pan),
            PageImage::Static(_) => {
                if let Some(handle1) = &handle1 {
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                    });
                }
            }
        }
    }
}

/// Draw a GIF at a specific rect (animated)
fn draw_gif_at_rect(
    ui: &mut Ui,
    loaded: &LoadedPage,
    rect: Rect,
    zoom: f32,
    pan: Vec2,
) {
    if let PageImage::AnimatedGif { frames, delays, start_time } = &loaded.image {
        if frames.is_empty() {
            warn!("GIF has no frames: {}", loaded.filename);
            return;
        }
        let elapsed = start_time.elapsed().as_millis() as u64;
        let total_duration: u64 = delays.iter().map(|d| *d as u64 * 10).sum();
        let mut acc = 0u64;
        let t = elapsed % total_duration;
        let mut idx = 0;
        for (i, delay) in delays.iter().enumerate() {
            let frame_time = *delay as u64 * 10;
            if t < acc + frame_time {
                idx = i;
                break;
            }
            acc += frame_time;
        }
        let frame = &frames[idx];
        let ctx = ui.ctx().clone();
        let handle = ctx.load_texture(
            format!("gif{}_{}", loaded.index, idx),
            frame.clone(),
            egui::TextureOptions::default(),
        );
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&handle).fit_to_exact_size(rect.size()));
        });
        ui.ctx().request_repaint();
    }
}