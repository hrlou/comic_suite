use crate::prelude::*;

impl CBZViewerApp {
    pub fn display_thumbnail_grid(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let columns = 5;
            let border = 8.0;
            let edge_margin = 24.0; // Extra margin for edges, especially top

            ui.add_space(edge_margin); // Top margin

            let thumb_size = ((available_width - (columns as f32 + 1.0) * border - 2.0 * edge_margin) / columns as f32)
                .floor() as u32;

            // Generate thumbnails if needed or if size changed
            if self.thumbnail_cache.is_empty()
                || self.thumbnail_cache.values().next().map(|img| img.width()) != Some(thumb_size)
            {
                self.generate_all_thumbnails((thumb_size, thumb_size));
            }

            let total = self.total_pages;
            let rows = (total + columns - 1) / columns;

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
                            let resp = if let Some(img) = self.thumbnail_cache.get(&page_idx) {
                                // Only render thumbnails that are visible in the scroll area
                                if ui.is_rect_visible(rect.1) {
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                        [img.width() as usize, img.height() as usize],
                                        &img.to_rgba8(),
                                    );
                                    let tex = ui.ctx().load_texture(
                                        format!("thumb_{}", page_idx),
                                        color_img,
                                        egui::TextureOptions::default(),
                                    );
                                    ui.put(
                                        rect.1,
                                        egui::ImageButton::new(&tex)
                                            .frame(false)
                                            .sense(egui::Sense::click()),
                                    )
                                } else {
                                    // Placeholder for offscreen thumbnails
                                    ui.put(rect.1, egui::Label::new(""))
                                }
                            } else {
                                ui.put(rect.1, egui::Label::new("..."))
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

    pub fn generate_all_thumbnails(&mut self, thumb_size: (u32, u32)) {
        let mut cache = std::collections::HashMap::new();
        let mut image_lru = self.image_lru.lock().unwrap();
        for idx in 0..self.total_pages {
            // Try to get from LRU cache first
            if let Some(loaded) = image_lru.get(&idx) {
                if let PageImage::Static(img) = &loaded.image {
                    let thumb = img.thumbnail(thumb_size.0, thumb_size.1);
                    cache.insert(idx, thumb);
                    continue;
                }
            }
            // If not in LRU, load from archive
            if let Some(mut archive) = self.archive.as_ref().and_then(|a| a.lock().ok()) {
                if let Some(filename) = archive.list_images().get(idx) {
                    if let Ok(buf) = archive.read_image_by_index(idx) {
                        if let Ok(img) = image::load_from_memory(&buf) {
                            let thumb = img.thumbnail(thumb_size.0, thumb_size.1);
                            cache.insert(idx, thumb);
                        }
                    }
                }
            }
        }
        self.thumbnail_cache = cache;
    }
}