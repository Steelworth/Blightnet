#!/usr/bin/env python3
"""Pull a huge Blight radio library (Kevin MacLeod CC BY) in four vibes."""

from __future__ import annotations

import json
import re
import subprocess
import urllib.parse
import urllib.request
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw" / "blight_music"
AUDIO = ROOT / "audio" / "blight" / "music"
OUT_JS = ROOT / "js" / "blight-radio.js"
PIECES = Path("/tmp/pieces.json")
UA = "Blightnet/3.6 (local table app)"
BASE = "https://incompetech.com/music/royalty-free/mp3-royaltyfree/"
PER_VIBE = 100
OWNED = {
    "Newer Wave.mp3",
    "Neon Laser Horizon.mp3",
    "Cyborg Ninja.mp3",
    "Space Fighter Loop.mp3",
    "Reformat.mp3",
    "Ethernight Club.mp3",
    "Equatorial Complex.mp3",
    "Cloud Dancer.mp3",
    "Laser Groove.mp3",
    "Digital Lemonade.mp3",
    "EDM Detection Mode.mp3",
    "Laserpack.mp3",
    "Raving Energy.mp3",
    "Dance Monster.mp3",
    "Cut Trance.mp3",
    "Voltaic.mp3",
    "Club Diver.mp3",
    "Kick Shock.mp3",
    "Shiny Tech.mp3",
    "Mega Hyper Ultrastorm.mp3",
    "Chill Wave.mp3",
    "Mellowtron.mp3",
    "Beauty Flow.mp3",
    "Dream Catcher.mp3",
    "Hypnothis.mp3",
    "Ambler.mp3",
    "Chillin Hard.mp3",
    "Deliberate Thought.mp3",
    "Tranquility.mp3",
    "Rising Tide.mp3",
    "Metalmania.mp3",
    "Ready Aim Fire.mp3",
    "Twisted.mp3",
    "Broken Reality.mp3",
    "Severe Tire Damage.mp3",
    "Exit the Premises.mp3",
    "Furious Freak.mp3",
    "Danger Storm.mp3",
    "Exhilarate.mp3",
    "Motherlode.mp3",
    "Hotrock.mp3",
    "Big Rock.mp3",
    "Cool Rock.mp3",
    "Too Cool.mp3",
    "RetroFuture Nasty.mp3",
    "Voxel Revolution.mp3",
    "Brain Dance.mp3",
    "Tyrant.mp3",
    "Raving Energy (faster).mp3",
    "Go Cart.mp3",
    "Show Your Moves.mp3",
    "Phat Sketch.mp3",
    "Future Cha Cha.mp3",
    "Harmful or Fatal.mp3",
    "Deep and Dirty.mp3",
    "Limit 70.mp3",
    "Blippy Trance.mp3",
    "Bit Shift.mp3",
    "Cipher2.mp3",
    "Getting it Done.mp3",
    "space explorers.mp3",
}

GENRES = {
    2: "African",
    3: "Blues",
    4: "Classical",
    5: "Contemporary",
    6: "Disco",
    7: "Electronica",
    8: "Funk",
    9: "Holiday",
    10: "Horror",
    11: "Jazz",
    13: "Modern",
    12: "Latin",
    14: "Musical",
    15: "Polka",
    18: "Reggae",
    19: "Rock",
    20: "Silent Film",
    21: "Ska",
    22: "Soundtrack",
    23: "Stings",
    24: "Unclassifiable",
    25: "World",
    26: "Urban",
    16: "Pop",
}
SKIP_GENRE = {9, 14, 15, 23, 20, 2, 12, 18, 21}
SKIP_TITLE = re.compile(
    r"christmas|jingle|polka|minstrel|celtic|fairies|goblin king|thatched|angevin|"
    r"gymnopedie|parting glass|fluffing|scheming weasel|happy bee|chipper|pinball|"
    r"cartoon|pizzicato|rainforest|village dawn|thatched villagers|boogie|lullaby|"
    r"birthday|circus|clown|ukulele|meet and greet|cretaceous|sauropod|dinosaur|"
    r"rap\b|children|kids song|whimsical|silly|goofy|banjo",
    re.I,
)

ICON = {
    "brutal": "storm",
    "rebellious": "swords",
    "melancholic": "moon",
    "relaxing": "fog",
}


