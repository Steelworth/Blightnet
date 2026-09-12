#!/usr/bin/env python3
"""Chroma-key magenta/green creator layers and resize into assets/creator."""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
CREATOR = ROOT / "assets" / "creator"

SIZES = {
    "bodies": (384, 768),
    "faces": (384, 512),
    "hair": (384, 512),
    "eyes": (256, 128),
    "wear": (384, 768),
    "chrome": (384, 768),
    "scars": (256, 256),
    "loss": (256, 256),
}


def key_magenta(im: Image.Image, thresh: int = 48) -> Image.Image:
    im = im.convert("RGBA")
    px = im.load()
    w, h = im.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if r > 180 and b > 180 and g < r - 40 and g < b - 40:
                dist = min(r, b) - g
                fade = max(0, min(255, int((dist - thresh) * 4)))
                px[x, y] = (r, g, b, max(0, a - fade))
            elif g > 180 and r < 80 and b < 80:
                dist = g - max(r, b)
                fade = max(0, min(255, int((dist - thresh) * 4)))
                px[x, y] = (r, g, b, max(0, a - fade))
    return im


def fit(im: Image.Image, size: tuple[int, int]) -> Image.Image:
    canvas = Image.new("RGBA", size, (0, 0, 0, 0))
    im.thumbnail(size, Image.Resampling.LANCZOS)
    x = (size[0] - im.width) // 2
    y = (size[1] - im.height) // 2
    canvas.paste(im, (x, y), im if im.mode == "RGBA" else None)
    return canvas


def save_layer(src: Path, dest: Path, kind: str, key: bool = True) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    im = Image.open(src)
    if key:
        im = key_magenta(im)
    else:
        im = im.convert("RGB")
    size = SIZES.get(kind, (384, 768))
    if key:
        im = fit(im.convert("RGBA"), size)
        im.save(dest, "PNG", optimize=True)
    else:
        im = im.convert("RGB")
        im.thumbnail(size, Image.Resampling.LANCZOS)
        canvas = Image.new("RGB", size, (26, 26, 28))
        x = (size[0] - im.width) // 2
        y = (size[1] - im.height) // 2
        canvas.paste(im, (x, y))
        canvas.save(dest, "JPEG", quality=90, optimize=True)


def main(argv: list[str]) -> int:
    if len(argv) < 4:
        print("usage: ingest_creator.py SRC DEST_REL kind [--key|--nokey]")
        return 2
    src = Path(argv[1])
    dest = CREATOR / argv[2]
    kind = argv[3]
    key = "--nokey" not in argv
    save_layer(src, dest, kind, key=key)
    print("wrote", dest, dest.stat().st_size)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
