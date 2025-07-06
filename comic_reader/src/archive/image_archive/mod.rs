//! Image archive abstraction for zip/cbz files and folders.

use crate::prelude::*;

pub mod manifest;

pub mod folder_archive;
pub use folder_archive::FolderImageArchive;
#[cfg(feature = "rar")]
pub mod rar_archive;
#[cfg(feature = "rar")]
pub use rar_archive::RarImageArchive;
pub mod web_archive;
pub use web_archive::WebImageArchive;
pub mod zip_archive;
pub use zip_archive::ZipImageArchive;

/// Represents an archive of images, either from a zip file or a folder.
pub enum ImageArchive {
    Zip(ZipImageArchive),
    Web(WebImageArchive),
    Folder(FolderImageArchive),
    #[cfg(feature = "rar")]
    Rar(RarImageArchive),
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
                #[cfg(feature = "rar")]
                "cbr" | "rar" => {                
                    let manifest = RarImageArchive::read_manifest(path)?;
                    Ok(ImageArchive::Rar(RarImageArchive {
                        path: path.to_path_buf(),
                        manifest,
                    }))
                }
                "cbz" | "zip" => {
                    let manifest = ZipImageArchive::read_manifest(path)?;
                    if manifest.meta.web_archive {
                        Ok(ImageArchive::Web(WebImageArchive {
                            path: path.to_path_buf(),
                            manifest,
                        }))
                    } else {
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

    pub fn manifest_mut_and_path(&mut self) -> Option<(&mut Manifest, &Path)> {
        match self {
            ImageArchive::Zip(zip) => Some((&mut zip.manifest, &zip.path)),
            ImageArchive::Web(web) => Some((&mut web.manifest, &web.path)),
            ImageArchive::Folder(_) => None,
            #[cfg(feature = "rar")]
            ImageArchive::Rar(rar) => {
                // RAR archives do not have a manifest, so we return None
                None
            }
        }
    }

    /// List image file names (flat, no nesting).
    pub fn list_images(&self) -> Vec<String> {
        match self {
            ImageArchive::Zip(zip) => zip.list_images(),
            ImageArchive::Folder(folder) => folder.list_images(),
            ImageArchive::Web(archive) => archive.list_images(),
            #[cfg(feature = "rar")]
            ImageArchive::Rar(rar) => rar.list_images(),
        }
    }

    /// Read the raw bytes of an image by filename.
    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        match self {
            ImageArchive::Zip(zip) => zip.read_image(filename),
            ImageArchive::Folder(folder) => folder.read_image(filename),
            ImageArchive::Web(archive) => archive.read_image(filename),
            #[cfg(feature = "rar")]
            ImageArchive::Rar(rar) => rar.read_image(filename),
        }
    }
}
