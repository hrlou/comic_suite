use crate::prelude::*;

/// Wrapper for zip files, providing access to image files.
pub struct ZipImageArchive {
    pub path: PathBuf,
    pub manifest: Manifest,
    // Optionally: pub zip: ZipArchive<File>,
}

impl ZipImageArchive {
    pub fn list_images(&self) -> Vec<String> {
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

    pub fn read_manifest<P: AsRef<Path>>(path: P) -> Result<Manifest, AppError> {
        let file = File::open(path.as_ref())?;
        let mut zip = ZipArchive::new(file)?;
        let manifest_file = zip.by_name("manifest.toml");

        if let Ok(mut manifest_file) = manifest_file {
            let mut contents = String::new();
            manifest_file.read_to_string(&mut contents)?;
            let manifest = toml::from_str(&contents)
                .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
            Ok(manifest)
        } else {
            Ok(Manifest::default())
        }
    }

    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        let file = File::open(&self.path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut file = zip.by_name(filename)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
