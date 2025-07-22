use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::ArchiveError;
use crate::model::Manifest;
use crate::{ImageArchiveTrait, is_supported_format};

pub struct FolderImageArchive {
    pub path: PathBuf,
}

impl FolderImageArchive {
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        if !path.is_dir() {
            return Err(ArchiveError::UnsupportedArchive);
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.path.join("manifest.toml")
    }
}

#[async_trait::async_trait]
impl ImageArchiveTrait for FolderImageArchive {
    fn list_images(&self) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if is_supported_format!(&name) {
                        files.push(name);
                    }
                }
            }
        }
        files.sort();
        files
    }

    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let img_path = self.path.join(filename);
        let mut file = fs::File::open(&img_path)
            .map_err(|e| ArchiveError::IoError(format!("Failed to open image: {}", e)))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| ArchiveError::IoError(format!("Failed to read image: {}", e)))?;
        Ok(buf)
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        let manifest_path = self.manifest_path();
        let mut file =
            fs::File::open(&manifest_path).map_err(|_| ArchiveError::ManifestNotFound)?;
        let mut s = String::new();
        file.read_to_string(&mut s)
            .map_err(|e| ArchiveError::IoError(format!("Failed to read manifest: {}", e)))?;
        Ok(s)
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let s = self.read_manifest_string()?;
        toml::from_str(&s).map_err(|e| ArchiveError::ManifestParseError(e.to_string()))
    }

    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        let manifest_path = self.manifest_path();
        let s = toml::to_string_pretty(manifest)
            .map_err(|e| ArchiveError::ManifestParseError(e.to_string()))?;
        fs::write(&manifest_path, s)
            .map_err(|e| ArchiveError::IoError(format!("Failed to write manifest: {}", e)))
    }
}
