// Hide the console window on Windows
#![windows_subsystem = "windows"]

use std::{
    collections::{HashSet},
    fs::File,
    io::Read,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use eframe::egui::{
    pos2, Align, ColorImage, Image, Layout, Rect, TextureFilter, TextureOptions, Vec2, Spinner,
};
use eframe::{egui, App, NativeOptions};
use lru::LruCache;
use image::{DynamicImage, GenericImageView};
use zip::ZipArchive;

// Constants for initial window size
const WIN_WIDTH: f32 = 720.0;
const WIN_HEIGHT: f32 = 1080.0;
const CACHE_SIZE: usize = 20; // Number of images to cache
const PAGE_MARGIN_SIZE: usize = 16; // Margin in pixels between pages
const DEFAULT_DUAL_PAGE_MODE: bool = false;
const DEFAULT_RIGHT_TO_LEFT: bool = false;
const READING_DIRECTION_AFFECTS_ARROWS: bool = true;

#[derive(Clone)]
struct LoadedPage {
    index: usize,
    filename: String,
    image: DynamicImage,
}

// Main application state
struct CBZViewerApp {
    zip_path: PathBuf,
    image_lru: Arc<Mutex<LruCache<usize, LoadedPage>>>,
    filenames: Vec<String>,
    current_page: usize,
    loading_thread: Option<thread::JoinHandle<()>>,
    zoom: f32,
    pan_offset: Vec2,
    window_size: Vec2,
    drag_start: Option<egui::Pos2>,
    has_initialised_zoom: bool,
    double_page_mode: bool,
    right_to_left: bool,
    loading_pages: Arc<Mutex<HashSet<usize>>>,
    // Texture caches
    single_texture_cache: Option<(usize, egui::TextureHandle)>,
    dual_texture_cache: Option<((usize, egui::TextureHandle), Option<(usize, egui::TextureHandle)>)>,
}

impl CBZViewerApp {
    fn new(zip_path: PathBuf) -> Self {
        let file = File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
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
            image_lru: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()))),
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
            single_texture_cache: None,
            dual_texture_cache: None,
        }
    }

    /// Spawn background thread to load and decode image
    fn load_image_async(&mut self, page: usize) {
        {
            // When starting a load, set progress to 0.0
            let mut loading = self.loading_pages.lock().unwrap();
            if loading.contains(&page) {
                return;
            }
            loading.insert(page);
        }

        // Check LRU cache first
        if self.image_lru.lock().unwrap().get(&page).is_some() {
            return;
        }

        let filenames = self.filenames.clone();
        let zip_path = self.zip_path.clone();
        let image_lru = Arc::clone(&self.image_lru);
        let loading_pages = Arc::clone(&self.loading_pages);

        self.loading_thread = Some(thread::spawn(move || {
            let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
            let mut file = archive.by_name(&filenames[page]).unwrap();
            let size = file.size();

            let mut buf = Vec::with_capacity(size as usize);
            let mut tmp = [0u8; 8192];

            while let Ok(n) = file.read(&mut tmp) {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }

            // After reading all bytes and decoding:
            let img = image::load_from_memory(&buf).unwrap();
            let loaded_page = LoadedPage {
                index: page,
                filename: filenames[page].clone(),
                image: img,
            };
            image_lru.lock().unwrap().put(page, loaded_page);

            // Remove from loading set
            loading_pages.lock().unwrap().remove(&page);
        }));
    }

    fn reset_zoom(&mut self, image_area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let zoom_x = image_area.width() / w as f32;
        let zoom_y = image_area.height() / h as f32;
        self.zoom = zoom_x.min(zoom_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
    }
}

