# Blightnet

A **local** table for D&D 5e and Cyberpunk RED. Mix music and weather, drop maps, run character sheets, buy and sell, and host friends — same house or another city.

Blightnet is a **native program** (Rust). It does **not** use a web browser. Keep the whole folder together so `audio/`, `assets/`, and `data/` sit next to the binary.

The download is large (~1.2 GB) because the music, ambience, and art ship with the app.

Repo: [github.com/Steelworth/Blightnet](https://github.com/Steelworth/Blightnet)

---

## Download for the first time

Pick **one** method. Keep the whole folder together (`start.sh` / `start.bat`, `audio`, `assets`, `data`). Do not scatter those files.

### 1. Download ZIP (no Git)

1. Open **[github.com/Steelworth/Blightnet](https://github.com/Steelworth/Blightnet)**.
2. Click the green **Code** button → **Download ZIP**.
3. Unzip it. You should get a folder named **`Blightnet-main`**.
4. Follow **Windows**, **Linux**, or **macOS** below.

Later, open Blightnet and click **02 UPDATE** on INDEX. That pulls new or changed files from GitHub onto this machine.

### 2. Git clone (if you already use Git)

```bash
git clone https://github.com/Steelworth/Blightnet.git
cd Blightnet
```

Then follow **Windows**, **Linux**, or **macOS** below. **02 UPDATE** can fast-forward this clone.

---

### Windows

You need [Rust](https://rustup.rs) once, so `start.bat` can build the native window (or use a prebuilt `target\release\blightnet.exe`).

1. Open the unzipped **`Blightnet-main`** folder.
2. Double-click **`start.bat`**. The first run compiles; later runs just launch.
3. A **Blightnet** window opens. There is no browser and no `http://127.0.0.1`.

Closing the window does not stop the node. Press **Online** again to take it offline, or INDEX **00 DISCONNECT**, or `blightnet daemon-stop`. You do not need the `tools` folder on Windows.

If Windows SmartScreen warns about an unknown app: **More info** → **Run anyway**. That is the local launcher, not a Store download.

If Windows Firewall asks, allow **Blightnet** on a private network for TCP and UDP **8766** so friends can Join.

Video call and screen share on Windows need [FFmpeg](https://ffmpeg.org) on PATH (camera uses DirectShow, screen uses gdigrab). Mic, speakers, chat, maps, and Host/Join work without it.

### Linux

You need **Rust** (`rustc` / `cargo`) for the first build. Then:

```bash
cd Blightnet-main    # or: cd Blightnet
chmod +x start.sh
./start.sh
```

That runs the native window. Install Rust from [rustup.rs](https://rustup.rs) if `cargo` is missing.

### macOS

You need **Rust** (`cargo`) for the first build.

1. Open **Terminal**.
2. `cd` into the unzipped folder.
3. Run:

```bash
chmod +x start.sh
./start.sh
```

If macOS blocks the binary: System Settings → Privacy & Security → **Open Anyway**. Install Rust from [rustup.rs](https://rustup.rs) if `cargo` is missing.

---

## After it is running

INDEX (neon orange deck):

- **01 BLIGHTNEXUS** — opens TABLE (mixer). Status bar NODE is ACTIVE or OFFLINE. **Online** starts or stops the node.
- **02 CHARS** — sheets and the 0–100 die
- **03 TUTORIAL** — field manual
- **04 BLACKJACK** — house 21
- **05 AUDIO** — send level
- **00 DISCONNECT** — quit

Set a **Handle**. Switch **Hearthsong** / **Blight** in the top bar. Catalogs (Bestiary, Datashard, Gangs, and the rest) sit on INDEX.

---

## What you get

**Hearthsong** (fantasy) — tavern gold, 5e sheets, SRD bestiary (334 creatures), NPCs, gods, Armory, market stalls.

**Blight** (Night City) — HUD chrome, Cyberpunk RED sheets, Datashard, Faces, Corps, Lore, **Gangs** (101 crews with portraits), Night Market + Black Chrome, radio, blackjack.

Both worlds:

- Mixer — scenes, looping layers, master, time of day, place paintings
- Catalogs — portraits and dossiers from `data/`
- Characters — name, HP, level/rank, percentage die
- House 21

Bestiary and 5e NPCs stay on Hearthsong. Datashard, Faces, Corps, Gangs, and Lore stay on Blight.

---

## First click

1. Wait for the wake, or click / press a key to skip it.
2. Set a **Handle**. Switch Hearthsong or Blight.
3. **01 TABLE**. Click a scene. Sound loops until **Silence**.
4. **Master** is local. **Place** and **Morning / Day / Evening / Night** change the painting.

INDEX returns to the deck without stopping the mix.

---

## If something is wrong

| What you see | What to do |
| --- | --- |
| No window | Install Rust from rustup.rs, then `./start.sh` or `start.bat`. |
| Missing catalog / no sound | Keep `data/`, `audio/`, and `assets/` next to the `blightnet` binary. |
| Wrong painting after Blight | Switch world in the top bar; Place resets to that world’s first location. |
| Cargo errors | `cargo build --release` from the folder that contains `Cargo.toml`. |

---

## Development

The native app is **Rust** (`src/`, `Cargo.toml`). It reads the same `audio/`, `assets/`, and `data/` trees. Mixer catalogs are `data/mixer-catalog.json`.

```bash
cargo run --release
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

- **Code** — [MIT](LICENSE)
- **Bundled music and SFX** — keep their own licenses; see [THIRD_PARTY.md](THIRD_PARTY.md)

Kevin MacLeod (incompetech.com), CC BY 4.0 — credit him if you share a recording of a session.
