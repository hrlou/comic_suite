use crate::prelude::*;

impl CBZViewerApp {
    pub fn display_thumbnail_grid(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let columns = 5;
            let border = 8.0;
            let edge_margin = 24.0;

            ui.add_space(edge_margin); // Top margin

            let thumb_size =
                ((available_width - (columns as f32 + 1.0) * border - 2.0 * edge_margin)
                    / columns as f32)
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
                            let rect =
                                ui.allocate_space(egui::vec2(thumb_size as f32, thumb_size as f32));
                            let resp = {
                                // Only generate if visible and not already cached
                                if ui.is_rect_visible(rect.1)
                                    && !self.thumbnail_cache.lock().unwrap().contains_key(&page_idx)
                                {
                                    let archive = self.archive.clone();
                                    let cache = self.thumbnail_cache.clone();
                                    let semaphore = self.thumb_semaphore.clone();
                                    let page_idx_copy = page_idx;
                                    let thumb_size_copy = thumb_size;
                                    let is_web_archive = self.is_web_archive;
                                    let image_lru = self.image_lru.clone();

                                    // Clone the filename while holding the lock, then drop the guard before spawn
                                    let filename = {
                                        let archive_ref = archive.as_ref().unwrap();
                                        let guard = archive_ref.lock().unwrap();
                                        guard.list_images().get(page_idx_copy).cloned()
                                    };

                                    if let Some(filename) = filename {
                                        tokio::spawn(async move {
                                            let _permit = semaphore.acquire().await.unwrap();

                                            // Lock the archive, get the image bytes synchronously, then drop the lock before .await
                                            let img_data = {
                                                let archive_ref = archive.as_ref().unwrap();
                                                let mut guard = archive_ref.lock().unwrap();
                                                guard.backend.read_image_by_name_sync(&filename)
                                            };

                                            if let Ok(img_data) = img_data {
                                                // Detect GIF by magic bytes
                                                let is_gif = img_data.starts_with(b"GIF87a")
                                                    || img_data.starts_with(b"GIF89a");
                                                let img_result = if is_gif {
                                                    use image::AnimationDecoder;
                                                    use image::codecs::gif::GifDecoder;
                                                    use std::io::Cursor;
                                                    let cursor = Cursor::new(&*img_data);
                                                    if let Ok(decoder) = GifDecoder::new(cursor) {
                                                        if let Ok(frames) =
                                                            decoder.into_frames().collect_frames()
                                                        {
                                                            if let Some(frame) = frames.get(0) {
                                                                Some(image::DynamicImage::from(
                                                                    frame.clone().into_buffer(),
                                                                ))
                                                            } else {
                                                                None
                                                            }
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    image::load_from_memory(&img_data).ok()
                                                };
                                                let filename_clone = filename.clone();
                                                let is_web_archive = is_web_archive;
                                                let image_lru = image_lru.clone();
                                                if let Some(img) = img_result {
                                                    // If this is a webarchive, add to LRU cache
                                                    if is_web_archive {
                                                        use crate::cache::image_cache::PageImage;
                                                        let mut lru = image_lru.lock().unwrap();
                                                        lru.put(page_idx_copy, crate::cache::image_cache::LoadedPage {
                                                            image: PageImage::Static(img.clone()),
                                                            filename: filename_clone,
                                                            index: page_idx_copy.clone(),
                                                        });
                                                    }
                                                    // Always resize to thumbnail size before caching
                                                    let thumb = img.resize_exact(
                                                        thumb_size_copy,
                                                        thumb_size_copy,
                                                        image::imageops::FilterType::Lanczos3,
                                                    );
                                                    let mut cache_guard = cache.lock().unwrap();
                                                    cache_guard.insert(page_idx_copy, thumb);
                                                }
                                            }
                                        });
                                    }
                                } else {
                                    // Before spawning the async loader:
                                    if !self.thumbnail_cache.lock().unwrap().contains_key(&page_idx) {
                                        // Try LRU cache first
                                        if let Some(lru_entry) = self.image_lru.lock().unwrap().get(&page_idx) {
                                            if let PageImage::Static(ref dyn_img) = lru_entry.image {
                                                let thumb = dyn_img.resize_exact(
                                                    thumb_size,
                                                    thumb_size,
                                                    image::imageops::FilterType::Lanczos3,
                                                );
                                                self.thumbnail_cache.lock().unwrap().insert(page_idx, thumb);
                                            } else {
                                                // If it's not a static image, skip or handle other variants as needed
                                            }
                                        } else {
                                            let archive = self.archive.clone();
                                            let cache = self.thumbnail_cache.clone();
                                            let semaphore = self.thumb_semaphore.clone();
                                            let page_idx_copy = page_idx;
                                            let thumb_size_copy = thumb_size;

                                            // Clone the filename while holding the lock, then drop the guard before spawn
                                            let filename = {
                                                let archive_ref = archive.as_ref().unwrap();
                                                let guard = archive_ref.lock().unwrap();
                                                guard.list_images().get(page_idx_copy).cloned()
                                            };

                                            if let Some(filename) = filename {
                                                tokio::spawn(async move {
                                                    let _permit = semaphore.acquire().await.unwrap();

                                                    // Lock the archive, get the image bytes synchronously, then drop the lock before .await
                                                    let img_data = {
                                                        let archive_ref = archive.as_ref().unwrap();
                                                        let mut guard = archive_ref.lock().unwrap();
                                                        guard.backend.read_image_by_name_sync(&filename)
                                                    };

                                                    if let Ok(img_data) = img_data {
                                                        // Detect GIF by magic bytes
                                                        let is_gif = img_data.starts_with(b"GIF87a")
                                                            || img_data.starts_with(b"GIF89a");
                                                        let img_result = if is_gif {
                                                            use image::AnimationDecoder;
                                                            use image::codecs::gif::GifDecoder;
                                                            use std::io::Cursor;
                                                            let cursor = Cursor::new(&*img_data);
                                                            if let Ok(decoder) = GifDecoder::new(cursor) {
                                                                if let Ok(frames) =
                                                                    decoder.into_frames().collect_frames()
                                                                {
                                                                    if let Some(frame) = frames.get(0) {
                                                                        Some(image::DynamicImage::from(
                                                                            frame.clone().into_buffer(),
                                                                        ))
                                                                    } else {
                                                                        None
                                                                    }
                                                                } else {
                                                                    None
                                                                }
                                                            } else {
                                                                None
                                                            }
                                                        } else {
                                                            image::load_from_memory(&img_data).ok()
                                                        };

                                                        if let Some(img) = img_result {
                                                            // Always resize to thumbnail size before caching
                                                            let thumb = img.resize_exact(
                                                                thumb_size_copy,
                                                                thumb_size_copy,
                                                                image::imageops::FilterType::Lanczos3,
                                                            );
                                                            let mut cache_guard = cache.lock().unwrap();
                                                            cache_guard.insert(page_idx_copy, thumb);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }

                                // Always show spinner until the thumbnail is loaded
                                if let Some(img) = self.thumbnail_cache.lock().unwrap().get(&page_idx) {
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
                                        egui::ImageButton::new(
                                            egui::Image::from_texture(&tex)
                                                .fit_to_exact_size(egui::vec2(img.width() as f32, img.height() as f32))
                                        )
                                        .frame(false)
                                        .sense(egui::Sense::click()),
                                    );
                                    if resp.hovered() {
                                        let stroke =
                                            egui::Stroke::new(3.0, egui::Color32::LIGHT_BLUE);
                                        ui.painter().rect_stroke(
                                            rect.1,
                                            6.0,
                                            stroke,
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                    // Draw index at bottom right
                                    let index_text = format!("{}", page_idx + 1);
                                    let text_pos = rect.1.right_bottom() - egui::vec2(6.0, 6.0);
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
                                    if resp.clicked() {
                                        self.current_page = page_idx;
                                        closed_by_user = true;
                                        // Defer on_page_changed to after the closure to avoid borrowing issues
                                        // self.on_page_changed();
                                    }
                                    ui.add_space(border);
                                } else {
                                    ui.put(rect.1, egui::Spinner::new());
                                    ui.add_space(border);
                                }
                            };
                        }
                        ui.add_space(edge_margin); // Right margin
                    });
                    idx += columns;
                    ui.add_space(border);
                }
                if closed_by_user {
                    self.show_thumbnail_grid = false;
                    self.on_page_changed();
                }
            });
        });
    }
}
