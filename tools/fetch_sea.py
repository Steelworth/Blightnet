#!/usr/bin/env python3
"""Beach and underwater music and beds."""

from __future__ import annotations

import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw"
AUDIO = ROOT / "audio"
UA = "Blightnet/3.3 (local table app)"

import urllib.request


def mix(i: int) -> str:
    return f"https://assets.mixkit.co/active_storage/sfx/{i}/{i}.wav"


MUSIC = {
    "water_prelude": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Water%20Prelude.mp3",
    "floating_cities": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Floating%20Cities.mp3",
    "with_the_sea": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/With%20the%20Sea.mp3",
    "shores_avalon": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Shores%20of%20Avalon.mp3",
}

LOOPS = {
    "ambience/beach": mix(1189),
    "ambience/shore": mix(1195),
    "ambience/sea_wind": mix(1200),
    "ambience/shore_birds": mix(1185),
    "ambience/underwater": mix(1204),
    "ambience/diving": mix(1179),
    "ambience/deep_hum": mix(2135),
    "ambience/sinking": mix(1178),
}

SHOTS = {
    "oneshots/bubble_1": mix(1321),
    "oneshots/bubble_2": mix(1317),
    "oneshots/bubble_3": mix(3017),
    "oneshots/splash_1": mix(1198),
    "oneshots/splash_2": mix(1180),
}


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
        with urllib.request.urlopen(req, timeout=90) as r, open(dest, "wb") as f:
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


def loop_ogg(src: Path, dest: Path, max_dur: float, xf: float, loud: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_name(dest.stem + ".prep.wav")
    take = min(max_dur, max(dur(src) - 0.05, 1.0))
    xf = min(xf, take / 3)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", f"{loud},atrim=0:{take:.3f},asetpts=PTS-STARTPTS",
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
    run(["ffmpeg", "-y", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "6", str(dest)])
    wav.unlink(missing_ok=True)
    looped.unlink(missing_ok=True)


def oneshot_ogg(src: Path, dest: Path, max_dur: float) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    d = dur(src) or max_dur
    take = min(d, max_dur)
    fade = min(0.35, take / 5)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af",
            f"loudnorm=I=-14:LRA=8:TP=-1.2,atrim=0:{take:.3f},afade=t=in:d=0.02,afade=t=out:st={max(0,take-fade):.3f}:d={fade:.3f}",
            "-ar", "44100", "-ac", "1", "-c:a", "libvorbis", "-q:a", "6",
            str(dest),
        ]
    )


def extend_loop(src: Path, dest: Path, seconds: float) -> None:
    tmp = dest.with_name(dest.stem + ".ext.wav")
    run(
        [
            "ffmpeg", "-y", "-stream_loop", "10", "-i", str(src), "-t", str(seconds),
            "-af", "loudnorm=I=-18:TP=-1.5", str(tmp),
        ]
    )
    loop_ogg(tmp, dest, seconds, 1.1, "loudnorm=I=-18:LRA=8:TP=-1.5")
    tmp.unlink(missing_ok=True)


def convert_loop(rel: str, raw: Path) -> str:
    folder, name = rel.split("/")
    dest = AUDIO / folder / f"{name}.ogg"
    if dest.exists() and dest.stat().st_size > 4000:
        return f"skip {rel}"
    if dur(raw) < 6:
        extend_loop(raw, dest, 22)
    else:
        loop_ogg(raw, dest, 28, 1.6, "loudnorm=I=-18:LRA=9:TP=-1.5")
    return f"ok {rel} {dest.stat().st_size // 1024}kb"


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)

    print("== download ==")
    jobs = []
    for name, url in MUSIC.items():
        jobs.append((url, RAW / "music" / f"{name}.mp3", name))
    for rel, url in LOOPS.items():
        folder, name = rel.split("/")
        jobs.append((url, RAW / folder / f"{name}.wav", rel))
    for rel, url in SHOTS.items():
        folder, name = rel.split("/")
        jobs.append((url, RAW / folder / f"{name}.wav", rel))

    ok_raw = {}
    with ThreadPoolExecutor(max_workers=6) as pool:
        futs = {pool.submit(download, url, dest): (key, dest) for url, dest, key in jobs}
        for fut in as_completed(futs):
            key, dest = futs[fut]
            good = fut.result()
            print(" ", "got" if good else "miss", key)
            if good:
                ok_raw[key] = dest

    print("== music ==")
    for name in MUSIC:
        raw = ok_raw.get(name)
        dest = AUDIO / "music" / f"{name}.ogg"
        if dest.exists() and dest.stat().st_size > 4000:
            print("  skip", name)
            continue
        if not raw:
            print("  miss", name)
            continue
        print(" ", name)
        loop_ogg(raw, dest, 70, 2.8, "loudnorm=I=-16:LRA=11:TP=-1.5")
        print("   ", dest.stat().st_size // 1024, "kb")

    print("== loops ==")
    with ThreadPoolExecutor(max_workers=3) as pool:
        futs = [pool.submit(convert_loop, rel, ok_raw[rel]) for rel in LOOPS if rel in ok_raw]
        for fut in as_completed(futs):
            print(" ", fut.result())

    print("== shots ==")
    for rel in SHOTS:
        raw = ok_raw.get(rel)
        folder, name = rel.split("/")
        dest = AUDIO / folder / f"{name}.ogg"
        if dest.exists() and dest.stat().st_size > 2000:
            print("  skip", rel)
            continue
        if not raw:
            print("  miss", rel)
            continue
        oneshot_ogg(raw, dest, 6)
        print("  ok", rel, dest.stat().st_size // 1024, "kb")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
