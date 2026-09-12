# Blightnet

A **local** table for D&D and Cyberpunk RED: mix music and weather, drop maps, run character sheets, and host a LAN session. Nothing is sent to the cloud except the explicit “join this table” connection on your own network.

You do **not** open `index.html` as a file. Start the local server, then use the Blightnet window (or **http://127.0.0.1:8765**).

This repository is large (~1.2 GB) because the music, ambience, and art ship with the app so a clone is playable.

---

## Requirements

| OS | What you need |
| --- | --- |
| **Linux / macOS** | Python 3 (stdlib only). No pip packages. |
| **Windows** | `Blightnet.exe` in this folder. Python is optional. |

A Chromium-based browser (or the bundled window) is used for the UI. Firefox/Brave are **not** launched unless you pass `--system-browser`.

---

## Quick start

### Linux or macOS

```bash
git clone https://github.com/Steelworth/Blightnet.git
cd blightnet
./start.sh
```

Blightnet opens as its own window when it can. If not, the terminal prints the address (usually **http://127.0.0.1:8765**).

- `./start.sh --system-browser` — use Firefox/Chrome/Brave instead of the dedicated window
- `./start.sh --no-open` — server only

**If the page says to run start.sh first**, you opened the file from the folder. Close that tab. Run `./start.sh`.

### Windows

1. Copy the whole **blightnet** folder. Keep `Blightnet.exe`, `index.html`, `js`, `css`, `audio`, and `assets` together. You do **not** need `tools/` on Windows.
2. Double-click **`Blightnet.exe`** (or **`start.bat`**).
3. A console stays open (that is the server). The table tries to open in app mode. If that fails, go to **http://127.0.0.1:8765**.

Closing the console stops the table.

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

## Share a table (same network)

Each computer runs Blightnet and sets a **Handle** in the top bar.

1. Skip the wake sequence if you want. You are in Blightnet.
2. Type a name. On the host click **Host**. The bar shows something like `blightnet://192.168.1.20:8765`. The console also prints **table →**.
3. On the other computers click **Join** and type that address.
4. **Chat** is in the yellow bar. `/w Name text` is a whisper (only those two see the body).
5. Open **TABLE** when you want the mixer. **Add sound** uploads audio. The host keeps the file and sends it to guests on the LAN.

**Leave** disconnects. Your firewall may ask the first time — allow it on the **private** network. Do not put this port on the public internet. See [SECURITY.md](SECURITY.md).

---

## First click

1. Wait for the wake sequence, or click / press a key to skip it.
2. Set your **Handle**. **Host** or **Join**, and open **Chat**, from the top bar.
3. Click **TABLE** when you want the mixer.
4. The table unrolls in the Blightnet window. Back / INDEX return to the launch page without dropping the table.

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

**Update** reloads sounds and maps the host shared, if a new upload does not appear.

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
| “Run start.sh first” | Do not open `index.html` from the folder. Windows: `Blightnet.exe`. Linux/Mac: `./start.sh`. Then **http://127.0.0.1:8765** |
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