def slug(title: str, used: set[str]) -> str:
    s = re.sub(r"[^a-z0-9]+", "_", title.lower()).strip("_")[:28] or "track"
    base = s
    n = 2
    while s in used:
        s = f"{base}_{n}"
        n += 1
    used.add(s)
    return s


def secs(p: dict) -> int:
    t = p.get("length") or "0"
    m = [int(x) for x in re.findall(r"\d+", t)]
    if len(m) >= 3:
        return m[0] * 3600 + m[1] * 60 + m[2]
    if len(m) == 2:
        return m[0] * 60 + m[1]
    return 0


def blob(p: dict) -> str:
    g = GENRES.get(int(p.get("genre") or 0), "")
    return f"{p.get('title','')} {p.get('feel','')} {p.get('description','')} {p.get('instruments','')} {g}".lower()


def classify(p: dict) -> tuple[str, float] | None:
    g = int(p.get("genre") or 0)
    if g in SKIP_GENRE:
        return None
    title = re.sub(r"\s+", " ", (p.get("title") or "")).strip()
    if SKIP_TITLE.search(title):
        return None
    if (p.get("filename") or "") in OWNED:
        return None
    if secs(p) < 55 or secs(p) > 480:
        return None
    b = blob(p)
    if any(k in b for k in ["ren faire", "medieval", "celtic", "tavern music"]):
        return None
    if any(k in b for k in ["humorous", "kids", "comedy", "bouncy, bright, humorous", "silly", "whimsical"]) and not any(
        k in b for k in ["dark", "aggressive", "intense", "sad", "electro", "rock"]
    ):
        return None
    scores = {"brutal": 0.0, "rebellious": 0.0, "melancholic": 0.0, "relaxing": 0.0}
    for k in ["aggressive", "intense", "industrial", "chaos", "heavy", "epic", "battle", "brutal", "feral", "danger", "storm"]:
        if k in b:
            scores["brutal"] += 2
    if any(k in b for k in ["edm", "rave", "dance monster", "techno", "hardcore", "drum and bass", "dnb"]):
        scores["brutal"] += 2.4
    for k in ["defiant", "rebell", "punk", "angry", "driving", "rawk", "riot", "anarch", "outlaw"]:
        if k in b:
            scores["rebellious"] += 2.2
    if "rock" in b:
        scores["rebellious"] += 1.4
    for k in ["sad", "somber", "melanch", "mourn", "tragic", "lonely", "haunting", "sorrow", "loss", "wounded", "noir"]:
        if k in b:
            scores["melancholic"] += 2.2
    if "dark" in b:
        scores["melancholic"] += 1.1
    for k in ["relax", "calm", "chill", "peaceful", "ambient", "mellow", "serene", "gentle", "meditat", "tranquil"]:
        if k in b:
            scores["relaxing"] += 2.2
    if g == 7:
        if any(k in b for k in ["groove", "edm", "dance", "energy", "rave", "techno"]):
            scores["brutal"] += 1.8
        else:
            scores["relaxing"] += 1.2
    if g == 19:
        scores["rebellious"] += 1.6
    if g == 10:
        scores["melancholic"] += 1.6
    if g == 26:
        scores["rebellious"] += 0.8
        scores["brutal"] += 0.6
    best = max(scores, key=scores.get)
    if scores[best] >= 1.6:
        return best, scores[best]
    return None


def url_of(filename: str) -> str:
    return BASE + urllib.parse.quote(filename)


def run(cmd: list[str]) -> None:
    subprocess.check_call(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def dur(path: Path) -> float:
    out = subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)],
        text=True,
    ).strip()
    try:
        return float(out)
    except ValueError:
        return 0.0


def download(url: str, dest: Path) -> bool:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 4000:
        return True
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=120) as r, open(dest, "wb") as f:
            while True:
                chunk = r.read(1024 * 256)
                if not chunk:
                    break
                f.write(chunk)
        return dest.stat().st_size > 2000
    except Exception as e:
        print("  fail", dest.name, e)
        dest.unlink(missing_ok=True)
        return False


