use serde::{Deserialize, Serialize};

/// Metadata about a comic archive, such as title, author, web archive flag, and optional page comments.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    /// The title of the comic.
    pub title: String,
    /// The author of the comic.
    pub author: String,
    /// Whether this archive is a web archive (uses external URLs).
    pub web_archive: bool,
    /// Optional comments for each page.
    #[serde(default)]
    pub comments: Option<Vec<String>>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: "Unknown".to_string(),
            author: "Unknown".to_string(),
            web_archive: false,
            comments: None,
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
    fn default() -> Self {
        Self { urls: Vec::new() }
    }
}

/// The manifest for a comic archive, containing metadata, version, and optional external pages.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    /// Manifest format version.
    #[serde(default = "Manifest::default_version")]
    pub version: u32,
    /// Metadata for the archive, including comments.
    pub meta: Metadata,
    /// Optional list of external pages (for web archives).
    pub external_pages: Option<ExternalPages>,
}

impl Manifest {
    /// The default manifest version.
    pub fn default_version() -> u32 {
        1
    }

    /// Convert an old manifest (without version/comments) to the new format.
    pub fn upgrade_from_v0_to_v1(toml_str: &str) -> Result<Manifest, toml::de::Error> {
        let mut manifest: Manifest = toml::from_str(toml_str)?;
        if manifest.version == 0 {
            manifest.version = Manifest::default_version();
        }
        if manifest.meta.comments.is_none() {
            manifest.meta.comments = None;
        }
        Ok(manifest)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: Manifest::default_version(),
            meta: Metadata::default(),
            external_pages: None,
        }
    }
}
