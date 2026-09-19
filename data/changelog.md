# Blightnet changelog

The deck writes here so you can see what changed after an update. Newest first. One heading per day. Edit this file only when a build is about to be pushed to GitHub.

## 2026-09-19

- Native INDEX chrome: hairlines, cyan ticks, chamfer frames, layered window shadow, watch ticks, framed NET/TALK/PLAY clusters.
- Tabs are INDEX, TABLE, and NETHOOKS. Nethooks stay off the TABLE tool bar.
- UPDATE is a filled cyan button on the top bar in the DECK cluster, next to NET and TALK.
- PLAY is a local music player on that bar: Prev / Play / Pause / Next / Library. Independent of the table mix and radio.
- Click catalog, NPC, lore, kit, character, or chat pictures to zoom. Escape or click the overlay to close.
- Chat sends pictures, audio, video, any file (~96 MB), and recorded voice notes. Incoming media streams; Download keeps a copy. Chat is permanent until Wipe chat.
- INDEX Nethooks: user sites on the grid, built-in HTML editor, INDEX chrome. Host, Join, or Go Online to sync. Only the author can edit or delete. Saved under data/nethooks.
- Gamemaster map tools: Ink, filled Circle, filled Square to fog the board. Erase and Clear drawings. Marks sync to the table.
- Combat log is back at the bottom of TABLE. Rolls stay out of chat. PRIVATE NOTES under it are local-only, keyed by Handle.
- Table tools: SHEET / BOOKS / GEAR / MORE. World lives on the top bar; Player/GM seat stays on the table.
- Mic downsample uses the real device rate. Windows WASAPI is often 44.1 kHz. I32 capture works.
- Mic and speaker lists show hardware, not Pulse/PipeWire/ALSA aliases. UPDATE rescans. Monitors are hidden.
- Windows: cameras via FFmpeg DirectShow, screen via gdigrab, files open with `start`. `start.bat` rebuilds when cargo is on PATH.
- Gear, Armory, Vendors, catalogs, and the character sheet show full item text at a readable size.
- House 21 is a felt table with real card faces, a hole card, and chip stacks.
- Blight radio adds Night City stations on top of Riot, Gloom, Dusk, and Warehouse.
- Jack-in is a walkable ASCII Night City (WASD, look-drag, Shift to run) with more building types, trains, lights, holograms, cached glyph atlas, and a capped ray grid.
- Background node starts with the window. Status bar NODE is ACTIVE or OFFLINE. Online starts the node; press again to stop it.
- Internet tables are node-to-node: encrypted invite (`blightnet://key@addrs`), UDP hole punch, optional UPnP. No Cloudflare tunnel.
- Table wire is X25519 + ChaCha20-Poly1305. Chat, files, voice, video, mix, sheets, tokens, and maps sync. Master volume stays local.
- Seats on TABLE: click a name to open that player's sheet. Combat log and private notes dock on the right and scroll. Panels close by pressing the same button. Hover tips follow the cursor.
- NETSPACE fills the tab with a stable city palette and auto-walk along streets. INDEX 01 Blightnexus opens TABLE. Manifesto is the first pinned Nethook.

## 2026-09-18

- Native Rust/egui table. No browser.
- Hearthsong and Blight worlds, mixer, catalogs, LAN/internet table, contacts, voice, video.
- GM Armory drag vs player Vendors. Equip boosts. Rests and death saves.
- Blight radio: Riot FM, Gloom Wire, Dusk Channel, Warehouse.
