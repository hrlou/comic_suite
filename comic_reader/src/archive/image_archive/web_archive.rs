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
}

impl<T: ManifestAware> ManifestAware for WebImageArchive<T> {
    fn read_manifest(path: &Path) -> Result<Manifest, AppError> {
        let manifest = T::read_manifest(path)?;
        // manifest.meta.web_archive = true;
        Ok(manifest)
    }

    fn write_manifest(&self, path: &Path, manifest: &Manifest) -> Result<(), AppError> {
        let mut patched = manifest.clone();
        // patched.meta.web_archive = true;
        self.inner.write_manifest(path, &patched)
    }
}
