use std::fs::File;
use std::io::{Read, Seek};
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
        if !path.exists() {
            return Err(AppError::UnsupportedArchive);
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "zip" || ext == "cbz" {
            let zip = ZipImageArchive::open(path)?;
            if zip.image_names().is_empty() {
                return Err(AppError::NoImages);
            }
            Ok(ImageArchive::Zip(zip))
        } else if path.is_dir() {
            let folder = FolderImageArchive::open(path)?;
            if folder.image_names().is_empty() {
                return Err(AppError::NoImages);
            }
            Ok(ImageArchive::Folder(folder))
        } else {
            Err(AppError::UnsupportedArchive)
        }
    }

    /// List image file names (flat, no nesting).
    pub fn image_names(&self) -> Vec<String> {
        match self {
            ImageArchive::Zip(z) => z.image_names(),
            ImageArchive::Folder(f) => f.image_names(),
        }
    }

    /// Read an image by name.
    pub fn read_image(&mut self, name: &str) -> Result<Vec<u8>, AppError> {
        match self {
            ImageArchive::Zip(z) => z.read_image(name),
            ImageArchive::Folder(f) => f.read_image(name),
        }
    }
}

/// Wrapper for zip files, providing access to image files.
pub struct ZipImageArchive {
    archive: ZipArchive<File>,
    image_names: Vec<String>,
}

impl ZipImageArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut image_names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                let lower = name.to_lowercase();
                if [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp"]
                    .iter()
                    .any(|ext| lower.ends_with(ext))
                {
                    image_names.push(name);
                }
            }
        }
        image_names.sort_by_key(|n| n.to_lowercase());
        Ok(Self { archive, image_names })
    }

    pub fn image_names(&self) -> Vec<String> {
        self.image_names.clone()
    }

    pub fn read_image(&mut self, name: &str) -> Result<Vec<u8>, AppError> {
        let idx = self
            .image_names
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| AppError::ImageNotFound(name.to_string()))?;
        let mut file = self.archive.by_index(idx)?;
        let mut buf = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

/// Wrapper for folders, providing access to image files (flat, no nesting).
pub struct FolderImageArchive {
    folder: PathBuf,
    image_names: Vec<String>,
}

impl FolderImageArchive {
    pub fn open<P: AsRef<Path>>(folder: P) -> Result<Self, AppError> {
        let folder = folder.as_ref().to_path_buf();
        let mut image_names = Vec::new();
        for entry in std::fs::read_dir(&folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let lower = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp"]
                    .iter()
                    .any(|ext| lower.ends_with(ext))
                {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        image_names.push(name.to_string());
                    }
                }
            }
        }
        image_names.sort_by_key(|n| n.to_lowercase());
        Ok(Self { folder, image_names })
    }

    pub fn image_names(&self) -> Vec<String> {
        self.image_names.clone()
    }

    pub fn read_image(&self, name: &str) -> Result<Vec<u8>, AppError> {
        let path = self.folder.join(name);
        std::fs::read(path).map_err(|_| AppError::ImageNotFound(name.to_string()))
    }
}