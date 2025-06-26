import requests
from bs4 import BeautifulSoup
import toml
import zipfile
import io
import sys
import time

def get_title_and_page_count(gallery_url):
    resp = requests.get(gallery_url)
    resp.raise_for_status()
    soup = BeautifulSoup(resp.text, 'html.parser')

    # Get title
    title_tag = soup.select_one("h1.title .pretty")
    title = title_tag.text.strip() if title_tag else "Untitled"

    # Count thumbnails for page count
    thumbs = soup.select("div.thumb-container img")
    page_count = len(thumbs)
    if page_count == 0:
        raise ValueError("Could not detect page count via thumbnails.")
    return title, page_count

def get_image_url(page_url):
    resp = requests.get(page_url)
    resp.raise_for_status()
    soup = BeautifulSoup(resp.text, 'html.parser')
    img = soup.select_one("#image-container img")
    if img:
        return img["src"]
    raise ValueError(f"Could not find image on page: {page_url}")

def generate_cbw_zip(title, author, image_urls, output_zip_path):
    manifest = {
        "meta": {
            "title": title,
            "author": author,
        },
        "pages": {
            "urls": image_urls
        }
    }

    manifest_toml = toml.dumps(manifest)

    with zipfile.ZipFile(output_zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("manifest.toml", manifest_toml)

def main():
    if len(sys.argv) != 3:
        print("Usage: python generate_nhentai_cbw.py <nhentai_url> <output.cbw>")
        sys.exit(1)

    gallery_url = sys.argv[1].rstrip("/")
    output_zip = sys.argv[2]

    title, page_count = get_title_and_page_count(gallery_url)
    print(f"Title: {title}")
    print(f"Pages: {page_count}")

    image_urls = []
    for i in range(1, page_count + 1):
        page_url = f"{gallery_url}/{i}"
        print(f"Fetching image URL for page {i}...")
        try:
            url = get_image_url(page_url)
            image_urls.append(url)
        except Exception as e:
            print(f"Error on page {i}: {e}")
            break
        time.sleep(0.25)  # be nice

    print(f"Creating CBW zip: {output_zip}")
    generate_cbw_zip(title, "nhentai", image_urls, output_zip)
    print("Done.")

if __name__ == "__main__":
    main()
