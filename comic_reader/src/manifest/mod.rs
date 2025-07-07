use crate::prelude::*;
use serde::{Deserialize, Serialize};
pub mod editor;
pub use editor::ManifestEditor;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub web_archive: bool,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: "Unknown".to_string(),
            author: "Unknown".to_string(),
            web_archive: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalPages {
    pub urls: Vec<String>,
}

impl Default for ExternalPages {
    fn default() -> Self {
        Self { urls: Vec::new() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub meta: Metadata,
    pub external_pages: Option<ExternalPages>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            meta: Metadata::default(),
            external_pages: None,
        }
    }
}

/// Archives that support embedded manifest metadata
pub trait ManifestAware {
    fn read_manifest(path: &Path) -> Result<Manifest, AppError>
    where
        Self: Sized;
    fn write_manifest(&self, path: &Path, manifest: &Manifest) -> Result<(), AppError>;
    // fn manifest_mut(&mut self) -> &mut Manifest;
}