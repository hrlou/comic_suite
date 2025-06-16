//! Entry point for the CBZ Viewer application.

mod config;
mod error;
mod archive;
mod cache;
mod ui;
// mod util;
mod app;

use crate::app::CBZViewerApp;
use config::{WIN_WIDTH, WIN_HEIGHT};
use eframe::egui::{self, Vec2};
use std::path::PathBuf;

/// Show a file dialog to pick a comic archive.
fn pick_comic() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Comic Book Archive", &["cbz", "zip"])
        .pick_file()
}

fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    let path = std::env::args().nth(1)
        .map(PathBuf::from)
        .or_else(pick_comic);

    if let Some(path) = path {
        let app = CBZViewerApp::new(path).expect("Failed to load comic");
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
            ..Default::default()
        };
        eframe::run_native(
            "CBZ Viewer",
            native_options,
            Box::new(|_cc| Box::new(app)),
        );
    }
}