use crate::prelude::*;

pub struct ArchiveView {
    pub archive_path: Option<PathBuf>,
    pub archive: Option<Arc<Mutex<ImageArchive>>>,
    pub filenames: Option<Vec<String>>,
    pub current_page: usize,
    pub texture_cache: TextureCache,
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub original_pan_offset: Vec2,
    pub drag_start: Option<egui::Pos2>,
    pub has_initialised_zoom: bool,
    pub loading_pages: Arc<Mutex<HashSet<usize>>>,
    pub total_pages: usize,
}

impl Default for ArchiveView {
    fn default() -> Self {
        Self {
            archive_path: None,
            archive: None,
            filenames: None,
            current_page: 0,
            texture_cache: TextureCache::new(),
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            original_pan_offset: Vec2::ZERO,
            drag_start: None,
            has_initialised_zoom: false,
            loading_pages: Arc::new(Mutex::new(HashSet::new())),
            total_pages: 0,
        }
    }
}

impl ArchiveView {
    pub fn goto_next_page(&mut self, double_page_mode: bool) {
        let step = if double_page_mode { 2 } else { 1 };
        let new_page = self.current_page + step;
        self.goto_page(new_page);
    }

    pub fn goto_prev_page(&mut self, double_page_mode: bool) {
        let step = if double_page_mode { 2 } else { 1 };
        let new_page = self.current_page.saturating_sub(step);
        self.goto_page(new_page);
    }

    pub fn goto_page(&mut self, page: usize) -> bool {
        if let Some(filenames) = &self.filenames {
            if page >= filenames.len() {
                return false;
            }
            self.current_page = page;
            true
        } else {
            false
        }
    }

    pub fn reset_zoom(&mut self, area: Rect, loaded: &LoadedPage) {
        let (w, h) = loaded.image.dimensions();
        let avail = area.size();
        let scale_x = avail.x / w as f32;
        let scale_y = avail.y / h as f32;
        self.zoom = scale_x.min(scale_y).min(1.0);
        self.pan_offset = Vec2::ZERO;
        self.has_initialised_zoom = true;
    }

    pub fn on_page_changed(&mut self) {
        self.has_initialised_zoom = false;
        self.texture_cache.clear();
        self.pan_offset = Vec2::ZERO;
    }

    pub fn preload_images(&mut self, ctx: &egui::Context, is_web_archive: bool) {
        let filenames = self.filenames.clone().unwrap_or_default();
        let mut pages_to_preload = vec![self.current_page];
        let read_ahead = if is_web_archive { READ_AHEAD_WEB } else { READ_AHEAD };
        for offset in 1..=read_ahead {
            let next = self.current_page + offset;
            if next < self.total_pages {
                pages_to_preload.push(next);
            }
        }
        for &page in &pages_to_preload {
            let filenames = Arc::new(filenames.clone());
            let archive = self.archive.clone().unwrap();
            let image_lru = new_image_cache(CACHE_SIZE);
            let loading_pages = self.loading_pages.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let _ = load_image_async(page, filenames, archive, image_lru, loading_pages, ctx).await;
            });
        }
    }

    pub fn get_image_from_cache(&self, image_lru: &SharedImageCache, thumbnail_cache: &Arc<Mutex<std::collections::HashMap<usize, image::DynamicImage>>>, page_idx: usize) -> Option<image::DynamicImage> {
        use crate::cache::image_cache::PageImage;
        if let Some(entry) = image_lru.lock().unwrap().get(&page_idx) {
            if let PageImage::Static(ref dyn_img) = entry.image {
                return Some(dyn_img.clone());
            }
        }
        if let Some(thumb) = thumbnail_cache.lock().unwrap().get(&page_idx) {
            return Some(thumb.clone());
        }
        None
    }
}