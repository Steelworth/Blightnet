#!/usr/bin/env python3
"""Download real CC0/CC-BY audio and convert to looping OGG."""

from __future__ import annotations

import json
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw"
AUDIO = ROOT / "audio"
UA = "BlightnetAudioBot/1.0 (local table app)"

# Mixkit WAV (royalty-free, no attribution required)
# Incompetech / Kevin MacLeod (CC-BY 4.0) — credit in the app
# OpenGameArt CC0

MUSIC = {
    "tavern_jig": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Minstrel%20Guild.mp3",
    "forest_wander": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Magic%20Forest.mp3",
    "dungeon_depths": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Night%20Cave.mp3",
    "battle_march": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Five%20Armies.mp3",
    "sacred_hymn": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Angevin.mp3",
    "ocean_voyage": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Skye%20Cuillin.mp3",
    "dark_rite": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Rites.mp3",
    "royal_court": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Teller%20of%20the%20Tales.mp3",
    "road_theme": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Folk%20Round.mp3",
    "elven_glade": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Enchanted%20Valley.mp3",
    "winter_march": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/The%20Ice%20Giants.mp3",
    "dragon_wake": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Dragon%20and%20Toast.mp3",
}

def mix(id_: int) -> str:
    return f"https://assets.mixkit.co/active_storage/sfx/{id_}/{id_}.wav"

LOOPS = {
    "weather/rain": mix(2394),
    "weather/downpour": mix(1253),
    "weather/wind": mix(1474),
    "weather/blizzard": mix(1172),
    "weather/distant_storm": mix(2415),
    "animals/songbirds": mix(1232),
    "animals/horses": mix(1762),
    "animals/crickets": mix(1227),
    "animals/frogs": mix(40),
    "animals/bats": mix(1230),
    "ambience/campfire": mix(1330),
    "ambience/tavern_hall": mix(460),
    "ambience/market": mix(444),
    "ambience/ocean": mix(1194),
    "ambience/river": mix(2456),
    "ambience/waterfall": mix(2518),
    "ambience/swamp": mix(1789),
    "ambience/graveyard": mix(1267),
    "ambience/ship": mix(1183),
    "ambience/war_camp": mix(3022),
    "ambience/forge": mix(1348),
    "ambience/torch": mix(1346),
}

OGA = {
    "ambience/dungeon": "https://opengameart.org/sites/default/files/dungeon_ambient_1_0.ogg",
    "ambience/cavern": "https://opengameart.org/sites/default/files/caverns_0.ogg",
}

SHOTS = {
    "oneshots/thunder_1": mix(1704),
    "oneshots/thunder_2": mix(1287),
    "oneshots/thunder_3": mix(2402),
    "oneshots/wolf_1": mix(1729),
    "oneshots/wolf_2": mix(1776),
    "oneshots/owl_1": mix(2479),
    "oneshots/gull_1": mix(1208),
}

EXTRA_TRY = {
    "oneshots/owl_2": mix(2479),
    "oneshots/raven_1": "https://upload.wikimedia.org/wikipedia/commons/4/45/Corvus_corax_2.ogg",
    "oneshots/raven_2": "https://upload.wikimedia.org/wikipedia/commons/9/9e/Corvus_corax.ogg",
    "oneshots/gull_2": mix(1208),
}


def run(cmd: list[str]) -> None:
    subprocess.check_call(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def probe_dur(path: Path) -> float:
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
        with urllib.request.urlopen(req, timeout=90) as r, open(dest, "wb") as f:
            while True:
                chunk = r.read(1024 * 256)
                if not chunk:
                    break
                f.write(chunk)
        return dest.stat().st_size > 4000
    except Exception as e:
        print(f"  fail {url} ({e})")
        dest.unlink(missing_ok=True)
        return False


def loop_ogg(src: Path, dest: Path, max_dur: float, xf: float, loud: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_suffix(".tmp.wav")
    take = max_dur
    dur = probe_dur(src)
    if dur <= 0:
        raise RuntimeError(f"no duration {src}")
    take = min(take, max(dur - 0.05, 1.0))
    xf = min(xf, take / 3)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", f"{loud},atrim=0:{take:.3f},asetpts=PTS-STARTPTS",
            "-ar", "44100", "-ac", "2", str(wav),
        ]
    )
    d = probe_dur(wav)
    if d > xf * 2 + 0.4:
        head = d - xf
        looped = dest.with_suffix(".loop.wav")
        run(
            [
                "ffmpeg", "-y", "-i", str(wav),
                "-filter_complex",
                f"[0]atrim=0:{head:.3f},asetpts=PTS-STARTPTS[h];"
                f"[0]atrim={head:.3f},asetpts=PTS-STARTPTS[t];"
                f"[t][h]acrossfade=d={xf:.3f}:c1=tri:c2=tri",
                str(looped),
            ]
        )
        wav.unlink(missing_ok=True)
        wav = looped
    run(
        ["ffmpeg", "-y", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "6", str(dest)]
    )
    wav.unlink(missing_ok=True)


