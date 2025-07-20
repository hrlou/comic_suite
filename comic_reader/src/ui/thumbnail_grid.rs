use crate::prelude::*;

impl CBZViewerApp {
    pub fn display_thumbnail_grid(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let columns = 5;
            let border = 8.0;
            let edge_margin = 24.0;

            ui.add_space(edge_margin); // Top margin

            let thumb_size = ((available_width - (columns as f32 + 1.0) * border - 2.0 * edge_margin) / columns as f32)
                .floor() as u32;

            let total = self.total_pages;

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut idx = 0;
                let mut closed_by_user = false;

                while idx < total {
                    ui.horizontal(|ui| {
                        ui.add_space(edge_margin); // Left margin
                        for col in 0..columns {
                            let page_idx = idx + col;
                            if page_idx >= total {
                                break;
                            }
                            let rect = ui.allocate_space(egui::vec2(thumb_size as f32, thumb_size as f32));
                            let resp = {
                                // Only generate if visible and not already cached
                                if ui.is_rect_visible(rect.1) {
                                    if !self.thumbnail_cache.contains_key(&page_idx) {
                                        // Try LRU first
                                        let mut image_lru = self.image_lru.lock().unwrap();
                                        if let Some(loaded) = image_lru.get(&page_idx) {
                                            if let PageImage::Static(img) = &loaded.image {
                                                let thumb = img.thumbnail(thumb_size, thumb_size);
                                                self.thumbnail_cache.insert(page_idx, thumb);
                                            }
                                        } else if let Some(mut archive) = self.archive.as_ref().and_then(|a| a.lock().ok()) {
                                            if let Some(filename) = archive.list_images().get(page_idx) {
                                                if let Ok(buf) = archive.read_image_by_index(page_idx) {
                                                    if let Ok(img) = image::load_from_memory(&buf) {
                                                        let thumb = img.thumbnail(thumb_size, thumb_size);
                                                        self.thumbnail_cache.insert(page_idx, thumb);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(img) = self.thumbnail_cache.get(&page_idx) {
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                        [img.width() as usize, img.height() as usize],
                                        &img.to_rgba8(),
                                    );
                                    let tex = ui.ctx().load_texture(
                                        format!("thumb_{}", page_idx),
                                        color_img,
                                        egui::TextureOptions::default(),
                                    );
                                    // Highlight border on hover
                                    let resp = ui.put(
                                        rect.1,
                                        egui::ImageButton::new(&tex)
                                            .frame(false)
                                            .sense(egui::Sense::click()),
                                    );
                                    // Draw highlight if hovered
                                    if resp.hovered() {
                                        let stroke = egui::Stroke::new(3.0, egui::Color32::LIGHT_BLUE);
                                        ui.painter().rect_stroke(rect.1, 6.0, stroke, egui::StrokeKind::Outside);
                                    }
                                    // Draw index at bottom right
                                    let index_text = format!("{}", page_idx + 1);
                                    let text_pos = rect.1.right_bottom() - egui::vec2(6.0, 6.0);
                                    // Draw background rectangle for text
                                    let galley = ui.painter().layout_no_wrap(
                                        index_text.clone(),
                                        egui::FontId::proportional(14.0),
                                        egui::Color32::WHITE,
                                    );
                                    let rect_bg = egui::Rect::from_min_size(
                                        text_pos - egui::vec2(galley.size().x, galley.size().y),
                                        galley.size(),
                                    );
                                    ui.painter().rect_filled(
                                        rect_bg,
                                        2.0,
                                        egui::Color32::from_black_alpha(160),
                                    );
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::RIGHT_BOTTOM,
                                        index_text,
                                        egui::FontId::proportional(14.0),
                                        egui::Color32::WHITE,
                                    );
                                    resp
                                } else {
                                    ui.put(rect.1, egui::Label::new("..."))
                                }
                            };

                            if resp.clicked() {
                                self.current_page = page_idx;
                                closed_by_user = true;
                                self.on_page_changed();
                            }
                            ui.add_space(border);
                        }
                        ui.add_space(edge_margin); // Right margin
                    });
                    idx += columns;
                    ui.add_space(border);
                }

                ui.add_space(edge_margin); // Bottom margin

                if closed_by_user {
                    self.show_thumbnail_grid = false;
                }
            });
        });
    }
}