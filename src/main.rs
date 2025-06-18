// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive;
mod cache;
mod config;
mod error;
mod ui;
mod prelude;
// mod util;
mod app;

use crate::prelude::*;

fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    // let path: Option<PathBuf> = std::env::args().nth(1)
    //     .map(PathBuf::from);

    // let app = CBZViewerApp::new(path).expect("Failed to load comic");
    // let native_options = eframe::NativeOptions {
    //     viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
    //     ..Default::default()
    // };
    // let _ = eframe::run_native(
    //     "CBZ Viewer",
    //     native_options,
    //     Box::new(|_cc| Box::new(app)),
    // );

    let path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "CBZ Viewer",
        native_options,
        Box::new(move |cc| {
            // Pass CreationContext to CBZViewerApp::new
            Box::new(CBZViewerApp::new(cc, path.clone()).expect("Failed to load comic"))
        }),
    );
}
