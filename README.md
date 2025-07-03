# Rust Simple CBZ Viewer

## Purpose
I was quite annoyed with CDisplayEX being slow, ugly, and crashing often. 
This is much more performant and lightweight.
If you want to have thumbnails in Windows file explorer, use CBXShell in the resources folder.

## CBW
Comic book web file.

```
webcomic.web.cbz
├── manifest.toml   # TOML manifest
└── thumb.jpg    	# optional: thumbnail
```
Manifest:
```toml
# Example CBW (Comic Book Web) Manifest
# Reference a comic from the internet

[metadata]
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
