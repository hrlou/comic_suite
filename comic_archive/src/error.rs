use thiserror::Error;
use std::io;
use zip::result::ZipError;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Unsupported archive format")]
    UnsupportedArchive,
    #[error("No images found in archive")]
    NoImages,
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    #[error("Image processing error: {0}")]
    ImageProcessingError(String),
    #[error("Manifest error: {0}")]
    ManifestError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] ZipError),
    #[error("Other error: {0}")]
    Other(String),
}