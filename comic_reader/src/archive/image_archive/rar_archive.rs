#![cfg(feature = "rar")]

use crate::prelude::*;
use std::process::Command;

pub struct RarImageArchive {
    pub path: PathBuf,
    pub manifest: Manifest,
}

impl RarImageArchive {
    pub fn list_images(&self) -> Vec<String> {
        let output = Command::new("unrar")
            .arg("lb") // list bare
            .arg(&self.path)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut images = stdout
                .lines()
                .filter(|line| {
                    let lower = line.to_lowercase();
                    lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".png")
                        || lower.ends_with(".gif")
                })
                .map(String::from)
                .collect::<Vec<_>>();
            images.sort();
            images
        } else {
            vec![]
        }
    }

    pub fn read_image(&self, filename: &str) -> Result<Vec<u8>, AppError> {
        let output = Command::new("unrar")
            .arg("p") // print to stdout
            .arg(&self.path)
            .arg(filename)
            .output()
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unrar failed: {e}"),
                ))
            })?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to extract: {}", filename),
            )))
        }
    }

    pub fn read_manifest<P: AsRef<Path>>(path: P) -> Result<Manifest, AppError> {
        let path = path.as_ref();

        // Run `unrar` to extract "manifest.toml" contents to stdout
        let output = Command::new("unrar")
            .arg("p") // print file to stdout
            .arg(path)
            .arg("manifest.toml")
            .output()
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to run unrar: {e}"),
                ))
            })?;

        if !output.status.success() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to extract manifest.toml from rar archive"),
            )));
        }

        // Parse output.stdout as UTF-8 string
        let manifest_text = String::from_utf8(output.stdout).map_err(|e| {
            AppError::ManifestError(format!("Manifest not valid UTF-8: {}", e))
        })?;

        // Parse TOML from string
        let manifest: Manifest = toml::from_str(&manifest_text).map_err(|e| {
            AppError::ManifestError(format!("Invalid TOML: {}", e))
        })?;

        Ok(manifest)
    }
}
