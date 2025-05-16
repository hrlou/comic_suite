use std::{fs::File, io::Read, path::PathBuf, sync::{Arc, Mutex}, thread};
use std::time::Duration;

use eframe::{egui, App, NativeOptions};
use eframe::egui::{ColorImage, TextureFilter, TextureOptions, Vec2, ProgressBar, Layout, Align, Rect, pos2, Image};
use image::{DynamicImage, GenericImageView};
use zip::ZipArchive;

const WIN_WIDTH: f32 = 1280.0;
const WIN_HEIGHT: f32 = 720.0;

struct CBZViewerApp {
    zip_path: PathBuf,
    filenames: Vec<String>,
    current_page: usize,
    image_cache: Arc<Mutex<Option<DynamicImage>>>,
    texture_cache: Arc<Mutex<Option<(usize, egui::TextureHandle)>>>,
    progress: Arc<Mutex<f32>>,
    loading_thread: Option<thread::JoinHandle<()>>,
    zoom: f32,
    pan_offset: Vec2,
    drag_start: Option<egui::Pos2>,
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
                if [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp"].iter().any(|ext| lower.ends_with(ext)) {
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
            texture_cache: Arc::new(Mutex::new(None)),
            progress: Arc::new(Mutex::new(0.0)),
            loading_thread: None,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            drag_start: None,
        }
    }

    fn load_image_async(&mut self, page: usize) {
        *self.texture_cache.lock().unwrap() = None;
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
            let mut archive = ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
            let mut file = archive.by_name(&filenames[page]).unwrap();
            let size = file.size();
            let mut buf = Vec::with_capacity(size as usize);
            let mut total = 0u64;
            let mut tmp = [0u8; 8192];
            while let Ok(n) = file.read(&mut tmp) {
                if n == 0 { break; }
                buf.extend_from_slice(&tmp[..n]);
                total += n as u64;
                *progress.lock().unwrap() = (total as f32 / size as f32).min(1.0);
            }
            let img = image::load_from_memory(&buf).unwrap();
            *image_cache.lock().unwrap() = Some(img);
            *progress.lock().unwrap() = 1.0;
        }));
    }
}

impl App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.loading_thread.is_none() {
            self.load_image_async(self.current_page);
        }
        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::ArrowRight) && self.current_page + 1 < self.filenames.len() {
            self.current_page += 1;
            self.load_image_async(self.current_page);
        }
        if input.key_pressed(egui::Key::ArrowLeft) && self.current_page > 0 {
            self.current_page -= 1;
            self.load_image_async(self.current_page);
        }
        if input.scroll_delta.y.abs() > 0.0 {
            self.zoom *= (1.0 + input.scroll_delta.y * 0.01).clamp(0.1, 10.0);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.filenames[self.current_page]);
                ui.label(format!("({}/{})", self.current_page + 1, self.filenames.len()));
            });
            ui.add_space(8.0);

            let show_progress = self.texture_cache.lock().unwrap().is_none();

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

            if let Some(img) = &*self.image_cache.lock().unwrap() {
                let (w, h) = img.dimensions();
                let disp_size = Vec2::new(w as f32 * self.zoom, h as f32 * self.zoom);
                let mut cache = self.texture_cache.lock().unwrap();
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
                    let rect = Rect::from_center_size(center + self.pan_offset, disp_size);
                    ui.allocate_ui_at_rect(rect, |ui| {
                        ui.add(Image::from_texture(handle).fit_to_exact_size(disp_size));
                    });
                }
            }

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
        });
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn main() {
    let zip_path = std::env::args()
        .nth(1)
        .expect("Usage: cbz_viewer <file.cbz>");
    let app = CBZViewerApp::new(PathBuf::from(zip_path));
    let opts = NativeOptions {
        initial_window_size: Some(Vec2::new(WIN_WIDTH, WIN_HEIGHT)),
        resizable: true,
        ..Default::default()
    };
    eframe::run_native("CBZ Viewer", opts, Box::new(|_| Box::new(app)));
}

