use crate::prelude::*;
use crate::ui::modules;

impl CBZViewerApp {
    pub fn display_main_empty(&mut self, ctx: &egui::Context) {
        // No archive loaded, show a message
        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.label(
                        RichText::new("No Image Loaded \u{e09a}").text_style(TextStyle::Heading),
                    );
                },
            );
        });
    }

    pub fn display_main_full(&mut self, ctx: &egui::Context) {
        if self.total_pages > 0 {
            let response = self.display_central_image_area(ctx, self.total_pages);

            // Check if mouse is over the zoom area and there is a scroll
            if let Some(cursor_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let _zoomed = handle_zoom(
                    &mut self.zoom,
                    &mut self.pan_offset,
                    cursor_pos,
                    response.rect,
                    ctx.input(|i| i.raw_scroll_delta.y),
                    0.05,
                    10.0,
                    &mut self.texture_cache, // pass cursor_pos here
                    &mut self.has_initialised_zoom,
                );
            }

            self.handle_input(ctx);
        }
    }

    pub fn display_manifest_editor(&mut self, ctx: &egui::Context) {
        if let Some(archive_mutex) = &self.archive {
            if let Ok(mut archive) = archive_mutex.lock() {
                if !self.loading_pages.lock().unwrap().is_empty() && self.total_pages > 0 {
                    self.ui_logger.warn(
                        "Please wait for all images to finish loading before editing the manifest.",
                    );
                } else {
                    Window::new("Edit Manifest")
                        .open(&mut self.show_manifest_editor)
                        .show(ctx, |ui| {
                            let mut editor = ManifestEditor::new(&mut archive);
                            if editor.ui(ui, ctx).is_err() {
                                self.ui_logger.error("Cannot edit Manifest");
                            }
                        });
                }
            }
        }
    }

    /// Draw the top bar (navigation, mode toggles, file info).
    pub fn display_top_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::menu::bar(ui, |ui| {
                    modules::ui_file(self, ui, ctx);
                    modules::ui_edit(self, ui, ctx);
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    modules::ui_navigation(self, ui);
                });
            });
        });
    }

    /// Draw the bottom bar (zoom, navigation, page info).
    pub fn display_bottom_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.archive_path.is_some() {
                    modules::ui_zoom_slider(self, ui);
                    ui.separator();
                    modules::ui_goto_page(self, ui);
                    ui.separator();
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        modules::ui_page_nav(self, ui, self.total_pages);
                        modules::ui_log_msg(self, ui);
                    });
                } else {
                    modules::ui_log_msg(self, ui);
                }
            });
        });
    }

    /// Draw the central image area (single/dual page, spinner).
    /// Returns the egui Response for further input handling.
    pub fn display_central_image_area(
        &mut self,
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
                let mut image_lru = self.image_lru.lock().unwrap();
                let page1 = self.current_page;
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
            let total_size = if self.double_page_mode {
                if let (Some(l1), Some(l2)) = (&loaded1, &loaded2) {
                    let (w1, h1) = l1.image.dimensions();
                    let (w2, h2) = l2.image.dimensions();
                    (w1 + w2, h1.max(h2))
                } else if let Some(l1) = &loaded1 {
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
                &mut self.pan_offset,
                &mut self.drag_start,
                &mut self.original_pan_offset,
                &response,
            );

            // Clamp pan after dragging ends
            if response.drag_stopped() {
                clamp_pan(self, total_size, image_area);
            }

            // Drawing happens after image_lru lock is dropped and pan handled
            if self.double_page_mode {
                let pairs = &self.dual_page_pairs;
                let pair_idx = pairs
                    .iter()
                    .position(|(l, _)| *l == self.current_page)
                    .unwrap_or(0);
                let (left_idx, right_idx_opt) = pairs[pair_idx];
                let mut image_lru = self.image_lru.lock().unwrap();
                let left = image_lru.get(&left_idx).cloned();
                let right = right_idx_opt.and_then(|r| image_lru.get(&r).cloned());
                drop(image_lru);

                match (left, right) {
                    (Some(l), Some(r)) => {
                        let handle = get_or_generate_dual_texture(
                            &mut self.texture_cache,
                            &l,
                            &r,
                            self.zoom,
                            ctx,
                        );
                        // Draw the stitched texture as a single image
                        let (w, h) = (
                            l.image.dimensions().0 + r.image.dimensions().0,
                            l.image.dimensions().1.max(r.image.dimensions().1),
                        );
                        let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);
                        let rect = egui::Rect::from_center_size(
                            image_area.center() + self.pan_offset,
                            disp_size,
                        );
                        let builder = egui::UiBuilder::default().max_rect(rect);
                        ui.allocate_new_ui(builder, |ui| {
                            ui.add(egui::Image::new(&handle).fit_to_exact_size(disp_size));
                        });
                    }
                    (Some(l), None) => {
                        draw_single_page(
                            ui,
                            &l,
                            image_area,
                            self.zoom,
                            self.pan_offset,
                            &mut self.texture_cache,
                        );
                    }
                    (None, _) => {
                        draw_spinner(ui, image_area);
                    }
                }
            } else {
                if let Some(ref loaded) = single_loaded {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, loaded);
                    }
                    draw_single_page(
                        ui,
                        loaded,
                        image_area,
                        self.zoom,
                        self.pan_offset,
                        &mut self.texture_cache,
                    );
                } else {
                    draw_spinner(ui, image_area);
                }
            }
        });

        response_opt.expect("Central panel UI always provides a response")
    }
}
