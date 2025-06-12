use eframe::egui::TextureHandle;
use log::{debug};

#[derive(Clone, PartialEq)]
pub struct TextureKey {
    pub page_idx: usize,
    pub zoom: f32,
}

pub struct PageTexture {
    pub key: TextureKey,
    pub handle: TextureHandle,
}

pub struct TextureCache {
    pub single: Option<PageTexture>,
    pub dual: Option<(PageTexture, Option<PageTexture>)>,
}

impl TextureCache {
    pub fn new() -> Self {
        debug!("TextureCache created");
        Self { single: None, dual: None }
    }

    pub fn get_single(&self, page_idx: usize, zoom: f32) -> Option<&TextureHandle> {
        if let Some(pt) = &self.single {
            if pt.key.page_idx == page_idx && (pt.key.zoom - zoom).abs() < f32::EPSILON {
                debug!("TextureCache hit: single page {} @ zoom {}", page_idx, zoom);
                return Some(&pt.handle);
            }
        }
        debug!("TextureCache miss: single page {} @ zoom {}", page_idx, zoom);
        None
    }

    pub fn set_single(&mut self, page_idx: usize, zoom: f32, handle: TextureHandle) {
        debug!("TextureCache set: single page {} @ zoom {}", page_idx, zoom);
        self.single = Some(PageTexture {
            key: TextureKey { page_idx, zoom },
            handle,
        });
    }

    pub fn clear(&mut self) {
        debug!("TextureCache cleared");
        self.single = None;
        self.dual = None;
        // When clearing the image LRU cache:
        log::debug!("Image LRU cache cleared");
    }
}