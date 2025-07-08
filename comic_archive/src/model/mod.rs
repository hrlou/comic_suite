use serde::{Deserialize, Serialize};

/// Metadata about a comic archive, such as title, author, and web archive flag.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    /// The title of the comic.
    pub title: String,
    /// The author of the comic.
    pub author: String,
    /// Whether this archive is a web archive (uses external URLs).
    pub web_archive: bool,
}

impl Default for Metadata {
    /// Returns default metadata with "Unknown" title and author, and web_archive set to false.
    fn default() -> Self {
        Self {
            title: "Unknown".to_string(),
            author: "Unknown".to_string(),
            web_archive: false,
        }
    }
}

/// A list of external page URLs for web archives.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalPages {
    /// URLs of external pages (images).
    pub urls: Vec<String>,
}

impl Default for ExternalPages {
    /// Returns an empty list of external pages.
    fn default() -> Self {
        Self { urls: Vec::new() }
    }
}

/// The manifest for a comic archive, containing metadata and optional external pages.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    /// Metadata for the archive.
    pub meta: Metadata,
    /// Optional list of external pages (for web archives).
    pub external_pages: Option<ExternalPages>,
}

impl Default for Manifest {
    /// Returns a manifest with default metadata and no external pages.
    fn default() -> Self {
        Self {
            meta: Metadata::default(),
            external_pages: None,
        }
    }
}