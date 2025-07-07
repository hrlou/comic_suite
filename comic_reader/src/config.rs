//! Application-wide configuration constants.

pub const NAME: &str = concat!("Comic Reader ", env!("CARGO_PKG_VERSION"));
/// Default window width.
pub const WIN_WIDTH: f32 = 720.0;
/// Default window height.
pub const WIN_HEIGHT: f32 = 1080.0;
/// Number of images to keep in cache.
pub const CACHE_SIZE: usize = 20;
/// Border size for image display.
// pub const BORDER_SIZE: f32 = 100.0;
/// Margin between pages in dual mode.
pub const PAGE_MARGIN_SIZE: usize = 0;
/// Default dual page mode.
pub const DEFAULT_DUAL_PAGE_MODE: bool = false;
/// Default reading direction.
pub const DEFAULT_RIGHT_TO_LEFT: bool = false;
/// Whether reading direction affects arrow keys.
// pub const READING_DIRECTION_AFFECTS_ARROWS: bool = true;
/// How many pages ahead to pre-cache.
pub const READ_AHEAD: usize = 16;
pub const LOG_TIMEOUT: usize = 2;
