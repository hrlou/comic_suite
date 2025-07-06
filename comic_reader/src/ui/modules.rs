use crate::{archive::manifest, prelude::*};

pub fn ui_goto_page(app: &mut CBZViewerApp, ui: &mut Ui) {
    let char_width = ui.fonts(|f| {
        let font_id = FontId::monospace(ui.style().text_styles[&TextStyle::Monospace].size);
        f.glyph_width(&font_id, '0')
    });
    let desired_width = char_width * 5.0 + 10.0;

    let response = ui.add_sized(
        [desired_width, ui.spacing().interact_size.y],
        TextEdit::singleline(&mut app.page_goto_box)
            .hint_text("###")
            .font(TextStyle::Monospace),
    );
    app.page_goto_box.retain(|c| c.is_ascii_digit());
    app.on_goto_page = ui
        .button("Jump")
        .on_hover_text("Jump to page number")
        .clicked()
        || (response.has_focus() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)));
}

pub fn ui_zoom_slider(app: &mut CBZViewerApp, ui: &mut Ui) {
    ui.add(egui::Slider::new(&mut app.zoom, 0.05..=10.0));
    if ui.button("Reset Zoom").clicked() {
        app.zoom = 1.0;
        app.pan_offset = Vec2::ZERO;
        app.has_initialised_zoom = false;
        app.texture_cache.clear();
    }
}

pub fn ui_page_nav(app: &mut CBZViewerApp, ui: &mut Ui, total_pages: usize) {
    if ui.button("\u{f061}").clicked() {
        app.goto_next_page();
    }
    if ui.button("\u{f060}").clicked() {
        app.goto_prev_page();
    }
    let page_label = if app.double_page_mode && app.current_page != 0 {
        let left = app.current_page;
        let right = (app.current_page + 1).min(total_pages.saturating_sub(1));
        if app.right_to_left {
            format!("Page ({},{})/{}", right + 1, left + 1, total_pages)
        } else {
            format!("Page ({},{})/{}", left + 1, right + 1, total_pages)
        }
    } else {
        format!("Page {}/{}", app.current_page + 1, total_pages)
    };
    ui.label(page_label);
}

pub fn ui_log_msg(app: &mut CBZViewerApp, ui: &mut Ui) {
    if let Some((msg, kind)) = &app.ui_logger.message {
        ui.separator();
        let color = match *kind {
            crate::ui::UiLogLevel::Info => egui::Color32::WHITE,
            crate::ui::UiLogLevel::Warning => egui::Color32::YELLOW,
            crate::ui::UiLogLevel::Error => egui::Color32::RED,
        };
        ui.colored_label(color, msg.clone());
    }
}

pub fn ui_file(app: &mut CBZViewerApp, ui: &mut Ui, _ctx: &Context) {
    ui.menu_button("File", |ui| {
        if ui.button("New Comic...").clicked() {
            app.on_new_comic = true;
            ui.close_menu();
        }
        if ui.button("Open Comic...").clicked() {
            app.on_open_comic = true;
            ui.close_menu();
        }
        if ui.button("Open Folder...").clicked() {
            app.on_open_folder = true;
            ui.close_menu();
        }
        if ui.button("Reload...").clicked() {
            if let Some(path) = app.archive_path.clone() {
                let _ = app.load_new_file(path);
            } else {
                app.ui_logger.warn("Failed to reload");
            }
            ui.close_menu();
        }
    });
}

pub fn ui_edit(app: &mut CBZViewerApp, ui: &mut Ui, _ctx: &Context) {
    ui.menu_button("Edit", |ui| {
        if ui.button("Edit Manifest...").clicked() {
            app.show_manifest_editor = true;
            ui.close_menu();
        }
    });
}

pub fn ui_navigation(app: &mut CBZViewerApp, ui: &mut Ui, total_pages: usize) {
    let direction_label = if app.right_to_left {
        "\u{f191}"
    } else {
        "\u{f152}"
    };
    if ui
        .button(direction_label)
        .on_hover_text("Reading direction")
        .clicked()
    {
        app.right_to_left = !app.right_to_left;
        app.texture_cache.clear();
    }

    if ui
        .selectable_label(app.double_page_mode, "\u{f518}")
        .on_hover_text("Dual page mode")
        .clicked()
    {
        if app.double_page_mode {
            app.double_page_mode = false;
            app.current_page = app.current_page.min(total_pages.saturating_sub(1));
            app.has_initialised_zoom = false;
            app.texture_cache.clear();
        } else {
            if app.current_page > 0 && app.current_page % 2 != 0 {
                app.current_page -= 1;
            }
            app.double_page_mode = true;
            app.has_initialised_zoom = false;
            app.texture_cache.clear();
        }
    }

    if app.double_page_mode {
        if ui
            .button("\u{f08e}")
            .on_hover_text("Bump over a single page, use this if there is misalignment")
            .clicked()
        {
            if app.current_page + 1 < total_pages {
                app.current_page += 1;
                app.has_initialised_zoom = false;
                app.texture_cache.clear();
            }
        }
    }
}
