use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub web_archive: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalPages {
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub meta: Metadata,
    pub external_pages: Option<ExternalPages>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            meta: Metadata {
                title: "Unknown".to_string(),
                author: "Unknown".to_string(),
                web_archive: false,
            },
            external_pages: None,
        }
    }
}

pub struct ManifestEditor<'a> {
    manifest: &'a mut Manifest,
    // url_input: String,
}

impl<'a> ManifestEditor<'a> {
    pub fn new(manifest: &'a mut Manifest) -> Self {
        Self {
            manifest,
            // url_input: String::new(),
        }
    }

    pub fn ui(&mut self, path: &Path, ui: &mut Ui, _ctx: &Context) -> Result<(), AppError> {
        ui.label("Title:");
        ui.text_edit_singleline(&mut self.manifest.meta.title);

        ui.label("Author:");
        ui.text_edit_singleline(&mut self.manifest.meta.author);

        ui.checkbox(&mut self.manifest.meta.web_archive, "Web Archive");

        // Only show URL editor if 'web_archive' is checked
        if self.manifest.meta.web_archive {
            let urls = self
                .manifest
                .external_pages
                .get_or_insert_with(|| ExternalPages { urls: vec![] });

            ui.separator();
            ui.label("External Page URLs:");

            // Render each URL entry with options to reorder or delete
            let mut to_remove = None;
            let mut to_move_up = None;
            let mut to_move_down = None;

            for i in 0..urls.urls.len() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut urls.urls[i])
                            .desired_width(ui.available_width() - 120.0),
                    );
                    if ui.button("↑").clicked() && i > 0 {
                        to_move_up = Some(i);
                    }
                    if ui.button("↓").clicked() && i + 1 < urls.urls.len() {
                        to_move_down = Some(i);
                    }
                    if ui.button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }

            if let Some(i) = to_move_up {
                urls.urls.swap(i, i - 1);
            }
            if let Some(i) = to_move_down {
                urls.urls.swap(i, i + 1);
            }
            if let Some(i) = to_remove {
                urls.urls.remove(i);
            }

            ui.horizontal(|ui| {
                if ui.button("+ Add Page").clicked() {
                    urls.urls.push(String::new());
                }
                if ui.button("Clear All").clicked() {
                    urls.urls.clear();
                }
            });
        }

        ui.separator();

        if ui.button("Rebuild").clicked() {
            crate::utils::rebuild_zip_with_manifest(path, self.manifest)?;
        }

        Ok(())
    }
}
