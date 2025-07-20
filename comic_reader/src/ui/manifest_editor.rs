use crate::prelude::*;

macro_rules! editable_list {
    ($ui:expr, $label:expr, $vec:expr, $hint:expr, $do_add:expr, $reorder:expr) => {{
        let mut to_remove = None;
        let mut to_move_up = None;
        let mut to_move_down = None;

        $ui.separator();
        $ui.label($label);

        for i in 0..$vec.len() {
            $ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut $vec[i])
                        .hint_text($hint)
                        .desired_width(ui.available_width() - 180.0),
                );
                if $reorder {
                    if ui.button("↑").clicked() && i > 0 {
                        to_move_up = Some(i);
                    }
                    if ui.button("↓").clicked() && i + 1 < $vec.len() {
                        to_move_down = Some(i);
                    }
                }
                if ui.button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(i) = to_move_up {
            $vec.swap(i, i - 1);
        }
        if let Some(i) = to_move_down {
            $vec.swap(i, i + 1);
        }
        if let Some(i) = to_remove {
            $vec.remove(i);
        }

        $ui.horizontal(|ui| {
            if $do_add {
                if ui.button(format!("+ Add {}", $label)).clicked() {
                    $vec.push(String::new());
                }
            }
            if ui.button("Clear All").clicked() {
                $vec.clear();
            }
        });
    }};
}

pub struct ManifestEditor<'a> {
    archive: &'a mut ImageArchive,
}

impl<'a> ManifestEditor<'a> {
    pub fn new(archive: &'a mut ImageArchive) -> Self {
        Self { archive }
    }

    pub fn ui(&mut self, ui: &mut Ui, _ctx: &Context) -> Result<(), AppError> {
        let mut manifest = self.archive.manifest.clone();
        {
            ui.label("Title:");
            ui.text_edit_singleline(&mut manifest.meta.title);

            ui.label("Author:");
            ui.text_edit_singleline(&mut manifest.meta.author);

            ui.checkbox(&mut manifest.meta.web_archive, "Web Archive");

            let mut num_pages = self.archive.list_images().len();

            // External Page URLs
            if manifest.meta.web_archive {
                num_pages = manifest
                    .external_pages
                    .as_ref()
                    .map(|e| e.urls.len())
                    .unwrap_or(0);

                let urls = manifest
                    .external_pages
                    .get_or_insert_with(ExternalPages::default);
                editable_list!(ui, "External Page URLs", urls.urls, "URL", true, true);
            }
        }

        ui.separator();

        if ui.button("Rebuild").clicked() {
            self.archive.write_manifest(&manifest)?;
        }
        self.archive.manifest = manifest;

        Ok(())
    }
}
