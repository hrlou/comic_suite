//! Texture cache for egui.

use crate::prelude::*;
use std::collections::HashMap;

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
    pub animated: HashMap<String, TextureHandle>, // Add this line
}

impl TextureCache {
    pub fn new() -> Self {
        debug!("TextureCache created");
        Self {
            single: None,
            dual: None,
            animated: HashMap::new(), // Initialize the new field
        }
    }

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

    pub fn set_single(&mut self, page_idx: usize, zoom: f32, handle: TextureHandle) {
        debug!("TextureCache set: single page {} @ zoom {}", page_idx, zoom);
        self.single = Some(PageTexture {
            key: TextureKey { page_idx, zoom },
            handle,
        });
    }

    fn quantize_zoom(zoom: f32) -> u32 {
        (zoom * 1000.0).round() as u32 // quantize to 3 decimal places
    }

    /*
    /// Get cached animated GIF frame texture by key.
    pub fn get_animated(&self, key: &str) -> Option<&TextureHandle> {
        self.animated.get(key)
    }

    /// Set cached animated GIF frame texture by key.
    pub fn set_animated(&mut self, key: String, handle: TextureHandle) {
        self.animated.insert(key, handle);
    }
    */

    pub fn clear(&mut self) {
        debug!("TextureCache cleared");
        self.single = None;
        self.dual = None;
        self.animated.clear(); // Clear animated cache as well
    }
}
