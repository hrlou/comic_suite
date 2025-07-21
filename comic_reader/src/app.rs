//! Main application state and logic.

use crate::{
    // archive::{self, ZipImageArchive},
    prelude::*,
};

/// The main application struct, holding all state.
pub struct CBZViewerApp {
    pub archive_path: Option<PathBuf>,
    pub archive: Option<Arc<Mutex<ImageArchive>>>,
    pub filenames: Option<Vec<String>>,
    pub image_lru: SharedImageCache,
    pub current_page: usize,
    pub texture_cache: TextureCache,
    pub ui_logger: UiLogger,
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
    pub is_web_archive: bool,
    pub total_pages: usize,
    pub show_thumbnail_grid: bool,
    pub thumbnail_cache: std::collections::HashMap<usize, image::DynamicImage>,
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
            ui_logger: UiLogger::new(),
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
            is_web_archive: false,
            total_pages: 0,
            show_thumbnail_grid: false,
            thumbnail_cache: std::collections::HashMap::new(),
        }
    }
}

impl CBZViewerApp {
    /// Create a new app instance from a given archive path.
    pub fn new(cc: &CreationContext, path: Option<PathBuf>) -> Result<Self, AppError> {
        crate::ui::setup_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        if let Some(path) = path {
            let _ = app.load_new_file(path);
        }
        #[cfg(feature = "7z")]
        {
            app.ui_logger.warn("7z archives are supported, however, it involves an external tool extracting files to a temporary directory.", Some(10));
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
            self.ui_logger
                .warn("Already at the first page, cannot go back.", None);
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
        if let Some(filenames) = &self.filenames {
            if page >= filenames.len() {
                self.ui_logger
                    .warn(format!("Requested page {} is out of bounds.", page), None);
                return false;
            }
            self.current_page = page;
            self.on_page_changed();
        } else {
            self.ui_logger
                .warn("No filenames available to go to specific page.", None);
            return false;
        }
        return true;
    }

    pub fn load_new_file(&mut self, path: PathBuf) -> Result<(), AppError> {
        let mut app = Self::default();
        let archive = ImageArchive::process(&path)?;

        let archive = Arc::new(Mutex::new(archive));
        if let Ok(guard) = archive.lock() {
            let filenames = guard.list_images();
            // if filenames.is_empty() {
            // return Err(AppError::NoImages);
            // }
            app.filenames = Some(filenames);
            app.is_web_archive = guard.manifest.meta.web_archive;
        }
        app.archive_path = Some(path);
        app.total_pages = app.filenames.as_ref().map_or(0, |f| f.len());
        app.archive = Some(Arc::clone(&archive));
        *self = app; // Replace current app state with the new one

        return Ok(());
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
            tokio::spawn(load_image_async(
                page,
                Arc::new(filenames.clone()),
                archive.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
                ctx.clone(),
            ));
        }
    }

    pub fn on_changes(&mut self) {
        if self.on_goto_page {
            self.on_goto_page = false;
            let page: usize = self.page_goto_box.parse().unwrap_or(0);
            if self.goto_page(page - 1) {
                self.ui_logger
                    .info(format!("Navigated to page {}", page), None);
            } else {
                self.ui_logger
                    .warn(format!("Failed to navigate to page {}", page), None);
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
                let _ = self.load_new_file(path);
                return; // Prevent further update with old state
            }
        }
        if self.on_open_folder {
            self.on_open_folder = false;
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let _ = self.load_new_file(path);
                return;
            }
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        // Keyboard navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.goto_next_page();
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
                    self.load_new_file(path.clone()).unwrap_or_else(|e| {
                        self.ui_logger
                            .error(format!("Failed to load file: {}", e), None);
                    });
                }
            }
        });

        self.update_window_title(ctx);

        /*if let Some(archive) = self.archive.as_ref() {
            let archive: Arc<Mutex<ImageArchive>> = Arc::clone(archive);
            self.preload_images(archive);
            self.display_main_full(ctx);
        } else {
            self.display_main_empty(ctx);
        }

        // Handle Manifest
        if self.show_manifest_editor {
            self.display_manifest_editor(ctx);
        }*/

        // Only preload images if we have an archive and not in manifest editor mode
        if self.show_manifest_editor {
            self.display_manifest_editor(ctx);
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

        self.on_changes();

        // Draw the top and bottom bars
        self.display_top_bar(ctx);
        self.display_bottom_bar(ctx);

        self.ui_logger.clear_expired();
    }
}
