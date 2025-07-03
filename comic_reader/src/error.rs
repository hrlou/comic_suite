//! Unified error type for the CBZ Viewer application.

use thiserror::Error;

/// All errors that can occur in the application.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("GIF error: {0}")]
    Gif(#[from] gif::DecodingError),
    #[error("Manifest error: {0}")]
    ManifestError(String),
    #[error("No images found in archive")]
    NoImages,
    // #[error("Image not found: {0}")]
    // ImageNotFound(String),
    #[error("Unsupported archive type or not found")]
    UnsupportedArchive,
    #[error("Network error: {0}")]
    NetworkError(String),
    // #[error("Other error: {0}")]
    // Other(String),
}
