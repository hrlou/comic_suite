// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive;
mod cache;
mod config;
mod error;
mod prelude;
mod ui;
mod model;
mod app;
mod macros;
mod utils;

use crate::prelude::*;

#[cfg(feature = "rar")]
fn check_unrar() {
    // Check for unrar in PATH
    log::info!("Checking for 'unrar' in PATH...");
    if which::which("unrar").is_err() {
        // Show error dialog and exit
        rfd::MessageDialog::new()
            .set_title("Missing unrar")
            .set_description(
                "The 'unrar' executable was not found in your PATH.\nRAR archives will not open.",
            )
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();
        // std::process::exit(1);
        log::warn!("'unrar' not found in PATH. RAR archives will not open.");
    } else {
        log::info!("'unrar' found in PATH.");
    }
}

fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    #[cfg(feature = "rar")]
    check_unrar();

    let path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIN_WIDTH, WIN_HEIGHT]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        NAME,
        native_options,
        Box::new(move |cc| {
            // Pass CreationContext to CBZViewerApp::new
            Ok(Box::new(
                CBZViewerApp::new(cc, path.clone()).expect("Failed to load comic"),
            ))
        }),
    );
}
