use crate::prelude::*;
use egui::{Color32, RichText};
use sysinfo::{RefreshKind, System};

impl CBZViewerApp {
    pub fn display_debug_menu(&mut self, ctx: &egui::Context) {
        if self.show_debug_menu {
            let mut show = self.show_debug_menu;
            egui::Window::new(
                RichText::new("\u{f06e} Debug Info")
                    .color(Color32::from_rgb(0, 180, 255))
                    .heading(),
            )
            .open(&mut show)
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.debug_thumbnail_cache(ui);
                    ui.separator();
                    self.debug_lru_cache(ui);
                    ui.separator();
                    self.debug_ram_usage(ui);
                    // ui.separator();
                    // self.debug_network_usage(ui);
                });
            });
            self.show_debug_menu = show;
        }
    }

    fn debug_thumbnail_cache(&self, ui: &mut egui::Ui) {
        ui.collapsing(
            RichText::new("\u{f03e} Thumbnail Cache")
                .color(Color32::from_rgb(255, 200, 0))
                .strong(),
            |ui| {
                let cache = self.thumbnail_cache.lock().unwrap();
                ui.label(
                    RichText::new(format!("Entries: {}", cache.len())).color(Color32::LIGHT_BLUE),
                );
                let mut total_thumb_bytes = 0usize;

                egui::Grid::new("thumb_cache_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("\u{0023} Page").strong());
                        ui.label(RichText::new("\u{f545} Size").strong());
                        ui.label(RichText::new("\u{f0c7} Bytes").strong());
                        ui.label(RichText::new("\u{f1b2} MB").strong());
                        ui.end_row();

                        for (k, v) in cache.iter() {
                            let bytes = v.as_bytes().len();
                            total_thumb_bytes += bytes;
                            ui.label(RichText::new(format!("{k}")).color(Color32::YELLOW));
                            ui.label(format!("{}x{}", v.width(), v.height()));
                            ui.label(
                                RichText::new(format!("{}", bytes)).color(Color32::LIGHT_GREEN),
                            );
                            ui.label(format!("{:.2}", bytes as f64 / (1024.0 * 1024.0)));
                            ui.end_row();
                        }
                    });

                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "\u{f1ec} Total: {} bytes ({:.2} MB)",
                        total_thumb_bytes,
                        total_thumb_bytes as f64 / (1024.0 * 1024.0)
                    ))
                    .color(Color32::from_rgb(0, 200, 0))
                    .strong(),
                );
            },
        );
    }

    fn debug_lru_cache(&self, ui: &mut egui::Ui) {
        ui.collapsing(
            RichText::new("\u{f07c} Image LRU Cache")
                .color(Color32::from_rgb(0, 220, 255))
                .strong(),
            |ui| {
                let image_lru = self.image_lru.lock().unwrap();
                ui.label(
                    RichText::new(format!("Entries: {}", image_lru.len()))
                        .color(Color32::LIGHT_BLUE),
                );
                let mut total_lru_bytes = 0usize;

                egui::Grid::new("lru_cache_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("\u{0023} Page").strong());
                        ui.label(RichText::new("\u{f545} Size").strong());
                        ui.label(RichText::new("\u{f0c7} Bytes").strong());
                        ui.label(RichText::new("\u{f1b2} MB").strong());
                        ui.end_row();

                        for (k, v) in image_lru.iter() {
                            let (w, h) = v.image.dimensions();
                            let bytes = (w as usize) * (h as usize) * 4; // RGBA8
                            total_lru_bytes += bytes;
                            ui.label(RichText::new(format!("{k}")).color(Color32::YELLOW));
                            ui.label(format!("{}x{}", w, h));
                            ui.label(
                                RichText::new(format!("{}", bytes)).color(Color32::LIGHT_GREEN),
                            );
                            ui.label(format!("{:.2}", bytes as f64 / (1024.0 * 1024.0)));
                            ui.end_row();
                        }
                    });

                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "\u{f1ec} Total: {} bytes ({:.2} MB)",
                        total_lru_bytes,
                        total_lru_bytes as f64 / (1024.0 * 1024.0)
                    ))
                    .color(Color32::from_rgb(0, 200, 0))
                    .strong(),
                );
            },
        );
    }

    fn debug_ram_usage(&self, ui: &mut egui::Ui) {
        ui.heading(
            RichText::new("\u{f5dc} RAM Usage")
                .color(Color32::from_rgb(255, 100, 100))
                .strong(),
        );
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let pid = sysinfo::get_current_pid().unwrap();
        if let Some(proc) = sys.process(pid) {
            ui.label(
                RichText::new(format!(
                    "Process memory: {:.2} MB",
                    proc.memory() as f64 / 1024.0 / 1024.0
                ))
                .color(Color32::LIGHT_RED)
                .strong(),
            );
        } else {
            ui.label(RichText::new("Unable to get process memory info.").color(Color32::RED));
        }
    }

    fn debug_network_usage(&self, ui: &mut egui::Ui) {
        todo!("Network usage debugging is currently disabled due to sysinfo limitations.");
        use egui::{Color32, RichText};
        use sysinfo::System;
        /*
            ui.heading(
                RichText::new("\u{f6ff} Network Usage")
                    .color(Color32::from_rgb(100, 200, 255))
                    .strong(),
            );

            let mut sys = System::new();
            sys.refresh_networks();

            let mut total_received = 0u64;
            let mut total_transmitted = 0u64;

            egui::Grid::new("network_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("\u{f0ac} Interface").strong());
                    ui.label(RichText::new("\u{f019} Received (MB)").strong());
                    ui.label(RichText::new("\u{f093} Sent (MB)").strong());
                    ui.end_row();

                    for (name, data) in sys.networks().iter() {
                        let received = data.received();
                        let transmitted = data.transmitted();
                        total_received += received;
                        total_transmitted += transmitted;
                        ui.label(RichText::new(name).color(Color32::YELLOW));
                        ui.label(format!("{:.2}", received as f64 / 1024.0 / 1024.0));
                        ui.label(format!("{:.2}", transmitted as f64 / 1024.0 / 1024.0));
                        ui.end_row();
                    }
                });

            ui.label(
                RichText::new(format!(
                    "Total: \u{f019} {:.2} MB, \u{f093} {:.2} MB",
                    total_received as f64 / 1024.0 / 1024.0,
                    total_transmitted as f64 / 1024.0 / 1024.0
                ))
                .color(Color32::from_rgb(0, 200, 255))
                .strong(),
            );
        }*/
    }
}
