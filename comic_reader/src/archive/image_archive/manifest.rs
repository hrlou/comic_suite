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

pub struct ManifestEditor {
    manifest: &mut Manifest,
    // url_input: String,
}

impl ManifestEditor {
    pub fn new(manifest: &mut Manifest) -> Self {
        Self {
            manifest,
            // url_input: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.label("Title:");
        ui.text_edit_singleline(&mut self.manifest.meta.title);

        ui.label("Author:");
        ui.text_edit_singleline(&mut self.manifest.meta.author);

        ui.checkbox(&mut self.manifest.meta.web_archive, "Web Archive");

        // ui.separator();

        // ui.label("Add External Page URL:");
        // ui.horizontal(|ui| {
        //     ui.text_edit_singleline(&mut self.url_input);
        //     if ui.button("Add").clicked() {
        //         if !self.url_input.trim().is_empty() {
        //             if let Some(ref mut pages) = self.manifest.external_pages {
        //                 pages.urls.push(self.url_input.trim().to_string());
        //             } else {
        //                 self.manifest.external_pages = Some(ExternalPages {
        //                     urls: vec![self.url_input.trim().to_string()],
        //                 });
        //             }
        //             self.url_input.clear();
        //         }
        //     }
        // });

        // if let Some(ref pages) = self.manifest.external_pages {
        //     ui.label("External Pages:");
        //     for (i, url) in pages.urls.iter().enumerate() {
        //         ui.horizontal(|ui| {
        //             ui.label(format!("{}: {}", i + 1, url));
        //             if ui.button("Remove").clicked() {
        //                 if let Some(ref mut pages) = self.manifest.external_pages {
        //                     pages.urls.remove(i);
        //                     if pages.urls.is_empty() {
        //                         self.manifest.external_pages = None;
        //                     }
        //                 }
        //             }
        //         });
        //     }
        // }
    }
}
