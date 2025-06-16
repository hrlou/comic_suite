// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod archive;
mod texture_cache;
mod image_cache;
mod ui;
mod app;
mod error;

use app::CBZViewerApp;
use config::{WIN_WIDTH, WIN_HEIGHT};
use eframe::egui::Vec2;
use std::path::PathBuf;

fn pick_comic() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Comic Book Archive", &["cbz", "zip"])
        .pick_file()
}

fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Debug)
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    let path = std::env::args().nth(1)
        .map(PathBuf::from)
        .or_else(pick_comic);

    if let Some(path) = path {
        match CBZViewerApp::new(path) {
            Ok(app) => {
                let _ = eframe::run_native(
                    "CBZ Viewer",
                    eframe::NativeOptions {
                        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
                        ..Default::default()
                    },
                    Box::new(|_cc| Box::new(app)),
                );
            }
            Err(e) => {
                eprintln!("Failed to open archive: {e}");
                // Optionally show a dialog or UI error here
            }
        }
    }
}