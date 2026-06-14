#!/usr/bin/env python3
"""Download Foozle's free Legend main character pack into ignored assets.

The pack is distributed as CC0 and can be modified and used commercially. The
raw source art still lives under assets/sprites so local users can refresh it
without mixing third-party art into ordinary code diffs.
"""

from __future__ import annotations

import json
import re
import urllib.parse
import urllib.request
from http.cookiejar import CookieJar
from pathlib import Path
from zipfile import ZipFile


PAGE_URL = "https://foozlecc.itch.io/legend-main-character"
PURCHASE_URL = f"{PAGE_URL}/purchase"
DOWNLOAD_URL = f"{PAGE_URL}/download_url"
TARGET_FILE = "Foozle_2DC0029_Legend_MainCharacter.zip"
TARGET_DIR = Path("assets/sprites/squad/foozle")


def main() -> int:
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(CookieJar()))
    purchase_html = fetch_text(opener, PURCHASE_URL)
    csrf = parse_csrf(purchase_html)

    download_payload = post_form(opener, DOWNLOAD_URL, {"csrf_token": csrf}, PURCHASE_URL)
    download_page_url = json.loads(download_payload)["url"]
    download_html = fetch_text(opener, download_page_url)
    csrf = parse_csrf(download_html)
    upload_id = parse_upload_id(download_html)

    file_payload = post_form(
        opener,
        f"{PAGE_URL}/file/{upload_id}?source=game_download",
        {"csrf_token": csrf},
        download_page_url,
    )
    zip_url = json.loads(file_payload)["url"]
    zip_bytes = fetch_bytes(opener, zip_url)

    TARGET_DIR.mkdir(parents=True, exist_ok=True)
    with ZipFile(io_bytes(zip_bytes)) as archive:
        extracted = 0
        for member in archive.infolist():
            if member.is_dir():
                continue
            member_path = Path(member.filename)
            if member_path.suffix.lower() not in {".png", ".aseprite", ".txt", ".gif"}:
                continue
            target = TARGET_DIR / member_path.name
            target.write_bytes(archive.read(member))
            extracted += 1

    print(f"Extracted {extracted} Foozle files to {TARGET_DIR}")
    print("Source:", PAGE_URL)
    return 0


def fetch_text(opener: urllib.request.OpenerDirector, url: str) -> str:
    return fetch_bytes(opener, url).decode("utf-8")


def fetch_bytes(opener: urllib.request.OpenerDirector, url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "HackmasterSim asset helper"})
    with opener.open(request) as response:
        return response.read()


def post_form(
    opener: urllib.request.OpenerDirector,
    url: str,
    fields: dict[str, str],
    referer: str,
) -> str:
    body = urllib.parse.urlencode(fields).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=body,
        headers={
            "Accept": "application/json, text/javascript, */*; q=0.01",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "Referer": referer,
            "User-Agent": "HackmasterSim asset helper",
            "X-Requested-With": "XMLHttpRequest",
        },
    )
    with opener.open(request) as response:
        return response.read().decode("utf-8")


def parse_csrf(html: str) -> str:
    match = re.search(r'<meta name="csrf_token" value="([^"]+)"', html)
    if not match:
        raise RuntimeError("Could not find itch.io csrf token")
    return match.group(1)


def parse_upload_id(html: str) -> str:
    pattern = re.compile(
        r'data-upload_id="(?P<id>\d+)"(?:(?!data-upload_id).)*?'
        + re.escape(TARGET_FILE),
        re.DOTALL,
    )
    match = pattern.search(html)
    if not match:
        raise RuntimeError(f"Could not find upload id for {TARGET_FILE}")
    return match.group("id")


def io_bytes(data: bytes):
    import io

    return io.BytesIO(data)


if __name__ == "__main__":
    raise SystemExit(main())