def encode_ogg(src: Path, dest: Path, max_dur: float = 82.0) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 4000:
        return
    take = min(max_dur, max(dur(src) - 0.05, 1.0))
    fade = min(0.9, take / 8)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-t", f"{take:.3f}",
            "-af", f"loudnorm=I=-16:LRA=11:TP=-1.5,afade=t=in:d=0.08,afade=t=out:st={take - fade:.3f}:d={fade:.3f}",
            "-ar", "44100", "-ac", "2",
            "-c:a", "libvorbis", "-q:a", "4",
            str(dest),
        ]
    )


def write_js(rows: list[dict]) -> None:
    lines = [
        "const M = \"audio/blight/music/\";",
        "export const BLIGHT_RADIO = [",
    ]
    for r in rows:
        name = re.sub(r"\s+", " ", r["name"]).replace("\\", " ").replace('"', '\\"')
        lines.append(
            f'  {{ id: "{r["id"]}", name: "{name}", category: "music", world: "blight", '
            f'icon: "{r["icon"]}", mood: "{r["mood"]}", file: M + "{r["id"]}.ogg" }},'
        )
    lines.append("];")
    OUT_JS.write_text("\n".join(lines) + "\n")
    print("wrote", OUT_JS, "n", len(rows))


def main() -> int:
    if not PIECES.exists():
        req = urllib.request.Request(
            "https://incompetech.com/music/royalty-free/pieces.json",
            headers={"User-Agent": UA},
        )
        with urllib.request.urlopen(req, timeout=60) as r:
            PIECES.write_bytes(r.read())
    pieces = json.loads(PIECES.read_text())
    buckets = defaultdict(list)
    for p in pieces:
        hit = classify(p)
        if not hit:
            continue
        vibe, score = hit
        buckets[vibe].append((score, p))
    used = set()
    chosen = []
    for vibe, rows in buckets.items():
        rows.sort(key=lambda item: (-item[0], -min(secs(item[1]), 240), item[1].get("title") or ""))
        take = [p for _, p in rows[:PER_VIBE]]
        print(vibe, "available", len(rows), "take", len(take), flush=True)
        for p in take:
            ident = "br_" + slug(p.get("title") or "track", used)
            chosen.append(
                {
                    "id": ident,
                    "name": re.sub(r"\s+", " ", (p.get("title") or ident)).strip()[:42],
                    "mood": vibe,
                    "icon": ICON[vibe],
                    "filename": p["filename"],
                }
            )
    RAW.mkdir(parents=True, exist_ok=True)
    AUDIO.mkdir(parents=True, exist_ok=True)

    print("== download", len(chosen), "==", flush=True)
    raws = {}
    ok_rows = []
    with ThreadPoolExecutor(max_workers=8) as pool:
        futs = {}
        for row in chosen:
            dest = RAW / (row["id"] + Path(row["filename"]).suffix)
            futs[pool.submit(download, url_of(row["filename"]), dest)] = (row, dest)
        n = 0
        for fut in as_completed(futs):
            row, dest = futs[fut]
            n += 1
            if fut.result():
                raws[row["id"]] = dest
                ok_rows.append(row)
                if n % 20 == 0 or n == len(futs):
                    print(f"  got {n}/{len(futs)}", flush=True)
            else:
                print("  miss", row["filename"], flush=True)
    print("== convert", len(raws), "==", flush=True)
    converted = 0
    with ThreadPoolExecutor(max_workers=4) as pool:
        futs = {pool.submit(encode_ogg, src, AUDIO / (key + ".ogg")): (key, src) for key, src in raws.items()}
        for fut in as_completed(futs):
            key, src = futs[fut]
            converted += 1
            try:
                fut.result()
                if converted % 10 == 0 or converted == len(futs):
                    print(f"  ogg {converted}/{len(futs)}", flush=True)
                if src.parent == RAW and src.name.startswith("br_"):
                    src.unlink(missing_ok=True)
            except Exception as e:
                print("  conv fail", key, e, flush=True)
                ok_rows = [r for r in ok_rows if r["id"] != key]
    have = {p.stem for p in AUDIO.glob("*.ogg")}
    final = [r for r in ok_rows if r["id"] in have]
    write_js(final)
    from collections import Counter
    print("vibes", dict(Counter(r["mood"] for r in final)), flush=True)
    print("done", len(final), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
