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
///
/// This struct provides methods to list images, extract images, and read/write the manifest
/// from RAR archives. It relies on the `unrar` tool for reading and `rar` (WinRAR) for writing.
pub struct RarImageArchive {
    /// Path to the RAR archive file.
    path: PathBuf,
    /// List of image entries in the archive.
    entries: Vec<String>,
}

impl RarImageArchive {
    /// Open a RAR archive and list its image entries.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RAR/CBR file.
    ///
    /// # Returns
    ///
    /// Returns a `RarImageArchive` on success, or an `ArchiveError` if the archive cannot be read.
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        let output = Command::new("unrar")
            .arg("l")
            .arg("-c-") // no comments, cleaner output
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
                // Each line looks like: attrs size date time name
                // If line too short or blank, stop
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
}

#[async_trait::async_trait]
impl ImageArchiveTrait for RarImageArchive {
    /// List all image filenames in the RAR archive.
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    /// Extract and return the raw bytes of an image by filename.
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the image file within the archive.
    ///
    /// # Returns
    ///
    /// A vector of bytes containing the image data, or an `ArchiveError` on failure.
    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let tmp_dir = tempdir().map_err(|_| {
            ArchiveError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create temp dir",
            ))
        })?;
        let status = Command::new("unrar")
            .arg("x")
            .arg("-y") // assume yes
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

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
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

    /// Read and parse the manifest from the RAR archive.
    ///
    /// # Returns
    ///
    /// The parsed `Manifest` struct, or an `ArchiveError` if extraction or parsing fails.
    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let manifest_str = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&manifest_str)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    /// Write the manifest to the RAR archive using the external `rar` tool (WinRAR required).
    ///
    /// # Arguments
    ///
    /// * `manifest` - The manifest to write.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `ArchiveError` if writing fails.
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        log::info!(
            "Preparing to write manifest to RAR archive: {:?}",
            &self.path
        );

        // Write manifest to a temp file
        let tmp_dir =
            tempdir().map_err(|_| ArchiveError::ManifestError("Tempdir failed".into()))?;
        let manifest_path = tmp_dir.path().join("manifest.toml");
        log::info!(
            "Writing manifest TOML to temporary file: {:?}",
            manifest_path
        );
        let toml = toml::to_string_pretty(manifest)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        fs::write(&manifest_path, toml)
            .map_err(|_| ArchiveError::ManifestError("Failed to write temp manifest".into()))?;

        // Use 'rar' to update the archive (requires WinRAR/rar.exe, not unrar)
        log::info!(
            "Running 'rar' to update manifest in archive: {:?}",
            &self.path
        );

        drop(tmp_dir);

        let status = Command::new("rar")
            .arg("u") // update
            .arg("-ep1") // exclude base dir from names
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
            log::error!("Failed to update manifest in archive (WinRAR required)");
            return Err(ArchiveError::ManifestError(
                "Failed to update manifest in archive (WinRAR required)".into(),
            ));
        }

        log::info!(
            "Manifest successfully written to RAR archive: {:?}",
            &self.path
        );
        Ok(())
    }
}
