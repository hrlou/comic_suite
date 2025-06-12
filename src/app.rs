use crate::archive::ImageArchive;
use crate::error::AppError;
use crate::config::*;
use crate::image_cache::{load_image_async, new_image_cache, LoadedPage, PageImage, SharedImageCache};
use crate::texture_cache::TextureCache;
use crate::ui::{draw_single_page, draw_dual_page, draw_spinner, show_menu_bar};

use eframe::{egui::{self, Vec2, Rect, Layout, pos2}, App};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct CBZViewerApp {
    pub archive_path: PathBuf,
    pub archive: Arc<Mutex<ImageArchive>>,
    pub image_lru: SharedImageCache,
    pub filenames: Vec<String>,
    pub current_page: usize,
    pub loading_thread: Option<std::thread::JoinHandle<()>>,
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub drag_start: Option<egui::Pos2>,
    pub has_initialised_zoom: bool,
    pub double_page_mode: bool,
    pub right_to_left: bool,
    pub loading_pages: Arc<Mutex<HashSet<usize>>>,
    pub texture_cache: TextureCache,
    pub on_open_comic: bool,
    pub on_open_folder: bool,
}

impl CBZViewerApp {
    pub fn new(archive_path: PathBuf) -> Result<Self, AppError> {
        let archive = Arc::new(Mutex::new(ImageArchive::process(&archive_path)?));
        let filenames = archive.lock().unwrap().image_names();

        Ok(Self {
            archive_path,
            archive,
            filenames,
            image_lru: new_image_cache(CACHE_SIZE),
            current_page: 0,
            loading_thread: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            drag_start: None,
            has_initialised_zoom: false,
            double_page_mode: DEFAULT_DUAL_PAGE_MODE,
            right_to_left: DEFAULT_RIGHT_TO_LEFT,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
            texture_cache: TextureCache::new(),
            on_open_comic: false,
            on_open_folder: false,
        })
    }

    fn reset_zoom(&mut self, image_area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let zoom_x = image_area.width() / w as f32;
        let zoom_y = image_area.height() / h as f32;
        self.zoom = zoom_x.min(zoom_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
        self.texture_cache.clear();
    }

    fn navigation_keys(&self) -> (egui::Key, egui::Key) {
        if READING_DIRECTION_AFFECTS_ARROWS && self.right_to_left {
            (egui::Key::ArrowLeft, egui::Key::ArrowRight)
        } else {
            (egui::Key::ArrowRight, egui::Key::ArrowLeft)
        }
    }

    /// Clamp pan so at least a corner of the image is visible
    fn clamp_pan(&mut self, image_size: (u32, u32), area: egui::Rect) {
        let border = BORDER_SIZE;
        
        let (img_w, img_h) = (image_size.0 as f32 * self.zoom, image_size.1 as f32 * self.zoom);
        let win_w = area.width();
        let win_h = area.height();
        
        // The image's center is at pan_offset = (0,0)
        // Allow panning, but always keep at least `border` pixels of the image visible
        
        let half_w = win_w / 2.0;
        let half_h = win_h / 2.0;
        let half_img_w = img_w / 2.0;
        let half_img_h = img_h / 2.0;
        
        let max_pan_x = (half_img_w - half_w + border).max(0.0);
        let max_pan_y = (half_img_h - half_h + border).max(0.0);
        
        // If the image is smaller than the viewport, allow panning within the border margin
        if img_w + border * 2.0 <= win_w {
            self.pan_offset.x = self.pan_offset.x.clamp(-(win_w/2.0 - half_img_w - border), win_w/2.0 - half_img_w - border);
        } else {
            self.pan_offset.x = self.pan_offset.x.clamp(-max_pan_x, max_pan_x);
        }
    
        if img_h + border * 2.0 <= win_h {
            self.pan_offset.y = self.pan_offset.y.clamp(-(win_h/2.0 - half_img_h - border), win_h/2.0 - half_img_h - border);
        } else {
            self.pan_offset.y = self.pan_offset.y.clamp(-max_pan_y, max_pan_y);
        }
    }
}

impl eframe::App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        show_menu_bar(ctx, &mut self.on_open_comic, &mut self.on_open_folder);

        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped_files.iter().find_map(|f| f.path.clone()) {
            let path = file.to_path_buf();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "cbz" || ext == "zip" || path.is_dir() {
                match CBZViewerApp::new(path) {
                    Ok(new_app) => {
                        *self = new_app;
                    }
                    Err(e) => {
                        // You can show a dialog, toast, or log the error here
                        log::error!("Failed to open archive: {e}");
                        // Optionally, store the error in the app state to display in the UI
                    }
                }
            }
        }

