#!/usr/bin/env python3
"""Fetch cyberpunk-table music (Kevin MacLeod CC BY) into audio/blight/music."""

from __future__ import annotations

import subprocess
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw" / "blight_music"
AUDIO = ROOT / "audio" / "blight" / "music"
UA = "Blightnet/3.5 (local table app)"
BASE = "https://incompetech.com/music/royalty-free/mp3-royaltyfree/"

# id -> (filename on incompetech, display, mood, icon)
TRACKS = {
    "newer_wave": ("Newer Wave.mp3", "Newer Wave", "synthwave", "spark"),
    "neon_laser": ("Neon Laser Horizon.mp3", "Neon Laser Horizon", "synthwave", "bolt"),
    "cyborg_ninja": ("Cyborg Ninja.mp3", "Cyborg Ninja", "synthwave", "spark"),
    "space_fighter": ("Space Fighter Loop.mp3", "Space Fighter", "synthwave", "bolt"),
    "reformat": ("Reformat.mp3", "Reformat", "synthwave", "spark"),
    "ethernight": ("Ethernight Club.mp3", "Ethernight Club", "synthwave", "moon"),
    "equatorial": ("Equatorial Complex.mp3", "Equatorial Complex", "synthwave", "spark"),
    "cloud_dancer": ("Cloud Dancer.mp3", "Cloud Dancer", "synthwave", "bolt"),
    "laser_groove": ("Laser Groove.mp3", "Laser Groove", "synthwave", "bolt"),
    "digital_lemonade": ("Digital Lemonade.mp3", "Digital Lemonade", "synthwave", "spark"),
    "edm_detect": ("EDM Detection Mode.mp3", "EDM Detection", "techno", "bolt"),
    "laserpack": ("Laserpack.mp3", "Laserpack", "techno", "bolt"),
    "raving_energy": ("Raving Energy.mp3", "Raving Energy", "techno", "storm"),
    "dance_monster": ("Dance Monster.mp3", "Dance Monster", "techno", "bolt"),
    "cut_trance": ("Cut Trance.mp3", "Cut Trance", "techno", "spark"),
    "voltaic": ("Voltaic.mp3", "Voltaic", "techno", "bolt"),
    "club_diver": ("Club Diver.mp3", "Club Diver", "techno", "moon"),
    "kick_shock": ("Kick Shock.mp3", "Kick Shock", "techno", "storm"),
    "shiny_tech": ("Shiny Tech.mp3", "Shiny Tech", "techno", "spark"),
    "ultra_storm": ("Mega Hyper Ultrastorm.mp3", "Ultra Storm", "techno", "storm"),
    "chill_wave": ("Chill Wave.mp3", "Chill Wave", "lofi", "moon"),
    "mellowtron": ("Mellowtron.mp3", "Mellowtron", "lofi", "fog"),
    "beauty_flow": ("Beauty Flow.mp3", "Beauty Flow", "lofi", "wave"),
    "dream_catcher": ("Dream Catcher.mp3", "Dream Catcher", "lofi", "moon"),
    "hypnothis": ("Hypnothis.mp3", "Hypnothis", "lofi", "fog"),
    "ambler": ("Ambler.mp3", "Ambler", "lofi", "path"),
    "chillin_hard": ("Chillin Hard.mp3", "Chillin Hard", "lofi", "moon"),
    "deliberate": ("Deliberate Thought.mp3", "Deliberate Thought", "lofi", "book"),
    "tranquility": ("Tranquility.mp3", "Tranquility", "lofi", "fog"),
    "rising_tide": ("Rising Tide.mp3", "Rising Tide", "lofi", "wave"),
    "metalmania": ("Metalmania.mp3", "Metalmania", "punk", "storm"),
    "ready_aim": ("Ready Aim Fire.mp3", "Ready Aim Fire", "punk", "swords"),
    "twisted": ("Twisted.mp3", "Twisted", "punk", "storm"),
    "broken_reality": ("Broken Reality.mp3", "Broken Reality", "punk", "mask"),
    "severe_tire": ("Severe Tire Damage.mp3", "Severe Tire Damage", "punk", "storm"),
    "exit_premises": ("Exit the Premises.mp3", "Exit the Premises", "punk", "sneak"),
    "furious_freak": ("Furious Freak.mp3", "Furious Freak", "punk", "bolt"),
    "danger_storm": ("Danger Storm.mp3", "Danger Storm", "punk", "storm"),
    "exhilarate": ("Exhilarate.mp3", "Exhilarate", "rock", "banner"),
    "motherlode": ("Motherlode.mp3", "Motherlode", "rock", "anvil"),
    "hotrock": ("Hotrock.mp3", "Hotrock", "rock", "fire"),
    "big_rock": ("Big Rock.mp3", "Big Rock", "rock", "anvil"),
    "cool_rock": ("Cool Rock.mp3", "Cool Rock", "rock", "fire"),
    "too_cool": ("Too Cool.mp3", "Too Cool", "rock", "moon"),
    "retro_nasty": ("RetroFuture Nasty.mp3", "RetroFuture Nasty", "rock", "spark"),
    "voxel_rev": ("Voxel Revolution.mp3", "Voxel Revolution", "techno", "spark"),
    "brain_dance": ("Brain Dance.mp3", "Brain Dance", "techno", "bolt"),
    "tyrant": ("Tyrant.mp3", "Tyrant", "punk", "mask"),
    "rave_faster": ("Raving Energy (faster).mp3", "Raving Energy Faster", "rave", "bolt"),
    "go_cart": ("Go Cart.mp3", "Go Cart", "rave", "bolt"),
    "show_moves": ("Show Your Moves.mp3", "Show Your Moves", "rave", "spark"),
    "phat_sketch": ("Phat Sketch.mp3", "Phat Sketch", "rave", "bolt"),
    "future_cha": ("Future Cha Cha.mp3", "Future Cha Cha", "rave", "spark"),
    "harmful": ("Harmful or Fatal.mp3", "Harmful or Fatal", "rave", "storm"),
    "deep_dirty": ("Deep and Dirty.mp3", "Deep and Dirty", "rave", "moon"),
    "limit_70": ("Limit 70.mp3", "Limit 70", "rave", "bolt"),
    "blippy": ("Blippy Trance.mp3", "Blippy Trance", "rave", "spark"),
    "bit_shift": ("Bit Shift.mp3", "Bit Shift", "techno", "spark"),
    "cipher_net": ("Cipher2.mp3", "Cipher", "techno", "sneak"),
    "getting_done": ("Getting it Done.mp3", "Getting it Done", "rave", "bolt"),
    "space_x": ("space explorers.mp3", "Space X-plorers", "synthwave", "spark"),
}


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


