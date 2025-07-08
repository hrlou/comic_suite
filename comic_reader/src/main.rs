// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cache;
mod config;
mod error;
mod macros;
mod prelude;
mod ui;

use crate::prelude::*;

fn check_bin(bin: &str, msg: &str) {
    log::info!("Checking for '{}' in PATH...", bin);
    if which::which(bin).is_err() {
        rfd::MessageDialog::new()
            .set_title(&format!("Missing {}", bin))
            .set_description(&format!(
                "The '{}' executable was not found in your PATH.\n{}",
                bin, msg
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();
        log::warn!("'{}' not found in PATH. {}", bin, msg);
    } else {
        log::info!("'{}' found in PATH.", bin);
    }
}

fn main() {
    // Initialize logging (to file and console)
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    log::info!("Initialising...");

    #[cfg(feature = "rar")]
    {
        check_bin("unrar", "RAR archives will not open.");
        check_bin("rar", "RAR archives will save.");
    }

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
