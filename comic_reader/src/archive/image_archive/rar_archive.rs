use crate::prelude::*;

use unrar::Archive;
use unrar::archive::{ArchiveList, ArchiveExtract};

pub struct RarImageArchive {
    path: PathBuf,
    entries: Vec<String>,
}

impl RarImageArchive {
    pub fn new(path: &Path) -> Result<Self, AppError> {
        let mut entries = Vec::new();
        let listing = Archive::new(path.to_str().unwrap()).list().process();

        match listing {
            Ok(items) => {
                for item in items.iter() {
                    if let Some(name) = &item.filename {
                        if name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png") || name.ends_with(".gif") {
                            entries.push(name.to_owned());
                        }
                    }
                }
                entries.sort();
            }
            Err(_) => return Err(AppError::UnsupportedArchive),
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }
}

impl ImageArchiveTrait for RarImageArchive {
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError> {
        let mut buffer = Vec::new();
        Archive::new(self.path.to_str().unwrap())
            .extract()
            .file(filename)
            .process_to_memory()
            .map_err(|_| AppError::UnsupportedArchive)
            .and_then(|files| {
                if let Some(file) = files.first() {
                    Ok(file.data.clone())
                } else {
                    Err(AppError::NoImages)
                }
            })
    }
}