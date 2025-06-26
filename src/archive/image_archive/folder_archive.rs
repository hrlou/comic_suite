use crate::prelude::*;

/// Wrapper for folders, providing access to image files (flat, no nesting).
pub struct FolderImageArchive {
    pub path: PathBuf,
}

impl FolderImageArchive {
    pub fn list_images(&self) -> Vec<String> {
        let mut images = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ["jpg", "jpeg", "png", "gif"].contains(&ext.to_lowercase().as_str()) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            images.push(name.to_string());
                        }
                    }
                }
            }
        }
        images
    }

    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        let path = self.path.join(filename);
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
