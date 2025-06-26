use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use image::{DynamicImage, imageops::FilterType};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
struct Manifest {
    pages: Vec<String>,
}

pub fn generate_thumb_cbw(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let folder = path.parent().ok_or("No parent folder")?;
    let manifest_path = folder.join("manifest.toml");
    let manifest_str = fs::read_to_string(&manifest_path)?;
    let manifest: Manifest = toml::from_str(&manifest_str)?;

    let first = manifest.pages.first().ok_or("No pages listed")?;
    let img = if is_http_url(first) {
        let bytes = reqwest::blocking::get(first)?.bytes()?;
        image::load_from_memory(&bytes)?
    } else {
        let img_path = folder.join(first);
        image::open(img_path)?
    };

    save_thumbnail(path, &img)
}

fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn save_thumbnail(original: &Path, img: &DynamicImage) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let thumb = img.resize(320, 320, image::imageops::FilterType::Lanczos3);
    let thumb_path = original.with_extension("jpg");

    let file = File::create(&thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 85);
    encoder.encode_image(&thumb)?;

    Ok(thumb_path)
}