use crate::error::AppError;
use crate::archive::image_archive::{Manifest, ManifestAware, ImageArchiveTrait};

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
                if name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png") || name.ends_with(".gif") {
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
}

impl ManifestAware for ZipImageArchive {
    fn read_manifest(path: &Path) -> Result<Manifest, AppError> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(file)?;
        let manifest_file = zip.by_name("manifest.toml");

        if let Ok(mut mf) = manifest_file {
            let mut contents = String::new();
            mf.read_to_string(&mut contents)?;
            let manifest = toml::from_str(&contents)
                .map_err(|e| AppError::ManifestError(format!("Invalid TOML: {}", e)))?;
            Ok(manifest)
        } else {
            Ok(Manifest::default())
        }
    }

    fn write_manifest(&self, _path: &Path, _manifest: &Manifest) -> Result<(), AppError> {
        // TODO: Implement manifest writing back into zip
        // Err(AppError::)
        todo!("ZipImageArchive::write_manifest not implemented yet")
    }
}
