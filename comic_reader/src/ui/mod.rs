//! UI rendering and layout.

pub mod image;
// pub mod layout;
pub mod display;
pub mod log;
pub mod modules;

pub use image::*;
// pub use layout::*;
pub use log::*;

use crate::prelude::*;

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "fa-solid".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fa-solid-900.ttf"
        ))),
    );

    fonts.families.insert(
        FontFamily::Name("FontAwesome".into()),
        vec!["fa-solid".to_owned()],
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(1, "fa-solid".to_owned());
    ctx.set_fonts(fonts);
}
