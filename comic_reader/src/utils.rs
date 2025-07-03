use crate::prelude::*;

use std::io::Write;
use zip::ZipWriter;

fn rebuild_zip_with_manifest(original_path: &Path, manifest: &Manifest) -> Result<(), AppError> {
    // Open original archive
    let original_file = File::open(original_path)?;
    let mut zip = ZipArchive::new(original_file)?;

    // Create a temporary output path next to original
    let mut temp_path = original_path.with_extension("rebuild.tmp.zip");
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
