use crate::error::*;
use crate::is_supported_format;
use crate::prelude::*;
use std::fs;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir; // Add this import at the top

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct SevenZipImageArchive {
    #[allow(dead_code)]
    path: PathBuf,
    entries: Vec<String>,
    temp_dir: TempDir,
}

impl SevenZipImageArchive {
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        let temp_dir = tempfile::tempdir().map_err(|_| ArchiveError::NoImages)?;
        log::info!("Extracting all files from archive: {:?}", path);

        let mut cmd = Command::new("7z");
        cmd.arg("x")
            .arg(path)
            .arg(format!("-o{}", temp_dir.path().display()));

        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let status = cmd.status().map_err(|_| ArchiveError::NoImages)?;

        if !status.success() {
            log::info!("7z extraction failed for {:?}", path);
            return Err(ArchiveError::NoImages);
        }

        // Recursively collect all supported image files from temp_dir
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(temp_dir.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let rel_path = entry
                .path()
                .strip_prefix(temp_dir.path())
                .unwrap()
                .to_string_lossy()
                .to_string();
            let rel_path_lower = rel_path.to_lowercase();
            log::info!("found extracted file: '{}'", rel_path);
            if is_supported_format!(&rel_path_lower) {
                log::info!("accepted image: '{}'", rel_path);
                entries.push(rel_path);
            }
        }
        entries.sort();
        log::info!("Archive entries: {:?}", entries);

        Ok(Self {
            path: path.to_path_buf(),
            entries,
            temp_dir,
        })
    }

    /// Reads a file from the archive by name and returns its contents as bytes.
    fn read_file_by_name(&self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let extracted_path = self.temp_dir.path().join(filename);
        log::info!("Reading extracted file at {:?}", extracted_path);

        if !extracted_path.exists() {
            log::info!("Extracted file not found: {:?}", extracted_path);
            return Err(ArchiveError::NoImages);
        }

        let mut file = fs::File::open(&extracted_path).map_err(|_| ArchiveError::NoImages)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|_| ArchiveError::NoImages)?;
        log::info!(
            "Successfully read {} bytes from {:?}",
            buffer.len(),
            extracted_path
        );
        Ok(buffer)
    }
}

impl ImageArchiveTrait for SevenZipImageArchive {
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        self.read_file_by_name(filename)
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        match self.read_file_by_name("manifest.toml") {
            Ok(buffer) => String::from_utf8(buffer).map_err(|_| {
                ArchiveError::ManifestError("manifest.toml is not valid UTF-8".into())
            }),
            Err(_) => {
                log::info!("manifest.toml not found in archive");
                Err(ArchiveError::ManifestError(
                    "manifest.toml not found".into(),
                ))
            }
        }
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        let manifest_str = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&manifest_str)
            .map_err(|e| ArchiveError::ManifestError(format!("Invalid TOML: {}", e)))?;
        Ok(manifest)
    }

    fn write_manifest(&mut self, _manifest: &Manifest) -> Result<(), ArchiveError> {
        // TODO: implement writing manifest with CLI
        Ok(())
    }
}
