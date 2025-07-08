# Comic Suite

Comic Suite is a modular, cross-platform comic archive toolkit and reader. It provides a fast, modern comic book reader (`comic_reader`), a thumbnail generator CLI (`comic_thumbgen`), and a reusable Rust library (`comic_archive`) for working with comic book archives (CBZ, CBR, and web-based comics).

---

## Features

- **Read CBZ (ZIP), CBR (RAR), and web-based comic archives**
- **Fast, responsive UI with pan/zoom and dual/single page modes**
- **Manifest support for metadata and web archives**
- **Thumbnail generation CLI for integration with file managers**
- **Modular Rust library for archive handling, usable in other projects**
- **Cross-platform: Windows and Linux support**

---

## Project Structure

- **comic_reader**: The main GUI comic book reader application (eframe/egui-based).
- **comic_thumbgen**: A CLI tool to generate JPEG thumbnails from comic archives.
- **comic_archive**: A Rust library crate providing archive reading, manifest, and thumbnail logic.

---

## Building and Installing

### Prerequisites

- **Rust toolchain** (https://rustup.rs)
- **Cargo** (comes with Rust)
- **Git** (to clone the repository)

#### For CBR/RAR support:
- **Windows**: [WinRAR](https://www.win-rar.com/) or [UnRAR](https://www.rarlab.com/rar_add.htm) must be installed and available in your `PATH`.
- **Linux**: Install `unrar` via your package manager (`sudo apt install unrar` or equivalent).

---

### Windows

1. **Clone the repository:**

   ```sh
   git clone https://github.com/yourusername/comic_suite.git
   cd comic_suite
   ```

2. **Build the project:**

   ```sh
   cargo build --release --all-features
   ```

3. **Run the comic reader:**

   ```sh
   cargo run -p comic_reader --release
   ```

4. **Generate a thumbnail:**

   ```sh
   cargo run -p comic_thumbgen --release -- <archive.cbz|archive.cbr> <output.jpg>
   ```

5. **(Optional) Build an installer:**
   - Install [Inno Setup](https://jrsoftware.org/isinfo.php).
   - Run the installer generator (see workspace documentation).

---

### Linux

1. **Clone the repository:**

   ```sh
   git clone https://github.com/yourusername/comic_suite.git
   cd comic_suite
   ```

2. **Install dependencies:**

   ```sh
   sudo apt install libgtk-3-dev unrar
   ```

3. **Build the project:**

   ```sh
   cargo build --release --all-features
   ```

4. **Run the comic reader:**

   ```sh
   cargo run -p comic_reader --release
   ```

5. **Generate a thumbnail:**

   ```sh
   cargo run -p comic_thumbgen --release -- <archive.cbz|archive.cbr> <output.jpg>
   ```

---

## Usage

- **comic_reader**: Open CBZ/CBR files, browse pages, pan/zoom, and view metadata.
- **comic_thumbgen**: Generate a JPEG thumbnail for a comic archive (for file manager integration or scripts).
- **comic_archive**: Use as a Rust library in your own projects to read, write, and process comic archives.

---

## CBW: Comic Book Web File

Comic Suite supports "web archives"—CBZ files that reference images on the internet via a manifest.

```
webcomic.web.cbz
├── manifest.toml   # TOML manifest
└── thumb.jpg    	# optional: thumbnail
```

Example manifest:
```toml
# Example CBW (Comic Book Web) Manifest
# Reference a comic from the internet

[meta]
title = "Foxes"
author = "Google Images"
web_archive = true

[external_pages]
urls = [
    "https://upload.wikimedia.org/wikipedia/commons/3/30/Vulpes_vulpes_ssp_fulvus.jpg",
    "https://i.natgeofe.com/k/6496b566-0510-4e92-84e8-7a0cf04aa505/red-fox-portrait_3x4.jpg",
    "https://maymont.org/wp-content/uploads/2020/04/banner-red-fox.jpg",
    "https://naturecanada.ca/wp-content/uploads/2022/01/January-2022-3.png",
    "https://friendsofanimals.org/wp-content/uploads/2023/12/foxfb.png",
]
```

---

## License

This project is licensed under the BSD 3-Clause License.  
See [LICENSE.md](LICENSE.md) for details.


---

## Contributing

Contributions, bug reports, and feature requests are welcome!  
Please open an issue or pull request on GitHub.

---

## Credits

- Built with [Rust](https://www.rust-lang.org/), [eframe/egui](https://github.com/emilk/egui), and [image](https://github.com/image-rs/image).
- RAR/CBR support via external tools (`unrar`, `rar`, `WinRAR`).

---

**Enjoy your comics!**
