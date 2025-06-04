// Hide the console window on Windows
#![windows_subsystem = "windows"]

use std::num::NonZeroUsize;
use std::time::Duration;
use std::{
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
const CACHE_SIZE: usize = 12; // Number of images to cache

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
        }
    }
    /// Spawn background thread to load and decode image
    fn load_image_async(&mut self, page: usize) {
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

        *progress.lock().unwrap() = 0.0;
        *image_cache.lock().unwrap() = None;

        // Spawn thread to load and decode the image
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

        // Handle arrow key navigation
        if input.key_pressed(egui::Key::ArrowRight) && self.current_page + 1 < self.filenames.len()
        {
            self.current_page += 1;
            self.load_image_async(self.current_page);
        }
        if input.key_pressed(egui::Key::ArrowLeft) && self.current_page > 0 {
            self.current_page -= 1;
            self.load_image_async(self.current_page);
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
            // Display filename and page number
            ui.horizontal(|ui| {
                // Single/Double page toggle
                if ui.selectable_label(!self.double_page_mode, "Single Page").clicked() {
                    self.double_page_mode = false;
                }
                if ui.selectable_label(self.double_page_mode, "Double Page").clicked() {
                    self.double_page_mode = true;
                }
                ui.separator();
                // Reading direction toggle
                let dir_label = if self.right_to_left { "Right to Left" } else { "Left to Right" };
                if ui.button(dir_label).clicked() {
                    self.right_to_left = !self.right_to_left;
                }
                // Spacer to push filename to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.filenames[self.current_page]);
                });
                // ui.label(&self.filenames[self.current_page]);
                // ui.label(format!(
                //     "({}/{})",
                //     self.current_page + 1,
                //     self.filenames.len()
                // ));
            });
            ui.add_space(8.0);

            // Track if we need to show a progress bar
            let show_progress = self.texture_cache.lock().unwrap().is_none();

            // Detect and handle drag for panning
            let response = ui.allocate_response(ui.available_size(), egui::Sense::drag());

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

            // Display loaded image if available
            if let Some(img) = &*self.image_cache.lock().unwrap() {
                let (w, h) = img.dimensions();
                let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);

                let mut cache = self.texture_cache.lock().unwrap();
                // If texture not cached for current page, upload to GPU
                if cache.as_ref().map(|(p, _)| *p) != Some(self.current_page) {
                    let color_img = ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        &img.to_rgba8(),
                    );
                    let handle = ui.ctx().load_texture(
                        format!("tex{}", self.current_page),
                        color_img,
                        TextureOptions {
                            magnification: TextureFilter::Linear,
                            minification: TextureFilter::Linear,
                            ..Default::default()
                        },
                    );
                    *cache = Some((self.current_page, handle));
                }

                if let Some((_, handle)) = &*cache {
                    let center = response.rect.center();
                    let view_size = response.rect.size();
                    let mut offset = self.pan_offset;

                    // Clamp panning to keep image within view bounds
                    let bound_x = ((disp_size.x + view_size.x) / 2.0).max(1.0);
                    let bound_y = ((disp_size.y + view_size.y) / 2.0).max(1.0);
                    offset.x = offset.x.clamp(-bound_x, bound_x);
                    offset.y = offset.y.clamp(-bound_y, bound_y);
                    self.pan_offset = offset;

                    let rect = Rect::from_center_size(center + offset, disp_size);
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.add(Image::from_texture(handle).fit_to_exact_size(disp_size));
                    });
                }
            }

            // Show progress bar if image is still loading
            if show_progress {
                let prog = *self.progress.lock().unwrap();
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add(
                        ProgressBar::new(prog)
                            .desired_width(ui.available_width() * 0.8)
                            .show_percentage(),
                    );
                });
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.label(format!(
                    "({}/{})",
                    self.current_page + 1,
                    self.filenames.len()
                ));
            });

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
