//! Unified image archive interface for CBZ, folders, RAR, and web archives.

pub mod error;
pub mod model;
pub mod prelude;

mod zip_archive;
pub use zip_archive::ZipImageArchive;

mod web_archive;
pub use web_archive::WebImageArchive;

mod folder_archive;
pub use folder_archive::FolderImageArchive;

#[cfg(feature = "rar")]
mod rar_archive;
#[cfg(feature = "rar")]
pub use rar_archive::RarImageArchive;

#[cfg(feature = "7z")]
mod seven_zip_archive;
#[cfg(feature = "7z")]
pub use seven_zip_archive::SevenZipImageArchive;

use image::codecs::jpeg::JpegEncoder;
use std::path::{Path, PathBuf};

use crate::prelude::*;

#[macro_export]
macro_rules! is_supported_format {
    ($name:expr) => {
        $name.ends_with(".jpg")
            || $name.ends_with(".jpeg")
            || $name.ends_with(".png")
            || $name.ends_with(".gif")
            || $name.ends_with(".bmp")
            || $name.ends_with(".webp")
    };
}

/// Macro to simplify archive backend instantiation and manifest extraction.
#[macro_export]
macro_rules! archive_case {
    (
        $archive_ty:ty, $path:expr
    ) => {{
        async {
            let archive = <$archive_ty>::new($path)?;
            let manifest = match archive.read_manifest_string().await {
                Ok(manifest_str) => {
                    match $crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                        Ok(upgraded) => upgraded,
                        Err(_) => {
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

            Ok(ImageArchive {
                path: $path.to_path_buf(),
                manifest,
                backend,
            })
        }
    }};
}

// =======================
// Trait and API (async)
// =======================
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait ImageArchiveTrait: Send + Sync {
    fn list_images(&self) -> Vec<String>;
    fn read_image_by_name_sync(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError>;
    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError>;
    async fn read_manifest_string(&self) -> Result<String, ArchiveError>;
    async fn read_manifest(&self) -> Result<Manifest, ArchiveError>;
    async fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError>;
}

#[cfg(not(feature = "async"))]
pub trait ImageArchiveTrait: Send + Sync {
    fn list_images(&self) -> Vec<String>;
    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError>;
    fn read_manifest_string(&self) -> Result<String, ArchiveError>;
    fn read_manifest(&self) -> Result<Manifest, ArchiveError>;
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError>;
}

/// Main archive wrapper.
pub struct ImageArchive {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub backend: Box<dyn ImageArchiveTrait>,
}

impl ImageArchive {
    /// Open and process an archive at the given path.
    #[cfg(feature = "async")]
    pub async fn process(path: &Path) -> Result<Self, ArchiveError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if path.is_dir() {
            archive_case!(FolderImageArchive, path).await
        } else {
            match ext.as_str() {
                "cbz" | "zip" => archive_case!(ZipImageArchive, path).await,
                #[cfg(feature = "rar")]
                "cbr" | "rar" => archive_case!(RarImageArchive, path).await,
                #[cfg(feature = "7z")]
                "cb7" | "7z" => archive_case!(SevenZipImageArchive, path).await,
                _ => Err(ArchiveError::UnsupportedArchive),
            }
        }
    }

    #[cfg(not(feature = "async"))]
    pub fn process(path: &Path) -> Result<Self, ArchiveError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if path.is_dir() {
            FolderImageArchive::new(path).and_then(|archive| {
                let manifest = match archive.read_manifest_string() {
                    Ok(manifest_str) => {
                        match crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                            Ok(upgraded) => upgraded,
                            Err(_) => toml::from_str(&manifest_str)
                                .unwrap_or_else(|_| Manifest::default()),
                        }
                    }
                    Err(_) => Manifest::default(),
                };
                let backend: Box<dyn ImageArchiveTrait> = Box::new(archive);
                Ok(ImageArchive {
                    path: path.to_path_buf(),
                    manifest,
                    backend,
                })
            })
        } else {
            match ext.as_str() {
                "cbz" | "zip" => ZipImageArchive::new(path).and_then(|archive| {
                    let manifest = match archive.read_manifest_string() {
                        Ok(manifest_str) => {
                            match crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                                Ok(upgraded) => upgraded,
                                Err(_) => toml::from_str(&manifest_str)
                                    .unwrap_or_else(|_| Manifest::default()),
                            }
                        }
                        Err(_) => Manifest::default(),
                    };
                    let backend: Box<dyn ImageArchiveTrait> = Box::new(archive);
                    Ok(ImageArchive {
                        path: path.to_path_buf(),
                        manifest,
                        backend,
                    })
                }),
                #[cfg(feature = "rar")]
                "cbr" | "rar" => RarImageArchive::new(path).and_then(|archive| {
                    let manifest = match archive.read_manifest_string() {
                        Ok(manifest_str) => {
                            match crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                                Ok(upgraded) => upgraded,
                                Err(_) => toml::from_str(&manifest_str)
                                    .unwrap_or_else(|_| Manifest::default()),
                            }
                        }
                        Err(_) => Manifest::default(),
                    };
                    let backend: Box<dyn ImageArchiveTrait> = Box::new(archive);
                    Ok(ImageArchive {
                        path: path.to_path_buf(),
                        manifest,
                        backend,
                    })
                }),
                #[cfg(feature = "7z")]
                "cb7" | "7z" => SevenZipImageArchive::new(path).and_then(|archive| {
                    let manifest = match archive.read_manifest_string() {
                        Ok(manifest_str) => {
                            match crate::model::Manifest::upgrade_from_v0_to_v1(&manifest_str) {
                                Ok(upgraded) => upgraded,
                                Err(_) => toml::from_str(&manifest_str)
                                    .unwrap_or_else(|_| Manifest::default()),
                            }
                        }
                        Err(_) => Manifest::default(),
                    };
                    let backend: Box<dyn ImageArchiveTrait> = Box::new(archive);
                    Ok(ImageArchive {
                        path: path.to_path_buf(),
                        manifest,
                        backend,
                    })
                }),
                _ => Err(ArchiveError::UnsupportedArchive),
            }
        }
    }

    /// Generate a JPEG thumbnail for the given image in the archive.
    #[cfg(feature = "async")]
    pub async fn generate_thumbnail(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let image_data = self.read_image_by_name(filename).await?;
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

    #[cfg(not(feature = "async"))]
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

    pub fn list_images(&self) -> Vec<String> {
        self.backend.list_images()
    }

    #[cfg(feature = "async")]
    pub async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        self.backend.read_image_by_name(filename).await
    }

    #[cfg(not(feature = "async"))]
    pub fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        self.backend.read_image_by_name(filename)
    }

    #[cfg(feature = "async")]
    pub async fn read_image_by_index(&mut self, index: usize) -> Result<Vec<u8>, ArchiveError> {
        let filenames = self.list_images();
        if index < filenames.len() {
            self.read_image_by_name(&filenames[index]).await
        } else {
            Err(ArchiveError::IndexOutOfBounds)
        }
    }

    #[cfg(not(feature = "async"))]
    pub fn read_image_by_index(&mut self, index: usize) -> Result<Vec<u8>, ArchiveError> {
        let filenames = self.list_images();
        if index < filenames.len() {
            self.read_image_by_name(&filenames[index])
        } else {
            Err(ArchiveError::IndexOutOfBounds)
        }
    }

    pub fn as_trait_mut(&mut self) -> &mut dyn ImageArchiveTrait {
        self.backend.as_mut()
    }

    pub fn manifest_mut(&mut self) -> &mut Manifest {
        &mut self.manifest
    }

    #[cfg(feature = "async")]
    pub async fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.backend.read_manifest_string().await
    }

    #[cfg(not(feature = "async"))]
    pub fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.backend.read_manifest_string()
    }

    #[cfg(feature = "async")]
    pub async fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.backend.read_manifest().await
    }

    #[cfg(not(feature = "async"))]
    pub fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.backend.read_manifest()
    }

    #[cfg(feature = "async")]
    pub async fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.backend.write_manifest(manifest).await
    }

    #[cfg(not(feature = "async"))]
    pub fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.backend.write_manifest(manifest)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
