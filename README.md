# Blightnet

A **local** table for D&D and Cyberpunk RED: mix music and weather, drop maps, run character sheets, and host a LAN session. Nothing is sent to the cloud except the explicit “join this table” connection on your own network.

You do **not** open `index.html` as a file. Download the folder, start it, then use the Blightnet window (or **http://127.0.0.1:8765**).

The download is large (~1.2 GB) because the music, ambience, and art ship with the app.

Repo: [github.com/Steelworth/Blightnet](https://github.com/Steelworth/Blightnet)

---

## Download for the first time

Pick **one** method. Keep the whole folder together (`Blightnet.exe` or `start.sh`, plus `index.html`, `js`, `css`, `audio`, `assets`). Do not scatter those files.

### 1. Download ZIP (no Git)

1. Open **[github.com/Steelworth/Blightnet](https://github.com/Steelworth/Blightnet)**.
2. Click the green **Code** button → **Download ZIP**.
3. Unzip it. You should get a folder named **`Blightnet-main`**.
4. Follow **Windows**, **Linux**, or **macOS** below.

Later, open Blightnet and click **02 UPDATE** on INDEX. That pulls new or changed files from GitHub onto this machine.

### 2. Git clone (best if you already use Git)

```bash
git clone https://github.com/Steelworth/Blightnet.git
cd Blightnet
```

Then follow **Windows**, **Linux**, or **macOS** below. **02 UPDATE** can fast-forward this clone.

---

### Windows

You do **not** need Python or Git.

1. Open the unzipped **`Blightnet-main`** folder (or the `Blightnet` folder from git). You should see **`Blightnet.exe`** next to **`index.html`**.
2. Double-click **`Blightnet.exe`**. If Windows blocks it, use **`start.bat`** instead (it unblocks the launcher, then starts it).
3. A black console stays open — that is the server. Leave it open.
4. Edge or Chrome should open as a Blightnet window. If nothing appears, read the console and open **http://127.0.0.1:8765** yourself.

Closing the console (or INDEX **00 DISCONNECT**) stops the table. You do not need the `tools` folder on Windows.

If Windows SmartScreen warns about an unknown app: **More info** → **Run anyway**. That is the local launcher, not a Store download.

If Windows Firewall asks, allow **Blightnet** on a private network so friends on the same house Wi-Fi can Join.

### Linux

You need **Python 3** (already on most distros). No pip packages.

```bash
cd Blightnet-main    # or: cd Blightnet
chmod +x start.sh
./start.sh
```

A dedicated Blightnet window opens when it can. If not, the terminal prints **http://127.0.0.1:8765**. Open that address yourself.

- `./start.sh --system-browser` — use Firefox/Chrome/Brave
- `./start.sh --no-open` — server only

**If the page says to run start.sh first**, you opened `index.html` from the folder. Close that tab. Run `./start.sh`.

If `python3` is missing: Ubuntu/Debian `sudo apt install python3`, Fedora `sudo dnf install python3`, Arch `sudo pacman -S python`.

### macOS

You need **Python 3**.

1. Open **Terminal**.
2. Drag the unzipped **`Blightnet-main`** folder onto the Terminal window after `cd ` and press Enter (or `cd` into the git clone).
3. Run:

```bash
chmod +x start.sh
./start.sh
```

If macOS blocks it: System Settings → Privacy & Security → **Open Anyway**. If `python3` is missing, install it from [python.org](https://www.python.org/downloads/) or `xcode-select --install`.

Same flags as Linux: `--system-browser`, `--no-open`. Use **http://127.0.0.1:8765** if no window appears.

---

## After it is running

On INDEX (the yellow NET menu):

- **01 TABLE** — the mixer
- **02 UPDATE** — check GitHub and patch this copy (hacking-style trace while it downloads)
- **03 TUTORIAL** — in-app field manual

Type a **Handle**, pick a **DISPLAY**, then open TABLE and **Light the hearth**.

---

## What you get

- **Hearthsong** — tavern gold, fantasy places, 5e sheets, SRD bestiary, Armory, gods, NPCs
- **Blight** — Night City HUD, RED sheets, Datashard, Faces, Corps, Lore, Night Market + Black Chrome
- **Mixer** — scenes, moods, shuffle, weather, animals, uploaded tracks
- **Maps** — VTT: pan, wheel-zoom, tokens, drawings, live for the table
- **Characters** — 5e and Cyberpunk RED, unarmed combat, advantage/disadvantage, death saves, level-up / IP
- **LAN table** — named host/join, chat, whispers, P2P files, voice, shared mix and combat log

Switch **Hearthsong** / **Blight** in the top-right. The choice is remembered on that computer.

---

## Share a table (same house or other cities)

Each computer runs Blightnet and sets a **Handle** in the top bar. Friends do **not** need to be on your Wi-Fi.

1. Skip the wake sequence if you want. You are in Blightnet.
2. Type a name. On the host click **Host**. Wait a few seconds. Status becomes **Hosting · Gamemaster** plus an address.
3. Click **Copy address**. Same house: a LAN IP like `192.168.1.20:8765`. Other city: an `https://…` link (Blightnet opens an outbound tunnel so their router does not need a port forward).
4. Send that address. They click **Join** and paste it, or open the https link in a browser.
5. **Chat** is in the yellow bar. `/w Name text` is a whisper.

If the https link never appears, the host machine needs OpenSSH (`ssh` on the PATH). LAN play still works. **Leave** disconnects. See [SECURITY.md](SECURITY.md).

---

## First click

1. Wait for the wake sequence, or click / press a key to skip it.
2. Set your **Handle**. **Host** or **Join**, and open **Chat**, from the top bar.
3. Optional: **02 UPDATE** on INDEX to pull the newest files from GitHub.
4. Click **01 TABLE** when you want the mixer.
5. The table unrolls in the Blightnet window. INDEX tab returns to the menu without dropping the table.

Opening TABLE is also what lets the browser play sound.

---

## How to play a scene

On the left is **Scenes**. Each one is a ready-made mix with its own picture. Type in **Find a scene** to shrink the list.

- Click **Quiet Tavern** for an inn.
- Click **Battles** for the fight playlist — every war track, one after another.
- Click **Dungeon Crawl** for a dungeon.
- Click **Raging Storm** for weather.
- Click **The Strand** for a beach, **Moonlit Shore** for night on the sand.
- Click **Below the Waves** to go underwater, **The Wreck** for a drowned ruin.

Gold on the left means that scene is the one you last pressed. Sound starts. The painting at the top changes to match.

---

## Change the mix

On the right is **The mix**. Every sound has a round button and a volume slider.

- Click the round button to turn that sound on or off. Gold means it is on.
- Drag the slider to make it louder or quieter.
- The top of the painting shows small pills for what is playing. Click a pill to drop that sound.

Tabs: **All / Playing / Music / Weather / Animals / Ambience**. **Playing** shows only what is on.

**Music** is sorted by mood. **Shuffle** swaps the playing piece for another of the same mood.

**Add sound** (table bar) picks a file from this computer. If you are hosting, the others receive it.

**Maps** — import a picture, pan, scroll to zoom, **Grid**, **Fit**. Host map is live for guests.

**Characters** — 5e or RED sheets on this machine, plus everyone else’s sheets at a joined table (read-only). Combat uses a 0–100 roll, not a d20. **Adv** / **Dis** on the combat log roll twice (higher / lower).

**Update** checks GitHub (`Steelworth/Blightnet`) for new or changed files, downloads them onto this machine, then reloads sounds and maps the host shared. If `serve.py` or the Windows exe changed, close Blightnet and start it again.

**Find a sound** — press `/` to jump there. Escape clears it (or silences the table if you are not typing).

---

## The painting, Place, hour

The picture stays on screen even if you scroll.

**Place** (top bar) opens taverns and wilds on Hearthsong, Night City districts plus the Moon and a casino on Blight. The camera does not pan when you pick a plate.

**Morning / Day / Evening / Night** crossfade the sky.

**Outside / Inside** is where your ear sits, not the picture. Outside: weather is close. Inside: rooms are close, rain is through the walls.

---

## Master, Fade, Silence

- **Master** — volume for everything.
- **Fade out** — about four seconds to silence.
- **Fade in** — bring the last mix back the same way.
- **Silence** — cut now. Escape does the same unless you are in a search box.

---

## Save a mix

1. Click **Save this mix** under the scene list.
2. Type a name.
3. Click **Save**.

It appears on the left with a picture of the place that was showing. Click × to throw it away.

The **i** button is credits, not a second mixer.

---

## What this computer remembers

In the browser on this machine: master volume, outside/inside, hour, last place, saved mixes, theme, handle, contacts, character sheets.

No account. No internet after you have the folder, except joining a table on your LAN.

---

## If something is wrong

| What you see | What to do |
| --- | --- |
| “Run start.sh first” | Do not open `index.html` from the folder. Windows: `Blightnet.exe` (or `start.bat`). Linux/Mac: `./start.sh`. Then **http://127.0.0.1:8765** |
| Windows SmartScreen | **More info** → **Run anyway**. Or double-click `start.bat`. |
| Windows: no window | Leave the black console open. Open **http://127.0.0.1:8765**. |
| No sound after Light the hearth | Click the page once. Unmute the tab. |
| Sound stopped | Click the page. Some browsers pause when you leave the tab. |
| Port 8765 is busy | The starter picks another port and prints it. |
| Page looks old after an update | Refresh (Ctrl+R or Cmd+R). |
| Mic refused on `http://192.168…` | Voice needs a secure context. `http://127.0.0.1` works. Plain LAN HTTP may block `getUserMedia`. |

Keep the terminal (or the black Windows console) open the whole session.

---

## Development

Source lives in `js/`, `css/`, `index.html`, and `serve.py`. Bundled media is `audio/` and `assets/`. Table catalogs are `data/*.json`.

Rebuild the Windows exe from Linux:

```bash
./tools/build_windows.sh
```

Go sources for that launcher: `tools/winlaunch/`.

Catalog builders (optional, already run): `tools/build_*.py`.

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

- **Code** — [MIT](LICENSE)
- **Bundled music and SFX** — keep their own licenses; see [THIRD_PARTY.md](THIRD_PARTY.md)

Kevin MacLeod (incompetech.com), CC BY 4.0 — credit him if you share a recording of a session.
