//! Unified image archive interface for CBZ, folders, RAR, and web archives.

pub mod error;
pub mod model;
pub mod prelude;

mod zip_archive;
pub use zip_archive::ZipImageArchive;

mod web_archive;
pub use web_archive::WebImageArchive;

#[cfg(feature = "rar")]
mod rar_archive;
#[cfg(feature = "rar")]
pub use rar_archive::RarImageArchive;

use image::codecs::jpeg::JpegEncoder;
use std::path::{Path, PathBuf};

use crate::prelude::*;

/// Macro to simplify archive backend instantiation and manifest extraction.
///
/// This macro creates an archive backend of the given type, reads its manifest,
/// and wraps it in a `WebImageArchive` if the manifest indicates a web archive.
/// Returns an `ImageArchive` instance on success.
#[macro_export]
macro_rules! archive_case {
    (
        $archive_ty:ty, $path:expr
    ) => {{
        let archive = <$archive_ty>::new($path)?;
        // Always try to read the manifest TOML string and upgrade if needed
        let manifest = match archive.read_manifest_string() {
            Ok(manifest_str) => {
                match $crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                    Ok(upgraded) => upgraded,
                    Err(_) => {
                        // Fallback: try parsing as Manifest directly
                        toml::from_str(&manifest_str).unwrap_or_else(|_| Manifest::default())
                    }
                }
            }
            Err(_) => Manifest::default(),
        };
        let is_web = manifest.meta.web_archive;

        let backend: Box<dyn ImageArchiveTrait> = if is_web {
            Box::new(WebImageArchive::new(archive, manifest.clone()))
        } else {
            Box::new(archive)
        };

        Ok(Self {
            path: $path.to_path_buf(),
            manifest,
            backend,
        })
    }};
}
/// Trait for all comic archive backends.
///
/// Implementors provide methods for listing images, reading images by name,
/// and reading/writing the manifest.
pub trait ImageArchiveTrait: Send + Sync {
    /// List all image filenames in the archive.
    fn list_images(&self) -> Vec<String>;
    /// Read the raw bytes of an image by filename.
    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError>;
    /// Read manifest string from the archive.
    fn read_manifest_string(&self) -> Result<String, ArchiveError>;
    /// Read the manifest from the archive.
    fn read_manifest(&self) -> Result<Manifest, ArchiveError>;
    /// Write the manifest to the archive.
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError>;
}

/// The main image archive abstraction, supporting CBZ, CBR, and web archives.
///
/// This struct wraps a dynamic backend and provides a unified interface for
/// listing images, reading images, and working with manifests.
pub struct ImageArchive {
    /// Path to the archive file.
    pub path: PathBuf,
    /// The manifest for this archive.
    pub manifest: Manifest,
    /// The backend implementation for this archive.
    pub backend: Box<dyn ImageArchiveTrait>,
}

impl ImageArchive {
    /// Open and process an archive at the given path.
    ///
    /// Automatically detects the archive type by file extension and loads the manifest.
    /// Returns an error if the archive type is unsupported or cannot be opened.
    pub fn process(path: &Path) -> Result<Self, ArchiveError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "cbz" | "zip" => archive_case!(ZipImageArchive, path),
            #[cfg(feature = "rar")]
            "cbr" | "rar" => archive_case!(RarImageArchive, path),
            _ => Err(ArchiveError::UnsupportedArchive),
        }
    }

    /// Generate a JPEG thumbnail for the given image in the archive.
    ///
    /// The thumbnail is resized to 200x200 pixels (preserving aspect ratio)
    /// and encoded as a JPEG with quality 80.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the image file within the archive.
    ///
    /// # Returns
    ///
    /// A vector of JPEG bytes on success, or an `ArchiveError` on failure.
    pub fn generate_thumbnail(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let image_data = self.read_image_by_name(filename)?;
        let img = image::load_from_memory(&image_data).map_err(|e| {
            ArchiveError::ImageProcessingError(format!("Failed to load image: {}", e))
        })?;

        let thumbnail = img.resize(200, 200, image::imageops::FilterType::Lanczos3);
        let mut buffer = Vec::new();
        {
            let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 80);
            encoder.encode_image(&thumbnail).map_err(|e| {
                ArchiveError::ImageProcessingError(format!("Failed to write thumbnail: {}", e))
            })?;
        }

        Ok(buffer)
    }

    /// List all image filenames in the archive.
    pub fn list_images(&self) -> Vec<String> {
        self.backend.list_images()
    }

    /// Read the raw bytes of an image by filename.
    pub fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        self.backend.read_image_by_name(filename)
    }

    /// Read the raw bytes of an image by its index in the archive.
    ///
    /// Returns an error if the index is out of bounds.
    pub fn read_image_by_index(&mut self, index: usize) -> Result<Vec<u8>, ArchiveError> {
        let filenames = self.list_images();
        if index < filenames.len() {
            self.read_image_by_name(&filenames[index])
        } else {
            Err(ArchiveError::IndexOutOfBounds)
        }
    }

    /// Get a mutable reference to the backend trait object.
    pub fn as_trait_mut(&mut self) -> &mut dyn ImageArchiveTrait {
        self.backend.as_mut()
    }

    /// Get a mutable reference to the manifest.
    pub fn manifest_mut(&mut self) -> &mut Manifest {
        &mut self.manifest
    }

    /// Read the manifest from the backend.
    pub fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.backend.read_manifest_string()
    }


    /// Read the manifest from the backend.
    pub fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.backend.read_manifest()
    }

    /// Write the manifest to the backend.
    pub fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.backend.write_manifest(manifest)
    }

    /// Get the path to the archive file.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
