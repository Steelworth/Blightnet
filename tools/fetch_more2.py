#!/usr/bin/env python3
"""Second expansion: more music, weather, animals, rooms."""

from __future__ import annotations

import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw"
AUDIO = ROOT / "audio"
UA = "Blightnet/3.2 (local table app)"

import urllib.request


def mix(i: int) -> str:
    return f"https://assets.mixkit.co/active_storage/sfx/{i}/{i}.wav"


MUSIC = {
    "lord_land": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Lord%20of%20the%20Land.mp3",
    "achaidh": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Achaidh%20Cheide.mp3",
    "call_adventure": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Call%20to%20Adventure.mp3",
    "black_vortex": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Black%20Vortex.mp3",
    "mystery": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Comfortable%20Mystery.mp3",
    "ossuary": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Ossuary%206%20-%20Air.mp3",
    "feral_chase": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Feral%20Chase.mp3",
    "at_rest": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/At%20Rest.mp3",
    "scarab": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Curse%20of%20the%20Scarab.mp3",
    "interloper": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Interloper.mp3",
    "pippin": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Pippin%20the%20Hunchback.mp3",
    "darkling": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Darkling.mp3",
    "moorland": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Moorland.mp3",
    "ritual_song": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Ritual.mp3",
    "blue_feather": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Blue%20Feather.mp3",
    "chamber": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/The%20Chamber.mp3",
}

LOOPS = {
    "weather/light_rain": mix(2474),
    "weather/forest_rain": mix(1225),
    "weather/metal_roof": mix(1265),
    "weather/night_rain": mix(1252),
    "weather/gale": mix(2407),
    "weather/breeze": mix(2427),
    "weather/howling": mix(2658),
    "weather/storm_wind": mix(2413),
    "weather/evil_storm": mix(2404),
    "weather/forest_storm": mix(2396),
    "weather/thunder_rain": mix(2390),
    "weather/heavy_rain": mix(1262),
    "animals/morning_birds": mix(2472),
    "animals/jungle_birds": mix(2434),
    "animals/river_birds": mix(2473),
    "animals/forest_birds": mix(1238),
    "animals/chickens": mix(1769),
    "animals/owl_woods": mix(2466),
    "animals/cicadas": mix(1788),
    "animals/gallop": mix(77),
    "animals/swamp_bugs": mix(39),
    "ambience/night_forest": mix(1224),
    "ambience/scary_woods": mix(2483),
    "ambience/tomb": mix(2500),
    "ambience/crowded_pub": mix(360),
    "ambience/street": mix(375),
    "ambience/fair": mix(368),
    "ambience/church_bells": mix(621),
    "ambience/clock": mix(1072),
    "ambience/big_fire": mix(1335),
    "ambience/desert_night": mix(2495),
    "ambience/harbor": mix(1206),
    "ambience/stormy_waves": mix(1199),
    "ambience/european_forest": mix(1213),
    "ambience/garden": mix(2464),
    "ambience/barn": mix(1755),
    "ambience/pond": mix(1783),
    "ambience/horror": mix(2482),
    "ambience/twilight_jungle": mix(2420),
    "ambience/rainforest": mix(1260),
    "ambience/campfire_wind": mix(1736),
    "ambience/leaves": mix(2430),
    "ambience/river_dawn": mix(2458),
    "ambience/waterfall_woods": mix(2517),
}

SHOTS = {
    "oneshots/goose_1": mix(20),
    "oneshots/goat_1": mix(1760),
    "oneshots/goat_2": mix(1771),
    "oneshots/dragon_1": mix(309),
    "oneshots/beast_1": mix(13),
    "oneshots/beast_2": mix(6),
    "oneshots/raven_3": mix(62),
    "oneshots/wolf_3": mix(2485),
    "oneshots/hawk_2": mix(1277),
    "oneshots/thunder_5": mix(1297),
    "oneshots/thunder_6": mix(2405),
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
        extend_loop(raw, dest, 18)
    else:
        loop_ogg(raw, dest, 26, 1.6, "loudnorm=I=-18:LRA=9:TP=-1.5")
    return f"ok {rel} {dest.stat().st_size // 1024}kb"


def main() -> None:
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
        futs = []
        for rel in LOOPS:
            raw = ok_raw.get(rel)
            if not raw:
                print("  miss", rel)
                continue
            futs.append(pool.submit(convert_loop, rel, raw))
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
        print(" ", rel)
        oneshot_ogg(raw, dest, 8)
        print("   ", dest.stat().st_size // 1024, "kb")

    print("done")


if __name__ == "__main__":
    main()