impl App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let total_pages = self.filenames.len();

        // --- Navigation logic ---
        let input = ctx.input(|i| i.clone());

        // Mode switching UI
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut mode_switched = false;
                if ui.selectable_label(!self.double_page_mode, "Single Page").clicked() {
                    if self.double_page_mode {
                        self.double_page_mode = false;
                        self.current_page = self.current_page.min(total_pages.saturating_sub(1));
                        self.single_texture_cache = None;
                        self.dual_texture_cache = None;
                        self.has_initialised_zoom = false;
                        mode_switched = true;
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
                        self.single_texture_cache = None;
                        self.dual_texture_cache = None;
                        self.has_initialised_zoom = false;
                        mode_switched = true;
                    }
                }
                ui.separator();
                let dir_label = if self.right_to_left { "Right to Left" } else { "Left to Right" };
                if ui.button(dir_label).clicked() {
                    self.right_to_left = !self.right_to_left;
                }
                // Spacer to push filenames to the right
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

        // Navigation
        if self.double_page_mode {
            let (next_key, prev_key) = if READING_DIRECTION_AFFECTS_ARROWS && self.right_to_left {
                (egui::Key::ArrowLeft, egui::Key::ArrowRight)
            } else {
                (egui::Key::ArrowRight, egui::Key::ArrowLeft)
            };

            if input.key_pressed(next_key) {
                if self.current_page == 0 && total_pages > 1 {
                    self.current_page = 1;
                } else if self.current_page + 2 < total_pages {
                    self.current_page += 2;
                } else if self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
                self.dual_texture_cache = None;
                self.single_texture_cache = None;
                self.has_initialised_zoom = false;
            }
            if input.key_pressed(prev_key) {
                if self.current_page == 1 || self.current_page == 0 {
                    self.current_page = 0;
                } else if self.current_page >= 2 {
                    self.current_page -= 2;
                }
                self.dual_texture_cache = None;
                self.single_texture_cache = None;
                self.has_initialised_zoom = false;
            }
        } else {
            let (next_key, prev_key) = if READING_DIRECTION_AFFECTS_ARROWS && self.right_to_left {
                (egui::Key::ArrowLeft, egui::Key::ArrowRight)
            } else {
                (egui::Key::ArrowRight, egui::Key::ArrowLeft)
            };

            if input.key_pressed(next_key) && self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.single_texture_cache = None;
                self.dual_texture_cache = None;
                self.has_initialised_zoom = false;
            }
            if input.key_pressed(prev_key) && self.current_page > 0 {
                self.current_page -= 1;
                self.single_texture_cache = None;
                self.dual_texture_cache = None;
                self.has_initialised_zoom = false;
            }
        }

        // Preload images for current view
        if self.double_page_mode {
            self.load_image_async(self.current_page);
            if self.current_page != 0 && self.current_page + 1 < total_pages {
                self.load_image_async(self.current_page + 1);
            }
        } else {
            self.load_image_async(self.current_page);
            if self.current_page + 1 < total_pages {
                self.load_image_async(self.current_page + 1); // Preload next page
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

            // --- Display images ---
            if self.double_page_mode {
                let page1 = self.current_page;
                let total_pages = self.filenames.len();

                if page1 == 0 {
                    // Cover: show only if loaded
                    let loaded = self.image_lru.lock().unwrap().get(&0).cloned();
                    if let Some(loaded) = loaded {
                        if !self.has_initialised_zoom {
                            self.reset_zoom(image_area, &loaded);
                        }
                        // Texture cache
                        let needs_update = self.single_texture_cache.as_ref().map_or(true, |(idx, _)| *idx != 0);
                        if needs_update {
                            let (w, h) = loaded.image.dimensions();
                            let color_img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &loaded.image.to_rgba8());
                            let handle = ui.ctx().load_texture(format!("tex{}", loaded.index), color_img, TextureOptions::default());
                            self.single_texture_cache = Some((0, handle));
                        }
                        if let Some((_, handle)) = &self.single_texture_cache {
                            let (w, h) = loaded.image.dimensions();
                            let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);
                            let center = image_area.center();
                            let rect = Rect::from_center_size(center, disp_size);
                            ui.allocate_ui_at_rect(rect, |ui| {
                                ui.add(Image::from_texture(handle).fit_to_exact_size(disp_size));
                            });
                        }
                    } else {
                        // Show spinner
                        let spinner_size = 48.0;
                        let spinner_rect = egui::Rect::from_center_size(
                            image_area.center(),
                            Vec2::splat(spinner_size),
                        );
                        ui.allocate_ui_at_rect(spinner_rect, |ui| {
                            ui.add(egui::Spinner::new().size(spinner_size).color(egui::Color32::WHITE));
                        });
                    }
                } else {
                    let page2 = if page1 + 1 < total_pages { page1 + 1 } else { usize::MAX };
                    let loaded1 = self.image_lru.lock().unwrap().get(&page1).cloned();
                    let loaded2 = if page2 != usize::MAX {
                        self.image_lru.lock().unwrap().get(&page2).cloned()
                    } else {
                        None
                    };

                    // Only display if BOTH are loaded (unless last page and only one remains)
                    let show_both = loaded1.is_some() && (page2 == usize::MAX || loaded2.is_some());
                    if show_both {
                        // Zoom initialization
                        if !self.has_initialised_zoom {
                            if let Some(loaded1) = loaded1.as_ref() {
                                self.reset_zoom(image_area, loaded1);
                            }
                        }

                        // Texture cache
                        let needs_update = self.dual_texture_cache.as_ref().map_or(true, |((idx1, _), opt2)| {
                            let idx2 = loaded2.as_ref().map(|l| l.index);
                            *idx1 != page1 || opt2.as_ref().map(|(i, _)| *i) != loaded2.as_ref().map(|l| l.index)
                        });
                        if needs_update {
                            if let Some(loaded1) = loaded1.as_ref() {
                                let (w1, h1) = loaded1.image.dimensions();
                                let color_img1 = ColorImage::from_rgba_unmultiplied([w1 as usize, h1 as usize], &loaded1.image.to_rgba8());
                                let handle1 = ui.ctx().load_texture(format!("tex{}", loaded1.index), color_img1, TextureOptions::default());
                                let handle2 = if let Some(loaded2) = loaded2.as_ref() {
                                    let (w2, h2) = loaded2.image.dimensions();
                                    let color_img2 = ColorImage::from_rgba_unmultiplied([w2 as usize, h2 as usize], &loaded2.image.to_rgba8());
                                    Some((loaded2.index, ui.ctx().load_texture(format!("tex{}", loaded2.index), color_img2, TextureOptions::default())))
                                } else {
                                    None
                                };
                                self.dual_texture_cache = Some(((loaded1.index, handle1), handle2));
                            }
                        }

                        if let Some(((idx1, handle1), opt2)) = &self.dual_texture_cache {
                            if let Some(loaded1) = loaded1.as_ref() {
                                let (w1, h1) = loaded1.image.dimensions();
                                let disp_size1 = Vec2::new(w1 as f32 * self.zoom, h1 as f32 * self.zoom);

                                if let Some((idx2, handle2)) = opt2 {
                                    if let Some(loaded2) = loaded2.as_ref() {
                                        let (w2, h2) = loaded2.image.dimensions();
                                        let disp_size2 = Vec2::new(w2 as f32 * self.zoom, h2 as f32 * self.zoom);

                                        let margin = PAGE_MARGIN_SIZE as f32;
                                        let total_width = disp_size1.x + disp_size2.x + margin;
                                        let center = image_area.center();
                                        let left_start = center.x - total_width / 2.0;

                                        if self.right_to_left {
                                            // Show page2 on the left, page1 on the right
                                            let rect2 = Rect::from_min_size(
                                                pos2(left_start, center.y - disp_size2.y / 2.0),
                                                disp_size2,
                                            );
                                            let rect1 = Rect::from_min_size(
                                                pos2(left_start + disp_size2.x + margin, center.y - disp_size1.y / 2.0),
                                                disp_size1,
                                            );
                                            ui.allocate_ui_at_rect(rect2, |ui| {
                                                ui.add(Image::from_texture(handle2).fit_to_exact_size(disp_size2));
                                            });
                                            ui.allocate_ui_at_rect(rect1, |ui| {
                                                ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                                            });
                                        } else {
                                            // Show page1 on the left, page2 on the right
                                            let rect1 = Rect::from_min_size(
                                                pos2(left_start, center.y - disp_size1.y / 2.0),
                                                disp_size1,
                                            );
                                            let rect2 = Rect::from_min_size(
                                                pos2(left_start + disp_size1.x + margin, center.y - disp_size2.y / 2.0),
                                                disp_size2,
                                            );
                                            ui.allocate_ui_at_rect(rect1, |ui| {
                                                ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                                            });
                                            ui.allocate_ui_at_rect(rect2, |ui| {
                                                ui.add(Image::from_texture(handle2).fit_to_exact_size(disp_size2));
                                            });
                                        }
                                    }
                                } else {
                                    // Only one page (last page, odd count)
                                    let center = image_area.center();
                                    let rect = Rect::from_center_size(center, disp_size1);
                                    ui.allocate_ui_at_rect(rect, |ui| {
                                        ui.add(Image::from_texture(handle1).fit_to_exact_size(disp_size1));
                                    });
                                }
                            }
                        }
                    } else {
                        // Show spinner
                        let spinner_size = 48.0;
                        let spinner_rect = egui::Rect::from_center_size(
                            image_area.center(),
                            Vec2::splat(spinner_size),
                        );
                        ui.allocate_ui_at_rect(spinner_rect, |ui| {
                            ui.add(egui::Spinner::new().size(spinner_size).color(egui::Color32::WHITE));
                        });
                    }
                }
            } else {
                // Single page mode
                let loaded = self.image_lru.lock().unwrap().get(&self.current_page).cloned();
                if let Some(loaded) = loaded {
                    if !self.has_initialised_zoom {
                        self.reset_zoom(image_area, &loaded);
                    }
                    // Texture cache
                    let needs_update = self.single_texture_cache.as_ref().map_or(true, |(idx, _)| *idx != self.current_page);
                    if needs_update {
                        let (w, h) = loaded.image.dimensions();
                        let color_img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &loaded.image.to_rgba8());
                        let handle = ui.ctx().load_texture(format!("tex{}", loaded.index), color_img, TextureOptions::default());
                        self.single_texture_cache = Some((self.current_page, handle));
                    }
                    if let Some((_, handle)) = &self.single_texture_cache {
                        let (w, h) = loaded.image.dimensions();
                        let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);
                        let center = image_area.center();
                        let rect = Rect::from_center_size(center, disp_size);
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.add(Image::from_texture(handle).fit_to_exact_size(disp_size));
                        });
                    }
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

            // --- Loading indicator ---
            let in_cache = self.image_lru.lock().unwrap().get(&self.current_page).is_some();

            if !in_cache {
                let spinner_size = 48.0;
                let spinner_rect = egui::Rect::from_center_size(
                    image_area.center(),
                    Vec2::splat(spinner_size),
                );
                ui.allocate_ui_at_rect(spinner_rect, |ui| {
                    ui.add(egui::Spinner::new().size(spinner_size).color(egui::Color32::WHITE));
                });
            }
        });
    }
}

/// Open file picker to select a CBZ or ZIP file
fn pick_comic() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Comic Book Archive", &["cbz", "zip"])
        .pick_file()
}

/// Initialize the app and run it with eframe
fn initialise(path: PathBuf) {
    let _ = eframe::run_native(
        "CBZ Viewer",
        NativeOptions {
            initial_window_size: Some(Vec2::new(WIN_WIDTH, WIN_HEIGHT)),
            ..Default::default()
        },
        Box::new(|_cc| Box::new(CBZViewerApp::new(path))),
    );
}

/// Entry point: load path from CLI or show file picker
fn main() {
    let path = std::env::args().nth(1);

    match path {
        Some(path) => {
            let path = PathBuf::from(path);
            initialise(path);
        }
        None => match pick_comic() {
            Some(path) => initialise(path),
            None => {
                println!("Exiting!");
                thread::sleep(Duration::from_secs(3));
            }
        },
    }
}