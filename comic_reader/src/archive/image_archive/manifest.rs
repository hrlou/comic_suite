use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub web_archive: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExternalPages {
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub meta: Metadata,
    pub external_pages: Option<ExternalPages>,
}