        if self.on_open_comic {
            self.on_open_comic = false;
            if let Some(path) = rfd::FileDialog::new().add_filter("Comic Book Archive", &["cbz", "zip"]).pick_file() {
                // handle opening comic
            }
        }
        if self.on_open_folder {
            self.on_open_folder = false;
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                // handle opening folder
            }
        }

        let total_pages = self.filenames.len();
        let input = ctx.input(|i| i.clone());

        // --- Overlay UI: always on top ---
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let direction_label = if self.right_to_left { "R <- L" } else { "L -> R" };
                if ui.button(direction_label)
                    .on_hover_text("Reading direction")
                    .clicked()
                {
                    self.right_to_left = !self.right_to_left;
                    self.texture_cache.clear();
                }

                if ui.selectable_label(self.double_page_mode, "Dual")
                    .on_hover_text("Show two pages at once, cover page will be excluded")
                    .clicked()
                {
                    if self.double_page_mode {
                        self.double_page_mode = false;
                        self.current_page = self.current_page.min(total_pages.saturating_sub(1));
                        self.has_initialised_zoom = false;
                        self.texture_cache.clear();
                    } else { 
                        if self.current_page > 0 && self.current_page % 2 != 0 {
                            self.current_page -= 1;
                        }
                        self.double_page_mode = true;
                        self.has_initialised_zoom = false;
                        self.texture_cache.clear();
                    }
                }

                if self.double_page_mode {
                    if ui.button("Bump")
                        .on_hover_text("Bump over a single page, use this if there is misalignment")
                        .clicked()
                    {
                        let total_pages = self.filenames.len();
                        if self.current_page + 1 < total_pages {
                            self.current_page += 1;
                            self.has_initialised_zoom = false;
                            self.texture_cache.clear();
                        }
                    }
                }
                
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let file_label = if self.double_page_mode && self.current_page != 0 {
                        let left = self.current_page;
                        let right = (self.current_page + 1).min(total_pages.saturating_sub(1));
                        if self.right_to_left {
                            format!(
                                "{} | {}",
                                self.filenames.get(right).unwrap_or(&String::from("")),
                                self.filenames.get(left).unwrap_or(&String::from(""))
                            )
                        } else {
                            format!(
                                "{} | {}",
                                self.filenames.get(left).unwrap_or(&String::from("")),
                                self.filenames.get(right).unwrap_or(&String::from(""))
                            )
                        }
                    } else {
                        self.filenames
                            .get(self.current_page)
                            .cloned()
                            .unwrap_or_else(|| String::from(""))
                    };
                    ui.label(file_label);
                });
            });
        });

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.zoom, 0.05..=10.0));
                if ui.button("Reset Zoom").clicked() {
                    self.zoom = 1.0;
                    self.pan_offset = Vec2::ZERO;
                    self.has_initialised_zoom = false;
                    self.texture_cache.clear();
                }
                ui.separator();
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    // Add your controls here, they will appear on the right side
                    if ui.button("Next").clicked() {
                        self.current_page = (self.current_page + 1).min(self.filenames.len().saturating_sub(1));
                    }
                    if ui.button("Prev").clicked() {
                        self.current_page = self.current_page.saturating_sub(1);
                    }
                    let page_label = if self.double_page_mode && self.current_page != 0 {
                        let left = self.current_page;
                        let right = (self.current_page + 1).min(total_pages.saturating_sub(1));
                        if self.right_to_left {
                            format!("Page ({},{})/{}", right + 1, left + 1, total_pages)
                        } else {
                            format!("Page ({},{})/{}", left + 1, right + 1, total_pages)
                        }
                    } else {
                        format!("Page {}/{}", self.current_page + 1, total_pages)
                    };
                    ui.label(page_label);
                });
            });
        });

        // --- Navigation logic (arrow keys) ---
        let (next_key, prev_key) = self.navigation_keys();
        let next_pressed = input.key_pressed(next_key) || input.key_pressed(egui::Key::ArrowDown);
        let prev_pressed = input.key_pressed(prev_key) || input.key_pressed(egui::Key::ArrowUp);

        if self.double_page_mode {
            if next_pressed {
                if self.current_page == 0 && total_pages > 1 {
                    self.current_page = 1;
                } else if self.current_page + 2 < total_pages {
                    self.current_page += 2;
                } else if self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
                self.has_initialised_zoom = false;
                self.texture_cache.clear();
            }
            if prev_pressed {
                if self.current_page == 1 || self.current_page == 0 {
                    self.current_page = 0;
                } else if self.current_page >= 2 {
                    self.current_page -= 2;
                }
                self.has_initialised_zoom = false;
                self.texture_cache.clear();
            }
        } else {
            if next_pressed && self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.has_initialised_zoom = false;
                self.texture_cache.clear();
            }
            if prev_pressed && self.current_page > 0 {
                self.current_page -= 1;
                self.has_initialised_zoom = false;
                self.texture_cache.clear();
            }
        }

        // --- Zoom with mouse wheel ---
        if input.raw_scroll_delta.y != 0.0 {
            let zoom_factor = 1.1_f32.powf(input.raw_scroll_delta.y / 10.0);
            self.zoom = (self.zoom * zoom_factor).clamp(0.05, 10.0);
            self.texture_cache.clear();
        }

        // --- Preload images for current view (and next page for smooth navigation) ---
        if self.double_page_mode {
            let _ = load_image_async(
                self.current_page,
                self.filenames.clone(),
                self.archive.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
            );
            if self.current_page != 0 && self.current_page + 1 < total_pages {
                let _ = load_image_async(
                    self.current_page + 1,
                    self.filenames.clone(),
                    self.archive.clone(),
                    self.image_lru.clone(),
                    self.loading_pages.clone(),
                );
            }
        } else {
            let _ = load_image_async(
                self.current_page,
                self.filenames.clone(),
                self.archive.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
            );
            if self.current_page + 1 < total_pages {
                let _ = load_image_async(
                    self.current_page + 1,
                    self.filenames.clone(),
                    self.archive.clone(),
                    self.image_lru.clone(),
                    self.loading_pages.clone(),
                );
            }
        }

        // --- Central image area (main viewer) ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_rect = ui.max_rect();
            //let image_area = available_rect;
            // let image_area = ui.available_size();
            let image_area = ui.available_rect_before_wrap();

            // --- Pan (drag to move image) ---
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
                        self.texture_cache.clear();
                    }
                }
            }
            if response.drag_released() {
                self.drag_start = None;
            }

            // --- Display images or spinner ---
            if self.double_page_mode {
                let page1 = self.current_page;
                let page2 = if page1 + 1 < total_pages { page1 + 1 } else { usize::MAX };
                let loaded1 = self.image_lru.lock().unwrap().get(&page1).cloned();
                let loaded2 = if page2 != usize::MAX {
                    self.image_lru.lock().unwrap().get(&page2).cloned()
                } else {
                    None
                };

                if let (Some(loaded1), Some(loaded2)) = (&loaded1, &loaded2) {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, loaded1);
                    }
                    let left_first = !self.right_to_left;
                    draw_dual_page(
                        ui,
                        loaded1,
                        Some(loaded2),
                        image_area,
                        self.zoom,
                        PAGE_MARGIN_SIZE as f32,
                        left_first,
                        self.pan_offset,
                        &mut self.texture_cache,
                    );
                    // Clamp pan so at least a corner of the image is visible
                    let (w1, h1) = loaded1.image.dimensions();
                    let (w2, h2) = loaded2.image.dimensions();
                    let total_width = w1 + w2;
                    let max_height = h1.max(h2);
                    self.clamp_pan((total_width, max_height), image_area);
                } else if let Some(loaded1) = &loaded1 {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, loaded1);
                    }
                    draw_single_page(ui, loaded1, image_area, self.zoom, self.pan_offset, &mut self.texture_cache);
                    self.clamp_pan(loaded1.image.dimensions(), image_area);
                } else {
                    draw_spinner(ui, image_area);
                }
            } else {
                let loaded = self.image_lru.lock().unwrap().get(&self.current_page).cloned();
                if let Some(loaded) = loaded {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, &loaded);
                    }
                    draw_single_page(ui, &loaded, image_area, self.zoom, self.pan_offset, &mut self.texture_cache);
                    self.clamp_pan(loaded.image.dimensions(), image_area);
                } else {
                    draw_spinner(ui, image_area);
                }
            }
        });
    }
}