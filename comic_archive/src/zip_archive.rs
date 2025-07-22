use crate::error::ArchiveError;
use crate::is_supported_format;
use crate::prelude::*;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::read::ZipArchive;

pub struct ZipImageArchive {
    path: PathBuf,
}

impl ZipImageArchive {
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn create_from_path(path: &Path) -> Result<(), ArchiveError> {
        use std::io::Write;
        use zip::{ZipWriter, write::FileOptions};

        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);

        let mut manifest = Manifest::default();
        manifest.meta.web_archive = true;

        let manifest_str = toml::to_string_pretty(&manifest)
            .map_err(|e| ArchiveError::ManifestError(format!("Couldn't serialize: {}", e)))?;

        zip.start_file("manifest.toml", options)?;
        zip.write_all(manifest_str.as_bytes())?;

        zip.finish()?; // Closes the archive and flushes everything

        Ok(())
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ImageArchiveTrait for ZipImageArchive {
    fn list_images(&self) -> Vec<String> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let mut zip = match ZipArchive::new(file) {
            Ok(z) => z,
            Err(_) => return vec![],
        };

        let mut images = Vec::new();
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                let name = file.name().to_string();
                if is_supported_format!(&name) {
                    images.push(name);
                }
            }
        }
        images.sort();
        images
    }

    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let path = self.path.clone();
        let filename = filename.to_string();
        tokio::task::spawn_blocking(move || {
            let file = File::open(&path)?;
            let mut zip = ZipArchive::new(file)?;
            let mut file = zip.by_name(&filename)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Ok(buf)
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }

    async fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let file = File::open(&path)?;
            let mut zip = ZipArchive::new(file)?;
            let mut manifest_file = zip
                .by_name("manifest.toml")
                .map_err(|_| ArchiveError::ManifestError("Manifest not found".to_string()))?;
            let mut contents = String::new();
            manifest_file
                .read_to_string(&mut contents)
                .map_err(|e| ArchiveError::ManifestError(format!("Failed to read manifest: {}", e)))?;
            Ok(contents)
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }

    async fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let contents = self.read_manifest_string().await?;
        let manifest: Manifest = toml::from_str(&contents)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    async fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        let path = self.path.clone();
        let manifest = manifest.clone();
        tokio::task::spawn_blocking(move || {
            use std::fs::{File, remove_file, rename};
            use std::io::Write;
            use zip::{ZipWriter, write::FileOptions};

            let file = File::open(&path)?;
            let mut zip = ZipArchive::new(file)?;

            let temp_path = path.with_extension("rebuild.tmp.zip");
            let mut temp_file = File::create(&temp_path)?;

            {
                let mut writer = ZipWriter::new(&mut temp_file);
                let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

                for i in 0..zip.len() {
                    let mut file = zip.by_index(i)?;
                    let name = file.name().to_string();

                    if name == "manifest.toml" {
                        continue;
                    }

                    writer.start_file(name.clone(), options)?;
                    std::io::copy(&mut file, &mut writer)?;
                }

                writer.start_file("manifest.toml", options)?;
                let toml = toml::to_string_pretty(&manifest)
                    .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
                writer.write_all(toml.as_bytes())?;
                writer.finish()?;
            }

            drop(temp_file);

            remove_file(&path).map_err(|e| {
                ArchiveError::ManifestError(format!("Failed to remove original file: {}", e))
            })?;

            rename(&temp_path, &path).map_err(|e| {
                ArchiveError::ManifestError(format!("Failed to rename temp file: {}", e))
            })?;

            Ok(())
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }
}

#[cfg(not(feature = "async"))]
impl ImageArchiveTrait for ZipImageArchive {
    fn list_images(&self) -> Vec<String> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let mut zip = match ZipArchive::new(file) {
            Ok(z) => z,
            Err(_) => return vec![],
        };

        let mut images = Vec::new();
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                let name = file.name().to_string();
                if is_supported_format!(&name) {
                    images.push(name);
                }
            }
        }
        images.sort();
        images
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut file = zip.by_name(filename)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut manifest_file = zip
            .by_name("manifest.toml")
            .map_err(|_| ArchiveError::ManifestError("Manifest not found".to_string()))?;
        let mut contents = String::new();
        manifest_file
            .read_to_string(&mut contents)
            .map_err(|e| ArchiveError::ManifestError(format!("Failed to read manifest: {}", e)))?;
        Ok(contents)
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let contents = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&contents)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        use std::fs::{File, remove_file, rename};
        use std::io::Write;
        use zip::{ZipWriter, write::FileOptions};

        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;

        let temp_path = self.path.with_extension("rebuild.tmp.zip");
        let mut temp_file = File::create(&temp_path)?;

        {
            let mut writer = ZipWriter::new(&mut temp_file);
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            for i in 0..zip.len() {
                let mut file = zip.by_index(i)?;
                let name = file.name().to_string();

                if name == "manifest.toml" {
                    continue;
                }

                writer.start_file(name.clone(), options)?;
                std::io::copy(&mut file, &mut writer)?;
            }

            writer.start_file("manifest.toml", options)?;
            let toml = toml::to_string_pretty(manifest)
                .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
            writer.write_all(toml.as_bytes())?;
            writer.finish()?;
        }

        drop(temp_file);

        remove_file(&self.path).map_err(|e| {
            ArchiveError::ManifestError(format!("Failed to remove original file: {}", e))
        })?;

        rename(&temp_path, &self.path).map_err(|e| {
            ArchiveError::ManifestError(format!("Failed to rename temp file: {}", e))
        })?;

        Ok(())
    }
}
