// Wrapper for CBWs
pub struct WebImageArchive {
    pub images: Vec<String>,
}

impl WebImageArchive {
    pub fn list_images(&self) -> Vec<String> {
        self.images.clone()
    }

    pub fn read_image(&mut self, filename: &str) -> Result<Vec<u8>, crate::error::AppError> {
        // Use reqwest blocking client to fetch image bytes
        let resp = reqwest::blocking::get(filename).map_err(|e| {
            crate::error::AppError::NetworkError(format!("Failed to GET {}: {}", filename, e))
        })?;

        if !resp.status().is_success() {
            return Err(crate::error::AppError::NetworkError(format!(
                "HTTP error {} for {}",
                resp.status(),
                filename
            )));
        }

        let bytes = resp.bytes().map_err(|e| {
            crate::error::AppError::NetworkError(format!(
                "Failed to read bytes from {}: {}",
                filename, e
            ))
        })?;

        Ok(bytes.to_vec())
    }
}