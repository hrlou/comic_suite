//! UI layout: top bar, bottom bar, and central image area.

use crate::prelude::*;

/// Draw the top bar (navigation, mode toggles, file info).
pub fn draw_top_bar(app: &mut CBZViewerApp, ctx: &Context, total_pages: usize) {
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Comic...").clicked() {
                        app.on_open_comic = true;
                        ui.close_menu();
                    }
                    if ui.button("Open Folder...").clicked() {
                        app.on_open_folder = true;
                        ui.close_menu();
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                let direction_label = if app.right_to_left {
                    "\u{f191}"
                } else {
                    "\u{f152}"
                };
                if ui
                    .button(direction_label)
                    .on_hover_text("Reading direction")
                    .clicked()
                {
                    app.right_to_left = !app.right_to_left;
                    app.texture_cache.clear();
                }

                if ui
                    .selectable_label(app.double_page_mode, "\u{f518}")
                    .on_hover_text("Dual page mode")
                    .clicked()
                {
                    if app.double_page_mode {
                        app.double_page_mode = false;
                        app.current_page = app.current_page.min(total_pages.saturating_sub(1));
                        app.has_initialised_zoom = false;
                        app.texture_cache.clear();
                    } else {
                        if app.current_page > 0 && app.current_page % 2 != 0 {
                            app.current_page -= 1;
                        }
                        app.double_page_mode = true;
                        app.has_initialised_zoom = false;
                        app.texture_cache.clear();
                    }
                }

                if app.double_page_mode {
                    if ui
                        .button("\u{f08e}")
                        .on_hover_text("Bump over a single page, use this if there is misalignment")
                        .clicked()
                    {
                        if app.current_page + 1 < total_pages {
                            app.current_page += 1;
                            app.has_initialised_zoom = false;
                            app.texture_cache.clear();
                        }
                    }
                }
            });
        });
    });
}

pub fn goto_page_module(app: &mut CBZViewerApp, ui: &mut Ui) {
    let char_width = ui.fonts(|f| {
        let font_id = FontId::monospace(ui.style().text_styles[&TextStyle::Monospace].size);
        f.glyph_width(&font_id, '0')
    });
    let desired_width = char_width * 5.0 + 10.0;

    let response = ui.add_sized(
        [desired_width, ui.spacing().interact_size.y],
        TextEdit::singleline(&mut app.page_goto_box)
            .hint_text("###")
            .font(TextStyle::Monospace),
    );
    app.page_goto_box.retain(|c| c.is_ascii_digit());
    app.on_goto_page = ui
        .button("Jump")
        .on_hover_text("Jump to page number")
        .clicked()
        || (response.has_focus() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)));
}

/// Draw the bottom bar (zoom, navigation, page info).
pub fn draw_bottom_bar(app: &mut CBZViewerApp, ctx: &Context, total_pages: usize) {
    egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut app.zoom, 0.05..=10.0));
            if ui.button("Reset Zoom").clicked() {
                app.zoom = 1.0;
                app.pan_offset = Vec2::ZERO;
                app.has_initialised_zoom = false;
                app.texture_cache.clear();
            }
            ui.separator();
            goto_page_module(app, ui);
            ui.separator();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("\u{f061}").clicked() {
                    app.goto_next_page();
                }
                if ui.button("\u{f060}").clicked() {
                    app.goto_prev_page();
                }
                let page_label = if app.double_page_mode && app.current_page != 0 {
                    let left = app.current_page;
                    let right = (app.current_page + 1).min(total_pages.saturating_sub(1));
                    if app.right_to_left {
                        format!("Page ({},{})/{}", right + 1, left + 1, total_pages)
                    } else {
                        format!("Page ({},{})/{}", left + 1, right + 1, total_pages)
                    }
                } else {
                    format!("Page {}/{}", app.current_page + 1, total_pages)
                };
                ui.label(page_label);

                if let Some((msg, kind)) = &app.ui_logger.message {
                    ui.separator();
                    let color = match *kind {
                        crate::ui::UiLogLevel::Info => egui::Color32::WHITE,
                        crate::ui::UiLogLevel::Warning => egui::Color32::YELLOW,
                        crate::ui::UiLogLevel::Error => egui::Color32::RED,
                    };
                    ui.colored_label(color, msg.clone());
                }
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