def head_ok(url: str) -> bool:
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return 200 <= r.status < 400
    except Exception:
        return False


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


def loop_ogg(src: Path, dest: Path, max_dur: float = 82.0, xf: float = 4.0) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_name(dest.stem + ".prep.wav")
    take = min(max_dur, max(dur(src) - 0.05, 1.0))
    xf = min(xf, take / 3)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", f"loudnorm=I=-16:LRA=11:TP=-1.5,atrim=0:{take:.3f},asetpts=PTS-STARTPTS",
            "-ar", "44100", "-ac", "2", str(wav),
        ]
    )
    d = dur(wav)
    looped = dest.with_name(dest.stem + ".loop.wav")
    if d > xf * 2 + 0.4:
        head = d - xf
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
        wav.unlink()
        wav = looped
    run(["ffmpeg", "-y", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "5", str(dest)])
    wav.unlink(missing_ok=True)
    looped.unlink(missing_ok=True)


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)
    AUDIO.mkdir(parents=True, exist_ok=True)
    print(f"== probe {len(TRACKS)} ==")
    ok = {}
    with ThreadPoolExecutor(max_workers=12) as pool:
        futs = {pool.submit(head_ok, url_of(v[0])): k for k, v in TRACKS.items()}
        for fut in as_completed(futs):
            key = futs[fut]
            fn = TRACKS[key][0]
            if fut.result():
                ok[key] = TRACKS[key]
                print("  OK", fn)
            else:
                print("  miss", fn)
    print(f"OK {len(ok)} / miss {len(TRACKS) - len(ok)}")
    print("== download ==")
    raws = {}
    with ThreadPoolExecutor(max_workers=6) as pool:
        futs = {}
        for key, (fn, *_) in ok.items():
            dest = RAW / (key + Path(fn).suffix)
            futs[pool.submit(download, url_of(fn), dest)] = (key, dest)
        for fut in as_completed(futs):
            key, dest = futs[fut]
            if fut.result():
                raws[key] = dest
                print("  got", key)
            else:
                print("  skip", key)
    print("== convert ==")
    with ThreadPoolExecutor(max_workers=4) as pool:
        futs = {}
        for key, src in raws.items():
            dest = AUDIO / (key + ".ogg")
            if dest.exists() and dest.stat().st_size > 4000:
                print("  have", dest.name)
                continue
            futs[pool.submit(loop_ogg, src, dest)] = key
        for fut in as_completed(futs):
            key = futs[fut]
            try:
                fut.result()
                print("  ogg", key)
            except Exception as e:
                print("  conv fail", key, e)
    print("done", sum(1 for p in AUDIO.glob("*.ogg")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
