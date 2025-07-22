use crate::error::ArchiveError;
use crate::is_supported_format;
use crate::prelude::*;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::read::ZipArchive;

/// An archive backend for CBZ/ZIP comic archives.
///
/// This struct provides methods to list images, extract images, and read/write the manifest
/// from ZIP-based comic archives.
pub struct ZipImageArchive {
    /// Path to the ZIP archive file.
    path: PathBuf,
}

impl ZipImageArchive {
    /// Create a new `ZipImageArchive` from a given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CBZ/ZIP file.
    ///
    /// # Returns
    ///
    /// Returns a `ZipImageArchive` on success, or an `ArchiveError` if the archive cannot be opened.
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Create a new ZIP archive at the given path with a default manifest.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to create the new ZIP archive.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `ArchiveError` if creation fails.
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

#[async_trait::async_trait]
impl ImageArchiveTrait for ZipImageArchive {
    /// List all image filenames in the ZIP archive.
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
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut file = zip.by_name(filename)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Read the manifest.toml file as a raw string from the ZIP archive.
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

    /// Read and parse the manifest from the ZIP archive.
    ///
    /// # Returns
    ///
    /// The parsed `Manifest` struct, or an `ArchiveError` if extraction or parsing fails.
    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let contents = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&contents)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    /// Write the manifest to the ZIP archive, replacing any existing manifest.
    ///
    /// # Arguments
    ///
    /// * `manifest` - The manifest to write.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `ArchiveError` if writing fails.
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        use std::fs::{File, remove_file, rename};
        use std::io::Write;
        use zip::{ZipWriter, write::FileOptions};

        log::info!("Opening zip file at {:?}", &self.path);
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;
        log::info!("Zip archive opened, contains {} entries", zip.len());

        // Prepare paths
        let temp_path = self.path.with_extension("rebuild.tmp.zip");
        log::info!("Creating temporary file at {:?}", temp_path);
        let mut temp_file = File::create(&temp_path)?;

        {
            let mut writer = ZipWriter::new(&mut temp_file);
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            // Copy existing entries, skipping manifest.toml
            for i in 0..zip.len() {
                let mut file = zip.by_index(i)?;
                let name = file.name().to_string();

                log::info!("Processing entry: {}", name);

                if name == "manifest.toml" {
                    log::info!("Skipping old manifest.toml");
                    continue;
                }

                writer.start_file(name.clone(), options)?;
                std::io::copy(&mut file, &mut writer)?;
                log::info!("Copied entry: {}", name);
            }

            // Write new manifest.toml
            log::info!("Writing new manifest.toml");
            writer.start_file("manifest.toml", options)?;
            let toml = toml::to_string_pretty(manifest)
                .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
            writer.write_all(toml.as_bytes())?;
            writer.finish()?;
            log::info!("Finished writing new manifest.toml");
        }

        // Close temp_file before renaming
        drop(temp_file);

        // Remove the original file
        log::info!("Removing original file {:?}", &self.path);
        remove_file(&self.path).map_err(|e| {
            log::error!("Failed to remove original file: {}", e);
            ArchiveError::ManifestError(format!("Failed to remove original file: {}", e))
        })?;

        // Rename temp file to original path
        log::info!("Renaming {:?} to {:?}", temp_path, &self.path);
        rename(&temp_path, &self.path).map_err(|e| {
            log::error!("Failed to rename temp file: {}", e);
            ArchiveError::ManifestError(format!("Failed to rename temp file: {}", e))
        })?;

        log::info!("Manifest successfully written to {:?}", &self.path);
        Ok(())
    }
}
