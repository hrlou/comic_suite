use thiserror::Error;

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

    #[error("No images found in archive")]
    NoImages,

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Unsupported archive type or not found")]
    UnsupportedArchive,

    #[error("Other error: {0}")]
    Other(String),
}

// Optionally, you can define module-specific errors and convert them to AppError if needed.