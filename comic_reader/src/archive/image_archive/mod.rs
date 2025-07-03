//! Image archive abstraction for zip/cbz files and folders.

use crate::prelude::*;

pub mod manifest;

pub mod zip_archive;
pub use zip_archive::ZipImageArchive;
pub mod folder_archive;
pub use folder_archive::FolderImageArchive;
pub mod web_archive;
pub use web_archive::WebImageArchive;

/// Represents an archive of images, either from a zip file or a folder.
pub enum ImageArchive {
    Zip(ZipImageArchive),
    Folder(FolderImageArchive),
    Web(WebImageArchive),
}

impl ImageArchive {
    /// Try to open the path as a zip/cbz or folder. Returns a unified error type.
    pub fn process<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let path = path.as_ref();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "cbz" | "zip" => {
                    let file = File::open(path)?;
                    let mut zip = ZipArchive::new(file)?;
                    let manifest_file = zip.by_name("manifest.toml");

                    if let Ok(mut manifest_file) = manifest_file {
                        let mut contents = String::new();
                        manifest_file.read_to_string(&mut contents)?;

                        let manifest: manifest::Manifest = toml::from_str(&contents)
                            .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;

                        if manifest.meta.web_archive {
                            Ok(ImageArchive::Web(WebImageArchive { manifest }))
                        } else {
                            Ok(ImageArchive::Zip(ZipImageArchive {
                                path: path.to_path_buf(),
                                manifest,
                            }))
                        }
                    } else {
                        let manifest = Manifest::default();

                        Ok(ImageArchive::Zip(ZipImageArchive {
                            path: path.to_path_buf(),
                            manifest,
                        }))
                    }
                }
                _ => Err(AppError::UnsupportedArchive),
            }
        } else if path.is_dir() {
            Ok(ImageArchive::Folder(FolderImageArchive {
                path: path.to_path_buf(),
            }))
        } else {
            Err(AppError::UnsupportedArchive)
        }
    }

    /// List image file names (flat, no nesting).
    pub fn list_images(&self) -> Vec<String> {
        match self {
            ImageArchive::Zip(zip) => zip.list_images(),
            ImageArchive::Folder(folder) => folder.list_images(),
            ImageArchive::Web(archive) => archive.list_images(),
        }
    }

    /// Read the raw bytes of an image by filename.
    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        match self {
            ImageArchive::Zip(zip) => zip.read_image(filename),
            ImageArchive::Folder(folder) => folder.read_image(filename),
            ImageArchive::Web(archive) => archive.read_image(filename),
        }
    }
}
