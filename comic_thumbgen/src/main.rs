use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

use image::{DynamicImage};
use image::codecs::jpeg::JpegEncoder;
use walkdir::WalkDir;

mod cbz;
mod cbw;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(root) = args.get(1) else {
        eprintln!("Usage: cb_thumbgen <folder>");
        std::process::exit(1);
    };

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().map(|e| e == "cbz").unwrap_or(false) {
            match cbz::generate_thumb_cbz(path) {
                Ok(out) => println!("Wrote thumbnail: {}", out.display()),
                Err(e) => eprintln!("Error: {}: {e}", path.display()),
            }
        } else if path.extension().map(|e| e == "cbw").unwrap_or(false) {
            match cbw::generate_thumb_cbw(path) {
                Ok(out) => println!("Wrote thumbnail: {}", out.display()),
                Err(e) => eprintln!("Error: {}: {e}", path.display()),
            }
        }
    }
}