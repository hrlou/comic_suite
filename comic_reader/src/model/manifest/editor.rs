use core::arch;

use crate::prelude::*;

pub struct ManifestEditor<'a> {
    archive: &'a mut ImageArchive,
}

impl<'a> ManifestEditor<'a> {
    pub fn new(archive: &'a mut ImageArchive) -> Self {
        Self { archive }
    }

    pub fn ui(&mut self, ui: &mut Ui, _ctx: &Context) -> Result<(), AppError> {
        // Scope the mutable borrow of manifest
        let mut manifest = self.archive.manifest.clone();
        {
            ui.label("Title:");
            ui.text_edit_singleline(&mut manifest.meta.title);

            ui.label("Author:");
            ui.text_edit_singleline(&mut manifest.meta.author);

            ui.checkbox(&mut manifest.meta.web_archive, "Web Archive");

            if manifest.meta.web_archive {
                let urls = manifest
                    .external_pages
                    .get_or_insert_with(ExternalPages::default);

                ui.separator();
                ui.label("External Page URLs:");

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
        } // <-- mutable borrow of manifest ends here

        ui.separator();

        if ui.button("Rebuild").clicked() {
            self.archive.write_manifest(&manifest)?;
        }
        self.archive.manifest = manifest;

        Ok(())
    }
}
