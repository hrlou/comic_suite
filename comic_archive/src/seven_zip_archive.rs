use crate::is_supported_format;
use crate::prelude::*;
use sevenz_rust::{Archive, Password};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use log::info;

pub struct SevenZipImageArchive {
    path: PathBuf,
    entries: Vec<String>,
}

impl SevenZipImageArchive {
    pub fn new(path: &Path) -> Result<Self, ArchiveError> {
        info!("Opening 7z archive: {:?}", path);
        let mut file = File::open(path)?;
        let size = file.metadata()?.len();
        info!("Archive size: {}", size);
        let archive = Archive::read(&mut file, size, &[])
            .map_err(|e| {
                info!("Failed to read archive: {}", e);
                ArchiveError::Other(format!("7z error: {e}"))
            })?;
        let mut entries = Vec::new();
        for file_entry in &archive.files {
            let name = file_entry.name.to_lowercase();
            if is_supported_format!(&name) {
                info!("Found image entry: {}", file_entry.name);
                entries.push(file_entry.name.clone());
            }
        }
        entries.sort();
        info!("Total image entries found: {}", entries.len());
        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }
}

impl ImageArchiveTrait for SevenZipImageArchive {
    fn list_images(&self) -> Vec<String> {
        info!("Listing images: {:?}", self.entries);
        self.entries.clone()
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        use std::cell::RefCell;
        info!("Attempting to read image: {}", filename);

        // Find the exact entry name (case-sensitive, as stored in archive)
        let entry_name = self.entries.iter()
            .find(|name| name.eq_ignore_ascii_case(filename));
        if let Some(entry_name) = entry_name {
            let buf = RefCell::new(Vec::new());
            let found = RefCell::new(false);

            let result = sevenz_rust::decompress_file_with_extract_fn(
                &self.path,
                std::env::temp_dir(),
                |entry: &sevenz_rust::SevenZArchiveEntry, reader, _out_path| {
                    if entry.name.eq(entry_name) {
                        info!("Extracting entry: {}", entry.name);
                        let mut tmp = Vec::new();
                        std::io::copy(reader, &mut tmp)
                            .map_err(|e| {
                                info!("Failed to copy image data: {}", e);
                                sevenz_rust::Error::Io(e, "extract image".into())
                            })?;
                        buf.borrow_mut().extend(tmp);
                        info!("Image data extracted for: {}", entry.name);
                        *found.borrow_mut() = true;
                        return Ok(true);
                    }
                    Ok(false)
                },
            );

            if let Err(e) = &result {
                info!("Failed to decompress image: {}", e);
            }
            result.map_err(|e| ArchiveError::ManifestError(format!("7z error: {e}")))?;
            if *found.borrow() && !buf.borrow().is_empty() {
                info!("Returning image buffer of size: {}", buf.borrow().len());
                return Ok(buf.into_inner());
            }
        }

        log::error!("Image '{}' not found or empty in archive", filename);
        Err(ArchiveError::Other(format!("Image '{}' not found in archive", filename)))
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        info!("Reading manifest.toml from archive");
        let buf = std::cell::RefCell::new(Vec::new());
        let found = std::cell::RefCell::new(false);

        let result = sevenz_rust::decompress_file_with_extract_fn(
            &self.path,
            std::env::temp_dir(),
            |entry: &sevenz_rust::SevenZArchiveEntry, reader, _out_path| {
                info!("Checking entry: {}", entry.name);
                let entry_file = std::path::Path::new(&entry.name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&entry.name);
                if entry_file.eq_ignore_ascii_case("manifest.toml") {
                    info!("Found manifest.toml");
                    let mut tmp = Vec::new();
                    std::io::copy(reader, &mut tmp)
                        .map_err(|e| {
                            info!("Failed to copy manifest data: {}", e);
                            sevenz_rust::Error::Io(e, "extract manifest".into())
                        })?;
                    buf.borrow_mut().extend(tmp);
                    info!("Manifest data extracted");
                    *found.borrow_mut() = true;
                    return Ok(true);
                }
                Ok(false)
            },
        );
        if let Err(e) = &result {
            info!("Failed to decompress manifest: {}", e);
        }
        result.map_err(|e| ArchiveError::ManifestError(format!("7z error: {e}")))?;
        if !*found.borrow() || buf.borrow().is_empty() {
            log::error!("manifest.toml not found or empty in archive");
            return Err(ArchiveError::Other("manifest.toml not found in archive".to_string()));
        }
        let contents = String::from_utf8(buf.into_inner())
            .map_err(|e| {
                info!("Failed to decode manifest as UTF8: {}", e);
                ArchiveError::ManifestError(format!("UTF8 error: {e}"))
            })?;
        info!("Manifest string read successfully");
        Ok(contents)
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        info!("Reading manifest struct from archive");
        let contents = self.read_manifest_string()?;
        let manifest: Manifest = toml::from_str(&contents)
            .map_err(|e| {
                info!("Failed to parse manifest TOML: {}", e);
                ArchiveError::ManifestError(format!("Invalid TOML: {}", e))
            })?;
        info!("Manifest parsed successfully");
        Ok(manifest)
    }

    fn write_manifest(&mut self, _manifest: &Manifest) -> Result<(), ArchiveError> {
        info!("Attempting to write manifest (not supported)");
        Err(ArchiveError::Other(
            "Writing to 7z archives is not supported".to_string(),
        ))
    }
}
