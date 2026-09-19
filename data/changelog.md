# Blightnet changelog

The deck writes here so you can see what changed after an update. Newest first.

## 2026-09-19 — Windows and Linux table

- Mic downsample uses the real device rate (WASAPI is often 44.1 kHz, not 48 kHz). I32 capture works too.
- Windows: cameras via FFmpeg DirectShow, screen via gdigrab, files open with `start`, internet Host downloads cloudflared without a Unix-only curl, no extra console windows.
- `start.bat` rebuilds whenever cargo is on PATH so a stale exe is not launched after a code change.
- Linux file open still uses xdg-open; hardware lists still skip Pulse/PipeWire aliases.

## 2026-09-19 — Map fog, big files, permanent chat

- Gamemaster map tools: Ink, filled Circle, filled Square to fog the board. Erase click and Clear drawings. Marks sync to the table.
- File send cap is about 96 MB. Any file type. Incoming media streams automatically (picture in chat, audio plays, video opens). Every clip has Download.
- Chat is saved on this deck (`data/chat.json`) and is not trimmed. Wipe chat clears it.

## 2026-09-19 — Nethooks tab, deck player, Update on the rail

- Tabs are INDEX, TABLE, and NETHOOKS.
- UPDATE is a filled cyan button on the top bar in the DECK cluster, next to NET and TALK. Device lists still live on INDEX.
- PLAY is a local music player on that same bar: Prev / Play / Pause / Next / Library. Add files or a folder from this machine. Independent of the table mix and radio. Library is saved in data/deck-library.json.
- Chrome: cyan inner frame, hatch on the title bar, corner ticks on NET/TALK clusters, cyan tick on the active tab.

## 2026-09-19 — Zoom, talk media, Nethooks

- Click catalog, NPC, lore, kit, character, or chat pictures to zoom. Click the overlay or press Escape to close.
- NET is a framed cluster: Host, Join, Online. TALK is a framed cluster: Chat, Contacts, Voice, Video. Location is labeled LOC so it is not a second NET.
- Chat sends pictures, audio files, video files, and a recorded voice note (Record / Stop).
- INDEX NETHOOKS opens user sites on the grid. Write simple HTML in the built-in editor. Same INDEX chrome. Host, Join, or Go Online and every table picks up new pages. Only the original author can edit or delete theirs. Saved under data/nethooks.

## 2026-09-19 — Table navigation, combat log, private notes

- Tabs are INDEX and TABLE. The top bar groups NET (Host/Join/Online) and TALK (Chat/Contacts/Voice/Video). Table tools are labeled SHEET / BOOKS / GEAR / MORE. World (Hearthsong/Blight) lives on the top bar; seat (Player/GM) stays on the table.
- Combat log is back at the bottom of TABLE. Rolls, rests, and hits stay out of chat. The same log syncs to the table over the net.
- Directly under it: PRIVATE NOTES. Only this deck, keyed by Handle, saved in data/notes.json. Other players never see them.
- Jack-in glyph atlas is cached; the city caps its ray grid so the table stays near 60 FPS while you walk Night City.

## 2026-09-19 — Readable deck, hardware audio, living Night City

- INDEX DECK LOG reads this file. New work is appended here so every table sees it.
- Mic and speaker lists now show **hardware** (the card or USB box Pulse describes), not Pulse/PipeWire/ALSA software aliases. UPDATE rescans. Monitors are hidden.
- Gear, Armory, Vendors, and catalogs show full item text again — cost, damage, properties, weight, rate of fire, and the long description — at a readable size. Scroll the panel instead of shrinking type.
- Character sheet Gear lists every field on the item, not just the name.
- House 21 is a felt table with real card faces, a hole card, and chip stacks.
- Blight radio adds more Night City stations on top of Riot, Gloom, Dusk, and Warehouse.
- Jack-in netspace has more building types (clinics, garages, ruins, diners, megatowers), trains, traffic lights, holograms, and wet-street reflections. Radar on the right still tracks you.

## 2026-09-19 — Jack-in city and INDEX chrome

- Jack-in is a walkable ASCII Night City (WASD, look-drag, Shift to run) in INDEX orange/cyan.
- Netspace uses a 2.5D raycaster: lots, courtyards, alleys, markets, ICE walls, neon signs.
- A right-hand RADAR tracks position, facing, and district.
- INDEX Update/Rescan was wired to refresh devices. Borderless window (no exclusive fullscreen) after the Wayland freeze.
- Frame loop capped so the deck cannot spin thousands of FPS and OOM the machine.

## 2026-09-18 — Native table

- Native Rust/egui table. No browser.
- Hearthsong and Blight worlds, mixer, catalogs, LAN/internet table, contacts, voice, video.
- GM Armory drag vs player Vendors. Equip boosts. Rests and death saves.
- Blight radio: Riot FM, Gloom Wire, Dusk Channel, Warehouse.
