use thiserror::Error;

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
    #[error("Manifest not found")]
    ManifestNotFound,
    #[error("Manifest parse error: {0}")]
    ManifestParseError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Internal IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}
