//! Unified image archive interface for CBZ, folders, RAR, and web archives.

mod zip_archive;
pub use zip_archive::ZipImageArchive;

mod web_archive;
pub use web_archive::WebImageArchive;

#[cfg(feature = "rar")]
mod rar_archive;
#[cfg(feature = "rar")]
pub use rar_archive::RarImageArchive;

// pub mod manifest;

use crate::prelude::*;

#[macro_export]
macro_rules! archive_case {
    (
        $archive_ty:ty, $path:expr
    ) => {{
        let archive = <$archive_ty>::new($path)?;
        let manifest = archive.read_manifest().unwrap_or_default();
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

/// Common image archive interface
pub trait ImageArchiveTrait: Send + Sync {
    fn list_images(&self) -> Vec<String>;
    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError>;
    // fn read_image_by_index(&mut self, index: usize) -> Result<Vec<u8>, AppError>;
    fn read_manifest(&self) -> Result<Manifest, AppError>;
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), AppError>;
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum ImageArchiveType {
// Zip,
// Rar,
// }

/// The main image archive abstraction
pub struct ImageArchive {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub backend: Box<dyn ImageArchiveTrait>,
    // pub kind: ImageArchiveType,
}

impl ImageArchive {
    pub fn process(path: &Path) -> Result<Self, AppError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "cbz" | "zip" => archive_case!(ZipImageArchive, path),
            #[cfg(feature = "rar")]
            "cbr" | "rar" => archive_case!(RarImageArchive, path),
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

    pub fn as_trait_mut(&mut self) -> &mut dyn ImageArchiveTrait {
        self.backend.as_mut()
    }

    pub fn manifest_mut(&mut self) -> &mut Manifest {
        &mut self.manifest
    }

    pub fn read_manifest(&self) -> Result<Manifest, AppError> {
        self.backend.read_manifest()
    }

    pub fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), AppError> {
        self.backend.write_manifest(manifest)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
