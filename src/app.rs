//! Main application state and logic.

use crate::archive::ImageArchive;
use crate::cache::{SharedImageCache, TextureCache, new_image_cache, load_image_async, LoadedPage, PageImage};
use crate::ui::{draw_single_page, draw_dual_page, draw_spinner, log::UiLogger};
use crate::config::*;
use crate::error::AppError;

use eframe::{egui::{self, Vec2, Rect, Layout, pos2}, App};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use crate::ui::image::{handle_zoom, handle_pan};

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
    pub drag_start: Option<egui::Pos2>,
    pub double_page_mode: bool,
    pub right_to_left: bool,
    pub has_initialised_zoom: bool,
    pub loading_pages: Arc<Mutex<HashSet<usize>>>,
    // Add any other fields you need
}

impl CBZViewerApp {
    /// Create a new app instance from a given archive path.
    pub fn new(path: Option<PathBuf>) -> Result<Self, AppError> {
        let mut app = Self {
            archive_path: None,
            archive: None,
            filenames: None,
            image_lru: new_image_cache(CACHE_SIZE),
            current_page: 0,
            texture_cache: TextureCache::new(),
            ui_logger: UiLogger::new(),
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            drag_start: None,
            double_page_mode: DEFAULT_DUAL_PAGE_MODE,
            right_to_left: DEFAULT_RIGHT_TO_LEFT,
            has_initialised_zoom: false,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
        };
        if let Some(path) = path {
            let archive = Arc::new(Mutex::new(ImageArchive::process(&path)?));
            let filenames = archive.lock().unwrap().list_images();
            if filenames.is_empty() { 
                return Err(AppError::NoImages);
            }
            self.archive_path = Some(path);
            self.archive = Some(archive);
            self.filenames = Some(filenames);
        }
        todo!()
    }

    // Example: Reset zoom logic
    pub fn reset_zoom(&mut self, area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let avail = area.size();
        let scale_x = avail.x / w as f32;
        let scale_y = avail.y / h as f32;
        self.zoom = scale_x.min(scale_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
    }

    // Example: Clamp pan logic
    pub fn clamp_pan(&mut self, image_dims: (u32, u32), area: Rect) {
        let (w, h) = image_dims;
        let avail = area.size();
        let max_x = ((w as f32 * self.zoom - avail.x) / 2.0).max(0.0);
        let max_y = ((h as f32 * self.zoom - avail.y) / 2.0).max(0.0);
        self.pan_offset.x = self.pan_offset.x.clamp(-max_x, max_x);
        self.pan_offset.y = self.pan_offset.y.clamp(-max_y, max_y);
    }

    /// Go to the previous page (with bounds checking).
    pub fn goto_prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.on_page_changed();
        }
    }

    /// Go to the next page (with bounds checking).
    pub fn goto_next_page(&mut self) {
        if self.current_page + 1 < self.filenames.len() {
            self.current_page += 1;
            self.on_page_changed();
        }
    }

    /// Go to a specific page (with bounds checking).
    pub fn goto_page(&mut self, page: usize) {
        let page = page.min(self.filenames.len().saturating_sub(1));
        if self.current_page != page {
            self.current_page = page;
            self.on_page_changed();
        }
    }

    /// Called whenever the page changes: resets zoom, pan, and clears texture cache.
    pub fn on_page_changed(&mut self) {
        self.has_initialised_zoom = false;
        self.texture_cache.clear();
        self.pan_offset = Vec2::ZERO;
    }
}

impl eframe::App for CBZViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let total_pages = self.filenames.len();
        
        // Must be first; underneath all other UI elements
        crate::ui::layout::draw_central_image_area(self, ctx, total_pages);
        
        // Draw the top and bottom bars
        crate::ui::layout::draw_top_bar(self, ctx, total_pages);
        crate::ui::layout::draw_bottom_bar(self, ctx, total_pages);

        // Mouse wheel zoom
        handle_zoom(
            &mut self.zoom,
            ctx,
            0.05,
            10.0,
            &mut self.texture_cache,
            &mut self.has_initialised_zoom,
        );

        // Pan (drag to move image)
        // You need to pass the correct area (Rect) for your image.
        // For example, if you have it as `let image_area = ...;`:
        // crate::ui::image::handle_pan(&mut self.pan_offset, &mut self.drag_start, &mut self.ima, &mut self.texture_cache);


        // Optionally, show log messages at the bottom
        // egui::TopBottomPanel::bottom("log_bar").show(ctx, |ui| {
        //     for msg in self.ui_logger.messages() {
        //         ui.colored_label(egui::Color32::YELLOW, msg);
        //     }
        // });

        // Preload images for current view and next pages
        let mut pages_to_preload = vec![self.current_page];
        for offset in 1..=READ_AHEAD {
            let next = self.current_page + offset;
            if next < total_pages {
                pages_to_preload.push(next);
            }
        }
        for &page in &pages_to_preload {
            let _ = load_image_async(
                page,
                self.filenames.clone(),
                self.archive.clone(),
                self.image_lru.clone(),
                self.loading_pages.clone(),
            );
        }

        // --- Zoom with mouse wheel ---
        let input = ctx.input(|i| i.clone());
        if input.raw_scroll_delta.y != 0.0 {
            let zoom_factor = 1.1_f32.powf(input.raw_scroll_delta.y / 10.0);
            self.zoom = (self.zoom * zoom_factor).clamp(0.05, 10.0);
            self.texture_cache.clear();
        }

        // Keyboard navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.goto_next_page();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.goto_prev_page();
        }
    }
}