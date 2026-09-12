#!/usr/bin/env python3
"""Procedural looping audio for Blightnet. numpy + ffmpeg only."""

from __future__ import annotations

import os
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np

SR = 44100
ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT / "audio"
TMP = ROOT / "tools" / "_wav"
RNG = np.random.default_rng(20260824)


def t_axis(n: int) -> np.ndarray:
    return np.arange(n, dtype=np.float64) / SR


def midi(n: float) -> float:
    return 440.0 * (2.0 ** ((n - 69.0) / 12.0))


def normalize(x: np.ndarray, peak: float = 0.89) -> np.ndarray:
    m = np.max(np.abs(x)) + 1e-12
    return (x * (peak / m)).astype(np.float64)


def fade_edges(x: np.ndarray, fade_in: float = 0.02, fade_out: float = 0.02) -> np.ndarray:
    y = x.copy()
    ni = min(len(y), int(fade_in * SR))
    no = min(len(y), int(fade_out * SR))
    if ni:
        y[:ni] *= np.linspace(0, 1, ni)
    if no:
        y[-no:] *= np.linspace(1, 0, no)
    return y


def make_loop(x: np.ndarray, cross: float = 1.25) -> np.ndarray:
    n = int(cross * SR)
    n = min(n, len(x) // 3)
    fade = np.linspace(0, 1, n)
    y = x[:-n].copy()
    y[:n] = y[:n] * fade + x[-n:] * (1 - fade)
    return fade_edges(y, 0.008, 0.008)


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


def adsr(n: int, a: float, d: float, s: float, r: float) -> np.ndarray:
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    ns = max(0, n - na - nd - nr)
    parts = []
    if na:
        parts.append(np.linspace(0, 1, na))
    if nd:
        parts.append(np.linspace(1, s, nd))
    if ns:
        parts.append(np.full(ns, s))
    if nr:
        parts.append(np.linspace(s, 0, nr))
    env = np.concatenate(parts) if parts else np.ones(n)
    if len(env) < n:
        env = np.pad(env, (0, n - len(env)))
    return env[:n]


def exp_env(n: int, rate: float) -> np.ndarray:
    return np.exp(-t_axis(n) * rate)


def sine(freq: np.ndarray | float, n: int) -> np.ndarray:
    if np.isscalar(freq):
        return np.sin(2 * np.pi * float(freq) * t_axis(n))
    phase = np.cumsum(freq) * (2 * np.pi / SR)
    return np.sin(phase)


def tone(freq: float, n: int, harmonics: list[tuple[float, float]]) -> np.ndarray:
    t = t_axis(n)
    y = np.zeros(n)
    for h, a in harmonics:
        y += a * np.sin(2 * np.pi * freq * h * t)
    return y


def vibrato_sine(freq: float, n: int, rate: float = 5.0, depth: float = 0.012) -> np.ndarray:
    t = t_axis(n)
    inst = freq * (1.0 + depth * np.sin(2 * np.pi * rate * t))
    return sine(inst, n)


def reverb(x: np.ndarray, taps: list[tuple[float, float]] | None = None, wet: float = 0.28) -> np.ndarray:
    if taps is None:
        taps = [(0.0297, 0.42), (0.0371, 0.33), (0.0411, 0.28), (0.0437, 0.22), (0.066, 0.16), (0.089, 0.1)]
    y = x.copy()
    acc = np.zeros_like(x)
    for delay, gain in taps:
        k = int(delay * SR)
        if k <= 0 or k >= len(x):
            continue
        echo = np.zeros_like(x)
        echo[k:] = x[:-k] * gain
        acc += echo
    return x * (1 - wet) + acc * wet


def place(dest: np.ndarray, src: np.ndarray, at: int) -> None:
    if at >= len(dest) or at < 0:
        return
    end = min(len(dest), at + len(src))
    dest[at:end] += src[: end - at]


def write_wav(path: Path, x: np.ndarray) -> None:
    pcm = np.clip(x, -1, 1)
    pcm = (pcm * 32767.0).astype(np.int16)
    with wave.open(str(path), "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def to_ogg(wav: Path, ogg: Path) -> None:
    ogg.parent.mkdir(parents=True, exist_ok=True)
    subprocess.check_call(
        ["ffmpeg", "-y", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "5", str(ogg)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


# ---------------------------------------------------------------------------
# Weather
# ---------------------------------------------------------------------------

def rain(dur: float = 16.0, density: float = 1.0) -> np.ndarray:
    n = int(dur * SR)
    hiss = fft_shelf(pink(n), low=700, high=9000, order=3) * 0.55 * density
    drops = np.zeros(n)
    count = int(dur * 18 * density)
    for _ in range(count):
        i = int(RNG.integers(0, n - 2000))
        dn = int(RNG.uniform(0.012, 0.045) * SR)
        d = fft_shelf(RNG.standard_normal(dn), low=1200, high=7000) * exp_env(dn, RNG.uniform(40, 90))
        place(drops, d * RNG.uniform(0.15, 0.55), i)
    rumble = fft_shelf(brown(n), high=180) * 0.12 * density
    return make_loop(normalize(hiss + drops + rumble, 0.78))


def downpour(dur: float = 16.0) -> np.ndarray:
    n = int(dur * SR)
    sheet = fft_shelf(pink(n), low=400, high=10000, order=2) * 0.85
    low = fft_shelf(brown(n), high=250) * 0.28
    gust = 0.75 + 0.25 * np.sin(2 * np.pi * 0.07 * t_axis(n) + 1.2)
    return make_loop(normalize((sheet + low) * gust, 0.86))


def wind(dur: float = 18.0, harsh: float = 0.0) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    base = fft_shelf(pink(n), low=80, high=900 + 1400 * harsh)
    air = fft_shelf(pink(n), low=400, high=3500) * (0.25 + 0.35 * harsh)
    lfo = 0.55 + 0.45 * (0.5 + 0.5 * np.sin(2 * np.pi * 0.05 * t))
    lfo *= 0.7 + 0.3 * np.sin(2 * np.pi * 0.13 * t + 2.1)
    whoosh = np.zeros(n)
    for _ in range(int(dur / 3)):
        i = int(RNG.integers(0, n - SR))
        wn = int(RNG.uniform(0.8, 2.2) * SR)
        w = fft_shelf(pink(wn), low=200, high=1800) * np.hanning(wn) * RNG.uniform(0.2, 0.5)
        place(whoosh, w, i)
    return make_loop(normalize((base + air) * lfo + whoosh, 0.8))


def blizzard(dur: float = 18.0) -> np.ndarray:
    n = int(dur * SR)
    w = wind(dur, harsh=0.7)
    ice = fft_shelf(pink(n), low=2500, high=12000) * 0.22
    return make_loop(normalize(w + ice[: len(w)], 0.82))


def thunder_shot(kind: int = 0) -> np.ndarray:
    dur = RNG.uniform(3.6, 6.4)
    n = int(dur * SR)
    t = t_axis(n)
    if kind == 0:
        crack_n = int(0.09 * SR)
        crack = fft_shelf(RNG.standard_normal(crack_n), low=200, high=6000) * exp_env(crack_n, 28)
        rumble = brown(n) * (np.exp(-t * 1.1) * (0.4 + 0.6 * np.sin(2 * np.pi * 18 * t) ** 2))
        rumble = fft_shelf(rumble, high=220)
        y = np.zeros(n)
        place(y, crack * 1.4, int(0.04 * SR))
        y += rumble * 1.1
    elif kind == 1:
        y = fft_shelf(brown(n), high=160) * np.exp(-t * 0.7)
        y += fft_shelf(pink(n), low=60, high=400) * np.exp(-t * 1.4) * 0.35
    else:
        y = np.zeros(n)
        for k, g in enumerate([1.0, 0.55, 0.3]):
            at = int((0.05 + k * RNG.uniform(0.18, 0.4)) * SR)
            cn = int(RNG.uniform(0.06, 0.16) * SR)
            c = fft_shelf(RNG.standard_normal(cn), low=150, high=5000) * exp_env(cn, 22) * g
            place(y, c, at)
        y += fft_shelf(brown(n), high=180) * np.exp(-t * 0.85) * 0.9
    return fade_edges(normalize(y, 0.95), 0.01, 0.4)


def distant_storm(dur: float = 20.0) -> np.ndarray:
    n = int(dur * SR)
    bed = fft_shelf(brown(n), high=120) * 0.45
    bed *= 0.7 + 0.3 * np.sin(2 * np.pi * 0.04 * t_axis(n))
    y = bed.copy()
    for _ in range(3):
        th = thunder_shot(1) * 0.45
        place(y, th, int(RNG.uniform(1, dur - 7) * SR))
    return make_loop(normalize(y, 0.7), 2.0)


# ---------------------------------------------------------------------------
# Animals
# ---------------------------------------------------------------------------

def chirp(f0: float, f1: float, dur: float) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    f = np.linspace(f0, f1, n)
    env = np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 1.4
    return sine(f, n) * env


def songbirds(dur: float = 16.0) -> np.ndarray:
    n = int(dur * SR)
    y = fft_shelf(pink(n), low=2000, high=8000) * 0.03
    for _ in range(int(dur * 2.2)):
        f0 = float(RNG.uniform(1800, 4200))
        f1 = f0 + float(RNG.uniform(-900, 1600))
        d = float(RNG.uniform(0.06, 0.22))
        phrases = int(RNG.integers(1, 4))
        at = int(RNG.uniform(0, dur - 1.2) * SR)
        for p in range(phrases):
            c = chirp(f0 * (1 + 0.04 * p), f1, d) * RNG.uniform(0.12, 0.28)
            place(y, c, at + int(p * (d + 0.05) * SR))
    return make_loop(normalize(reverb(y, wet=0.2), 0.55))


def wolf_howl() -> np.ndarray:
    dur = float(RNG.uniform(2.8, 4.6))
    n = int(dur * SR)
    t = t_axis(n)
    f0 = float(RNG.uniform(220, 280))
    peak = f0 * float(RNG.uniform(1.55, 2.05))
    f = np.piecewise(
        t,
        [t < dur * 0.25, (t >= dur * 0.25) & (t < dur * 0.7)],
        [
            lambda x: f0 + (peak - f0) * (x / (dur * 0.25)),
            lambda x: peak + (f0 * 0.92 - peak) * ((x - dur * 0.25) / (dur * 0.45)),
            lambda x: f0 * 0.92 - 30 * ((x - dur * 0.7) / (dur * 0.3)),
        ],
    )
    f = f * (1 + 0.018 * np.sin(2 * np.pi * 6.2 * t))
    y = sine(f, n)
    y += 0.45 * sine(f * 2, n)
    y += 0.18 * sine(f * 3, n)
    y += 0.06 * fft_shelf(pink(n), low=300, high=1600)
    env = adsr(n, 0.35, 0.4, 0.75, 1.1)
    return fade_edges(normalize(reverb(y * env, wet=0.35), 0.88), 0.05, 0.2)


def owl_hoot() -> np.ndarray:
    def hoot(freq: float, dur: float) -> np.ndarray:
        n = int(dur * SR)
        y = vibrato_sine(freq, n, 4.5, 0.01)
        y += 0.35 * sine(freq * 2, n)
        y *= adsr(n, 0.04, 0.06, 0.7, 0.12)
        return y

    a = hoot(float(RNG.uniform(380, 460)), 0.28)
    b = hoot(float(RNG.uniform(300, 360)), 0.38)
    gap = np.zeros(int(RNG.uniform(0.12, 0.22) * SR))
    y = np.concatenate([a, gap, b]) * 0.7
    return fade_edges(normalize(reverb(y, wet=0.25), 0.7), 0.01, 0.05)


def raven_call() -> np.ndarray:
    dur = float(RNG.uniform(0.28, 0.55))
    n = int(dur * SR)
    t = t_axis(n)
    f = np.linspace(RNG.uniform(520, 700), RNG.uniform(280, 420), n)
    grain = sine(f, n) * (0.5 + 0.5 * np.sign(np.sin(2 * np.pi * RNG.uniform(28, 48) * t)))
    raspy = fft_shelf(pink(n), low=400, high=2800)
    y = (0.55 * grain + 0.7 * raspy) * adsr(n, 0.02, 0.05, 0.6, 0.12)
    return fade_edges(normalize(y, 0.75), 0.005, 0.04)


def gull_call() -> np.ndarray:
    dur = float(RNG.uniform(0.45, 0.9))
    n = int(dur * SR)
    t = t_axis(n)
    f0 = float(RNG.uniform(900, 1400))
    f = f0 * (1 + 0.22 * np.sin(2 * np.pi * 7 * t) + 0.08 * t)
    y = vibrato_sine(float(np.mean(f)), n, 8, 0.04)
    y = sine(f, n) + 0.3 * sine(f * 2, n)
    y *= adsr(n, 0.05, 0.1, 0.55, 0.2)
    return fade_edges(normalize(reverb(y, wet=0.22), 0.7), 0.01, 0.08)


def hoofbeats(dur: float = 12.0) -> np.ndarray:
    n = int(dur * SR)
    y = np.zeros(n)
    gait = 0.28
    t0 = 0.2
    while t0 < dur - 0.3:
        pattern = [0.0, 0.09, 0.18, 0.24]
        for p in pattern:
            thud_n = int(0.09 * SR)
            thud = fft_shelf(RNG.standard_normal(thud_n), high=180) * exp_env(thud_n, 38)
            thud += sine(70, thud_n) * exp_env(thud_n, 50) * 0.4
            place(y, thud * RNG.uniform(0.35, 0.7), int((t0 + p) * SR))
        t0 += gait + float(RNG.uniform(-0.02, 0.03))
    air = fft_shelf(pink(n), low=80, high=600) * 0.04
    return make_loop(normalize(y + air, 0.7), 0.8)


def crickets(dur: float = 14.0) -> np.ndarray:
    n = int(dur * SR)
    y = np.zeros(n)
    t = 0.3
    while t < dur - 0.6:
        chirps = int(RNG.integers(2, 5))
        f = float(RNG.uniform(4200, 6200))
        for k in range(chirps):
            pn = int(0.012 * SR)
            pulse = sine(f, pn) * np.hanning(pn)
            pulse += 0.4 * sine(f * 1.02, pn) * np.hanning(pn)
            place(y, pulse * 0.22, int((t + k * 0.045) * SR))
        t += float(RNG.uniform(0.35, 1.1))
    bed = fft_shelf(pink(n), low=3000, high=9000) * 0.04
    return make_loop(normalize(y + bed, 0.5), 0.8)


def frogs(dur: float = 14.0) -> np.ndarray:
    n = int(dur * SR)
    y = np.zeros(n)
    t = 0.4
    while t < dur - 0.8:
        d = float(RNG.uniform(0.12, 0.28))
        nn = int(d * SR)
        tt = t_axis(nn)
        f = float(RNG.uniform(140, 260)) * (1 + 0.15 * np.sin(2 * np.pi * 12 * tt))
        croak = sine(f, nn) + 0.5 * sine(f * 2, nn)
        croak *= (0.5 + 0.5 * np.sin(2 * np.pi * RNG.uniform(18, 30) * tt)) ** 2
        croak *= adsr(nn, 0.02, 0.04, 0.7, 0.06)
        place(y, croak * RNG.uniform(0.25, 0.5), int(t * SR))
        t += float(RNG.uniform(0.5, 1.8))
    wet = fft_shelf(pink(n), low=200, high=1600) * 0.05
    return make_loop(normalize(reverb(y + wet, wet=0.3), 0.55), 0.9)


def bats(dur: float = 12.0) -> np.ndarray:
    n = int(dur * SR)
    y = fft_shelf(pink(n), high=400) * 0.04
    for _ in range(int(dur * 3)):
        d = float(RNG.uniform(0.012, 0.04))
        nn = int(d * SR)
        f = float(RNG.uniform(6000, 11000))
        click = sine(f, nn) * np.hanning(nn) * RNG.uniform(0.08, 0.18)
        place(y, click, int(RNG.uniform(0, dur - 0.05) * SR))
    return make_loop(normalize(y, 0.4), 0.6)


# ---------------------------------------------------------------------------
# Ambience
# ---------------------------------------------------------------------------

def campfire(dur: float = 16.0) -> np.ndarray:
    n = int(dur * SR)
    rumble = fft_shelf(brown(n), high=140) * 0.55
    hiss = fft_shelf(pink(n), low=1500, high=7000) * 0.12
    pops = np.zeros(n)
    t = 0.2
    while t < dur - 0.2:
        dn = int(RNG.uniform(0.02, 0.11) * SR)
        pop = fft_shelf(RNG.standard_normal(dn), low=400, high=5000) * exp_env(dn, RNG.uniform(25, 70))
        place(pops, pop * RNG.uniform(0.2, 0.9), int(t * SR))
        t += float(RNG.exponential(0.22))
    flicker = 0.85 + 0.15 * np.sin(2 * np.pi * 3.4 * t_axis(n))
    return make_loop(normalize((rumble + hiss) * flicker + pops, 0.78), 1.0)


def crowd(dur: float = 16.0, density: float = 1.0, clink: bool = True) -> np.ndarray:
    n = int(dur * SR)
    voices = np.zeros(n)
    for _ in range(int(10 * density)):
        band = fft_shelf(pink(n), low=float(RNG.uniform(120, 220)), high=float(RNG.uniform(1400, 2800)))
        lfo = 0.6 + 0.4 * np.sin(2 * np.pi * RNG.uniform(0.4, 2.2) * t_axis(n) + RNG.uniform(0, 6))
        voices += band * lfo * RNG.uniform(0.08, 0.18)
    voices = fft_shelf(voices, low=100, high=2400)
    if clink:
        for _ in range(int(dur * 0.35 * density)):
            dn = int(RNG.uniform(0.04, 0.16) * SR)
            f = float(RNG.uniform(1200, 2800))
            c = (sine(f, dn) + 0.4 * sine(f * 2.1, dn)) * exp_env(dn, 18) * 0.12
            place(voices, c, int(RNG.uniform(0, dur - 0.2) * SR))
    return make_loop(normalize(voices, 0.55), 1.2)


def water_flow(dur: float = 16.0, heavy: bool = False) -> np.ndarray:
    n = int(dur * SR)
    hi = 1800 if not heavy else 1200
    flow = fft_shelf(pink(n), low=120, high=hi) * (0.7 if not heavy else 1.0)
    gurgle = fft_shelf(brown(n), high=300) * (0.2 if not heavy else 0.45)
    lfo = 0.8 + 0.2 * np.sin(2 * np.pi * 0.08 * t_axis(n))
    splashes = np.zeros(n)
    for _ in range(int(dur * (1.2 if heavy else 0.5))):
        dn = int(RNG.uniform(0.05, 0.2) * SR)
        s = fft_shelf(RNG.standard_normal(dn), low=400, high=5000) * exp_env(dn, 16) * RNG.uniform(0.08, 0.25)
        place(splashes, s, int(RNG.uniform(0, dur - 0.3) * SR))
    return make_loop(normalize((flow + gurgle) * lfo + splashes, 0.75), 1.2)


def ocean(dur: float = 18.0) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    swell = 0.5 + 0.5 * (0.5 + 0.5 * np.sin(2 * np.pi * 0.055 * t)) ** 1.6
    roar = fft_shelf(pink(n), low=40, high=700) * swell
    foam = fft_shelf(pink(n), low=800, high=8000) * (swell ** 2) * 0.45
    return make_loop(normalize(roar + foam, 0.8), 2.0)


def dungeon(dur: float = 18.0) -> np.ndarray:
    n = int(dur * SR)
    air = fft_shelf(brown(n), high=160) * 0.5
    air += fft_shelf(pink(n), low=80, high=400) * 0.08
    drips = np.zeros(n)
    t = 0.8
    while t < dur - 1:
        f = float(RNG.uniform(1400, 2600))
        dn = int(0.35 * SR)
        d = (sine(f, dn) + 0.25 * sine(f * 2.3, dn)) * exp_env(dn, 9) * RNG.uniform(0.12, 0.28)
        d = reverb(d, wet=0.5)
        place(drips, d, int(t * SR))
        t += float(RNG.uniform(1.1, 3.4))
    return make_loop(normalize(air + drips, 0.6), 1.6)


def cavern(dur: float = 18.0) -> np.ndarray:
    n = int(dur * SR)
    air = fft_shelf(brown(n), high=90) * 0.7
    windy = fft_shelf(pink(n), low=60, high=350) * 0.18
    y = reverb(air + windy, wet=0.45)
    drips = dungeon(dur) * 0.35
    m = min(len(y), len(drips))
    return normalize(y[:m] + drips[:m], 0.62)


def swamp(dur: float = 16.0) -> np.ndarray:
    bugs = fft_shelf(pink(int(dur * SR)), low=2500, high=7000) * 0.08
    wet = water_flow(dur, heavy=False) * 0.45
    f = frogs(dur) * 0.7
    m = min(len(bugs), len(wet), len(f))
    return normalize(bugs[:m] + wet[:m] + f[:m], 0.62)


def graveyard(dur: float = 18.0) -> np.ndarray:
    air = wind(dur, harsh=0.15) * 0.55
    n = len(air)
    low = fft_shelf(brown(n), high=70) * 0.35
    y = air + low
    for _ in range(3):
        at = int(RNG.uniform(0.4, max(1.0, (n / SR) - 3)) * SR)
        place(y, raven_call() * 0.35, at)
    return normalize(y, 0.58)


def magic_hum(dur: float = 16.0) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    f = 55 * (1 + 0.03 * np.sin(2 * np.pi * 0.07 * t))
    y = sine(f, n) * 0.4
    y += sine(f * 2, n) * 0.18
    y += sine(f * 3.02, n) * 0.1
    y += sine(f * 4.97, n) * 0.06
    shimmer = fft_shelf(pink(n), low=2000, high=8000) * (0.04 + 0.03 * np.sin(2 * np.pi * 0.2 * t))
    y = reverb(y + shimmer, wet=0.4)
    return make_loop(normalize(y, 0.55), 2.0)


def forge(dur: float = 14.0) -> np.ndarray:
    fire = campfire(dur) * 0.45
    y = fire.copy()
    n = len(y)
    t = 0.6
    end = n / SR - 0.5
    while t < end:
        kn = int(0.25 * SR)
        tt = t_axis(kn)
        hit = sine(180 * np.exp(-tt * 18) + 60, kn) * exp_env(kn, 14)
        hit += fft_shelf(RNG.standard_normal(kn), low=200, high=4000) * exp_env(kn, 22) * 0.5
        place(y, hit * RNG.uniform(0.4, 0.8), int(t * SR))
        t += float(RNG.uniform(1.3, 2.4))
    return normalize(y, 0.78)


def ship(dur: float = 16.0) -> np.ndarray:
    water = ocean(dur) * 0.35
    w = wind(dur, 0.2) * 0.35
    m = min(len(water), len(w))
    creaks = np.zeros(m)
    t = 0.5
    end = m / SR - 0.8
    while t < end:
        d = float(RNG.uniform(0.4, 1.4))
        nn = int(d * SR)
        f = float(RNG.uniform(90, 220))
        ck = sine(f * (1 + 0.08 * t_axis(nn)), nn)
        ck += fft_shelf(pink(nn), low=80, high=800) * 0.4
        ck *= adsr(nn, 0.08, 0.2, 0.4, 0.3) * RNG.uniform(0.15, 0.35)
        place(creaks, ck, int(t * SR))
        t += float(RNG.uniform(1.2, 3.0))
    return normalize(water[:m] + creaks + w[:m], 0.7)


def desert(dur: float = 16.0) -> np.ndarray:
    w = wind(dur, 0.35)
    n = len(w)
    sand = fft_shelf(pink(n), low=600, high=4000) * 0.12
    return normalize(w + sand, 0.68)


def lava(dur: float = 16.0) -> np.ndarray:
    n = int(dur * SR)
    boil = fft_shelf(brown(n), high=180) * 0.7
    hiss = fft_shelf(pink(n), low=800, high=5000) * 0.16
    bubbles = np.zeros(n)
    t = 0.3
    while t < dur - 0.4:
        dn = int(RNG.uniform(0.08, 0.35) * SR)
        b = fft_shelf(RNG.standard_normal(dn), low=40, high=400) * exp_env(dn, 8) * RNG.uniform(0.2, 0.6)
        place(bubbles, b, int(t * SR))
        t += float(RNG.exponential(0.35))
    return make_loop(normalize(boil + hiss + bubbles, 0.8), 1.0)


def war_camp(dur: float = 16.0) -> np.ndarray:
    murmur = crowd(dur, density=0.7, clink=False) * 0.7
    n = len(murmur)
    drums = np.zeros(n)
    t = 0.4
    end = n / SR - 0.8
    while t < end:
        kn = int(0.45 * SR)
        tt = t_axis(kn)
        k = sine(70 * np.exp(-tt * 6) + 40, kn) * exp_env(kn, 5) * 0.45
        place(drums, k, int(t * SR))
        t += float(RNG.uniform(1.6, 2.8))
    return normalize(murmur + drums, 0.62)


def temple_hall(dur: float = 16.0) -> np.ndarray:
    hum = magic_hum(dur) * 0.35
    n = len(hum)
    air = fft_shelf(pink(n), low=80, high=500) * 0.08
    y = air + hum
    for _ in range(2):
        f = float(RNG.choice([523.25, 659.25, 783.99]))
        dn = int(2.8 * SR)
        bell = (sine(f, dn) + 0.4 * sine(f * 2.01, dn) + 0.15 * sine(f * 3.1, dn)) * exp_env(dn, 1.4)
        at = int(RNG.uniform(1.0, max(2.0, n / SR - 3)) * SR)
        place(y, bell * 0.18, at)
    return normalize(reverb(y, wet=0.5), 0.55)


def torch(dur: float = 12.0) -> np.ndarray:
    n = int(dur * SR)
    hiss = fft_shelf(pink(n), low=2000, high=8000) * 0.18
    low = fft_shelf(brown(n), high=120) * 0.22
    pops = np.zeros(n)
    t = 0.1
    while t < dur - 0.1:
        dn = int(RNG.uniform(0.01, 0.05) * SR)
        p = fft_shelf(RNG.standard_normal(dn), low=800, high=6000) * exp_env(dn, 50) * RNG.uniform(0.1, 0.4)
        place(pops, p, int(t * SR))
        t += float(RNG.exponential(0.35))
    return make_loop(normalize(hiss + low + pops, 0.5), 0.6)


# ---------------------------------------------------------------------------
# Music
# ---------------------------------------------------------------------------

SCALES = {
    "ionian": [0, 2, 4, 5, 7, 9, 11],
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "phrygian": [0, 1, 3, 5, 7, 8, 10],
    "lydian": [0, 2, 4, 6, 7, 9, 11],
    "mixolydian": [0, 2, 4, 5, 7, 9, 10],
    "aeolian": [0, 2, 3, 5, 7, 8, 10],
    "pent": [0, 2, 4, 7, 9],
    "mpent": [0, 3, 5, 7, 10],
}


def pad_note(freq: float, n: int, amp: float) -> np.ndarray:
    y = vibrato_sine(freq, n, 4.2, 0.006) * 0.55
    y += vibrato_sine(freq * 1.003, n, 3.6, 0.005) * 0.35
    y += sine(freq * 2, n) * 0.12
    y += sine(freq * 3, n) * 0.05
    env = adsr(n, 0.6, 0.8, 0.75, 1.4)
    return y * env * amp


def pluck(freq: float, dur: float, amp: float = 0.25) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    y = sine(freq, n) * np.exp(-t * 3.8)
    y += 0.45 * sine(freq * 2, n) * np.exp(-t * 6)
    y += 0.18 * sine(freq * 3, n) * np.exp(-t * 9)
    y += 0.08 * fft_shelf(RNG.standard_normal(n), high=freq * 6) * np.exp(-t * 40)
    return y * amp


def flute(freq: float, dur: float, amp: float = 0.22) -> np.ndarray:
    n = int(dur * SR)
    y = vibrato_sine(freq, n, 5.2, 0.012)
    y += 0.12 * sine(freq * 2, n)
    breath = fft_shelf(pink(n), low=freq, high=freq * 8) * 0.08
    y = (y + breath) * adsr(n, 0.06, 0.1, 0.8, 0.18) * amp
    return y


def choir(freq: float, n: int, amp: float) -> np.ndarray:
    y = np.zeros(n)
    for det in (-0.008, -0.003, 0.0, 0.0035, 0.007):
        y += vibrato_sine(freq * (1 + det), n, 5.0, 0.004)
    y += 0.25 * sine(freq * 2, n)
    return y * adsr(n, 0.8, 0.6, 0.8, 1.6) * (amp / 5)


def mallet(freq: float, dur: float, amp: float = 0.2) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    y = sine(freq, n) * np.exp(-t * 2.2)
    y += 0.5 * sine(freq * 2.01, n) * np.exp(-t * 3.5)
    y += 0.2 * sine(freq * 4.1, n) * np.exp(-t * 6)
    return y * amp


def kick(dur: float = 0.42) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    f = 140 * np.exp(-t * 14) + 38
    return sine(f, n) * np.exp(-t * 7)


def snare(dur: float = 0.22) -> np.ndarray:
    n = int(dur * SR)
    t = t_axis(n)
    body = sine(180, n) * np.exp(-t * 16)
    noise = fft_shelf(RNG.standard_normal(n), low=800, high=8000) * np.exp(-t * 18)
    return 0.35 * body + 0.8 * noise


def hat(dur: float = 0.07) -> np.ndarray:
    n = int(dur * SR)
    return fft_shelf(RNG.standard_normal(n), low=6000, high=14000) * exp_env(n, 55)


def brass(freq: float, dur: float, amp: float = 0.18) -> np.ndarray:
    n = int(dur * SR)
    y = tone(freq, n, [(1, 1), (2, 0.55), (3, 0.4), (4, 0.18), (5, 0.22), (6, 0.1)])
    return y * adsr(n, 0.08, 0.18, 0.7, 0.2) * amp


def compose(
    dur: float,
    root: int,
    scale_name: str,
    bpm: float,
    kind: str,
) -> np.ndarray:
    n = int(dur * SR)
    y = np.zeros(n)
    scale = SCALES[scale_name]
    beat = 60.0 / bpm
    bar = beat * 4
    degrees = [0, 3, 4, 0, 5, 3, 4, 0]
    if kind in ("dungeon", "rite", "dragon"):
        degrees = [0, 0, 5, 3, 0, 6, 5, 0]
    if kind in ("sacred", "elven"):
        degrees = [0, 4, 5, 4, 0, 3, 5, 0]

    t = 0.0
    bar_i = 0
    while t < dur - 0.5:
        deg = degrees[bar_i % len(degrees)]
        chord = [deg, deg + 2, deg + 4]
        freqs = []
        for c in chord:
            octv = 0
            st = scale[c % len(scale)] + 12 * (c // len(scale) + octv)
            freqs.append(midi(root + st))
        bn = int(min(bar * 1.05, dur - t) * SR)
        at = int(t * SR)
        bass_f = midi(root + scale[deg % len(scale)] - 12)

        if kind in ("tavern", "road", "court"):
            place(y, pad_note(bass_f, bn, 0.12), at)
            for i, f in enumerate(freqs):
                place(y, pad_note(f, bn, 0.06), at)
            steps = 8 if kind != "court" else 4
            for s in range(steps):
                st = scale[(deg + [0, 2, 4, 2, 0, 4, 2, 3][s % 8]) % len(scale)]
                note = midi(root + st + (12 if kind == "court" else 12))
                d = beat / (2 if steps == 8 else 1) * 0.9
                instr = pluck if kind != "court" else mallet
                place(y, instr(note, d, 0.16 if s % 4 == 0 else 0.11), int((t + s * (bar / steps)) * SR))
            if kind == "tavern":
                for s in range(8):
                    if s % 2 == 1:
                        place(y, hat() * 0.07, int((t + s * beat / 2) * SR))

        elif kind in ("forest", "ocean", "winter"):
            place(y, pad_note(bass_f, bn, 0.16), at)
            for f in freqs:
                place(y, pad_note(f * 0.5 if kind == "ocean" else f, bn, 0.08), at)
            if bar_i % 2 == 0:
                seq = [4, 2, 5, 4, 2, 0, 3, 2]
                for s, ddeg in enumerate(seq[:4]):
                    st = scale[ddeg % len(scale)]
                    nf = midi(root + st + 12)
                    d = beat * 0.95
                    fn = flute if kind != "winter" else mallet
                    place(y, fn(nf, d, 0.14), int((t + s * beat) * SR))

        elif kind in ("dungeon", "rite"):
            place(y, pad_note(bass_f, bn, 0.22), at)
            place(y, pad_note(bass_f * 1.498, bn, 0.08 if kind == "rite" else 0.04), at)
            for f in freqs:
                place(y, pad_note(f, bn, 0.05), at)
            if bar_i % 2 == 1:
                st = scale[(deg + 4) % len(scale)]
                place(y, pluck(midi(root + st), beat * 1.6, 0.1), at + int(beat * SR))

        elif kind == "battle":
            place(y, pad_note(bass_f, bn, 0.14), at)
            ost = [0, 0, 2, 0, 3, 0, 2, 4]
            for s, od in enumerate(ost):
                st = scale[od % len(scale)]
                place(y, brass(midi(root + st), beat / 2 * 0.9, 0.12), int((t + s * beat / 2) * SR))
            place(y, kick() * 0.7, at)
            place(y, kick() * 0.45, at + int(beat * 2 * SR))
            place(y, snare() * 0.35, at + int(beat * SR))
            place(y, snare() * 0.35, at + int(beat * 3 * SR))
            for s in range(8):
                place(y, hat() * 0.08, int((t + s * beat / 2) * SR))

        elif kind == "sacred":
            place(y, choir(bass_f, bn, 0.22), at)
            for f in freqs:
                place(y, choir(f, bn, 0.12), at)
            if bar_i % 4 == 0:
                place(y, mallet(midi(root + 24), 2.5, 0.12), at)

        elif kind == "elven":
            place(y, pad_note(bass_f, bn, 0.1), at)
            arp = [0, 2, 4, 7, 4, 2, 0, 4]
            for s, od in enumerate(arp):
                st = scale[od % len(scale)]
                place(y, mallet(midi(root + st + 12), beat / 2, 0.16), int((t + s * beat / 2) * SR))

        elif kind == "dragon":
            place(y, pad_note(bass_f / 2, bn, 0.25), at)
            place(y, pad_note(bass_f, bn, 0.16), at)
            for f in freqs:
                place(y, brass(f * 0.5, min(bar, 1.6), 0.1), at)
            if bar_i % 2 == 0:
                place(y, kick() * 0.55, at)
                place(y, kick() * 0.3, at + int(bar * 0.5 * SR))

        t += bar
        bar_i += 1

    y = reverb(y, wet=0.32 if kind not in ("battle",) else 0.18)
    return make_loop(normalize(y, 0.78), 1.8)


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

JOBS: list[tuple[str, str, callable]] = []


def add(folder: str, name: str, fn) -> None:
    JOBS.append((folder, name, fn))


def main() -> None:
    TMP.mkdir(parents=True, exist_ok=True)
    for sub in ("music", "weather", "animals", "ambience", "oneshots"):
        (AUDIO / sub).mkdir(parents=True, exist_ok=True)

    add("weather", "rain", lambda: rain(16, 1.0))
    add("weather", "downpour", lambda: downpour(16))
    add("weather", "wind", lambda: wind(18, 0.15))
    add("weather", "blizzard", lambda: blizzard(18))
    add("weather", "distant_storm", lambda: distant_storm(20))
    add("oneshots", "thunder_1", lambda: thunder_shot(0))
    add("oneshots", "thunder_2", lambda: thunder_shot(1))
    add("oneshots", "thunder_3", lambda: thunder_shot(2))

    add("animals", "songbirds", lambda: songbirds(16))
    add("animals", "crickets", lambda: crickets(14))
    add("animals", "frogs", lambda: frogs(14))
    add("animals", "horses", lambda: hoofbeats(12))
    add("animals", "bats", lambda: bats(12))
    add("oneshots", "wolf_1", wolf_howl)
    add("oneshots", "wolf_2", wolf_howl)
    add("oneshots", "owl_1", owl_hoot)
    add("oneshots", "owl_2", owl_hoot)
    add("oneshots", "raven_1", raven_call)
    add("oneshots", "raven_2", raven_call)
    add("oneshots", "gull_1", gull_call)
    add("oneshots", "gull_2", gull_call)

    add("ambience", "campfire", lambda: campfire(16))
    add("ambience", "tavern_hall", lambda: crowd(16, 1.0, True))
    add("ambience", "market", lambda: crowd(16, 1.5, True))
    add("ambience", "dungeon", lambda: dungeon(18))
    add("ambience", "cavern", lambda: cavern(18))
    add("ambience", "ocean", lambda: ocean(18))
    add("ambience", "river", lambda: water_flow(16, False))
    add("ambience", "waterfall", lambda: water_flow(16, True))
    add("ambience", "swamp", lambda: swamp(16))
    add("ambience", "graveyard", lambda: graveyard(18))
    add("ambience", "magic", lambda: magic_hum(16))
    add("ambience", "forge", lambda: forge(14))
    add("ambience", "ship", lambda: ship(16))
    add("ambience", "desert", lambda: desert(16))
    add("ambience", "lava", lambda: lava(16))
    add("ambience", "war_camp", lambda: war_camp(16))
    add("ambience", "temple", lambda: temple_hall(16))
    add("ambience", "torch", lambda: torch(12))

    add("music", "tavern_jig", lambda: compose(28, 67, "mixolydian", 112, "tavern"))
    add("music", "forest_wander", lambda: compose(28, 62, "dorian", 72, "forest"))
    add("music", "dungeon_depths", lambda: compose(28, 50, "phrygian", 58, "dungeon"))
    add("music", "battle_march", lambda: compose(24, 57, "aeolian", 128, "battle"))
    add("music", "sacred_hymn", lambda: compose(28, 65, "lydian", 56, "sacred"))
    add("music", "ocean_voyage", lambda: compose(28, 57, "dorian", 64, "ocean"))
    add("music", "dark_rite", lambda: compose(28, 52, "phrygian", 48, "rite"))
    add("music", "royal_court", lambda: compose(26, 60, "ionian", 88, "court"))
    add("music", "road_theme", lambda: compose(26, 67, "mixolydian", 98, "road"))
    add("music", "elven_glade", lambda: compose(26, 69, "lydian", 74, "elven"))
    add("music", "winter_march", lambda: compose(28, 62, "aeolian", 62, "winter"))
    add("music", "dragon_wake", lambda: compose(26, 48, "aeolian", 78, "dragon"))

    total = len(JOBS)
    for i, (folder, name, fn) in enumerate(JOBS, 1):
        ogg = AUDIO / folder / f"{name}.ogg"
        if ogg.exists() and ogg.stat().st_size > 2000:
            print(f"[{i:02d}/{total}] {folder}/{name} (skip)", flush=True)
            continue
        print(f"[{i:02d}/{total}] {folder}/{name} ...", flush=True)
        x = np.nan_to_num(fn(), copy=False)
        wav = TMP / f"{name}.wav"
        write_wav(wav, x)
        to_ogg(wav, ogg)
        wav.unlink(missing_ok=True)
        print(f"         {ogg.stat().st_size // 1024} kb", flush=True)

    try:
        TMP.rmdir()
    except OSError:
        pass
    print("done.")


if __name__ == "__main__":
    sys.exit(main())
