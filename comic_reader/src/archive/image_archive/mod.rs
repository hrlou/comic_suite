//! Unified image archive interface for CBZ, folders, RAR, and web archives.

mod zip_archive;
pub use zip_archive::ZipImageArchive;

mod web_archive;
pub use web_archive::WebImageArchive;

#[cfg(feature = "rar")]
mod rar_archive;
#[cfg(feature = "rar")]
pub use rar_archive::RarImageArchive;

pub mod manifest;

use crate::prelude::*;

/// Common image archive interface
pub trait ImageArchiveTrait: Send + Sync {
    fn list_images(&self) -> Vec<String>;
    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageArchiveType {
    Zip,
    Rar,
}

/// The main image archive abstraction
pub struct ImageArchive {
    pub path: PathBuf,
    pub manifest: Manifest,
    // pub is_web_archive: bool,
    pub backend: Box<dyn ImageArchiveTrait>,
    pub kind: ImageArchiveType,
}

impl ImageArchive {
    pub fn process(path: &Path) -> Result<Self, AppError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "cbz" | "zip" => {
                let manifest = ZipImageArchive::read_manifest(path)?;
                let is_web = manifest.meta.web_archive;

                let backend: Box<dyn ImageArchiveTrait> = if is_web {
                    Box::new(WebImageArchive::new(ZipImageArchive::new(path)?, manifest.clone()))
                } else {
                    Box::new(ZipImageArchive::new(path)?)
                };

                Ok(Self {
                    path: path.to_path_buf(),
                    manifest,
                    // is_web_archive: is_web,
                    backend,
                    kind: ImageArchiveType::Zip,
                })
            }
            #[cfg(feature = "rar")]
            "cbr" | "rar" => {
                let backend = Box::new(RarImageArchive::new(path)?);
                Ok(Self {
                    path: path.to_path_buf(),
                    manifest: Manifest::default(),
                    // is_web_archive: false,
                    backend,
                    kind: ImageArchiveType::Rar,
                })
            }
            // _ if path.is_dir() => {
            //     let backend = Box::new(crate::image_archive::folder_archive::FolderImageArchive::new(path)?);
            //     Ok(Self {
            //         path: path.to_path_buf(),
            //         manifest: Manifest::default(),
            //         is_web_archive: false,
            //         backend,
            //     })
            // }
            _ => Err(AppError::UnsupportedArchive),
        }
    }

    pub fn manifest_mut_and_path(&mut self) -> (&mut Manifest, &Path) {
        (&mut self.manifest, self.path.as_path())
    }

    pub fn list_images(&self) -> Vec<String> {
        self.backend.list_images()
    }

    pub fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError> {
        self.backend.read_image_by_name(filename)
    }

    pub fn read_image_by_index(&mut self, index: usize) -> Result<Vec<u8>, AppError> {
        let filenames = self.list_images();
        if index < filenames.len() {
            self.read_image_by_name(&filenames[index])
        } else {
            Err(AppError::IndexOutOfBounds)
        }
    }
}