def oneshot_ogg(src: Path, dest: Path, max_dur: float = 8.0) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    dur = probe_dur(src)
    take = min(dur, max_dur) if dur else max_dur
    fade = min(0.35, take / 6)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", f"loudnorm=I=-14:LRA=8:TP=-1.2,atrim=0:{take:.3f},afade=t=in:d=0.02,afade=t=out:st={take-fade:.3f}:d={fade:.3f}",
            "-ar", "44100", "-ac", "1", "-c:a", "libvorbis", "-q:a", "6",
            str(dest),
        ]
    )


def lava_from_fire(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_suffix(".tmp.wav")
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", "asetrate=44100*0.62,aresample=44100,lowpass=f=700,bass=g=10:f=60,loudnorm=I=-18:TP=-1.5,atrim=0:28",
            str(wav),
        ]
    )
    loop_ogg(wav, dest, 28, 2.0, "loudnorm=I=-18:LRA=8:TP=-1.5")
    wav.unlink(missing_ok=True)


def desert_from_wind(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_suffix(".tmp.wav")
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", "highpass=f=180,treble=g=4,loudnorm=I=-18:TP=-1.5,atrim=0:24",
            str(wav),
        ]
    )
    loop_ogg(wav, dest, 24, 1.6, "loudnorm=I=-18:LRA=8:TP=-1.5")
    wav.unlink(missing_ok=True)


def magic_from_cave(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_suffix(".tmp.wav")
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", "asetrate=44100*0.85,aresample=44100,lowpass=f=1200,chorus=0.5:0.8:40:0.3:0.25:1.5,loudnorm=I=-20:TP=-1.5,atrim=0:32",
            str(wav),
        ]
    )
    loop_ogg(wav, dest, 32, 2.4, "loudnorm=I=-20:LRA=7:TP=-1.5")
    wav.unlink(missing_ok=True)


def temple_from_hymn(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_suffix(".tmp.wav")
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", "lowpass=f=900,volume=0.45,aecho=0.8:0.88:120:0.3,loudnorm=I=-22:TP=-1.8,atrim=0:36",
            str(wav),
        ]
    )
    loop_ogg(wav, dest, 36, 2.8, "loudnorm=I=-22:LRA=7:TP=-1.8")
    wav.unlink(missing_ok=True)


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)
    jobs = []

    print("== music ==")
    for name, url in MUSIC.items():
        raw = RAW / "music" / f"{name}.mp3"
        print(f"  {name}")
        if download(url, raw):
            jobs.append(("music", name, raw, "music"))

    print("== loops ==")
    for rel, url in {**LOOPS, **OGA}.items():
        folder, name = rel.split("/")
        ext = Path(url).suffix or ".wav"
        raw = RAW / folder / f"{name}{ext}"
        print(f"  {rel}")
        if download(url, raw):
            jobs.append((folder, name, raw, "loop"))

    print("== shots ==")
    for rel, url in {**SHOTS, **EXTRA_TRY}.items():
        folder, name = rel.split("/")
        ext = Path(url).suffix or ".wav"
        raw = RAW / folder / f"{name}{ext}"
        print(f"  {rel}")
        if download(url, raw):
            jobs.append((folder, name, raw, "shot"))

    print("== convert ==")
    music_loud = "loudnorm=I=-16:LRA=11:TP=-1.5"
    loop_loud = "loudnorm=I=-18:LRA=9:TP=-1.5"
    for folder, name, raw, kind in jobs:
        dest = AUDIO / folder / f"{name}.ogg"
        print(f"  ogg {folder}/{name}")
        try:
            if kind == "music":
                loop_ogg(raw, dest, 78, 3.0, music_loud)
            elif kind == "loop":
                maxd = 36 if "waterfall" in name or "river" in name or "market" in name or "ocean" in name else 28
                loop_ogg(raw, dest, maxd, 1.8, loop_loud)
            else:
                maxd = 12 if "thunder" in name or "wolf" in name else 6
                oneshot_ogg(raw, dest, maxd)
            print(f"     {dest.stat().st_size // 1024} kb")
        except subprocess.CalledProcessError:
            print("     convert failed")

    # derived textures from real recordings
    fire = RAW / "ambience" / "campfire.wav"
    if not fire.exists():
        fire = RAW / "ambience" / "torch.wav"
    wind = RAW / "weather" / "wind.wav"
    cave = RAW / "ambience" / "cavern.ogg"
    hymn = RAW / "music" / "sacred_hymn.mp3"
    if fire.exists():
        print("  derive lava")
        lava_from_fire(fire, AUDIO / "ambience" / "lava.ogg")
    if wind.exists():
        print("  derive desert")
        desert_from_wind(wind, AUDIO / "ambience" / "desert.ogg")
    if cave.exists():
        print("  derive magic")
        magic_from_cave(cave, AUDIO / "ambience" / "magic.ogg")
    if hymn.exists():
        print("  derive temple")
        temple_from_hymn(hymn, AUDIO / "ambience" / "temple.ogg")

    credits = {
        "music": "Music by Kevin MacLeod (incompetech.com), licensed CC BY 4.0",
        "sfx": "Ambience and creatures from Mixkit (mixkit.co) and OpenGameArt CC0",
    }
    (ROOT / "assets" / "credits.json").write_text(json.dumps(credits, indent=2))
    print("done")
    return 0


if __name__ == "__main__":
    sys.exit(main())
