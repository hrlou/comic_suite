use std::fs::{self, File};
use std::io::{Read, Cursor};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use image::{DynamicImage, imageops::FilterType};

pub fn generate_thumb_cbz(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_ascii_lowercase();
        if name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".jpeg") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            let img = image::load_from_memory(&buf)?;
            return save_thumbnail(path, &img);
        }
    }
    Err("No image found".into())
}

fn save_thumbnail(original: &Path, img: &DynamicImage) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let thumb = img.resize(320, 320, image::imageops::FilterType::Lanczos3);
    let thumb_path = original.with_extension("jpg");

    let file = File::create(&thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 85);
    encoder.encode_image(&thumb)?;

    Ok(thumb_path)
}