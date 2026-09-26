# Blightnet changelog

The deck writes here so you can see what changed after an update. Newest first. One heading per day. Edit this file only when a build is about to be pushed to GitHub.

## 2026-09-26

- Command bar tabs are INDEX, TABLE, NETHOOKS, NETSPACE, ROTN, TERMINAL, and RECON. CHAT, CONTACTS, VOICE, VIDEO, and PLAYER stay pinned on that row.
- The bar shows this computer: CPU, GPU or a dash, RAM, and free disk. The numbers stay on this machine.
- Online starts the node. It does not start when the window opens. The window X leaves the node running. INDEX 00 shuts the node down and closes. Press Online again, or run daemon-stop, to stop it without using 00.
- Document pages scroll inside the frame. The table center is tiles: scenes, mix, the board, maps, the pit, chess, and the other tools. The sheet, the combat log, and private notes stay on the sides. Notes stay on this computer.
- Player is local pictures, video, PDF, and music. Station dots show what is on air. Stop ends the station and leaves the table mix alone.
- NETSPACE is a walkable city: look, streets, doors, a map, and telephone booths. WASD, Shift to run, C to cruise.
- Nethooks can be a handout, a rumor, a job, or a lesson. Post to table, then Board. Lessons stay pinned.
- ROTN is a local fixer. It will only talk to a model on this computer. The terminal is a local shell. Neither is sent to the table.
- RECON is a local case desk for people and companies. Files stay in data/recon.json. Delete asks twice.
- Invites stay blightnet:// from node to node. No Cloudflare. Contact quality is a short round trip: clear, steady, slow, or poor.
- Linux can launch Blightnet-x86_64.AppImage. Keep audio/, assets/, and data/ beside the image.
- Rescan devices looks for mics, speakers, and cameras. Update pulls from GitHub.
- After Online, Host still offers a local table and an internet table. The invite names the port that is listening. Join stays on the Join panel until you are in.
- A voice or video call can add more contacts and crew members. Hang up drops you. The call ends when the last person leaves.
- Place, Calendar, and Calc are table rail buttons. Place shows the painting. Calendar is a month. Calc stays on this computer.
- Recon files, nethooks you wrote, and character sheets have Send. Pick one contact or one crew. Pinned lessons do not send. Private notes stay here.
- The HTML and CSS lessons name what the painter keeps and what it cuts.
- ROTN answers on this computer with no other service. A model file is optional.
- Netspace streets, sidewalks, parks, water, and sky use ordinary colors. Acid and cyan stay on your own marks.
- New photographs for the castle, tavern, dungeon, forest, Hearthsong city, and Night City, each with morning, day, evening, and night.

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
- Background node starts with the window. Status bar NODE is ACTIVE or OFFLINE. Online starts the node; press again to stop it. If the child process cannot bind, the node runs in-process so Online still works.
- Internet tables are node-to-node: encrypted invite (`blightnet://key@addrs`), UDP hole punch, optional UPnP. No Cloudflare tunnel.
- Table wire is X25519 + ChaCha20-Poly1305. Chat, files, voice, video, mix, sheets, tokens, and maps sync. Master volume stays local.
- Seats on TABLE: click a name to open that player's sheet. Combat log and private notes dock on the right and scroll. Panels close by pressing the same button. Hover tips follow the cursor.
- NETSPACE fills the tab with a stable city palette and auto-walk along streets. INDEX 01 Blightnexus opens TABLE. Manifesto is the first pinned Nethook.

## 2026-09-18

- Native Rust/egui table. No browser.
- Hearthsong and Blight worlds, mixer, catalogs, LAN/internet table, contacts, voice, video.
- GM Armory drag vs player Vendors. Equip boosts. Rests and death saves.
- Blight radio: Riot FM, Gloom Wire, Dusk Channel, Warehouse.
