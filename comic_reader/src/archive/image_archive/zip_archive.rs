use crate::archive::image_archive::{ImageArchiveTrait, Manifest};
use crate::error::AppError;

use crate::prelude::*;

pub struct ZipImageArchive {
    path: PathBuf,
}

impl ZipImageArchive {
    pub fn new(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn create_from_path(path: &Path) -> Result<(), AppError> {
        use std::io::Write;
        use zip::{ZipWriter, write::FileOptions};

        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored) // or .Deflated
            .unix_permissions(0o644);

        let mut manifest = Manifest::default();
        manifest.meta.web_archive = true;

        let manifest_str = toml::to_string_pretty(&manifest)
            .map_err(|e| AppError::ManifestError(format!("Couldn't serialize: {}", e)))?;

        zip.start_file("manifest.toml", options)?;
        zip.write_all(manifest_str.as_bytes())?;

        zip.finish()?; // Closes the archive and flushes everything

        Ok(())
    }
}

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
                if name.ends_with(".jpg")
                    || name.ends_with(".jpeg")
                    || name.ends_with(".png")
                    || name.ends_with(".gif")
                {
                    images.push(name);
                }
            }
        }
        images.sort();
        images
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError> {
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut file = zip.by_name(filename)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn read_manifest(&self) -> Result<Manifest, AppError> {
        let file = File::open(&self.path)?;
        let mut zip = ZipArchive::new(file)?;

        // Try to read the manifest file
        if let Ok(mut manifest_file) = zip.by_name("manifest.toml") {
            let mut contents = String::new();
            manifest_file.read_to_string(&mut contents)?;
            let manifest: Manifest = toml::from_str(&contents)
                .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
            Ok(manifest)
        } else {
            Err(AppError::ManifestError("Manifest not found".to_string()))
        }
    }

    // fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), AppError> {
    //     use std::io::Write;
    //     use zip::{ZipWriter, write::FileOptions};

    //     let file = File::open(&self.path)?;
    //     let mut zip = ZipArchive::new(file)?;

    //     // Create a temporary output file next to original
    //     let temp_path = self.path.with_extension("rebuild.tmp.zip");
    //     let mut temp_file = tempfile::NamedTempFile::new_in(
    //         self.path
    //             .parent()
    //             .unwrap_or_else(|| std::path::Path::new(".")),
    //     )?;
    //     {
    //         let mut writer = ZipWriter::new(&mut temp_file);
    //         let options = zip::write::FileOptions::default()
    //             .compression_method(zip::CompressionMethod::Stored);

    //         // Copy existing entries, skipping manifest.toml
    //         for i in 0..zip.len() {
    //             let mut file = zip.by_index(i)?;
    //             let name = file.name().to_string();

    //             if name == "manifest.toml" {
    //                 continue;
    //             }

    //             writer.start_file(name, options)?;
    //             std::io::copy(&mut file, &mut writer)?;
    //         }

    //         // Write new manifest.toml
    //         writer.start_file("manifest.toml", options)?;
    //         let toml = toml::to_string_pretty(manifest)
    //             .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
    //         writer.write_all(toml.as_bytes())?;
    //         writer.finish()?;
    //     }

    //     // Persist the temp file to the target path
    //     temp_file
    //         .into_temp_path()
    //         .persist(&self.path)
    //         .map_err(|_| {
    //             AppError::ManifestError("Failed to persist temporary manifest file".to_string())
    //         })?;

    //     Ok(())
    // }
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), AppError> {
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
                .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
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
            AppError::ManifestError(format!("Failed to remove original file: {}", e))
        })?;

        // Rename temp file to original path
        log::info!("Renaming {:?} to {:?}", temp_path, &self.path);
        rename(&temp_path, &self.path).map_err(|e| {
            log::error!("Failed to rename temp file: {}", e);
            AppError::ManifestError(format!("Failed to rename temp file: {}", e))
        })?;

        log::info!("Manifest successfully written to {:?}", &self.path);
        Ok(())
    }
}
