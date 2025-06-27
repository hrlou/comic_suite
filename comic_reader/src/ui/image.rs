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
        PageImage::AnimatedGif { .. } => draw_gif(ui, loaded, area, zoom, pan, cache),
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
    let ctx = ui.ctx().clone(); // clone context early to avoid ui borrow conflicts

    // Helper function: returns display size and owned clone of texture handle (if any)
    fn get_page_data(
        page: &LoadedPage,
        zoom: f32,
        cache: &mut TextureCache,
        ctx: &egui::Context,
    ) -> Option<(Vec2, Option<egui::TextureHandle>)> {
        let (w, h) = match &page.image {
            PageImage::Static(img) => img.dimensions(),
            PageImage::AnimatedGif { frames, .. } if !frames.is_empty() => {
                (frames[0].size[0] as u32, frames[0].size[1] as u32)
            }
            _ => return None,
        };
        let disp_size = Vec2::new(w as f32 * zoom, h as f32 * zoom);

        let handle = if let PageImage::Static(img) = &page.image {
            if let Some(handle) = cache.get_single(page.index, zoom) {
                Some(handle.clone()) // clone owned handle, no borrow leak
            } else {
                let rgba_bytes = img.to_rgba8();
                let color_img = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    rgba_bytes.as_flat_samples().as_slice(),
                );
                let handle = ctx.load_texture(
                    format!("tex{}_{}", page.index, zoom),
                    color_img,
                    egui::TextureOptions::default(),
                );
                cache.set_single(page.index, zoom, handle.clone());
                Some(handle)
            }
        } else {
            None
        };

        Some((disp_size, handle))
    }

    let (disp_size1, handle1) = match get_page_data(loaded_left, zoom, cache, &ctx) {
        Some(data) => data,
        None => return,
    };

    let (disp_size2, handle2) = if let Some(loaded2) = loaded_right {
        match get_page_data(loaded2, zoom, cache, &ctx) {
            Some(data) => (Some(data.0), data.1),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    let center = area.center() + pan;

    if let (Some(disp_size2), Some(handle2)) = (disp_size2, handle2) {
        let total_width = disp_size1.x + disp_size2.x + margin;
        let left_start = center.x - total_width * 0.5;

        let rect_left = egui::Rect::from_min_size(
            egui::pos2(left_start, center.y - disp_size1.y * 0.5),
            disp_size1,
        );
        let rect_right = egui::Rect::from_min_size(
            egui::pos2(left_start + disp_size1.x + margin, center.y - disp_size2.y * 0.5),
            disp_size2,
        );

        if left_first {
            match &loaded_left.image {
                PageImage::AnimatedGif { .. } => {
                    draw_gif_at_rect(ui, loaded_left, rect_left, zoom, pan, cache);
                }
                PageImage::Static(_) => {
                    if let Some(handle) = handle1 {
                        ui.allocate_ui_at_rect(rect_left, |ui| {
                            ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            }
            if let Some(loaded2) = loaded_right {
                match &loaded2.image {
                    PageImage::AnimatedGif { .. } => {
                        draw_gif_at_rect(ui, loaded2, rect_right, zoom, pan, cache);
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
                        draw_gif_at_rect(ui, loaded2, rect_left, zoom, pan, cache);
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
                    draw_gif_at_rect(ui, loaded_left, rect_right, zoom, pan, cache);
                }
                PageImage::Static(_) => {
                    if let Some(handle) = handle1 {
                        ui.allocate_ui_at_rect(rect_right, |ui| {
                            ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            }
        }
    } else {
        let rect = egui::Rect::from_center_size(center, disp_size1);
        match &loaded_left.image {
            PageImage::AnimatedGif { .. } => {
                draw_gif_at_rect(ui, loaded_left, rect, zoom, pan, cache);
            }
            PageImage::Static(_) => {
                if let Some(handle) = handle1 {
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size1));
                    });
                }
            }
        }
    }
}

/// Draw a GIF in the given area by forwarding to `draw_gif_at_rect` (calls must pass cache).
pub fn draw_gif(
    ui: &mut Ui,
    loaded: &LoadedPage,
    area: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut TextureCache,
) {
    let (w, h) = if let PageImage::AnimatedGif { frames, .. } = &loaded.image {
        if frames.is_empty() {
            warn!("GIF has no frames: {}", loaded.filename);
            return;
        }
        (frames[0].size[0] as f32, frames[0].size[1] as f32)
    } else {
        warn!("draw_gif called on non-gif image");
        return;
    };

    let disp_size = Vec2::new(w * zoom, h * zoom);
    let rect = Rect::from_center_size(area.center() + pan, disp_size);

    draw_gif_at_rect(ui, loaded, rect, zoom, pan, cache);
}

/// Draw a GIF at the specified rect, using the texture cache to avoid reloads.
pub fn draw_gif_at_rect(
    ui: &mut Ui,
    loaded: &LoadedPage,
    rect: Rect,
    zoom: f32,
    pan: Vec2,
    cache: &mut TextureCache,
) {
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
        let total_duration: u64 = delays.iter().map(|d| *d as u64).sum();
        let t = elapsed % total_duration;

        let mut acc = 0u64;
        let mut idx = 0;
        for (i, delay) in delays.iter().enumerate() {
            let frame_time = *delay as u64;
            if t < acc + frame_time {
                idx = i;
                break;
            }
            acc += frame_time;
        }

        let ctx = ui.ctx().clone();
        let key = format!("gif{}_{}", loaded.index, idx);

        let handle = if let Some(handle) = cache.get_animated(&key) {
            handle.clone()
        } else {
            let new_handle = ctx.load_texture(
                key.clone(),
                frames[idx].clone(),
                egui::TextureOptions::default(),
            );
            cache.set_animated(key, new_handle.clone());
            new_handle
        };

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&handle).fit_to_exact_size(rect.size()));
        });

        ui.ctx().request_repaint();
    }
}

pub fn clamp_pan(app: &mut CBZViewerApp, image_dims: (u32, u32), viewport_rect: egui::Rect) {
    let (img_w, img_h) = image_dims;
    let viewport_size = viewport_rect.size();

    let scaled_w = img_w as f32 * app.zoom;
    let scaled_h = img_h as f32 * app.zoom;

    // Half-size margin around the image
    let margin_x = scaled_w * 0.5;
    let margin_y = scaled_h * 0.5;

    // Calculate max pan with margin included
    let max_pan_x = ((scaled_w - viewport_size.x) / 2.0 + margin_x).max(0.0);
    let max_pan_y = ((scaled_h - viewport_size.y) / 2.0 + margin_y).max(0.0);

    // Smooth spring-back factor
    let k = 0.2;

    let target_x = app.pan_offset.x.clamp(-max_pan_x, max_pan_x);
    let target_y = app.pan_offset.y.clamp(-max_pan_y, max_pan_y);

    // Move pan offset gently towards the clamped target
    app.pan_offset.x += (target_x - app.pan_offset.x) * k;
    app.pan_offset.y += (target_y - app.pan_offset.y) * k;
}

/// Adjust the pan offset based on drag input.
/// Records starting position and moves the view with the drag delta.
pub fn handle_pan(
    pan_offset: &mut Vec2,
    drag_start: &mut Option<egui::Pos2>,
    original_offset: &mut Vec2,
    response: &egui::Response,
) {
    if response.drag_started() {
        *drag_start = response.interact_pointer_pos();
        *original_offset = *pan_offset;
    }

    if response.dragged() {
        if let Some(start_pos) = *drag_start {
            if let Some(current_pos) = response.interact_pointer_pos() {
                let delta = current_pos - start_pos;
                *pan_offset = *original_offset + Vec2::new(delta.x, delta.y);
            }
        }
    }

    if response.drag_released() {
        *drag_start = None;
    }
}

/// Handle zooming centered at the cursor position.
///
/// `zoom`: current zoom level.
/// `pan_offset`: current pan offset (will be adjusted).
/// `cursor_pos`: cursor position in screen coords.
/// `area_rect`: rect of the image/view in screen coords.
/// `scroll_delta_y`: vertical scroll delta (positive to zoom in).
/// `min_zoom`, `max_zoom`: zoom clamp range.
/// `texture_cache`: texture cache to clear on zoom change.
/// `has_initialised_zoom`: mutable flag to track first zoom event.
///
/// Returns true if zoom changed.
pub fn handle_zoom(
    zoom: &mut f32,
    pan_offset: &mut egui::Vec2,
    cursor_pos: egui::Pos2,
    area_rect: egui::Rect,
    scroll_delta_y: f32,
    min_zoom: f32,
    max_zoom: f32,
    texture_cache: &mut TextureCache,
    has_initialised_zoom: &mut bool,
) -> bool {
    if scroll_delta_y.abs() < f32::EPSILON {
        return false;
    }

    let old_zoom = *zoom;
    let zoom_sensitivity = 0.1;

    // Continuous zooming using scroll delta
    let zoom_factor = (1.0 + zoom_sensitivity * scroll_delta_y).clamp(0.5, 2.0);
    *zoom = (*zoom * zoom_factor).clamp(min_zoom, max_zoom);

    if (*zoom - old_zoom).abs() > f32::EPSILON {
        let cursor_rel = egui::vec2(
            cursor_pos.x - area_rect.center().x,
            cursor_pos.y - area_rect.center().y,
        );
        let effective_factor = *zoom / old_zoom;

        *pan_offset = (*pan_offset - cursor_rel) * effective_factor + cursor_rel;
        *has_initialised_zoom = true;
        texture_cache.clear();
        return true;
    }

    false
}