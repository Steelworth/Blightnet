#!/usr/bin/env python3
"""Add more music, weather, and animal beds."""

from __future__ import annotations

import json
import subprocess
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw"
AUDIO = ROOT / "audio"
UA = "Blightnet/2.0 (local table app)"


def mix(i: int) -> str:
    return f"https://assets.mixkit.co/active_storage/sfx/{i}/{i}.wav"


MUSIC = {
    "harvest_dance": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Celtic%20Impulse.mp3",
    "quiet_work": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Sneaky%20Adventure.mp3",
    "wizards_tower": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Wizardtorium.mp3",
    "oppressive_gloom": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Oppressive%20Gloom.mp3",
    "stormfront": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Stormfront.mp3",
    "hidden_past": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Hidden%20Past.mp3",
    "heroic_age": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/Heroic%20Age.mp3",
    "goblin_road": "https://incompetech.com/music/royalty-free/mp3-royaltyfree/The%20Path%20of%20the%20Goblin%20King%20v2.mp3",
}

LOOPS = {
    "weather/drizzle": mix(1294),
    "weather/roof_rain": mix(2399),
    "weather/rumble": mix(2395),
    "weather/winter_wind": mix(539),
    "animals/bees": mix(1926),
    "animals/flies": mix(326),
    "animals/farmyard": mix(1763),
}

SHOTS = {
    "oneshots/cat_1": mix(92),
    "oneshots/cat_2": mix(45),
    "oneshots/cat_3": mix(95),
    "oneshots/dog_1": mix(60),
    "oneshots/dog_2": mix(741),
    "oneshots/dog_3": mix(59),
    "oneshots/rooster_1": mix(2470),
    "oneshots/rooster_2": mix(1756),
    "oneshots/eagle_1": mix(72),
    "oneshots/cow_1": mix(1748),
    "oneshots/cow_2": mix(1744),
}

COMMONS = {
    "oneshots/fox_1": "File:Red Fox (Vulpes vulpes) (W1CDR0001529 BD12).ogg",
    "oneshots/hawk_1": "File:Buteo buteo warning the fledglings 7643.ogg",
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
        print("  fail", url, e)
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


def commons_url(title: str) -> str | None:
    q = urllib.parse.urlencode(
        {"action": "query", "titles": title, "prop": "imageinfo", "iiprop": "url", "format": "json"}
    )
    req = urllib.request.Request("https://commons.wikimedia.org/w/api.php?" + q, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=25) as r:
        data = json.loads(r.read().decode())
    for v in data["query"]["pages"].values():
        ii = (v.get("imageinfo") or [None])[0]
        if ii:
            return ii["url"].split("?")[0]
    return None


def main() -> None:
    RAW.mkdir(parents=True, exist_ok=True)
    print("== music ==")
    for name, url in MUSIC.items():
        raw = RAW / "music" / f"{name}.mp3"
        print(" ", name)
        if download(url, raw):
            loop_ogg(raw, AUDIO / "music" / f"{name}.ogg", 72, 2.8, "loudnorm=I=-16:LRA=11:TP=-1.5")
            print("   ", (AUDIO / "music" / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== loops ==")
    for rel, url in LOOPS.items():
        folder, name = rel.split("/")
        raw = RAW / folder / f"{name}.wav"
        print(" ", rel)
        if download(url, raw):
            maxd = 28 if "rumble" in name or "drizzle" in name else 22
            loop_ogg(raw, AUDIO / folder / f"{name}.ogg", maxd, 1.6, "loudnorm=I=-18:LRA=9:TP=-1.5")
            print("   ", (AUDIO / folder / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== shots ==")
    for rel, url in SHOTS.items():
        folder, name = rel.split("/")
        raw = RAW / folder / f"{name}.wav"
        print(" ", rel)
        if download(url, raw):
            oneshot_ogg(raw, AUDIO / folder / f"{name}.ogg", 5 if "cat" in name else 7)
            print("   ", (AUDIO / folder / f"{name}.ogg").stat().st_size // 1024, "kb")

    print("== commons ==")
    for rel, title in COMMONS.items():
        folder, name = rel.split("/")
        try:
            url = commons_url(title)
            print(" ", rel, url)
            if not url:
                continue
            raw = RAW / folder / f"{name}.src"
            if download(url, raw):
                oneshot_ogg(raw, AUDIO / folder / f"{name}.ogg", 8)
                print("   ", (AUDIO / folder / f"{name}.ogg").stat().st_size // 1024, "kb")
        except Exception as e:
            print("  skip", rel, e)

    # derived weather
    wind = RAW / "weather" / "wind_long.wav"
    if not wind.exists():
        wind = RAW / "weather" / "winter_wind.wav"
    if wind.exists():
        print("== derive fog / hail ==")
        tmp = RAW / "weather" / "fog_prep.wav"
        run(
            [
                "ffmpeg", "-y", "-i", str(wind),
                "-af", "lowpass=f=420,volume=0.85,loudnorm=I=-20:TP=-1.6,atrim=0:26",
                str(tmp),
            ]
        )
        loop_ogg(tmp, AUDIO / "weather" / "fog.ogg", 26, 2.0, "loudnorm=I=-20:LRA=8:TP=-1.6")
        tmp.unlink(missing_ok=True)
        hail_src = RAW / "weather" / "roof_rain.wav"
        if hail_src.exists():
            tmp = RAW / "weather" / "hail_prep.wav"
            run(
                [
                    "ffmpeg", "-y", "-i", str(hail_src),
                    "-af", "asetrate=44100*1.35,aresample=44100,highpass=f=900,treble=g=6,loudnorm=I=-16:TP=-1.3,atrim=0:18",
                    str(tmp),
                ]
            )
            loop_ogg(tmp, AUDIO / "weather" / "hail.ogg", 18, 1.2, "loudnorm=I=-16:LRA=8:TP=-1.3")
            tmp.unlink(missing_ok=True)
        print("  fog", (AUDIO / "weather" / "fog.ogg").stat().st_size // 1024, "kb")
        if (AUDIO / "weather" / "hail.ogg").exists():
            print("  hail", (AUDIO / "weather" / "hail.ogg").stat().st_size // 1024, "kb")

    # loop short farmyard
    farm = RAW / "animals" / "farmyard.wav"
    if farm.exists() and dur(farm) < 8:
        tmp = RAW / "animals" / "farm_prep.wav"
        run(["ffmpeg", "-y", "-stream_loop", "8", "-i", str(farm), "-t", "20", "-af", "loudnorm=I=-18:TP=-1.5", str(tmp)])
        loop_ogg(tmp, AUDIO / "animals" / "farmyard.ogg", 20, 1.0, "loudnorm=I=-18:LRA=8:TP=-1.5")
        tmp.unlink(missing_ok=True)

    print("done")


if __name__ == "__main__":
    main()
