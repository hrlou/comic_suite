// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


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


fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    let path: Option<PathBuf> = std::env::args().nth(1)
        .map(PathBuf::from);

    let app = CBZViewerApp::new(path).expect("Failed to load comic");
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "CBZ Viewer",
        native_options,
        Box::new(|_cc| Box::new(app)),
    );
}