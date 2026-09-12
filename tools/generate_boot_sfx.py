#!/usr/bin/env python3
"""Hardware-like boot SFX for the Blightnet jack-in. numpy + ffmpeg."""

from __future__ import annotations

import subprocess
import wave
from pathlib import Path

import numpy as np

SR = 44100
ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT / "audio" / "boot"
TMP = ROOT / "tools" / "_wav" / "boot"
RNG = np.random.default_rng(20770816)


def t_axis(n: int) -> np.ndarray:
    return np.arange(n, dtype=np.float64) / SR


def normalize(x: np.ndarray, peak: float = 0.78) -> np.ndarray:
    m = np.max(np.abs(x)) + 1e-12
    return (x * (peak / m)).astype(np.float64)


def fade_edges(x: np.ndarray, fade_in: float = 0.004, fade_out: float = 0.02) -> np.ndarray:
    y = x.copy()
    ni = min(len(y), int(fade_in * SR))
    no = min(len(y), int(fade_out * SR))
    if ni:
        y[:ni] *= np.linspace(0, 1, ni)
    if no:
        y[-no:] *= np.linspace(1, 0, no)
    return y


def fft_shelf(x: np.ndarray, low: float | None = None, high: float | None = None, order: int = 6) -> np.ndarray:
    n = len(x)
    spec = np.fft.rfft(x)
    f = np.fft.rfftfreq(n, 1.0 / SR)
    h = np.ones_like(f)
    if low is not None:
        hp = 1.0 / (1.0 + (low / (f + 1e-9)) ** (2 * order))
        hp[0] = 0.0
        h *= hp
    if high is not None:
        h *= 1.0 / (1.0 + ((f + 1e-9) / high) ** (2 * order))
    y = np.fft.irfft(spec * h, n)
    return y.astype(np.float64)


def pink(n: int) -> np.ndarray:
    w = RNG.standard_normal(n)
    spec = np.fft.rfft(w)
    f = np.fft.rfftfreq(n, 1.0 / SR)
    f[0] = 1.0
    spec /= np.sqrt(f)
    spec[0] = 0
    y = np.fft.irfft(spec, n)
    return normalize(y, 0.5)


def brown(n: int) -> np.ndarray:
    w = RNG.standard_normal(n)
    spec = np.fft.rfft(w)
    f = np.fft.rfftfreq(n, 1.0 / SR)
    f[0] = 1.0
    spec /= f
    spec[0] = 0
    y = np.fft.irfft(spec, n)
    return normalize(y, 0.5)


def sine(freq: np.ndarray | float, n: int) -> np.ndarray:
    if np.isscalar(freq):
        return np.sin(2 * np.pi * float(freq) * t_axis(n))
    phase = np.cumsum(freq) * (2 * np.pi / SR)
    return np.sin(phase)


def squareish(freq: float, n: int) -> np.ndarray:
    t = t_axis(n)
    y = np.zeros(n)
    for h, a in ((1, 1.0), (3, 0.28), (5, 0.14), (7, 0.07)):
        y += a * np.sin(2 * np.pi * freq * h * t)
    return y


def exp_env(n: int, rate: float) -> np.ndarray:
    return np.exp(-t_axis(n) * rate)


def place(dest: np.ndarray, src: np.ndarray, at: int) -> None:
    if at >= len(dest) or at < 0:
        return
    end = min(len(dest), at + len(src))
    dest[at:end] += src[: end - at]


