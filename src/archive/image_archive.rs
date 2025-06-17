//! Image archive abstraction for zip/cbz files and folders.

use std::fs::{self, File};
use std::io::{Read};
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;
use crate::error::AppError;

/// Represents an archive of images, either from a zip file or a folder.
pub enum ImageArchive {
    Zip(ZipImageArchive),
    Folder(FolderImageArchive),
}

impl ImageArchive {
    /// Try to open the path as a zip/cbz or folder. Returns a unified error type.
    pub fn process<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let path = path.as_ref();
        if path.is_file() {
            // let file = File::open(path)?;
            // let zip = ZipArchive::new(file)?;
            Ok(ImageArchive::Zip(ZipImageArchive {
                path: path.to_path_buf(),
                // ...store zip if you want, or open on demand...
            }))
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
        }
    }

    /// Read the raw bytes of an image by filename.
    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        match self {
            ImageArchive::Zip(zip) => zip.read_image(filename),
            ImageArchive::Folder(folder) => folder.read_image(filename),
        }
    }
}

/// Wrapper for zip files, providing access to image files.
pub struct ZipImageArchive {
    pub path: PathBuf,
    // Optionally: pub zip: ZipArchive<File>,
}

impl ZipImageArchive {
    pub fn list_images(&self) -> Vec<String> {
        // Open zip and list image files
        let file = File::open(&self.path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut images = Vec::new();
        for i in 0..zip.len() {
            let file = zip.by_index(i).unwrap();
            let name = file.name().to_string();
            if name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".jpeg") || name.ends_with(".gif") {
                images.push(name);
            }
        }
        images
    }

    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        let file = File::open(&self.path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut file = zip.by_name(filename)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

/// Wrapper for folders, providing access to image files (flat, no nesting).
pub struct FolderImageArchive {
    pub path: PathBuf,
}

impl FolderImageArchive {
    pub fn list_images(&self) -> Vec<String> {
        let mut images = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ["jpg", "jpeg", "png", "gif"].contains(&ext.to_lowercase().as_str()) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            images.push(name.to_string());
                        }
                    }
                }
            }
        }
        images
    }

    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        let path = self.path.join(filename);
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}