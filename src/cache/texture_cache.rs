//! Texture cache for egui.

use crate::prelude::*;

/// Key for a cached texture (page index and zoom).
#[derive(Clone, PartialEq)]
pub struct TextureKey {
    pub page_idx: usize,
    pub zoom: f32,
}

/// A cached page texture.
pub struct PageTexture {
    pub key: TextureKey,
    pub handle: TextureHandle,
}

/// Texture cache for single and dual page modes.
pub struct TextureCache {
    pub single: Option<PageTexture>,
    pub dual: Option<(PageTexture, Option<PageTexture>)>,
}

impl TextureCache {
    /// Create a new, empty texture cache.
    pub fn new() -> Self {
        debug!("TextureCache created");
        Self {
            single: None,
            dual: None,
        }
    }

    /// Get a single page texture if present.
    pub fn get_single(&self, page_idx: usize, zoom: f32) -> Option<&TextureHandle> {
        if let Some(pt) = &self.single {
            if pt.key.page_idx == page_idx && (pt.key.zoom - zoom).abs() < f32::EPSILON {
                debug!("TextureCache hit: single page {} @ zoom {}", page_idx, zoom);
                return Some(&pt.handle);
            }
        }
        debug!(
            "TextureCache miss: single page {} @ zoom {}",
            page_idx, zoom
        );
        None
    }

    /// Set the single page texture.
    pub fn set_single(&mut self, page_idx: usize, zoom: f32, handle: TextureHandle) {
        debug!("TextureCache set: single page {} @ zoom {}", page_idx, zoom);
        self.single = Some(PageTexture {
            key: TextureKey { page_idx, zoom },
            handle,
        });
    }

    /// Clear all cached textures.
    pub fn clear(&mut self) {
        debug!("TextureCache cleared");
        self.single = None;
        self.dual = None;
    }
}
