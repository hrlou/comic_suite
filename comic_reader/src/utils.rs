use crate::prelude::*;

use std::io::Write;
use zip::{ZipWriter, write::FileOptions};

pub fn rebuild_zip_with_manifest(
    original_path: &Path,
    manifest: &Manifest,
) -> Result<(), AppError> {
    // Open original archive
    let original_file = File::open(original_path)?;
    let mut zip = ZipArchive::new(original_file)?;

    // Create a temporary output path next to original
    let temp_path = original_path.with_extension("rebuild.tmp.zip");
    let mut temp_file = File::create(&temp_path)?;

    let mut writer = ZipWriter::new(&mut temp_file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Copy existing entries, skipping manifest.toml
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();

        if name == "manifest.toml" {
            continue;
        }

        writer.start_file(name, options)?;
        std::io::copy(&mut file, &mut writer)?;
    }

    // Write new manifest.toml
    writer.start_file("manifest.toml", options)?;
    let toml = toml::to_string_pretty(manifest)
        .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
    writer.write_all(toml.as_bytes())?;
    writer.finish()?;

    // Replace original file
    fs::rename(temp_path, original_path)?;

    Ok(())
}

pub fn create_cbz_with_manifest(path: &std::path::Path) -> Result<(), AppError> {
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
