import requests
from bs4 import BeautifulSoup
import toml
import zipfile
import sys
import time

HEADERS = {
    "User-Agent": "Mozilla/5.0"
}

def get_title_and_page_count(url):
    resp = requests.get(url, headers=HEADERS)
    resp.raise_for_status()
    soup = BeautifulSoup(resp.text, 'html.parser')

    title_tag = soup.select_one("#gn")
    title = title_tag.text.strip() if title_tag else "Untitled"

    # Total number of pages
    pagecount_el = soup.find("td", string="Length:")
    if pagecount_el:
        pagecount_text = pagecount_el.find_next_sibling("td").text
        count = int(pagecount_text.split()[0])
    else:
        raise ValueError("Could not determine page count.")

    return title, count

def get_gallery_image_pages(base_url, total_pages):
    image_page_urls = []

    # paginated: every 40 thumbnails = one gallery page
    pages = (total_pages + 39) // 40

    for gallery_page in range(pages):
        page_url = base_url + f"?p={gallery_page}"
        print(f"Fetching gallery page: {page_url}")
        resp = requests.get(page_url, headers=HEADERS)
        resp.raise_for_status()
        soup = BeautifulSoup(resp.text, 'html.parser')

        thumbs = soup.select("div#gdt a")
        for a in thumbs:
            href = a["href"]
            image_page_urls.append(href)

        time.sleep(0.5)

    return image_page_urls[:total_pages]

def get_full_image_url(image_page_url):
    resp = requests.get(image_page_url, headers=HEADERS)
    resp.raise_for_status()
    soup = BeautifulSoup(resp.text, 'html.parser')

    img = soup.select_one("#img")
    if img:
        return img["src"]
    raise ValueError(f"Could not find full image on page: {image_page_url}")

def generate_cbw_zip(title, author, image_urls, output_path):
    manifest = {
        "meta": {
            "title": title,
            "author": author,
        },
        "pages": {
            "urls": image_urls
        }
    }
    with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("manifest.toml", toml.dumps(manifest))

def main():
    if len(sys.argv) != 3:
        print("Usage: python generate_ehentai_cbw.py <gallery_url> <output.cbw>")
        sys.exit(1)

    gallery_url = sys.argv[1].rstrip("/")
    output_file = sys.argv[2]

    title, total_pages = get_title_and_page_count(gallery_url)
    print(f"Title: {title}")
    print(f"Pages: {total_pages}")

    image_page_urls = get_gallery_image_pages(gallery_url, total_pages)

    image_urls = []
    for idx, page in enumerate(image_page_urls, 1):
        print(f"[{idx}/{total_pages}] Fetching full image URL...")
        try:
            img_url = get_full_image_url(page)
            image_urls.append(img_url)
        except Exception as e:
            print(f"Failed on {page}: {e}")
            break
        time.sleep(0.5)

    print(f"Writing CBW to {output_file}")
    generate_cbw_zip(title, "e-hentai", image_urls, output_file)
    print("Done.")

if __name__ == "__main__":
    main()
