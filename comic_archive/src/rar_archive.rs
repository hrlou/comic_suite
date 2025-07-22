use crate::is_supported_format;
use crate::prelude::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// An archive backend for RAR/CBR comic archives using the external `unrar` and `rar` tools.
pub struct RarImageArchive {
    path: PathBuf,
    entries: Vec<String>,
}

impl RarImageArchive {
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        let output = Command::new("unrar")
            .arg("l")
            .arg("-c-")
            .arg(path)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|_| ArchiveError::UnsupportedArchive)?;

        if !output.status.success() {
            return Err(ArchiveError::UnsupportedArchive);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut listing_started = false;

        for line in stdout.lines() {
            if line.trim().starts_with("--------") {
                listing_started = true;
                continue;
            }
            if listing_started {
                if line.trim().is_empty() {
                    break;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 5 {
                    break;
                }
                let filename = parts[4..].join(" ");
                let filename_lower = filename.to_lowercase();
                if is_supported_format!(&filename_lower) {
                    entries.push(filename);
                }
            }
        }
        entries.sort();

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    fn read_image_by_name_sync(&self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let tmp_dir = tempdir().map_err(|_| {
            ArchiveError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create temp dir",
            ))
        })?;
        let status = Command::new("unrar")
            .arg("x")
            .arg("-y")
            .arg(&self.path)
            .arg(filename)
            .arg(tmp_dir.path())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|_| ArchiveError::UnsupportedArchive)?;

        if !status.success() {
            return Err(ArchiveError::UnsupportedArchive);
        }

        let extracted_path = tmp_dir.path().join(filename);
        let mut file = fs::File::open(&extracted_path).map_err(|_| ArchiveError::NoImages)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|_| ArchiveError::NoImages)?;

        Ok(buffer)
    }

    fn read_manifest_string_sync(&self) -> Result<String, ArchiveError> {
        let tmp_dir =
            tempdir().map_err(|_| ArchiveError::ManifestError("Tempdir failed".into()))?;
        let status = Command::new("unrar")
            .arg("x")
            .arg("-y")
            .arg(&self.path)
            .arg("manifest.toml")
            .arg(tmp_dir.path())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|_| ArchiveError::ManifestError("Failed to run unrar".into()))?;

        if !status.success() {
            return Err(ArchiveError::ManifestError(
                "manifest.toml not found in archive".into(),
            ));
        }

        let manifest_path = tmp_dir.path().join("manifest.toml");
        let manifest_str = fs::read_to_string(&manifest_path)
            .map_err(|_| ArchiveError::ManifestError("Failed to read manifest.toml".into()))?;
        Ok(manifest_str)
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ImageArchiveTrait for RarImageArchive {
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let path = self.path.clone();
        let filename = filename.to_string();
        tokio::task::spawn_blocking(move || {
            let archive = RarImageArchive { path, entries: Vec::new() }; // entries unused
            archive.read_image_by_name_sync(&filename)
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }

    async fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let archive = RarImageArchive { path, entries: Vec::new() };
            archive.read_manifest_string_sync()
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }

    async fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let manifest_str = self.read_manifest_string().await?;
        let manifest: Manifest = toml::from_str(&manifest_str)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    async fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        let path = self.path.clone();
        let toml = toml::to_string_pretty(manifest)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        tokio::task::spawn_blocking(move || {
            let tmp_dir =
                tempdir().map_err(|_| ArchiveError::ManifestError("Tempdir failed".into()))?;
            let manifest_path = tmp_dir.path().join("manifest.toml");
            fs::write(&manifest_path, &toml)
                .map_err(|_| ArchiveError::ManifestError("Failed to write temp manifest".into()))?;

            let status = Command::new("rar")
                .arg("u")
                .arg("-ep1")
                .arg(&path)
                .arg(&manifest_path)
                .creation_flags(CREATE_NO_WINDOW)
                .status()
                .map_err(|_| {
                    ArchiveError::ManifestError(
                        "Failed to run rar.exe (WinRAR required for writing)".into(),
                    )
                })?;

            if !status.success() {
                return Err(ArchiveError::ManifestError(
                    "Failed to update manifest in archive (WinRAR required)".into(),
                ));
            }
            Ok(())
        })
        .await
        .unwrap_or_else(|e| Err(ArchiveError::Other(format!("Join error: {e}"))))
    }
}

#[cfg(not(feature = "async"))]
impl ImageArchiveTrait for RarImageArchive {
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        self.read_image_by_name_sync(filename)
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.read_manifest_string_sync()
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let manifest_str = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&manifest_str)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        let tmp_dir =
            tempdir().map_err(|_| ArchiveError::ManifestError("Tempdir failed".into()))?;
        let manifest_path = tmp_dir.path().join("manifest.toml");
        let toml = toml::to_string_pretty(manifest)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        fs::write(&manifest_path, toml)
            .map_err(|_| ArchiveError::ManifestError("Failed to write temp manifest".into()))?;

        let status = Command::new("rar")
            .arg("u")
            .arg("-ep1")
            .arg(&self.path)
            .arg(&manifest_path)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|_| {
                ArchiveError::ManifestError(
                    "Failed to run rar.exe (WinRAR required for writing)".into(),
                )
            })?;

        if !status.success() {
            return Err(ArchiveError::ManifestError(
                "Failed to update manifest in archive (WinRAR required)".into(),
            ));
        }
        Ok(())
    }
}
