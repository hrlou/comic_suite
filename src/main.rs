// Hide the console window on Windows
#![windows_subsystem = "windows"]

mod config;
mod image_cache;
mod ui;
mod app;

use app::CBZViewerApp;
use config::{WIN_WIDTH, WIN_HEIGHT};
use eframe::egui::Vec2;
use std::path::PathBuf;

// Constants for initial window size
const CACHE_SIZE: usize = 20; // Number of images to cache
const PAGE_MARGIN_SIZE: usize = 16; // Margin in pixels between pages
const DEFAULT_DUAL_PAGE_MODE: bool = false;
const DEFAULT_RIGHT_TO_LEFT: bool = false;
const READING_DIRECTION_AFFECTS_ARROWS: bool = true;

fn pick_comic() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Comic Book Archive", &["cbz", "zip"])
        .pick_file()
}

fn main() {
    let path = std::env::args().nth(1)
        .map(PathBuf::from)
        .or_else(pick_comic);

    if let Some(path) = path {
        let _ = eframe::run_native(
            "CBZ Viewer",
            eframe::NativeOptions {
                initial_window_size: Some(Vec2::new(WIN_WIDTH, WIN_HEIGHT)),
                ..Default::default()
            },
            Box::new(|_cc| Box::new(CBZViewerApp::new(path))),
        );
    }
}