def write_wav(path: Path, x: np.ndarray) -> None:
    pcm = np.clip(x, -1, 1)
    pcm = (pcm * 32767.0).astype(np.int16)
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def to_ogg(wav: Path, ogg: Path) -> None:
    ogg.parent.mkdir(parents=True, exist_ok=True)
    subprocess.check_call(
        ["ffmpeg", "-y", "-i", str(wav), "-vn", "-c:a", "libvorbis", "-q:a", "5", str(ogg)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def power() -> np.ndarray:
    n = int(0.7 * SR)
    y = np.zeros(n)
    click_n = int(0.012 * SR)
    click = fft_shelf(RNG.standard_normal(click_n), low=400, high=9000) * exp_env(click_n, 180)
    place(y, click * 0.9, int(0.02 * SR))
    thump_n = int(0.22 * SR)
    thump = sine(48, thump_n) * exp_env(thump_n, 14)
    thump += sine(92, thump_n) * 0.35 * exp_env(thump_n, 18)
    place(y, thump * 0.7, int(0.028 * SR))
    fan = fft_shelf(brown(n), high=380) * np.linspace(0, 1, n) ** 1.4 * 0.22
    y += fan
    return fade_edges(normalize(y, 0.72), 0.001, 0.08)


def hdd() -> np.ndarray:
    dur = 3.4
    n = int(dur * SR)
    t = t_axis(n)
    motor_f = np.linspace(68, 121, n)
    motor = sine(motor_f, n) * np.linspace(0.12, 0.55, n)
    motor += sine(motor_f * 2.02, n) * 0.12 * np.linspace(0.05, 0.28, n)
    platters = fft_shelf(brown(n), low=40, high=900) * np.linspace(0.08, 0.42, n)
    y = motor * 0.55 + platters * 0.7
    for _ in range(14):
        i = int(RNG.uniform(0.35, dur - 0.08) * SR)
        dn = int(RNG.uniform(0.012, 0.028) * SR)
        seek = fft_shelf(RNG.standard_normal(dn), low=1200, high=4800) * exp_env(dn, 70)
        place(y, seek * RNG.uniform(0.18, 0.38), i)
    coil = sine(np.linspace(2400, 3100, n), n) * 0.015 * np.linspace(0, 1, n)
    y += coil
    return fade_edges(normalize(y, 0.58), 0.02, 0.18)


def beep() -> np.ndarray:
    n = int(0.16 * SR)
    y = squareish(1000, n) * 0.55
    y *= np.concatenate(
        [np.linspace(0, 1, int(0.004 * SR)), np.ones(n - int(0.018 * SR)), np.linspace(1, 0, int(0.014 * SR))]
    )[:n]
    return normalize(y, 0.48)


def modem() -> np.ndarray:
    dur = 4.4
    n = int(dur * SR)
    y = np.zeros(n)

    def tone_at(freq: float, start: float, length: float, amp: float = 0.42) -> None:
        tn = int(length * SR)
        sig = sine(freq, tn) * amp
        sig = fade_edges(sig, 0.008, 0.02)
        place(y, sig, int(start * SR))

    tone_at(2100, 0.04, 0.42, 0.38)
    tone_at(2225, 0.48, 0.28, 0.34)
    # originate FSK chatter
    fsk_n = int(0.7 * SR)
    bits = RNG.integers(0, 2, fsk_n)
    fsk_f = np.where(bits == 0, 1070.0, 1270.0)
    # hold each bit ~8ms
    hold = int(0.008 * SR)
    for i in range(0, fsk_n, hold):
        fsk_f[i : i + hold] = fsk_f[i]
    place(y, sine(fsk_f, fsk_n) * 0.32, int(0.82 * SR))
    # V.22bis-ish 1200/2400
    fsk2_n = int(0.55 * SR)
    bits2 = RNG.integers(0, 2, fsk2_n)
    f2 = np.where(bits2 == 0, 1200.0, 2400.0)
    for i in range(0, fsk2_n, int(0.006 * SR)):
        f2[i : i + int(0.006 * SR)] = f2[i]
    place(y, sine(f2, fsk2_n) * 0.28, int(1.52 * SR))
    # scrambled QAM — the screaming handshake
    qn = int(2.15 * SR)
    qam = fft_shelf(RNG.standard_normal(qn), low=500, high=3400, order=4)
    am = 0.55 + 0.45 * np.sin(2 * np.pi * 22 * t_axis(qn))
    am += 0.18 * np.sin(2 * np.pi * 7.5 * t_axis(qn))
    qam *= am
    # a few carrier pips inside the scramble
    qam += sine(1800, qn) * 0.08
    qam += sine(np.linspace(1650, 2100, qn), qn) * 0.06
    place(y, fade_edges(qam * 0.42, 0.04, 0.12), int(2.05 * SR))
    carrier = sine(1800, int(0.45 * SR)) * np.linspace(0.22, 0.0, int(0.45 * SR))
    place(y, carrier, int(3.9 * SR))
    return fade_edges(normalize(y, 0.62), 0.01, 0.08)


def dive() -> np.ndarray:
    n = int(2.3 * SR)
    whoosh = fft_shelf(pink(n), low=80, high=2200)
    sweep = np.linspace(0.15, 1.0, n) * np.linspace(1.0, 0.25, n)
    whoosh *= sweep
    # rising bandpass sense via extra high layer
    air = fft_shelf(pink(n), low=1800, high=9000) * np.linspace(0.0, 0.7, n) ** 1.6
    sub = sine(np.linspace(38, 22, n), n) * exp_env(n, 1.4) * 0.55
    y = whoosh * 0.7 + air * 0.45 + sub * 0.5
    return fade_edges(normalize(y, 0.7), 0.02, 0.18)


def ice() -> np.ndarray:
    n = int(1.9 * SR)
    wall = fft_shelf(pink(n), low=200, high=1400) * 0.4
    drone = sine(54, n) * 0.35 + sine(81, n) * 0.18
    drone *= np.linspace(0.2, 1.0, n)
    y = wall + drone
    for _ in range(18):
        i = int(RNG.uniform(0.05, 1.7) * SR)
        dn = int(RNG.uniform(0.018, 0.05) * SR)
        pkt = fft_shelf(RNG.standard_normal(dn), low=2400, high=8000) * exp_env(dn, 55)
        place(y, pkt * RNG.uniform(0.12, 0.28), i)
    metal = sine(np.linspace(420, 280, n), n) * 0.08 * np.linspace(0, 1, n)
    y += metal
    return fade_edges(normalize(y, 0.58), 0.03, 0.15)


def tick() -> np.ndarray:
    n = int(0.07 * SR)
    body = fft_shelf(RNG.standard_normal(n), low=800, high=7000) * exp_env(n, 90)
    click = fft_shelf(RNG.standard_normal(int(0.008 * SR)), low=2000, high=12000) * 0.8
    y = body * 0.7
    place(y, click, 0)
    return fade_edges(normalize(y, 0.42), 0.0005, 0.02)


def hum() -> np.ndarray:
    n = int(8.0 * SR)
    t = t_axis(n)
    mains = sine(60, n) * 0.22 + sine(120, n) * 0.1 + sine(180, n) * 0.04
    fan = fft_shelf(brown(n), high=280) * 0.28
    coil = sine(15600, n) * 0.012 * (0.7 + 0.3 * np.sin(2 * np.pi * 0.7 * t))
    y = mains + fan + coil
    # loop-friendly
    cross = int(0.4 * SR)
    fade = np.linspace(0, 1, cross)
    y[:-cross] = y[:-cross]
    y[:cross] = y[:cross] * fade + y[-cross:] * (1 - fade)
    y = y[:-cross]
    return fade_edges(normalize(y, 0.38), 0.01, 0.01)


def land() -> np.ndarray:
    n = int(1.15 * SR)
    y = np.zeros(n)
    clunk_n = int(0.09 * SR)
    clunk = fft_shelf(brown(clunk_n), high=500) * exp_env(clunk_n, 28)
    clunk += sine(70, clunk_n) * exp_env(clunk_n, 22) * 0.6
    place(y, clunk * 0.85, int(0.02 * SR))
    # two short confirm beeps, PC-speaker style, not a chord
    b1 = squareish(880, int(0.07 * SR)) * 0.28
    b1 = fade_edges(b1, 0.002, 0.02)
    b2 = squareish(1175, int(0.09 * SR)) * 0.24
    b2 = fade_edges(b2, 0.002, 0.03)
    place(y, b1, int(0.18 * SR))
    place(y, b2, int(0.30 * SR))
    floor = fft_shelf(brown(n), high=200) * np.linspace(0.35, 0.0, n) * 0.4
    y += floor
    return fade_edges(normalize(y, 0.62), 0.002, 0.12)


def main() -> None:
    TMP.mkdir(parents=True, exist_ok=True)
    AUDIO.mkdir(parents=True, exist_ok=True)
    jobs = {
        "power": power,
        "hdd": hdd,
        "beep": beep,
        "modem": modem,
        "dive": dive,
        "ice": ice,
        "tick": tick,
        "hum": hum,
        "land": land,
    }
    for name, fn in jobs.items():
        wav = TMP / f"{name}.wav"
        ogg = AUDIO / f"{name}.ogg"
        write_wav(wav, fn())
        to_ogg(wav, ogg)
        print(f"wrote {ogg.relative_to(ROOT)}  {ogg.stat().st_size} bytes")


if __name__ == "__main__":
    main()
