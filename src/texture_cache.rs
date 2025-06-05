use eframe::egui::{Context, TextureHandle, TextureOptions, ColorImage};
use crate::image_cache::LoadedPage;

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
        Self { single: None, dual: None }
    }

    pub fn get_single(&self, page_idx: usize, zoom: f32) -> Option<&TextureHandle> {
        self.single.as_ref().and_then(|pt| {
            if pt.key.page_idx == page_idx && (pt.key.zoom - zoom).abs() < f32::EPSILON {
                Some(&pt.handle)
            } else {
                None
            }
        })
    }

    pub fn set_single(&mut self, page_idx: usize, zoom: f32, handle: TextureHandle) {
        self.single = Some(PageTexture {
            key: TextureKey { page_idx, zoom },
            handle,
        });
    }

    pub fn get_dual(
        &self,
        page1: usize,
        page2: Option<usize>,
        zoom: f32,
    ) -> Option<(&TextureHandle, Option<&TextureHandle>)> {
        self.dual.as_ref().and_then(|(pt1, opt_pt2)| {
            let match1 = pt1.key.page_idx == page1 && (pt1.key.zoom - zoom).abs() < f32::EPSILON;
            let match2 = match (&opt_pt2, page2) {
                (Some(pt2), Some(idx2)) => pt2.key.page_idx == idx2 && (pt2.key.zoom - zoom).abs() < f32::EPSILON,
                (None, None) => true,
                _ => false,
            };
            if match1 && match2 {
                Some((&pt1.handle, opt_pt2.as_ref().map(|pt| &pt.handle)))
            } else {
                None
            }
        })
    }

    pub fn set_dual(
        &mut self,
        page1: usize,
        handle1: TextureHandle,
        zoom: f32,
        page2: Option<(usize, TextureHandle)>,
    ) {
        let pt1 = PageTexture {
            key: TextureKey { page_idx: page1, zoom },
            handle: handle1,
        };
        let opt_pt2 = page2.map(|(idx, handle)| PageTexture {
            key: TextureKey { page_idx: idx, zoom },
            handle,
        });
        self.dual = Some((pt1, opt_pt2));
    }

    pub fn clear(&mut self) {
        self.single = None;
        self.dual = None;
    }
}