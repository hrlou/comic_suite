use comic_archive::{ImageArchive, error::ArchiveError};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn print_usage() {
    eprintln!("Usage: comic_thumbgen <comic> <output.jpg> [image_name]");
    eprintln!("If image_name is omitted, the first image in the archive will be used.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }

    let archive_path = &args[1];
    let output_path = &args[2];
    let image_name = args.get(3);

    let mut archive = match ImageArchive::process(Path::new(archive_path)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open archive: {e}");
            std::process::exit(2);
        }
    };

    let image_list = archive.list_images();
    if image_list.is_empty() {
        eprintln!("No images found in archive.");
        std::process::exit(3);
    }

    let image_to_use = match image_name {
        Some(name) => {
            if image_list.contains(name) {
                name
            } else {
                eprintln!("Image '{}' not found in archive.", name);
                std::process::exit(4);
            }
        }
        None => &image_list[0],
    };

    let thumb = match archive.generate_thumbnail(image_to_use) {
        Ok(buf) => buf,
        Err(e) => {
            eprintln!("Failed to generate thumbnail: {e}");
            std::process::exit(5);
        }
    };

    let mut file = match File::create(output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {e}");
            std::process::exit(6);
        }
    };

    if let Err(e) = file.write_all(&thumb) {
        eprintln!("Failed to write thumbnail: {e}");
        std::process::exit(7);
    }

    println!("Thumbnail written to {}", output_path);
}
