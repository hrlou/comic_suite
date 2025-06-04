// Hide the console window on Windows
#![windows_subsystem = "windows"]

use std::num::NonZeroUsize;
use std::time::Duration;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{
    pos2, Align, ColorImage, Image, Layout, ProgressBar, Rect, TextureFilter, TextureOptions, Vec2,
};
use eframe::{egui, App, NativeOptions};
use lru::LruCache;
use image::{DynamicImage, GenericImageView};
use zip::ZipArchive;

// Constants for initial window size
const WIN_WIDTH: f32 = 720.0;
const WIN_HEIGHT: f32 = 1080.0;
const CACHE_SIZE: usize = 20; // Number of images to cache

// Main application state
struct CBZViewerApp {
    zip_path: PathBuf,                             // Path to the CBZ/ZIP archive
    image_lru: Arc<Mutex<LruCache<usize, DynamicImage>>>, // Image cache for recently used images 
    filenames: Vec<String>,                        // List of image filenames in the archive
    current_page: usize,                           // Index of currently displayed image
    image_cache: Arc<Mutex<Option<DynamicImage>>>, // Shared cache for decoded image
    texture_cache: Arc<Mutex<Option<(usize, egui::TextureHandle)>>>, // Shared cache for GPU texture
    progress: Arc<Mutex<f32>>,                     // Shared loading progress
    loading_thread: Option<thread::JoinHandle<()>>, // Background thread handle for image loading
    zoom: f32,                                     // Current zoom level
    pan_offset: Vec2,                              // Pan offset for dragging
    window_size: Vec2,                             // Current window size, for calculating default zoom
    drag_start: Option<egui::Pos2>,                // Start position of drag
    has_initialised_zoom: bool,
    double_page_mode: bool,                        // Whether to show two pages side by side
    right_to_left: bool,                           // Whether to read right to left (manga style)
    loading_pages: Arc<Mutex<HashSet<usize>>>,    // Set of pages currently being loaded
    dual_texture_cache: Option<((usize, Option<egui::TextureHandle>), (usize, Option<egui::TextureHandle>))>, // Dual texture cache for double page mode
}

