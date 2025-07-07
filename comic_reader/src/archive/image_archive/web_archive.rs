use crate::prelude::*;

/// WebImageArchive wraps any archive backend but overrides manifest handling.
pub struct WebImageArchive<T> {
    pub inner: T,
    pub manifest: Manifest,
}

impl<T: ImageArchiveTrait> WebImageArchive<T> {
    pub fn new(inner: T, manifest: Manifest) -> Self {
        Self { inner, manifest }
    }
}

impl<T: ImageArchiveTrait> ImageArchiveTrait for WebImageArchive<T> {
    fn list_images(&self) -> Vec<String> {
        if let Some(images) = self.manifest.external_pages.clone() {
            images.urls
        } else {
            vec![]
        }
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError> {
        let resp = reqwest::blocking::get(filename).map_err(|e| {
            crate::error::AppError::NetworkError(format!("Failed to GET {}: {}", filename, e))
        })?;

        if !resp.status().is_success() {
            return Err(AppError::NetworkError(format!(
                "HTTP error {} for {}",
                resp.status(),
                filename
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            AppError::NetworkError(format!("Failed to read bytes from {}: {}", filename, e))
        })?;

        Ok(bytes.to_vec())
    }

    fn read_manifest(&self) -> Result<Manifest, AppError> {
        self.inner.read_manifest().or_else(|_| {
            // If the inner archive doesn't have a manifest, return our own
            Ok(self.manifest.clone())
        })
    }

    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), AppError> {
        self.inner.write_manifest(manifest)?;
        Ok(())
    }
}
