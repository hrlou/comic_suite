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

/// Macro to handle drawing a static image using the cache.
/// Reduces boilerplate for both single and dual page drawing.
macro_rules! draw_static {
    ($ui:expr, $loaded:expr, $area:expr, $zoom:expr, $pan:expr, $cache:expr, $disp_size:ident, $handle:ident) => {{
        let (w, h) = match &$loaded.image {
            PageImage::Static(img) => img.dimensions(),
            _ => return,
        };
        let $disp_size = Vec2::new(w as f32 * $zoom, h as f32 * $zoom);

        let ctx = $ui.ctx().clone();
        let $handle = if let Some(handle) = $cache.get_single($loaded.index, $zoom) {
            handle.clone()
        } else {
            let color_img = match &$loaded.image {
                PageImage::Static(img) => {
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.to_rgba8())
                }
                _ => return,
            };
            let handle = ctx.load_texture(
                format!("tex{}_{}", $loaded.index, $zoom),
                color_img,
                egui::TextureOptions::default(),
            );
            $cache.set_single($loaded.index, $zoom, handle.clone());
            handle
        };

        let rect = Rect::from_center_size($area.center() + $pan, $disp_size);
        $ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&$handle).fit_to_exact_size($disp_size));
        });
    }};
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
        PageImage::Static(_) => draw_static!(ui, loaded, area, zoom, pan, cache, disp_size, handle),
    }
}

/// Macro to handle dual page drawing logic, including GIF/static dispatch.
macro_rules! draw_page_at_rect {
    ($ui:expr, $loaded:expr, $rect:expr, $disp_size:expr, $handle:expr, $cache:expr, $zoom:expr, $pan:expr) => {
        match &$loaded.image {
            PageImage::AnimatedGif { .. } => {
                draw_gif_at_rect($ui, $loaded, $rect, $zoom, $pan, $cache);
            }
            PageImage::Static(_) => {
                if let Some(handle) = &$handle {
                    $ui.allocate_ui_at_rect($rect, |ui| {
                        ui.add(Image::from_texture(handle).fit_to_exact_size($disp_size));
                    });
                }
            }
        }
    };
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
    let ctx = ui.ctx().clone();

    // Helper: get display size and texture handle for a page
    fn get_page_data(
        page: &LoadedPage,
        zoom: f32,
        cache: &mut TextureCache,
        ctx: &egui::Context,
    ) -> Option<(Vec2, Option<egui::TextureHandle>)> {
        match &page.image {
            PageImage::Static(img) => {
                let (w, h) = img.dimensions();
                let disp_size = Vec2::new(w as f32 * zoom, h as f32 * zoom);
                let handle = if let Some(h) = cache.get_single(page.index, zoom) {
                    Some(h.clone())
                } else {
                    let rgba_bytes = img.to_rgba8();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        rgba_bytes.as_flat_samples().as_slice(),
                    );
                    let h = ctx.load_texture(
                        format!("tex{}_{}", page.index, zoom),
                        color_img,
                        egui::TextureOptions::default(),
                    );
                    cache.set_single(page.index, zoom, h.clone());
                    Some(h)
                };
                Some((disp_size, handle))
            }
            PageImage::AnimatedGif { frames, .. } if !frames.is_empty() => {
                let (w, h) = (frames[0].size[0] as u32, frames[0].size[1] as u32);
                Some((Vec2::new(w as f32 * zoom, h as f32 * zoom), None))
            }
            _ => None,
        }
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
            egui::pos2(
                left_start + disp_size1.x + margin,
                center.y - disp_size2.y * 0.5,
            ),
            disp_size2,
        );

        if left_first {
            draw_page_at_rect!(ui, loaded_left, rect_left, disp_size1, handle1, cache, zoom, pan);
            if let Some(loaded2) = loaded_right {
                draw_page_at_rect!(ui, loaded2, rect_right, disp_size2, Some(handle2), cache, zoom, pan);
            }
        } else {
            if let Some(loaded2) = loaded_right {
                draw_page_at_rect!(ui, loaded2, rect_left, disp_size2, Some(handle2), cache, zoom, pan);
            }
            draw_page_at_rect!(ui, loaded_left, rect_right, disp_size1, handle1, cache, zoom, pan);
        }
    } else {
        // Only one page to show
        let rect = egui::Rect::from_center_size(center, disp_size1);
        draw_page_at_rect!(ui, loaded_left, rect, disp_size1, handle1, cache, zoom, pan);
    }
}

/// Draw a GIF in the given area by forwarding to `draw_gif_at_rect`.
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
/// Handles frame timing and texture management for animated playback.
pub fn draw_gif_at_rect(
    ui: &mut Ui,
    loaded: &LoadedPage,
    rect: Rect,
    _zoom: f32,
    _pan: Vec2,
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

        // Compute which frame to show based on elapsed time and per-frame delays
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

        // Request repaint for smooth animation
        ui.ctx().request_repaint();
    }
}

/// Clamp the pan offset so the image stays within the viewport bounds.
/// Uses a spring-back effect for smoothness.
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

    // Spring-back factor for smooth clamping
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

    if response.drag_stopped() {
        *drag_start = None;
    }
}

/// Handle zooming centered at the cursor position.
/// Adjusts pan so the zoom is focused at the cursor.
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
        // Adjust pan so the zoom is centered at the cursor
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
