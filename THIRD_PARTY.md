# Third-party audio and art

The Blightnet **source code** is MIT-licensed. Bundled media keeps its own licenses. Credit these if you share a recording of a session.

## Music

- **Kevin MacLeod** (incompetech.com) — [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
  - Hearthsong playlists and many Blight electronic / rock pieces.

## Sound effects

- **[Mixkit](https://mixkit.co)** — Mixkit License (weather, fire, crowds, many creatures).
- **OpenGameArt** — CC0 (some dungeon / cavern beds).
- **Wikimedia Commons** wildlife recordings — ravens, owls, gulls (see each file’s Commons page for the exact license).

## In-app credit

The **i** button on the table repeats the same music credit.

Generated portraits, place paintings, and still-life kit photos in `assets/` were made for this table. Treat them as part of the project unless a file says otherwise.

## Native libraries

- **[Opus](https://opus-codec.org/)** — BSD-style (libopus). Linked for live-call voice encode/decode. System package on Linux (`libopus`); not WebRTC. Same system lib expected on Windows/macOS builds.
- **[SpeexDSP](https://gitlab.xiph.org/xiph/speexdsp)** — BSD-style (libspeexdsp). Acoustic echo cancellation on the capture path before Opus encode. System package on Linux (`speexdsp` / `libspeexdsp`); probed via pkg-config at build time. If missing, Blightnet builds without AEC and voice still works.

