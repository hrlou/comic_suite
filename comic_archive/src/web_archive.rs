use crate::prelude::*;

pub struct WebImageArchive<T> {
    pub inner: T,
    pub manifest: Manifest,
}

impl<T: ImageArchiveTrait> WebImageArchive<T> {
    pub fn new(inner: T, manifest: Manifest) -> Self {
        Self { inner, manifest }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<T: ImageArchiveTrait + Send + Sync> ImageArchiveTrait for WebImageArchive<T> {
    fn list_images(&self) -> Vec<String> {
        if let Some(images) = self.manifest.external_pages.clone() {
            images.urls
        } else {
            vec![]
        }
    }

    fn read_image_by_name_sync(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let resp = reqwest::blocking::get(filename).map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to GET {}: {}", filename, e))
        })?;

        if !resp.status().is_success() {
            return Err(ArchiveError::NetworkError(format!(
                "HTTP error {} for {}",
                resp.status(),
                filename
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to read bytes from {}: {}", filename, e))
        })?;

        Ok(bytes.to_vec())
    }

    async fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let resp = reqwest::get(filename).await.map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to GET {}: {}", filename, e))
        })?;

        if !resp.status().is_success() {
            return Err(ArchiveError::NetworkError(format!(
                "HTTP error {} for {}",
                resp.status(),
                filename
            )));
        }

        let bytes = resp.bytes().await.map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to read bytes from {}: {}", filename, e))
        })?;

        Ok(bytes.to_vec())
    }

    async fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.inner.read_manifest_string().await
    }

    async fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.inner
            .read_manifest()
            .await
            .or_else(|_| Ok(self.manifest.clone()))
    }

    async fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.inner.write_manifest(manifest).await
    }
}

#[cfg(not(feature = "async"))]
impl<T: ImageArchiveTrait> ImageArchiveTrait for WebImageArchive<T> {
    fn list_images(&self) -> Vec<String> {
        if let Some(images) = self.manifest.external_pages.clone() {
            images.urls
        } else {
            vec![]
        }
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, ArchiveError> {
        let resp = reqwest::blocking::get(filename).map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to GET {}: {}", filename, e))
        })?;

        if !resp.status().is_success() {
            return Err(ArchiveError::NetworkError(format!(
                "HTTP error {} for {}",
                resp.status(),
                filename
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            ArchiveError::NetworkError(format!("Failed to read bytes from {}: {}", filename, e))
        })?;

        Ok(bytes.to_vec())
    }

    fn read_manifest_string(&self) -> Result<String, ArchiveError> {
        self.inner.read_manifest_string()
    }

    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.inner
            .read_manifest()
            .or_else(|_| Ok(self.manifest.clone()))
    }

    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.inner.write_manifest(manifest)
    }
}
