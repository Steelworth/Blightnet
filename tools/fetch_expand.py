#!/usr/bin/env python3
"""Expand Blightnet music, weather, animals, and rooms."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw"
AUDIO = ROOT / "audio"
UA = "Blightnet/3.0 (local table app)"

import urllib.request


def mix(i: int) -> str:
    return f"https://assets.mixkit.co/active_storage/sfx/{i}/{i}.wav"


MUSIC = {
    "village_fair": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Thatched%20Villagers.mp3",
    "the_descent": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/The%20Descent.mp3",
    "the_pyre": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/The%20Pyre.mp3",
    "clash_defiant": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Clash%20Defiant.mp3",
    "morgana": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Morgana%20Rides.mp3",
    "lost_time": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Lost%20Time.mp3",
    "crusade": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Crusade.mp3",
    "darkest_child": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Darkest%20Child.mp3",
    "ghost_story": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Ghost%20Story.mp3",
    "maestro": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/One-eyed%20Maestro.mp3",
}

LOOPS = {
    "weather/jungle_rain": mix(2415),
    "weather/canopy_wind": mix(1177),
    "weather/mountain_air": mix(2505),
    "animals/night_woods": mix(1230),
    "animals/jungle": mix(2414),
    "animals/purring": mix(86),
    "ambience/night_woods": mix(1230),
    "ambience/jungle": mix(2414),
    "ambience/town_square": mix(444),
    "ambience/surf": mix(1196),
    "ambience/cascade": mix(2516),
    "ambience/fireplace": mix(1348),
}

SHOTS = {
    "oneshots/donkey_1": mix(1741),
    "oneshots/neigh_1": mix(76),
    "oneshots/neigh_2": mix(83),
    "oneshots/growl_1": mix(51),
    "oneshots/thunder_4": mix(2402),
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


def main() -> None:
    RAW.mkdir(parents=True, exist_ok=True)
    print("== music ==")
    for name, url in MUSIC.items():
        raw = RAW / "music" / f"{name}.mp3"
        print(" ", name)
        if download(url, raw):
            loop_ogg(raw, AUDIO / "music" / f"{name}.ogg", 70, 2.8, "loudnorm=I=-16:LRA=11:TP=-1.5")
            print("   ", (AUDIO / "music" / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== loops ==")
    for rel, url in LOOPS.items():
        folder, name = rel.split("/")
        raw = RAW / folder / f"{name}.wav"
        print(" ", rel)
        if download(url, raw):
            maxd = 26
            if dur(raw) < 6:
                extend_loop(raw, AUDIO / folder / f"{name}.ogg", 18)
            else:
                loop_ogg(raw, AUDIO / folder / f"{name}.ogg", maxd, 1.6, "loudnorm=I=-18:LRA=9:TP=-1.5")
            print("   ", (AUDIO / folder / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== shots ==")
    for rel, url in SHOTS.items():
        folder, name = rel.split("/")
        raw = RAW / folder / f"{name}.wav"
        print(" ", rel)
        if download(url, raw):
            oneshot_ogg(raw, AUDIO / folder / f"{name}.ogg", 6)
            print("   ", (AUDIO / folder / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== derived rooms ==")
    dungeon = AUDIO / "ambience" / "dungeon.ogg"
    river = AUDIO / "ambience" / "river.ogg"
    tavern = AUDIO / "ambience" / "tavern_hall.ogg"
    temple = AUDIO / "ambience" / "temple.ogg"
    if dungeon.exists() and river.exists():
        tmp = RAW / "ambience" / "sewers_mix.wav"
        run(
            [
                "ffmpeg", "-y", "-i", str(dungeon), "-i", str(river),
                "-filter_complex",
                "[0]atrim=0:22,volume=0.7[a];[1]atrim=0:22,lowpass=f=900,volume=0.55[b];[a][b]amix=inputs=2:duration=first,loudnorm=I=-18:TP=-1.5",
                str(tmp),
            ]
        )
        loop_ogg(tmp, AUDIO / "ambience" / "sewers.ogg", 22, 1.8, "loudnorm=I=-18:LRA=8:TP=-1.5")
        tmp.unlink(missing_ok=True)
        print("  sewers")
    if tavern.exists():
        tmp = RAW / "ambience" / "kitchen_prep.wav"
        run(
            [
                "ffmpeg", "-y", "-i", str(tavern),
                "-af", "highpass=f=220,treble=g=3,loudnorm=I=-18:TP=-1.5,atrim=0:20",
                str(tmp),
            ]
        )
        loop_ogg(tmp, AUDIO / "ambience" / "kitchen.ogg", 20, 1.4, "loudnorm=I=-18:LRA=8:TP=-1.5")
        tmp.unlink(missing_ok=True)
        print("  kitchen")
    if temple.exists():
        tmp = RAW / "ambience" / "library_prep.wav"
        run(
            [
                "ffmpeg", "-y", "-i", str(temple),
                "-af", "lowpass=f=1600,volume=0.7,loudnorm=I=-22:TP=-1.8,atrim=0:24",
                str(tmp),
            ]
        )
        loop_ogg(tmp, AUDIO / "ambience" / "library.ogg", 24, 2.0, "loudnorm=I=-22:LRA=7:TP=-1.8")
        tmp.unlink(missing_ok=True)
        print("  library")
    if dungeon.exists():
        tmp = RAW / "ambience" / "drips_prep.wav"
        run(
            [
                "ffmpeg", "-y", "-i", str(dungeon),
                "-af", "highpass=f=600,treble=g=5,loudnorm=I=-18:TP=-1.5,atrim=0:20",
                str(tmp),
            ]
        )
        loop_ogg(tmp, AUDIO / "ambience" / "drips.ogg", 20, 1.5, "loudnorm=I=-18:LRA=8:TP=-1.5")
        tmp.unlink(missing_ok=True)
        print("  drips")

    print("done")


if __name__ == "__main__":
    main()
