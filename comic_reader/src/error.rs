//! Unified error type for the CBZ Viewer application.

use comic_archive::error::ArchiveError;
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
    #[error("Index out of bound")]
    IndexOutOfBounds,
    #[error("Image processing error: {0}")]
    ImageProcessingError(String),
    #[error("Unsupported archive type or not found")]
    UnsupportedArchive,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<ArchiveError> for AppError {
    fn from(err: ArchiveError) -> Self {
        match err {
            ArchiveError::UnsupportedArchive => AppError::UnsupportedArchive,
            ArchiveError::NoImages => AppError::NoImages,
            ArchiveError::IndexOutOfBounds => AppError::IndexOutOfBounds,
            ArchiveError::ManifestError(e) => AppError::ManifestError(e),
            ArchiveError::NetworkError(e) => AppError::NetworkError(e),
            ArchiveError::Io(e) => AppError::Io(e),
            ArchiveError::Other(e) => AppError::Other(e),
            ArchiveError::Zip(e) => AppError::Zip(e),
            ArchiveError::ImageProcessingError(e) => AppError::ImageProcessingError(e),
            _ => AppError::Other("Unknown archive error".to_string()),
        }
    }
}