impl CBZViewerApp {
    /// Create a new viewer from a CBZ file path
    fn new(zip_path: PathBuf) -> Self {
        let file = File::open(&zip_path).expect("Failed to open CBZ file");
        let mut archive = ZipArchive::new(file).expect("Failed to read zip");

        // Filter image files
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
        // Sort image files alphabetically
        names.sort_by_key(|n| n.to_lowercase());

        Self {
            zip_path,
            filenames: names,
            image_lru: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()))), // Cache for 8 images
            current_page: 0,
            image_cache: Arc::new(Mutex::new(None)),
            texture_cache: Arc::new(Mutex::new(None)),
            progress: Arc::new(Mutex::new(0.0)),
            loading_thread: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            window_size: Vec2::ZERO,
            drag_start: None,
            has_initialised_zoom: false,
            double_page_mode: false,
            right_to_left: false,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
            dual_texture_cache: None,
        }
    }
    /// Spawn background thread to load and decode image
    fn load_image_async(&mut self, page: usize) {
        // Prevent duplicate loads
        {
            let mut loading = self.loading_pages.lock().unwrap();
            if loading.contains(&page) {
                return;
            }
            loading.insert(page);
        }

        *self.texture_cache.lock().unwrap() = None;

        // Check LRU cache first
        if let Some(img) = self.image_lru.lock().unwrap().get(&page).cloned() {
            *self.image_cache.lock().unwrap() = Some(img);
            *self.progress.lock().unwrap() = 1.0;
            return;
        }

        // Setup for new thread
        let filenames = self.filenames.clone();
        let zip_path = self.zip_path.clone();
        let image_cache = Arc::clone(&self.image_cache);
        let progress = Arc::clone(&self.progress);
        let image_lru = Arc::clone(&self.image_lru);
        let loading_pages = Arc::clone(&self.loading_pages);
        self.loading_thread = Some(thread::spawn(move || {
            let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
            let mut file = archive.by_name(&filenames[page]).unwrap();
            let size = file.size();

            let mut buf = Vec::with_capacity(size as usize);
            let mut total = 0u64;
            let mut tmp = [0u8; 8192];

            // Read file in chunks while updating progress
            while let Ok(n) = file.read(&mut tmp) {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                total += n as u64;
                *progress.lock().unwrap() = (total as f32 / size as f32).min(1.0);
            }

            // Decode image
            let img = image::load_from_memory(&buf).unwrap();
            *image_cache.lock().unwrap() = Some(img.clone());
            *progress.lock().unwrap() = 1.0;
            image_lru.lock().unwrap().put(page, img); // Cache the image

            // Remove from loading set
            loading_pages.lock().unwrap().remove(&page);
        }));

        // --- Preload next and previous pages in the background ---
        let filenames = self.filenames.clone();
        let zip_path = self.zip_path.clone();
        let image_lru = Arc::clone(&self.image_lru);

        if page + 1 < self.filenames.len() {
            let filenames = filenames.clone();
            let zip_path = zip_path.clone();
            let image_lru = Arc::clone(&image_lru);
            let next_page = page + 1;
            thread::spawn(move || {
                if image_lru.lock().unwrap().get(&next_page).is_none() {
                    let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
                    {
                        if let Ok(mut file) = archive.by_name(&filenames[next_page]) {
                            let mut buf = Vec::with_capacity(file.size() as usize);
                            let mut tmp = [0u8; 8192];
                            while let Ok(n) = file.read(&mut tmp) {
                                if n == 0 { break; }
                                buf.extend_from_slice(&tmp[..n]);
                            }
                            if let Ok(img) = image::load_from_memory(&buf) {
                                image_lru.lock().unwrap().put(next_page, img);
                            }
                        }
                    }; // <-- file borrow ends here before archive is dropped
                }
            });
        }

        // Preload previous page
        if page > 0 {
            let filenames = filenames.clone();
            let zip_path = zip_path.clone();
            let image_lru = Arc::clone(&image_lru);
            let prev_page = page - 1;
            thread::spawn(move || {
                if image_lru.lock().unwrap().get(&prev_page).is_none() {
                    let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
                    {
                        if let Ok(mut file) = archive.by_name(&filenames[prev_page]) {
                            let mut buf = Vec::with_capacity(file.size() as usize);
                            let mut tmp = [0u8; 8192];
                            while let Ok(n) = file.read(&mut tmp) {
                                if n == 0 { break; }
                                buf.extend_from_slice(&tmp[..n]);
                            }
                            if let Ok(img) = image::load_from_memory(&buf) {
                                image_lru.lock().unwrap().put(prev_page, img);
                            }
                        }
                    }; // <-- file borrow ends here before archive is dropped
                }
            });
        }
    }
}

impl App for CBZViewerApp {
    /// Main UI update loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load current image if not already loading
        if self.loading_thread.is_none() {
            self.load_image_async(self.current_page);
        }

        let input = ctx.input(|i| i.clone());
        self.window_size = ctx.screen_rect().size();

        let total_pages = self.filenames.len();

