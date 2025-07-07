use crate::prelude::*;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct RarImageArchive {
    path: PathBuf,
    entries: Vec<String>,
}

impl RarImageArchive {
    pub fn new(path: &Path) -> Result<Self, AppError> {
        let output = Command::new("unrar")
            .arg("l")
            .arg("-c-") // no comments, cleaner output
            .arg(path)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|_| AppError::UnsupportedArchive)?;

        if !output.status.success() {
            return Err(AppError::UnsupportedArchive);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut listing_started = false;

        for line in stdout.lines() {
            if line.trim().starts_with("--------") {
                listing_started = true;
                continue;
            }
            if listing_started {
                // Each line looks like: attrs size date time name
                // If line too short or blank, stop
                if line.trim().is_empty() {
                    break;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 5 {
                    break;
                }
                let filename = parts[4..].join(" ");
                let filename_lower = filename.to_lowercase();
                if filename_lower.ends_with(".jpg")
                    || filename_lower.ends_with(".jpeg")
                    || filename_lower.ends_with(".png")
                    || filename_lower.ends_with(".gif")
                {
                    entries.push(filename);
                }
            }
        }
        entries.sort();

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }
}

impl ImageArchiveTrait for RarImageArchive {
    fn list_images(&self) -> Vec<String> {
        self.entries.clone()
    }

    fn read_image_by_name(&mut self, filename: &str) -> Result<Vec<u8>, AppError> {
        let tmp_dir = tempdir().map_err(|_| AppError::UnsupportedArchive)?;
        let status = Command::new("unrar")
            .arg("x")
            .arg("-y") // assume yes
            .arg(&self.path)
            .arg(filename)
            .arg(tmp_dir.path())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|_| AppError::UnsupportedArchive)?;

        if !status.success() {
            return Err(AppError::UnsupportedArchive);
        }

        let extracted_path = tmp_dir.path().join(filename);
        let mut file = fs::File::open(&extracted_path).map_err(|_| AppError::NoImages)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|_| AppError::NoImages)?;

        Ok(buffer)
    }
}
