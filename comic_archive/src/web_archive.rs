use crate::prelude::*;

/// A wrapper archive backend for web-based comic archives (external image URLs).
///
/// `WebImageArchive` wraps any other archive backend, but overrides image listing and reading
/// to use external URLs specified in the manifest. This allows support for "web archives"
/// where images are not stored in the archive, but referenced by URL.
pub struct WebImageArchive<T> {
    /// The inner archive backend (for manifest fallback and compatibility).
    pub inner: T,
    /// The manifest containing external page URLs.
    pub manifest: Manifest,
}

impl<T: ImageArchiveTrait> WebImageArchive<T> {
    /// Create a new `WebImageArchive` from an inner backend and a manifest.
    pub fn new(inner: T, manifest: Manifest) -> Self {
        Self { inner, manifest }
    }
}

impl<T: ImageArchiveTrait> ImageArchiveTrait for WebImageArchive<T> {
    /// List all external image URLs from the manifest.
    fn list_images(&self) -> Vec<String> {
        if let Some(images) = self.manifest.external_pages.clone() {
            images.urls
        } else {
            vec![]
        }
    }

    /// Download and return the raw bytes of an image by URL.
    ///
    /// # Arguments
    ///
    /// * `filename` - The URL of the image to fetch.
    ///
    /// # Returns
    ///
    /// A vector of bytes containing the image data, or an `ArchiveError` on failure.
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

    /// Read the manifest from the inner backend, or return our own if not present.
    fn read_manifest(&self) -> Result<Manifest, ArchiveError> {
        self.inner.read_manifest().or_else(|_| {
            // If the inner archive doesn't have a manifest, return our own
            Ok(self.manifest.clone())
        })
    }

    /// Write the manifest to the inner backend.
    fn write_manifest(&mut self, manifest: &Manifest) -> Result<(), ArchiveError> {
        self.inner.write_manifest(manifest)?;
        Ok(())
    }
}
