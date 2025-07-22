use crate::ui::modules;
use crate::{prelude::*, ui};

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
                    if let Ok(mut logger) = self.ui_logger.lock() {
                        logger.warn(
                            "Please wait for all images to finish loading before editing the manifest.",
                            None,
                        );
                    }
                } else {
                    let mut editor = ManifestEditor::new(&mut archive);
                    // Extract the mutable reference before the closure to avoid borrow checker issues
                    let show_manifest_editor = &mut self.show_manifest_editor;

                    // Clone the logger Arc<Mutex<...>> before the closure
                    let logger = self.ui_logger.clone();

                    // Run the editor UI, ensuring no borrow of self inside the closure
                    let window_output = {
                        let editor_ref = &mut editor;
                        Window::new("Edit Manifest")
                            .open(show_manifest_editor)
                            .show(ctx, |ui| {
                                futures::executor::block_on(editor_ref.ui(ui, ctx))
                            })
                    };

                    if let Some(window_output) = window_output {
                        if let Some(Err(_)) = window_output.inner {
                            if let Ok(mut logger) = logger.lock() {
                                logger.error("Cannot edit Manifest", None);
                            }
                        }
                    }

                    // No further use of self here
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
                    modules::ui_debug(self, ui, ctx);
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    modules::ui_navigation(self, ui);
                });
            });
        });
    }

    pub fn display_notification_bar(&mut self, ctx: &Context) {
        /*if let Ok(logger) = self.ui_logger.lock() {
            if logger.message.is_some() {
                // You can adjust the position and width as needed
                egui::Area::new("notification_area".into())
                    .anchor(egui::Align2::LEFT_BOTTOM, [2.0, -24.0])
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        modules::ui_log_msg(self, ui);
                    });
            }
        }*/
        let ref ui_logger = self.ui_logger.lock();
        let (msg, kind) = if let Ok(logger) = ui_logger {
            if let Some((msg, kind)) = &logger.message {
                (msg.clone(), kind.clone())
            } else {
                return; // No message to display
            }
        } else {
            return; // Failed to lock logger
        };
        egui::Area::new("notification_area".into())
            .anchor(egui::Align2::LEFT_BOTTOM, [2.0, -24.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                modules::ui_log_msg(ui, &msg, kind);
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
                    });
                }
            });
        });
        self.display_notification_bar(ctx);
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
                if let (Some(l1), Some(l2)) = (&loaded1, &loaded2) {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, l1);
                    }
                    draw_dual_page(
                        ui,
                        l1,
                        Some(l2),
                        image_area,
                        self.zoom,
                        PAGE_MARGIN_SIZE as f32,
                        !self.right_to_left,
                        self.pan_offset,
                        &mut self.texture_cache,
                    );
                } else if let Some(l1) = &loaded1 {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, l1);
                    }
                    draw_single_page(
                        ui,
                        l1,
                        image_area,
                        self.zoom,
                        self.pan_offset,
                        &mut self.texture_cache,
                    );
                } else {
                    draw_spinner(ui, image_area);
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