        if self.double_page_mode {
            // --- Navigation ---
            if input.key_pressed(egui::Key::ArrowRight) {
                if self.current_page == 0 && total_pages > 1 {
                    self.current_page = 1;
                } else if self.current_page + 2 < total_pages {
                    self.current_page += 2;
                } else if self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
                self.load_image_async(self.current_page);
                if self.current_page + 1 < total_pages {
                    self.load_image_async(self.current_page + 1);
                }
            }
            if input.key_pressed(egui::Key::ArrowLeft) {
                if self.current_page == 1 || self.current_page == 0 {
                    self.current_page = 0;
                } else if self.current_page >= 2 {
                    self.current_page -= 2;
                }
                self.load_image_async(self.current_page);
                if self.current_page + 1 < total_pages {
                    self.load_image_async(self.current_page + 1);
                }
            }
        } else {
            // --- Single page navigation ---
            if input.key_pressed(egui::Key::ArrowRight) && self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.load_image_async(self.current_page);
            }
            if input.key_pressed(egui::Key::ArrowLeft) && self.current_page > 0 {
                self.current_page -= 1;
                self.load_image_async(self.current_page);
            }
        }

        // --- Display ---
        if self.current_page == 0 {
            // Show only page 0
            if let Some(img) = self.image_lru.lock().unwrap().get(&0).cloned() {
                // ...draw single page 0...
            }
        } else {
            let page1 = self.current_page;
            let page2 = page1 + 1;
            let img1 = self.image_lru.lock().unwrap().get(&page1).cloned();
            let img2 = if page2 < total_pages {
                self.image_lru.lock().unwrap().get(&page2).cloned()
            } else {
                None
            };

            // ...draw img1 and img2 side by side if img2 exists, else just img1...
        }

        // Calculate initial zoom value from window size and image resoultion
        if !self.has_initialised_zoom {
            if let Some(img) = &*self.image_cache.lock().unwrap() {
                let (img_w, img_h) = img.dimensions();
                let zoom_x = self.window_size.x / img_w as f32;
                let zoom_y = self.window_size.y / img_h as f32;
                self.zoom = zoom_x.min(zoom_y).min(1.0); // Cap if desired
                self.pan_offset = Vec2::ZERO;
                self.has_initialised_zoom = true;
            }
        }

        // Zoom with scroll wheel
        if input.scroll_delta.y.abs() > 0.0 {
            self.zoom *= (1.0 + input.scroll_delta.y * 0.01).clamp(0.1, 10.0);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Top bar: buttons and filename ---
            let top_bar_height = 32.0;
            ui.horizontal(|ui| {
                let mut mode_switched = false;
                if ui.selectable_label(!self.double_page_mode, "Single Page").clicked() {
                    if self.double_page_mode {
                        self.double_page_mode = false;
                        *self.texture_cache.lock().unwrap() = None;
                        self.dual_texture_cache = None;
                        self.load_image_async(self.current_page);
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
                        *self.texture_cache.lock().unwrap() = None;
                        self.dual_texture_cache = None;
                        self.load_image_async(self.current_page);
                        if self.current_page + 1 < self.filenames.len() {
                            self.load_image_async(self.current_page + 1);
                        }
                        mode_switched = true;
                    }
                }
                ui.separator();
                let dir_label = if self.right_to_left { "Right to Left" } else { "Left to Right" };
                if ui.button(dir_label).clicked() {
                    self.right_to_left = !self.right_to_left;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.filenames[self.current_page]);
                });
            });
            ui.add_space(8.0);
        
            // Reserve space for the bottom bar (page number)
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
        
            // Track if we need to show a progress bar
            let show_progress = self.texture_cache.lock().unwrap().is_none();
        
            // Detect and handle drag for panning
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
        
            // --- Double page mode implementation ---
            if self.double_page_mode {
                let max_page = self.filenames.len().saturating_sub(1);
                let page1 = self.current_page;
                let page2 = if self.right_to_left {
                    if page1 > 0 { page1 - 1 } else { page1 }
                } else {
                    if page1 + 1 <= max_page { page1 + 1 } else { page1 }
                };

                // Only show two pages if not at the first or last page
                let show_two_pages = if self.right_to_left {
                    page1 > 0
                } else {
                    page1 + 1 <= max_page
                };

                // Preload the second page if not already loaded and we're not at the edge
                if show_two_pages && self.image_lru.lock().unwrap().get(&page2).is_none() {
                    self.load_image_async(page2);
                }

                // Load both images from cache
                let img1 = self.image_lru.lock().unwrap().get(&page1).cloned();
                let img2 = if show_two_pages {
                    self.image_lru.lock().unwrap().get(&page2).cloned()
                } else {
                    None
                };

                // --- Texture caching for dual page ---
                let mut update_cache = false;
                if self.dual_texture_cache.is_none()
                    || self.dual_texture_cache.as_ref().unwrap().0.0 != page1
                    || self.dual_texture_cache.as_ref().unwrap().1.0 != page2
                {
                    update_cache = true;
                }

                if update_cache {
                    let tex1 = img1.as_ref().map(|img| {
                        let (w, h) = img.dimensions();
                        let color_img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.to_rgba8());
                        ctx.load_texture(format!("tex{}", page1), color_img, TextureOptions::default())
                    });
                    let tex2 = img2.as_ref().map(|img| {
                        let (w, h) = img.dimensions();
                        let color_img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.to_rgba8());
                        ctx.load_texture(format!("tex{}", page2), color_img, TextureOptions::default())
                    });
                    self.dual_texture_cache = Some(((page1, tex1), (page2, tex2)));
                }

                if let Some(((p1, Some(ref tex1)), (p2, tex2))) = &self.dual_texture_cache {
                    // Layout: if tex2 is Some, show both, else show only tex1 centered
                    if let Some(tex2) = tex2 {
                        // Draw side by side, centered
                        let (w1, h1) = img1.as_ref().unwrap().dimensions();
                        let (w2, h2) = img2.as_ref().unwrap().dimensions();
                        let disp_size1 = Vec2::new(w1 as f32 * self.zoom, h1 as f32 * self.zoom);
                        let disp_size2 = Vec2::new(w2 as f32 * self.zoom, h2 as f32 * self.zoom);
                        let total_width = disp_size1.x + disp_size2.x;
                        let center = image_area.center();
                        let left_start = center.x - total_width / 2.0;

                        if self.right_to_left {
                            // page2 (left), page1 (right)
                            let rect2 = Rect::from_min_size(
                                pos2(left_start, center.y - disp_size2.y / 2.0),
                                disp_size2,
                            );
                            let rect1 = Rect::from_min_size(
                                pos2(left_start + disp_size2.x, center.y - disp_size1.y / 2.0),
                                disp_size1,
                            );
                            ui.allocate_ui_at_rect(rect2, |ui| {
                                ui.add(Image::from_texture(tex2).fit_to_exact_size(disp_size2));
                            });
                            ui.allocate_ui_at_rect(rect1, |ui| {
                                ui.add(Image::from_texture(tex1).fit_to_exact_size(disp_size1));
                            });
                        } else {
                            // page1 (left), page2 (right)
                            let rect1 = Rect::from_min_size(
                                pos2(left_start, center.y - disp_size1.y / 2.0),
                                disp_size1,
                            );
                            let rect2 = Rect::from_min_size(
                                pos2(left_start + disp_size1.x, center.y - disp_size2.y / 2.0),
                                disp_size2,
                            );
                            ui.allocate_ui_at_rect(rect1, |ui| {
                                ui.add(Image::from_texture(tex1).fit_to_exact_size(disp_size1));
                            });
                            ui.allocate_ui_at_rect(rect2, |ui| {
                                ui.add(Image::from_texture(tex2).fit_to_exact_size(disp_size2));
                            });
                        }
                    } else {
                        // Only one page (last page, odd count)
                        let (w1, h1) = img1.as_ref().unwrap().dimensions();
                        let disp_size1 = Vec2::new(w1 as f32 * self.zoom, h1 as f32 * self.zoom);

                        let center = image_area.center();
                        let rect = Rect::from_center_size(center, disp_size1);
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.add(Image::from_texture(tex1).fit_to_exact_size(disp_size1));
                        });
                    }
                }
            } else {
                // Single page mode (your existing code)
                if !self.double_page_mode {
                    if let Some(img) = self.image_lru.lock().unwrap().get(&self.current_page).cloned() {
                        let (w, h) = img.dimensions();
                        let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);

                        // Always create a new texture for the current page
                        let color_img = ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            &img.to_rgba8(),
                        );
                        let handle = ui.ctx().load_texture(
                            format!("tex{}", self.current_page),
                            color_img,
                            TextureOptions::default(),
                        );

                        let center = image_area.center();
                        let rect = Rect::from_center_size(center, disp_size);
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size));
                        });
                    }
                }
            }
        }); 
        
        // Schedule next frame repaint
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

/// Open file picker to select a CBZ or ZIP file
fn pick_comic() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("CBZ or ZIP files", &["cbz", "zip"])
        .set_title("Select a CBZ or ZIP file")
        .pick_file()
}

/// Initialize the app and run it with eframe
fn initialise(path: PathBuf) {
    let app = CBZViewerApp::new(path);
    let opts = NativeOptions {
        initial_window_size: Some(Vec2::new(WIN_WIDTH, WIN_HEIGHT)),
        resizable: true,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "CBZ Viewer",
        opts,
        Box::new(|_| Box::new(app)),
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
