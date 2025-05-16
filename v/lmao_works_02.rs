use std::{fs::File, io::Read, path::PathBuf, sync::{Arc, Mutex}, thread};
use std::time::Duration;

use eframe::{egui, App, NativeOptions};
use eframe::egui::{ColorImage, TextureFilter, TextureOptions, Vec2, ScrollArea, ProgressBar};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use zip::ZipArchive;

const WIN_WIDTH: f32 = 1280.0;
const WIN_HEIGHT: f32 = 720.0;

struct CBZViewerApp {
    zip_path: PathBuf,
    filenames: Vec<String>,
    current_page: usize,
    image_cache: Arc<Mutex<Option<DynamicImage>>>,
    progress: Arc<Mutex<f32>>,
    loading_thread: Option<thread::JoinHandle<()>>,
    zoom: f32,
}

impl CBZViewerApp {
    fn new(zip_path: PathBuf) -> Self {
        let file = File::open(&zip_path).expect("Failed to open CBZ file");
        let mut archive = ZipArchive::new(file).expect("Failed to read zip");

        let mut names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".png")
                    || lower.ends_with(".bmp")
                    || lower.ends_with(".gif")
                    || lower.ends_with(".webp")
                {
                    names.push(name);
                }
            }
        }
        names.sort_by_key(|n| n.to_lowercase());

        Self {
            zip_path,
            filenames: names,
            current_page: 0,
            image_cache: Arc::new(Mutex::new(None)),
            progress: Arc::new(Mutex::new(0.0)),
            loading_thread: None,
            zoom: 1.0,
        }
    }

    fn load_image_async(&mut self, page: usize) {
        if let Some(handle) = self.loading_thread.take() {
            handle.join().ok();
        }
        let filenames = self.filenames.clone();
        let zip_path = self.zip_path.clone();
        let image_cache = Arc::clone(&self.image_cache);
        let progress = Arc::clone(&self.progress);

        *progress.lock().unwrap() = 0.0;
        *image_cache.lock().unwrap() = None;

        self.loading_thread = Some(thread::spawn(move || {
            let file = File::open(&zip_path).expect("Failed to open CBZ file");
            let mut archive = ZipArchive::new(file).expect("Failed to open zip");
            if page >= filenames.len() {
                return;
            }
            let filename = &filenames[page];
            let mut file_in_zip = archive.by_name(filename).expect("File missing in zip");

            let mut buf = Vec::with_capacity(file_in_zip.size() as usize);
            let mut total = 0u64;
            let size = file_in_zip.size();
            let mut temp = [0u8; 8192];
            while let Ok(n) = file_in_zip.read(&mut temp) {
                if n == 0 { break; }
                buf.extend_from_slice(&temp[..n]);
                total += n as u64;
                let prog = (total as f32 / size as f32).min(1.0);
                *progress.lock().unwrap() = prog;
            }

            let dyn_img = image::load_from_memory(&buf).expect("Decode failed");
            *image_cache.lock().unwrap() = Some(dyn_img);
            *progress.lock().unwrap() = 1.0;
        }));
    }
}

impl App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start initial load
        if self.loading_thread.is_none() {
            self.load_image_async(self.current_page);
        }

        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::ArrowRight) && self.current_page + 1 < self.filenames.len() {
            self.current_page += 1;
            self.zoom = 1.0;
            self.load_image_async(self.current_page);
        }
        if input.key_pressed(egui::Key::ArrowLeft) && self.current_page > 0 {
            self.current_page -= 1;
            self.zoom = 1.0;
            self.load_image_async(self.current_page);
        }

        // Zoom with scroll wheel
        if input.scroll_delta.y != 0.0 {
            self.zoom *= (1.0 + input.scroll_delta.y * 0.01).clamp(0.1, 10.0);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.filenames[self.current_page]);
                    ui.label(format!("({}/{})", self.current_page+1, self.filenames.len()));
                });
            });

            ui.add_space(8.0);

            // Image Scroll with pan
            ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(img) = &*self.image_cache.lock().unwrap() {
                        let size = Vec2::new(
                            img.width() as f32 * self.zoom,
                            img.height() as f32 * self.zoom,
                        );
                        let resized = img.resize_exact(
                            size.x as u32,
                            size.y as u32,
                            FilterType::Lanczos3,
                        );
                        let rgba = resized.to_rgba8();
                        let color_img = ColorImage::from_rgba_unmultiplied(
                            [resized.width() as usize, resized.height() as usize], &rgba,
                        );
                        let tex = ui.ctx().load_texture(
                            "cbz_tex",
                            color_img,
                            TextureOptions { magnification: TextureFilter::Linear, minification: TextureFilter::Linear, ..Default::default() },
                        );
                        ui.add(egui::Image::new((tex.id(), size)));
                    }
                });

            // Progress bar at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                let prog = *self.progress.lock().unwrap();
                ui.add(ProgressBar::new(prog)
                    .desired_width(ui.available_width() * 0.8)
                    .show_percentage());
            });
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn main() {
    let zip_path = std::env::args().nth(1).expect("Usage: cbz_viewer <file.cbz>");
    let mut app = CBZViewerApp::new(PathBuf::from(zip_path));
    
    let native_opts = NativeOptions {
        initial_window_size: Some(Vec2::new(WIN_WIDTH, WIN_HEIGHT)),
        resizable: true,
        ..Default::default()
    };
    eframe::run_native(
        "CBZ Viewer",
        native_opts,
        Box::new(|_cc| Box::new(app)),
    );
}

