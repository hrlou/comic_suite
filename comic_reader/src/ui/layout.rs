//! UI layout: top bar, bottom bar, and central image area.

use crate::{prelude::*, ui::modules};

/// Draw the top bar (navigation, mode toggles, file info).
pub fn draw_top_bar(app: &mut CBZViewerApp, ctx: &Context, total_pages: usize) {
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            modules::ui_file(app, ui);
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                modules::ui_navigation(app, ui, total_pages);
            });
        });
    });
}

/// Draw the bottom bar (zoom, navigation, page info).
pub fn draw_bottom_bar(app: &mut CBZViewerApp, ctx: &Context) {
    egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            modules::ui_zoom_slider(app, ui);
            ui.separator();
            modules::ui_goto_page(app, ui);
            ui.separator();
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                modules::ui_log_msg(app, ui);
            });
        });
    });
}

/// Draw the central image area (single/dual page, spinner).
/// Returns the egui Response for further input handling.
pub fn draw_central_image_area(
    app: &mut CBZViewerApp,
    ctx: &Context,
    total_pages: usize,
) -> egui::Response {
    let mut response_opt = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        let image_area = ui.available_rect_before_wrap();

        let response = ui.allocate_rect(image_area, egui::Sense::click_and_drag());
        response_opt = Some(response.clone());

        // Load images from image_lru with a short lock scope
        let (loaded1, loaded2, single_loaded) = {
            let mut image_lru = app.image_lru.lock().unwrap();
            let page1 = app.current_page;
            let page2 = if page1 + 1 < total_pages {
                page1 + 1
            } else {
                usize::MAX
            };

            (
                image_lru.get(&page1).cloned(),
                if page2 != usize::MAX {
                    image_lru.get(&page2).cloned()
                } else {
                    None
                },
                image_lru.get(&page1).cloned(),
            )
            // lock dropped here
        };

        // Determine total size for clamping pan
        let total_size = if app.double_page_mode {
            if let (Some(ref l1), Some(ref l2)) = (&loaded1, &loaded2) {
                let (w1, h1) = l1.image.dimensions();
                let (w2, h2) = l2.image.dimensions();
                (w1 + w2, h1.max(h2))
            } else if let Some(ref l1) = loaded1 {
                l1.image.dimensions()
            } else {
                (0, 0)
            }
        } else {
            if let Some(ref l) = single_loaded {
                l.image.dimensions()
            } else {
                (0, 0)
            }
        };

        // Handle pan with a closure for clamping
        // Call handle_pan without closure
        handle_pan(
            &mut app.pan_offset,
            &mut app.drag_start,
            &mut app.original_pan_offset,
            &response,
        );

        // Clamp pan after dragging ends
        if response.drag_released() {
            clamp_pan(app, total_size, image_area);
        }

        // Drawing happens after image_lru lock is dropped and pan handled
        if app.double_page_mode {
            if let (Some(ref l1), Some(ref l2)) = (&loaded1, &loaded2) {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, l1);
                }
                draw_dual_page(
                    ui,
                    l1,
                    Some(l2),
                    image_area,
                    app.zoom,
                    PAGE_MARGIN_SIZE as f32,
                    !app.right_to_left,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
            } else if let Some(ref l1) = loaded1 {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, l1);
                }
                draw_single_page(
                    ui,
                    l1,
                    image_area,
                    app.zoom,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
            } else {
                draw_spinner(ui, image_area);
            }
        } else {
            if let Some(ref loaded) = single_loaded {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, loaded);
                }
                draw_single_page(
                    ui,
                    loaded,
                    image_area,
                    app.zoom,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
            } else {
                draw_spinner(ui, image_area);
            }
        }
    });

    response_opt.expect("Central panel UI always provides a response")
}
