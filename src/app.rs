mod config;
mod image_cache;
mod ui;

use config::*;
use image_cache::*;
use ui::*;
use eframe::egui::{self, App, Vec2, Rect, pos2};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct CBZViewerApp {
    pub zip_path: PathBuf,
    pub image_lru: SharedImageCache,
    pub filenames: Vec<String>,
    pub current_page: usize,
    pub loading_thread: Option<std::thread::JoinHandle<()>>,
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub window_size: Vec2,
    pub drag_start: Option<egui::Pos2>,
    pub has_initialised_zoom: bool,
    pub double_page_mode: bool,
    pub right_to_left: bool,
    pub loading_pages: Arc<Mutex<HashSet<usize>>>,
}

impl CBZViewerApp {
    pub fn new(zip_path: PathBuf) -> Self {
        let file = std::fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                let lower = name.to_lowercase();
                if [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp"]
                    .iter()
                    .any(|ext| lower.ends_with(ext))
                {
                    names.push(name);
                }
            }
        }
        names.sort_by_key(|n| n.to_lowercase());

        Self {
            zip_path,
            filenames: names,
            image_lru: new_image_cache(CACHE_SIZE),
            current_page: 0,
            loading_thread: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            window_size: Vec2::ZERO,
            drag_start: None,
            has_initialised_zoom: false,
            double_page_mode: DEFAULT_DUAL_PAGE_MODE,
            right_to_left: DEFAULT_RIGHT_TO_LEFT,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn reset_zoom(&mut self, image_area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let zoom_x = image_area.width() / w as f32;
        let zoom_y = image_area.height() / h as f32;
        self.zoom = zoom_x.min(zoom_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
    }

    fn navigation_keys(&self) -> (egui::Key, egui::Key) {
        if READING_DIRECTION_AFFECTS_ARROWS && self.right_to_left {
            (egui::Key::ArrowLeft, egui::Key::ArrowRight)
        } else {
            (egui::Key::ArrowRight, egui::Key::ArrowLeft)
        }
    }
}

impl App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let total_pages = self.filenames.len();
        let input = ctx.input(|i| i.clone());

        // --- Top bar: navigation and legend ---
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(!self.double_page_mode, "Single Page").clicked() {
                    if self.double_page_mode {
                        self.double_page_mode = false;
                        self.current_page = self.current_page.min(total_pages.saturating_sub(1));
                        self.has_initialised_zoom = false;
                    }
                }
                if ui.selectable_label(self.double_page_mode, "Double Page").clicked() {
                    if !self.double_page_mode {
                        if self.current_page == 0 {
                            // Stay at 0
                        } else if self.current_page % 2 == 0 {
                            // Even page, stay
                        } else {
                            self.current_page -= 1;
                        }
                        self.double_page_mode = true;
                        self.has_initialised_zoom = false;
                    }
                }
                ui.separator();
                let dir_label = if self.right_to_left { "Right to Left" } else { "Left to Right" };
                if ui.button(dir_label).clicked() {
                    self.right_to_left = !self.right_to_left;
                }
                // Legend
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.double_page_mode {
                        let page1 = self.current_page;
                        if page1 == 0 {
                            ui.label(&self.filenames[0]);
                        } else {
                            let page2 = if page1 + 1 < total_pages { page1 + 1 } else { page1 };
                            if self.right_to_left {
                                ui.label(format!(
                                    "{} | {}",
                                    &self.filenames[page2],
                                    &self.filenames[page1]
                                ));
                            } else {
                                ui.label(format!(
                                    "{} | {}",
                                    &self.filenames[page1],
                                    &self.filenames[page2]
                                ));
                            }
                        }
                    } else {
                        ui.label(&self.filenames[self.current_page]);
                    }
                });
            });
        });

        // --- Navigation logic ---
        let (next_key, prev_key) = self.navigation_keys();
        if self.double_page_mode {
            if input.key_pressed(next_key) {
                if self.current_page == 0 && total_pages > 1 {
                    self.current_page = 1;
                } else if self.current_page + 2 < total_pages {
                    self.current_page += 2;
                } else if self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
                self.has_initialised_zoom = false;
            }
            if input.key_pressed(prev_key) {
                if self.current_page == 1 || self.current_page == 0 {
                    self.current_page = 0;
                } else if self.current_page >= 2 {
                    self.current_page -= 2;
                }
                self.has_initialised_zoom = false;
            }
        } else {
            if input.key_pressed(next_key) && self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.has_initialised_zoom = false;
            }
            if input.key_pressed(prev_key) && self.current_page > 0 {
                self.current_page -= 1;
                self.has_initialised_zoom = false;
            }
        }

        // --- Preload images for current view ---
        if self.double_page_mode {
            load_image_async(
                self.current_page,
                self.filenames.clone(),
                self.zip_path.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
            );
            if self.current_page != 0 && self.current_page + 1 < total_pages {
                load_image_async(
                    self.current_page + 1,
                    self.filenames.clone(),
                    self.zip_path.clone(),
                    self.image_lru.clone(),
                    self.loading_pages.clone(),
                );
            }
        } else {
            load_image_async(
                self.current_page,
                self.filenames.clone(),
                self.zip_path.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
            );
            if self.current_page + 1 < total_pages {
                load_image_async(
                    self.current_page + 1,
                    self.filenames.clone(),
                    self.zip_path.clone(),
                    self.image_lru.clone(),
                    self.loading_pages.clone(),
                );
            }
        }

        // --- Central image area ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let top_bar_height = 32.0;
            let bottom_bar_height = 32.0;
            let available_rect = ui.max_rect();
            let image_area = Rect::from_min_max(
                pos2(
                    available_rect.left(),
                    available_rect.top() + top_bar_height + 8.0,
                ),
                pos2(
                    available_rect.right(),
                    available_rect.bottom() - bottom_bar_height,
                ),
            );

            // Drag for panning
            let response = ui.allocate_rect(image_area, egui::Sense::drag());
            if response.drag_started() {
                self.drag_start = response.interact_pointer_pos();
            }
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some(start) = self.drag_start {
                        let delta = pos - start;
                        self.pan_offset += delta;
                        self.drag_start = Some(pos);
                    }
                }
            }
            if response.drag_released() {
                self.drag_start = None;
            }

            // --- Display images or spinner ---
            if self.double_page_mode {
                let page1 = self.current_page;
                if page1 == 0 {
                    let loaded = self.image_lru.lock().unwrap().get(&0).cloned();
                    if let Some(loaded) = loaded {
                        if !self.has_initialised_zoom {
                            self.reset_zoom(image_area, &loaded);
                        }
                        draw_single_page(ui, &loaded, image_area, self.zoom);
                    } else {
                        draw_spinner(ui, image_area);
                    }
                } else {
                    let page2 = if page1 + 1 < total_pages { page1 + 1 } else { usize::MAX };
                    let loaded1 = self.image_lru.lock().unwrap().get(&page1).cloned();
                    let loaded2 = if page2 != usize::MAX {
                        self.image_lru.lock().unwrap().get(&page2).cloned()
                    } else {
                        None
                    };

                    let both_loaded = loaded1.is_some() && (page2 == usize::MAX || loaded2.is_some());
                    if both_loaded {
                        if !self.has_initialised_zoom {
                            if let Some(ref loaded1) = loaded1 {
                                self.reset_zoom(image_area, loaded1);
                            }
                        }
                        let left_first = !self.right_to_left;
                        draw_dual_page(
                            ui,
                            loaded1.as_ref().unwrap(),
                            loaded2.as_ref(),
                            image_area,
                            self.zoom,
                            PAGE_MARGIN_SIZE as f32,
                            left_first,
                        );
                    } else {
                        draw_spinner(ui, image_area);
                    }
                }
            } else {
                let loaded = self.image_lru.lock().unwrap().get(&self.current_page).cloned();
                if let Some(loaded) = loaded {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, &loaded);
                    }
                    draw_single_page(ui, &loaded, image_area, self.zoom);
                } else {
                    draw_spinner(ui, image_area);
                }
            }

            // --- Bottom right: page number ---
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.label(format!(
                    "({}/{})",
                    self.current_page + 1,
                    self.filenames.len()
                ));
            });
        });
    }
}