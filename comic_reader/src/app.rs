//! Main application state and logic.

use crate::{
    // archive::{self, ZipImageArchive},
    prelude::*,
};
use egui::{Pos2, epaint::tessellator::Path};
use tokio::sync::Semaphore;

/// The main application struct, holding all state.
pub struct CBZViewerApp {
    pub archive_path: Option<PathBuf>,
    pub archive: Option<Arc<Mutex<ImageArchive>>>,
    pub filenames: Option<Vec<String>>,
    pub image_lru: SharedImageCache,
    pub current_page: usize,
    pub texture_cache: TextureCache,
    pub ui_logger: Arc<Mutex<UiLogger>>,
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub original_pan_offset: Vec2,
    pub drag_start: Option<egui::Pos2>,
    pub double_page_mode: bool,
    pub right_to_left: bool,
    pub has_initialised_zoom: bool,
    pub loading_pages: Arc<Mutex<HashSet<usize>>>,
    pub page_goto_box: String,
    pub show_manifest_editor: bool,
    pub on_goto_page: bool,
    pub on_new_comic: bool,
    pub on_open_comic: bool,
    pub on_open_folder: bool,
    pub on_save_image: bool,
    pub is_web_archive: bool,
    pub total_pages: usize,
    pub show_thumbnail_grid: bool,
    pub thumbnail_cache: Arc<Mutex<std::collections::HashMap<usize, image::DynamicImage>>>,
    pub thumb_semaphore: Arc<Semaphore>,
    pub new_page: Option<PathBuf>,
    pub show_debug_menu: bool,
    pub slideshow_mode: bool,
    pub slideshow_last_tick: std::time::Instant,
    pub slideshow_interval_secs: f32,
    pub show_slideshow_interval_popup: bool, // New field to control the popup
    pub archive_view: ArchiveView,
}

impl Default for CBZViewerApp {
    fn default() -> Self {
        Self {
            archive_path: None,
            archive: None,
            filenames: None,
            image_lru: new_image_cache(CACHE_SIZE),
            current_page: 0,
            texture_cache: TextureCache::new(),
            ui_logger: Arc::new(Mutex::new(UiLogger::new())),
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            original_pan_offset: Vec2::ZERO,
            drag_start: None,
            double_page_mode: DEFAULT_DUAL_PAGE_MODE,
            right_to_left: DEFAULT_RIGHT_TO_LEFT,
            has_initialised_zoom: false,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
            page_goto_box: "1".to_string(),
            show_manifest_editor: false,
            on_goto_page: false,
            on_new_comic: false,
            on_open_comic: false,
            on_open_folder: false,
            on_save_image: false,
            is_web_archive: false,
            total_pages: 0,
            show_thumbnail_grid: false,
            thumbnail_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            thumb_semaphore: Arc::new(Semaphore::new(8)), // Limit to 8 concurrent thumbnail loads
            new_page: None,
            show_debug_menu: false,
            slideshow_mode: false,
            slideshow_last_tick: std::time::Instant::now(),
            slideshow_interval_secs: 5.0, // Default slideshow interval
            show_slideshow_interval_popup: false, // Initialize the popup control field
            archive_view: ArchiveView::default(),
        }
    }
}

impl CBZViewerApp {
    /// Create a new app instance from a given archive path.
    pub fn new(cc: &CreationContext, path: Option<PathBuf>) -> Result<Self, AppError> {
        crate::ui::setup_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        if let Some(path) = path {
            let _ = futures::executor::block_on(app.load_new_file(path));
        }
        #[cfg(feature = "7z")]
        {
            if let Ok(mut logger) = app.ui_logger.lock() {
                logger.warn("7z archives are supported, however, it involves an external tool extracting files to a temporary directory.", Some(10));
            }
        }
        Ok(app)
    }

