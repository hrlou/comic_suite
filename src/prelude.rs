// std
pub use std::collections::HashSet;
pub use std::fs::{self, File};
pub use std::io::Read;
pub use std::num::NonZeroUsize;
pub use std::path::{Path, PathBuf};
pub use std::sync::{Arc, Mutex};
pub use std::time::Instant;


// external crates
pub use eframe::{
    egui::{
        self, CentralPanel, Color32, ColorImage, Context, FontData, FontDefinitions, FontFamily,
        FontId, Image, Layout, Rect, RichText, Spinner, TextEdit, TextureHandle, TextStyle, Ui, Vec2,
    },
    CreationContext,
};
pub use image::{DynamicImage, GenericImageView};
pub use log::{debug, warn};
pub use lru::LruCache;

// crate modules
pub use crate::{
    app::CBZViewerApp,
    archive::{ImageArchive},
    cache::{
        image_cache::{LoadedPage, PageImage},
        texture_cache::TextureCache,
        load_image_async,
        new_image_cache,
        SharedImageCache,
    },
    config::*,
    error::AppError,
    ui::{
        draw_bottom_bar,
        draw_central_image_area,
        draw_dual_page,
        draw_single_page,
        draw_spinner,
        draw_top_bar,
        handle_pan,
        handle_zoom,
        log::UiLogger,
    },
};
