#!/usr/bin/env python3
import os, sys
from pathlib import Path
try:
    from PIL import Image
except ImportError:
    Image = None

ROOT = Path("/home/steelworth/projects/blightnet/assets")

def save(src, dest_id, folder="bestiary"):
    dest_dir = ROOT / folder
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{dest_id}.jpg"
    if Image is None:
        os.system(f'ffmpeg -y -i {src!s} -vf scale=720:-1 -q:v 5 {dest} >/dev/null 2>&1')
        return dest
    im = Image.open(src).convert("RGB")
    im.thumbnail((720, 960), Image.Resampling.LANCZOS)
    im.save(dest, "JPEG", quality=82, optimize=True)
    print(dest, dest.stat().st_size)
    return dest

if __name__ == "__main__":
    folder = sys.argv[3] if len(sys.argv) > 3 else "bestiary"
    save(sys.argv[1], sys.argv[2], folder)