    pub fn reset_zoom(&mut self, area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let avail = area.size();
        let scale_x = avail.x / w as f32;
        let scale_y = avail.y / h as f32;
        self.zoom = scale_x.min(scale_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
    }

    /// Go to the previous page (with bounds checking).
    pub fn goto_prev_page(&mut self) {
        if self.current_page == 0 {
            if let Ok(mut logger) = self.ui_logger.lock() {
                logger.warn("Already at the first page, cannot go back.", None);
            }
            return;
        }
        let step = if self.double_page_mode { 2 } else { 1 };
        let new_page = self.current_page.saturating_sub(step);
        self.goto_page(new_page);
    }

    /// Go to the next page (with bounds checking).
    pub fn goto_next_page(&mut self) {
        let step = if self.double_page_mode { 2 } else { 1 };
        let new_page = self.current_page + step;
        self.goto_page(new_page);
    }

    /// Go to a specific page (with bounds checking).
    pub fn goto_page(&mut self, page: usize) -> bool {
        self.on_page_changed();
        if let Some(filenames) = &self.filenames {
            if page >= filenames.len() {
                if let Ok(mut logger) = self.ui_logger.lock() {
                    logger.warn(
                        format!("Requested page {} is out of bounds.", page + 1),
                        None,
                    );
                }
                return false;
            }
            self.current_page = page;
            true
        } else {
            if let Ok(mut logger) = self.ui_logger.lock() {
                logger.warn("No filenames available to go to specific page.", None);
            }
            false
        }
    }

    pub async fn load_new_file(&mut self, path: PathBuf) -> Result<(), AppError> {
        // Reset self to default values, but keep the logger and context if needed
        let mut new_self = Self::default();

        // Optionally preserve logger or other fields if needed
        new_self.ui_logger = Arc::clone(&self.ui_logger);

        let archive = ImageArchive::process(&path).await?;

        let archive = Arc::new(Mutex::new(archive));
        if let Ok(guard) = archive.lock() {
            let filenames = guard.list_images();
            new_self.filenames = Some(filenames);
            new_self.is_web_archive = guard.manifest.meta.web_archive;
        }
        new_self.archive_path = Some(path);
        new_self.total_pages = new_self.filenames.as_ref().map_or(0, |f| f.len());
        new_self.archive = Some(Arc::clone(&archive));
        new_self.image_lru = new_image_cache(CACHE_SIZE);
        new_self.current_page = 0;

        // Move new_self's fields into self
        *self = new_self;

        Ok(())
    }

    /// Called whenever the page changes: resets zoom, pan, and clears texture cache.
    pub fn on_page_changed(&mut self) {
        self.has_initialised_zoom = false;
        self.texture_cache.clear();
        self.pan_offset = Vec2::ZERO;
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        // Set the window title based on the archive name or path
        if let Some(archive) = self.archive.as_ref() {
            let mut title = NAME.to_string();
            if let Ok(archive) = archive.lock() {
                if !archive.manifest.meta.title.is_empty()
                    && archive.manifest.meta.title != "Unknown"
                {
                    title = archive.manifest.meta.title.clone();
                    if !archive.manifest.meta.author.is_empty()
                        && archive.manifest.meta.author != "Unknown"
                    {
                        title = format!(
                            "{} - {}",
                            archive.manifest.meta.author, archive.manifest.meta.title
                        );
                    }
                } else if let Some(path) = &self.archive_path {
                    title = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(NAME)
                        .to_string();
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
    }

    pub fn preload_images(&mut self, ctx: &egui::Context, archive: Arc<Mutex<ImageArchive>>) {
        let filenames = self.filenames.clone().unwrap_or_default();

        // Preload images for current view and next pages
        let mut pages_to_preload = vec![self.current_page];
        let read_ahead = if self.is_web_archive {
            READ_AHEAD_WEB
        } else {
            READ_AHEAD
        };

        for offset in 1..=read_ahead {
            let next = self.current_page + offset;
            if next < self.total_pages {
                pages_to_preload.push(next);
            }
        }
        for &page in &pages_to_preload {
            let filenames = Arc::new(filenames.clone());
            let archive = archive.clone();
            let image_lru = self.image_lru.clone();
            let loading_pages = self.loading_pages.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                // Do NOT lock any mutex here before await!
                let _ =
                    load_image_async(page, filenames, archive, image_lru, loading_pages, ctx).await;
            });
        }
    }

    /// Try to get the full-size image for a page from the LRU cache.
    pub fn get_image_from_cache(&self, page_idx: usize) -> Option<image::DynamicImage> {
        use crate::cache::image_cache::PageImage;

        // Try LRU cache first
        if let Some(entry) = self.image_lru.lock().unwrap().get(&page_idx) {
            if let PageImage::Static(ref dyn_img) = entry.image {
                return Some(dyn_img.clone());
            }
        }

        // Optionally, try thumbnail cache (not full-size)
        if let Some(thumb) = self.thumbnail_cache.lock().unwrap().get(&page_idx) {
            return Some(thumb.clone());
        }

        // Not found in cache
        None
    }

    pub fn on_changes(&mut self) {
        // Handle goto page logic
        if self.on_goto_page {
            self.on_goto_page = false;
            // Parse the page number from the goto box
            if let Ok(page) = self.page_goto_box.trim().parse::<usize>() {
                let page = page.saturating_sub(1); // Convert to 0-based index
                if self.goto_page(page) {
                    if let Ok(mut logger) = self.ui_logger.lock() {
                        logger.info(format!("Navigated to page {}", page + 1), None);
                    }
                } else {
                    if let Ok(mut logger) = self.ui_logger.lock() {
                        logger.warn(format!("Failed to navigate to page {}", page + 1), None);
                    }
                }
            } else {
                if let Ok(mut logger) = self.ui_logger.lock() {
                    logger.warn("Invalid page number entered".to_string(), None);
                }
            }
        }

        if self.on_new_comic {
            self.on_new_comic = false;
            if let Some(path) = crate::comic_filters!().set_file_name("Comic").save_file() {
                let _ = ZipImageArchive::create_from_path(&path);
                let _ = self.load_new_file(path);
                return; // Prevent further update with old state
            }
        }
        if self.on_open_comic {
            self.on_open_comic = false;
            if let Some(path) = crate::comic_filters!().set_file_name("Comic").pick_file() {
                // let _ = self.load_new_file(path);
                self.new_page = Some(path);
                return; // Prevent further update with old state
            }
        }
        if self.on_open_folder {
            self.on_open_folder = false;
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                // let _ = self.load_new_file(path);
                self.new_page = Some(path);
                return;
            }
        }
        // Handle the async image saving outside the closure
        if self.on_save_image {
            self.on_save_image = false;
            let ui_logger = self.ui_logger.clone();
            let archive = self.archive.clone();
            let filenames = self.filenames.clone();
            let current_page = self.current_page;
            // Spawn a background task to avoid blocking the UI
            tokio::spawn(async move {
                if let Some(archive_mutex) = archive {
                    // Lock and extract the filename while holding the lock
                    let filename = filenames
                        .as_ref()
                        .and_then(|f| f.get(current_page).cloned())
                        .unwrap_or_else(|| "image".to_string());

                    // Clone the Arc<Mutex<ImageArchive>> for use in async block
                    let archive_clone = archive_mutex.clone();

                    // Clone the filename for use after the lock is dropped
                    let filename_clone = filename.clone();

                    // Lock the archive, extract what is needed, and drop the guard before await
                    let image_data = {
                        let mut archive_guard = archive_clone.lock().unwrap();
                        // Clone the data needed for the async call
                        let filename_owned = filename_clone.clone();
                        // Call the async function and await it while holding the lock
                        // If possible, refactor read_image_by_name to not borrow self
                        futures::executor::block_on(
                            archive_guard.read_image_by_name(&filename_owned),
                        )
                    };

                    if let Ok(image) = image_data {
                        let image_vec: Vec<u8> = image.into();

                        use std::path::Path;
                        let basename = Path::new(&filename)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&filename);
                        log::info!("Saving image as {}", basename);

                        if let Some(save_path) = rfd::FileDialog::new()
                            .set_title("Save Image")
                            .set_file_name(basename)
                            .save_file()
                        {
                            use tokio::io::AsyncWriteExt;
                            match tokio::fs::File::create(&save_path).await {
                                Ok(mut file) => {
                                    if let Err(e) = file.write_all(&image_vec).await {
                                        if let Ok(mut logger) = ui_logger.lock() {
                                            logger.error(
                                                format!("Failed to save image: {}", e),
                                                None,
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut logger) = ui_logger.lock() {
                                        logger.error(format!("Failed to save image: {}", e), None);
                                    }
                                }
                            }
                        } else {
                            if let Ok(mut logger) = ui_logger.lock() {
                                logger.warn("No file selected for saving".to_string(), None);
                            }
                        }
                    } else {
                        if let Ok(mut logger) = ui_logger.lock() {
                            logger.warn("No image to save".to_string(), None);
                        }
                    }
                }
            });
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        // Keyboard navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.archive_view.goto_next_page(self.double_page_mode);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.goto_prev_page();
        }
    }
}

impl eframe::App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if file is dragged and dropped
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    self.new_page = Some(path.clone());
                }
            }
        });
        // Load file synchronously to avoid borrow checker issues
        if let Some(path) = self.new_page.take() {
            if let Err(e) = futures::executor::block_on(self.load_new_file(path.clone())) {
                if let Ok(mut logger) = self.ui_logger.lock() {
                    logger.error(format!("Failed to load file: {}", e), None);
                }
            }
            self.new_page = None;
        }

        self.update_window_title(ctx);

        // Only preload images if we have an archive and not in manifest editor mode
        if self.show_manifest_editor {
            if let Ok(mut logger) = self.ui_logger.lock() {
                logger.clear_expired();
            }
        } else {
            if let Some(archive) = self.archive.as_ref() {
                let archive: Arc<Mutex<ImageArchive>> = Arc::clone(archive);
                self.preload_images(ctx, archive);
            }
        }

        if self.total_pages > 0 {
            if self.show_thumbnail_grid {
                self.display_thumbnail_grid(ctx);
            } else {
                self.display_main_full(ctx);
            }
        } else {
            self.display_main_empty(ctx);
        }

        if self.slideshow_mode && self.total_pages > 0 {
            let now = std::time::Instant::now();
            if self.current_page >= self.total_pages {
                self.current_page = 0; // Reset to first page if out of bounds
            }
            if now.duration_since(self.slideshow_last_tick).as_secs_f32() >= self.slideshow_interval_secs {
                self.archive_view.goto_next_page(self.double_page_mode);
                self.slideshow_last_tick = now;
            }
        }

        self.display_debug_menu(ctx);
        self.on_changes();

        // Draw the top and bottom bars
        self.display_top_bar(ctx);
        self.display_bottom_bar(ctx);

        if let Ok(mut logger) = self.ui_logger.lock() {
            logger.clear_expired();
        }

        // Show the slideshow interval settings popup if enabled
        if self.show_slideshow_interval_popup {
            egui::Window::new("Set Slideshow Interval")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Seconds per slide:");
                    let mut interval = self.slideshow_interval_secs;
                    if ui.add(egui::DragValue::new(&mut interval).clamp_range(1.0..=60.0)).changed() {
                        self.slideshow_interval_secs = interval;
                    }
                    if ui.button("OK").clicked() {
                        self.show_slideshow_interval_popup = false;
                    }
                });
        }
    }
}
