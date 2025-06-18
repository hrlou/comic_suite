//! Image drawing helpers for egui.

use crate::prelude::*;

/// Draw a centered spinner in the given area.
pub fn draw_spinner(ui: &mut Ui, area: Rect) {
    let spinner_size = 48.0;
    let spinner_rect = Rect::from_center_size(area.center(), Vec2::splat(spinner_size));
    ui.allocate_ui_at_rect(spinner_rect, |ui| {
        ui.add(Spinner::new().size(spinner_size).color(Color32::WHITE));
    });
}

/// Draw a single page image, using the texture cache for efficiency.
pub fn draw_single_page(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut TextureCache,
) {
    match &loaded.image {
        PageImage::AnimatedGif { .. } => draw_gif(ui, loaded, area, zoom, pan),
        PageImage::Static(_) => draw_static_image(ui, loaded, area, zoom, pan, cache),
    }
}

fn draw_static_image(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut TextureCache,
) {
    let (w, h) = match &loaded.image {
        PageImage::Static(img) => img.dimensions(),
        _ => return,
    };
    let disp_size = Vec2::new(w as f32 * zoom, h as f32 * zoom);

    let ctx = ui.ctx().clone();
    let handle = if let Some(handle) = cache.get_single(loaded.index, zoom) {
        handle.clone()
    } else {
        let color_img = match &loaded.image {
            PageImage::Static(img) => {
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.to_rgba8())
            }
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

fn draw_gif(ui: &mut Ui, loaded: &LoadedPage, area: Rect, zoom: f32, pan: Vec2) {
    ui.ctx().request_repaint();
    if let PageImage::AnimatedGif {
        frames,
        delays,
        start_time,
    } = &loaded.image
    {
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
        let w = frame.size[0] as f32;
        let h = frame.size[1] as f32;
        let disp_size = Vec2::new(w * zoom, h * zoom);
        let rect = Rect::from_center_size(area.center() + pan, disp_size);

        let ctx = ui.ctx().clone();
        let tex_name = format!("gif{}_{}", loaded.index, idx);
        let handle = ctx.load_texture(
            tex_name,
            frame.clone(),
            egui::TextureOptions::default(),
        );

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size));
        });
        ui.ctx().request_repaint();
    } else {
        warn!("draw_gif called on non-gif image");
    }
}

/// Draw two pages side by side, using the texture cache for efficiency.
pub fn draw_dual_page(
    ui: &mut Ui,
    loaded_left: &LoadedPage,
    loaded_right: Option<&LoadedPage>,
    area: Rect,
    zoom: f32,
    margin: f32,
    left_first: bool,
    pan: Vec2,
    cache: &mut TextureCache,
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
            egui::Rect::from_min_size(
                egui::pos2(left_start, center.y - disp_size1.y / 2.0),
                disp_size1,
            ),
            egui::Rect::from_min_size(
                egui::pos2(
                    left_start + disp_size1.x + margin,
                    center.y - disp_size2.y / 2.0,
                ),
                disp_size2,
            ),
        );

        if left_first {
            match &loaded_left.image {
                PageImage::AnimatedGif { .. } => {
                    draw_gif_at_rect(ui, loaded_left, rect_left, zoom, pan)
                }
                PageImage::Static(_) => {
                    if let Some(handle1) = &handle1 {
                        ui.allocate_ui_at_rect(rect_left, |ui| {
                            ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            }
            if let Some(loaded2) = loaded_right {
                match &loaded2.image {
                    PageImage::AnimatedGif { .. } => {
                        draw_gif_at_rect(ui, loaded2, rect_right, zoom, pan)
                    }
                    PageImage::Static(_) => {
                        ui.allocate_ui_at_rect(rect_right, |ui| {
                            ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
                        });
                    }
                }
            }
        } else {
            if let Some(loaded2) = loaded_right {
                match &loaded2.image {
                    PageImage::AnimatedGif { .. } => {
                        draw_gif_at_rect(ui, loaded2, rect_left, zoom, pan)
                    }
                    PageImage::Static(_) => {
                        ui.allocate_ui_at_rect(rect_left, |ui| {
                            ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
                        });
                    }
                }
            }
            match &loaded_left.image {
                PageImage::AnimatedGif { .. } => {
                    draw_gif_at_rect(ui, loaded_left, rect_right, zoom, pan)
                }
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
        let rect = egui::Rect::from_center_size(area.center() + pan, disp_size1);
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

fn draw_gif_at_rect(ui: &mut Ui, loaded: &LoadedPage, rect: Rect, zoom: f32, pan: Vec2) {
    if let PageImage::AnimatedGif {
        frames,
        delays,
        start_time,
    } = &loaded.image
    {
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

/// Adjust the zoom factor based on scroll input.
/// Returns true if zoom changed.
pub fn handle_zoom(
    zoom: &mut f32,
    ctx: &egui::Context,
    min_zoom: f32,
    max_zoom: f32,
    texture_cache: &mut TextureCache,
    has_initialised_zoom: &mut bool,
) -> bool {
    let zoom_speed = 1.1;
    let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
    if scroll_delta.y != 0.0 && ctx.is_pointer_over_area() {
        let old_zoom = *zoom;
        if scroll_delta.y > 0.0 {
            *zoom = (*zoom * zoom_speed).min(max_zoom);
        } else if scroll_delta.y < 0.0 {
            *zoom = (*zoom / zoom_speed).max(min_zoom);
        }
        if (*zoom - old_zoom).abs() > f32::EPSILON {
            *has_initialised_zoom = true;
            texture_cache.clear();
            return true;
        }
    }
    false
}

pub fn clamp_pan(app: &mut CBZViewerApp, image_dims: (u32, u32), area: Rect) {
    let (w, h) = image_dims;
    let avail = area.size();
    let max_x = ((w as f32 * app.zoom - avail.x) / 2.0).max(0.0);
    let max_y = ((h as f32 * app.zoom - avail.y) / 2.0).max(0.0);
    app.pan_offset.x = app.pan_offset.x.clamp(-max_x, max_x);
    app.pan_offset.y = app.pan_offset.y.clamp(-max_y, max_y);
}

/// Adjust the pan offset based on drag input.
pub fn handle_pan(
    pan_offset: &mut Vec2,
    drag_start: &mut Option<egui::Pos2>,
    response: &egui::Response,
    texture_cache: &mut TextureCache,
) {
    if response.drag_started() {
        *drag_start = response.interact_pointer_pos();
    }

    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(start) = *drag_start {
                let delta = pos - start;
                *pan_offset += delta;
                *drag_start = Some(pos);
                texture_cache.clear();
            }
        }
    }

    if response.drag_stopped() {
        *drag_start = None;
    }
}