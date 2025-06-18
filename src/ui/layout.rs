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

            // ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            //    let file_label = if app.double_page_mode && app.current_page != 0 {
            //        let left = app.current_page;
            //        let right = (app.current_page + 1).min(total_pages.saturating_sub(1));
            //        if app.right_to_left {
            //            format!(
            //                "{} | {}",
            //                app.filenames.get(right).unwrap_or(&String::from("")),
            //                app.filenames.get(left).unwrap_or(&String::from(""))
            //            )
            //        } else {
            //            format!(
            //                "{} | {}",
            //                app.filenames.get(left).unwrap_or(&String::from("")),
            //                app.filenames.get(right).unwrap_or(&String::from(""))
            //            )
            //        }
            //    } else {
            //        app.filenames
            //            .get(app.current_page)
            //            .cloned()
            //            .unwrap_or_else(|| String::from(""))
            //    };
            //    ui.label(file_label);
            // });
        });
    });
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

            let char_width = ui.fonts(|f| {
                let font_id = FontId::monospace(ui.style().text_styles[&TextStyle::Monospace].size);
                f.glyph_width(&font_id, '0')
            });
            let desired_width = char_width * 4.0 + 10.0; // +10 for padding
            let mut input_string = app.on_goto_page.1.to_string();
            ui.add_sized(
                [desired_width, ui.spacing().interact_size.y],
                TextEdit::singleline(&mut input_string)
                    .hint_text("Go to a page")
                    .font(TextStyle::Monospace),
            );
            input_string.retain(|c| c.is_ascii_digit());
            app.on_goto_page = (
                false,
                input_string
                    .parse::<usize>()
                    .unwrap_or("0".parse().unwrap_or(0)),
            );
            if ui.button("Goto").clicked() {
                app.on_goto_page.0 = true;
            }

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

        // Allocate rect without panning (no Sense::drag)
        let response = ui.allocate_rect(image_area, egui::Sense::hover());
        response_opt = Some(response);

        let response = ui.allocate_rect(image_area, egui::Sense::click_and_drag());
        handle_pan(
            &mut app.pan_offset,
            &mut app.drag_start,
            &response,
            &mut app.texture_cache,
        );

        // Display images or spinner depending on mode and loaded pages
        if app.double_page_mode {
            let page1 = app.current_page;
            let page2 = if page1 + 1 < total_pages {
                page1 + 1
            } else {
                usize::MAX
            };

            let loaded1 = app.image_lru.lock().unwrap().get(&page1).cloned();
            let loaded2 = if page2 != usize::MAX {
                app.image_lru.lock().unwrap().get(&page2).cloned()
            } else {
                None
            };

            if let (Some(loaded1), Some(loaded2)) = (&loaded1, &loaded2) {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, loaded1);
                }
                let left_first = !app.right_to_left;
                draw_dual_page(
                    ui,
                    loaded1,
                    Some(loaded2),
                    image_area,
                    app.zoom,
                    PAGE_MARGIN_SIZE as f32,
                    left_first,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
                let (w1, h1) = loaded1.image.dimensions();
                let (w2, h2) = loaded2.image.dimensions();
                let total_width = w1 + w2;
                let max_height = h1.max(h2);
                clamp_pan(app, (total_width, max_height), image_area);
            } else if let Some(loaded1) = &loaded1 {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, loaded1);
                }
                draw_single_page(
                    ui,
                    loaded1,
                    image_area,
                    app.zoom,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
                clamp_pan(app, loaded1.image.dimensions(), image_area);
            } else {
                draw_spinner(ui, image_area);
            }
        } else {
            let loaded = app
                .image_lru
                .lock()
                .unwrap()
                .get(&app.current_page)
                .cloned();
            if let Some(loaded) = loaded {
                if !app.has_initialised_zoom {
                    app.reset_zoom(image_area, &loaded);
                }
                draw_single_page(
                    ui,
                    &loaded,
                    image_area,
                    app.zoom,
                    app.pan_offset,
                    &mut app.texture_cache,
                );
                clamp_pan(app, loaded.image.dimensions(), image_area);
            } else {
                draw_spinner(ui, image_area);
            }
        }
    });

    response_opt.expect("Central panel UI always provides a response")
}
