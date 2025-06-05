use eframe::egui::{self, Ui, Vec2, Rect, Image, Spinner, Color32};
use crate::image_cache::LoadedPage;

/// Draw a centered spinner in the given area
pub fn draw_spinner(ui: &mut Ui, area: Rect) {
    let spinner_size = 48.0;
    let spinner_rect = Rect::from_center_size(area.center(), Vec2::splat(spinner_size));
    ui.allocate_ui_at_rect(spinner_rect, |ui| {
        ui.add(Spinner::new().size(spinner_size).color(Color32::WHITE));
    });
}

/// Draw a single page image centered in the area
pub fn draw_single_page(ui: &mut Ui, loaded: &LoadedPage, area: Rect, zoom: f32) {
    let (w, h) = loaded.image.dimensions();
    let disp_size = Vec2::new(w as f32 * zoom, h as f32 * zoom);
    let color_img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &loaded.image.to_rgba8());
    let handle = ui.ctx().load_texture(format!("tex{}", loaded.index), color_img, egui::TextureOptions::default());
    let rect = Rect::from_center_size(area.center(), disp_size);
    ui.allocate_ui_at_rect(rect, |ui| {
        ui.add(Image::from_texture(&handle).fit_to_exact_size(disp_size));
    });
}

/// Draw two pages side by side with a margin, order depends on reading direction
pub fn draw_dual_page(
    ui: &mut Ui,
    loaded_left: &LoadedPage,
    loaded_right: Option<&LoadedPage>,
    area: Rect,
    zoom: f32,
    margin: f32,
    left_first: bool,
) {
    let (w1, h1) = loaded_left.image.dimensions();
    let disp_size1 = Vec2::new(w1 as f32 * zoom, h1 as f32 * zoom);
    let color_img1 = egui::ColorImage::from_rgba_unmultiplied([w1 as usize, h1 as usize], &loaded_left.image.to_rgba8());
    let handle1 = ui.ctx().load_texture(format!("tex{}", loaded_left.index), color_img1, egui::TextureOptions::default());

    if let Some(loaded2) = loaded_right {
        let (w2, h2) = loaded2.image.dimensions();
        let disp_size2 = Vec2::new(w2 as f32 * zoom, h2 as f32 * zoom);
        let color_img2 = egui::ColorImage::from_rgba_unmultiplied([w2 as usize, h2 as usize], &loaded2.image.to_rgba8());
        let handle2 = ui.ctx().load_texture(format!("tex{}", loaded2.index), color_img2, egui::TextureOptions::default());

        let total_width = disp_size1.x + disp_size2.x + margin;
        let center = area.center();
        let left_start = center.x - total_width / 2.0;

        let (rect_left, rect_right) = (
            Rect::from_min_size(
                pos2(left_start, center.y - disp_size1.y / 2.0),
                disp_size1,
            ),
            Rect::from_min_size(
                pos2(left_start + disp_size1.x + margin, center.y - disp_size2.y / 2.0),
                disp_size2,
            ),
        );

        if left_first {
            ui.allocate_ui_at_rect(rect_left, |ui| {
                ui.add(Image::from_texture(&handle1).fit_to_exact_size(disp_size1));
            });
            ui.allocate_ui_at_rect(rect_right, |ui| {
                ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
            });
        } else {
            ui.allocate_ui_at_rect(rect_left, |ui| {
                ui.add(Image::from_texture(&handle2).fit_to_exact_size(disp_size2));
            });
            ui.allocate_ui_at_rect(rect_right, |ui| {
                ui.add(Image::from_texture(&handle1).fit_to_exact_size(disp_size1));
            });
        }
    } else {
        let rect = Rect::from_center_size(area.center(), disp_size1);
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.add(Image::from_texture(&handle1).fit_to_exact_size(disp_size1));
        });
    }
}