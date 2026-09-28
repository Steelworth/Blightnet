use crate::audio::Mixer;
use crate::catalog::{load_list, Catalog, Layer};
use crate::chars::{Character, KitSpec};
use crate::dice::{Luck, Roll};
use crate::images::{self, TexCache};
use crate::maps::{self, MapBoard, MapOp, TokenSpec};
use crate::names::Names;
use crate::net::{self, Contact, NetEvent, NetHub, Role};
use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED, ORANGE, PANEL, RAIL};
use eframe::egui::{self, Color32, FontId, Rect, RichText, Vec2};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Wake at most every 16.67ms. Do not sleep on the UI thread to enforce this.
const FRAME: Duration = Duration::from_nanos(16_666_667);
const RECV_QUIET: Duration = Duration::from_secs(20);
const BOOT_HOLD: f32 = 1.6;
const BOOT_SKIP: f32 = 0.2;
const BOOT_STEPS: u32 = 5;
const COMBAT_MARK: &str = "\u{2060}C|";
const FILE_CAP: usize = 96 * 1024 * 1024;
const MAP_WIRE: &str = "__table-map";
const FILM_W: u32 = 960;
const FILM_H: u32 = 540;
const FILM_BYTES: usize = (FILM_W as usize) * (FILM_H as usize) * 3;

/// TX allowed when table/call voice is on, not muted, and net is live.
fn voice_tx_allowed(voice_on: bool, mute: bool, live: bool) -> bool {
    voice_on && !mute && live
}

/// RX allowed for remote PCM when not deafened (mute must not gate hearing).
fn voice_rx_allowed(from_self: bool, deaf: bool) -> bool {
    !from_self && !deaf
}


struct MediaJob {
    gen: u64,
    ok: bool,
    msg: String,
}

struct FilmSlot {
    rgb: std::sync::Mutex<Option<(u64, Vec<u8>)>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct PicEdit {
    turns: u8,
    flip: bool,
    bright: i32,
    contrast: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

struct PicDesk {
    edit: PicEdit,
    arm: bool,
    for_path: Option<PathBuf>,
    dirty: bool,
    gen: u64,
    mark: Instant,
}

struct PicDone {
    gen: u64,
    body: Result<PicBody, String>,
}

enum PicBody {
    Preview(Vec<u8>),
    Saved(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextMove {
    Index(usize),
    Next,
    Prev,
}

struct GifDone {
    gen: u64,
    path: PathBuf,
    set: Result<crate::deskfile::GifSet, String>,
}

enum TagMsg {
    Read(Result<(PathBuf, crate::deskfile::TagSet), String>),
    Wrote(Result<(), String>),
}

impl PicDesk {
    fn blank() -> Self {
        Self {
            edit: PicEdit::default(),
            arm: false,
            for_path: None,
            dirty: false,
            gen: 0,
            mark: Instant::now(),
        }
    }

    fn fresh(path: PathBuf) -> Self {
        let mut desk = Self::blank();
        desk.for_path = Some(path);
        desk
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShellPanel {
    None,
    Chat,
    Contacts,
    Voice,
    Video,
    Join,
    Host,
    Player,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Boot,
    Index,
    Table,
    Catalog(&'static str),
    Chars,
    Tutorial,
    Audio,
    Blackjack,
    Nethooks,
    Netspace,
    Rotn,
    Player,
    Terminal,
    Recon,
    Tree,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TourOpen {
    None,
    Panel(Overlay),
    Games,
    Book,
    Seat,
}

struct TourStep {
    title: &'static str,
    body: &'static str,
    page: Page,
    dock: ShellPanel,
    open: TourOpen,
}

const TOUR: &[TourStep] = &[
    TourStep {
        title: "INDEX",
        body: "The front desk. The world name switches Hearthsong and Blight. The mix goes quiet and Place resets. 01 opens the table. The tiles open Contacts, this manual, Chat, Voice, Blackjack, Video, and Nethooks. Stamp a Handle in the command bar. The deck log is what has already shipped.",
        page: Page::Index,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "ONLINE",
        body: "Online in the command bar starts the node. Press it again to stop it. The meters show CPU, GPU, memory, and free disk. Update pulls the latest build. Rescan devices looks for mics, speakers, and cameras. Closing the window leaves the node running. INDEX 00 shuts the window and the node. daemon-stop does the same from a terminal.",
        page: Page::Index,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "HOST",
        body: "After Online, Host offers a local table or an internet table. Copy the blightnet:// invite. Host does not start the node by itself. Leave drops the table. The node keeps running until you press Online again, or INDEX 00.",
        page: Page::Index,
        dock: ShellPanel::Host,
        open: TourOpen::None,
    },
    TourStep {
        title: "JOIN",
        body: "Join pastes the blightnet:// invite the host copied, then Connect. Join does not start the node. Press Online first. The same invite is refused if it is a web address.",
        page: Page::Index,
        dock: ShellPanel::Join,
        open: TourOpen::None,
    },
    TourStep {
        title: "TABLE",
        body: "The world name, the place, and the clock stay on top. Morning, Day, Evening, and Night set the hour. Outside and Inside change the painting. Player and GM choose who runs the day. Fade out, Fade in, and Silence are this table's loudness. The Local slider is only this computer. On Blight, the painting carries the radio. The left rail is Net, Stage, Sheet, Seat, Books, Gear, and More.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "SCENES",
        body: "Scenes are looks for this world. Pick one to set the place and the mix. A star is a mix you saved. Save this mix keeps the one playing now. Search finds a name.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Scenes),
    },
    TourStep {
        title: "MIX",
        body: "Mix is the table's own sound. Check a layer to play it, and the slider is that layer. Shuffle here picks table music. It is not the player's Shuffle. Add sound on the rail brings in a file from this computer.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Mix),
    },
    TourStep {
        title: "PLACE",
        body: "Place is the painting for where you are, at morning, day, evening, and night. The place name on the top bar opens the same picture. Switching worlds resets it.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Place),
    },
    TourStep {
        title: "CALENDAR",
        body: "The date on the top bar opens this month. Only the gamemaster changes the day. A new day restocks the stall. The watch on the top bar opens the clock. Run clock moves it while you are the GM. Past midnight the day rolls.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Calendar),
    },
    TourStep {
        title: "CALC",
        body: "Calc is a calculator on this computer. The keys never go to the table.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Calc),
    },
    TourStep {
        title: "SEAT",
        body: "Sheet and Seat open this character. Click the name again to close. Other people at the table are buttons under Seat. Dice, hits, and rests go to the log. Private notes stay on this computer.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Seat,
    },
    TourStep {
        title: "BOOKS",
        body: "Books are the lists for this world. Hearthsong has Bestiary, NPCs, and Gods. Blight has Datashard, Faces, Corps, Gangs, and Lore. This card opens the first book. Search finds a name. Drag a row onto a map.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Book,
    },
    TourStep {
        title: "ARMORY",
        body: "Armory is the full gear list. The gamemaster drags an item onto a sheet. Everyone else can look. Search finds a name.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Armory),
    },
    TourStep {
        title: "STALL",
        body: "The stall is Vendors on Hearthsong and Night Market on Blight. Stock is shuffled for the buyer, and nothing above their level is out. Players buy here. A new day restocks it.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Vendors),
    },
    TourStep {
        title: "MAPS",
        body: "Maps are for the gamemaster. Ink, Circle, and Square fog the board. Erase removes a mark. Clear drawings wipes them. Grid, cell size, and Fit frame the board. Drag a token. Alt-drag pans. Marks go to everyone at the table. Drop a picture onto the table to hang it as the map.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Maps),
    },
    TourStep {
        title: "LOG",
        body: "Log is the shared roll list. Dice, hits, and rests land here. Everyone at the table sees it. Press Log again to close.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Log),
    },
    TourStep {
        title: "NOTES",
        body: "Notes are private. They stay on this computer and never go to the table. Press Notes again to close.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Notes),
    },
    TourStep {
        title: "GAMES",
        body: "21 is the house game, also on the INDEX tile. On Blight, More adds Chess beside 21. This card opens the games for the world you are in.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Games,
    },
    TourStep {
        title: "CHAT",
        body: "Chat sits in the dock. Pick Table, a contact, or a crew. Send text, a picture, audio, video, a voice note, or any file. A file streams, with no size cap. The status line shows about how long it will take, and how long is left. Wipe chat clears the log. The log stays until you wipe it.",
        page: Page::Table,
        dock: ShellPanel::Chat,
        open: TourOpen::None,
    },
    TourStep {
        title: "CONTACTS",
        body: "Contacts are people you save. Add them, message them, call them, or start video when Online. Create crew groups several people. Join table uses a saved address. A filled dot means they are online.",
        page: Page::Table,
        dock: ShellPanel::Contacts,
        open: TourOpen::None,
    },
    TourStep {
        title: "VOICE",
        body: "Voice is table talk, or a private call from Contacts. Table voice, Mute, and Deaf are here. Mic send is how loud you go out, and it stays on this computer. The mic starts when you turn table voice on. Go Online first.",
        page: Page::Table,
        dock: ShellPanel::Voice,
        open: TourOpen::None,
    },
    TourStep {
        title: "AUDIO",
        body: "Audio page is the same mic send, on its own screen. Listen levels here never change someone else's table. Voice on the top row is where calls live.",
        page: Page::Audio,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "VIDEO",
        body: "Video calls a contact or a whole crew. Pick a camera, or Rescan cameras. On a call you can send the camera, share the screen, mute, or hang up. Accept answers an incoming call. Go Online first.",
        page: Page::Table,
        dock: ShellPanel::Video,
        open: TourOpen::None,
    },
    TourStep {
        title: "PLAYER",
        body: "The player is this computer's library. Stations sit at the top: acid means this deck is playing it, cyan is on air, red is off air. Then songs, video, pictures, gifs, PDF pages, and text. Shuffle and the seek bar are for audio and video. A gif loops. Pause holds the frame. A still picture can be rotated, flipped, cropped, and shifted in brightness and contrast. Save copy writes a new file. Save over asks twice. Tags show the file. Save tags and Clear tags ask again. Restore last puts the backup back. Text can be edited. Send picks contacts and streams a file of any size. The status line shows about how long, and how long is left. None of this changes the table mix.",
        page: Page::Player,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "TREE",
        body: "TREE is the folders on this computer. This deck, Home, Computer, and Up. Open a folder to read it. A media file opens in the player. New file and New folder write here. Delete asks again, and a folder that is not empty asks once more. The top of the disk and this Blightnet folder stay. Nothing here is sent to the table.",
        page: Page::Tree,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "TERMINAL",
        body: "A real shell in this Blightnet folder. The wheel scrolls back. Typing returns to the live end. The list is every command on this computer. Click types the name. Enter runs it. Filter and Rescan narrow the list. Nothing typed here is sent to the table.",
        page: Page::Terminal,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "RECON",
        body: "Files on people and companies. Fill the form and keep a portrait, or a mark for a company. Send reaches one contact or a crew when Online. Delete asks again. The file stays on this computer until you send it.",
        page: Page::Recon,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "NETHOOKS",
        body: "Pages you can show the table: a handout, a rumor, or a job. The preview is live. Learn HTML and Learn CSS are pinned lessons. Post to table, then the next card opens Board. Take down removes it. Private notes never become a page.",
        page: Page::Nethooks,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "BOARD",
        body: "Board is what has been posted to this table. Open it from More on the left rail. If nothing is posted, write a page in NETHOOKS and press Post to table.",
        page: Page::Table,
        dock: ShellPanel::None,
        open: TourOpen::Panel(Overlay::Board),
    },
    TourStep {
        title: "NETSPACE",
        body: "A city you walk. WASD moves. Drag looks. Shift runs. Q and E turn. C auto-walks. M is the map. E is a booth. F is a door. Click looks at a mark. The NETSPACE tab or Jack-in on Blight opens it. When Online, other seats show under their handles. Acid and cyan stay on your own marks.",
        page: Page::Netspace,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
    TourStep {
        title: "ROTN",
        body: "The fixer knows this table and answers here. Hour, Table, and Place are the facts. Rumor, Job, and NPC are made up. You can type a question. Post rumor and Post job turn the last answer into a page you can put on the Board. Reroll soul makes a new fixer. Add model is optional and stays on this computer. None of that is sent away.",
        page: Page::Rotn,
        dock: ShellPanel::None,
        open: TourOpen::None,
    },
];

#[derive(Clone, PartialEq, Eq)]
enum ChatTarget {
    Table,
    Dm(String),
    Crew(String),
}

#[derive(Clone, Serialize, Deserialize)]
struct Crew {
    id: String,
    name: String,
    members: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct DevicePref {
    #[serde(default)]
    mic: String,
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    camera: String,
}

struct RemoteFeed {
    name: String,
    cam: Vec<u8>,
    screen: Vec<u8>,
}

struct InboxFile {
    mime: String,
    filename: String,
    path: PathBuf,
}

struct FileIn {
    from: String,
    name: String,
    mime: String,
    filename: String,
    size: u64,
    got: u64,
    path: PathBuf,
    file: Option<std::fs::File>,
    started: Instant,
    touched: Instant,
}

struct LiveSend {
    filename: String,
    who: String,
    index: u32,
    count: u32,
    done: u64,
    total: u64,
    started: Instant,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct InitRow {
    id: String,
    name: String,
    score: i32,
    ready: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct InitWire {
    rows: Vec<InitRow>,
    turn: usize,
}

fn pack_init(rows: &[InitRow], turn: usize) -> String {
    serde_json::to_string(&InitWire {
        rows: rows.to_vec(),
        turn,
    })
    .unwrap_or_else(|_| r#"{"rows":[],"turn":0}"#.into())
}

fn unpack_init(body: &str) -> Option<(Vec<InitRow>, usize)> {
    let w: InitWire = serde_json::from_str(body).ok()?;
    let turn = if w.rows.is_empty() {
        0
    } else {
        w.turn.min(w.rows.len().saturating_sub(1))
    };
    Some((w.rows, turn))
}

/// Aim victim wins; else open roster sheet if different from attacker.
fn resolve_roll_victim(aim: Option<&str>, open_id: Option<&str>, attacker_id: &str) -> String {
    if let Some(a) = aim {
        if !a.is_empty() && a != attacker_id {
            return a.to_string();
        }
    }
    if let Some(o) = open_id {
        if !o.is_empty() && o != attacker_id {
            return o.to_string();
        }
    }
    String::new()
}

/// Build combat init roster: living PCs plus map-token sheets (NPC or otherwise).
fn build_init_roster(chars: &[Character], tokens: &[maps::MapTok]) -> Vec<InitRow> {
    let mut rows: Vec<InitRow> = Vec::new();
    let mut seen = HashSet::new();
    for c in chars {
        if c.dead {
            continue;
        }
        if !c.npc {
            seen.insert(c.id.clone());
            rows.push(InitRow {
                id: c.id.clone(),
                name: c.name.clone(),
                score: 0,
                ready: false,
            });
        }
    }
    for tok in tokens {
        if tok.sheet.is_empty() {
            continue;
        }
        let sid = tok.sheet.as_str();
        if seen.contains(sid) {
            continue;
        }
        if let Some(c) = chars.iter().find(|c| c.id == sid) {
            if c.dead {
                continue;
            }
            seen.insert(c.id.clone());
            rows.push(InitRow {
                id: c.id.clone(),
                name: c.name.clone(),
                score: 0,
                ready: false,
            });
        } else {
            seen.insert(sid.to_string());
            rows.push(InitRow {
                id: sid.to_string(),
                name: tok.name.clone(),
                score: 0,
                ready: false,
            });
        }
    }
    rows
}

fn sort_init_roster(rows: &mut [InitRow]) {
    rows.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Overlay {
    None,
    Catalog,
    Chars,
    Maps,
    Blackjack,
    Chess,
    Armory,
    Vendors,
    Jackin,
    Scenes,
    Mix,
    Board,
    Place,
    Calendar,
    Calc,
    Log,
    Notes,
}

#[derive(Clone, Serialize, Deserialize)]
struct SavedMix {
    name: String,
    blight: bool,
    place: String,
    time: String,
    inside: bool,
    layers: HashMap<String, f32>,
}

pub struct Blightnet {
    root: PathBuf,
    catalog: Catalog,
    mixer: Mixer,
    page: Page,
    blight: bool,
    master: f32,
    time: &'static str,
    place: String,
    scene: String,
    search: String,
    cat_filter: String,
    catalog_q: String,
    catalog_rows: Vec<serde_json::Value>,
    catalog_pick: usize,
    handle: String,
    status: String,
    chat: Vec<String>,
    chat_in: String,
    chars: Vec<Character>,
    char_i: usize,
    dice: String,
    mic_gain: f32,
    err: String,
    bj: Bj,
    inside: bool,
    clock: u32,
    cal_y: i32,
    cal_m: u32,
    cal_d: u32,

    clock_run: bool,
    kit_rows: Vec<serde_json::Value>,
    kit_sig: String,
    watch_open: bool,
    place_open: bool,
    open: Vec<Overlay>,
    overlay_cat: &'static str,
    mix_search: String,
    mix_msg: String,
    saved: Vec<SavedMix>,
    held_mix: Option<HashMap<String, f32>>,
    map: MapBoard,
    radio_on: bool,
    radio_track: String,
    radio_station: String,
    radio_rx: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    air: HashMap<String, bool>,
    air_i: usize,
    air_at: Instant,
    air_rx: Option<std::sync::mpsc::Receiver<(String, bool)>>,
    player_full: bool,
    player_back: Page,
    media_page: i32,
    media_msg: String,
    media_run: bool,
    media_gen: u64,
    media_rx: Option<std::sync::mpsc::Receiver<MediaJob>>,
    media_child: Option<std::process::Child>,
    media_offset: f32,
    media_started: Instant,
    media_stamp: Option<std::time::SystemTime>,
    media_audio: Option<std::process::Child>,
    media_slot: Option<std::sync::Arc<FilmSlot>>,
    media_arx: Option<std::sync::mpsc::Receiver<Vec<f32>>>,
    pic: PicDesk,
    pic_rx: Option<std::sync::mpsc::Receiver<PicDone>>,
    vendor_stock: Vec<serde_json::Value>,
    custom: Vec<Layer>,
    kit_filter: String,
    kit_focus: String,
    tex: TexCache,
    net: NetHub,
    probe_n: u64,
    probe_at: Instant,
    probe_sent: HashMap<u64, Instant>,
    ping_ms: HashMap<String, u128>,
    ping_at: HashMap<String, Instant>,
    contacts: Vec<Contact>,
    shell: ShellPanel,
    join_in: String,
    voice_on: bool,
    voice_mute: bool,
    /// Stop hearing remote voice (independent of mute TX).
    voice_deaf: bool,
    media_at: Instant,
    incoming: Option<(String, String)>,
    call_id: Option<String>,
    call_with: Vec<String>,
    call_drop: Vec<String>,
    call_add: bool,
    whisper_to: Option<String>,
    mic_rx: Option<Receiver<Vec<f32>>>,
    mic_rate: u32,
    _mic: Option<cpal::Stream>,
    /// Opus encoder for live voice (20 ms @ 16 kHz). Created lazily.
    opus_enc: Option<crate::media::Encoder>,
    /// Accrues PCM until one Opus frame (320 samples).
    opus_buf: Vec<f32>,
    /// Acoustic echo canceller (Speex when linked).
    aec: crate::aec::Aec,
    /// Once-per-call notice when Opus encode falls back to Wire PCM (reset on hang-up).
    opus_fallback_noted: bool,
    crews: Vec<Crew>,
    crew_name: String,
    chat_target: ChatTarget,
    mic_name: String,
    speaker_name: String,
    inputs: Vec<crate::audio::AudioDev>,
    outputs: Vec<crate::audio::AudioDev>,
    inbox: HashMap<String, InboxFile>,
    file_in: HashMap<String, FileIn>,
    chat_saved: usize,
    is_gm: bool,
    target: Option<String>,
    /// Secondary Aim — combat HP victim (map `target` stays primary / attacker).
    aim: Option<String>,
    /// True while local INIT DragValue is focused — skip remote stomps.
    init_editing: bool,
    viewing: Option<String>,
    remote_chars: HashMap<String, Vec<Character>>,
    token_sig: u64,
    sheet_dirty: bool,
    sheet_at: Instant,
    mix_dirty: bool,
    mix_at: Instant,
    luck: Luck,
    roll: Option<Roll>,
    /// Session kill-credit Confirm plate (Track E). Not persisted.
    kill_credit: Option<crate::chars::KillCredit>,
    names: Names,
    jack_at: Instant,
    netspace: crate::netspace::Netspace,
    cameras: Vec<crate::video::Camera>,
    ffmpeg_ok: bool,
    cam_name: String,
    cam_on: bool,
    screen_on: bool,
    video_on: bool,
    cam_cap: Option<crate::video::Capture>,
    screen_cap: Option<crate::video::Capture>,
    local_cam: Vec<u8>,
    local_screen: Vec<u8>,
    remote_vid: HashMap<String, RemoteFeed>,
    incoming_video: Option<(String, String, Option<String>)>,
    call_crew: Option<String>,
    fps: f32,
    last_frame: Instant,
    visuals_on: bool,
    chrome: theme::ChromeTheme,
    theme_pick: bool,
    /// INDEX 09 trust ledger pane (readout only).
    index_ledger: bool,
    ui_scale: f32,
    node_live: bool,
    /// Last Host path-health Status line (UPnP/public IP/mesh). Kept when later status overwrites.
    path_health: String,
    node_at: Instant,
    clock_acc: f32,
    net_pos_at: Instant,
    tour: Option<usize>,
    mix_cache: Vec<(String, String)>,
    mix_cache_key: String,
    cat_cache: Vec<(usize, String, String)>,
    cat_cache_key: String,
    devices_on: bool,
    combat_log: Vec<String>,
    /// Session-local initiative tracker (TABLE combat rail). Not autopilot.
    init_rows: Vec<InitRow>,
    init_turn: usize,
    /// Attack index for map-token NPC Roll.
    map_atk_i: usize,
    notes: String,
    notes_dirty: bool,
    notes_at: Instant,
    changelog: Vec<(String, Vec<String>)>,
    zoom_path: Option<PathBuf>,
    zoom_key: Option<String>,
    rec_on: bool,
    rec: Vec<f32>,
    nethooks: Vec<crate::nethook::Nethook>,
    hook_i: usize,
    hook_edit: bool,
    hook_build: bool,
    hook_blocks: Vec<crate::nethook::Block>,
    hook_block_i: usize,
    hook_draft_title: String,
    hook_draft_html: String,
    hook_sound: String,
    hook_vid: Option<std::process::Child>,
    board_open: bool,
    tile_ratio: f32,
    deck_list: Vec<PathBuf>,
    deck_i: usize,
    deck_on: bool,
    deck_vol: f32,
    deck_shuffle: bool,
    deck_bag: Vec<usize>,
    deck_note: String,
    deck_base: f32,
    seek_drag: Option<f32>,
    len_cache: HashMap<PathBuf, f32>,
    len_miss: HashSet<PathBuf>,
    len_rx: Option<std::sync::mpsc::Receiver<(PathBuf, Option<f32>)>>,
    len_pending: Option<PathBuf>,
    tree_at: PathBuf,
    tree_rows: Vec<crate::tree::Entry>,
    tree_err: String,
    tree_sel: Option<PathBuf>,
    tree_name: String,
    tree_arm: Option<PathBuf>,
    tree_deep: bool,
    tree_drives: bool,
    tree_loaded: bool,
    tree_chrome: crate::tree::Chrome,
    tree_tags: crate::deskfile::TagSet,
    tree_tags_for: Option<PathBuf>,
    tree_tags_note: String,
    tree_tags_gen: u64,
    tree_tags_rx: Option<std::sync::mpsc::Receiver<(u64, Result<crate::deskfile::TagSet, String>)>>,
    tree_log_tail: String,
    tree_log_at: Instant,
    term_cwd: Option<PathBuf>,
    text_body: String,
    text_for: Option<PathBuf>,
    text_dirty: bool,
    text_hold: Option<TextMove>,
    gif_frames: Vec<crate::deskfile::GifFrame>,
    gif_for: Option<PathBuf>,
    gif_i: usize,
    gif_acc: f32,
    gif_run: bool,
    gif_note: String,
    gif_gen: u64,
    gif_rx: Option<std::sync::mpsc::Receiver<GifDone>>,
    tags_open: bool,
    tags_for: Option<PathBuf>,
    tags: crate::deskfile::TagSet,
    tags_arm: u8,
    tags_note: String,
    tags_gen: u64,
    tags_rx: Option<std::sync::mpsc::Receiver<(u64, TagMsg)>>,
    send_path: Option<PathBuf>,
    send_ids: HashSet<String>,
    send_rx: Option<std::sync::mpsc::Receiver<crate::net::SendNote>>,
    send_live: Option<LiveSend>,
    recv_focus: Option<String>,
    rotn: crate::rotn::Rotn,
    chess: crate::chess::Game,
    term: Option<crate::term::Shell>,
    term_filter: String,
    term_cmds: Vec<String>,
    share_pick: Option<(String, String, String)>,
    calc_acc: Option<f64>,
    calc_op: Option<char>,
    calc_entry: String,
    calc_fresh: bool,
    update_rx: Option<Receiver<Result<String, String>>>,
    recon: Vec<crate::recon::Dossier>,
    recon_i: usize,
    recon_q: String,
    recon_arm: String,
    cpu_tick: crate::sys::CpuTick,
    machine: crate::sys::Machine,
    meter_at: Instant,
}



struct Bj {
    player: Vec<u8>,
    dealer: Vec<u8>,
    bet: i32,
    bank: i32,
    msg: String,
    live: bool,
}

impl Bj {
    fn new() -> Self {
        Self {
            player: vec![],
            dealer: vec![],
            bet: 50,
            bank: 500,
            msg: "Ante up.".into(),
            live: false,
        }
    }
}

fn card() -> u8 {
    rand::thread_rng().gen_range(0u8..52)
}

fn card_rank(c: u8) -> u8 {
    c % 13 + 1
}

fn card_suit(c: u8) -> u8 {
    c / 13
}

fn total(h: &[u8]) -> u8 {
    let mut sum = 0u8;
    let mut aces = 0u8;
    for &c in h {
        match card_rank(c) {
            1 => {
                aces += 1;
                sum += 11;
            }
            11 | 12 | 13 => sum += 10,
            n => sum += n,
        }
    }
    while sum > 21 && aces > 0 {
        sum -= 10;
        aces -= 1;
    }
    sum
}

fn card_label(c: u8) -> &'static str {
    match card_rank(c) {
        1 => "A",
        11 => "J",
        12 => "Q",
        13 => "K",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        _ => "?",
    }
}

fn card_suit_mark(c: u8) -> (&'static str, Color32) {
    match card_suit(c) {
        0 => ("S", CREAM),
        1 => ("H", theme::HOT),
        2 => ("D", theme::HOT),
        _ => ("C", CREAM),
    }
}

fn paint_card(ui: &mut egui::Ui, c: u8, hole: bool, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if hole {
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(17, 17, 8));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(2.0, CYAN),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "BN",
            FontId::new(18.0, theme::display()),
            CYAN,
        );
        return;
    }
    ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(22, 18, 10));
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(2.0, CYAN),
        egui::StrokeKind::Inside,
    );
    let rank = card_label(c);
    let (suit, col) = card_suit_mark(c);
    ui.painter().text(
        rect.left_top() + Vec2::new(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        rank,
        FontId::new(18.0, theme::display()),
        col,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        suit,
        FontId::new(22.0, theme::display()),
        col,
    );
    ui.painter().text(
        rect.right_bottom() + Vec2::new(-8.0, -6.0),
        egui::Align2::RIGHT_BOTTOM,
        rank,
        FontId::new(16.0, theme::display()),
        col,
    );
}

struct Desk {
    root: PathBuf,
    catalog: Catalog,
    inputs: Vec<crate::audio::AudioDev>,
    outputs: Vec<crate::audio::AudioDev>,
    mic_name: String,
    speaker_name: String,
    cam_name: String,
    chat: Vec<String>,
    chars: Vec<Character>,
    saved: Vec<SavedMix>,
    contacts: Vec<Contact>,
    crews: Vec<Crew>,
    inbox: HashMap<String, InboxFile>,
    names: Names,
    combat_log: Vec<String>,
    notes: String,
    changelog: Vec<(String, Vec<String>)>,
    nethooks: Vec<crate::nethook::Nethook>,
    rotn: crate::rotn::Rotn,
    recon: Vec<crate::recon::Dossier>,
    net: NetHub,
    netspace: crate::netspace::Netspace,
}

impl Blightnet {
    fn prepare(root: PathBuf, mut note: impl FnMut(BootMsg)) -> Result<Desk, String> {
        let catalog = Catalog::load(&root)?;
        note(BootMsg::Step("catalog"));
        let pref = load_devices(&root);
        let inputs = crate::audio::list_inputs();
        let outputs = crate::audio::list_outputs();
        let mic_name = crate::audio::pick_listed(
            &inputs,
            &pref.mic,
            crate::audio::default_input_name(),
        );
        let speaker_name = crate::audio::pick_listed(
            &outputs,
            &pref.speaker,
            crate::audio::default_output_name(),
        );
        note(BootMsg::Step("devices"));
        note(BootMsg::NeedMixer(speaker_name.clone()));
        let chat = load_chat(&root).unwrap_or_else(|| {
            vec!["INDEX // stamp a Handle, then Host or Join, then JACK IN.".into()]
        });
        let mut chars = crate::chars::load(&root);
        if chars.is_empty() {
            let mut ada = Character::new("hearthsong");
            ada.name = "Ada".into();
            ada.hp = 12;
            ada.hp_max = 12;
            ada.gp = 80;
            chars.push(ada);
        }
        let saved = load_saved(&root);
        let contacts = net::load_contacts(&root);
        let crews = load_crews(&root);
        let inbox = load_inbox(&root);
        let names = Names::load(&root);
        let combat_log = load_combat_log(&root);
        let notes = load_notes(&root, "Traveller");
        let changelog = load_changelog(&root);
        let nethooks = crate::nethook::load_all(&root);
        let rotn = crate::rotn::Rotn::load(&root);
        let recon = crate::recon::load(&root);
        let net = NetHub::new("Traveller".into(), &root);
        note(BootMsg::Step("records"));
        let netspace = crate::netspace::Netspace::new();
        note(BootMsg::Step("city"));
        let cam_name = pref.camera;
        Ok(Desk {
            root,
            catalog,
            inputs,
            outputs,
            mic_name,
            speaker_name,
            cam_name,
            chat,
            chars,
            saved,
            contacts,
            crews,
            inbox,
            names,
            combat_log,
            notes,
            changelog,
            nethooks,
            rotn,
            recon,
            net,
            netspace,
        })
    }

    fn from_parts(desk: Desk, mixer: Mixer) -> Result<Self, String> {
        let Desk {
            root,
            catalog,
            inputs,
            outputs,
            mic_name,
            speaker_name,
            cam_name,
            chat,
            chars,
            saved,
            contacts,
            crews,
            inbox,
            names,
            combat_log,
            notes,
            changelog,
            nethooks,
            rotn,
            recon,
            net,
            netspace,
        } = desk;
        let place = catalog
            .settings(false)
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "tavern".into());
        let mut app = Self {
            mixer,
            catalog,
            root: root.clone(),
            page: Page::Boot,
            blight: false,
            master: 0.85,
            time: "day",
            place,
            scene: String::new(),
            search: String::new(),
            cat_filter: "all".into(),
            catalog_q: String::new(),
            catalog_rows: vec![],
            catalog_pick: 0,
            handle: "Traveller".into(),
            status: "Offline".into(),
            chat,
            chat_in: String::new(),
            chars,
            char_i: 0,
            dice: String::new(),
            mic_gain: 1.0,
            err: String::new(),
            bj: Bj::new(),
            inside: false,
            clock: 13 * 60,
            cal_y: 1492,
            cal_m: 3,
            cal_d: 9,

            clock_run: false,
            kit_rows: vec![],
            kit_sig: String::new(),
            watch_open: false,
            place_open: false,
            open: vec![],
            overlay_cat: "Bestiary",
            mix_search: String::new(),
            mix_msg: String::new(),
            saved,
            held_mix: None,
            map: MapBoard::default(),
            radio_on: false,
            radio_track: String::new(),
            radio_station: String::new(),
            radio_rx: None,
            air: HashMap::new(),
            air_i: 0,
            air_at: Instant::now(),
            air_rx: None,
            player_full: false,
            player_back: Page::Table,
            media_page: 1,
            media_msg: String::new(),
            media_run: false,
            media_gen: 0,
            media_rx: None,
            media_child: None,
            media_offset: 0.0,
            media_started: Instant::now(),
            media_stamp: None,
            media_audio: None,
            media_slot: None,
            media_arx: None,
            pic: PicDesk::blank(),
            pic_rx: None,
            vendor_stock: vec![],
            custom: vec![],
            kit_filter: String::new(),
            kit_focus: String::new(),
            tex: TexCache::default(),
            net,
            probe_n: 0,
            probe_at: Instant::now(),
            probe_sent: HashMap::new(),
            ping_ms: HashMap::new(),
            ping_at: HashMap::new(),
            contacts,
            shell: ShellPanel::None,
            join_in: String::new(),
            voice_on: false,
            voice_mute: false,
            voice_deaf: false,
            media_at: Instant::now(),
            incoming: None,
            call_id: None,
            call_with: Vec::new(),
            call_drop: Vec::new(),
            call_add: false,
            whisper_to: None,
            mic_rx: None,
            mic_rate: 48000,
            _mic: None,
            opus_enc: None,
            opus_buf: Vec::new(),
            aec: crate::aec::Aec::new(),
            opus_fallback_noted: false,
            crews,
            crew_name: String::new(),
            chat_target: ChatTarget::Table,
            mic_name,
            speaker_name,
            inputs,
            outputs,
            inbox,
            file_in: HashMap::new(),
            chat_saved: 0,
            is_gm: true,
            target: None,
            aim: None,
            init_editing: false,
            viewing: None,
            remote_chars: HashMap::new(),
            token_sig: 0,
            sheet_dirty: false,
            sheet_at: Instant::now(),
            mix_dirty: false,
            mix_at: Instant::now(),
            luck: Luck::Norm,
            roll: None,
            kill_credit: None,
            names,
            jack_at: Instant::now(),
            netspace,
            cameras: vec![],
            ffmpeg_ok: crate::sys::ffmpeg_ready(),
            cam_name,
            cam_on: false,
            screen_on: false,
            video_on: false,
            cam_cap: None,
            screen_cap: None,
            local_cam: vec![],
            local_screen: vec![],
            remote_vid: HashMap::new(),
            incoming_video: None,
            call_crew: None,
            fps: 60.0,
            last_frame: Instant::now(),
            visuals_on: false,
            chrome: theme::ChromeTheme::NeonDeck,
            theme_pick: false,
            index_ledger: false,
            ui_scale: UI_SCALE_DEFAULT,
            node_live: false,
            path_health: String::new(),
            node_at: Instant::now(),
            clock_acc: 0.0,
            net_pos_at: Instant::now(),
            tour: None,
            mix_cache: vec![],
            mix_cache_key: String::new(),
            cat_cache: vec![],
            cat_cache_key: String::new(),
            devices_on: false,
            combat_log,
            init_rows: Vec::new(),
            init_turn: 0,
            map_atk_i: 0,
            notes,
            notes_dirty: false,
            notes_at: Instant::now(),
            changelog,
            zoom_path: None,
            zoom_key: None,
            rec_on: false,
            rec: vec![],
            nethooks,
            hook_i: 0,
            hook_edit: false,
            hook_build: false,
            hook_blocks: Vec::new(),
            hook_block_i: 0,
            hook_draft_title: String::new(),
            hook_draft_html: String::new(),
            hook_sound: String::new(),
            hook_vid: None,
            board_open: false,
            tile_ratio: 0.5,
            deck_list: vec![],
            deck_i: 0,
            deck_on: false,
            deck_vol: 0.7,
            deck_shuffle: false,
            deck_bag: Vec::new(),
            deck_note: String::new(),
            deck_base: 0.0,
            seek_drag: None,
            len_cache: HashMap::new(),
            len_miss: HashSet::new(),
            len_rx: None,
            len_pending: None,
            tree_at: root.clone(),
            tree_rows: Vec::new(),
            tree_err: String::new(),
            tree_sel: None,
            tree_name: String::new(),
            tree_arm: None,
            tree_deep: false,
            tree_drives: false,
            tree_loaded: false,
            tree_chrome: crate::tree::Chrome::new(),
            tree_tags: crate::deskfile::TagSet::default(),
            tree_tags_for: None,
            tree_tags_note: String::new(),
            tree_tags_gen: 0,
            tree_tags_rx: None,
            tree_log_tail: String::new(),
            tree_log_at: Instant::now() - Duration::from_secs(5),
            term_cwd: None,
            text_body: String::new(),
            text_for: None,
            text_dirty: false,
            text_hold: None,
            gif_frames: Vec::new(),
            gif_for: None,
            gif_i: 0,
            gif_acc: 0.0,
            gif_run: false,
            gif_note: String::new(),
            gif_gen: 0,
            gif_rx: None,
            tags_open: false,
            tags_for: None,
            tags: crate::deskfile::TagSet::default(),
            tags_arm: 0,
            tags_note: String::new(),
            tags_gen: 0,
            tags_rx: None,
            send_path: None,
            send_ids: HashSet::new(),
            send_rx: None,
            send_live: None,
            recv_focus: None,
            rotn,
            chess: crate::chess::Game::new(),
            term: None,
            term_filter: String::new(),
            term_cmds: Vec::new(),
            share_pick: None,
            calc_acc: None,
            calc_op: None,
            calc_entry: "0".into(),
            calc_fresh: true,
            update_rx: None,
            recon,
            recon_i: 0,
            recon_q: String::new(),
            recon_arm: String::new(),
            cpu_tick: crate::sys::CpuTick::default(),
            machine: crate::sys::Machine::default(),
            meter_at: Instant::now() - Duration::from_secs(2),
        };
        app.chat_saved = app.chat.len();
        let (chrome, ui_scale) = load_chrome(&app.root);
        app.chrome = chrome;
        app.ui_scale = ui_scale;
        theme::set_chrome(app.chrome);
        app.deck_list = load_deck_lib(&app.root);
        if let Some(v) = load_deck_vol(&app.root) {
            app.deck_vol = v;
            app.mixer.set_deck_vol(v);
        }
        match images::probe(&app.root.join("assets/hearth.jpg")) {
            Ok(_) => {}
            Err(e) => app.err = format!("picture decoder: {e}"),
        }
        let paint = app.painting_path();
        if !paint.is_file() {
            app.err = format!("missing painting {}", paint.display());
        }
        if let Some((y, m, d)) = load_calendar(&app.root) {
            app.cal_y = y;
            app.cal_m = m;
            app.cal_d = d;
        }
        app.node_live = false;
        app.status = "Node offline".into();
        if let Err(e) = crate::audio::probe_ogg(&app.root, "audio/music/tavern_jig.ogg") {
            if app.err.is_empty() {
                app.err = e;
            }
        }
        Ok(app)
    }

    fn files(&self) -> HashMap<String, String> {
        let mut m: HashMap<String, String> = self
            .catalog
            .layers(self.blight)
            .iter()
            .filter_map(|l| l.audio_file().map(|f| (l.id.clone(), f.to_string())))
            .collect();
        for l in &self.custom {
            if let Some(f) = l.audio_file() {
                m.insert(l.id.clone(), f.to_string());
            }
        }
        m
    }

    fn pal(&self) -> theme::Palette {
        theme::index_palette()
    }

    fn layer_any(&self, id: &str) -> Option<Layer> {
        self.catalog
            .layer(self.blight, id)
            .cloned()
            .or_else(|| self.custom.iter().find(|l| l.id == id).cloned())
    }

    fn set_clock(&mut self, mins: u32) {
        self.clock = mins % 1440;
        let t = period_from_clock(self.clock);
        let period_changed = self.time != t;
        self.time = t;
        self.refresh_presence();
        if period_changed {
            self.broadcast_mix();
        }
    }

    fn date_stepper(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Y").family(theme::mono()).size(10.0).color(DIM));
        if theme::neon_btn(ui, "−y").clicked() {
            self.cal_y -= 1;
            save_calendar(&self.root, self.cal_y, self.cal_m, self.cal_d);
            self.restock_vendor();
        }
        ui.label(
            RichText::new(format!("{}", self.cal_y))
                .family(theme::mono())
                .size(13.0)
                .color(theme::ACID),
        );
        if theme::neon_btn(ui, "+y").clicked() {
            self.cal_y += 1;
            save_calendar(&self.root, self.cal_y, self.cal_m, self.cal_d);
            self.restock_vendor();
        }
        ui.label(RichText::new("M").family(theme::mono()).size(10.0).color(DIM));
        if theme::neon_btn(ui, "−m").clicked() {
            self.advance_month(-1);
        }
        ui.label(
            RichText::new(format!("{:02}", self.cal_m))
                .family(theme::mono())
                .size(13.0)
                .color(theme::HOT),
        );
        if theme::neon_btn(ui, "+m").clicked() {
            self.advance_month(1);
        }
        ui.label(RichText::new("D").family(theme::mono()).size(10.0).color(DIM));
        if theme::neon_btn(ui, "−d").clicked() {
            self.advance_day(-1);
        }
        ui.label(
            RichText::new(format!("{:02}", self.cal_d))
                .family(theme::mono())
                .size(13.0)
                .color(theme::NEON_RED),
        );
        if theme::neon_btn(ui, "+d").clicked() {
            self.advance_day(1);
        }
    }

    fn advance_month(&mut self, delta: i32) {
        let mut m = self.cal_m as i32 + delta;
        let mut y = self.cal_y;
        while m > 12 {
            m -= 12;
            y += 1;
        }
        while m < 1 {
            m += 12;
            y -= 1;
        }
        let dim = days_in_month(y, m);
        self.cal_y = y;
        self.cal_m = m as u32;
        if self.cal_d as i32 > dim {
            self.cal_d = dim as u32;
        }
        save_calendar(&self.root, self.cal_y, self.cal_m, self.cal_d);
        self.restock_vendor();
    }

    fn shift_clock(&mut self, delta: i32) {
        let mut m = self.clock as i32 + delta;
        let mut days = 0i32;
        while m >= 1440 {
            m -= 1440;
            days += 1;
        }
        while m < 0 {
            m += 1440;
            days -= 1;
        }
        self.clock = m as u32;
        if days != 0 {
            self.advance_day(days);
        }
        let t = period_from_clock(self.clock);
        let period_changed = self.time != t;
        self.time = t;
        self.refresh_presence();
        if period_changed {
            self.broadcast_mix();
        }
    }

    fn advance_day(&mut self, days: i32) {
        let mut n = self.cal_d as i32 + days;
        let mut m = self.cal_m as i32;
        let mut y = self.cal_y;
        while n > days_in_month(y, m) {
            n -= days_in_month(y, m);
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        while n < 1 {
            m -= 1;
            if m < 1 {
                m = 12;
                y -= 1;
            }
            n += days_in_month(y, m);
        }
        self.cal_y = y;
        self.cal_m = m as u32;
        self.cal_d = n as u32;
        save_calendar(&self.root, y, self.cal_m, self.cal_d);
        self.restock_vendor();
    }

    fn set_period(&mut self, time: &'static str) {
        self.set_clock(clock_from_period(time));
    }

    fn set_inside(&mut self, inside: bool) {
        self.inside = inside;
        self.refresh_presence();
        self.broadcast_mix();
    }

    fn presence_of(&self, id: &str) -> f32 {
        let Some(l) = self.layer_any(id) else {
            return 1.0;
        };
        let space = space_of(&l);
        let indoor = self.inside;
        let mut dry = match (indoor, space) {
            (true, Space::Out) => 0.52,
            (true, Space::In) => 1.0,
            (true, Space::Both) => 1.0,
            (false, Space::Out) => 1.0,
            (false, Space::In) => 0.48,
            (false, Space::Both) => 1.0,
        };
        let night = self.time == "night" || self.time == "evening";
        if night {
            dry *= 0.92;
        }
        dry
    }

    fn refresh_presence(&mut self) {
        let ids: Vec<String> = self.mixer.playing().map(|(id, _)| id.clone()).collect();
        for id in ids {
            if id.starts_with("__") {
                continue;
            }
            let p = self.presence_of(&id);
            self.mixer.set_presence(&id, p);
        }
    }

    fn apply_scene(&mut self, id: &str) {
        let Some(sc) = self
            .catalog
            .scenes(self.blight)
            .iter()
            .find(|s| s.id == id)
            .cloned()
        else {
            return;
        };
        self.scene = sc.id.clone();
        let mood = sc.mood.to_lowercase();
        if mood.contains("night") {
            self.set_period("night");
        } else if mood.contains("dusk") || mood.contains("evening") {
            self.set_period("evening");
        } else if mood.contains("morning") || mood.contains("dawn") {
            self.set_period("morning");
        } else if mood.contains("day") {
            self.set_period("day");
        }
        if let Some(place) = self.catalog.settings(self.blight).iter().find(|s| s.id == self.place) {
            self.set_inside(place.indoor);
        }
        let files = self.files();
        match self.mixer.apply_scene(&sc.layers, &files) {
            Ok(()) => self.err.clear(),
            Err(e) => self.err = e,
        }
        self.refresh_presence();
        self.mix_msg.clear();
        self.broadcast_mix();
    }

    fn set_world(&mut self, blight: bool) {
        if self.blight == blight {
            return;
        }
        self.mixer.silence();
        self.blight = blight;
        self.scene.clear();
        self.place = self
            .catalog
            .settings(blight)
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| if blight { "nightcity".into() } else { "tavern".into() });
        self.err.clear();
        self.open.clear();
        self.mix_cache.clear();
        self.mix_cache_key.clear();
        self.radio_on = false;
        self.radio_track.clear();
        self.radio_station.clear();
        self.mixer.stop("__radio");
    }

    fn toggle_layer(&mut self, id: &str) {
        if self.mixer.is_on(id) {
            self.mixer.stop(id);
            self.broadcast_mix();
            return;
        }
        if let Some(l) = self.layer_any(id) {
            if let Some(file) = l.audio_file() {
                if let Err(e) = self.mixer.play(id, file, 0.45) {
                    self.err = e;
                } else {
                    let p = self.presence_of(id);
                    self.mixer.set_presence(id, p);
                    self.broadcast_mix();
                }
            } else {
                self.err = format!("{} has no audio file", l.name);
            }
        }
    }

    fn open_catalog(&mut self, name: &'static str, file: &str) {
        self.catalog_rows = load_list(&self.root, file);
        self.catalog_pick = 0;
        self.catalog_q.clear();
        self.page = Page::Catalog(name);
    }

    fn open_table_catalog(&mut self, name: &'static str, file: &str) {
        if self.panel_on(Overlay::Catalog) && self.overlay_cat == name {
            self.open.retain(|o| *o != Overlay::Catalog);
            return;
        }
        self.catalog_rows = load_list(&self.root, file);
        self.catalog_pick = 0;
        self.catalog_q.clear();
        self.overlay_cat = name;
        if !self.panel_on(Overlay::Catalog) {
            self.open.push(Overlay::Catalog);
        }
    }

    fn panel_on(&self, o: Overlay) -> bool {
        self.open.contains(&o)
    }

    fn toggle_overlay(&mut self, o: Overlay) {
        if let Some(i) = self.open.iter().position(|x| *x == o) {
            self.open.remove(i);
        } else {
            self.open.push(o);
            if o == Overlay::Vendors {
                self.restock_vendor();
            }
            if o == Overlay::Jackin {
                self.jack_at = Instant::now();
            }
        }
    }

    fn restock_vendor(&mut self) {
        let file = if self.blight {
            "red-kit.json"
        } else {
            "srd-kit.json"
        };
        let mut rows = load_list(&self.root, file);
        let lvl = self
            .chars
            .get(self.char_i)
            .map(|c| {
                if c.is_blight() {
                    c.role_rank.max(c.level)
                } else {
                    c.level
                }
            })
            .unwrap_or(1)
            .max(1);
        rows.retain(|v| crate::chars::item_min_level(v) <= lvl);
        rows.shuffle(&mut rand::thread_rng());
        if rows.len() > 22 {
            rows.truncate(22);
        }
        self.vendor_stock = rows;
        self.kit_sig.clear();
    }

    fn close_panel(&mut self, o: Overlay) {
        self.open.retain(|x| *x != o);
    }

    fn fade_mix(&mut self) {
        if self.mixer.playing().next().is_none() {
            if let Some(held) = self.held_mix.clone() {
                let files = self.files();
                let _ = self.mixer.apply_scene(&held, &files);
                self.refresh_presence();
                self.held_mix = None;
                self.mix_msg = "Last mix returned.".into();
                self.broadcast_mix();
            }
            return;
        }
        self.held_mix = Some(self.mixer.snapshot());
        self.mixer.silence();
        self.mix_msg = "Table faded. Fade in brings it back.".into();
        self.broadcast_mix();
    }

    fn shuffle_music(&mut self) {
        let playing: Vec<(String, f32)> = self
            .mixer
            .playing()
            .filter(|(id, _)| !id.starts_with("__"))
            .map(|(id, v)| (id.clone(), v))
            .collect();
        let live_music: Vec<Layer> = playing
            .iter()
            .filter_map(|(id, _)| self.layer_any(id))
            .filter(|l| l.category == "music")
            .collect();
        let preferred = live_music.first().map(|l| l.mood.clone()).unwrap_or_default();
        let exclude: Vec<String> = live_music.iter().map(|l| l.id.clone()).collect();
        let mut pool: Vec<Layer> = self
            .catalog
            .layers(self.blight)
            .iter()
            .filter(|l| l.category == "music" && !exclude.contains(&l.id))
            .cloned()
            .collect();
        if !preferred.is_empty() {
            let mooded: Vec<Layer> = pool
                .iter()
                .filter(|l| l.mood == preferred)
                .cloned()
                .collect();
            if !mooded.is_empty() {
                pool = mooded;
            }
        }
        if pool.is_empty() {
            self.mix_msg = "No other music of that mood.".into();
            return;
        }
        let pick = pool[rand::thread_rng().gen_range(0..pool.len())].clone();
        let vol = live_music
            .first()
            .and_then(|l| playing.iter().find(|(id, _)| id == &l.id).map(|(_, v)| *v))
            .unwrap_or(0.46);
        for l in &live_music {
            self.mixer.stop(&l.id);
        }
        if let Some(file) = pick.audio_file() {
            if let Err(e) = self.mixer.play(&pick.id, file, vol) {
                self.err = e;
            } else {
                let p = self.presence_of(&pick.id);
                self.mixer.set_presence(&pick.id, p);
                self.mix_msg = format!("Now {}", pick.name);
            }
        }
    }

    fn save_this_mix(&mut self) {
        let layers = self.mixer.snapshot();
        if layers.is_empty() {
            self.mix_msg = "Nothing to save.".into();
            return;
        }
        let name = if self.scene.is_empty() {
            format!(
                "Mix {}",
                self.saved.iter().filter(|s| s.blight == self.blight).count() + 1
            )
        } else {
            format!("{} (saved)", self.scene)
        };
        self.saved.push(SavedMix {
            name,
            blight: self.blight,
            place: self.place.clone(),
            time: self.time.to_string(),
            inside: self.inside,
            layers,
        });
        save_saved(&self.root, &self.saved);
        self.mix_msg = "Mix kept on this table.".into();
    }

    fn apply_saved(&mut self, i: usize) {
        let Some(mix) = self.saved.get(i).cloned() else {
            return;
        };
        if mix.blight != self.blight {
            self.set_world(mix.blight);
        }
        self.place = mix.place;
        self.inside = mix.inside;
        if let Some(t) = HOURS.iter().copied().find(|h| *h == mix.time.as_str()) {
            self.set_period(t);
        }
        let files = self.files();
        match self.mixer.apply_scene(&mix.layers, &files) {
            Ok(()) => self.err.clear(),
            Err(e) => self.err = e,
        }
        self.refresh_presence();
        self.scene.clear();
        self.mix_msg = format!("Loaded {}", mix.name);
    }

    fn add_sound_file(&mut self, path: PathBuf) {
        let Some(name) = path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()) else {
            return;
        };
        let dest_dir = self.root.join("audio/custom");
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest = dest_dir.join(&name);
        if path != dest {
            if let Err(e) = std::fs::copy(&path, &dest) {
                self.err = format!("copy sound: {e}");
                return;
            }
        }
        let id = format!(
            "custom_{}",
            name.replace([' ', '.', '/', '\\'], "_").to_lowercase()
        );
        let rel = format!("audio/custom/{name}");
        self.custom.push(Layer {
            id: id.clone(),
            name: name.clone(),
            category: "music".into(),
            mood: "custom".into(),
            file: rel.clone(),
            files: vec![],
            world: if self.blight {
                "blight".into()
            } else {
                "hearthsong".into()
            },
        });
        if let Err(e) = self.mixer.play(&id, &rel, 0.5) {
            self.err = e;
        } else {
            self.mix_msg = format!("Added {name}");
        }
    }

    fn pick_sound(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["ogg", "mp3", "wav", "flac", "opus"])
            .set_title("Add a sound to this table")
            .pick_file()
        {
            self.add_sound_file(path);
        }
    }

    fn pick_map(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["jpg", "jpeg", "png", "webp"])
            .set_title("Import a play map")
            .pick_file()
        {
            self.map.image = Some(path);
            self.map.tokens.clear();
            if !self.panel_on(Overlay::Maps) {
                self.open.push(Overlay::Maps);
            }
            self.push_map_image();
            if self.table_live() {
                self.net.send_map_tokens(vec![]);
            }
        }
    }

    fn cycle_radio(&mut self) {
        if !self.radio_on || self.radio_station.is_empty() {
            self.tune_station(crate::stations::all()[0].id);
            return;
        }
        let stations = crate::stations::all();
        let i = stations
            .iter()
            .position(|s| s.id == self.radio_station)
            .unwrap_or(0);
        let next = (i + 1) % (stations.len() + 1);
        if next == stations.len() {
            self.mixer.stop("__radio");
            self.radio_on = false;
            self.radio_track.clear();
            self.radio_station.clear();
        } else {
            self.tune_station(stations[next].id);
        }
    }

    fn tune_station(&mut self, id: &str) {
        let live = crate::stations::get(id).and_then(|s| s.url).is_some();
        if self.radio_on && self.radio_station == id && !live {
            self.play_radio_track();
            return;
        }
        self.radio_station = id.into();
        self.radio_rx = None;
        if live {
            self.mixer.stop("__radio");
            self.radio_on = true;
            self.radio_track.clear();
            self.fetch_live();
        } else {
            self.play_radio_track();
        }
    }

    fn fetch_live(&mut self) {
        if self.radio_rx.is_some() {
            return;
        }
        let Some(st) = crate::stations::get(&self.radio_station) else {
            return;
        };
        let Some(url) = st.url else {
            return;
        };
        let url = url.to_string();
        let id = st.id.to_string();
        let dir = self.root.join("data/radio-cache");
        let (tx, rx) = std::sync::mpsc::channel();
        self.radio_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(pull_live(&url, &dir, &id));
        });
    }

    fn poll_radio(&mut self) {
        if let Some(rx) = self.radio_rx.take() {
            match rx.try_recv() {
                Ok(Ok(path)) => {
                    if let Err(e) = self.mixer.play_once("__radio", &path, 0.5) {
                        self.err = e;
                        self.radio_on = false;
                        if !self.radio_station.is_empty() {
                            self.air.insert(self.radio_station.clone(), false);
                        }
                    } else {
                        self.radio_on = true;
                        if !self.radio_station.is_empty() {
                            self.air.insert(self.radio_station.clone(), true);
                        }
                    }
                }
                Ok(Err(e)) => {
                    if !self.radio_station.is_empty() {
                        self.air.insert(self.radio_station.clone(), false);
                    }
                    self.err = e;
                    self.radio_on = false;
                    self.radio_station.clear();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => self.radio_rx = Some(rx),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.err = "The station stopped.".into();
                    self.radio_on = false;
                }
            }
        }
        let live = crate::stations::get(&self.radio_station)
            .and_then(|s| s.url)
            .is_some();
        if self.radio_on && live && self.radio_rx.is_none() && self.mixer.voice_done("__radio") {
            self.fetch_live();
        }
        if let Some(rx) = self.air_rx.take() {
            match rx.try_recv() {
                Ok((id, on)) => {
                    self.air.insert(id, on);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => self.air_rx = Some(rx),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
            }
        }
        self.probe_next_station();
    }

    fn probe_next_station(&mut self) {
        if self.air_rx.is_some() || self.air_at.elapsed() < Duration::from_secs(4) {
            return;
        }
        let live: Vec<_> = crate::stations::all()
            .into_iter()
            .filter(|s| s.url.is_some())
            .collect();
        if live.is_empty() {
            return;
        }
        self.air_i %= live.len();
        let st = live[self.air_i];
        self.air_i += 1;
        self.air_at = Instant::now();
        let url = st.url.unwrap_or("").to_string();
        let id = st.id.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.air_rx = Some(rx);
        std::thread::spawn(move || {
            let ok = curl_bin(&url, 4).is_ok();
            let _ = tx.send((id, ok));
        });
    }

    fn local_station_has_tracks(&self, id: &str) -> bool {
        let Some(st) = crate::stations::get(id) else {
            return false;
        };
        if st.url.is_some() {
            return false;
        }
        self.catalog.layers(true).iter().any(|l| l.category == "music" && (st.mood.is_empty() || l.mood == st.mood))
    }

    fn air_color(&self, id: &str) -> Color32 {
        if self.radio_on && self.radio_station == id {
            return theme::ACID;
        }
        if let Some(on) = self.air.get(id) {
            return if *on { CYAN } else { KILL };
        }
        if crate::stations::get(id).and_then(|s| s.url).is_none() {
            return if self.local_station_has_tracks(id) { CYAN } else { KILL };
        }
        DIM
    }

    fn air_word(&self, id: &str) -> &'static str {
        if self.radio_on && self.radio_station == id {
            "playing"
        } else if self.air.get(id) == Some(&true) || self.local_station_has_tracks(id) {
            "on air"
        } else if self.air.get(id) == Some(&false) {
            "off air"
        } else if crate::stations::get(id).and_then(|s| s.url).is_none() {
            "off air"
        } else {
            "not checked"
        }
    }

    fn poll_meters(&mut self) {
        if self.meter_at.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.meter_at = Instant::now();
        self.machine = crate::sys::sample_machine(&mut self.cpu_tick, &self.root);
    }

    fn poll_probe(&mut self) {
        self.probe_sent.retain(|_, t| t.elapsed() < Duration::from_secs(8));
        if !self.node_live || self.probe_at.elapsed() < Duration::from_secs(2) {
            return;
        }
        let others = self.net.peers.iter().any(|p| p.id != self.net.self_id);
        if !others {
            return;
        }
        self.probe_at = Instant::now();
        self.probe_n = self.probe_n.wrapping_add(1);
        self.probe_sent.insert(self.probe_n, Instant::now());
        self.net.send_probe(self.probe_n);
    }

    fn link_readout(&self, id: &str) -> Option<(String, Color32)> {
        let ms = *self.ping_ms.get(id)?;
        let fresh = self
            .ping_at
            .get(id)
            .map(|t| t.elapsed() < Duration::from_secs(6))
            .unwrap_or(false);
        if !fresh {
            return Some((format!("{ms} ms · poor"), KILL));
        }
        let (word, col) = if ms < 80 {
            ("clear", theme::ACID)
        } else if ms < 160 {
            ("steady", CYAN)
        } else if ms < 300 {
            ("slow", DIM)
        } else {
            ("poor", KILL)
        };
        Some((format!("{ms} ms · {word}"), col))
    }

    fn stop_radio(&mut self) {
        self.mixer.stop("__radio");
        self.radio_on = false;
        self.radio_track.clear();
        self.radio_station.clear();
        self.radio_rx = None;
    }

    fn play_radio_track(&mut self) {
        let st = crate::stations::get(&self.radio_station);
        let mood = st.map(|s| s.mood).unwrap_or("melancholic");
        let needle = st.map(|s| s.needle).unwrap_or("");
        let needles: Vec<&str> = needle
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let hit = |l: &Layer| {
            if l.category != "music" {
                return false;
            }
            if l.mood != mood {
                return false;
            }
            if needles.is_empty() {
                return true;
            }
            let blob = format!("{} {}", l.id, l.name).to_lowercase();
            needles.iter().any(|n| blob.contains(n))
        };
        let mut tracks: Vec<Layer> = self
            .catalog
            .layers(true)
            .iter()
            .filter(|l| hit(l))
            .cloned()
            .collect();
        if tracks.is_empty() {
            tracks = self
                .catalog
                .layers(true)
                .iter()
                .filter(|l| l.category == "music" && l.mood == mood)
                .cloned()
                .collect();
        }
        if tracks.is_empty() {
            tracks = self
                .catalog
                .layers(true)
                .iter()
                .filter(|l| l.category == "music")
                .cloned()
                .collect();
        }
        if tracks.is_empty() {
            return;
        }
        tracks.shuffle(&mut rand::thread_rng());
        if let Some(cur) = tracks.iter().position(|l| l.id == self.radio_track) {
            if tracks.len() > 1 {
                tracks.remove(cur);
            }
        }
        let pick = &tracks[0];
        self.mixer.stop("__radio");
        if let Some(file) = pick.audio_file() {
            if let Err(e) = self.mixer.play("__radio", file, 0.42) {
                self.err = e;
                self.radio_on = false;
            } else {
                self.radio_on = true;
                self.radio_track = pick.id.clone();
                if self.radio_station.is_empty() {
                    self.radio_station = st
                        .map(|s| s.id.to_string())
                        .unwrap_or_else(|| pick.mood.clone());
                }
            }
        }
    }

    fn spawn_catalog_token(&mut self, spec: &TokenSpec) {
        if spec.cat.is_empty() || spec.src.is_empty() {
            return;
        }
        let file = match spec.cat.as_str() {
            "Bestiary" => "bestiary.json",
            "NPCs" => "srd-npcs.json",
            "Gods" => "gods.json",
            "Datashard" => "datashard.json",
            "Faces" => "npcs.json",
            "Gangs" => "gangs.json",
            "Corps" => "corps.json",
            _ => return,
        };
        let rows = load_list(&self.root, file);
        let Some(row) = rows
            .iter()
            .find(|v| v.get("id").and_then(|x| x.as_str()) == Some(spec.src.as_str()))
            .cloned()
        else {
            return;
        };
        let mut ch = crate::chars::sheet_from_catalog(&spec.cat, &row, &self.chars);
        if ch.portrait.is_empty() && !spec.image.is_empty() {
            let p = PathBuf::from(&spec.image);
            if p.is_file() {
                if let Ok(rel) = p.strip_prefix(&self.root) {
                    ch.portrait = rel.to_string_lossy().into();
                } else {
                    ch.portrait = spec.image.clone();
                }
            }
        }
        let id = ch.id.clone();
        self.chars.push(ch);
        if let Some(tok) = self.map.tokens.last_mut() {
            tok.sheet = id.clone();
            tok.name = self.chars.last().map(|c| c.name.clone()).unwrap_or(tok.name.clone());
        }
        self.char_i = self.chars.len() - 1;
        if !self.panel_on(Overlay::Chars) {
            self.open.push(Overlay::Chars);
        }
        crate::chars::save(&self.root, &self.chars);
        self.mark_sheets();
        self.mix_msg = format!("{} takes the field.", spec.name);
    }

    fn place_name(&self) -> String {
        self.catalog
            .settings(self.blight)
            .iter()
            .find(|s| s.id == self.place)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| self.place.clone())
    }

    fn painting_for(&self, id: &str) -> PathBuf {
        let dir = if self.blight {
            "assets/places-blight"
        } else {
            "assets/places"
        };
        let timed = self.root.join(dir).join(format!("{id}-{}.jpg", self.time));
        if timed.is_file() {
            timed
        } else {
            self.root.join(dir).join(format!("{id}-day.jpg"))
        }
    }

    fn painting_path(&self) -> PathBuf {
        let dir = if self.blight {
            "assets/places-blight"
        } else {
            "assets/places"
        };
        let t = self.time;
        let p = self.root.join(dir).join(format!("{}-{t}.jpg", self.place));
        if p.is_file() {
            return p;
        }
        let day = self.root.join(dir).join(format!("{}-day.jpg", self.place));
        if day.is_file() {
            day
        } else {
            self.root.join("assets/hearth.jpg")
        }
    }

    fn toggle_shell(&mut self, p: ShellPanel) {
        if self.shell == p {
            self.shell = ShellPanel::None;
        } else {
            self.shell = p;
        }
    }

    fn start_host(&mut self, internet: bool) {
        if !self.node_live {
            self.chat.push("Press Online first. The node stays off until you ask.".into());
            self.status = "Node offline".into();
            return;
        }
        self.net.host(internet, self.root.clone());
        self.net.internet = internet;
        self.path_health.clear();
        if internet {
            self.chat.push(
                "Table open. The node is punching an internet path. Friends paste the invite into Join — their node talks to yours.".into(),
            );
            self.status = "Hosting · building invite…".into();
        } else {
            self.chat.push(
                "Table open on the local network. Same-house friends Join with the invite you copy.".into(),
            );
            self.status = "Hosting · local network".into();
        }
        self.shell = ShellPanel::Host;
    }

    fn do_join(&mut self, addr: &str) {
        let Some(inv) = crate::crypt::parse_invite(addr) else {
            let msg = "Need a blightnet:// invite from Host.".to_string();
            self.err = msg.clone();
            self.chat.push(msg);
            return;
        };
        if inv.key.len() != crate::crypt::KEY_LEN {
            let msg = "That invite has no table key. Ask the host for a fresh blightnet://key@… invite.".to_string();
            self.err = msg.clone();
            self.chat.push(msg);
            return;
        }
        if !self.node_live {
            let msg = "Press Online first. The node stays off until you ask.".to_string();
            self.err = msg.clone();
            self.chat.push(msg);
            self.status = "Node offline".into();
            return;
        }
        self.err.clear();
        self.net.join(addr);
        let shown = crate::crypt::encode_invite(&inv.key, &inv.addrs);
        self.chat.push(format!("Joining {shown}…"));
    }

    fn leave_table(&mut self) {
        self.hang_up();
        self.net.leave();
        self.path_health.clear();
        self.chat.push("Left the table.".into());
    }

    fn send_chat_line(&mut self) {
        let line = self.chat_in.trim().to_string();
        if line.is_empty() {
            return;
        }
        self.chat_in.clear();
        if !self.net.presence && self.net.role == Role::Idle {
            self.chat.push("Go Online, or Host / Join, to talk across machines.".into());
            return;
        }
        match &self.chat_target {
            ChatTarget::Dm(id) => {
                self.net.send_chat(&line, Some(id.clone()), true);
                return;
            }
            ChatTarget::Crew(cid) => {
                if let Some(crew) = self.crews.iter().find(|c| &c.id == cid).cloned() {
                    self.chat.push(format!("you → crew {}: {line}", crew.name));
                    for mid in &crew.members {
                        if self.contact_online(mid) {
                            self.net.send_chat(&line, Some(mid.clone()), true);
                        }
                    }
                }
                return;
            }
            ChatTarget::Table => {}
        }
        let (whisper, to, text) = if let Some(rest) = line.strip_prefix("/w ").or_else(|| line.strip_prefix("/whisper ")) {
            let rest = rest.trim();
            if let Some(id) = &self.whisper_to {
                (true, Some(id.clone()), rest.to_string())
            } else if let Some((name, body)) = rest.split_once(' ') {
                let hit = self
                    .net
                    .peers
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                    .map(|p| p.id.clone());
                if let Some(id) = hit {
                    (true, Some(id), body.to_string())
                } else {
                    self.chat.push(format!("No one named {name} at this table."));
                    return;
                }
            } else {
                self.chat.push("Whisper with /w Name message.".into());
                return;
            }
        } else if let Some(id) = &self.whisper_to {
            (true, Some(id.clone()), line)
        } else {
            (false, None, line)
        };
        self.net.send_chat(&text, to, whisper);
    }

    fn add_contact(&mut self, id: &str, name: &str) {
        if self.contacts.iter().any(|c| c.id == id) {
            self.chat.push(format!("{name} is already a contact."));
            return;
        }
        let addr = if self.net.role == Role::Guest {
            self.net.join_addr.clone()
        } else {
            self.net.paste_link()
        };
        self.contacts.push(Contact {
            id: id.to_string(),
            name: name.to_string(),
            addr: addr.clone(),
        });
        self.contacts
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        net::save_contacts(&self.root, &self.contacts);
        self.chat.push(format!("Saved {name} to contacts."));
        if self.net.presence {
            self.net.dial(&addr);
        }
    }

    fn go_online(&mut self) {
        if !self.net.daemon || !crate::daemon::is_up(&self.root) {
            let handle = self.net.handle.clone();
            self.net = crate::net::NetHub::attach(handle, &self.root);
        }
        self.node_live = self.net.daemon && crate::daemon::is_up(&self.root);
        if !self.node_live {
            self.chat.push("Could not start the node. Check data/daemon.log.".into());
            self.status = "Node offline".into();
            return;
        }
        self.net.go_online();
        for c in &self.contacts {
            if !c.addr.is_empty() {
                self.net.dial(&c.addr);
            }
        }
        self.chat.push("Node is active. Contacts can see you and DM without a table invite.".into());
        self.status = "Node active".into();
        self.announce_hooks();
    }

    fn live_link(&self) -> bool {
        self.net.presence || self.net.role != Role::Idle
    }

    fn table_live(&self) -> bool {
        matches!(self.net.role, Role::Host | Role::Guest)
    }

    fn push_owned_hooks(&self) {
        if !self.live_link() {
            return;
        }
        for h in &self.nethooks {
            if h.owner_id == self.net.self_id && !h.pinned() {
                self.send_hook(h);
            }
        }
    }

    fn send_hook(&self, hook: &crate::nethook::Nethook) {
        self.net.send_nethook_put(hook.clone());
        for f in &hook.files {
            let Some(path) = crate::nethook::hook_file(&self.root, &hook.id, &f.name) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            if bytes.len() > FILE_CAP {
                continue;
            }
            self.net.send_file(
                None,
                None,
                &f.kind,
                &format!("__hook|{}|{}", hook.id, f.name),
                &bytes,
            );
        }
    }

    fn announce_hooks(&self) {
        self.push_owned_hooks();
        if self.live_link() {
            self.net.send_nethook_ask();
        }
    }

    fn apply_hook_put(&mut self, hook: crate::nethook::Nethook) {
        let id = hook.id.clone();
        let fresh = !self.nethooks.iter().any(|h| h.id == id);
        let title = hook.title.clone();
        let owner = hook.owner_name.clone();
        let posted = hook.posted;
        crate::nethook::merge(&mut self.nethooks, hook.clone());
        if posted {
            let mine = self.net.self_id.clone();
            let mut quiet = Vec::new();
            for h in &mut self.nethooks {
                if h.id != id && h.posted {
                    h.posted = false;
                    if h.owner_id == mine && !h.pinned() {
                        quiet.push(h.clone());
                    }
                }
            }
            for h in &quiet {
                crate::nethook::save_one(&self.root, h);
            }
            self.board_open = true;
            if !self.panel_on(Overlay::Board) {
                self.open.push(Overlay::Board);
            }
        }
        crate::nethook::ensure_pinned(&mut self.nethooks);
        if let Some(h) = self.nethooks.iter().find(|h| h.id == id) {
            crate::nethook::save_one(&self.root, h);
        }
        if fresh && hook.owner_id != self.net.self_id {
            self.chat.push(format!("Nethook online: {title} · {owner}"));
        }
    }

    fn apply_hook_del(&mut self, id: &str, owner_id: &str) {
        if id == crate::nethook::MANIFESTO_ID {
            return;
        }
        let ok = self
            .nethooks
            .iter()
            .any(|h| h.id == id && h.owner_id == owner_id && !h.pinned());
        if !ok {
            return;
        }
        self.nethooks.retain(|h| h.id != id);
        crate::nethook::delete_one(&self.root, id);
        crate::nethook::ensure_pinned(&mut self.nethooks);
        if self.hook_i >= self.nethooks.len() {
            self.hook_i = self.nethooks.len().saturating_sub(1);
        }
        self.hook_edit = false;
    }

    fn go_offline(&mut self) {
        self.hang_up();
        self.net.leave();
        if self.net.daemon {
            crate::daemon::stop(&self.root);
        }
        let handle = self.net.handle.clone();
        self.net = crate::net::NetHub::new(handle, &self.root);
        self.node_live = false;
        self.path_health.clear();
        self.chat.push("Node is offline.".into());
        self.status = "Node offline".into();
    }

    fn refresh_node(&mut self) {
        if self.node_at.elapsed() < Duration::from_millis(800) {
            return;
        }
        self.node_at = Instant::now();
        let up = crate::daemon::is_up(&self.root);
        let attached = self.net.daemon;
        let live = attached && up;
        if self.node_live && !live {
            self.node_live = false;
            if !up {
                self.net.daemon = false;
            }
            self.status = "Node offline".into();
        } else if live {
            self.node_live = true;
        }
    }

    fn contact_online(&self, id: &str) -> bool {
        self.net.is_seen(id)
    }

    fn send_chat_image(&mut self) {
        self.pick_chat_file(
            "Send a picture",
            "Image",
            &["jpg", "jpeg", "png", "webp"],
        );
    }

    fn send_chat_audio_file(&mut self) {
        self.pick_chat_file(
            "Send audio",
            "Audio",
            &["ogg", "mp3", "wav", "flac", "opus", "m4a", "aac"],
        );
    }

    fn send_chat_video_file(&mut self) {
        self.pick_chat_file(
            "Send video",
            "Video",
            &["mp4", "webm", "mov", "mkv", "avi"],
        );
    }

    fn send_chat_any_file(&mut self) {
        self.pick_chat_file("Send a file", "Any", &["*"]);
    }

    fn pick_chat_file(&mut self, title: &str, filter: &str, exts: &[&str]) {
        let mut dlg = rfd::FileDialog::new().set_title(title);
        if filter != "Any" {
            dlg = dlg.add_filter(filter, exts);
        }
        let Some(path) = dlg.pick_file() else {
            return;
        };
        if !path.is_file() {
            self.chat.push("Could not read that file.".into());
            return;
        }
        self.spawn_send(path, self.chat_targets());
    }

    fn chat_targets(&self) -> Vec<(Option<String>, String)> {
        match &self.chat_target {
            ChatTarget::Dm(id) => {
                let name = self
                    .contacts
                    .iter()
                    .find(|c| &c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| id.clone());
                vec![(Some(id.clone()), name)]
            }
            ChatTarget::Crew(cid) => self
                .crews
                .iter()
                .find(|c| &c.id == cid)
                .map(|c| {
                    c.members
                        .iter()
                        .filter(|id| self.contact_online(id))
                        .map(|id| {
                            let name = self
                                .contacts
                                .iter()
                                .find(|c| &c.id == id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| id.clone());
                            (Some(id.clone()), name)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            ChatTarget::Table => vec![(None, "the table".into())],
        }
    }

    fn spawn_send(&mut self, path: PathBuf, targets: Vec<(Option<String>, String)>) {
        if self.send_rx.is_some() {
            self.chat.push("A send is already running.".into());
            return;
        }
        if !self.node_live {
            self.chat.push("Press Online first. Then you can send it.".into());
            return;
        }
        if targets.is_empty() {
            self.chat.push("Pick a contact first.".into());
            return;
        }
        let tx = self.net.file_sender();
        let from = self.net.self_id.clone();
        let handle = if self.net.handle.trim().is_empty() {
            self.handle.clone()
        } else {
            self.net.handle.clone()
        };
        let (evt_tx, evt_rx) = std::sync::mpsc::channel();
        self.send_rx = Some(evt_rx);
        self.send_live = None;
        let count = targets.len() as u32;
        std::thread::spawn(move || {
            let filename = path
                .file_name()
                .map(|s| safe_filename(&s.to_string_lossy()))
                .unwrap_or_else(|| "file".into());
            let mime = mime_of(&path);
            for (i, (id, who)) in targets.iter().enumerate() {
                if let Err(err) = crate::net::stream_file(
                    &tx,
                    &from,
                    &handle,
                    id.clone(),
                    &path,
                    mime,
                    &filename,
                    who,
                    i as u32 + 1,
                    count,
                    &evt_tx,
                ) {
                    let _ = evt_tx.send(crate::net::SendNote::Finished(Err(err)));
                    return;
                }
            }
            let _ = evt_tx.send(crate::net::SendNote::Finished(Ok(())));
        });
    }

    fn ingest_outgoing(&mut self, mime: &str, filename: &str, bytes: Vec<u8>) {
        let tag = media_tag(mime);
        let key = format!("{tag}-out-{:08x}", rand::random::<u32>());
        match &self.chat_target {
            ChatTarget::Dm(id) => {
                self.net
                    .send_file(Some(id.clone()), None, mime, filename, &bytes)
            }
            ChatTarget::Crew(cid) => {
                if let Some(crew) = self.crews.iter().find(|c| &c.id == cid).cloned() {
                    for mid in crew.members {
                        if self.contact_online(&mid) {
                            self.net
                                .send_file(Some(mid), None, mime, filename, &bytes);
                        }
                    }
                }
            }
            ChatTarget::Table => self.net.send_file(None, None, mime, filename, &bytes),
        }
        let line = media_caption(tag, "you");
        self.store_inbox(&key, mime, filename, &bytes);
        self.chat.push(format!("[{tag}:{key}|{filename}] {line}"));
    }

    fn send_voice_note(&mut self) {
        if self.rec.len() < 4_000 {
            self.chat.push("Recording too short.".into());
            self.rec.clear();
            return;
        }
        let wav = crate::audio::encode_wav(&self.rec, 16000);
        self.rec.clear();
        if wav.len() > FILE_CAP {
            self.chat.push("That recording is too large.".into());
            return;
        }
        self.ingest_outgoing("audio/wav", "voice.wav", wav);
    }

    fn remember_inbox(&mut self, key: &str, mime: &str, filename: &str, path: PathBuf) {
        self.inbox.insert(
            key.to_string(),
            InboxFile {
                mime: mime.into(),
                filename: filename.into(),
                path,
            },
        );
    }

    fn sweep_quiet_files(&mut self) {
        let quiet: Vec<String> = self
            .file_in
            .iter()
            .filter(|(_, f)| recv_is_quiet(f.touched.elapsed()))
            .map(|(id, _)| id.clone())
            .collect();
        for id in quiet {
            self.fail_recv(&id, "The file stopped.");
        }
    }

    fn fail_recv(&mut self, id: &str, msg: &str) {
        if let Some(f) = self.file_in.remove(id) {
            drop(f.file);
            let _ = std::fs::remove_file(&f.path);
        }
        if self.recv_focus.as_deref() == Some(id) {
            self.recv_focus = None;
        }
        self.status = msg.into();
    }

    fn drop_partials(&mut self) {
        let ids: Vec<String> = self.file_in.keys().cloned().collect();
        for id in ids {
            self.fail_recv(&id, "The file stopped.");
        }
    }

    fn store_inbox(&mut self, key: &str, mime: &str, filename: &str, bytes: &[u8]) {
        let dir = self.root.join("data/inbox");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(sanitize_key(key));
        let _ = std::fs::write(&path, bytes);
        self.inbox.insert(
            key.to_string(),
            InboxFile {
                mime: mime.into(),
                filename: filename.into(),
                path,
            },
        );
    }

    fn inbox_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let it = self.inbox.get(key)?;
        std::fs::read(&it.path).ok()
    }

    fn auto_stream(&mut self, tag: &str, bytes: &[u8]) {
        match tag {
            "aud" => self.mixer.play_bytes(bytes),
            "vid" => open_media(bytes, sniff_ext(bytes)),
            _ => {}
        }
    }

    fn download_inbox(&mut self, key: &str) {
        let Some(it) = self.inbox.get(key) else {
            return;
        };
        let name = it.filename.clone();
        let path = it.path.clone();
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name(&name)
            .set_title("Save file")
            .save_file()
        else {
            return;
        };
        match std::fs::copy(&path, &dest) {
            Ok(_) => self.chat.push(format!("Saved {}", dest.display())),
            Err(e) => self.chat.push(format!("Could not save: {e}")),
        }
    }

    fn wipe_chat(&mut self) {
        self.drop_partials();
        self.chat.clear();
        save_chat(&self.root, &self.chat);
        self.chat_saved = 0;
        self.chat.push("Chat wiped.".into());
    }

    fn set_speaker(&mut self, name: String) {
        self.speaker_name = name.clone();
        save_devices(
            &self.root,
            &DevicePref {
                mic: self.mic_name.clone(),
                speaker: name.clone(),
                camera: self.cam_name.clone(),
            },
        );
        let snap = self.mixer.snapshot();
        let master = self.mixer.master;
        if let Ok(mut m) = Mixer::with_output(self.root.clone(), Some(name.as_str()).filter(|s| !s.is_empty()))
        {
            m.master = master;
            m.set_master(master);
            m.set_deck_vol(self.deck_vol);
            let files = self.files();
            let _ = m.apply_scene(&snap, &files);
            self.mixer = m;
            self.refresh_presence();
            if self.deck_on {
                self.deck_play_current();
            }
        }
    }

    fn set_mic(&mut self, name: String) {
        self.mic_name = name.clone();
        save_devices(
            &self.root,
            &DevicePref {
                mic: name,
                speaker: self.speaker_name.clone(),
                camera: self.cam_name.clone(),
            },
        );
        self._mic = None;
        self.mic_rx = None;
        self.ensure_mic();
    }

    fn poll_net(&mut self) {
        for ev in self.net.poll() {
            match ev {
                NetEvent::Status(s) => {
                    if s.contains("Node link dropped") {
                        self.node_live = false;
                        self.net.daemon = false;
                        self.drop_partials();
                        self.path_health.clear();
                    }
                    if s.contains("path health") {
                        self.path_health = s.clone();
                    }
                    self.status = s;
                }
                NetEvent::Hosting { addrs, internet, .. } => {
                    self.net.internet = internet;
                    let lan = self.net.lan_link();
                    self.chat.push(format!("Same house can Join with {lan}"));
                    if !self.net.internet {
                        self.status = "Hosting · local network".into();
                    }
                    let _ = addrs;
                    self.announce_hooks();
                    if !self.map.marks.is_empty() {
                        self.net.send_map_marks(self.map.marks.clone());
                    }
                    if !self.map.tokens.is_empty() {
                        self.net.send_map_tokens(self.portable_tokens());
                    }
                    self.push_map_image();
                    self.push_mix_now();
                    self.push_sheets();
                }
                NetEvent::Relay { url } => {
                    self.chat.push(format!(
                        "Friends paste this invite into Join. Only Blightnet can read the table:\n{url}"
                    ));
                    self.status = "Hosting · invite ready".into();
                    self.shell = ShellPanel::Host;
                }
                NetEvent::Joined { addr } => {
                    self.err.clear();
                    self.chat.push(format!("Joined {addr}"));
                    self.status = "Joined".into();
                    self.page = Page::Table;
                    if self.shell == ShellPanel::Join {
                        self.shell = ShellPanel::None;
                    }
                    self.announce_hooks();
                    self.net.send_map_marks_ask();
                    self.net.send_map_tokens_ask();
                    self.net.send_map_image_ask();
                    self.net.send_sheet_ask();
                    self.net.send_pit("init", "ask");
                    self.push_sheets();
                }
                NetEvent::Left => {
                    self.status = "Offline".into();
                    self.voice_on = false;
                }
                NetEvent::Peers(_) => {
                    self.push_sheets();
                    self.push_mix_now();
                    if !self.map.tokens.is_empty() {
                        self.net.send_map_tokens(self.portable_tokens());
                    }
                    if !self.map.marks.is_empty() {
                        self.net.send_map_marks(self.map.marks.clone());
                    }
                    self.push_map_image();
                    self.push_init();
                }
                NetEvent::Chat {
                    from,
                    name,
                    text,
                    whisper,
                } => {
                    if let Some(rest) = text.strip_prefix(COMBAT_MARK) {
                        self.push_combat_silent(format!("{name} · {rest}"));
                    } else {
                        let tag = if whisper { "whisper" } else { "chat" };
                        self.chat.push(format!("{name} [{tag}]: {text}"));
                    }
                    let _ = from;
                }
                NetEvent::Voice {
                    from,
                    name,
                    action,
                    to,
                    crew,
                } => {
                    match action.as_str() {
                        "invite" if to.as_deref() == Some(&self.net.self_id) => {
                            self.incoming = Some((from, name.clone()));
                            if crew.is_some() {
                                self.call_crew = crew.clone();
                            }
                            self.chat.push(format!("{name} is calling."));
                            if self.shell != ShellPanel::Video {
                                self.shell = ShellPanel::Voice;
                            }
                        }
                        "roster" if to.as_deref() == Some(&self.net.self_id) => {
                            self.apply_roster(&name);
                        }
                        "accept" => {
                            self.remember_party(&from);
                            if crew.is_some() {
                                self.call_crew = crew.clone();
                            }
                            self.voice_on = true;
                            self.ensure_mic();
                            self.push_roster();
                            self.chat.push(format!("{name} picked up."));
                        }
                        "decline" | "hangup" => {
                            self.remote_vid.remove(&from);
                            self.call_with.retain(|id| id != &from);
                            if self.call_id.as_deref() == Some(&from) {
                                self.call_id = None;
                            }
                            if !self.call_drop.contains(&from) {
                                self.call_drop.push(from.clone());
                            }
                            if action == "decline" {
                                self.incoming = None;
                                self.incoming_video = None;
                            }
                            if self.call_targets().is_empty()
                                && self.incoming.is_none()
                                && self.incoming_video.is_none()
                            {
                                self.end_call_local();
                            }
                            self.chat.push(format!("{name}: {action}"));
                        }
                        "table-on" => self.chat.push(format!("{name} opened table voice.")),
                        "table-off" => self.chat.push(format!("{name} closed table voice.")),
                        _ => {}
                    }
                }
                NetEvent::VoicePcm { from, samples, to, .. } => {
                    if let Some(tid) = &to {
                        if tid != &self.net.self_id {
                            continue;
                        }
                    }
                    // Deaf = no RX. Mute must not silence remote voice.
                    if voice_rx_allowed(from == self.net.self_id, self.voice_deaf) {
                        self.aec.feed_playback(&samples);
                        self.mixer.play_pcm(samples, 16000);
                    }
                }
                NetEvent::Mix {
                    layers,
                    blight,
                    place,
                    time,
                    inside,
                } => {
                    let same = self.mixer.snapshot() == layers
                        && self.blight == blight
                        && self.place == place
                        && self.time == time
                        && self.inside == inside;
                    if same {
                        continue;
                    }
                    if self.blight != blight {
                        self.set_world(blight);
                    }
                    self.place = place;
                    self.inside = inside;
                    if let Some(t) = HOURS.iter().copied().find(|h| *h == time.as_str()) {
                        let clock = clock_from_period(t);
                        self.clock = clock;
                        self.time = t;
                    }
                    let files = self.files();
                    let _ = self.mixer.apply_scene(&layers, &files);
                    self.refresh_presence();
                }
                NetEvent::Share {
                    from,
                    name,
                    kind,
                    body,
                } => {
                    if from != self.net.self_id {
                        self.take_share(&name, &kind, &body);
                    }
                }
                NetEvent::Error(e) => {
                    self.err = e.clone();
                    self.chat.push(e);
                }
                NetEvent::Online { .. } => {
                    self.status = "Online".into();
                    for c in &self.contacts {
                        if !c.addr.is_empty() {
                            self.net.dial(&c.addr);
                        }
                    }
                    self.announce_hooks();
                }
                NetEvent::PeerSeen { id, name, addr } => {
                    if let Some(c) = self.contacts.iter_mut().find(|c| c.id == id) {
                        if !addr.is_empty() {
                            c.addr = addr.clone();
                        }
                        if !name.is_empty() {
                            c.name = name.clone();
                        }
                        net::save_contacts(&self.root, &self.contacts);
                    }
                    let label = if name.is_empty() { id } else { name };
                    self.chat.push(format!("{label} is online."));
                    self.push_owned_hooks();
                }
                NetEvent::Image {
                    from,
                    name,
                    mime,
                    data,
                    to,
                    ..
                } => {
                    if let Some(tid) = &to {
                        if tid != &self.net.self_id && from != self.net.self_id {
                            continue;
                        }
                    }
                    let tag = media_tag(&mime);
                    let key = format!("{tag}-inbox-{from}-{:08x}", rand::random::<u32>());
                    let filename = format!("clip.{tag}");
                    self.store_inbox(&key, &mime, &filename, &data);
                    self.chat.push(format!(
                        "[{tag}:{key}|{filename}] {}",
                        media_caption(tag, &name)
                    ));
                    self.auto_stream(tag, &data);
                    if from != self.net.self_id
                        && !self.contacts.iter().any(|c| c.id == from)
                    {
                        self.add_contact(&from, &name);
                    }
                }
                NetEvent::Video {
                    from,
                    name,
                    action,
                    to,
                    crew,
                } => {
                    match action.as_str() {
                        "invite" if to.as_deref() == Some(&self.net.self_id) => {
                            self.incoming_video = Some((from.clone(), name.clone(), crew.clone()));
                            self.incoming = Some((from, name.clone()));
                            self.call_crew = crew;
                            self.chat.push(format!("{name} is video calling."));
                            self.shell = ShellPanel::Video;
                        }
                        "accept" => {
                            self.remember_party(&from);
                            if crew.is_some() {
                                self.call_crew = crew;
                            }
                            self.video_on = true;
                            self.voice_on = true;
                            self.ensure_mic();
                            self.start_cam();
                            self.push_roster();
                            self.chat.push(format!("{name} joined the video call."));
                            self.shell = ShellPanel::Video;
                        }
                        "decline" | "hangup" => {
                            self.remote_vid.remove(&from);
                            self.call_with.retain(|id| id != &from);
                            if self.call_id.as_deref() == Some(&from) {
                                self.call_id = None;
                            }
                            if !self.call_drop.contains(&from) {
                                self.call_drop.push(from.clone());
                            }
                            if action == "decline" {
                                self.incoming_video = None;
                                self.incoming = None;
                            }
                            if self.call_targets().is_empty()
                                && self.incoming.is_none()
                                && self.incoming_video.is_none()
                            {
                                self.end_call_local();
                            }
                            self.chat.push(format!("{name}: video {action}"));
                        }
                        _ => {}
                    }
                }
                NetEvent::VideoFrame {
                    from,
                    name,
                    kind,
                    data,
                    ..
                } => {
                    if from == self.net.self_id {
                        continue;
                    }
                    let feed = self.remote_vid.entry(from).or_insert_with(|| RemoteFeed {
                        name: name.clone(),
                        cam: vec![],
                        screen: vec![],
                    });
                    feed.name = name;
                    if kind == "screen" {
                        feed.screen = data;
                    } else {
                        feed.cam = data;
                    }
                }
                NetEvent::NethookPut { hook } => self.apply_hook_put(hook),
                NetEvent::NethookDel { id, owner_id } => self.apply_hook_del(&id, &owner_id),
                NetEvent::NethookAsk => self.push_owned_hooks(),
                NetEvent::Sheet { from, chars } => {
                    if from != self.net.self_id {
                        let chars = chars
                            .into_iter()
                            .map(|mut c| {
                                c.portrait = images::portable_rel(&self.root, &c.portrait);
                                c.fullbody = images::portable_rel(&self.root, &c.fullbody);
                                c
                            })
                            .collect();
                        self.remote_chars.insert(from, chars);
                    }
                }
                NetEvent::SheetAsk => self.push_sheets(),
                NetEvent::MapTokens { tokens } => {
                    self.map.tokens = tokens
                        .into_iter()
                        .map(|mut t| {
                            t.image = images::portable_rel(&self.root, &t.image);
                            t
                        })
                        .collect();
                    self.token_sig = token_sig(&self.map.tokens);
                }
                NetEvent::MapTokensAsk => {
                    if !self.map.tokens.is_empty() {
                        self.net.send_map_tokens(self.portable_tokens());
                    }
                }
                NetEvent::MapImageAsk => self.push_map_image(),
                NetEvent::Probe { from, n } => {
                    if from != self.net.self_id {
                        self.net.send_probe_back(n);
                    }
                }
                NetEvent::ProbeBack { from, n } => {
                    if let Some(sent) = self.probe_sent.get(&n).copied() {
                        self.ping_ms.insert(from.clone(), sent.elapsed().as_millis());
                        self.ping_at.insert(from, Instant::now());
                    }
                }
                NetEvent::Pit { from, game, body } => {
                    if from != self.net.self_id {
                        self.apply_pit(&game, &body, &from);
                    }
                }
                NetEvent::NetPos { from, name, x, z, yaw } => {
                    if from != self.net.self_id {
                        self.netspace.note_person(name, x, z, yaw);
                    }
                }
                NetEvent::FileStart {
                    from,
                    name,
                    to,
                    mime,
                    filename,
                    id,
                    size,
                    ..
                } => {
                    if let Some(tid) = &to {
                        if tid != &self.net.self_id && from != self.net.self_id {
                            continue;
                        }
                    }
                    if from == self.net.self_id {
                        continue;
                    }
                    let special = filename.starts_with(MAP_WIRE) || filename.starts_with("__hook|");
                    if special && size > FILE_CAP as u64 {
                        self.chat.push(format!("{name} sent a file that is too large."));
                        continue;
                    }
                    let path = self.root.join("data/inbox").join(format!("partial-{id}"));
                    if std::fs::create_dir_all(self.root.join("data/inbox")).is_err() {
                        self.status = "The disk is full.".into();
                        continue;
                    }
                    let file = match std::fs::File::create(&path) {
                        Ok(file) => file,
                        Err(_) => {
                            self.status = "The disk is full.".into();
                            continue;
                        }
                    };
                    self.recv_focus = Some(id.clone());
                    self.file_in.insert(
                        id,
                        FileIn {
                            from,
                            name,
                            mime,
                            filename,
                            size,
                            got: 0,
                            path,
                            file: Some(file),
                            started: Instant::now(),
                            touched: Instant::now(),
                        },
                    );
                }
                NetEvent::FileChunk { id, data } => {
                    let mut failed = false;
                    let mut wrote = false;
                    if let Some(f) = self.file_in.get_mut(&id) {
                        match f.file.as_mut() {
                            Some(file) => match std::io::Write::write_all(file, &data) {
                                Ok(()) => {
                                    f.got += data.len() as u64;
                                    f.touched = Instant::now();
                                    wrote = true;
                                }
                                Err(_) => failed = true,
                            },
                            None => failed = true,
                        }
                    }
                    if wrote {
                        self.recv_focus = Some(id);
                    } else if failed {
                        self.fail_recv(&id, "The disk is full.");
                    }
                }
                NetEvent::FileStop { id } => {
                    self.fail_recv(&id, "The file stopped.");
                }
                NetEvent::FileDone { id } => {
                    if let Some(mut f) = self.file_in.remove(&id) {
                        f.file.take();
                        if f.size > 0 && f.got != f.size {
                            let _ = std::fs::remove_file(&f.path);
                            self.status = "The file stopped.".into();
                            if self.recv_focus.as_deref() == Some(id.as_str()) {
                                self.recv_focus = None;
                            }
                            continue;
                        }
                        let final_path = self.root.join("data/inbox").join(format!(
                            "{id}-{}",
                            crate::deskfile::safe_download_name(&f.filename)
                        ));
                        if std::fs::rename(&f.path, &final_path).is_err() {
                            let _ = std::fs::remove_file(&f.path);
                            self.status = "The file stopped.".into();
                            continue;
                        }
                        if self.recv_focus.as_deref() == Some(id.as_str()) {
                            self.recv_focus = None;
                        }
                        if f.filename.starts_with(MAP_WIRE) {
                            if let Ok(bytes) = std::fs::read(&final_path) {
                                self.install_map_image(&f.filename, &bytes);
                            }
                            let _ = std::fs::remove_file(&final_path);
                            continue;
                        }
                        if let Some(rest) = f.filename.strip_prefix("__hook|") {
                            if let Some((hid, name)) = rest.split_once('|') {
                                if let Some(name) = crate::nethook::safe_file_name(name) {
                                    if let Ok(bytes) = std::fs::read(&final_path) {
                                        let dir = crate::nethook::file_dir(&self.root, hid);
                                        let _ = std::fs::create_dir_all(&dir);
                                        let _ = std::fs::write(dir.join(name), &bytes);
                                    }
                                }
                            }
                            let _ = std::fs::remove_file(&final_path);
                            continue;
                        }
                        let tag = media_tag(&f.mime);
                        let key = id;
                        self.remember_inbox(&key, &f.mime, &f.filename, final_path.clone());
                        if is_library_file(Path::new(&f.filename)) {
                            self.add_deck_paths(vec![final_path.clone()]);
                            if matches!(media_kind(&final_path), "audio" | "video") {
                                if let Some(i) = self.deck_list.iter().position(|p| p == &final_path) {
                                    self.deck_i = i;
                                    self.deck_base = 0.0;
                                    self.media_offset = 0.0;
                                    self.deck_play_current();
                                }
                            }
                        }
                        self.chat.push(format!(
                            "[{tag}:{key}|{}] {}",
                            f.filename,
                            media_caption(tag, &f.name)
                        ));
                        self.status = "File received".into();
                        if f.from != self.net.self_id
                            && !self.contacts.iter().any(|c| c.id == f.from)
                        {
                            self.add_contact(&f.from, &f.name);
                        }
                    }
                }
                NetEvent::MapMark { mark } => {
                    if !self.map.marks.iter().any(|m| m.id == mark.id) {
                        self.map.marks.push(mark);
                    }
                }
                NetEvent::MapMarkDel { id } => {
                    self.map.marks.retain(|m| m.id != id);
                }
                NetEvent::MapMarks { marks } => {
                    self.map.marks = marks;
                }
                NetEvent::MapMarksClear => {
                    self.map.marks.clear();
                }
                NetEvent::MapMarksAsk => {
                    if !self.map.marks.is_empty() {
                        self.net.send_map_marks(self.map.marks.clone());
                    }
                }
            }
        }
    }

    fn ensure_mic(&mut self) {
        if self._mic.is_some() {
            return;
        }
        if let Some((stream, rx, rate)) = start_mic(if self.mic_name.is_empty() {
            None
        } else {
            Some(self.mic_name.as_str())
        }) {
            self._mic = Some(stream);
            self.mic_rx = Some(rx);
            self.mic_rate = rate.max(8000);
        } else {
            self.chat
                .push("No microphone found. Voice signaling still works.".into());
        }
    }

    fn pump_mic(&mut self) {
        let live = self.net.presence || self.net.role != Role::Idle;
        // Mute = no TX (do not spam silence keepalive PCM). Deaf is RX-only.
        let talk = voice_tx_allowed(self.voice_on, self.voice_mute, live);
        if self.rec_on {
            self.ensure_mic();
        }
        if !talk && !self.rec_on {
            return;
        }
        let Some(rx) = self.mic_rx.as_ref() else {
            return;
        };
        let mut acc: Vec<f32> = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            acc.extend(chunk);
            if acc.len() > 8000 {
                break;
            }
        }
        if acc.is_empty() {
            return;
        }
        let gain = self.mic_gain;
        for s in &mut acc {
            *s *= gain;
        }
        let down = downsample(&acc, self.mic_rate.max(8000), 16000);
        if self.rec_on {
            self.rec.extend_from_slice(&down);
            const MAX: usize = 16000 * 45;
            if self.rec.len() >= MAX {
                self.rec.truncate(MAX);
                self.rec_on = false;
                self.send_voice_note();
                return;
            }
        }
        if talk {
            // Opus 20 ms frames @ 16 kHz on binary media path (Phase A #2).
            self.opus_buf.extend_from_slice(&down);
            if self.opus_enc.is_none() {
                self.opus_enc = crate::media::Encoder::new();
                if self.opus_enc.is_none() && !self.opus_fallback_noted {
                    self.opus_fallback_noted = true;
                    self.chat.push(
                        "Voice: Opus encoder missing — using PCM fallback (this call).".into(),
                    );
                }
            }
            let frame_n = crate::media::FRAME_SAMPLES;
            while self.opus_buf.len() >= frame_n {
                let mut frame: Vec<f32> = self.opus_buf.drain(..frame_n).collect();
                // Phase A #3: AEC on capture before Opus (far-end = recent playout).
                self.aec.process_frame(&mut frame);
                let opus = match self.opus_enc.as_mut().and_then(|e| e.encode(&frame)) {
                    Some(p) => p,
                    None => {
                        // Encoder missing / failed — Wire PCM fallback keeps voice up.
                        if !self.opus_fallback_noted {
                            self.opus_fallback_noted = true;
                            self.chat.push(
                                "Voice: Opus encode unavailable — using PCM fallback (this call).".into(),
                            );
                        }
                        self.send_pcm_scoped(&frame);
                        continue;
                    }
                };
                self.send_opus_scoped(&opus);
            }
        }
    }

    fn send_opus_scoped(&self, opus: &[u8]) {
        let targets = self.call_targets();
        if targets.is_empty() {
            self.net.send_opus_to(opus, None, None);
        } else {
            let crew = self.call_crew.clone();
            for id in targets {
                self.net.send_opus_to(opus, Some(id), crew.clone());
            }
        }
    }

    fn send_pcm_scoped(&self, samples: &[f32]) {
        let targets = self.call_targets();
        if targets.is_empty() {
            self.net.send_pcm(samples);
        } else {
            let crew = self.call_crew.clone();
            for id in targets {
                self.net.send_pcm_to(samples, Some(id), crew.clone());
            }
        }
    }

    fn ensure_cameras(&mut self) {
        if !self.cameras.is_empty() {
            return;
        }
        self.cameras = crate::video::list_cameras();
        if self.cam_name.is_empty() {
            if let Some(c) = self.cameras.first() {
                self.cam_name = c.path.clone();
            }
        }
    }

    fn refresh_ffmpeg(&mut self) -> bool {
        self.ffmpeg_ok = crate::sys::ffmpeg_ready();
        self.ffmpeg_ok
    }

    fn ui_ffmpeg_missing(&mut self, ui: &mut egui::Ui, for_video: bool) {
        if self.ffmpeg_ok {
            return;
        }
        let what = if for_video {
            "Video call, camera, and screen share need FFmpeg on this computer."
        } else {
            "In-deck video needs FFmpeg on this computer. Open outside still works."
        };
        wrap_text(ui, what, CYAN, 12.0);
        wrap_text(ui, crate::sys::ffmpeg_install_tip(), MUTED, 11.0);
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Rescan").clicked() {
                if self.refresh_ffmpeg() {
                    self.cameras = crate::video::list_cameras();
                    if self.cam_name.is_empty() {
                        if let Some(c) = crate::video::default_camera() {
                            self.set_camera(c.path);
                        }
                    }
                    self.status = "FFmpeg found.".into();
                    if !for_video {
                        self.media_msg.clear();
                        if let Some(p) = self.deck_list.get(self.deck_i).cloned() {
                            if media_kind(&p) == "video" {
                                self.start_video(&p);
                            }
                        }
                    }
                } else {
                    self.status = "FFmpeg still missing.".into();
                }
            }
        });
    }

    fn ensure_devices(&mut self) {
        if self.devices_on {
            return;
        }
        self.devices_on = true;
        self.refresh_devices();
    }

    fn do_update(&mut self) {
        if self.update_rx.is_some() {
            self.chat.push("Update is already running.".into());
            return;
        }
        let root = self.root.clone();
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        self.status = "Updating…".into();
        self.chat.push("Updating from GitHub…".into());
        std::thread::spawn(move || {
            let _ = tx.send(github_update(&root));
        });
    }

    fn poll_update(&mut self) {
        let Some(rx) = self.update_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(text)) => {
                self.chat.push(format!(
                    "Pulled from GitHub.\n{text}\nRestart Blightnet to run the new build."
                ));
                self.status = "Update pulled".into();
            }
            Ok(Err(err)) => {
                self.chat.push(format!("Update failed. {err}"));
                self.status = "Update failed".into();
            }
            Err(TryRecvError::Empty) => {
                self.update_rx = Some(rx);
                self.status = "Updating…".into();
            }
            Err(TryRecvError::Disconnected) => {
                self.chat.push("Update failed. Git stopped.".into());
                self.status = "Update failed".into();
            }
        }
    }

    fn deck_track_name(&self) -> Option<String> {
        let p = self.deck_list.get(self.deck_i)?;
        Some(
            p.file_stem()
                .or_else(|| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| p.display().to_string()),
        )
    }

    fn current_kind(&self) -> &'static str {
        self.deck_list
            .get(self.deck_i)
            .map(|p| media_kind(p))
            .unwrap_or("audio")
    }

    fn stop_media_proc(&mut self) {
        self.media_arx = None;
        kill_child(&mut self.media_child);
        kill_child(&mut self.media_audio);
        self.mixer.film_stop();
    }

    fn poll_media_proc(&mut self) {
        let video_done = take_exit(&mut self.media_child);
        let audio_done = take_exit(&mut self.media_audio);
        if let Some(status) = audio_done {
            if self.media_run && self.media_child.is_some() && !status.success() && self.media_msg.is_empty()
            {
                self.media_msg = "The picture is playing. The sound did not start.".into();
            }
        }
        if let Some(status) = video_done {
            if self.media_run {
                if status.success() {
                    self.media_offset = 0.0;
                    if self.media_msg.starts_with("The picture is playing") {
                        self.media_msg.clear();
                    }
                } else if self.media_msg.is_empty() {
                    self.media_msg = "The video stopped.".into();
                }
                self.media_run = false;
                self.media_arx = None;
                kill_child(&mut self.media_audio);
                self.mixer.film_stop();
            }
        }
    }

    fn refresh_media_frame(&mut self, ctx: &egui::Context) {
        let dest = self.root.join("data/media-frame.png");
        let Ok(meta) = std::fs::metadata(&dest) else {
            return;
        };
        let modified = meta.modified().ok();
        if self.media_stamp.is_some() && modified == self.media_stamp {
            return;
        }
        let Ok(bytes) = std::fs::read(&dest) else {
            return;
        };
        if self.tex.put_bytes(ctx, "media-stage", &bytes).is_some() {
            self.media_stamp = modified;
        }
    }

    fn queue_pdf(&mut self, path: &Path) {
        self.stop_media_proc();
        self.media_run = false;
        self.media_gen = self.media_gen.wrapping_add(1);
        let gen = self.media_gen;
        let src = path.to_path_buf();
        let stem = self.root.join("data/media-frame");
        let page = self.media_page.max(1);
        let (tx, rx) = std::sync::mpsc::channel();
        self.media_rx = Some(rx);
        let _ = std::fs::create_dir_all(self.root.join("data"));
        std::thread::spawn(move || {
            let (ok, msg) = render_pdf(&src, &stem, page);
            let _ = tx.send(MediaJob { gen, ok, msg });
        });
    }

    fn start_video(&mut self, path: &Path) {
        self.stop_media_proc();
        self.media_rx = None;
        self.media_slot = None;
        let Some(bin) = crate::sys::ffmpeg_bin() else {
            self.ffmpeg_ok = false;
            self.media_run = false;
            self.media_msg =
                "FFmpeg is missing, so this deck cannot play the video here. Open outside still works."
                    .into();
            return;
        };
        self.ffmpeg_ok = true;
        self.tex.forget("media-stage");
        self.media_stamp = None;
        self.ask_len(path);
        let sec = format!("{:.2}", self.media_offset.max(0.0));
        let slot = std::sync::Arc::new(FilmSlot {
            rgb: std::sync::Mutex::new(None),
        });
        let mut video = match spawn_ffmpeg(&bin, &film_video_args(&sec, path)) {
            Ok(child) => child,
            Err(err) => {
                self.media_run = false;
                self.media_msg = err;
                return;
            }
        };
        if let Some(out) = video.stdout.take() {
            let slot_th = std::sync::Arc::clone(&slot);
            std::thread::spawn(move || pump_film_frames(out, slot_th));
        }
        self.media_child = Some(video);
        self.media_slot = Some(slot);
        let mut sound_note = String::new();
        match spawn_ffmpeg(&bin, &film_audio_args(&sec, path)) {
            Ok(mut audio) => {
                if let Some(out) = audio.stdout.take() {
                    let (tx, rx) = std::sync::mpsc::sync_channel(8);
                    self.media_arx = Some(rx);
                    std::thread::spawn(move || pump_film_audio(out, tx));
                }
                self.media_audio = Some(audio);
                self.mixer.film_start();
            }
            Err(_) => {
                sound_note = "The picture is playing. The sound did not start.".into();
            }
        }
        self.media_run = true;
        self.media_started = Instant::now();
        self.media_msg = sound_note;
    }

    fn pause_video(&mut self) {
        if self.media_run {
            self.media_offset += self.media_started.elapsed().as_secs_f32();
        }
        self.media_run = false;
        self.stop_media_proc();
    }

    fn step_media(&mut self, ctx: &egui::Context) {
        if let Some(rx) = self.media_rx.take() {
            match rx.try_recv() {
                Ok(job) => {
                    if job.gen == self.media_gen {
                        self.media_msg = job.msg;
                        if job.ok {
                            self.media_stamp = None;
                            self.tex.forget("media-stage");
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => self.media_rx = Some(rx),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if self.media_msg.is_empty() {
                        self.media_msg = "The file did not open.".into();
                    }
                }
            }
        }
        self.poll_pic(ctx);
        let kind = self.current_kind();
        if kind == "video" {
            self.drain_film_audio();
            self.pull_film_frame(ctx);
        }
        self.poll_media_proc();
        if kind == "pdf" {
            self.refresh_media_frame(ctx);
        }
    }

    fn drain_film_audio(&mut self) {
        let Some(rx) = self.media_arx.take() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(pcm) => self.mixer.film_push(pcm),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.media_arx = Some(rx);
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
            }
        }
    }

    fn pull_film_frame(&mut self, ctx: &egui::Context) {
        let Some(slot) = self.media_slot.clone() else {
            return;
        };
        let taken = slot.rgb.lock().ok().and_then(|mut guard| guard.take());
        if let Some((gen, rgb)) = taken {
            let _ = self.tex.put_rgb(ctx, "media-stage", FILM_W, FILM_H, &rgb, gen);
        }
    }

    fn poll_pic(&mut self, ctx: &egui::Context) {
        if self.current_kind() != "image" {
            return;
        }
        if self.pic.edit == PicEdit::default() {
            self.pic.dirty = false;
        }
        let Some(rx) = self.pic_rx.take() else {
            if self.pic.dirty && self.pic.mark.elapsed() > Duration::from_millis(180) {
                self.queue_pic(None);
            }
            return;
        };
        match rx.try_recv() {
            Ok(done) => {
                if done.gen == self.pic.gen {
                    match done.body {
                        Ok(PicBody::Preview(bytes)) => {
                            let _ = self.tex.put_bytes(ctx, "pic-stage", &bytes);
                        }
                        Ok(PicBody::Saved(path)) => self.finish_pic_save(path),
                        Err(err) => self.media_msg = err,
                    }
                }
                if self.pic.dirty && self.pic.mark.elapsed() > Duration::from_millis(180) {
                    self.queue_pic(None);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.pic_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if self.media_msg.is_empty() {
                    self.media_msg = "The picture edit stopped.".into();
                }
            }
        }
    }

    fn finish_pic_save(&mut self, path: PathBuf) {
        let key = path.to_string_lossy().to_string();
        self.tex.forget(&key);
        let replaced = self.pic.for_path.as_ref() == Some(&path);
        if replaced {
            self.pic.edit = PicEdit::default();
            self.pic.arm = false;
            self.pic.dirty = false;
            self.pic.gen = self.pic.gen.wrapping_add(1);
            self.tex.forget("pic-stage");
            self.media_msg = "Replaced the picture.".into();
            return;
        }
        self.pic.arm = false;
        self.media_msg = format!("Saved a copy: {}", path.display());
        self.add_deck_paths(vec![path]);
    }

    fn queue_pic(&mut self, save: Option<PathBuf>) {
        let Some(src) = self.pic.for_path.clone() else {
            return;
        };
        if save.is_none() && (self.pic_rx.is_some() || self.pic.edit == PicEdit::default()) {
            self.pic.dirty = false;
            return;
        }
        self.pic_rx = None;
        self.pic.dirty = false;
        let edit = self.pic.edit;
        let gen = self.pic.gen;
        let (tx, rx) = std::sync::mpsc::channel();
        self.pic_rx = Some(rx);
        std::thread::spawn(move || {
            let body = match bake_picture(&src, &edit, save.as_deref()) {
                Ok(bytes) => Ok(match save {
                    Some(path) => PicBody::Saved(path),
                    None => PicBody::Preview(bytes),
                }),
                Err(err) => Err(err),
            };
            let _ = tx.send(PicDone { gen, body });
        });
    }

    fn note_pic(&mut self, immediate: bool) {
        self.pic.arm = false;
        self.pic.dirty = true;
        self.pic.gen = self.pic.gen.wrapping_add(1);
        self.pic.mark = if immediate {
            Instant::now() - Duration::from_secs(2)
        } else {
            Instant::now()
        };
    }

    fn ui_pic_tools(&mut self, ui: &mut egui::Ui, path: &Path) {
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Rotate").clicked() {
                self.pic.edit.turns = (self.pic.edit.turns + 1) % 4;
                self.note_pic(true);
            }
            if theme::neon_btn(ui, if self.pic.edit.flip { "Unflip" } else { "Flip" }).clicked() {
                self.pic.edit.flip = !self.pic.edit.flip;
                self.note_pic(true);
            }
            if theme::neon_btn(ui, "Reset").clicked() {
                self.pic.edit = PicEdit::default();
                self.pic.arm = false;
                self.pic.dirty = false;
                self.pic.gen = self.pic.gen.wrapping_add(1);
                self.tex.forget("pic-stage");
            }
        });
        let mut bright = self.pic.edit.bright;
        if ui
            .add(egui::Slider::new(&mut bright, -100..=100).text("Brightness"))
            .changed()
        {
            self.pic.edit.bright = bright;
            self.note_pic(false);
        }
        let mut contrast = self.pic.edit.contrast;
        if ui
            .add(egui::Slider::new(&mut contrast, -100..=100).text("Contrast"))
            .changed()
        {
            self.pic.edit.contrast = contrast;
            self.note_pic(false);
        }
        wrap_text(ui, "Cut a percent off each edge.", MUTED, 11.0);
        let mut left = self.pic.edit.left;
        let mut top = self.pic.edit.top;
        let mut right = self.pic.edit.right;
        let mut bottom = self.pic.edit.bottom;
        if ui.add(egui::Slider::new(&mut left, 0..=40).text("Left")).changed() {
            self.pic.edit.left = left;
            self.note_pic(false);
        }
        if ui.add(egui::Slider::new(&mut top, 0..=40).text("Top")).changed() {
            self.pic.edit.top = top;
            self.note_pic(false);
        }
        if ui.add(egui::Slider::new(&mut right, 0..=40).text("Right")).changed() {
            self.pic.edit.right = right;
            self.note_pic(false);
        }
        if ui.add(egui::Slider::new(&mut bottom, 0..=40).text("Bottom")).changed() {
            self.pic.edit.bottom = bottom;
            self.note_pic(false);
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Save copy").clicked() {
                let dest = edit_copy_path(path);
                self.pic.gen = self.pic.gen.wrapping_add(1);
                self.queue_pic(Some(dest));
            }
            let label = if self.pic.arm { "Replace file" } else { "Save over" };
            if theme::neon_btn_color(ui, label, KILL, self.pic.arm).clicked() {
                if self.pic.arm {
                    let dest = path.to_path_buf();
                    self.pic.arm = false;
                    self.pic.gen = self.pic.gen.wrapping_add(1);
                    self.queue_pic(Some(dest));
                } else {
                    self.pic.arm = true;
                    self.media_msg = "Press Replace file to write over this picture.".into();
                }
            }
        });
    }

    fn stage_size(ui: &egui::Ui, full: bool) -> Vec2 {
        let w = ui.available_width().max(40.0);
        let avail = ui.available_height();
        let h = if !avail.is_finite() {
            if full { 360.0 } else { 180.0 }
        } else if full {
            (avail * 0.62).clamp(160.0, 640.0)
        } else {
            180.0_f32.min(avail.max(120.0))
        };
        Vec2::new(w, h)
    }

    fn paint_media_stage(&mut self, ui: &mut egui::Ui, full: bool) {
        let Some(p) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        let kind = media_kind(&p);
        if kind == "text" {
            self.paint_text(ui, &p);
            return;
        }
        if kind == "gif" {
            self.paint_gif(ui, full, &p);
            return;
        }
        if kind == "audio" {
            return;
        }
        if !self.media_msg.is_empty() {
            wrap_text(ui, &self.media_msg, CYAN, 12.0);
        }
        if kind == "video" && !self.ffmpeg_ok {
            self.ui_ffmpeg_missing(ui, false);
        }
        let max = Self::stage_size(ui, full);
        if kind == "image" {
            if self.pic.for_path.as_deref() != Some(p.as_path()) {
                self.pic = PicDesk::fresh(p.clone());
                self.pic_rx = None;
                self.tex.forget("pic-stage");
            }
            let path = p.clone();
            let edited = self.pic.edit != PicEdit::default();
            if edited {
                if let Some(tex) = self.tex.get_key("pic-stage") {
                    let _ = paint_contain(ui, &tex, max);
                } else if let Some(tex) = self.tex.get(ui.ctx(), &path) {
                    let _ = paint_contain(ui, &tex, max);
                } else {
                    let _ = images::show_fit(ui, &mut self.tex, &path, max);
                }
            } else if let Some(tex) = self.tex.get(ui.ctx(), &path) {
                if paint_contain(ui, &tex, max).clicked() {
                    self.zoom_path = Some(path.clone());
                }
            } else {
                let _ = images::show_fit(ui, &mut self.tex, &path, max);
            }
            self.ui_pic_tools(ui, &path);
        } else if kind == "video" && !self.ffmpeg_ok {
            // Tip + Rescan already shown above; skip the "Waiting for a frame…" placeholder.
        } else if let Some(tex) = self.tex.get_key("media-stage") {
            let _ = paint_contain(ui, &tex, max);
        } else {
            let (rect, _) = ui.allocate_exact_size(max, egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, PANEL);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if kind == "pdf" {
                    "Opening the page…"
                } else if kind == "video" {
                    "Waiting for a frame…"
                } else {
                    "Opening the picture…"
                },
                FontId::new(13.0, theme::mono()),
                DIM,
            );
        }
        ui.horizontal_wrapped(|ui| {
            if kind == "pdf" && theme::neon_btn(ui, "Prev page").clicked() && self.media_page > 1 {
                self.media_page -= 1;
                let path = p.clone();
                self.queue_pdf(&path);
            }
            if kind == "pdf" && theme::neon_btn(ui, "Next page").clicked() {
                self.media_page += 1;
                let path = p.clone();
                self.queue_pdf(&path);
            }
            if theme::neon_btn(ui, "Open outside").clicked() && !crate::sys::open_path(&p) {
                self.media_msg = "This computer did not open that file.".into();
            }
        });
    }

    fn deck_play_current(&mut self) {
        let Some(p) = self.deck_list.get(self.deck_i).cloned() else {
            self.deck_on = false;
            self.mixer.deck_stop();
            self.stop_media_proc();
            self.media_run = false;
            return;
        };
        let kind = media_kind(&p);
        if kind != "audio" {
            self.mixer.deck_stop();
            self.deck_on = false;
            self.media_msg.clear();
            self.media_page = 1;
            self.media_offset = 0.0;
            self.tex.forget("media-stage");
            self.media_stamp = None;
            self.gif_run = false;
            if kind == "video" {
                self.start_video(&p);
            } else if kind == "pdf" {
                self.queue_pdf(&p);
            } else if kind == "text" {
                self.stop_media_proc();
                self.media_run = false;
                self.load_text(&p);
            } else if kind == "gif" {
                self.stop_media_proc();
                self.media_run = false;
                self.load_gif(&p);
            } else {
                self.stop_media_proc();
                self.media_run = false;
                self.media_rx = None;
            }
            return;
        }
        self.gif_run = false;
        self.stop_media_proc();
        self.media_run = false;
        self.media_rx = None;
        self.media_msg.clear();
        self.seek_drag = None;
        let at = Duration::from_secs_f32(self.deck_base.max(0.0));
        let started = if at < Duration::from_millis(40) {
            self.mixer.deck_play_path(&p)
        } else {
            self.mixer.deck_play_at(&p, at, false)
        };
        match started {
            Ok(()) => {
                self.deck_on = true;
                self.mixer.set_deck_vol(self.deck_vol);
                if at < Duration::from_millis(40) {
                    self.note_len_now(&p);
                }
                self.ask_len(&p);
            }
            Err(e) => {
                self.deck_on = false;
                self.chat.push(format!("Player: {e}"));
            }
        }
    }

    fn deck_toggle(&mut self) {
        if self.deck_list.is_empty() {
            self.shell = ShellPanel::Player;
            self.chat.push("Add files to the player library.".into());
            return;
        }
        match self.current_kind() {
            "video" => {
                if self.media_run {
                    self.pause_video();
                } else if let Some(p) = self.deck_list.get(self.deck_i).cloned() {
                    self.start_video(&p);
                }
                return;
            }
            "gif" => {
                if !self.gif_frames.is_empty() {
                    self.gif_run = !self.gif_run;
                }
                return;
            }
            "image" | "pdf" | "text" => return,
            _ => {}
        }
        if self.deck_on && self.mixer.deck_live() {
            self.mixer.deck_pause();
            self.deck_on = false;
            return;
        }
        if self.mixer.deck_done() {
            self.deck_play_current();
        } else {
            self.mixer.deck_resume();
            self.deck_on = true;
        }
    }

    fn deck_next(&mut self, force: bool) {
        if self.deck_list.is_empty() {
            return;
        }
        if !self.gate_text(TextMove::Next) {
            return;
        }
        let keep = force || self.deck_on || self.media_run || self.gif_run;
        if self.deck_shuffle {
            self.step_shuffle(keep);
            return;
        }
        self.deck_i = (self.deck_i + 1) % self.deck_list.len();
        self.cue_from_start(keep);
        save_deck_lib(&self.root, &self.deck_list);
    }

    fn deck_prev(&mut self) {
        if self.deck_list.is_empty() {
            return;
        }
        if !self.gate_text(TextMove::Prev) {
            return;
        }
        let keep = self.deck_on || self.media_run || self.gif_run;
        if self.deck_shuffle {
            self.step_shuffle(keep);
            return;
        }
        if self.deck_i == 0 {
            self.deck_i = self.deck_list.len() - 1;
        } else {
            self.deck_i -= 1;
        }
        self.cue_from_start(keep);
        save_deck_lib(&self.root, &self.deck_list);
    }

    fn gate_text(&mut self, mv: TextMove) -> bool {
        if self.current_kind() != "text" || !self.text_dirty {
            self.text_hold = None;
            return true;
        }
        if self.text_hold == Some(mv) {
            self.text_dirty = false;
            self.text_hold = None;
            self.text_for = None;
            return true;
        }
        self.text_hold = Some(mv);
        self.deck_note = "This text is not saved. Save, or press Discard.".into();
        false
    }

    fn cue_from_start(&mut self, keep: bool) {
        self.deck_base = 0.0;
        self.media_offset = 0.0;
        self.seek_drag = None;
        if keep || self.current_kind() != "audio" {
            self.deck_play_current();
        }
    }

    fn step_shuffle(&mut self, keep: bool) {
        let playable = self.playable_indices();
        match draw_shuffle(&mut self.deck_bag, &playable, self.deck_i) {
            Some(i) => {
                self.deck_note.clear();
                self.deck_i = i;
                self.cue_from_start(keep);
            }
            None => {
                self.deck_note = "Nothing else to shuffle.".into();
                if self.mixer.deck_done() && !self.mixer.deck_loading() {
                    self.deck_on = false;
                }
            }
        }
    }

    fn playable_indices(&self) -> Vec<usize> {
        self.deck_list
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(media_kind(p), "audio" | "video"))
            .map(|(i, _)| i)
            .collect()
    }

    fn pump_deck(&mut self) {
        if self.deck_on
            && self.mixer.deck_done()
            && !self.mixer.deck_loading()
            && !self.deck_list.is_empty()
        {
            self.deck_next(true);
        }
    }

    fn poll_deck_ready(&mut self) {
        let Some(ready) = self.mixer.poll_deck_ready() else {
            return;
        };
        match ready {
            Ok(ready) => {
                self.deck_base = ready.base;
                if ready.pause {
                    self.deck_on = false;
                }
                if let Some(len) = ready.len {
                    self.note_len(&ready.path, len);
                } else {
                    self.ask_len(&ready.path);
                }
            }
            Err(err) => {
                self.deck_on = false;
                self.chat.push(format!("Player: {err}"));
            }
        }
    }

    fn note_len_now(&mut self, path: &Path) {
        if let Some(len) = self.mixer.take_known_len() {
            self.note_len(path, len);
        }
    }

    fn note_len(&mut self, path: &Path, secs: f32) {
        if secs.is_finite() && secs > 0.05 {
            self.len_cache.insert(path.to_path_buf(), secs);
            self.len_miss.remove(path);
        }
    }

    fn ask_len(&mut self, path: &Path) {
        if self.len_cache.contains_key(path) || self.len_miss.contains(path) {
            return;
        }
        if self.len_pending.as_deref() == Some(path) {
            return;
        }
        let path = path.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        self.len_rx = Some(rx);
        self.len_pending = Some(path.clone());
        std::thread::spawn(move || {
            let secs = probe_media_len(&path);
            let _ = tx.send((path, secs));
        });
    }

    fn poll_len(&mut self) {
        let Some(rx) = self.len_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok((path, Some(secs))) => {
                self.len_pending = None;
                self.note_len(&path, secs);
                if !self.len_cache.contains_key(&path) {
                    self.len_miss.insert(path);
                }
            }
            Ok((path, None)) => {
                self.len_pending = None;
                self.len_miss.insert(path);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.len_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.len_pending = None,
        }
    }

    fn audio_head(&self) -> f32 {
        if let Some(t) = self.seek_drag {
            return t;
        }
        self.deck_base + self.mixer.deck_pos()
    }

    fn video_head(&self) -> f32 {
        if let Some(t) = self.seek_drag {
            return t;
        }
        if self.media_run {
            self.media_offset + self.media_started.elapsed().as_secs_f32()
        } else {
            self.media_offset
        }
    }

    fn jump_to(&mut self, secs: f32) {
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        let len = self.len_cache.get(&path).copied();
        let secs = match len {
            Some(len) if len > 0.3 => secs.clamp(0.0, len - 0.25),
            _ => secs.max(0.0),
        };
        match media_kind(&path) {
            "video" => {
                let playing = self.media_run;
                self.media_offset = secs;
                if playing {
                    self.start_video(&path);
                }
            }
            "audio" => self.jump_audio(&path, secs),
            "gif" => self.jump_gif(secs),
            _ => {}
        }
    }

    fn jump_audio(&mut self, path: &Path, secs: f32) {
        let playing = self.deck_on && self.mixer.deck_live();
        let paused = self.mixer.deck_paused();
        if playing && self.mixer.deck_try_seek(Duration::from_secs_f32(secs)) {
            self.deck_base = 0.0;
            return;
        }
        self.deck_base = secs;
        if !playing && !paused {
            self.deck_on = false;
            return;
        }
        if let Err(err) = self.mixer.deck_play_at(path, Duration::from_secs_f32(secs), !playing) {
            self.deck_on = false;
            self.chat.push(format!("Player: {err}"));
            return;
        }
        self.deck_on = playing;
    }

    fn load_text(&mut self, path: &Path) {
        if self.text_for.as_deref() == Some(path) && !self.text_body.is_empty() {
            return;
        }
        match crate::deskfile::read_text(path) {
            Ok(body) => {
                self.text_body = body;
                self.text_for = Some(path.to_path_buf());
                self.text_dirty = false;
                self.deck_note.clear();
            }
            Err(err) => {
                self.text_body.clear();
                self.text_for = Some(path.to_path_buf());
                self.text_dirty = false;
                self.deck_note = err;
            }
        }
    }

    fn paint_text(&mut self, ui: &mut egui::Ui, path: &Path) {
        self.load_text(path);
        if !self.deck_note.is_empty() && self.text_body.is_empty() {
            wrap_text(ui, &self.deck_note, CYAN, 12.0);
        }
        let rows = self.text_body.lines().count().clamp(8, 400);
        let width = ui.available_width().max(40.0);
        let resp = ui.add(
            egui::TextEdit::multiline(&mut self.text_body)
                .desired_width(width)
                .desired_rows(rows)
                .font(egui::FontId::new(14.0, theme::mono()))
                .code_editor(),
        );
        if resp.changed() {
            self.text_dirty = true;
            self.text_hold = None;
        }
    }

    fn save_text(&mut self) {
        let Some(path) = self.text_for.clone() else {
            return;
        };
        match crate::deskfile::write_text(&self.root, &path, &self.text_body) {
            Ok(()) => {
                self.text_dirty = false;
                self.text_hold = None;
                self.deck_note = "Saved the text.".into();
            }
            Err(err) => self.deck_note = err,
        }
    }

    fn discard_text(&mut self) {
        self.text_dirty = false;
        self.text_for = None;
        let hold = self.text_hold.take();
        self.deck_note.clear();
        match hold {
            Some(TextMove::Index(i)) if i < self.deck_list.len() => {
                self.deck_i = i;
                self.cue_from_start(true);
            }
            Some(TextMove::Next) => self.deck_next(false),
            Some(TextMove::Prev) => self.deck_prev(),
            _ => {
                if let Some(p) = self.deck_list.get(self.deck_i).cloned() {
                    self.load_text(&p);
                }
            }
        }
    }

    fn restore_selected(&mut self) {
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        match crate::deskfile::restore_file(&self.root, &path) {
            Ok(()) => {
                self.text_for = None;
                self.text_dirty = false;
                self.tex.forget(&path.to_string_lossy());
                self.tex.forget("gif-stage");
                self.tex.forget("media-stage");
                self.gif_for = None;
                self.tags_for = None;
                self.deck_note = "Restored the last copy.".into();
                self.deck_play_current();
            }
            Err(err) => self.deck_note = err,
        }
    }

    fn load_gif(&mut self, path: &Path) {
        if self.gif_for.as_deref() == Some(path) && !self.gif_frames.is_empty() {
            self.gif_run = true;
            return;
        }
        self.gif_gen = self.gif_gen.wrapping_add(1);
        let gen = self.gif_gen;
        self.gif_frames.clear();
        self.gif_i = 0;
        self.gif_acc = 0.0;
        self.gif_run = false;
        self.gif_note.clear();
        self.gif_for = Some(path.to_path_buf());
        let path = path.to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        self.gif_rx = Some(rx);
        std::thread::spawn(move || {
            let set = crate::deskfile::decode_gif(&path);
            let _ = tx.send(GifDone { gen, path, set });
        });
    }

    fn poll_gif(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.gif_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(done) => {
                if done.gen != self.gif_gen {
                    return;
                }
                match done.set {
                    Ok(set) => {
                        self.gif_note = set.note;
                        self.gif_frames = set.frames;
                        self.gif_for = Some(done.path);
                        self.gif_i = 0;
                        self.gif_acc = 0.0;
                        self.gif_run = !self.gif_frames.is_empty();
                        self.show_gif_frame(ctx);
                    }
                    Err(err) => self.gif_note = err,
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.gif_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if self.gif_note.is_empty() {
                    self.gif_note = "The gif did not open.".into();
                }
            }
        }
    }

    fn step_gif(&mut self, ctx: &egui::Context, dt: f32) {
        self.poll_gif(ctx);
        if self.current_kind() != "gif" || !self.gif_run || self.gif_frames.is_empty() {
            return;
        }
        self.gif_acc += dt * 1000.0;
        let mut moved = false;
        for _ in 0..8 {
            let ms = self.gif_frames[self.gif_i].ms.max(10) as f32;
            if self.gif_acc < ms {
                break;
            }
            self.gif_acc -= ms;
            self.gif_i = (self.gif_i + 1) % self.gif_frames.len();
            moved = true;
        }
        if moved {
            self.show_gif_frame(ctx);
        }
    }

    fn show_gif_frame(&mut self, ctx: &egui::Context) {
        if self.gif_frames.is_empty() {
            return;
        }
        let i = self.gif_i.min(self.gif_frames.len() - 1);
        let w = self.gif_frames[i].w;
        let h = self.gif_frames[i].h;
        let rgb = std::mem::take(&mut self.gif_frames[i].rgb);
        let gen = self.gif_gen.wrapping_add(i as u64).wrapping_add(1);
        let _ = self.tex.put_rgb(ctx, "gif-stage", w, h, &rgb, gen);
        self.gif_frames[i].rgb = rgb;
    }

    fn gif_secs(&self) -> Option<f32> {
        if self.gif_frames.is_empty() {
            return None;
        }
        let ms: u32 = self.gif_frames.iter().map(|f| f.ms.max(10)).sum();
        Some(ms as f32 / 1000.0)
    }

    fn gif_head(&self) -> f32 {
        if let Some(t) = self.seek_drag {
            return t;
        }
        let mut ms = 0u32;
        for frame in self.gif_frames.iter().take(self.gif_i) {
            ms += frame.ms.max(10);
        }
        ms as f32 / 1000.0 + self.gif_acc / 1000.0
    }

    fn jump_gif(&mut self, secs: f32) {
        if self.gif_frames.is_empty() {
            return;
        }
        let mut left = (secs.max(0.0) * 1000.0) as u32;
        for (i, frame) in self.gif_frames.iter().enumerate() {
            let ms = frame.ms.max(10);
            if left < ms {
                self.gif_i = i;
                self.gif_acc = left as f32;
                return;
            }
            left -= ms;
        }
        self.gif_i = self.gif_frames.len() - 1;
        self.gif_acc = 0.0;
    }

    fn paint_gif(&mut self, ui: &mut egui::Ui, full: bool, path: &Path) {
        if self.gif_for.as_deref() != Some(path) {
            self.load_gif(path);
        }
        self.show_gif_frame(ui.ctx());
        if !self.gif_note.is_empty() {
            wrap_text(ui, &self.gif_note, CYAN, 12.0);
        }
        let max = Self::stage_size(ui, full);
        if let Some(tex) = self.tex.get_key("gif-stage") {
            let _ = paint_contain(ui, &tex, max);
        } else {
            let (rect, _) = ui.allocate_exact_size(max, egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, PANEL);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Opening the gif…",
                FontId::new(14.0, theme::mono()),
                CYAN,
            );
        }
    }

    fn open_tags(&mut self) {
        self.tags_open = !self.tags_open;
        self.tags_arm = 0;
        if self.tags_open {
            self.queue_tags();
        }
    }

    fn queue_tags(&mut self) {
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        if self.tags_for.as_ref() == Some(&path) && self.tags_rx.is_none() {
            return;
        }
        self.tags_gen = self.tags_gen.wrapping_add(1);
        let gen = self.tags_gen;
        let (tx, rx) = std::sync::mpsc::channel();
        self.tags_rx = Some(rx);
        self.tags_for = Some(path.clone());
        std::thread::spawn(move || {
            let result = crate::deskfile::read_tags(&path).map(|tags| (path, tags));
            let _ = tx.send((gen, TagMsg::Read(result)));
        });
    }

    fn poll_tags(&mut self) {
        let Some(rx) = self.tags_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok((gen, TagMsg::Read(Ok((path, tags))))) => {
                if gen == self.tags_gen {
                    self.tags = tags;
                    self.tags_for = Some(path);
                    self.tags_note.clear();
                }
            }
            Ok((gen, TagMsg::Read(Err(err)))) => {
                if gen == self.tags_gen {
                    self.tags_note = err;
                }
            }
            Ok((gen, TagMsg::Wrote(result))) => self.finish_tag_write(gen, result),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.tags_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }

    fn arm_tags(&mut self, save: bool) {
        let kind = self.current_kind();
        if !matches!(kind, "audio" | "video" | "image") {
            self.tags_note = "This file has no media tags.".into();
            return;
        }
        if crate::sys::ffmpeg_bin().is_none() {
            self.tags_note = "FFmpeg is missing, so tags cannot be changed here.".into();
            return;
        }
        let want = if save { 1 } else { 2 };
        if self.tags_arm != want {
            self.tags_arm = want;
            self.tags_note = if !save {
                "Press Clear tags again to strip the tags.".into()
            } else if kind == "image" {
                "Press Save tags again. The picture file is written again.".into()
            } else {
                "Press Save tags again.".into()
            };
            return;
        }
        self.tags_arm = 0;
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        let tags = self.tags.clone();
        let root = self.root.clone();
        let clear = !save;
        self.tags_gen = self.tags_gen.wrapping_add(1);
        let gen = self.tags_gen;
        let (tx, rx) = std::sync::mpsc::channel();
        self.tags_rx = Some(rx);
        std::thread::spawn(move || {
            let result = crate::deskfile::apply_tags(&root, &path, &tags, kind, clear);
            let _ = tx.send((gen, TagMsg::Wrote(result)));
        });
        self.tags_note = if save {
            "Writing the tags…".into()
        } else {
            "Clearing the tags…".into()
        };
    }

    fn finish_tag_write(&mut self, gen: u64, result: Result<(), String>) {
        if gen != self.tags_gen {
            return;
        }
        match result {
            Ok(()) => {
                self.tags_note = if self.tags_note.starts_with("Clear") {
                    "Cleared the tags.".into()
                } else {
                    "Saved the tags.".into()
                };
                self.tags_for = None;
                if let Some(p) = self.deck_list.get(self.deck_i).cloned() {
                    self.tex.forget(&p.to_string_lossy());
                }
                self.queue_tags();
            }
            Err(err) => self.tags_note = err,
        }
    }

    fn paint_tags(&mut self, ui: &mut egui::Ui) {
        if !self.tags_open {
            return;
        }
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            return;
        };
        if self.tags_for.as_ref() != Some(&path) && self.tags_rx.is_none() {
            self.queue_tags();
        }
        ui.add_space(6.0);
        theme::kicker(ui, "TAGS");
        let folder = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        wrap_text(
            ui,
            &format!(
                "{}\n{}\n{} · {}",
                path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                folder,
                brief_bytes(size),
                crate::deskfile::changed_stamp(&path)
            ),
            MUTED,
            11.0,
        );
        let kind = media_kind(&path);
        if matches!(kind, "audio" | "video" | "image") {
            let mut title = self.tags.title.clone();
            let mut artist = self.tags.artist.clone();
            let mut album = self.tags.album.clone();
            let mut year = self.tags.year.clone();
            let mut comment = self.tags.comment.clone();
            if Self::tag_line(ui, "Title", &mut title) {
                self.tags.title = title;
                self.tags_arm = 0;
            }
            if Self::tag_line(ui, "Artist", &mut artist) {
                self.tags.artist = artist;
                self.tags_arm = 0;
            }
            if Self::tag_line(ui, "Album", &mut album) {
                self.tags.album = album;
                self.tags_arm = 0;
            }
            if Self::tag_line(ui, "Year", &mut year) {
                self.tags.year = year;
                self.tags_arm = 0;
            }
            if Self::tag_line(ui, "Comment", &mut comment) {
                self.tags.comment = comment;
                self.tags_arm = 0;
            }
            if self.tags.width > 0 {
                wrap_text(
                    ui,
                    &format!("{}×{}", self.tags.width, self.tags.height),
                    DIM,
                    11.0,
                );
            }
            ui.horizontal_wrapped(|ui| {
                let save = if self.tags_arm == 1 { "Save tags?" } else { "Save tags" };
                let clear = if self.tags_arm == 2 { "Clear tags?" } else { "Clear tags" };
                if theme::neon_btn(ui, save).clicked() {
                    self.arm_tags(true);
                }
                if theme::neon_btn_color(ui, clear, KILL, self.tags_arm == 2).clicked() {
                    self.arm_tags(false);
                }
            });
        } else if kind == "gif" {
            wrap_text(
                ui,
                "Tags on a gif stay as they are so the animation is not rewritten.",
                MUTED,
                11.0,
            );
        } else {
            wrap_text(ui, "This file has no media tags.", MUTED, 11.0);
        }
        if !self.tags_note.is_empty() {
            wrap_text(ui, &self.tags_note, CYAN, 12.0);
        }
    }

    fn tag_line(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .family(theme::mono())
                    .size(11.0)
                    .color(DIM),
            );
            if ui
                .add(
                    egui::TextEdit::singleline(value)
                        .desired_width(ui.available_width().max(80.0)),
                )
                .changed()
            {
                changed = true;
            }
        });
        changed
    }

    fn open_file_send(&mut self) {
        if !self.node_live {
            self.chat.push("Press Online first. Then you can send it.".into());
            return;
        }
        let Some(path) = self.deck_list.get(self.deck_i).cloned() else {
            self.chat.push("Choose a file first.".into());
            return;
        };
        self.send_path = Some(path);
        self.send_ids.clear();
    }

    fn ui_file_send(&mut self, ctx: &egui::Context) {
        if self.send_path.is_none() {
            return;
        }
        let name = self
            .send_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into());
        let mut close = false;
        let mut fire = false;
        let contacts: Vec<(String, String)> = self
            .contacts
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        let crews: Vec<(String, String, Vec<String>)> = self
            .crews
            .iter()
            .map(|c| (c.id.clone(), c.name.clone(), c.members.clone()))
            .collect();
        egui::Window::new("Send file")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                wrap_text(
                    ui,
                    &format!("Send {name} to one or more contacts."),
                    CREAM,
                    13.0,
                );
                for (id, who) in &contacts {
                    let mut on = self.send_ids.contains(id);
                    if ui.checkbox(&mut on, who).changed() {
                        if on {
                            self.send_ids.insert(id.clone());
                        } else {
                            self.send_ids.remove(id);
                        }
                    }
                }
                for (_, crew, members) in &crews {
                    if theme::neon_btn(ui, &format!("Check {crew}")).clicked() {
                        for id in members {
                            if id != &self.net.self_id {
                                self.send_ids.insert(id.clone());
                            }
                        }
                    }
                }
                if contacts.is_empty() && crews.is_empty() {
                    wrap_text(ui, "Save a contact or a crew first.", DIM, 12.0);
                }
                ui.horizontal_wrapped(|ui| {
                    if theme::neon_btn(ui, "Send").clicked() {
                        fire = true;
                    }
                    if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
                        close = true;
                    }
                });
            });
        if fire {
            self.fire_file_send();
        } else if close {
            self.send_path = None;
            self.send_ids.clear();
        }
    }

    fn fire_file_send(&mut self) {
        let Some(path) = self.send_path.clone() else {
            return;
        };
        self.send_ids.retain(|id| id != &self.net.self_id);
        let names: std::collections::HashMap<String, String> = self
            .contacts
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        let targets: Vec<(Option<String>, String)> = self
            .send_ids
            .iter()
            .map(|id| {
                let name = names.get(id).cloned().unwrap_or_else(|| id.clone());
                (Some(id.clone()), name)
            })
            .collect();
        self.spawn_send(path, targets);
    }

    fn poll_send(&mut self) {
        let Some(rx) = self.send_rx.take() else {
            return;
        };
        let mut put_back = true;
        loop {
            match rx.try_recv() {
                Ok(crate::net::SendNote::Tick(tick)) => {
                    let started = self
                        .send_live
                        .as_ref()
                        .filter(|live| live.filename == tick.filename && live.index == tick.index)
                        .map(|live| live.started)
                        .unwrap_or_else(Instant::now);
                    self.send_live = Some(LiveSend {
                        filename: tick.filename,
                        who: tick.who,
                        index: tick.index,
                        count: tick.count,
                        done: tick.done,
                        total: tick.total,
                        started,
                    });
                }
                Ok(crate::net::SendNote::Finished(Ok(()))) => {
                    if let Some(live) = self.send_live.take() {
                        self.chat.push(format!(
                            "Sent {} to {}.",
                            live.filename,
                            if live.count == 1 {
                                live.who
                            } else {
                                format!("{} people", live.count)
                            }
                        ));
                    }
                    self.send_path = None;
                    self.send_ids.clear();
                    put_back = false;
                    break;
                }
                Ok(crate::net::SendNote::Finished(Err(err))) => {
                    self.send_live = None;
                    self.chat.push(err);
                    put_back = false;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.send_live = None;
                    put_back = false;
                    break;
                }
            }
        }
        if put_back {
            self.send_rx = Some(rx);
        }
    }

    fn send_clock_line(&self) -> Option<String> {
        let live = self.send_live.as_ref()?;
        let who = if live.count > 1 {
            format!("{} · {} of {}", live.who, live.index, live.count)
        } else {
            live.who.clone()
        };
        Some(transfer_line(
            &format!("Sending {} to {who}", live.filename),
            live.done,
            live.total,
            live.started.elapsed().as_secs_f32(),
        ))
    }

    fn recv_clock_line(&self) -> Option<String> {
        let id = self.recv_focus.as_ref()?;
        let file = self.file_in.get(id)?;
        Some(transfer_line(
            &format!("Receiving {}", file.filename),
            file.got,
            file.size,
            file.started.elapsed().as_secs_f32(),
        ))
    }

    fn ui_seek_bar(&mut self, ui: &mut egui::Ui) {
        let kind = self.current_kind();
        let file_live = self.deck_on || self.media_run || self.mixer.deck_paused();
        if self.radio_on && !file_live {
            ui.label(
                RichText::new("LIVE")
                    .family(theme::mono())
                    .size(12.0)
                    .color(CYAN),
            );
            return;
        }
        if kind != "audio" && kind != "video" && kind != "gif" {
            return;
        }
        let path = self.deck_list.get(self.deck_i).cloned();
        if kind != "gif" {
            if let Some(path) = &path {
                self.ask_len(path);
            }
        }
        let len = if kind == "gif" {
            self.gif_secs()
        } else {
            path.as_ref().and_then(|p| self.len_cache.get(p).copied())
        };
        let head = match kind {
            "video" => self.video_head(),
            "gif" => self.gif_head(),
            _ => self.audio_head(),
        };
        let head = len.map(|l| head.min(l)).unwrap_or(head).max(0.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(fmt_media(head))
                    .family(theme::mono())
                    .size(12.0)
                    .color(CYAN),
            );
            if let Some(len) = len {
                ui.label(
                    RichText::new(format!("{} left", fmt_media((len - head).max(0.0))))
                        .family(theme::mono())
                        .size(12.0)
                        .color(theme::ACID),
                );
            }
        });
        let width = ui.available_width().max(40.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 16.0), egui::Sense::click_and_drag());
        let shown = self.seek_drag.unwrap_or(head);
        let frac = len
            .filter(|l| *l > 0.0)
            .map(|l| (shown / l).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        ui.painter().rect_filled(rect, 0.0, PANEL);
        let fill = rect.with_max_x(rect.left() + rect.width() * frac);
        ui.painter().rect_filled(fill, 0.0, theme::ACID);
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, theme::HOT),
            egui::StrokeKind::Inside,
        );
        let Some(len) = len else {
            return;
        };
        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                self.seek_drag = Some(t * len);
            }
        }
        if resp.drag_stopped() || (resp.clicked() && !resp.dragged()) {
            if let Some(at) = self.seek_drag.take() {
                self.jump_to(at);
            }
        }
    }

    fn add_deck_paths(&mut self, paths: Vec<PathBuf>) {
        let mut n = 0;
        for p in paths {
            if !is_library_file(&p) || !p.is_file() {
                continue;
            }
            if self.deck_list.iter().any(|x| x == &p) {
                continue;
            }
            self.deck_list.push(p);
            n += 1;
        }
        if n > 0 {
            self.deck_bag.clear();
        }
        save_deck_lib(&self.root, &self.deck_list);
        if n > 0 {
            self.chat.push(format!("Player: added {n} file{}.", if n == 1 { "" } else { "s" }));
        }
    }

    fn add_deck_folder(&mut self, dir: PathBuf) {
        let mut found = Vec::new();
        collect_music(&dir, &mut found, 4);
        self.add_deck_paths(found);
    }

    fn refresh_devices(&mut self) {
        self.inputs = crate::audio::list_inputs();
        self.outputs = crate::audio::list_outputs();
        self.cameras = crate::video::list_cameras();
        let mic = crate::audio::pick_listed(
            &self.inputs,
            &self.mic_name,
            crate::audio::default_input_name(),
        );
        let spk = crate::audio::pick_listed(
            &self.outputs,
            &self.speaker_name,
            crate::audio::default_output_name(),
        );
        if mic != self.mic_name {
            self.set_mic(mic);
        }
        if spk != self.speaker_name {
            self.set_speaker(spk);
        }
        if self.cam_name.is_empty() {
            if let Some(c) = self.cameras.first() {
                self.set_camera(c.path.clone());
            }
        } else if !self.cameras.iter().any(|c| c.path == self.cam_name) {
            if let Some(c) = crate::video::default_camera() {
                self.set_camera(c.path);
            }
        }
        let mic_n = self.inputs.len();
        let spk_n = self.outputs.len();
        self.status = format!("Audio {mic_n} in / {spk_n} out");
    }

    fn persist_devices(&self) {
        save_devices(
            &self.root,
            &DevicePref {
                mic: self.mic_name.clone(),
                speaker: self.speaker_name.clone(),
                camera: self.cam_name.clone(),
            },
        );
    }

    fn set_chrome_theme(&mut self, t: theme::ChromeTheme) {
        self.chrome = t;
        theme::set_chrome(t);
        self.visuals_on = false;
        self.theme_pick = false;
        save_chrome(&self.root, t, self.ui_scale);
    }

    fn set_ui_scale(&mut self, scale: f32) {
        let s = clamp_ui_scale(scale);
        if (self.ui_scale - s).abs() < 0.001 {
            return;
        }
        self.ui_scale = s;
        save_chrome(&self.root, self.chrome, s);
    }

    fn set_camera(&mut self, path: String) {
        self.cam_name = path;
        self.persist_devices();
        if self.cam_on {
            self.start_cam();
        }
    }

    fn start_cam(&mut self) {
        self.cam_cap = None;
        if !self.refresh_ffmpeg() {
            self.chat.push(format!(
                "Camera needs FFmpeg. {}",
                crate::sys::ffmpeg_install_tip()
            ));
            self.cam_on = false;
            return;
        }
        let path = if self.cam_name.is_empty() {
            crate::video::default_camera().map(|c| c.path).unwrap_or_default()
        } else {
            self.cam_name.clone()
        };
        if path.is_empty() {
            self.chat.push("No camera found.".into());
            self.cam_on = false;
            return;
        }
        match crate::video::start_camera(&path) {
            Some(cap) => {
                self.cam_cap = Some(cap);
                self.cam_on = true;
                self.cam_name = path;
            }
            None => {
                self.chat.push("Could not open that camera.".into());
                self.cam_on = false;
            }
        }
    }

    fn start_screen_share(&mut self) {
        self.screen_cap = None;
        // PipeWire portal path may use GStreamer (not FFmpeg). Still tip if neither works.
        match crate::video::start_screen() {
            Ok(started) => {
                let _backend = started.backend; // PipeWire / x11grab / gdigrab
                self.screen_cap = Some(started.capture);
                self.screen_on = true;
                self.chat.push(started.note);
            }
            Err(e) => {
                self.chat.push(e);
                self.screen_on = false;
            }
        }
    }

    fn call_targets(&self) -> Vec<String> {
        let mut v = Vec::new();
        let push = |v: &mut Vec<String>, id: &str| {
            if !id.is_empty()
                && id != self.net.self_id
                && !self.call_drop.iter().any(|d| d == id)
                && !v.iter().any(|x| x == id)
            {
                v.push(id.to_string());
            }
        };
        if let Some(cid) = &self.call_crew {
            if let Some(c) = self.crews.iter().find(|c| &c.id == cid) {
                for id in &c.members {
                    push(&mut v, id);
                }
            }
        }
        if let Some(id) = &self.call_id {
            push(&mut v, id);
        }
        for id in &self.call_with {
            push(&mut v, id);
        }
        v
    }

    fn remember_party(&mut self, id: &str) {
        if id.is_empty() || id == self.net.self_id {
            return;
        }
        self.call_drop.retain(|d| d != id);
        if !self.call_with.iter().any(|x| x == id) {
            self.call_with.push(id.to_string());
        }
        if self.call_id.is_none() {
            self.call_id = Some(id.to_string());
        }
    }

    fn push_roster(&self) {
        let mut ids = self.call_targets();
        ids.insert(0, self.net.self_id.clone());
        let packed = ids.join(",");
        for id in self.call_targets() {
            self.net.send_call_roster(&id, &packed);
        }
    }

    fn add_person_to_call(&mut self, id: String, video: bool) {
        if id == self.net.self_id {
            return;
        }
        self.remember_party(&id);
        self.voice_on = true;
        self.ensure_mic();
        if video {
            self.video_on = true;
            if !self.cam_on {
                self.start_cam();
            }
            self.net
                .send_video("invite", Some(id.clone()), self.call_crew.clone());
        }
        self.net
            .send_voice_ex("invite", Some(id.clone()), self.call_crew.clone());
        self.push_roster();
        let name = self
            .contacts
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or(id);
        self.chat.push(format!("Added {name} to the call."));
    }

    fn ui_call_people(&mut self, ui: &mut egui::Ui, video: bool) {
        let names: Vec<String> = self
            .call_targets()
            .into_iter()
            .map(|id| {
                self.contacts
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or(id)
            })
            .collect();
        if !names.is_empty() {
            wrap_text(ui, &format!("On the call: {}", names.join(", ")), CYAN, 12.0);
        }
        if theme::neon_btn_color(ui, "Add", CYAN, self.call_add).clicked() {
            self.call_add = !self.call_add;
        }
        if !self.call_add {
            return;
        }
        let have = self.call_targets();
        let mut choices: Vec<(String, String)> = self
            .contacts
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        for crew in &self.crews {
            for id in &crew.members {
                if !choices.iter().any(|(have_id, _)| have_id == id) {
                    let name = self
                        .contacts
                        .iter()
                        .find(|c| &c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| id.clone());
                    choices.push((id.clone(), format!("{name} · {}", crew.name)));
                }
            }
        }
        for (id, name) in choices {
            if id == self.net.self_id || have.iter().any(|x| x == &id) {
                continue;
            }
            if theme::wide_btn(ui, &name, "ADD TO CALL", false).clicked() {
                self.add_person_to_call(id, video);
            }
        }
    }

    fn apply_roster(&mut self, packed: &str) {
        for id in packed.split(',') {
            let id = id.trim();
            if !id.is_empty() {
                self.remember_party(id);
            }
        }
        if !self.call_targets().is_empty() {
            self.voice_on = true;
            self.ensure_mic();
        }
    }

    fn tick_fps(&mut self) {
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if dt > 0.00005 {
            let inst = (1.0 / dt).min(1000.0);
            self.fps = if self.fps < 1.0 {
                inst
            } else {
                self.fps * 0.88 + inst * 0.12
            };
        }
    }

    fn refresh_mix_cache(&mut self) {
        let key = format!(
            "{}|{}|{}",
            self.blight as u8, self.cat_filter, self.mix_search
        );
        if key == self.mix_cache_key {
            return;
        }
        self.mix_cache_key = key;
        let q = self.mix_search.to_lowercase();
        self.mix_cache = self
            .catalog
            .layers(self.blight)
            .iter()
            .filter(|l| self.cat_filter == "all" || l.category == self.cat_filter)
            .filter(|l| {
                q.is_empty()
                    || l.name.to_lowercase().contains(&q)
                    || l.mood.to_lowercase().contains(&q)
            })
            .map(|l| (l.id.clone(), l.name.clone()))
            .collect();
        self.mix_cache.extend(
            self.custom
                .iter()
                .filter(|l| q.is_empty() || l.name.to_lowercase().contains(&q))
                .map(|l| (l.id.clone(), l.name.clone())),
        );
    }

    fn refresh_cat_cache(&mut self) {
        let key = format!(
            "{}|{}|{}",
            self.overlay_cat,
            self.catalog_q,
            self.catalog_rows.len()
        );
        if key == self.cat_cache_key {
            return;
        }
        self.cat_cache_key = key;
        let q = self.catalog_q.to_lowercase();
        self.cat_cache = self
            .catalog_rows
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let extra = v
                    .get("blurb")
                    .or_else(|| v.get("text"))
                    .or_else(|| v.get("kind"))
                    .or_else(|| v.get("type"))
                    .or_else(|| v.get("role"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if q.is_empty()
                    || name.to_lowercase().contains(&q)
                    || extra.to_lowercase().contains(&q)
                {
                    Some((i, name, extra.chars().take(140).collect()))
                } else {
                    None
                }
            })
            .collect();
    }

    fn trim_logs(&mut self) {
        if self.combat_log.len() > 160 {
            let n = self.combat_log.len() - 120;
            self.combat_log.drain(0..n);
            save_combat_log(&self.root, &self.combat_log);
        }
        if self.chat.len() != self.chat_saved {
            save_chat(&self.root, &self.chat);
            self.chat_saved = self.chat.len();
        }
    }

    fn pump_video(&mut self) {
        if !self.video_on {
            return;
        }
        if self.media_at.elapsed() < Duration::from_millis(100) {
            return;
        }
        self.media_at = Instant::now();
        let targets = self.call_targets();
        // Soft-fail: capture death clears cam/screen only — voice/call stay up.
        if self.cam_on {
            let dead = self
                .cam_cap
                .as_mut()
                .map(|c| !c.alive())
                .unwrap_or(true);
            if dead {
                self.chat.push(
                    "Camera capture ended — voice stays up. Toggle Camera to retry.".into(),
                );
                self.cam_on = false;
                self.cam_cap = None;
                self.local_cam.clear();
            } else if let Some(cap) = self.cam_cap.as_ref() {
                if let Some(f) = cap.latest() {
                    self.local_cam = f;
                    if self.local_cam.len() <= 160_000 && !targets.is_empty() {
                        let crew = self.call_crew.clone();
                        for id in &targets {
                            self.net.send_video_frame(
                                "cam",
                                Some(id.clone()),
                                crew.clone(),
                                &self.local_cam,
                            );
                        }
                    }
                }
            }
        }
        if self.screen_on {
            let dead = self
                .screen_cap
                .as_mut()
                .map(|c| !c.alive())
                .unwrap_or(true);
            if dead {
                self.chat.push(
                    "Screen share ended — voice stays up. Share screen again to retry.".into(),
                );
                self.screen_on = false;
                self.screen_cap = None;
                self.local_screen.clear();
            } else if let Some(cap) = self.screen_cap.as_ref() {
                if let Some(f) = cap.latest() {
                    self.local_screen = f;
                    if self.local_screen.len() <= 160_000 && !targets.is_empty() {
                        let crew = self.call_crew.clone();
                        for id in &targets {
                            self.net.send_video_frame(
                                "screen",
                                Some(id.clone()),
                                crew.clone(),
                                &self.local_screen,
                            );
                        }
                    }
                }
            }
        }
    }

    fn end_call_local(&mut self) {
        self.call_id = None;
        self.call_with.clear();
        self.call_drop.clear();
        self.call_add = false;
        self.call_crew = None;
        self.incoming = None;
        self.incoming_video = None;
        self.video_on = false;
        self.cam_on = false;
        self.screen_on = false;
        self.cam_cap = None;
        self.screen_cap = None;
        self.local_cam.clear();
        self.local_screen.clear();
        self.remote_vid.clear();
        self.voice_on = false;
        self.aec.reset();
        self.opus_buf.clear();
        // Once-per-call: allow the Opus→PCM notice again on the next call.
        self.opus_fallback_noted = false;
    }

    fn hang_up(&mut self) {
        let crew = self.call_crew.clone();
        for id in self.call_targets() {
            self.net.send_video("hangup", Some(id.clone()), crew.clone());
            self.net.send_voice_ex("hangup", Some(id), crew.clone());
        }
        self.end_call_local();
    }

    fn start_video_to(&mut self, id: String, crew: Option<String>) {
        self.call_with.clear();
        self.call_drop.clear();
        self.call_id = Some(id.clone());
        self.call_crew = crew.clone();
        self.video_on = true;
        self.voice_on = true;
        self.ensure_mic();
        self.start_cam();
        if let Some(cid) = &crew {
            if let Some(c) = self.crews.iter().find(|c| &c.id == cid).cloned() {
                for mid in c.members {
                    if mid != self.net.self_id {
                        self.net.send_video("invite", Some(mid.clone()), crew.clone());
                        self.net.send_voice_ex("invite", Some(mid), crew.clone());
                    }
                }
            }
        } else {
            self.net.send_video("invite", Some(id.clone()), None);
            self.net.send_voice("invite", Some(id));
        }
        self.shell = ShellPanel::Video;
        self.chat.push("Video call ringing…".into());
    }

    fn accept_video(&mut self) {
        let Some((id, _, crew)) = self.incoming_video.clone() else {
            if let Some((id, _)) = self.incoming.clone() {
                self.net.send_voice("accept", Some(id.clone()));
                self.remember_party(&id);
                self.incoming = None;
                self.voice_on = true;
                self.ensure_mic();
                self.push_roster();
            }
            return;
        };
        self.remember_party(&id);
        self.call_crew = crew.clone();
        self.incoming = None;
        self.incoming_video = None;
        self.video_on = true;
        self.voice_on = true;
        self.ensure_mic();
        self.start_cam();
        if let Some(cid) = &crew {
            if let Some(c) = self.crews.iter().find(|c| &c.id == cid).cloned() {
                for mid in c.members {
                    if mid != self.net.self_id {
                        self.net.send_video("accept", Some(mid.clone()), crew.clone());
                        self.net.send_voice_ex("accept", Some(mid), crew.clone());
                    }
                }
            }
        } else {
            self.net.send_video("accept", Some(id.clone()), None);
            self.net.send_voice("accept", Some(id));
        }
        self.push_roster();
        self.shell = ShellPanel::Video;
    }

    fn open_share(&mut self, kind: &str, label: &str, body: String) {
        if body.is_empty() {
            return;
        }
        if !self.node_live {
            self.chat
                .push("Press Online first. Then you can send it.".into());
            return;
        }
        self.share_pick = Some((kind.to_string(), label.to_string(), body));
    }

    fn dispatch_share(&mut self, who: &str) {
        let Some((kind, label, body)) = self.share_pick.clone() else {
            return;
        };
        let mut ids = Vec::new();
        if let Some(c) = self.contacts.iter().find(|c| c.id == who) {
            ids.push(c.id.clone());
        } else if let Some(crew) = self.crews.iter().find(|c| c.id == who) {
            ids.extend(crew.members.clone());
        }
        ids.retain(|id| id != &self.net.self_id);
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            self.chat.push("That person is not on this deck.".into());
            return;
        }
        for id in &ids {
            self.net.send_share(id, &kind, &label, &body);
        }
        self.chat
            .push(format!("Sent {label} to {}.", ids.len()));
        self.share_pick = None;
    }

    fn take_share(&mut self, from_name: &str, kind: &str, body: &str) {
        match kind {
            "recon" => {
                let pack: serde_json::Value = match serde_json::from_str(body) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let Some(mut file) = pack
                    .get("file")
                    .and_then(|v| serde_json::from_value::<crate::recon::Dossier>(v.clone()).ok())
                else {
                    return;
                };
                file.id = format!("rc-{:08x}", rand::random::<u32>());
                file.file_no = format!("R-{:04}", crate::recon::next_no(&self.recon));
                let b64 = pack
                    .get("portrait_b64")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !b64.is_empty() {
                    if let Ok(bytes) =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
                    {
                        let ext = pack
                            .get("ext")
                            .and_then(|v| v.as_str())
                            .unwrap_or("png");
                        let rel = format!("data/recon/{}.{}", file.id, ext);
                        let path = self.root.join(&rel);
                        if std::fs::create_dir_all(self.root.join("data/recon")).is_ok()
                            && std::fs::write(&path, bytes).is_ok()
                        {
                            file.portrait = rel;
                        }
                    }
                } else {
                    file.portrait.clear();
                }
                let title = file.title();
                self.recon.push(file);
                crate::recon::save(&self.root, &self.recon);
                self.recon_i = self.recon.len() - 1;
                self.chat
                    .push(format!("{from_name} sent the file {title}."));
                self.page = Page::Recon;
            }
            "nethook" => {
                let Ok(mut hook) = serde_json::from_str::<crate::nethook::Nethook>(body) else {
                    return;
                };
                if hook.pinned() {
                    return;
                }
                hook.id = format!("nh-{:08x}", rand::random::<u32>());
                hook.owner_id = self.net.self_id.clone();
                hook.owner_name = self.handle.clone();
                hook.posted = false;
                hook.files.clear();
                let title = hook.title.clone();
                crate::nethook::save_one(&self.root, &hook);
                self.nethooks.push(hook);
                self.hook_i = self.nethooks.len() - 1;
                self.chat
                    .push(format!("{from_name} sent the page {title}."));
                self.page = Page::Nethooks;
            }
            "sheet" => {
                let Ok(mut sheet) = serde_json::from_str::<crate::chars::Character>(body) else {
                    return;
                };
                sheet.id = format!("ch-{:08x}", rand::random::<u32>());
                let title = sheet.name.clone();
                self.chars.push(sheet);
                self.char_i = self.chars.len() - 1;
                crate::chars::save(&self.root, &self.chars);
                self.chat
                    .push(format!("{from_name} sent the sheet {title}."));
            }
            _ => {}
        }
    }

    fn ui_share_pick(&mut self, ctx: &egui::Context) {
        if self.share_pick.is_none() {
            return;
        }
        let label = self
            .share_pick
            .as_ref()
            .map(|(_, label, _)| label.clone())
            .unwrap_or_default();
        let mut chosen = None;
        let mut close = false;
        egui::Window::new("Send")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                wrap_text(
                    ui,
                    &format!("Send {label} to one contact or one crew."),
                    CREAM,
                    13.0,
                );
                for c in &self.contacts.clone() {
                    if theme::wide_btn(ui, &c.name, "CONTACT", false).clicked() {
                        chosen = Some(c.id.clone());
                    }
                }
                for crew in &self.crews.clone() {
                    if theme::wide_btn(ui, &crew.name, "CREW", false).clicked() {
                        chosen = Some(crew.id.clone());
                    }
                }
                if self.contacts.is_empty() && self.crews.is_empty() {
                    wrap_text(ui, "Save a contact or a crew first.", DIM, 12.0);
                }
                if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
                    close = true;
                }
            });
        if let Some(id) = chosen {
            self.dispatch_share(&id);
        } else if close {
            self.share_pick = None;
        }
    }

    fn broadcast_mix(&mut self) {
        if !self.table_live() {
            return;
        }
        self.mix_dirty = true;
        self.mix_at = Instant::now();
    }

    fn push_mix_now(&self) {
        if !self.table_live() {
            return;
        }
        self.net.send_mix(
            self.mixer.snapshot(),
            self.blight,
            self.place.clone(),
            self.time.to_string(),
            self.inside,
        );
    }

    fn flush_mix(&mut self) {
        if !self.mix_dirty {
            return;
        }
        if self.mix_at.elapsed() < Duration::from_millis(80) {
            return;
        }
        self.mix_dirty = false;
        self.push_mix_now();
    }

    fn mark_sheets(&mut self) {
        self.sheet_dirty = true;
        self.sheet_at = Instant::now();
    }

    fn flush_sheets(&mut self) {
        if !self.sheet_dirty {
            return;
        }
        if self.sheet_at.elapsed() < Duration::from_millis(450) {
            return;
        }
        self.sheet_dirty = false;
        crate::chars::save(&self.root, &self.chars);
        self.push_sheets();
    }

    fn open_seat(&mut self, id: Option<String>) {
        let me = self.net.self_id.clone();
        let target = id.unwrap_or_else(|| me.clone());
        let showing = self.panel_on(Overlay::Chars)
            && self.viewing.as_deref().unwrap_or(me.as_str()) == target.as_str();
        if showing {
            self.open.retain(|o| *o != Overlay::Chars);
            self.viewing = None;
            return;
        }
        self.viewing = if target == me { None } else { Some(target.clone()) };
        if !self.panel_on(Overlay::Chars) {
            self.open.push(Overlay::Chars);
        }
        if self.viewing.is_some() {
            self.net.send_sheet_ask();
        }
    }

    fn portable_tokens(&self) -> Vec<maps::MapTok> {
        self.map
            .tokens
            .iter()
            .cloned()
            .map(|mut t| {
                t.image = images::portable_rel(&self.root, &t.image);
                t
            })
            .collect()
    }

    fn push_map_image(&mut self) {
        if !self.table_live() {
            return;
        }
        let Some(path) = self.map.image.clone() else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        if bytes.len() > FILE_CAP {
            self.err = "Map image is too large to send.".into();
            return;
        }
        let mime = mime_of(&path);
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg")
            .to_lowercase();
        let filename = format!("{MAP_WIRE}.{ext}");
        self.net.send_file(None, None, mime, &filename, &bytes);
    }

    fn install_map_image(&mut self, filename: &str, bytes: &[u8]) {
        let ext = Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg")
            .to_lowercase();
        let ext = match ext.as_str() {
            "png" | "webp" | "jpg" | "jpeg" => {
                if ext == "jpeg" {
                    "jpg"
                } else {
                    ext.as_str()
                }
            }
            _ => "jpg",
        };
        let dir = self.root.join("data/maps");
        let _ = std::fs::create_dir_all(&dir);
        let dest = dir.join(format!("table.{ext}"));
        if std::fs::write(&dest, bytes).is_ok() {
            if let Some(old) = &self.map.image {
                self.tex.forget(old.to_string_lossy().as_ref());
            }
            self.map.image = Some(dest);
            if !self.panel_on(Overlay::Maps) {
                self.open.push(Overlay::Maps);
            }
            self.status = "Map received".into();
        }
    }
}

impl eframe::App for Blightnet {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sweep_quiet_files();
        self.tick_fps();
        self.poll_net();
        self.poll_update();
        self.poll_meters();
        self.poll_probe();
        self.poll_radio();
        self.step_media(ctx);
        self.step_gif(ctx, ctx.input(|i| i.stable_dt));
        self.poll_deck_ready();
        self.poll_len();
        self.poll_tags();
        self.poll_send();
        if self.page == Page::Netspace && self.node_live && self.net_pos_at.elapsed() > Duration::from_millis(200) {
            self.net_pos_at = Instant::now();
            let name = if self.handle.trim().is_empty() {
                "YOU".into()
            } else {
                self.handle.clone()
            };
            self.net.send_net_pos(&name, self.netspace.x, self.netspace.z, self.netspace.yaw_pub());
        }
        self.refresh_node();
        if self.clock_run && self.is_gm {
            self.clock_acc += ctx.input(|i| i.stable_dt);
            if self.clock_acc >= 4.0 {
                let steps = (self.clock_acc / 4.0) as i32;
                self.clock_acc -= steps as f32 * 4.0;
                self.shift_clock(steps.max(1));
            }
        }
        self.trim_logs();
        self.flush_notes();
        self.flush_mix();
        self.flush_sheets();
        self.pump_mic();
        self.pump_video();
        self.pump_deck();
        if self.net.handle != self.handle {
            save_notes(&self.root, &self.net.handle, &self.notes);
            self.notes = load_notes(&self.root, &self.handle);
            self.notes_dirty = false;
            self.net.set_handle(self.handle.clone());
        }
        if !self.visuals_on {
            ctx.set_visuals(theme::visuals());
            ctx.style_mut(|s| {
                s.interaction.tooltip_delay = 0.08;
                s.interaction.tooltip_grace_time = 0.12;
            });
            self.visuals_on = true;
        }
        ctx.set_pixels_per_point(self.ui_scale);
        // Never sleep on this thread: Wayland frame callbacks would stall and
        // the boot screen would freeze. Cap rate with a delayed wake instead.
        ctx.request_repaint_after(FRAME);
        self.ui_shell(ctx);
    }
}

impl Blightnet {
    fn ui_shell(&mut self, ctx: &egui::Context) {
        let t = ctx.input(|i| i.time) as f32;
        let ch = theme::chrome();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(ch.bg))
            .show(ctx, |ui| {
                let win = ui.max_rect();
                ui.painter().rect_filled(win, 0.0, ch.bg);
                theme::holo_grid(ui, win);
                ui.painter().rect_stroke(
                    win,
                    0.0,
                    egui::Stroke::new(1.0, ch.hot),
                    egui::StrokeKind::Inside,
                );
                theme::hud_ticks(ui, win.shrink(8.0), ch.cyan, 12.0);
                let mut ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(win.shrink(1.0))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                self.draw_tour(ctx);
                self.draw_command_bar(&mut ui);
                let rest = ui.available_rect_before_wrap();
                let (body, status) = rest.split_top_bottom_at_y(rest.bottom() - 28.0);
                let dock_open = self.shell != ShellPanel::None;
                let dock_w = if dock_open {
                    let cap = (body.width() * 0.5).max(0.0);
                    (body.width() * 0.34).clamp(220.0_f32.min(cap), cap)
                } else {
                    0.0
                };
                let (main, dock) = if dock_open {
                    body.split_left_right_at_x(body.right() - dock_w)
                } else {
                    (body, Rect::from_min_size(body.right_top(), Vec2::ZERO))
                };
                let mut body_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(main)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                match self.page {
                    Page::Index => self.ui_index(&mut body_ui, t),
                    Page::Table => self.ui_table_root(&mut body_ui),
                    Page::Catalog(_) => self.ui_catalog(&mut body_ui),
                    Page::Chars => self.ui_chars(&mut body_ui),
                    Page::Tutorial => self.ui_tutorial(&mut body_ui),
                    Page::Audio => self.ui_audio(&mut body_ui),
                    Page::Blackjack => self.ui_bj(&mut body_ui),
                    Page::Nethooks => self.ui_nethooks(&mut body_ui),
                    Page::Netspace => self.ui_netspace(&mut body_ui),
                    Page::Rotn => self.ui_rotn(&mut body_ui),
                    Page::Player => self.ui_player_panel(&mut body_ui),
                    Page::Terminal => self.ui_terminal(&mut body_ui),
                    Page::Recon => self.ui_recon(&mut body_ui),
                    Page::Tree => self.ui_tree(&mut body_ui),
                    Page::Boot => {}
                }
                if dock_open {
                    ui.painter().vline(
                        dock.left(),
                        body.y_range(),
                        egui::Stroke::new(1.0, theme::HOT),
                    );
                    let dock_inner = dock.shrink2(Vec2::new(12.0, 8.0));
                    let mut dock_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(dock_inner)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    dock_ui.set_clip_rect(dock_inner);
                    self.ui_dock(&mut dock_ui);
                }
                self.ui_zoom(ui.ctx());
                self.ui_share_pick(ui.ctx());
                self.ui_file_send(ui.ctx());
                let mut st = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(status)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                st.painter().rect_filled(status, 0.0, RAIL);
                st.painter().hline(
                    status.x_range(),
                    status.top(),
                    egui::Stroke::new(1.0, theme::ACID),
                );
                let inner = status.shrink2(Vec2::new(10.0, 1.0));
                let mut line = st.new_child(
                    egui::UiBuilder::new()
                        .max_rect(inner)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                line.set_clip_rect(inner);
                egui::ScrollArea::horizontal()
                    .id_salt("status-line")
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(
                        egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden,
                    )
                    .show(&mut line, |ui| {
                        self.paint_status_items(ui);
                    });
            });
    }

    fn page_loc(&self) -> &'static str {
        match self.page {
            Page::Index => "blightnet://index",
            Page::Table => "blightnet://table",
            Page::Tutorial => "blightnet://tutorial",
            Page::Audio => "blightnet://audio",
            Page::Blackjack => "blightnet://blackjack",
            Page::Nethooks => "blightnet://nethooks",
            Page::Netspace => "blightnet://netspace",
            Page::Rotn => "blightnet://rotn",
            Page::Player => "blightnet://player",
            Page::Terminal => "blightnet://terminal",
            Page::Recon => "blightnet://recon",
            Page::Tree => "blightnet://tree",
            Page::Chars => "blightnet://chars",
            Page::Catalog(_) => "blightnet://catalog",
            Page::Boot => "blightnet://boot",
        }
    }

    fn page_name(&self) -> &'static str {
        match self.page {
            Page::Index => "INDEX",
            Page::Table => "TABLE",
            Page::Chars => "CHARS",
            Page::Tutorial => "TUTORIAL",
            Page::Audio => "AUDIO",
            Page::Blackjack => "BLACKJACK",
            Page::Nethooks => "NETHOOKS",
            Page::Netspace => "NETSPACE",
            Page::Rotn => "ROTN",
            Page::Player => "PLAYER",
            Page::Terminal => "TERMINAL",
            Page::Recon => "RECON",
            Page::Tree => "TREE",
            Page::Catalog(n) => n,
            Page::Boot => "BOOT",
        }
    }

    fn link_addr(&self) -> Option<String> {
        if let Some(a) = self.net.addrs.iter().find(|a| !a.starts_with("udp:")) {
            return Some(a.clone());
        }
        if self.node_live {
            Some(format!("127.0.0.1:{}", crate::daemon::IPC_PORT))
        } else {
            None
        }
    }

    fn paint_status_items(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            status_pair(ui, "LOC", self.page_loc(), DIM);
            status_pair(
                ui,
                "NODE",
                if self.node_live { "ACTIVE" } else { "OFFLINE" },
                if self.node_live { theme::ACID } else { KILL },
            );
            let link = match self.net.role {
                Role::Host => "HOSTING",
                Role::Guest => "JOINED",
                Role::Presence => "ONLINE",
                Role::Idle => "LOCAL",
            };
            status_pair(ui, "LINK", link, CREAM);
            if let Some(addr) = self.link_addr() {
                ui.label(
                    RichText::new(addr)
                        .family(theme::mono())
                        .size(10.0)
                        .color(CYAN),
                );
            }
            if self.table_live() {
                let n = 1 + self
                    .net
                    .peers
                    .iter()
                    .filter(|p| p.id != self.net.self_id)
                    .count();
                status_pair(ui, "SEATS", &n.to_string(), CREAM);
            }
            status_pair(ui, "PAGE", self.page_name(), CREAM);
            if let Some(line) = self.recv_clock_line() {
                ui.label(
                    RichText::new(line)
                        .family(theme::mono())
                        .size(10.0)
                        .color(CYAN),
                );
            }
            if let Some(line) = self.send_clock_line() {
                ui.label(
                    RichText::new(line)
                        .family(theme::mono())
                        .size(10.0)
                        .color(theme::ACID),
                );
            }
            if let Some(name) = self.deck_track_name() {
                let playing = self.media_run || (self.deck_on && self.mixer.deck_live());
                if quiet_btn(ui, if playing { "PLAY" } else { "DECK" }, DIM)
                    .on_hover_text("Play or pause this deck. Pictures, video, and PDF stay on this machine.")
                    .clicked()
                {
                    self.deck_toggle();
                }
                if quiet_btn(ui, &name, if playing { CYAN } else { DIM })
                    .on_hover_text("Play or pause this deck. Pictures, video, and PDF stay on this machine.")
                    .clicked()
                {
                    self.deck_toggle();
                }
            }
            ui.label(
                RichText::new(format!("{:>3.0} FPS", self.fps))
                    .family(theme::mono())
                    .size(10.0)
                    .color(if self.fps >= 59.0 { CYAN } else { KILL }),
            );
            if quiet_btn(ui, "Update", CREAM)
                .on_hover_text("Pull the latest Blightnet from GitHub, then restart.")
                .clicked()
            {
                self.do_update();
            }
        });
    }

    fn paint_meters(&self, ui: &mut egui::Ui) {
        let m = &self.machine;
        let (slot, _) = ui.allocate_exact_size(Vec2::new(360.0, 28.0), egui::Sense::hover());
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(slot)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        row.set_clip_rect(slot);
        row.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
        let cpu_bad = m.cpu >= 90.0;
        let cpu_hot = m.cpu >= 75.0;
        meter_pair(&mut row, "CPU", &format!("{:.0}%", m.cpu), cpu_hot, cpu_bad);
        let gpu = m.gpu.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
        let gpu_bad = m.gpu.unwrap_or(0.0) >= 90.0;
        let gpu_hot = m.gpu.unwrap_or(0.0) >= 75.0;
        meter_pair(&mut row, "GPU", &gpu, gpu_hot && m.gpu.is_some(), gpu_bad && m.gpu.is_some());
        let ram = if m.ram_total == 0 {
            "—".into()
        } else {
            format!("{}/{}", brief_bytes(m.ram_used), brief_bytes(m.ram_total))
        };
        let ram_frac = if m.ram_total == 0 {
            0.0
        } else {
            m.ram_used as f32 / m.ram_total as f32
        };
        meter_pair(&mut row, "RAM", &ram, ram_frac >= 0.75, ram_frac >= 0.90);
        let disk = if m.disk_total == 0 {
            "—".into()
        } else {
            brief_bytes(m.disk_free)
        };
        let free_frac = if m.disk_total == 0 {
            1.0
        } else {
            m.disk_free as f32 / m.disk_total as f32
        };
        meter_pair(&mut row, "DISK", &disk, free_frac < 0.15, free_frac < 0.05);
    }

    fn command_app_tabs(&mut self, ui: &mut egui::Ui) {
        // Side-rail destinations (dock). Same chrome_tab selected language as page tabs.
        // PLAYER here = dock library rail — full-page PLAYER lives on the page tab row.
        for (panel, label, tip) in [
            (
                ShellPanel::Chat,
                "CHAT",
                "Side rail · table talk, DMs, and files. Press again to close.",
            ),
            (
                ShellPanel::Contacts,
                "CONTACTS",
                "Side rail · people saved on this deck. Press again to close.",
            ),
            (
                ShellPanel::Voice,
                "VOICE",
                "Side rail · mic and calls. Press again to close.",
            ),
            (
                ShellPanel::Video,
                "VIDEO",
                "Side rail · video. Press again to close.",
            ),
            (
                ShellPanel::Player,
                "PLAYER",
                "Side rail · pictures, video, PDF, and music. Full page via Dock → Full page.",
            ),
        ] {
            let on = self.shell == panel;
            if tab(ui, label, on).on_hover_text(tip).clicked() {
                if on {
                    self.shell = ShellPanel::None;
                } else {
                    // Opening a dock rail leaves full-page Player if we were there.
                    if self.player_full {
                        self.player_full = false;
                        if self.page == Page::Player {
                            self.page = if self.player_back == Page::Player {
                                Page::Table
                            } else {
                                self.player_back
                            };
                        }
                    }
                    self.shell = panel;
                    if panel == ShellPanel::Voice {
                        self.ensure_mic();
                    }
                    if panel == ShellPanel::Video {
                        self.ensure_cameras();
                    }
                }
            }
        }
    }

    fn command_page_tabs(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        ui.horizontal(|ui| {
            // Primary destinations — same chrome_tab language as the side-rail row.
            if tab(ui, "INDEX", matches!(self.page, Page::Index))
                .on_hover_text("Deck home · session, link, systems, protocol.")
                .clicked()
            {
                self.page = Page::Index;
            }
            if tab(ui, "TABLE", self.page == Page::Table)
                .on_hover_text("Blightnexus mix · scenes, sheets, map, overlays.")
                .clicked()
            {
                self.page = Page::Table;
            }
            if tab(ui, "NETSPACE", self.page == Page::Netspace)
                .on_hover_text("Cruise the grid · co-presence when Online.")
                .clicked()
            {
                self.page = Page::Netspace;
                self.jack_at = Instant::now();
            }
            if tab(ui, "TREE", self.page == Page::Tree)
                .on_hover_text("Local filesystem on this deck.")
                .clicked()
            {
                self.page = Page::Tree;
            }
            if tab(ui, "TERMINAL", self.page == Page::Terminal)
                .on_hover_text("Local shell · nothing leaves this deck.")
                .clicked()
            {
                self.page = Page::Terminal;
            }
            if tab(ui, "RECON", self.page == Page::Recon)
                .on_hover_text("Local dossiers · people and companies.")
                .clicked()
            {
                self.page = Page::Recon;
            }
            if tab(ui, "NETHOOKS", self.page == Page::Nethooks)
                .on_hover_text("User pages on the grid · handouts, rumors, jobs.")
                .clicked()
            {
                self.page = Page::Nethooks;
                self.hook_edit = false;
            }
            if tab(ui, "ROTN", self.page == Page::Rotn)
                .on_hover_text("Local fixer · rumors and jobs stay on this computer.")
                .clicked()
            {
                self.page = Page::Rotn;
            }
            // Full-page Player only when expanded — avoids twin PLAYER destinations.
            if self.player_full
                && tab(ui, "PLAYER", self.page == Page::Player)
                    .on_hover_text("Full-page library · Dock returns to the side rail.")
                    .clicked()
            {
                self.page = Page::Player;
            }
        });
    }

    fn draw_command_bar(&mut self, ui: &mut egui::Ui) {
        let h = 44.0;
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(1.0), h),
            egui::Sense::hover(),
        );
        let ch_bar = theme::chrome();
        ui.painter().rect_filled(rect, 0.0, ch_bar.rail);
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 1.0,
            egui::Stroke::new(1.0, ch_bar.acid),
        );
        let mut bar = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(8.0, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        bar.set_clip_rect(rect);
        bar.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

        let (mark, mark_resp) =
            bar.allocate_exact_size(Vec2::new(112.0, 36.0), egui::Sense::click_and_drag());
        let ch_mark = theme::chrome();
        bar.painter().text(
            mark.left_center() + Vec2::new(0.0, -3.0),
            egui::Align2::LEFT_CENTER,
            "BLIGHTNET",
            FontId::new(15.0, theme::display()),
            ch_mark.acid,
        );
        bar.painter().rect_filled(
            Rect::from_min_size(
                mark.left_bottom() + Vec2::new(0.0, -8.0),
                Vec2::new(64.0, 2.0),
            ),
            0.0,
            ch_mark.cyan,
        );
        if mark_resp.drag_started() {
            bar.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if mark_resp.double_clicked() {
            let maxed = bar.ctx().input(|i| i.viewport().maximized.unwrap_or(true));
            bar.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
        }

        bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            if theme::neon_btn_color(ui, "×", KILL, true).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            let maxed = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(true));
            if theme::neon_btn(ui, if maxed { "❐" } else { "□" }).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
            }
            if self.node_live {
                if theme::neon_btn_color(ui, "Online", theme::chrome().acid, true).clicked() {
                    self.go_offline();
                }
            } else if theme::neon_btn(ui, "Online").clicked() {
                self.go_online();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.handle)
                    .desired_width(96.0)
                    .hint_text("Handle")
                    .font(FontId::new(13.0, theme::ui_font()))
                    .text_color(CREAM),
            );
            self.paint_meters(ui);
            self.command_app_tabs(ui);
            let (grip, grip_resp) =
                ui.allocate_exact_size(Vec2::new(14.0, 22.0), egui::Sense::click_and_drag());
            for i in 0..3 {
                let x = grip.left() + 2.0 + i as f32 * 4.0;
                ui.painter().vline(
                    x,
                    (grip.top() + 4.0)..=(grip.bottom() - 4.0),
                    egui::Stroke::new(1.0, theme::fade(theme::chrome().acid, 150)),
                );
            }
            if grip_resp.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if grip_resp.double_clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
            }
            let mid_w = ui.available_width().max(8.0);
            let (mid, _) = ui.allocate_exact_size(Vec2::new(mid_w, 40.0), egui::Sense::hover());
            let mut tabs = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(mid)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            tabs.set_clip_rect(mid);
            self.command_page_tabs(&mut tabs);
        });
    }



    fn ui_index(&mut self, ui: &mut egui::Ui, t: f32) {
        let page_h = ui.available_height().max(40.0);
        let ch = theme::chrome();
        egui::ScrollArea::vertical()
            .id_salt("index")
            .max_height(page_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                ui.add_space(8.0);

                // B: status-first deck strip — icon | NETDIR + DECK + NODE | world | theme | kicker
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(12.0);
                    let icon = self.root.join("assets/icon.png");
                    let (badge, _) =
                        ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::hover());
                    theme::fill_chamfer(
                        ui,
                        badge,
                        10.0,
                        Color32::from_rgb(8, 8, 5),
                        egui::Stroke::new(1.5, ch.acid),
                    );
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(badge.shrink(4.0))
                            .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)),
                    );
                    images::show_fit(&mut child, &mut self.tex, &icon, Vec2::splat(46.0));

                    ui.vertical(|ui| {
                        let netdir = match self.net.role {
                            Role::Host => "NETDIR://HOST · BLIGHTNET DECK",
                            Role::Guest => "NETDIR://JOIN · BLIGHTNET DECK",
                            Role::Presence => "NETDIR://ONLINE · BLIGHTNET DECK",
                            Role::Idle => "NETDIR://LOCAL · BLIGHTNET DECK",
                        };
                        ui.label(
                            RichText::new(netdir)
                                .family(theme::mono())
                                .size(11.0)
                                .color(ch.cyan),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("DECK")
                                    .family(theme::display())
                                    .size(34.0)
                                    .color(ch.acid),
                            );
                            let node = match self.net.role {
                                Role::Host => "HOST",
                                Role::Guest => "JOIN",
                                Role::Presence => "ONLINE",
                                Role::Idle => "LOCAL",
                            };
                            let node_col = if self.node_live { ch.acid } else { ch.cyan };
                            egui::Frame::NONE
                                .fill(theme::glass())
                                .stroke(egui::Stroke::new(1.0, theme::fade(node_col, 200)))
                                .inner_margin(egui::Margin::symmetric(10, 4))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("NODE · {node}"))
                                            .family(theme::mono())
                                            .size(12.0)
                                            .color(node_col),
                                    );
                                });
                        });
                        ui.horizontal_wrapped(|ui| {
                            let world = if self.blight { "BLIGHT" } else { "HEARTHSONG" };
                            if theme::neon_btn_color(ui, world, ch.cyan, self.blight)
                                .on_hover_text(
                                    "Switch Hearthsong and Blight. The mix goes quiet and Place resets.",
                                )
                                .clicked()
                            {
                                self.set_world(!self.blight);
                                self.broadcast_mix();
                            }
                            let theme_label = format!("THEME · {}", self.chrome.short());
                            let theme_resp = theme::neon_btn(ui, &theme_label).on_hover_text(
                                "Deck chrome. Neon Deck / Void Deeper / Acid Forward / Cyan Ice. Right-click cycles.",
                            );
                            if theme_resp.clicked() {
                                self.theme_pick = !self.theme_pick;
                            }
                            if theme_resp.secondary_clicked() {
                                self.set_chrome_theme(self.chrome.next());
                            }
                            meta_c(ui, "CLK", &format_clock(self.clock), ch.cyan);
                            meta_c(ui, "ICE", "CLEAR", ch.cyan);
                        });
                        if self.theme_pick {
                            ui.horizontal_wrapped(|ui| {
                                for t in theme::ChromeTheme::ALL {
                                    let on = self.chrome == t;
                                    if theme::neon_btn_color(ui, t.label(), ch.cyan, on).clicked()
                                    {
                                        self.set_chrome_theme(t);
                                    }
                                }
                            });
                        }
                        theme::kicker(ui, "Stamp a Handle · Online · Host or Join · JACK IN");
                    });
                });

                ui.add_space(4.0);
                self.ui_index_session(ui);

                if !self.err.is_empty() {
                    wrap_text(ui, &self.err.clone(), ch.cyan, 13.0);
                }
                ui.add_space(4.0);

                // A zones: PRIMARY LINK | DECK SYSTEMS | RUN PROTOCOL (+ settings strip under systems)
                let wide = ui.available_width() >= 980.0;
                if wide {
                    ui.columns(3, |cols| {
                        self.ui_index_link(&mut cols[0], t);
                        self.ui_index_systems(&mut cols[1]);
                        self.ui_index_protocol(&mut cols[2]);
                    });
                } else if ui.available_width() >= 640.0 {
                    ui.columns(2, |cols| {
                        self.ui_index_link(&mut cols[0], t);
                        self.ui_index_systems(&mut cols[1]);
                    });
                    ui.add_space(8.0);
                    self.ui_index_protocol(ui);
                } else {
                    self.ui_index_link(ui, t);
                    ui.add_space(8.0);
                    self.ui_index_systems(ui);
                    ui.add_space(8.0);
                    self.ui_index_protocol(ui);
                }
                ui.add_space(8.0);
                self.ui_index_log(ui);
                ui.add_space(10.0);
            });
    }


    /// Process-manager strip: role, peers, probe RTT, host path health, daemon.
    /// Reads existing App/net/daemon fields only — no STUN/UPnP reimplementation.
    fn ui_index_session(&self, ui: &mut egui::Ui) {
        let ch = theme::chrome();
        theme::pane().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                theme::section_head(ui, "SESSION", "NODE");
                theme::kicker(ui, "Live readout · same signals as Tree / Netspace");
            });
            ui.add_space(2.0);

            let (role_lbl, role_col) = if !self.node_live {
                ("OFFLINE", theme::DIM)
            } else {
                match self.net.role {
                    Role::Host => ("HOST", theme::ORANGE),
                    Role::Guest => ("JOIN", theme::ORANGE),
                    Role::Presence | Role::Idle => ("ONLINE", ch.cyan),
                }
            };
            let (daemon_lbl, daemon_col) = if self.node_live {
                ("UP", ch.acid)
            } else {
                ("DOWN", KILL)
            };

            let peers: Vec<&net::PeerInfo> = self
                .net
                .peers
                .iter()
                .filter(|p| p.id != self.net.self_id)
                .collect();
            let peer_n = peers.len();
            let peer_handles: String = {
                let names: Vec<&str> = peers
                    .iter()
                    .map(|p| {
                        let n = p.name.trim();
                        if n.is_empty() {
                            p.id.as_str()
                        } else {
                            n
                        }
                    })
                    .take(4)
                    .collect();
                if names.is_empty() {
                    "—".into()
                } else {
                    let mut s = names.join(" · ");
                    if peer_n > names.len() {
                        s.push_str(&format!(" +{}", peer_n - names.len()));
                    }
                    s
                }
            };
            let peers_val = if peer_n == 0 {
                "0".into()
            } else {
                format!("{peer_n} · {peer_handles}")
            };
            let peers_col = if peer_n == 0 { MUTED } else { ch.cyan };

            let (rtt_val, rtt_col) = match self.netspace_ping_ms() {
                Some(ms) => {
                    let col = if ms < 80 {
                        ch.acid
                    } else if ms < 160 {
                        ch.cyan
                    } else if ms < 300 {
                        DIM
                    } else {
                        KILL
                    };
                    (format!("{ms} ms"), col)
                }
                None => ("—".into(), MUTED),
            };

            let (path_val, path_col) = session_path_readout(
                self.node_live,
                self.net.role,
                self.net.internet,
                &self.status,
                &self.path_health,
            );

            ui.horizontal_wrapped(|ui| {
                session_chip(ui, "ROLE", role_lbl, role_col);
                session_chip(ui, "DAEMON", daemon_lbl, daemon_col);
                session_chip(ui, "PEERS", &peers_val, peers_col);
                session_chip(ui, "PROBE", &rtt_val, rtt_col);
                session_chip(ui, "PATH", &path_val, path_col);
            });
        });
    }

    fn ui_index_link(&mut self, ui: &mut egui::Ui, t: f32) {
        self.ui_index_link_inner(ui, t);
    }

    fn ui_index_link_inner(&mut self, ui: &mut egui::Ui, t: f32) {
        let ch = theme::chrome();
        theme::section_head(ui, "01", "PRIMARY LINK");
        if theme::jack_tile(ui, t).clicked() {
            self.page = Page::Table;
        }
        wrap_text(
            ui,
            match self.net.role {
                Role::Host => "HOSTING · FRIENDS PASTE YOUR ADDRESS",
                Role::Guest => "JOINED · YOU FOLLOW THE HOST MIX",
                Role::Presence => "ONLINE · CONTACTS CAN DM AND CALL",
                Role::Idle => "READY · LOCAL NODE · PRESS TO OPEN BLIGHTNEXUS",
            },
            DIM,
            10.0,
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Host").clicked() {
                self.shell = ShellPanel::Host;
            }
            if theme::neon_btn(ui, "Join").clicked() {
                self.shell = ShellPanel::Join;
            }
        });
        if self.net.role == Role::Host {
            wrap_text(ui, &self.net.paste_link(), ch.cyan, 11.0);
        }
    }

    fn ui_index_systems(&mut self, ui: &mut egui::Ui) {
        self.ui_index_systems_inner(ui);
    }

    fn ui_index_systems_inner(&mut self, ui: &mut egui::Ui) {
        theme::section_head(ui, "02", "DECK SYSTEMS");
        ui.columns(2, |g| {
            if theme::sys_tile(&mut g[0], "02", "CONTACTS", "SAVED PEOPLE", "OPEN", false).clicked()
            {
                self.shell = ShellPanel::Contacts;
            }
            if theme::sys_tile(&mut g[1], "03", "TUTORIAL", "FIELD MANUAL", "READ", false).clicked()
            {
                self.page = Page::Tutorial;
            }
        });
        ui.columns(2, |g| {
            if theme::sys_tile(&mut g[0], "04", "CHAT", "TABLE TALK", "OPEN", false).clicked() {
                self.shell = ShellPanel::Chat;
            }
            if theme::sys_tile(&mut g[1], "05", "VOICE", "MIC AND CALLS", "TUNE", false).clicked() {
                self.shell = ShellPanel::Voice;
                self.ensure_mic();
            }
        });
        if theme::sys_tile(ui, "06", "BLACKJACK", "HOUSE GAME", "PLAY", false).clicked() {
            self.page = Page::Table;
            if !self.panel_on(Overlay::Blackjack) {
                self.toggle_overlay(Overlay::Blackjack);
            }
        }
        if theme::sys_tile(ui, "07", "VIDEO CALL", "CONTACTS · CREWS", "OPEN", false).clicked() {
            self.cameras = crate::video::list_cameras();
            self.shell = ShellPanel::Video;
        }
        if theme::sys_tile(ui, "08", "NETHOOKS", "USER SITES ON THE GRID", "OPEN", false).clicked()
        {
            self.page = Page::Nethooks;
            self.hook_edit = false;
        }
        if theme::sys_tile(
            ui,
            "09",
            "LEDGER",
            "TRUST · WIRE VS LOCAL",
            if self.index_ledger { "HIDE" } else { "READ" },
            false,
        )
        .clicked()
        {
            self.index_ledger = !self.index_ledger;
        }
        ui.add_space(8.0);
        // DISCONNECT isolated at bottom of systems (A)
        if theme::sys_tile(ui, "00", "DISCONNECT", "SHUT DOWN BLIGHTNET", "KILL", true).clicked() {
            self.go_offline();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ui.add_space(6.0);
        self.ui_index_settings(ui);
        if self.index_ledger {
            ui.add_space(6.0);
            self.ui_index_ledger(ui);
        }
    }

    fn ui_index_settings(&mut self, ui: &mut egui::Ui) {
        self.ensure_devices();
        self.ensure_cameras();
        let ch = theme::chrome();
        theme::section_head(ui, "SET", "SETTINGS");
        wrap_text(
            ui,
            &format!(
                "Devices · {} mic{} · {} speaker{}",
                self.inputs.len(),
                if self.inputs.len() == 1 { "" } else { "s" },
                self.outputs.len(),
                if self.outputs.len() == 1 { "" } else { "s" },
            ),
            MUTED,
            11.0,
        );
        let inputs = self.inputs.clone();
        let outputs = self.outputs.clone();
        let mut mic = self.mic_name.clone();
        let mut spk = self.speaker_name.clone();
        let mut cam = self.cam_name.clone();
        let cams = self.cameras.clone();
        let w = ui.available_width().max(80.0);
        let mic_label = inputs
            .iter()
            .find(|d| d.id == mic || d.alt == mic)
            .map(|d| d.label.clone())
            .unwrap_or_else(|| {
                if mic.is_empty() {
                    "System default mic".into()
                } else {
                    mic.clone()
                }
            });
        egui::ComboBox::from_id_salt("mic")
            .selected_text(mic_label)
            .width(w)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mic, String::new(), "System default mic");
                for d in &inputs {
                    ui.selectable_value(&mut mic, d.id.clone(), &d.label);
                }
            });
        let spk_label = outputs
            .iter()
            .find(|d| d.id == spk || d.alt == spk)
            .map(|d| d.label.clone())
            .unwrap_or_else(|| {
                if spk.is_empty() {
                    "System default speaker".into()
                } else {
                    spk.clone()
                }
            });
        egui::ComboBox::from_id_salt("spk")
            .selected_text(spk_label)
            .width(w)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut spk, String::new(), "System default speaker");
                for d in &outputs {
                    ui.selectable_value(&mut spk, d.id.clone(), &d.label);
                }
            });
        let cam_label = cams
            .iter()
            .find(|c| c.path == cam)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| {
                if cam.is_empty() {
                    "System default camera".into()
                } else {
                    cam.clone()
                }
            });
        egui::ComboBox::from_id_salt("cam")
            .selected_text(cam_label)
            .width(w)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut cam, String::new(), "System default camera");
                for c in &cams {
                    ui.selectable_value(&mut cam, c.path.clone(), &c.name);
                }
            });
        if mic != self.mic_name {
            self.set_mic(mic);
        }
        if spk != self.speaker_name {
            self.set_speaker(spk);
        }
        if cam != self.cam_name {
            self.set_camera(cam);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("UI SCALE")
                    .family(theme::mono())
                    .size(11.0)
                    .color(ch.acid),
            );
            ui.label(
                RichText::new(format!("{:.2}×", self.ui_scale))
                    .family(theme::mono())
                    .size(11.0)
                    .color(ch.cyan),
            );
        });
        {
            let mut scale = self.ui_scale;
            let slider_w = w.max(160.0);
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = slider_w;
                ui.spacing_mut().interact_size.y = 28.0;
                let resp = ui
                    .add(
                        egui::Slider::new(&mut scale, UI_SCALE_MIN..=UI_SCALE_MAX)
                            .step_by(0.05)
                            .show_value(false),
                    )
                    .on_hover_text(
                        "Deck UI scale for Steam Deck / handheld. Touch-drag. Saved with chrome theme.",
                    );
                if resp.changed() {
                    self.set_ui_scale(scale);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Rescan devices").clicked() {
                self.refresh_devices();
            }
            if theme::neon_btn(ui, "Update").clicked() {
                self.do_update();
            }
            if self.node_live {
                if theme::neon_btn_color(ui, "Go offline", KILL, false).clicked() {
                    self.go_offline();
                }
            } else if theme::neon_btn(ui, "Go online").clicked() {
                self.go_online();
            }
        });
    }

    fn ui_index_ledger(&mut self, ui: &mut egui::Ui) {
        let ch = theme::chrome();
        let wire = theme::ORANGE;
        theme::pane().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("09 · TRUST LEDGER")
                        .family(theme::mono())
                        .size(11.0)
                        .color(ch.acid),
                );
                let (chip, chip_col) = if !self.node_live {
                    ("OFFLINE", theme::DIM)
                } else {
                    match self.net.role {
                        Role::Host => ("HOST", wire),
                        Role::Guest => ("JOIN", wire),
                        Role::Presence | Role::Idle => ("ONLINE", ch.cyan),
                    }
                };
                egui::Frame::NONE
                    .fill(theme::glass())
                    .stroke(egui::Stroke::new(1.0, theme::fade(chip_col, 200)))
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(chip)
                                .family(theme::mono())
                                .size(11.0)
                                .color(chip_col),
                        );
                    });
            });
            wrap_text(
                ui,
                "Readout only — not a permission toggle. What leaves this deck vs what stays.",
                MUTED,
                11.0,
            );
            ui.add_space(6.0);

            let leaves: &[&str] = &[
                "chat · whispers",
                "voice · calls · PCM",
                "video · frames",
                "files · images",
                "net-pos",
                "character sheets",
                "map tokens · fog · marks · image asks",
                "table mix",
                "posted nethooks",
                "peer presence · Pit · Probe · Share",
            ];
            let local: &[&str] = &[
                "TREE",
                "TERM",
                "RECON",
                "ROTN · GGUF · 127.0.0.1",
                "PLAYER playback",
                "unposted · private nethooks",
                "GM role chrome",
                "daemon IPC · loopback",
            ];

            let wide = ui.available_width() >= 280.0;
            if wide {
                ui.columns(2, |cols| {
                    cols[0].label(
                        RichText::new("LEAVES · WIRE")
                            .family(theme::mono())
                            .size(11.0)
                            .color(wire),
                    );
                    for line in leaves {
                        wrap_text(&mut cols[0], line, theme::CREAM, 11.0);
                    }
                    cols[1].label(
                        RichText::new("LOCAL")
                            .family(theme::mono())
                            .size(11.0)
                            .color(ch.acid),
                    );
                    for line in local {
                        wrap_text(&mut cols[1], line, theme::CREAM, 11.0);
                    }
                });
            } else {
                ui.label(
                    RichText::new("LEAVES · WIRE")
                        .family(theme::mono())
                        .size(11.0)
                        .color(wire),
                );
                for line in leaves {
                    wrap_text(ui, line, theme::CREAM, 11.0);
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new("LOCAL")
                        .family(theme::mono())
                        .size(11.0)
                        .color(ch.acid),
                );
                for line in local {
                    wrap_text(ui, line, theme::CREAM, 11.0);
                }
            }

            ui.add_space(6.0);
            ui.label(
                RichText::new("ALSO ON THE WIRE")
                    .family(theme::mono())
                    .size(11.0)
                    .color(ch.cyan),
            );
            wrap_text(
                ui,
                "invite blightnet:// · LAN beacon id+handle+port · mesh/TCP HELLO ECDH · Host STUN/ipify/UPnP · Update→GitHub (user)",
                MUTED,
                11.0,
            );
            ui.add_space(4.0);
            wrap_text(
                ui,
                "Table traffic is peer-to-peer with invite key + ECDH/ChaCha — not a cloud hub. Helpers are reachability only. Host-shared mix/maps on host disk; chat pics/voice P2P not on a hub.",
                ch.cyan,
                11.0,
            );
            match (self.node_live, self.net.role) {
                (false, _) => wrap_text(
                    ui,
                    "OFFLINE · no peer wire, beacon, STUN, ipify, or UPnP.",
                    MUTED,
                    11.0,
                ),
                (true, Role::Host) => wrap_text(
                    ui,
                    "HOST · full peer LEAVES; internet host also STUN/ipify/UPnP.",
                    wire,
                    11.0,
                ),
                (true, Role::Guest) => wrap_text(
                    ui,
                    "JOIN · full peer LEAVES once at the table.",
                    wire,
                    11.0,
                ),
                (true, _) => wrap_text(
                    ui,
                    "ONLINE · beacon+listen; no full table sync until Host or Join.",
                    ch.cyan,
                    11.0,
                ),
            };
        });
    }

    fn ui_index_log(&mut self, ui: &mut egui::Ui) {
        let ch = theme::chrome();
        theme::pane().show(ui, |ui| {
                ui.label(
                    RichText::new("DECK LOG")
                        .family(theme::mono())
                        .size(11.0)
                        .color(ch.acid),
                );
                ui.label(
                    RichText::new("one date · everything shipped that day")
                        .family(theme::mono())
                        .size(11.0)
                        .color(DIM),
                );
                ui.add_space(6.0);
                if self.changelog.is_empty() {

                    wrap_text(ui, "No changelog on this deck.", MUTED, 14.0);
                }
                for (i, (day, bullets)) in self.changelog.iter().take(3).enumerate() {
                    if i > 0 {
                        ui.add_space(12.0);
                    }
                    ui.label(
                        RichText::new(day)
                            .family(theme::display())
                            .size(16.0)
                            .color(ch.acid),
                    );
                    for b in bullets {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("▸").color(ch.cyan).size(14.0));
                            wrap_text(ui, b, CREAM, 14.0);
                        });
                    }
                }
            });
    }

    fn ui_index_protocol(&mut self, ui: &mut egui::Ui) {
        let ch = theme::chrome();
        let handle_ok = !self.handle.trim().is_empty();
        let online_ok = self.node_live;
        let table_ok = matches!(self.net.role, Role::Host | Role::Guest);
        let mix_ok = matches!(self.page, Page::Table);
        let talk_ok = self.shell != ShellPanel::None || self.chat.len() > 1;
        let player_ok = !self.deck_list.is_empty();
        theme::pane().show(ui, |ui| {
                theme::section_head(ui, "03", "RUN PROTOCOL");
                ui.add_space(4.0);
                let steps: [(&str, bool); 6] = [
                    ("Stamp a Handle in the command bar.", handle_ok),
                    (
                        "Press Online. Nothing listens until you do. Press again to stop the node.",
                        online_ok,
                    ),
                    (
                        "Host or Join a table. Closing the window keeps the table. INDEX 00 ends the node.",
                        table_ok,
                    ),
                    (
                        "Press 01 BLIGHTNEXUS or the TABLE tab to mix. NETSPACE is its own tab.",
                        mix_ok,
                    ),
                    (
                        "CHAT · CONTACTS · VOICE · VIDEO · PLAYER open the side rail. Press again to close. Same tabs live inside the dock.",
                        talk_ok,
                    ),
                    (
                        "PLAYER rail is the library. Full page via Full page. Status-line track name plays or pauses.",
                        player_ok,
                    ),
                ];
                for (n, (step, done)) in steps.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (r, _) =
                            ui.allocate_exact_size(Vec2::new(22.0, 18.0), egui::Sense::hover());
                        let fill = if *done {
                            theme::fade(ch.acid, 80)
                        } else {
                            PANEL
                        };
                        let stroke = if *done {
                            ch.acid
                        } else {
                            theme::fade(ch.hot, 140)
                        };
                        theme::fill_chamfer(
                            ui,
                            r,
                            3.0,
                            fill,
                            egui::Stroke::new(1.0, stroke),
                        );
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            if *done { "OK".into() } else { format!("{n:02}") },
                            FontId::new(10.0, theme::mono()),
                            if *done { ch.acid } else { ch.cyan },
                        );
                        wrap_text(
                            ui,
                            step,
                            if *done { MUTED } else { theme::CREAM },
                            13.0,
                        );
                    });
                    ui.add_space(5.0);
                }
                wrap_text(
                    ui,
                    "Bestiary, Datashard, maps, and the mix live on Blightnexus. Contacts remember people you add. Crews are group chats.",
                    MUTED,
                    11.0,
                );
                if let Some(last) = self.chat.last() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("LAST LINE")
                            .family(theme::mono())
                            .size(11.0)
                            .color(ch.acid),
                    );
                    wrap_text(ui, last, ch.cyan, 11.0);
                }
            });
    }

    fn ui_dock(&mut self, ui: &mut egui::Ui) {
        // Same selected chrome as command_app_tabs (proposal C). HOST/JOIN stay dock-only.
        theme::kicker(ui, "DOCK://RAIL");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            for (label, panel) in [
                ("CHAT", ShellPanel::Chat),
                ("CONTACTS", ShellPanel::Contacts),
                ("VOICE", ShellPanel::Voice),
                ("VIDEO", ShellPanel::Video),
                ("PLAYER", ShellPanel::Player),
                ("HOST", ShellPanel::Host),
                ("JOIN", ShellPanel::Join),
            ] {
                let on = self.shell == panel;
                if tab(ui, label, on).clicked() {
                    if on {
                        self.shell = ShellPanel::None;
                    } else {
                        self.shell = panel;
                        if panel == ShellPanel::Voice {
                            self.ensure_mic();
                        }
                        if panel == ShellPanel::Video {
                            self.cameras = crate::video::list_cameras();
                        }
                    }
                }
            }
        });
        ui.add_space(6.0);
        let dock_h = ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("dock")
            .max_height(dock_h)
            .auto_shrink([false, false])
            .show(ui, |ui| match self.shell {
                ShellPanel::Host => self.ui_host_panel(ui),
                ShellPanel::Join => self.ui_join_panel(ui),
                ShellPanel::Chat => self.ui_chat_panel(ui),
                ShellPanel::Contacts => self.ui_contacts_panel(ui),
                ShellPanel::Voice => self.ui_voice_panel(ui),
                ShellPanel::Video => self.ui_video_panel(ui),
                ShellPanel::Player => self.ui_player_panel(ui),
                ShellPanel::None => {}
            });
    }

    fn toggle_player_page(&mut self) {
        if self.player_full {
            let back = self.player_back;
            self.player_full = false;
            self.page = if back == Page::Player { Page::Table } else { back };
            self.shell = ShellPanel::Player;
        } else {
            if self.page != Page::Player {
                self.player_back = self.page;
            }
            self.player_full = true;
            self.page = Page::Player;
            self.shell = ShellPanel::None;
        }
    }

    fn ui_player_panel(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
        let wide = self.player_full
            && ui.available_width() > 900.0
            && ui.available_height().is_finite()
            && ui.available_height() > 420.0;
        if wide {
            let rect = ui.available_rect_before_wrap();
            ui.allocate_rect(rect, egui::Sense::hover());
            let (left, right) = rect.split_left_right_at_x(rect.left() + rect.width() * 0.60);
            let left = left.shrink2(Vec2::new(8.0, 4.0));
            let right = right.shrink2(Vec2::new(8.0, 4.0));
            let mut stage = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(left)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            stage.set_clip_rect(left);
            egui::ScrollArea::vertical()
                .id_salt("player-page-stage")
                .max_height(left.height().max(40.0))
                .auto_shrink([false, false])
                .show(&mut stage, |ui| {
                    ui.set_width((left.width() - 12.0).max(40.0));
                    self.ui_player_stage(ui, true);
                });
            let mut side = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(right)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            side.set_clip_rect(right);
            egui::ScrollArea::vertical()
                .id_salt("player-page-side")
                .max_height(right.height().max(40.0))
                .auto_shrink([false, false])
                .show(&mut side, |ui| {
                    ui.set_width((right.width() - 12.0).max(40.0));
                    self.ui_player_library(ui, true);
                });
            return;
        }
        if self.player_full {
            let rect = ui.available_rect_before_wrap();
            ui.allocate_rect(rect, egui::Sense::hover());
            let mut page = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            page.set_clip_rect(rect);
            egui::ScrollArea::vertical()
                .id_salt("player-page-stack")
                .max_height(rect.height().max(40.0))
                .auto_shrink([false, false])
                .show(&mut page, |ui| {
                    ui.set_width((rect.width() - 16.0).max(40.0));
                    self.ui_player_stage(ui, true);
                    self.ui_player_library(ui, true);
                });
            return;
        }
        self.ui_player_stage(ui, false);
        self.ui_player_library(ui, false);
    }

    fn ui_player_stage(&mut self, ui: &mut egui::Ui, full: bool) {
        theme::page_chrome(
            ui,
            if full { "PLAYER://PAGE" } else { "PLAYER://DOCK" },
            "PLAYER",
            if full {
                "Pick a track · Dock returns the side rail"
            } else {
                "Pick a track · Full page for stage + library"
            },
        );
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, if self.player_full { "Dock" } else { "Full page" }).clicked() {
                self.toggle_player_page();
            }
        });
        let kind = self.current_kind();
        if kind == "audio" || self.radio_on {
            let wave = self.mixer.viz_wave();
            paint_deck_viz(ui, &wave);
            ui.add_space(8.0);
        }
        wrap_text(
            ui,
            "Pictures, gifs, text, video, PDF, and music on this deck. Send is the only way a file leaves this computer, and it does not change the mix.",
            MUTED,
            12.0,
        );
        ui.add_space(6.0);
        self.paint_media_stage(ui, full);
        ui.add_space(6.0);
        self.ui_seek_bar(ui);
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Prev").clicked() {
                self.deck_prev();
            }
            let playing = self.media_run || self.gif_run || (self.deck_on && self.mixer.deck_live());
            if theme::neon_btn_color(ui, if playing { "Pause" } else { "Play" }, CYAN, playing)
                .clicked()
            {
                self.deck_toggle();
            }
            if theme::neon_btn(ui, "Next").clicked() {
                self.deck_next(false);
            }
            if theme::neon_btn_color(ui, "Shuffle", theme::ACID, self.deck_shuffle).clicked() {
                self.deck_shuffle = !self.deck_shuffle;
                self.deck_bag.clear();
                if self.deck_shuffle && self.playable_indices().len() <= 1 {
                    self.deck_note = "Nothing else to shuffle.".into();
                } else {
                    self.deck_note.clear();
                }
            }
        });
        if !self.deck_note.is_empty() {
            wrap_text(ui, &self.deck_note, CYAN, 12.0);
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn_color(ui, "Tags", CYAN, self.tags_open).clicked() {
                self.open_tags();
            }
            if theme::neon_btn(ui, "Send").clicked() {
                self.open_file_send();
            }
            if let Some(line) = self.send_clock_line() {
                wrap_text(ui, &line, CYAN, 12.0);
            }
            if theme::neon_btn(ui, "Restore last").clicked() {
                self.restore_selected();
            }
            if self.current_kind() == "text" {
                if theme::neon_btn(ui, "Save").clicked() {
                    self.save_text();
                }
                if theme::neon_btn_color(ui, "Discard", KILL, false).clicked() {
                    self.discard_text();
                }
            }
        });
        self.paint_tags(ui);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("VOL")
                    .family(theme::mono())
                    .size(10.0)
                    .color(DIM),
            );
            let mut vol = self.deck_vol;
            if ui
                .add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                .changed()
            {
                self.deck_vol = vol;
                self.mixer.set_deck_vol(vol);
                save_deck_vol(&self.root, vol);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Add files").clicked() {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter(
                        "Media",
                        &["ogg", "mp3", "wav", "flac", "opus", "m4a", "aac", "png", "jpg", "jpeg", "webp", "gif", "mp4", "webm", "mkv", "pdf", "txt", "md", "log", "csv", "json", "toml"],
                    )
                    .set_title("Add pictures, video, PDF, or music")
                    .pick_files()
                {
                    self.add_deck_paths(files);
                }
            }
            if theme::neon_btn(ui, "Add folder").clicked() {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Add a media folder")
                    .pick_folder()
                {
                    self.add_deck_folder(dir);
                }
            }
            if !self.deck_list.is_empty()
                && theme::neon_btn_color(ui, "Clear", KILL, false).clicked()
            {
                self.mixer.deck_stop();
                self.deck_on = false;
                self.stop_media_proc();
                self.media_run = false;
                self.media_rx = None;
                self.media_msg.clear();
                self.deck_list.clear();
                self.deck_i = 0;
                self.deck_bag.clear();
                self.deck_base = 0.0;
                self.deck_note.clear();
                self.seek_drag = None;
                save_deck_lib(&self.root, &self.deck_list);
            }
        });
    }

    fn ui_player_library(&mut self, ui: &mut egui::Ui, full: bool) {
        ui.label(
            RichText::new("STATIONS")
                .family(theme::mono())
                .size(11.0)
                .color(theme::ACID),
        );
        wrap_text(
            ui,
            "A dot sits on every station. Acid means this deck is playing it. Cyan is on air. Red is off air. Dim has not been checked yet.",
            MUTED,
            11.0,
        );
        paint_air_legend(ui);
        if self.radio_on && theme::neon_btn_color(ui, "Stop", KILL, false).clicked() {
            self.stop_radio();
        }
        let station_h = if full { 240.0 } else { 150.0 };
        egui::ScrollArea::vertical()
            .id_salt("player-stations")
            .max_height(station_h)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for st in crate::stations::all() {
                    let on = self.radio_on && self.radio_station == st.id;
                    if station_row(ui, st.name, self.air_word(st.id), on, self.air_color(st.id)).clicked()
                        && !on
                    {
                        self.tune_station(st.id);
                    }
                }
            });
        ui.add_space(8.0);
        if self.deck_list.is_empty() {
            wrap_text(
                ui,
                "No files yet. Add pictures, video, PDF, or music from this machine.",
                MUTED,
                13.0,
            );
            return;
        }
        let n = self.deck_list.len();
        wrap_text(
            ui,
            &format!("{n} FILE{}", if n == 1 { "" } else { "S" }),
            CYAN,
            11.0,
        );
        let names: Vec<(usize, String)> = self
            .deck_list
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p
                    .file_stem()
                    .or_else(|| p.file_name())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.display().to_string());
                (i, name)
            })
            .collect();
        let kinds: Vec<&'static str> = self.deck_list.iter().map(|p| media_kind(p)).collect();
        for (i, name) in names {
            let kind = kinds.get(i).copied().unwrap_or("audio");
            let selected = self.deck_i == i;
            let on = selected && (self.deck_on || self.media_run || self.gif_run || kind != "audio");
            let sub = if selected {
                match kind {
                    "video" if self.media_run => "PLAYING",
                    "video" => "VIDEO",
                    "gif" if self.gif_run => "PLAYING",
                    "gif" => "GIF",
                    "image" => "PICTURE",
                    "pdf" => "PDF",
                    "text" => "TEXT",
                    _ if self.deck_on => "NOW PLAYING",
                    _ => "SELECTED",
                }
            } else {
                match kind {
                    "video" => "VIDEO",
                    "gif" => "GIF",
                    "image" => "PICTURE",
                    "pdf" => "PDF",
                    "text" => "TEXT",
                    _ => "",
                }
            };
            if theme::wide_btn(ui, &name, sub, on).clicked() {
                if i != self.deck_i && !self.gate_text(TextMove::Index(i)) {
                    return;
                }
                self.deck_i = i;
                self.media_page = 1;
                self.media_msg.clear();
                self.deck_base = 0.0;
                self.media_offset = 0.0;
                self.seek_drag = None;
                if self.current_kind() != "text" {
                    self.deck_note.clear();
                }
                self.deck_play_current();
            }
        }
    }

    fn ui_host_panel(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(ui, "HOST://LINK", "HOST", "Share an address · friends join");
        ui.label(
            RichText::new("Pick how friends reach this table.")
                .color(MUTED)
                .small(),
        );
        wrap_text(
            ui,
            if self.node_live {
                "NODE is ACTIVE. Hosting stays live if you close this window. Online again turns the node off."
            } else {
                "NODE is OFFLINE. Press Online in the command bar to start it."
            },
            MUTED,
            11.0,
        );
        ui.add_space(8.0);
        if self.net.role == Role::Guest {
            wrap_text(
                ui,
                "You are already at a table. Leave it before you host.",
                CREAM,
                13.0,
            );
            if theme::neon_btn_color(ui, "Leave table", KILL, true).clicked() {
                self.leave_table();
            }
        } else if self.net.role == Role::Host {
            ui.label(
                RichText::new(if self.net.internet {
                    "INTERNET TABLE"
                } else {
                    "LOCAL TABLE"
                })
                .family(theme::mono())
                .size(11.0)
                .color(CYAN),
            );
            let lan = self.net.lan_link();
            ui.label(
                RichText::new("Same house")
                    .family(theme::mono())
                    .size(10.0)
                    .color(DIM),
            );
            ui.label(
                RichText::new(&lan)
                    .family(theme::mono())
                    .size(12.0)
                    .color(CYAN),
            );
            if theme::neon_btn(ui, "Copy LAN address").clicked() {
                ui.ctx().copy_text(lan.clone());
                self.chat.push(format!("Copied {lan}"));
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(if self.net.internet {
                    "Invite (any network · encrypted)"
                } else {
                    "Invite (encrypted)"
                })
                .family(theme::mono())
                .size(10.0)
                .color(DIM),
            );
            let invite = self.net.paste_link();
            wrap_text(ui, &invite, CYAN, 12.0);
            if theme::neon_btn(ui, "Copy invite link").clicked() {
                ui.ctx().copy_text(invite.clone());
                self.chat.push(format!(
                    "Copied. Friends paste this into Join:\n{invite}"
                ));
            }
            ui.add_space(8.0);
            if theme::neon_btn_color(ui, "Leave table", KILL, true).clicked() {
                self.leave_table();
            }
        } else if self.node_live {
            ui.label(
                RichText::new("LOCAL NETWORK")
                    .family(theme::mono())
                    .size(11.0)
                    .color(theme::ACID),
            );
            ui.label(
                RichText::new(
                    "Same house or the same Wi-Fi. Friends paste the blightnet:// invite into Join.",
                )
                .color(CREAM)
                .small(),
            );
            if theme::neon_btn(ui, "Host on local network").clicked() {
                self.start_host(false);
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new("INTERNET")
                    .family(theme::mono())
                    .size(11.0)
                    .color(theme::ACID),
            );
            ui.label(
                RichText::new("Other networks. Your node punches UDP to their node and maps TCP/UDP if the router allows it. Copy the invite. No Cloudflare.")
                    .color(CREAM)
                    .small(),
            );
            if theme::neon_btn(ui, "Host on the internet").clicked() {
                self.start_host(true);
            }
        }
    }

    fn ui_join_panel(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(ui, "JOIN://LINK", "JOIN", "Paste a host address · sit at the table");
        ui.label(
            RichText::new(
                "Paste the blightnet:// invite the host copied. This program talks to the table itself.",
            )
            .color(MUTED)
            .small(),
        );
        wrap_text(
            ui,
            if self.node_live {
                "NODE is ACTIVE. Paste the invite, then Connect."
            } else {
                "NODE is OFFLINE. Press Online in the command bar to start it."
            },
            MUTED,
            11.0,
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.join_in)
                .hint_text("blightnet://invite@192.168.0.12:8766,1.2.3.4:8766")
                .desired_width(ui.available_width()),
        );
        ui.horizontal(|ui| {
            if theme::neon_btn(ui, "Connect").clicked() {
                let addr = self.join_in.clone();
                self.do_join(&addr);
            }
        });
        if !self.err.is_empty() {
            wrap_text(ui, &self.err.clone(), KILL, 13.0);
        }
    }

    fn ui_chat_panel(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(ui, "CHAT://RAIL", "CHAT", "Pick Table, a contact, or a crew");
        wrap_text(
            ui,
            "Pick Table, a contact, or a crew. Send text or any file. Incoming media streams automatically — Download keeps a copy. Chat is permanent until you wipe it.",
            MUTED,
            11.0,
        );
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn_color(ui, "Table", CYAN, self.chat_target == ChatTarget::Table)
                .clicked()
            {
                self.chat_target = ChatTarget::Table;
                self.whisper_to = None;
            }
            let contacts = self.contacts.clone();
            for c in &contacts {
                let on = matches!(&self.chat_target, ChatTarget::Dm(id) if id == &c.id);
                let lab = format!(
                    "{} {}",
                    c.name,
                    if self.contact_online(&c.id) { "●" } else { "○" }
                );
                if theme::neon_btn_color(ui, &lab, if self.contact_online(&c.id) { CYAN } else { CYAN }, on)
                    .clicked()
                {
                    self.chat_target = ChatTarget::Dm(c.id.clone());
                    self.whisper_to = Some(c.id.clone());
                }
            }
            let crews = self.crews.clone();
            for crew in &crews {
                let on = matches!(&self.chat_target, ChatTarget::Crew(id) if id == &crew.id);
                if theme::neon_btn_color(ui, &format!("crew:{}", crew.name), CYAN, on).clicked() {
                    self.chat_target = ChatTarget::Crew(crew.id.clone());
                }
            }
        });
        ui.add_space(6.0);
        let chat_n = self.chat.len();
        let start = chat_n.saturating_sub(80);
        if chat_n == 0 {
            wrap_text(ui, "No lines yet. Write below, or send a file.", DIM, 12.0);
        }
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(240.0)
            .show(ui, |ui| {
                if start > 0 {
                    wrap_text(
                        ui,
                        &format!("{start} earlier lines stay on this deck."),
                        DIM,
                        11.0,
                    );
                }
                for i in start..chat_n {
                    let line = self.chat[i].clone();
                    if self.ui_chat_media(ui, &line) {
                        continue;
                    }
                    wrap_text(
                        ui,
                        &line,
                        if line.contains("[whisper]") { CYAN } else { CREAM },
                        12.0,
                    );
                }
            });
        ui.add_space(6.0);
        let hint = match &self.chat_target {
            ChatTarget::Dm(_) => "Message this contact…",
            ChatTarget::Crew(_) => "Message this crew…",
            ChatTarget::Table => "Say something…  /w Name for a whisper",
        };
        let resp = ui.add(
            egui::TextEdit::multiline(&mut self.chat_in)
                .hint_text(hint)
                .desired_width(ui.available_width())
                .desired_rows(2),
        );
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
            self.send_chat_line();
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Send").clicked() {
                self.send_chat_line();
            }
            if theme::neon_btn(ui, "Image").clicked() {
                self.send_chat_image();
            }
            if theme::neon_btn(ui, "Audio").clicked() {
                self.send_chat_audio_file();
            }
            if theme::neon_btn(ui, "Video").clicked() {
                self.send_chat_video_file();
            }
            if theme::neon_btn(ui, "File").clicked() {
                self.send_chat_any_file();
            }
            if theme::neon_btn_color(ui, "Wipe chat", KILL, false).clicked() {
                self.wipe_chat();
            }
            if self.rec_on {
                let secs = self.rec.len() as f32 / 16000.0;
                if theme::neon_btn_color(ui, &format!("Stop {secs:.0}s"), KILL, true).clicked() {
                    self.rec_on = false;
                    self.send_voice_note();
                }
            } else if theme::neon_btn(ui, "Record").clicked() {
                self.ensure_mic();
                self.rec.clear();
                self.rec_on = true;
            }
            if !self.net.presence && self.net.role == Role::Idle {
                wrap_text(ui, "Go Online so contacts can talk without Host.", MUTED, 11.0);
            }
        });
    }

    fn ui_contacts_panel(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(ui, "CONTACTS://RAIL", "CONTACTS", "Save people · open a DM or call");
        wrap_text(
            ui,
            "Add someone once. After that, Go Online — no new invite for text, calls, or pictures. ● online  ○ offline.",
            MUTED,
            11.0,
        );
        ui.add_space(4.0);
        ui.label(RichText::new("NEARBY / TABLE").family(theme::mono()).size(10.0).color(CYAN));
        let peers: Vec<_> = self
            .net
            .peers
            .iter()
            .filter(|p| p.id != self.net.self_id)
            .cloned()
            .collect();
        if peers.is_empty() {
            wrap_text(ui, "No one else in reach. Go Online or Host a table.", MUTED, 11.0);
        }
        for p in &peers {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&p.name).color(CREAM));
                ui.label(RichText::new("●").color(CYAN));
                if let Some((text, col)) = self.link_readout(&p.id) {
                    ui.label(RichText::new(text).family(theme::mono()).size(11.0).color(col));
                } else {
                    ui.label(RichText::new("checking").family(theme::mono()).size(11.0).color(DIM));
                }
                if theme::neon_btn(ui, "Add").clicked() {
                    self.add_contact(&p.id, &p.name);
                }
                if theme::neon_btn(ui, "Message").clicked() {
                    self.chat_target = ChatTarget::Dm(p.id.clone());
                    self.whisper_to = Some(p.id.clone());
                    self.shell = ShellPanel::Chat;
                }
            });
        }
        ui.add_space(8.0);
        ui.label(RichText::new("SAVED").family(theme::mono()).size(10.0).color(CYAN));
        if self.contacts.is_empty() {
            wrap_text(ui, "No contacts yet. Add someone at a table or nearby.", MUTED, 11.0);
        }
        let saved = self.contacts.clone();
        for (i, c) in saved.iter().enumerate() {
            let on = self.contact_online(&c.id);
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&c.name).color(CYAN));
                    let link = if on {
                        self.link_readout(&c.id)
                            .map(|(t, _)| t)
                            .unwrap_or_else(|| "online · checking".into())
                    } else {
                        "offline".into()
                    };
                    let col = if !on {
                        DIM
                    } else {
                        self.link_readout(&c.id).map(|(_, c)| c).unwrap_or(DIM)
                    };
                    wrap_text(ui, &link, col, 11.0);
                });
                if on && theme::neon_btn(ui, "Message").clicked() {
                    self.chat_target = ChatTarget::Dm(c.id.clone());
                    self.whisper_to = Some(c.id.clone());
                    self.shell = ShellPanel::Chat;
                }
                if on && theme::neon_btn(ui, "Call").clicked() {
                    self.net.send_voice("invite", Some(c.id.clone()));
                    self.call_id = Some(c.id.clone());
                    self.shell = ShellPanel::Voice;
                }
                if on && theme::neon_btn(ui, "Video").clicked() {
                    self.start_video_to(c.id.clone(), None);
                }
                if !c.addr.is_empty() && theme::neon_btn(ui, "Join table").clicked() {
                    let addr = c.addr.clone();
                    self.do_join(&addr);
                }
                if theme::neon_btn_color(ui, "Remove", KILL, false).clicked() {
                    self.contacts.remove(i);
                    net::save_contacts(&self.root, &self.contacts);
                }
            });
        }
        ui.add_space(10.0);
        ui.label(RichText::new("CREWS").family(theme::mono()).size(10.0).color(CYAN));
        wrap_text(ui, "A crew is a group chat. Add saved contacts, then message them together.", MUTED, 11.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.crew_name)
                    .hint_text("Crew name")
                    .desired_width(140.0),
            );
            if theme::neon_btn(ui, "Create crew").clicked() {
                let name = self.crew_name.trim().to_string();
                if !name.is_empty() {
                    self.crews.push(Crew {
                        id: format!("crew-{}", rand::random::<u32>()),
                        name,
                        members: vec![],
                    });
                    self.crew_name.clear();
                    save_crews(&self.root, &self.crews);
                }
            }
        });
        let crews = self.crews.clone();
        let contact_names: Vec<(String, String)> = self
            .contacts
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        for (ci, crew) in crews.iter().enumerate() {
            wrap_text(ui, &format!("▸ {}", crew.name), CYAN, 14.0);
            ui.horizontal_wrapped(|ui| {
                if theme::neon_btn(ui, "Open chat").clicked() {
                    self.chat_target = ChatTarget::Crew(crew.id.clone());
                    self.shell = ShellPanel::Chat;
                }
                if theme::neon_btn(ui, "Video").clicked() {
                    self.start_video_to(crew.id.clone(), Some(crew.id.clone()));
                }
                if theme::neon_btn_color(ui, "Disband", KILL, false).clicked() {
                    self.crews.remove(ci);
                    save_crews(&self.root, &self.crews);
                }
            });
            for (id, name) in &contact_names {
                let in_crew = crew.members.iter().any(|m| m == id);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).color(if in_crew { CYAN } else { MUTED }).small());
                    let lab = if in_crew { "Remove" } else { "Add" };
                    if theme::neon_btn(ui, lab).clicked() {
                        if let Some(c) = self.crews.get_mut(ci) {
                            if in_crew {
                                c.members.retain(|m| m != id);
                            } else {
                                c.members.push(id.clone());
                            }
                        }
                        save_crews(&self.root, &self.crews);
                    }
                });
            }
        }
    }

    fn ui_voice_panel(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(ui, "VOICE://RAIL", "VOICE", "Mic · calls · press again on VOICE to close");
        wrap_text(
            ui,
            "Table voice talks to everyone at a mix table. Private calls are under Contacts. Go Online and you can call a saved contact with no new invite.",
            MUTED,
            11.0,
        );
        if !self.net.presence && self.net.role == Role::Idle {
            wrap_text(ui, "Go Online or Host / Join first.", CYAN, 12.0);
        }
        ui.horizontal(|ui| {
            let lab = if self.voice_on { "Table voice ON" } else { "Table voice" };
            if theme::neon_btn_color(ui, lab, CYAN, self.voice_on).clicked() {
                self.voice_on = !self.voice_on;
                if self.voice_on {
                    self.ensure_mic();
                    self.net.send_voice("table-on", None);
                } else {
                    self.net.send_voice("table-off", None);
                }
            }
            if theme::neon_btn_color(ui, if self.voice_mute { "Muted" } else { "Mute" }, CYAN, self.voice_mute)
                .clicked()
            {
                self.voice_mute = !self.voice_mute;
            }
            if theme::neon_btn_color(ui, if self.voice_deaf { "Deafened" } else { "Deaf" }, CYAN, self.voice_deaf)
                .clicked()
            {
                self.voice_deaf = !self.voice_deaf;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("MIC SEND").family(theme::mono()).size(10.0).color(DIM));
            ui.add(egui::Slider::new(&mut self.mic_gain, 0.0..=2.0).suffix("x"));
        });
        wrap_text(
            ui,
            &format!(
                "Mic: {}  ·  Speaker: {}",
                if self.mic_name.is_empty() {
                    "default"
                } else {
                    &self.mic_name
                },
                if self.speaker_name.is_empty() {
                    "default"
                } else {
                    &self.speaker_name
                }
            ),
            DIM,
            11.0,
        );
        if let Some((id, name)) = self.incoming.clone() {
            ui.label(RichText::new(format!("{name} is calling.")).color(CYAN));
            ui.horizontal(|ui| {
                if theme::neon_btn(ui, "Accept").clicked() {
                    self.net.send_voice("accept", Some(id.clone()));
                    self.remember_party(&id);
                    self.incoming = None;
                    self.voice_on = true;
                    self.ensure_mic();
                    self.push_roster();
                }
                if theme::neon_btn_color(ui, "Decline", KILL, true).clicked() {
                    self.net.send_voice("decline", Some(id.clone()));
                    self.incoming = None;
                }
            });
        }
        if self.call_id.is_some() || self.call_crew.is_some() || !self.call_with.is_empty() {
            self.ui_call_people(ui, false);
            if theme::neon_btn_color(ui, "Hang up", KILL, true).clicked() {
                self.hang_up();
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new("PEOPLE").family(theme::mono()).size(10.0).color(CYAN));
        let peers: Vec<_> = self
            .net
            .peers
            .iter()
            .filter(|p| p.id != self.net.self_id)
            .cloned()
            .collect();
        for p in &peers {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&p.name).color(CREAM));
                if theme::neon_btn(ui, "Call").clicked() {
                    self.call_with.clear();
                    self.call_drop.clear();
                    self.call_crew = None;
                    self.net.send_voice("invite", Some(p.id.clone()));
                    self.call_id = Some(p.id.clone());
                    self.chat.push(format!("Calling {}…", p.name));
                }
            });
        }
        if peers.is_empty() {
            ui.label(RichText::new("No one else at the table.").color(MUTED).small());
        }
        ui.add_space(6.0);
        if theme::neon_btn(ui, "Audio page").clicked() {
            self.page = Page::Audio;
        }
    }

    fn ui_video_panel(&mut self, ui: &mut egui::Ui) {
        self.ensure_cameras();
        theme::page_chrome(ui, "VIDEO://RAIL", "VIDEO", "Camera · contacts and crews");
        wrap_text(
            ui,
            "Call a contact or a crew. Camera is auto-detected. Share your screen on the same call (one GM share). On Wayland, approve the system portal dialog. LAN-good; WAN screenshare is best-effort. Video failure leaves voice up. Go Online first.",
            MUTED,
            11.0,
        );
        if !self.net.presence && self.net.role == Role::Idle {
            wrap_text(ui, "Go Online or Host / Join first.", CYAN, 12.0);
        }
        self.ui_ffmpeg_missing(ui, true);
        ui.add_space(6.0);
        ui.label(
            RichText::new("CAMERA")
                .family(theme::mono())
                .size(10.0)
                .color(CYAN),
        );
        let cams = self.cameras.clone();
        let mut cam = self.cam_name.clone();
        let label = cams
            .iter()
            .find(|c| c.path == cam)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| {
                if cam.is_empty() {
                    "No camera".into()
                } else {
                    cam.clone()
                }
            });
        egui::ComboBox::from_id_salt("vid-cam")
            .selected_text(label)
            .width(ui.available_width().max(80.0))
            .show_ui(ui, |ui| {
                for c in &cams {
                    ui.selectable_value(&mut cam, c.path.clone(), &c.name);
                }
            });
        if cam != self.cam_name {
            self.set_camera(cam);
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Rescan cameras").clicked() {
                let _ = self.refresh_ffmpeg();
                self.cameras = crate::video::list_cameras();
                if self.cam_name.is_empty() {
                    if let Some(c) = crate::video::default_camera() {
                        self.set_camera(c.path);
                    }
                }
            }
        });
        if let Some((id, name, _)) = self.incoming_video.clone() {
            ui.add_space(6.0);
            ui.label(RichText::new(format!("{name} is video calling.")).color(CYAN));
            ui.horizontal_wrapped(|ui| {
                if theme::neon_btn(ui, "Accept").clicked() {
                    self.accept_video();
                }
                if theme::neon_btn_color(ui, "Decline", KILL, true).clicked() {
                    let crew = self.call_crew.clone();
                    self.net.send_video("decline", Some(id.clone()), crew.clone());
                    self.net.send_voice_ex("decline", Some(id), crew);
                    self.incoming_video = None;
                    self.incoming = None;
                }
            });
        }
        if self.video_on {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if theme::neon_btn_color(ui, "Camera", CYAN, self.cam_on).clicked() {
                    if self.cam_on {
                        self.cam_on = false;
                        self.cam_cap = None;
                        self.local_cam.clear();
                    } else {
                        self.start_cam();
                    }
                }
                if theme::neon_btn_color(ui, "Share screen", CYAN, self.screen_on).clicked() {
                    if self.screen_on {
                        self.screen_on = false;
                        self.screen_cap = None;
                        self.local_screen.clear();
                    } else {
                        self.start_screen_share();
                    }
                }
                if theme::neon_btn_color(ui, if self.voice_mute { "Muted" } else { "Mute" }, CYAN, self.voice_mute)
                    .clicked()
                {
                    self.voice_mute = !self.voice_mute;
                }
                if theme::neon_btn_color(ui, if self.voice_deaf { "Deafened" } else { "Deaf" }, CYAN, self.voice_deaf)
                    .clicked()
                {
                    self.voice_deaf = !self.voice_deaf;
                }
                if theme::neon_btn_color(ui, "Hang up", KILL, true).clicked() {
                    self.hang_up();
                }
            });
            self.ui_call_people(ui, true);
            ui.add_space(6.0);
            ui.label(
                RichText::new("YOU")
                    .family(theme::mono())
                    .size(10.0)
                    .color(DIM),
            );
            let feed = Vec2::new(ui.available_width().min(220.0).max(72.0), 140.0);
            ui.horizontal_wrapped(|ui| {
                images::show_bytes(ui, &mut self.tex, "local-cam", &self.local_cam, feed);
                if !self.local_screen.is_empty() {
                    let screen = Vec2::new(ui.available_width().min(260.0).max(72.0), 140.0);
                    images::show_bytes(
                        ui,
                        &mut self.tex,
                        "local-screen",
                        &self.local_screen,
                        screen,
                    );
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("THEM")
                    .family(theme::mono())
                    .size(10.0)
                    .color(CYAN),
            );
            let ids: Vec<String> = self.remote_vid.keys().cloned().collect();
            if ids.is_empty() {
                wrap_text(ui, "Waiting for the other side…", MUTED, 12.0);
            }
            for id in &ids {
                let name = self
                    .remote_vid
                    .get(id)
                    .map(|f| f.name.clone())
                    .unwrap_or_default();
                wrap_text(ui, &name, CYAN, 12.0);
                let feed_size = Vec2::new(ui.available_width().min(220.0).max(72.0), 140.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(feed) = self.remote_vid.get(id) {
                        images::show_bytes(
                            ui,
                            &mut self.tex,
                            &format!("cam-{id}"),
                            &feed.cam,
                            feed_size,
                        );
                    }
                    if self
                        .remote_vid
                        .get(id)
                        .map(|f| !f.screen.is_empty())
                        .unwrap_or(false)
                    {
                        if let Some(feed) = self.remote_vid.get(id) {
                            let screen = Vec2::new(ui.available_width().min(260.0).max(72.0), 140.0);
                            images::show_bytes(
                                ui,
                                &mut self.tex,
                                &format!("scr-{id}"),
                                &feed.screen,
                                screen,
                            );
                        }
                    }
                });
            }
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new("CONTACTS")
                .family(theme::mono())
                .size(10.0)
                .color(CYAN),
        );
        let saved = self.contacts.clone();
        if saved.is_empty() {
            wrap_text(ui, "Save someone under Contacts first.", MUTED, 11.0);
        }
        for c in &saved {
            let on = self.contact_online(&c.id);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&c.name).color(if on { CREAM } else { DIM }));
                ui.label(
                    RichText::new(if on { "●" } else { "○" }).color(if on { CYAN } else { DIM }),
                );
                if on && theme::neon_btn(ui, "Video").clicked() {
                    self.start_video_to(c.id.clone(), None);
                }
            });
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new("CREWS")
                .family(theme::mono())
                .size(10.0)
                .color(CYAN),
        );
        let crews = self.crews.clone();
        for crew in &crews {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&crew.name).color(CYAN));
                if theme::neon_btn(ui, "Crew video").clicked() {
                    self.start_video_to(crew.id.clone(), Some(crew.id.clone()));
                }
            });
        }
        if crews.is_empty() {
            wrap_text(ui, "Make a crew under Contacts, then call the whole crew.", MUTED, 11.0);
        }
    }

    fn ui_table_root(&mut self, ui: &mut egui::Ui) {
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(ext.as_str(), "ogg" | "mp3" | "wav" | "flac" | "opus") {
                self.add_sound_file(path);
            } else if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                self.map.image = Some(path);
                self.map.tokens.clear();
                if !self.panel_on(Overlay::Maps) {
                    self.open.push(Overlay::Maps);
                }
                self.push_map_image();
                if self.table_live() {
                    self.net.send_map_tokens(vec![]);
                }
            }
        }
        let pal = self.pal();
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                theme::page_chrome(
                    ui,
                    if self.blight {
                        "TABLE://BLIGHT"
                    } else {
                        "TABLE://HEARTH"
                    },
                    "TABLE",
                    "Rail opens Scenes · Mix · Sheets · Map",
                );
                let full = ui.available_rect_before_wrap();
                let rail_w = 168.0_f32.min(full.width() * 0.34);
                let (rail, main) = full.split_left_right_at_x(full.left() + rail_w);
                ui.allocate_rect(full, egui::Sense::hover());
                ui.painter().vline(
                    rail.right(),
                    rail.y_range(),
                    egui::Stroke::new(1.0, theme::HOT),
                );
                let rail_in = rail.shrink2(Vec2::new(8.0, 6.0));
                let mut rail_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rail_in)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                rail_ui.set_clip_rect(rail_in);
                egui::ScrollArea::vertical()
                    .id_salt("table-rail")
                    .max_height(rail_in.height().max(40.0))
                    .auto_shrink([false, false])
                    .show(&mut rail_ui, |ui| {
                        ui.set_min_width(rail_in.width());
                        self.ui_table_tools(ui);
                    });
                let mut ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(main)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                ui.set_clip_rect(main);
                self.ui_table_top(&mut ui, pal);
                if self.watch_open || self.place_open {
                    self.ui_table_expand(&mut ui, pal);
                }
                let rails: Vec<Overlay> = [Overlay::Chars, Overlay::Log, Overlay::Notes]
                    .into_iter()
                    .filter(|o| self.panel_on(*o))
                    .collect();
                let rest = ui.available_rect_before_wrap();
                ui.allocate_rect(rest, egui::Sense::hover());
                let nrail = rails.len();
                let right_w = if nrail > 0 {
                    let leave = 180.0_f32.min(rest.width() * 0.40);
                    (rest.width() * 0.32)
                        .clamp(250.0, 380.0)
                        .min(rest.width() - leave)
                        .max(0.0)
                } else {
                    0.0
                };
                let (main, right) = if nrail > 0 && right_w > 160.0 {
                    rest.split_left_right_at_x(rest.right() - right_w)
                } else {
                    (rest, Rect::from_min_size(rest.right_top(), Vec2::ZERO))
                };
                let mut main_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(main)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let tiles = self.open.iter().any(|o| {
                    !matches!(o, Overlay::Chars | Overlay::Log | Overlay::Notes)
                });
                if tiles {
                    self.ui_tiled_panels(&mut main_ui, pal);
                } else {
                    let rest = main_ui.available_rect_before_wrap();
                    main_ui.allocate_rect(rest, egui::Sense::hover());
                    main_ui.painter().rect_filled(rest, 0.0, theme::BG);
                }
                self.ui_dice_fx(&mut main_ui);
                if nrail > 0 && right.width() > 160.0 {
                    ui.painter().vline(
                        right.left(),
                        rest.y_range(),
                        egui::Stroke::new(1.0, theme::HOT),
                    );
                    let slice_h = right.height() / nrail as f32;
                    for (i, kind) in rails.into_iter().enumerate() {
                        let y0 = right.top() + i as f32 * slice_h;
                        let pane = Rect::from_min_max(
                            egui::pos2(right.left(), y0),
                            egui::pos2(right.right(), y0 + slice_h),
                        );
                        if i > 0 {
                            ui.painter().hline(
                                pane.x_range(),
                                pane.top(),
                                egui::Stroke::new(1.0, theme::HOT),
                            );
                        }
                        let inner = pane.shrink2(Vec2::new(10.0, 8.0));
                        let mut pane_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(inner)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        pane_ui.set_clip_rect(inner);
                        pane_ui.set_max_width(inner.width());
                        pane_ui.set_max_height(inner.height());
                        match kind {
                            Overlay::Chars => self.ui_chars_panel(&mut pane_ui, pal),
                            Overlay::Log => self.ui_overlay_log(&mut pane_ui),
                            Overlay::Notes => self.ui_rail_notes(&mut pane_ui),
                            _ => {}
                        }
                    }
                }
            },
        );
    }

    fn ui_tiled_panels(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let panels: Vec<Overlay> = self
            .open
            .iter()
            .copied()
            .filter(|o| !matches!(o, Overlay::Chars | Overlay::Log | Overlay::Notes))
            .collect();
        let n = panels.len();
        if n == 0 {
            return;
        }
        let rect = ui.available_rect_before_wrap();
        let h = rect.height().max(80.0);
        let board = Rect::from_min_size(rect.min, Vec2::new(rect.width(), h));
        ui.allocate_rect(board, egui::Sense::hover());
        let inner = board.shrink2(Vec2::new(4.0, 4.0));
        self.tile_ratio = self.tile_ratio.clamp(0.22, 0.78);
        let cells: Vec<Rect> = match n {
            1 => vec![inner],
            2 => {
                let x = inner.left() + inner.width() * self.tile_ratio;
                let (a, b) = inner.split_left_right_at_x(x);
                let hit = Rect::from_center_size(egui::pos2(x, inner.center().y), Vec2::new(8.0, inner.height()));
                let resp = ui.interact(hit, egui::Id::new("tile-x"), egui::Sense::click_and_drag());
                if resp.dragged() {
                    self.tile_ratio = ((x + resp.drag_delta().x - inner.left()) / inner.width().max(1.0)).clamp(0.22, 0.78);
                }
                ui.painter().vline(
                    x,
                    inner.y_range(),
                    egui::Stroke::new(2.0, if resp.hovered() || resp.dragged() { theme::chrome().acid } else { theme::HOT }),
                );
                vec![a.shrink(4.0), b.shrink(4.0)]
            }
            _ => {
                let y = inner.top() + inner.height() * self.tile_ratio;
                let (top, bot) = inner.split_top_bottom_at_y(y);
                let hit = Rect::from_center_size(egui::pos2(inner.center().x, y), Vec2::new(inner.width(), 8.0));
                let resp = ui.interact(hit, egui::Id::new("tile-y"), egui::Sense::click_and_drag());
                if resp.dragged() {
                    self.tile_ratio = ((y + resp.drag_delta().y - inner.top()) / inner.height().max(1.0)).clamp(0.22, 0.78);
                }
                ui.painter().hline(
                    inner.x_range(),
                    y,
                    egui::Stroke::new(2.0, if resp.hovered() || resp.dragged() { theme::chrome().acid } else { theme::HOT }),
                );
                let (a, b) = top.split_left_right_at_x(top.center().x);
                let mut v = vec![a.shrink(3.0), b.shrink(3.0)];
                if n == 3 {
                    v.push(bot.shrink(3.0));
                } else {
                    let (c, d) = bot.split_left_right_at_x(bot.center().x);
                    v.push(c.shrink(3.0));
                    v.push(d.shrink(3.0));
                }
                v
            }
        };
        for (i, kind) in panels.into_iter().enumerate() {
            let Some(cell) = cells.get(i).copied() else {
                break;
            };
            theme::plate(ui, cell);
            let cell_in = cell.shrink(8.0);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_in)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(cell_in);
            match kind {
                Overlay::Chars | Overlay::Log | Overlay::Notes => {}
                Overlay::Maps => self.ui_overlay_maps(&mut child, pal),
                Overlay::Catalog => self.ui_overlay_catalog(&mut child, pal),
                Overlay::Blackjack => self.ui_overlay_bj(&mut child, pal),
                Overlay::Chess => self.ui_overlay_chess(&mut child),
                Overlay::Armory => self.ui_overlay_kit(&mut child, pal, false),
                Overlay::Vendors => self.ui_overlay_kit(&mut child, pal, true),
                Overlay::Jackin => self.ui_overlay_jackin(&mut child, pal),
                Overlay::Scenes => self.ui_scenes_panel(&mut child, pal),
                Overlay::Mix => self.ui_mix_panel(&mut child, pal),
                Overlay::Board => self.ui_table_board(&mut child),
                Overlay::Place => self.ui_place_tile(&mut child),
                Overlay::Calendar => self.ui_calendar_tile(&mut child),
                Overlay::Calc => self.ui_calc_tile(&mut child),
                Overlay::None => {}
            }
        }
    }

    fn ui_table_top(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let shown = egui::Frame::NONE
            .fill(Color32::from_rgb(17, 17, 8))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                egui::ScrollArea::horizontal()
                    .id_salt("tbl-world")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let icon = self.root.join("assets/icon.png");
                    images::show_fit(ui, &mut self.tex, &icon, Vec2::splat(28.0));
                    ui.vertical(|ui| {
                        let world = if self.blight { "BLIGHT" } else { "HEARTHSONG" };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(world)
                                        .family(theme::display())
                                        .size(15.0)
                                        .color(theme::chrome().cyan),
                                )
                                .frame(false),
                            )
                            .on_hover_text(
                                "Switch Hearthsong and Blight. The mix goes quiet and Place resets.",
                            )
                            .clicked()
                        {
                            self.set_world(!self.blight);
                            self.broadcast_mix();
                        }
                        ui.label(
                            RichText::new(if self.blight {
                                "NETDIR://TABLE · BLIGHT MIX"
                            } else {
                                "NETDIR://TABLE · HEARTH MIX"
                            })
                            .family(theme::mono())
                            .size(10.0)
                            .color(theme::chrome().cyan),
                        );
                    });
                    let ch_top = theme::chrome();
                    if theme::neon_btn_color(ui, &self.place_name(), ch_top.cyan, true).clicked() {
                        self.place_open = !self.place_open;
                        self.watch_open = false;
                    }
                    if theme::neon_btn_color(ui, "Outside", ch_top.cyan, !self.inside).clicked() {
                        self.set_inside(false);
                    }
                    if theme::neon_btn_color(ui, "Inside", ch_top.cyan, self.inside).clicked() {
                        self.set_inside(true);
                    }
                    if theme::analog_watch(ui, self.clock, pal).clicked() {
                        self.watch_open = !self.watch_open;
                        self.place_open = false;
                    }
                    ui.vertical(|ui| {
                        let ch_clk = theme::chrome();
                        ui.label(
                            RichText::new(format_clock(self.clock))
                                .family(theme::mono())
                                .size(13.0)
                                .color(ch_clk.cyan),
                        );
                        ui.label(
                            RichText::new(period_label(self.time))
                                .family(theme::mono())
                                .size(10.0)
                                .color(ch_clk.cyan),
                        );
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!(
                                        "{:04}-{:02}-{:02}",
                                        self.cal_y, self.cal_m, self.cal_d
                                    ))
                                    .family(theme::mono())
                                    .size(10.0)
                                    .color(theme::chrome().acid),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Open the calendar.")
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Calendar);
                        }
                    });
                    let ch_run = theme::chrome();
                    if self.is_gm
                        && theme::neon_btn_color(ui, "Run clock", ch_run.acid, self.clock_run)
                            .clicked()
                    {
                        self.clock_run = !self.clock_run;
                    }
                    for (id, lab) in [
                        ("morning", "Morning"),
                        ("day", "Day"),
                        ("evening", "Evening"),
                        ("night", "Night"),
                    ] {
                        if theme::neon_btn_color(ui, lab, ch_run.cyan, self.time == id).clicked() {
                            self.set_period(id);
                        }
                    }
                    ui.label(
                        RichText::new("LOCAL")
                            .family(theme::mono())
                            .size(10.0)
                            .color(DIM),
                    )
                    .on_hover_text("Loudness on this deck only. Not sent to other seats.");
                    let mut m = self.master;
                    let mut master_changed = false;
                    ui.allocate_ui(Vec2::new(140.0, 22.0), |ui| {
                        if ui
                            .add(
                                egui::Slider::new(&mut m, 0.0..=1.0)
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .changed()
                        {
                            master_changed = true;
                        }
                    });
                    if master_changed {
                        self.master = m;
                        self.mixer.set_master(m);
                    }
                    let faded = self.held_mix.is_some() && self.mixer.playing().next().is_none();
                    if theme::neon_btn_color(
                        ui,
                        if faded { "Fade in" } else { "Fade out" },
                        CYAN,
                        faded,
                    )
                    .clicked()
                    {
                        self.fade_mix();
                    }
                    if theme::neon_btn_color(ui, "Silence", KILL, false).clicked() {
                        self.mixer.silence();
                        self.mix_msg = "Silent.".into();
                        self.broadcast_mix();
                    }
                    ui.label(RichText::new("SEAT").family(theme::mono()).size(10.0).color(DIM));
                    if theme::neon_btn_color(ui, "Player", CYAN, !self.is_gm).clicked() {
                        self.is_gm = false;
                    }
                    if theme::neon_btn_color(ui, "GM", CYAN, self.is_gm).clicked() {
                        self.is_gm = true;
                    }
                });
                });
            });
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(1.0, theme::ACID),
        );
    }

    fn ui_table_tools(&mut self, ui: &mut egui::Ui) {
                    rail_block(ui, "NET", |ui| {
                        match self.net.role {
                            Role::Idle | Role::Presence => {
                                if theme::rail_row(ui, "Host", self.shell == ShellPanel::Host, false)
                                .clicked()
                                {
                                    self.toggle_shell(ShellPanel::Host);
                                }
                                if theme::rail_row(ui, "Join", self.shell == ShellPanel::Join, false)
                                .clicked()
                                {
                                    self.toggle_shell(ShellPanel::Join);
                                }
                            }
                            Role::Host => {
                                if theme::rail_row(ui, "Copy address", false, false).clicked() {
                                    let link = self.net.paste_link();
                                    ui.ctx().copy_text(link.clone());
                                    self.chat.push(format!("Copied {link}"));
                                }
                                if theme::rail_row(ui, "Leave", false, true).clicked() {
                                    self.leave_table();
                                }
                            }
                            Role::Guest => {
                                if theme::rail_row(ui, "Leave", false, true).clicked() {
                                    self.leave_table();
                                }
                            }
                        }
                    });
                    rail_block(ui, "STAGE", |ui| {
                        if theme::rail_row(ui, "Scenes", self.panel_on(Overlay::Scenes), false).clicked()
                        {
                            self.toggle_overlay(Overlay::Scenes);
                        }
                        if theme::rail_row(ui, "Mix", self.panel_on(Overlay::Mix), false).clicked() {
                            self.toggle_overlay(Overlay::Mix);
                        }
                        if theme::rail_row(ui, "Place", self.panel_on(Overlay::Place), false).clicked()
                        {
                            self.toggle_overlay(Overlay::Place);
                        }
                        if theme::rail_row(ui, "Calendar", self.panel_on(Overlay::Calendar), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Calendar);
                        }
                        if theme::rail_row(ui, "Calc", self.panel_on(Overlay::Calc), false).clicked()
                        {
                            self.toggle_overlay(Overlay::Calc);
                        }
                    });
                    let overlay_cat = self.overlay_cat;
                    let catalog_open = self.panel_on(Overlay::Catalog);
                    let cat_on = |name: &str| catalog_open && overlay_cat == name;
                    rail_block(ui, "SHEET", |ui| {
                        if theme::rail_row(ui, "Characters", self.panel_on(Overlay::Chars) && self.viewing.is_none(), false)
                        .clicked()
                        {
                            self.open_seat(None);
                        }
                    });
                    rail_block(ui, "SEAT", |ui| {
                        let mine = if self.handle.trim().is_empty() {
                            "YOU".to_string()
                        } else {
                            self.handle.clone()
                        };
                        let on_me = self.panel_on(Overlay::Chars)
                            && self
                                .viewing
                                .as_ref()
                                .map(|id| id == &self.net.self_id)
                                .unwrap_or(true);
                        if theme::rail_row(ui, &mine, on_me, false)
                        .clicked()
                        {
                            self.open_seat(None);
                        }
                        let peers: Vec<(String, String)> = self
                            .net
                            .peers
                            .iter()
                            .filter(|p| p.id != self.net.self_id)
                            .map(|p| {
                                (
                                    p.id.clone(),
                                    if p.name.trim().is_empty() {
                                        p.id.clone()
                                    } else {
                                        p.name.clone()
                                    },
                                )
                            })
                            .collect();
                        if peers.is_empty() {
                            ui.label(
                                RichText::new("solo")
                                    .family(theme::mono())
                                    .size(10.0)
                                    .color(DIM),
                            );
                        }
                        for (id, name) in peers {
                            let on = self.viewing.as_deref() == Some(id.as_str())
                                && self.panel_on(Overlay::Chars);
                            if theme::rail_row(ui, &name, on, false)
                            .clicked()
                            {
                                self.open_seat(Some(id));
                            }
                        }
                    });
                    rail_block(ui, "BOOKS", |ui| {
                        if self.blight {
                            if theme::rail_row(ui, "Datashard", cat_on("Datashard"), false)
                                .clicked()
                            {
                                self.open_table_catalog("Datashard", "datashard.json");
                            }
                            if theme::rail_row(ui, "Faces", cat_on("Faces"), false).clicked()
                            {
                                self.open_table_catalog("Faces", "npcs.json");
                            }
                            if theme::rail_row(ui, "Corps", cat_on("Corps"), false).clicked()
                            {
                                self.open_table_catalog("Corps", "corps.json");
                            }
                            if theme::rail_row(ui, "Gangs", cat_on("Gangs"), false).clicked()
                            {
                                self.open_table_catalog("Gangs", "gangs.json");
                            }
                            if theme::rail_row(ui, "Lore", cat_on("Lore"), false).clicked() {
                                self.open_table_catalog("Lore", "lore.json");
                            }
                        } else {
                            if theme::rail_row(ui, "Bestiary", cat_on("Bestiary"), false)
                                .clicked()
                            {
                                self.open_table_catalog("Bestiary", "bestiary.json");
                            }
                            if theme::rail_row(ui, "NPCs", cat_on("NPCs"), false).clicked() {
                                self.open_table_catalog("NPCs", "srd-npcs.json");
                            }
                            if theme::rail_row(ui, "Gods", cat_on("Gods"), false).clicked() {
                                self.open_table_catalog("Gods", "gods.json");
                            }
                        }
                    });
                    rail_block(ui, "GEAR", |ui| {
                        if theme::rail_row(ui, "Armory", self.panel_on(Overlay::Armory), false)
                        .clicked()
                        {
                            self.kit_filter.clear();
                            self.toggle_overlay(Overlay::Armory);
                        }
                        let vendor = if self.blight { "Night Market" } else { "Vendors" };
                        if theme::rail_row(ui, vendor, self.panel_on(Overlay::Vendors), false)
                            .clicked()
                        {
                            self.kit_filter.clear();
                            self.toggle_overlay(Overlay::Vendors);
                        }
                    });
                    rail_block(ui, "MORE", |ui| {
                        if self.blight
                            && theme::rail_row(ui, "Chess", self.panel_on(Overlay::Chess), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Chess);
                        }
                        if self.blight
                            && theme::rail_row(ui, "21", self.panel_on(Overlay::Blackjack), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Blackjack);
                        }
                        if theme::rail_row(ui, "Add sound", false, false).clicked() {
                            self.pick_sound();
                        }
                        if theme::rail_row(ui, "Maps", self.panel_on(Overlay::Maps), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Maps);
                        }
                        if theme::rail_row(ui, "Log", self.panel_on(Overlay::Log), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Log);
                        }
                        if theme::rail_row(ui, "Notes", self.panel_on(Overlay::Notes), false)
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Notes);
                        }
                        if theme::rail_row(ui, "Board", self.panel_on(Overlay::Board), false).clicked() {
                            self.toggle_overlay(Overlay::Board);
                        }
                        if self.blight
                            && theme::rail_row(ui, "Jack-in", self.page == Page::Netspace, false)
                            .clicked()
                        {
                            self.page = Page::Netspace;
                            self.jack_at = Instant::now();
                        }
                    });
    }

    #[allow(dead_code)]
    fn ui_table_overlay(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = (ui, pal);
    }

    fn ui_overlay_catalog(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        let cat = self.overlay_cat.to_uppercase();
        if theme::overlay_chrome(
            ui,
            "TABLE://BOOKS",
            &cat,
            "Browse · filter · select a row · drag onto Maps",
        ) {
            self.toggle_overlay(Overlay::Catalog);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("FILTER")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::chrome().cyan),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.catalog_q)
                    .hint_text("name or blurb…")
                    .desired_width(200.0),
            );
            ui.label(
                RichText::new("SELECT row · ACTIONS on detail")
                    .family(theme::mono())
                    .size(10.0)
                    .color(MUTED),
            );
        });
        self.refresh_cat_cache();
        ui.columns(2, |cols| {
            let n = self.cat_cache.len();
            egui::ScrollArea::vertical()
                .id_salt("ov-cat-list")
                .show_rows(&mut cols[0], 42.0, n, |ui, range| {
                    for row in range {
                        let (i, name, extra) = &self.cat_cache[row];
                        let i = *i;
                        let on = self.catalog_pick == i;
                        let art = self
                            .catalog_rows
                            .get(i)
                            .and_then(|v| v.get("id").and_then(|x| x.as_str()))
                            .map(|id| catalog_art(&self.root, self.overlay_cat, id))
                            .filter(|p| p.is_file())
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let sheet = self
                            .catalog_rows
                            .get(i)
                            .and_then(|v| v.get("id").and_then(|x| x.as_str()))
                            .unwrap_or("")
                            .to_string();
                        let resp = maps::drag_source(
                            ui,
                            ("cat", i),
                            || TokenSpec {
                                name: name.clone(),
                                image: art.clone(),
                                sheet: sheet.clone(),
                                cat: self.overlay_cat.to_string(),
                                src: sheet.clone(),
                            },
                            |ui| {
                                theme::catalog_row(ui, name, extra, on);
                            },
                        );
                        if resp.clicked() {
                            self.catalog_pick = i;
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("ov-cat-page")
                .show(&mut cols[1], |ui| {
                    let row = self.catalog_rows.get(self.catalog_pick).cloned();
                    if let Some(v) = row {
                        if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                            ui.label(
                                RichText::new(name)
                                    .family(theme::display())
                                    .size(20.0)
                                    .color(theme::chrome().acid),
                            );
                        }
                        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
                            let art = catalog_art(&self.root, self.overlay_cat, id);
                            if art.is_file() {
                                if images::show_fit(
                                    ui,
                                    &mut self.tex,
                                    &art,
                                    Vec2::new(ui.available_width().min(220.0).max(48.0), 140.0),
                                )
                                .on_hover_text("Click to zoom")
                                .clicked()
                                {
                                    self.zoom_path = Some(art);
                                    self.zoom_key = None;
                                }
                            }
                        }
                        wrap_text(ui, &pretty(&v), CREAM, 14.0);
                    }
                });
        });
    }

    fn ui_overlay_jackin(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        if theme::overlay_chrome(
            ui,
            "TABLE://JACKIN",
            "JACK-IN",
            "Walk the grid · WASD · full NETSPACE tab for co-presence",
        ) {
            self.toggle_overlay(Overlay::Jackin);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            let ping = self.netspace_ping_ms();
            let t = self.jack_at.elapsed().as_secs_f32();
            crate::netspace::hud(ui, &self.netspace, t, ping);
        });
        self.netspace.apply_table_seed(&self.net.table_key);
        let ping = self.netspace_ping_ms();
        let t = self.jack_at.elapsed().as_secs_f32();
        crate::netspace::paint(ui, &mut self.netspace, t, false, ping);
    }

    fn ui_overlay_maps(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        if theme::overlay_chrome(
            ui,
            "TABLE://MAPS",
            "MAPS",
            "Import · tokens · GM ink · press Maps or CLOSE",
        ) {
            self.toggle_overlay(Overlay::Maps);
            return;
        }
        let (dropped, ops) = maps::ui(
            ui,
            &mut self.map,
            &mut self.tex,
            &self.root,
            &mut self.target,
            self.is_gm,
        );
        for spec in dropped {
            self.spawn_catalog_token(&spec);
        }
        let sig = token_sig(&self.map.tokens);
        if sig != self.token_sig {
            self.token_sig = sig;
            if self.table_live() {
                self.net.send_map_tokens(self.portable_tokens());
            }
        }
        if self.table_live() {
            for op in ops {
                match op {
                    MapOp::Add(mark) => self.net.send_map_mark(mark),
                    MapOp::Del(id) => self.net.send_map_mark_del(&id),
                    MapOp::Clear => self.net.send_map_marks_clear(),
                    MapOp::Image => self.push_map_image(),
                }
            }
        }
        self.ui_map_token_roll(ui);
    }

    /// NPC / token Roll attack from map selection (no Seat open required).
    fn ui_map_token_roll(&mut self, ui: &mut egui::Ui) {
        let Some(tid) = self.target.clone() else {
            return;
        };
        let on_map = self.map.tokens.iter().any(|t| {
            let id = if t.sheet.is_empty() {
                t.id.as_str()
            } else {
                t.sheet.as_str()
            };
            id == tid
        });
        if !on_map {
            return;
        }
        let Some(ci) = self
            .chars
            .iter()
            .position(|c| c.id == tid || c.name == tid)
        else {
            ui.label(
                RichText::new("Selected token has no sheet — drop a catalog face or Aim a roster row.")
                    .color(MUTED)
                    .size(11.0),
            );
            return;
        };
        let cname = self.chars[ci].name.clone();
        let attacker_id = self.chars[ci].id.clone();
        let attacks = self.chars[ci].attacks.clone();
        if attacks.is_empty() {
            return;
        }
        let open_id = self.chars.get(self.char_i).map(|c| c.id.as_str());
        let victim = resolve_roll_victim(self.aim.as_deref(), open_id, &attacker_id);
        let victim_name = if victim.is_empty() {
            "—".into()
        } else {
            self.chars
                .iter()
                .find(|c| c.id == victim || c.name == victim)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| victim.clone())
        };
        let victim_src = if self
            .aim
            .as_deref()
            .is_some_and(|a| a == victim.as_str())
        {
            "Aim"
        } else if !victim.is_empty() {
            "open sheet"
        } else {
            "set Aim on roster"
        };
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("TOKEN ROLL · {cname}"))
                .family(theme::mono())
                .size(11.0)
                .color(theme::chrome().cyan),
        );
        ui.label(
            RichText::new(format!(
                "Attacker · {cname} (map) · Victim · {victim_name} ({victim_src})"
            ))
            .color(DIM)
            .size(11.0),
        );
        if self.map_atk_i >= attacks.len() {
            self.map_atk_i = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, atk) in attacks.iter().enumerate() {
                let on = self.map_atk_i == i;
                let lab = if atk.name.trim().is_empty() {
                    format!("Atk {}", i + 1)
                } else {
                    atk.name.clone()
                };
                if theme::neon_btn_color(ui, &lab, CYAN, on).clicked() {
                    self.map_atk_i = i;
                }
            }
        });
        let atk = &attacks[self.map_atk_i];
        let (hit, dmg, _) = crate::chars::scaled_attack(&self.chars[ci], atk);
        ui.label(
            RichText::new(format!("hit {hit}  {dmg}"))
                .family(theme::mono())
                .size(12.0)
                .color(CYAN),
        );
        let (use_luck, luck_note) =
            crate::chars::effective_attack_luck(&self.chars[ci], self.luck);
        if !luck_note.is_empty() {
            ui.label(
                RichText::new(luck_note)
                    .family(theme::mono())
                    .size(11.0)
                    .color(ORANGE),
            );
        }
        if theme::neon_btn(ui, "Roll attack").clicked() {
            let name = atk.name.clone();
            let low = name.to_lowercase();
            let heal = low.contains("heal") || low.contains("cure") || low.contains("aid");
            if use_luck != Luck::Norm {
                self.luck = use_luck;
            }
            let mut r = crate::dice::fire(&name, &dmg, heal, use_luck, &victim);
            r.attacker = attacker_id.clone();
            let note = if luck_note.is_empty() {
                String::new()
            } else {
                format!(" [{luck_note}]")
            };
            self.dice = format!(
                "{}{}: {}% {}{}",
                name,
                note,
                r.pct,
                r.grade,
                r.damage.map(|d| format!(" · {d}")).unwrap_or_default()
            );
            let line = format!(
                "{} uses {}{} → {} ({}){}",
                cname,
                name,
                note,
                if r.target.is_empty() { "—" } else { &r.target },
                r.grade,
                r.damage.map(|d| format!(" {d}")).unwrap_or_default()
            );
            self.combat_log.push(line.clone());
            save_combat_log(&self.root, &self.combat_log);
            if self.table_live() {
                self.net.send_chat(&format!("{COMBAT_MARK}{line}"), None, false);
            }
            self.roll = Some(r);
            self.apply_roll_hp();
            self.force_sheets();
        }
    }

    fn ui_overlay_chess(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://CHESS",
            "CHESS",
            "Host deals white · guest is black when Online",
        ) {
            self.toggle_overlay(Overlay::Chess);
            return;
        }
        let mine = if self.handle.trim().is_empty() {
            "YOU".into()
        } else {
            self.handle.clone()
        };
        if self.table_live() && self.net.role == Role::Host && self.chess.white == "White" {
            self.chess.white = mine.clone();
            if let Some(p) = self.net.peers.iter().find(|p| p.id != self.net.self_id) {
                self.chess.black = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };
            }
        }
        let guest = self.table_live() && self.net.role == Role::Guest;
        let my_turn = if !self.table_live() {
            true
        } else if self.net.role == Role::Host {
            self.chess.white_turn
        } else {
            !self.chess.white_turn
        };
        if guest {
            ui.label(RichText::new("You are black when a host is dealing the board.").color(DIM).size(11.0));
        }
        let can = my_turn || !self.table_live();
        let changed = crate::chess::paint(ui, &mut self.chess, can);
        if changed && self.table_live() {
            let body = self.chess.pack();
            self.net.send_pit("chess", &body);
        }
        let _ = mine;
    }

    fn publish_bj(&self) {
        if self.table_live() && self.net.role == Role::Host {
            let d = self.bj.dealer.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
            let p = self.bj.player.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
            self.net.send_pit(
                "bj",
                &format!(
                    "S|{}|{}|{}|{}|{}|{}",
                    self.bj.live as u8, self.bj.msg, d, p, self.bj.bank, self.bj.bet
                ),
            );
        }
    }

    fn apply_pit(&mut self, game: &str, body: &str, from: &str) {
        if game == "init" {
            self.apply_init_wire(body);
            return;
        }
        if game == "chess" {
            if let Some(g) = crate::chess::Game::unpack(body) {
                self.chess = g;
                if !self.panel_on(Overlay::Chess) {
                    self.open.push(Overlay::Chess);
                }
            }
            return;
        }
        if game != "bj" {
            return;
        }
        if self.net.role == Role::Host && (body == "hit" || body == "stand" || body == "deal") {
            if body == "deal" && !self.bj.live {
                self.bj.player = vec![card(), card()];
                self.bj.dealer = vec![card(), card()];
                self.bj.live = total(&self.bj.player) != 21;
                self.bj.msg = if self.bj.live { "Hit or stand.".into() } else { "Blackjack.".into() };
            } else if body == "hit" && self.bj.live {
                self.bj.player.push(card());
                if total(&self.bj.player) > 21 {
                    self.bj.live = false;
                    self.bj.msg = format!("{from} busts.");
                }
            } else if body == "stand" && self.bj.live {
                while total(&self.bj.dealer) < 17 {
                    self.bj.dealer.push(card());
                }
                let p = total(&self.bj.player);
                let d = total(&self.bj.dealer);
                self.bj.live = false;
                self.bj.msg = if d > 21 || p > d {
                    "The table wins.".into()
                } else if p == d {
                    self.bj.bank += self.bj.bet;
                    "Push.".into()
                } else {
                    "House wins.".into()
                };
            }
            self.publish_bj();
            return;
        }
        let mut parts = body.split('|');
        if parts.next() != Some("S") {
            return;
        }
        self.bj.live = parts.next() == Some("1");
        self.bj.msg = parts.next().unwrap_or("").to_string();
        self.bj.dealer = parts
            .next()
            .unwrap_or("")
            .split(',')
            .filter_map(|n| n.parse().ok())
            .collect();
        self.bj.player = parts
            .next()
            .unwrap_or("")
            .split(',')
            .filter_map(|n| n.parse().ok())
            .collect();
        if let Some(n) = parts.next().and_then(|s| s.parse().ok()) {
            self.bj.bank = n;
        }
        if let Some(n) = parts.next().and_then(|s| s.parse().ok()) {
            self.bj.bet = n;
        }
        if !self.panel_on(Overlay::Blackjack) {
            self.open.push(Overlay::Blackjack);
        }
    }

    fn ui_overlay_bj(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        if theme::overlay_chrome(
            ui,
            "TABLE://21",
            "HOUSE 21",
            "Deal · hit · stand · chips stay on this table",
        ) {
            self.toggle_overlay(Overlay::Blackjack);
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("bj-overlay")
            .show(ui, |ui| {
                self.ui_bj_body(ui, pal);
            });
    }

    fn ui_overlay_kit(&mut self, ui: &mut egui::Ui, _pal: theme::Palette, vendor: bool) {
        let title = if vendor {
            if self.blight {
                "Night Market"
            } else {
                "Vendors"
            }
        } else {
            "Armory"
        };
        let loc = if vendor { "TABLE://VENDORS" } else { "TABLE://ARMORY" };
        let next = if vendor {
            "Filter · Buy · equip on Gear"
        } else {
            "Filter · GM Give / drag onto sheet"
        };
        if theme::overlay_chrome(ui, loc, &title.to_uppercase(), next) {
            self.toggle_overlay(if vendor { Overlay::Vendors } else { Overlay::Armory });
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("FILTER")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::chrome().cyan),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.kit_filter)
                    .hint_text("name, cat, or blurb…")
                    .desired_width(180.0),
            );
            if let Some(c) = self.chars.get(self.char_i) {
                ui.label(
                    RichText::new(if self.blight {
                        format!("{} · {} eb", c.name, c.eddies)
                    } else {
                        format!("{} · {} gp", c.name, c.gp)
                    })
                    .color(theme::chrome().cyan)
                    .family(theme::mono())
                    .size(12.0),
                );
            }
        });
        if vendor {
            wrap_text(
                ui,
                "Stock is shuffled for this buyer. Nothing above their level is on the stall. Players buy here. The gamemaster drags from Armory onto a sheet.",
                MUTED,
                11.0,
            );
        } else if self.is_gm {
            wrap_text(
                ui,
                "Drag an item onto the character sheet to add it. Only the gamemaster can do that.",
                MUTED,
                11.0,
            );
        } else {
            wrap_text(
                ui,
                "Look, don't take. Buy from the stall.",
                MUTED,
                11.0,
            );
        }
        let file = if self.blight {
            "red-kit.json"
        } else {
            "srd-kit.json"
        };
        let sig = format!("{}:{}", file, vendor);
        if self.kit_sig != sig {
            self.kit_rows = if vendor {
                if self.vendor_stock.is_empty() {
                    self.restock_vendor();
                }
                self.vendor_stock.clone()
            } else {
                load_list(&self.root, file)
            };
            self.kit_sig = sig;
        } else if vendor && self.kit_rows.is_empty() && !self.vendor_stock.is_empty() {
            self.kit_rows = self.vendor_stock.clone();
        }
        let q = self.kit_filter.to_lowercase();
        let shown: Vec<usize> = self
            .kit_rows
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                if q.is_empty() {
                    return true;
                }
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let cat = v.get("cat").and_then(|x| x.as_str()).unwrap_or("");
                format!("{} {} {}", name, cat, kit_blurb(v))
                    .to_lowercase()
                    .contains(&q)
            })
            .map(|(i, _)| i)
            .collect();
        let mut bought: Option<KitSpec> = None;
        let mut paid = 0;
        let mut fail = String::new();
        egui::ScrollArea::vertical()
            .id_salt("kit")
            .show_rows(ui, 64.0, shown.len(), |ui, range| {
                for row in range {
                    let idx = shown[row];
                    let spec = crate::chars::kit_spec_from_json(&self.kit_rows[idx]);
                    let blurb = kit_blurb(&self.kit_rows[idx]);
                    let lvl = crate::chars::item_min_level(&self.kit_rows[idx]);
                    let name = spec.name.clone();
                    let cat = spec.cat.clone();
                    let cost = spec.cost.clone();
                    let dmg = spec.dmg.clone();
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let id = spec.id.clone();
                        if let Some(art) = images::kit_art(&self.root, &id, &name) {
                            if images::show_fit(ui, &mut self.tex, &art, Vec2::splat(56.0))
                                .on_hover_text("Click to zoom")
                                .clicked()
                            {
                                self.zoom_path = Some(art);
                                self.zoom_key = None;
                            }
                        }
                        ui.vertical(|ui| {
                            let focused = self.kit_focus == spec.id;
                            let ch_kit = theme::chrome();
                            let row = |ui: &mut egui::Ui| {
                                ui.label(
                                    RichText::new(&name)
                                        .color(if focused { ch_kit.acid } else { CREAM })
                                        .size(15.0)
                                        .family(theme::ui_font()),
                                );
                                if !cat.is_empty() || !cost.is_empty() || !dmg.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            [cat.as_str(), cost.as_str(), dmg.as_str()]
                                                .into_iter()
                                                .filter(|s| !s.is_empty())
                                                .collect::<Vec<_>>()
                                                .join(" · "),
                                        )
                                        .color(ch_kit.cyan)
                                        .size(12.0)
                                        .family(theme::mono()),
                                    );
                                }
                            };
                            if self.is_gm && !vendor {
                                let picked = crate::maps::drag_click(
                                    ui,
                                    ("kit-drag", spec.id.as_str()),
                                    || spec.clone(),
                                    |ui| row(ui),
                                );
                                if picked.clicked() {
                                    self.kit_focus = spec.id.clone();
                                }
                            } else {
                                let body = ui.scope(|ui| row(ui));
                                if body
                                    .response
                                    .interact(egui::Sense::click())
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.kit_focus = spec.id.clone();
                                }
                            }
                            if !blurb.is_empty() {
                                wrap_text(ui, &blurb, CREAM, 14.0);
                            }
                        });
                        if vendor {
                            if theme::neon_btn(ui, "Buy").clicked() {
                                let price = parse_coins(&cost);
                                if let Some(c) = self.chars.get(self.char_i) {
                                    let have = if c.is_blight() {
                                        c.role_rank.max(c.level)
                                    } else {
                                        c.level
                                    };
                                    if lvl > have {
                                        fail = "Too rich for this runner.".into();
                                    } else if self.blight {
                                        if c.eddies >= price {
                                            bought = Some(spec.clone());
                                            paid = price;
                                        } else {
                                            fail = "Not enough eddies.".into();
                                        }
                                    } else if c.gp >= price {
                                        bought = Some(spec.clone());
                                        paid = price;
                                    } else {
                                        fail = "Not enough coin.".into();
                                    }
                                }
                            }
                        } else if self.is_gm && theme::neon_btn(ui, "Give").clicked() {
                            bought = Some(spec.clone());
                            paid = 0;
                        }
                    });
                }
            });
        if let Some(spec) = bought {
            if let Some(c) = self.chars.get_mut(self.char_i) {
                if paid > 0 {
                    if self.blight {
                        c.eddies -= paid;
                    } else {
                        c.gp -= paid;
                    }
                }
                let nm = spec.name.clone();
                crate::chars::receive_kit(c, &spec);
                self.mix_msg = if vendor {
                    format!("Bought {nm}. Equip it on Gear.")
                } else {
                    format!("{nm} on the sheet. Equip it on Gear.")
                };
            }
            crate::chars::save(&self.root, &self.chars);
        } else if !fail.is_empty() {
            self.mix_msg = fail;
        }
    }

    fn ui_place_tile(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://PLACE",
            "PLACE",
            "Painting for this world · top bar also opens it",
        ) {
            self.toggle_overlay(Overlay::Place);
            return;
        }
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, egui::Sense::hover());
        let path = self.painting_path();
        images::paint_cover(ui, &mut self.tex, &path, rect);
        ui.painter().text(
            rect.left_top() + Vec2::new(12.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("{}  ·  {}", self.place_name(), period_label(self.time)),
            egui::FontId::new(14.0, theme::display()),
            theme::chrome().acid,
        );
    }

    fn ui_calendar_tile(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://CAL",
            "CALENDAR",
            "GM sets the day · vendors restock on change",
        ) {
            self.toggle_overlay(Overlay::Calendar);
            return;
        }
        let month = [
            "", "January", "February", "March", "April", "May", "June", "July", "August",
            "September", "October", "November", "December",
        ];
        let name = month.get(self.cal_m as usize).copied().unwrap_or("");
        let ch = theme::chrome();
        ui.horizontal_wrapped(|ui| {
            if self.is_gm && theme::neon_btn(ui, "◀").clicked() {
                self.advance_month(-1);
            }
            ui.label(
                RichText::new(format!("{name} {}", self.cal_y))
                    .family(theme::display())
                    .size(20.0)
                    .color(ch.acid),
            );
            if self.is_gm && theme::neon_btn(ui, "▶").clicked() {
                self.advance_month(1);
            }
        });
        ui.horizontal(|ui| {
            let ch = theme::chrome();
            for day in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
                ui.add_sized(
                    [36.0, 18.0],
                    egui::Label::new(
                        RichText::new(day)
                            .family(theme::mono())
                            .size(11.0)
                            .color(ch.cyan),
                    ),
                );
            }
        });
        let first = weekday_sun0(self.cal_y, self.cal_m, 1);
        let dim = days_in_month(self.cal_y, self.cal_m as i32) as u32;
        let mut day = 1u32;
        for week in 0..6 {
            if day > dim {
                break;
            }
            ui.horizontal(|ui| {
                for col in 0..7 {
                    let slot = week * 7 + col;
                    if slot < first || day > dim {
                        ui.add_sized([36.0, 32.0], egui::Label::new(""));
                    } else {
                        let n = day;
                        let on = n == self.cal_d;
                        if theme::neon_btn_color(ui, &n.to_string(), theme::chrome().cyan, on).clicked() && self.is_gm
                        {
                            self.cal_d = n;
                            save_calendar(&self.root, self.cal_y, self.cal_m, self.cal_d);
                            self.restock_vendor();
                        }
                        day += 1;
                    }
                }
            });
        }
        if !self.is_gm {
            wrap_text(ui, "The gamemaster sets the day.", DIM, 12.0);
        }
    }

    fn ui_calc_tile(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://CALC",
            "CALC",
            "Local only · keys never go to the table",
        ) {
            self.toggle_overlay(Overlay::Calc);
            return;
        }
        ui.label(
            RichText::new(&self.calc_entry)
                .family(theme::mono())
                .size(28.0)
                .color(CREAM),
        );
        let keys = [
            ["7", "8", "9", "÷"],
            ["4", "5", "6", "×"],
            ["1", "2", "3", "−"],
            ["0", ".", "C", "+"],
            ["=", "", "", ""],
        ];
        for row in keys {
            ui.horizontal(|ui| {
                for key in row {
                    if key.is_empty() {
                        continue;
                    }
                    if theme::neon_btn(ui, key).clicked() {
                        self.calc_press(key);
                    }
                }
            });
        }
        wrap_text(ui, "This calculator stays on this computer.", DIM, 11.0);
    }

    fn calc_press(&mut self, key: &str) {
        match key {
            "C" => {
                self.calc_acc = None;
                self.calc_op = None;
                self.calc_entry = "0".into();
                self.calc_fresh = true;
            }
            "÷" | "×" | "−" | "+" => {
                let n = self.calc_entry.parse::<f64>().unwrap_or(0.0);
                if let (Some(acc), Some(op)) = (self.calc_acc, self.calc_op) {
                    self.calc_acc = Some(calc_apply(acc, op, n));
                    self.calc_entry = fmt_calc(self.calc_acc.unwrap_or(0.0));
                } else {
                    self.calc_acc = Some(n);
                }
                self.calc_op = Some(key.chars().next().unwrap_or('+'));
                self.calc_fresh = true;
            }
            "=" => {
                let n = self.calc_entry.parse::<f64>().unwrap_or(0.0);
                if let (Some(acc), Some(op)) = (self.calc_acc, self.calc_op) {
                    let out = calc_apply(acc, op, n);
                    self.calc_entry = fmt_calc(out);
                    self.calc_acc = Some(out);
                    self.calc_op = None;
                    self.calc_fresh = true;
                }
            }
            "." => {
                if self.calc_fresh {
                    self.calc_entry = "0.".into();
                    self.calc_fresh = false;
                } else if !self.calc_entry.contains('.') {
                    self.calc_entry.push('.');
                }
            }
            d if d.chars().all(|c| c.is_ascii_digit()) => {
                if self.calc_fresh || self.calc_entry == "0" {
                    self.calc_entry = d.to_string();
                    self.calc_fresh = false;
                } else if self.calc_entry.len() < 16 {
                    self.calc_entry.push_str(d);
                }
            }
            _ => {}
        }
    }

    fn ui_table_sky(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        let avail = ui.available_height();
        let reserve = if self.blight { 86.0 } else { 8.0 };
        let h = if avail.is_finite() {
            (avail - reserve).clamp(48.0, avail.max(48.0))
        } else {
            200.0
        };
        if h < 48.0 {
            return;
        }
        let rect = ui.available_rect_before_wrap();
        let sky = Rect::from_min_size(
            rect.min + Vec2::new(8.0, 6.0),
            Vec2::new(rect.width() - 16.0, h),
        );
        ui.allocate_rect(sky, egui::Sense::hover());
        theme::plate(ui, sky);
        let inner = sky.shrink(6.0);
        let path = self.painting_path();
        images::paint_cover(ui, &mut self.tex, &path, inner);
        theme::brackets(ui, inner.shrink(6.0), CYAN, 14.0);
        let paint = ui.painter().with_clip_rect(inner);
        let mut top_y = inner.top() + 12.0;
        if self.blight {
            let chip = Rect::from_min_size(inner.left_top() + Vec2::new(12.0, 12.0), Vec2::new(176.0, 36.0));
            theme::fill_chamfer(ui, chip, 6.0, PANEL, egui::Stroke::new(1.0, CYAN));
            let st = crate::stations::get(&self.radio_station)
                .map(|s| format!("{} {}", s.freq, s.call))
                .unwrap_or_else(|| "OFF AIR".into());
            let sub = if self.radio_on {
                self.catalog
                    .layer(true, &self.radio_track)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "ON AIR".into())
            } else {
                "TUNE A STATION".into()
            };
            let clip = ui.painter().with_clip_rect(chip.shrink(4.0));
            clip.text(
                chip.left_top() + Vec2::new(10.0, 6.0),
                egui::Align2::LEFT_TOP,
                st,
                FontId::new(12.0, theme::mono()),
                CYAN,
            );
            clip.text(
                chip.left_bottom() + Vec2::new(10.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                sub,
                FontId::new(10.0, theme::mono()),
                MUTED,
            );
            if ui
                .interact(chip, egui::Id::new("radio-chip"), egui::Sense::click())
                .clicked()
            {
                self.cycle_radio();
            }
            top_y = chip.bottom() + 8.0;
        }
        let place = self.place_name().to_uppercase();
        paint.text(
            egui::pos2(inner.left() + 14.0, top_y),
            egui::Align2::LEFT_TOP,
            place,
            FontId::new(16.0, theme::display()),
            CYAN,
        );
        let playing: Vec<&str> = self
            .mixer
            .playing()
            .filter(|(id, _)| !id.starts_with("__"))
            .filter_map(|(id, _)| {
                self.catalog
                    .layer(self.blight, id)
                    .map(|l| l.name.as_str())
                    .or_else(|| {
                        self.custom
                            .iter()
                            .find(|l| &l.id == id)
                            .map(|l| l.name.as_str())
                    })
            })
            .take(3)
            .collect();
        let mut pill = if playing.is_empty() {
            "STANDBY · CHOOSE A SCENE".to_string()
        } else {
            playing.join(" · ")
        };
        if pill.chars().count() > 36 {
            pill = format!("{}…", pill.chars().take(34).collect::<String>());
        }
        paint.text(
            inner.right_top() + Vec2::new(-12.0, 12.0),
            egui::Align2::RIGHT_TOP,
            pill,
            FontId::new(11.0, theme::mono()),
            CYAN,
        );
        let t = ui.input(|i| i.time) as f32;
        let levels: Vec<f32> = self
            .mixer
            .playing()
            .filter(|(id, _)| !id.starts_with("__"))
            .map(|(_, v)| v)
            .collect();
        if !levels.is_empty() {
            let viz = Rect::from_min_max(
                inner.left_bottom() + Vec2::new(16.0, -52.0),
                inner.right_bottom() + Vec2::new(-16.0, -10.0),
            );
            let n = levels.len().max(1);
            let w = viz.width() / n as f32;
            for (i, vol) in levels.iter().enumerate() {
                let pulse = ((t * (2.4 + i as f32 * 0.37)).sin().abs() * 0.45 + 0.55) * *vol;
                let h = viz.height() * pulse.clamp(0.06, 1.0);
                let x = viz.left() + i as f32 * w;
                let bar = Rect::from_min_max(
                    egui::pos2(x + 2.0, viz.bottom() - h),
                    egui::pos2(x + w - 2.0, viz.bottom()),
                );
                ui.painter().rect_filled(
                    bar,
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 106, 18, 180),
                );
            }
        }
        if self.blight {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("RADIO")
                        .family(theme::mono())
                        .size(10.0)
                        .color(DIM),
                );
                if self.radio_on && theme::neon_btn_color(ui, "Stop", KILL, false).clicked() {
                    self.stop_radio();
                }
            });
            paint_air_legend(ui);
            egui::ScrollArea::horizontal()
                .id_salt("blight-radio")
                .max_height(40.0)
                .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        for st in crate::stations::all() {
                            let on = self.radio_on && self.radio_station == st.id;
                            let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.0, self.air_color(st.id));
                            let label = if st.url.is_some() {
                                st.call.to_string()
                            } else {
                                format!("{} {}", st.freq, st.call)
                            };
                            let word = self.air_word(st.id);
                            let resp = theme::neon_btn_color(ui, &label, CYAN, on);
                            if resp.clicked() {
                                if on && st.url.is_none() {
                                    self.play_radio_track();
                                } else if !on {
                                    self.tune_station(st.id);
                                }
                            }
                            resp.on_hover_text(format!("{} · {word}", st.name));
                        }
                    });
                });
            if self.radio_on {
                let id = self.radio_station.clone();
                let name = crate::stations::get(&id)
                    .map(|s| s.name)
                    .unwrap_or("Station");
                let track = self
                    .catalog
                    .layer(true, &self.radio_track)
                    .map(|l| l.name.as_str())
                    .unwrap_or("…");
                let word = self.air_word(&id);
                wrap_text(ui, &format!("{name} · {word} · {track}"), CYAN, 11.0);
            }
        }
    }

    fn ui_scenes_panel(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        theme::plate(ui, ui.max_rect());
        ui.add_space(6.0);
        if theme::overlay_chrome(
            ui,
            "TABLE://SCENES",
            "SCENES",
            "Pick a look · ★ saved mixes · Save this mix",
        ) {
            self.toggle_overlay(Overlay::Scenes);
            return;
        }
        let energy = self.mixer.master.max(0.05);
        let phase = ui.input(|i| i.time) as f32 * 0.25;
        paint_viz(ui, ui.available_width().max(40.0), 22.0, energy, phase);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("FILTER")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::chrome().cyan),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("scene name or blurb…")
                    .desired_width((ui.available_width() - 16.0).max(80.0)),
            );
        });
        let q = self.search.to_lowercase();
        let saved: Vec<(usize, String)> = self
            .saved
            .iter()
            .enumerate()
            .filter(|(_, s)| s.blight == self.blight)
            .filter(|(_, s)| q.is_empty() || s.name.to_lowercase().contains(&q))
            .map(|(i, s)| (i, s.name.clone()))
            .collect();
        let scenes: Vec<(String, String, String)> = self
            .catalog
            .scenes(self.blight)
            .iter()
            .filter(|s| {
                q.is_empty()
                    || s.name.to_lowercase().contains(&q)
                    || s.blurb.to_lowercase().contains(&q)
            })
            .map(|s| (s.id.clone(), s.name.clone(), s.blurb.clone()))
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("scenes")
            .show(ui, |ui| {
                for (i, name) in &saved {
                    if theme::wide_btn(ui, &format!("★ {name}"), "saved mix", false).clicked() {
                        self.apply_saved(*i);
                    }
                }
                for (id, name, blurb) in &scenes {
                    let on = self.scene == *id;
                    if theme::wide_btn(ui, name, blurb, on).clicked() {
                        self.apply_scene(id);
                    }
                }
            });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Save this mix").clicked() {
                self.save_this_mix();
            }
        });
        let _ = pal;
    }

    fn ui_mix_panel(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        theme::plate(ui, ui.max_rect());
        ui.add_space(6.0);
        let n = self
            .mixer
            .playing()
            .filter(|(id, _)| !id.starts_with("__"))
            .count();
        let note = if n == 0 {
            "Nothing playing · pick layers or Shuffle"
        } else if !self.mix_msg.is_empty() {
            self.mix_msg.as_str()
        } else {
            "Layers live · check to play · slider is volume"
        };
        if theme::overlay_chrome(ui, "TABLE://MIX", "THE MIX", note) {
            self.toggle_overlay(Overlay::Mix);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn_color(ui, "Shuffle", theme::chrome().acid, false).clicked() {
                self.shuffle_music();
            }
            ui.label(
                RichText::new(if n == 0 {
                    "0 LIVE".into()
                } else {
                    format!("{n} LIVE")
                })
                .family(theme::mono())
                .size(11.0)
                .color(theme::chrome().cyan),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("FILTER")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::chrome().cyan),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.mix_search)
                    .hint_text("layer name…")
                    .desired_width((ui.available_width() - 12.0).max(80.0)),
            );
        });
        ui.horizontal_wrapped(|ui| {
            let ch = theme::chrome();
            let cats: Vec<&str> = if self.blight {
                vec!["all", "music", "ambience", "animals"]
            } else {
                vec!["all", "music", "weather", "animals", "ambience"]
            };
            for c in cats {
                if theme::neon_btn_color(ui, c, ch.cyan, self.cat_filter == c).clicked() {
                    self.cat_filter = c.into();
                }
            }
        });
        if !self.err.is_empty() {
            ui.colored_label(CYAN, &self.err);
        }
        let playing: Vec<String> = self.mixer.playing().map(|(id, _)| id.clone()).collect();
        self.refresh_mix_cache();
        let n = self.mix_cache.len();
        egui::ScrollArea::vertical()
            .id_salt("mix")
            .show_rows(ui, 36.0, n, |ui, range| {
                for row in range {
                    let (id, name) = self.mix_cache[row].clone();
                    let on = playing.iter().any(|p| p == &id);
                    ui.horizontal(|ui| {
                        let mut enabled = on;
                        if ui.checkbox(&mut enabled, "").changed() {
                            self.toggle_layer(&id);
                        }
                        if theme::neon_btn_color(ui, &name, CYAN, on).clicked() {
                            self.toggle_layer(&id);
                        }
                        let mut v = if on {
                            self.mixer.volume_of(&id)
                        } else {
                            0.45
                        };
                        if ui
                            .add(
                                egui::Slider::new(&mut v, 0.0..=1.0)
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .changed()
                        {
                            if on {
                                self.mixer.set_volume(&id, v);
                                self.broadcast_mix();
                            } else if let Some(file) = self
                                .catalog
                                .layer(self.blight, &id)
                                .and_then(|l| l.audio_file())
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    self.custom
                                        .iter()
                                        .find(|l| l.id == id)
                                        .and_then(|l| l.audio_file().map(|s| s.to_string()))
                                })
                            {
                                let _ = self.mixer.play(&id, &file, v);
                                let p = self.presence_of(&id);
                                self.mixer.set_presence(&id, p);
                            }
                        }
                    });
                }
            });
        let _ = pal;
    }

    fn ui_overlay_log(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://LOG",
            "COMBAT LOG",
            "Init tracker · shared rolls · press Log or CLOSE",
        ) {
            self.toggle_overlay(Overlay::Log);
            return;
        }
        self.ui_init_tracker(ui);
        ui.add_space(4.0);
        let combat_h = ui.available_height().max(80.0);
        ui.label(
            RichText::new("ROLLS")
                .family(theme::mono())
                .size(11.0)
                .color(theme::chrome().cyan),
        );
        egui::ScrollArea::vertical()
            .id_salt("combat-log")
            .max_height(combat_h)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.combat_log.is_empty() {
                    ui.label(
                        RichText::new("Rolls, rests, and hits land here. Chat stays in TALK.")
                            .color(DIM)
                            .size(13.0),
                    );
                }
                for line in &self.combat_log {
                    ui.label(
                        RichText::new(line)
                            .family(theme::mono())
                            .size(13.0)
                            .color(CREAM),
                    );
                }
            });
    }

    fn can_drive_init(&self) -> bool {
        !self.table_live() || self.net.role == Role::Host || self.is_gm
    }

    fn push_init(&self) {
        if !self.table_live() {
            return;
        }
        if !(self.net.role == Role::Host || self.is_gm) {
            return;
        }
        self.net.send_pit("init", &pack_init(&self.init_rows, self.init_turn));
    }

    fn apply_init_wire(&mut self, body: &str) {
        if body == "ask" {
            if self.net.role == Role::Host || self.is_gm {
                self.push_init();
            }
            return;
        }
        // Graceful: seat mid-edit keeps local scores until DragValue blurs.
        if self.init_editing {
            return;
        }
        if let Some((rows, turn)) = unpack_init(body) {
            self.init_rows = rows;
            self.init_turn = turn;
        }
    }

    fn ui_init_tracker(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("INIT")
                .family(theme::mono())
                .size(11.0)
                .color(theme::chrome().cyan),
        );
        let drive = self.can_drive_init();
        ui.label(
            RichText::new(if self.table_live() {
                if drive {
                    "Host/GM drives order — peers sync over Pit. Roll or set, then Next."
                } else {
                    "Host/GM drives order — you see the live roster."
                }
            } else {
                "Clerical turn order — roll or set, then Next. No auto tactics."
            })
            .color(DIM)
            .size(11.0),
        );
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            if drive && theme::neon_btn(ui, "Sync roster").clicked() {
                let prev: HashMap<String, (i32, bool)> = self
                    .init_rows
                    .iter()
                    .map(|r| (r.id.clone(), (r.score, r.ready)))
                    .collect();
                let mut rows = build_init_roster(&self.chars, &self.map.tokens);
                for r in &mut rows {
                    if let Some((sc, ready)) = prev.get(&r.id) {
                        r.score = *sc;
                        r.ready = *ready;
                    }
                }
                sort_init_roster(&mut rows);
                self.init_rows = rows;
                if self.init_turn >= self.init_rows.len() {
                    self.init_turn = 0;
                }
                dirty = true;
            }
            if drive && theme::neon_btn(ui, "Roll all").clicked() {
                if self.init_rows.is_empty() {
                    self.init_rows = build_init_roster(&self.chars, &self.map.tokens);
                }
                for r in &mut self.init_rows {
                    if let Some(c) = self.chars.iter().find(|c| c.id == r.id) {
                        r.score = crate::chars::roll_initiative(c);
                        r.ready = true;
                    } else {
                        r.score = rand::thread_rng().gen_range(1..=20);
                        r.ready = true;
                    }
                }
                sort_init_roster(&mut self.init_rows);
                self.init_turn = 0;
                let line = format!(
                    "Init rolled · {}",
                    self.init_rows
                        .iter()
                        .map(|r| format!("{} {}", r.name, r.score))
                        .collect::<Vec<_>>()
                        .join(" · ")
                );
                self.push_combat_silent(line);
                dirty = true;
            }
            if drive && theme::neon_btn(ui, "Next").clicked() && !self.init_rows.is_empty() {
                self.init_turn = (self.init_turn + 1) % self.init_rows.len();
                dirty = true;
            }
            if drive && theme::muted_btn(ui, "Clear").clicked() {
                self.init_rows.clear();
                self.init_turn = 0;
                dirty = true;
            }
        });
        if self.init_rows.is_empty() {
            ui.label(
                RichText::new(if drive {
                    "Sync roster to pull PCs + map tokens, then Roll all or set scores."
                } else {
                    "Waiting for Host/GM init sync…"
                })
                .color(MUTED)
                .size(12.0),
            );
            if dirty {
                self.push_init();
            }
            return;
        }
        let mut resort = false;
        let mut editing = false;
        let turn = self.init_turn;
        let n = self.init_rows.len();
        for i in 0..n {
            let is_turn = i == turn;
            let id = self.init_rows[i].id.clone();
            let name = self.init_rows[i].name.clone();
            ui.horizontal(|ui| {
                let mark = if is_turn { "►" } else { "·" };
                ui.label(
                    RichText::new(format!("{mark} {name}"))
                        .family(theme::mono())
                        .size(12.0)
                        .color(if is_turn {
                            theme::chrome().acid
                        } else {
                            CREAM
                        }),
                );
                let mut score = self.init_rows[i].score;
                if drive {
                    let resp = ui.add(egui::DragValue::new(&mut score).range(-20..=40).speed(1.0));
                    if resp.has_focus() || resp.dragged() {
                        editing = true;
                    }
                    if resp.changed() {
                        self.init_rows[i].score = score;
                        self.init_rows[i].ready = true;
                        resort = true;
                        dirty = true;
                    }
                    if theme::neon_btn(ui, "Roll").clicked() {
                        if let Some(c) = self.chars.iter().find(|c| c.id == id) {
                            self.init_rows[i].score = crate::chars::roll_initiative(c);
                        } else {
                            self.init_rows[i].score = rand::thread_rng().gen_range(1..=20);
                        }
                        self.init_rows[i].ready = true;
                        resort = true;
                        dirty = true;
                    }
                } else {
                    ui.label(
                        RichText::new(format!("{score}"))
                            .family(theme::mono())
                            .size(12.0)
                            .color(CYAN),
                    );
                }
            });
        }
        self.init_editing = editing;
        if resort {
            let cur_id = self
                .init_rows
                .get(self.init_turn)
                .map(|r| r.id.clone());
            sort_init_roster(&mut self.init_rows);
            if let Some(id) = cur_id {
                if let Some(ni) = self.init_rows.iter().position(|r| r.id == id) {
                    self.init_turn = ni;
                }
            }
        }
        if dirty {
            self.push_init();
        }
    }

    fn ui_rail_notes(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://NOTES",
            "NOTES",
            "This deck only · never sent · press Notes or CLOSE",
        ) {
            self.toggle_overlay(Overlay::Notes);
            return;
        }
        let h = ui.available_height().max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("notes-rail")
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let nw = ui.available_width().max(80.0);
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut self.notes)
                        .desired_width(nw)
                        .desired_rows(12)
                        .font(FontId::new(14.0, theme::ui_font()))
                        .text_color(CREAM),
                );
                if resp.changed() {
                    self.notes_dirty = true;
                    self.notes_at = Instant::now();
                }
                if resp.lost_focus() {
                    self.flush_notes_now();
                }
            });
    }

    fn flush_notes(&mut self) {
        if self.notes_dirty && self.notes_at.elapsed() > Duration::from_millis(700) {
            self.flush_notes_now();
        }
    }

    fn flush_notes_now(&mut self) {
        if !self.notes_dirty {
            return;
        }
        save_notes(&self.root, &self.handle, &self.notes);
        self.notes_dirty = false;
    }

    fn push_combat_silent(&mut self, line: String) {
        self.combat_log.push(line);
        save_combat_log(&self.root, &self.combat_log);
    }

    fn ui_chars_panel(&mut self, ui: &mut egui::Ui, _pal: theme::Palette) {
        if theme::overlay_chrome(
            ui,
            "TABLE://SHEET",
            "CHARACTERS",
            "Roster selects · Aim secondary · Delete is KILL",
        ) {
            self.open.retain(|o| *o != Overlay::Chars);
            self.viewing = None;
            return;
        }
        let h = ui.available_height().max(80.0);
        let w = ui.available_width().max(80.0);
        ui.set_max_width(w);
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        egui::ScrollArea::vertical()
            .id_salt("sheet-root")
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width((w - 6.0).max(40.0));
                self.run_sheet(ui);
            });
    }

    fn ui_table_expand(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        let shown = egui::Frame::NONE
            .fill(Color32::from_rgb(10, 10, 6))
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                if self.watch_open {
                    theme::kicker(ui, "CLK://TABLE");
                    ui.horizontal_wrapped(|ui| {
                        if theme::neon_btn(ui, "−").clicked() {
                            self.shift_clock(-15);
                        }
                        ui.label(
                            RichText::new(format_clock(self.clock))
                                .family(theme::mono())
                                .size(16.0)
                                .color(CYAN),
                        );
                        if theme::neon_btn(ui, "+").clicked() {
                            self.shift_clock(15);
                        }
                        if self.is_gm {
                            if theme::neon_btn_color(ui, "Run clock", theme::ACID, self.clock_run)
                                .clicked()
                            {
                                self.clock_run = !self.clock_run;
                            }
                            self.date_stepper(ui);
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "{:04}-{:02}-{:02}",
                                    self.cal_y, self.cal_m, self.cal_d
                                ))
                                .family(theme::mono())
                                .size(14.0)
                                .color(theme::ACID),
                            );
                        }
                        let mut mins = self.clock as i32;
                        if ui
                            .add(
                                egui::Slider::new(&mut mins, 0..=1439)
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .changed()
                        {
                            self.set_clock(mins as u32);
                        }

                    });
                }
                if self.place_open {
                    theme::kicker(ui, "PLACE://PAINTING");
                    let sets: Vec<(String, String)> = self
                        .catalog
                        .settings(self.blight)
                        .iter()
                        .map(|s| (s.id.clone(), s.name.clone()))
                        .collect();
                    ui.horizontal_wrapped(|ui| {
                        for (id, name) in &sets {
                            let on = self.place == *id;
                            let path = self.painting_for(id);
                            let (rect, resp) = ui.allocate_exact_size(
                                Vec2::new(148.0, 84.0),
                                egui::Sense::click(),
                            );
                            theme::fill_chamfer(
                                ui,
                                rect,
                                6.0,
                                PANEL,
                                egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { theme::chrome().acid } else { theme::fade(theme::chrome().cyan, 90) }),
                            );
                            let mut pic = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect.shrink(4.0))
                                    .layout(egui::Layout::top_down(egui::Align::Min)),
                            );
                            images::paint_cover(&mut pic, &mut self.tex, &path, rect.shrink(4.0));
                            ui.painter().text(
                                rect.left_bottom() + Vec2::new(8.0, -6.0),
                                egui::Align2::LEFT_BOTTOM,
                                name,
                                egui::FontId::new(11.0, theme::ui_font()),
                                if on { theme::chrome().acid } else { CREAM },
                            );
                            if resp.clicked() {
                                self.place = id.clone();
                                self.broadcast_mix();
                                if !self.panel_on(Overlay::Place) {
                                    self.toggle_overlay(Overlay::Place);
                                }
                            }
                        }
                    });
                }
            });
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 110)),
        );
    }

    fn ui_bj_body(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
            let _ = pal;
            let felt = ui.available_rect_before_wrap();
            ui.painter().rect_filled(felt, 0.0, Color32::from_rgb(6, 16, 18));
            ui.painter().rect_stroke(
                felt.shrink(6.0),
                0.0,
                egui::Stroke::new(1.5, theme::ACID),
                egui::StrokeKind::Inside,
            );
            let ch_pit = theme::chrome();
            ui.label(
                RichText::new("THE PIT")
                    .family(theme::display())
                    .size(26.0)
                    .color(ch_pit.acid),
            );
            ui.label(
                RichText::new(&self.bj.msg)
                    .family(theme::mono())
                    .size(14.0)
                    .color(ch_pit.cyan),
            );
            ui.label(
                RichText::new(format!("CHIPS {}    BET {}", self.bj.bank, self.bj.bet))
                    .color(CREAM)
                    .size(14.0)
                    .family(theme::mono()),
            );
            ui.add_space(8.0);
            ui.label(RichText::new("DEALER").family(theme::mono()).size(12.0).color(DIM));
            ui.horizontal(|ui| {
                if self.bj.dealer.is_empty() {
                    paint_card(ui, 0, true, Vec2::new(72.0, 100.0));
                    paint_card(ui, 0, true, Vec2::new(72.0, 100.0));
                } else {
                    for (i, &c) in self.bj.dealer.iter().enumerate() {
                        let hole = self.bj.live && i > 0;
                        paint_card(ui, c, hole, Vec2::new(72.0, 100.0));
                    }
                }
                if !self.bj.live && !self.bj.dealer.is_empty() {
                    ui.label(
                        RichText::new(format!("= {}", total(&self.bj.dealer)))
                            .family(theme::display())
                            .size(22.0)
                            .color(CYAN),
                    );
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new("YOU").family(theme::mono()).size(12.0).color(DIM));
            ui.horizontal(|ui| {
                if self.bj.player.is_empty() {
                    paint_card(ui, 0, true, Vec2::new(72.0, 100.0));
                    paint_card(ui, 0, true, Vec2::new(72.0, 100.0));
                } else {
                    for &c in &self.bj.player {
                        paint_card(ui, c, false, Vec2::new(72.0, 100.0));
                    }
                    ui.label(
                        RichText::new(format!("= {}", total(&self.bj.player)))
                            .family(theme::display())
                            .size(22.0)
                            .color(CYAN),
                    );
                }
            });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Slider::new(&mut self.bj.bet, 10..=200).text("bet"));
                if !self.bj.live && theme::neon_btn(ui, "Deal").clicked() {
                    if self.table_live() && self.net.role == Role::Guest {
                        self.net.send_pit("bj", "deal");
                    } else if self.bj.bet > self.bj.bank {
                        self.bj.msg = "Not enough.".into();
                    } else {
                        self.bj.bank -= self.bj.bet;
                        self.bj.player = vec![card(), card()];
                        self.bj.dealer = vec![card(), card()];
                        let p = total(&self.bj.player);
                        if p == 21 {
                            self.bj.live = false;
                            self.bj.bank += (self.bj.bet as f32 * 2.5) as i32;
                            self.bj.msg = "Blackjack.".into();
                        } else {
                            self.bj.live = true;
                            self.bj.msg = "Hit or stand. Other seats can hit too.".into();
                        }
                        self.publish_bj();
                    }
                }
                if self.bj.live && theme::neon_btn(ui, "Hit").clicked() {
                    if self.table_live() && self.net.role == Role::Guest {
                        self.net.send_pit("bj", "hit");
                    } else {
                    self.bj.player.push(card());
                    if total(&self.bj.player) > 21 {
                        self.bj.live = false;
                        self.bj.msg = "Bust.".into();
                    }
                    self.publish_bj();
                    }
                }
                if self.bj.live && theme::neon_btn(ui, "Stand").clicked() {
                    if self.table_live() && self.net.role == Role::Guest {
                        self.net.send_pit("bj", "stand");
                    } else {
                    while total(&self.bj.dealer) < 17 {
                        self.bj.dealer.push(card());
                    }
                    let p = total(&self.bj.player);
                    let d = total(&self.bj.dealer);
                    self.bj.live = false;
                    if d > 21 || p > d {
                        self.bj.bank += self.bj.bet * 2;
                        self.bj.msg = "You win.".into();
                    } else if p == d {
                        self.bj.bank += self.bj.bet;
                        self.bj.msg = "Push.".into();
                    } else {
                        self.bj.msg = "Dealer.".into();
                    }
                    self.publish_bj();
                    }
                }
            });
            if self.table_live() {
                wrap_text(
                    ui,
                    "The host deals. Every seat can hit or stand. The cards match. Chips stay in the pit.",
                    DIM,
                    11.0,
                );
            }
    }

    fn ui_catalog(&mut self, ui: &mut egui::Ui) {
        let title = match self.page {
            Page::Catalog(n) => n,
            _ => "Catalog",
        };
        theme::page_chrome(
            ui,
            "CATALOG://BOOKS",
            &title.to_uppercase(),
            "Filter · select a row · detail on the right",
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("FILTER")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::chrome().cyan),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.catalog_q)
                    .hint_text("name or blurb…")
                    .desired_width(220.0),
            );
            ui.label(
                RichText::new("SELECT · no action until you pick")
                    .family(theme::mono())
                    .size(10.0)
                    .color(MUTED),
            );
        });
            let q = self.catalog_q.to_lowercase();
            let rows: Vec<(usize, String, String)> = self
                .catalog_rows
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let extra = v
                        .get("blurb")
                        .or_else(|| v.get("text"))
                        .or_else(|| v.get("kind"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    if q.is_empty()
                        || name.to_lowercase().contains(&q)
                        || extra.to_lowercase().contains(&q)
                    {
                        Some((i, name, extra.chars().take(120).collect()))
                    } else {
                        None
                    }
                })
                .collect();
            let col_h = (ui.available_height() - 4.0).max(80.0);
            ui.columns(2, |cols| {
                egui::ScrollArea::vertical().max_height(col_h).show(&mut cols[0], |ui| {
                    for (i, name, extra) in &rows {
                        let on = self.catalog_pick == *i;
                        if theme::catalog_row(ui, name, extra, on).clicked() {
                            self.catalog_pick = *i;
                        }
                    }
                });
                egui::ScrollArea::vertical().max_height(col_h).show(&mut cols[1], |ui| {
                    let row = self.catalog_rows.get(self.catalog_pick).cloned();
                    if let Some(v) = row {
                        if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                            ui.label(
                                RichText::new(name)
                                    .family(theme::display())
                                    .size(22.0)
                                    .color(theme::chrome().acid),
                            );
                        }
                        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
                            let art = catalog_art(&self.root, title, id);
                            if art.is_file() {
                                if images::show_fit(
                                    ui,
                                    &mut self.tex,
                                    &art,
                                    Vec2::new(ui.available_width().min(280.0).max(48.0), 220.0),
                                )
                                .on_hover_text("Click to zoom")
                                .clicked()
                                {
                                    self.zoom_path = Some(art);
                                    self.zoom_key = None;
                                }
                            }
                        }
                        wrap_text(ui, &pretty(&v), CREAM, 14.0);
                    }
                });
            });
    }

    fn ui_chars(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(
            ui,
            "CHARS://SHEET",
            "CHARACTERS",
            "Roster selects · Aim is secondary · Delete is KILL",
        );
        let h = ui.available_height().max(80.0);
        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
        egui::ScrollArea::vertical()
            .id_salt("chars-page")
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width((ui.available_width() - 8.0).max(40.0));
                self.run_sheet(ui);
            });
    }

    fn run_sheet(&mut self, ui: &mut egui::Ui) {
        let sid = self.net.self_id.clone();
        let remote = self
            .viewing
            .as_ref()
            .filter(|id| *id != &sid)
            .cloned();
        if let Some(id) = remote {
            wrap_text(
                ui,
                "Read-only seat. Their live sheet from the table.",
                MUTED,
                12.0,
            );
            let mut chars = self.remote_chars.remove(&id).unwrap_or_default();
            if chars.is_empty() {
                wrap_text(ui, "Waiting for their sheet on the wire…", DIM, 13.0);
                self.remote_chars.insert(id, chars);
                return;
            }
            let mut i = 0;
            let before = self.combat_log.len();
            let mut remote_sync = false;
            crate::chars::ui_sheet(
                ui,
                &self.root,
                &mut self.tex,
                &mut chars,
                &mut i,
                self.blight,
                &mut self.dice,
                &mut self.combat_log,
                false,
                "",
                &mut self.aim,
                &mut self.luck,
                &mut self.roll,
                &self.names,
                &mut self.zoom_path,
                &mut None,
                &mut remote_sync,
                &mut self.kill_credit,
            );
            self.remote_chars.insert(id, chars);
            if self.zoom_path.is_some() {
                self.zoom_key = None;
            }
            if self.combat_log.len() > before {
                save_combat_log(&self.root, &self.combat_log);
            }
            return;
        }
        let before = self.combat_log.len();
        let oid = self.net.self_id.clone();
        // Claim only the open sheet when still unowned — do not stamp the whole roster.
        let claim_i = self.char_i;
        if let Some(c) = self.chars.get_mut(claim_i) {
            if c.owner.is_empty() && !oid.is_empty() {
                c.owner = oid.clone();
            }
        }
        let sheet_before = sheet_sig(&self.chars);
        let mut sheet_send = None;
        let mut sheet_sync = false;
        crate::chars::ui_sheet(
            ui,
            &self.root,
            &mut self.tex,
            &mut self.chars,
            &mut self.char_i,
            self.blight,
            &mut self.dice,
            &mut self.combat_log,
            self.is_gm,
            &oid,
            &mut self.aim,
            &mut self.luck,
            &mut self.roll,
            &self.names,
            &mut self.zoom_path,
            &mut sheet_send,
            &mut sheet_sync,
            &mut self.kill_credit,
        );
        if let Some(body) = sheet_send {
            let label = self
                .chars
                .get(self.char_i)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Sheet".into());
            self.open_share("sheet", &label, body);
        }
        if self.zoom_path.is_some() {
            self.zoom_key = None;
        }
        if self.combat_log.len() > before {
            let fresh: Vec<String> = self.combat_log[before..].to_vec();
            for line in &fresh {
                self.net.send_chat(&format!("{COMBAT_MARK}{line}"), None, false);
            }
            save_combat_log(&self.root, &self.combat_log);
            self.push_sheets();
        }
        self.apply_roll_hp();
        if sheet_sync {
            self.force_sheets();
        } else if sheet_sig(&self.chars) != sheet_before {
            self.mark_sheets();
        }
    }

    fn force_sheets(&mut self) {
        self.sheet_dirty = false;
        crate::chars::save(&self.root, &self.chars);
        self.push_sheets();
    }

    fn push_sheets(&self) {
        if !self.table_live() {
            return;
        }
        let mut chars = self.chars.clone();
        for c in &mut chars {
            c.portrait = images::portable_rel(&self.root, &c.portrait);
            c.fullbody = images::portable_rel(&self.root, &c.fullbody);
        }
        self.net.send_sheet(chars);
    }

    fn apply_roll_hp(&mut self) {
        let Some(r) = self.roll.as_mut() else {
            return;
        };
        if !r.done() || r.applied {
            return;
        }
        r.applied = true;
        let Some(amt) = r.damage else {
            return;
        };
        let attacker = self
            .chars
            .get(self.char_i)
            .map(|c| c.id.clone())
            .unwrap_or_default();
        let killer_id = if r.attacker.is_empty() {
            attacker.clone()
        } else {
            r.attacker.clone()
        };
        let killer_name = self
            .chars
            .iter()
            .find(|x| x.id == killer_id || x.name == killer_id)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| killer_id.clone());
        let tid = if r.taken {
            attacker
        } else if r.target.is_empty() {
            return;
        } else {
            r.target.clone()
        };
        let mut credit = None;
        if let Some(c) = self.chars.iter_mut().find(|c| c.id == tid || c.name == tid) {
            if r.heal && !r.taken {
                c.hp = (c.hp + amt).min(c.hp_max);
                if c.hp > 0 {
                    c.downed = false;
                    c.death_ok = 0;
                    c.death_fail = 0;
                }
            } else if c.downed && !c.dead {
                c.death_fail = (c.death_fail + 1).min(3);
                if c.death_fail >= 3 {
                    c.dead = true;
                    c.downed = false;
                }
            } else {
                let mut left = amt;
                let soak = c.hp_temp.min(left);
                c.hp_temp -= soak;
                left -= soak;
                c.hp = (c.hp - left).max(0);
                if c.hp <= 0 && !c.dead {
                    if c.npc {
                        credit = crate::chars::mark_defeated(c, &killer_id, &killer_name);
                    } else {
                        c.downed = true;
                    }
                }
            }
        }
        if self.kill_credit.is_none() {
            if let Some(draft) = credit {
                self.kill_credit = Some(draft);
            }
        }
        crate::chars::save(&self.root, &self.chars);
    }

    fn ui_dice_fx(&mut self, ui: &mut egui::Ui) {
        self.apply_roll_hp();
        let Some(r) = &self.roll else {
            return;
        };
        let t = r.start.elapsed().as_secs_f32();
        if t > 2.4 {
            return;
        }
        let rect = ui.max_rect();
        let size = Vec2::new(
            260.0_f32.min((rect.width() - 24.0).max(48.0)),
            128.0_f32.min((rect.height() - 24.0).max(48.0)),
        );
        let pad = Vec2::new(12.0, 12.0);
        let boxr = Rect::from_min_size(
            egui::pos2(
                (rect.right() - size.x - pad.x).max(rect.left() + 4.0),
                (rect.bottom() - size.y - pad.y).max(rect.top() + 4.0),
            ),
            size,
        )
        .intersect(rect);
        ui.painter().rect_filled(
            boxr,
            8.0,
            Color32::from_rgba_unmultiplied(12, 12, 8, 230),
        );
        ui.painter().rect_stroke(
            boxr,
            8.0,
            egui::Stroke::new(2.0, CYAN),
            egui::StrokeKind::Inside,
        );
        let shown = r.display_pct();
        let color = match r.grade {
            "critical success" if r.done() => CYAN,
            "critical failure" if r.done() => KILL,
            _ => CYAN,
        };
        let paint = ui.painter().with_clip_rect(boxr.shrink(6.0));
        let num_size = if boxr.width() < 180.0 { 32.0 } else { 56.0 };
        paint.text(
            boxr.center() + Vec2::new(0.0, -28.0),
            egui::Align2::CENTER_CENTER,
            format!("{shown}"),
            FontId::new(num_size, theme::display()),
            color,
        );
        let luck_tag = match r.luck {
            Luck::Adv => "ADV",
            Luck::Dis => "DIS",
            Luck::Norm => "",
        };
        if !luck_tag.is_empty() {
            paint.text(
                boxr.center() + Vec2::new(0.0, -52.0),
                egui::Align2::CENTER_CENTER,
                luck_tag,
                FontId::new(12.0, theme::mono()),
                ORANGE,
            );
        }
        if r.done() {
            paint.text(
                boxr.center() + Vec2::new(0.0, 22.0),
                egui::Align2::CENTER_CENTER,
                r.grade.to_uppercase(),
                FontId::new(16.0, theme::mono()),
                color,
            );
            if let Some(d) = r.damage {
                let extra = if r.heal {
                    format!("HEAL {d}")
                } else if r.taken {
                    format!("TAKEN {d}")
                } else {
                    format!("DMG {d}")
                };
                paint.text(
                    boxr.center() + Vec2::new(0.0, 46.0),
                    egui::Align2::CENTER_CENTER,
                    extra,
                    FontId::new(13.0, theme::mono()),
                    CYAN,
                );
            }
        }
        ui.ctx().request_repaint_after(FRAME);
    }

    fn ui_chat_media(&mut self, ui: &mut egui::Ui, line: &str) -> bool {
        let (tag, rest) = if let Some(r) = line.strip_prefix("[img:") {
            ("img", r)
        } else if let Some(r) = line.strip_prefix("[aud:") {
            ("aud", r)
        } else if let Some(r) = line.strip_prefix("[vid:") {
            ("vid", r)
        } else if let Some(r) = line.strip_prefix("[file:") {
            ("file", r)
        } else {
            return false;
        };
        let Some((keypart, caption)) = rest.split_once(']') else {
            return false;
        };
        let (key, filename) = match keypart.split_once('|') {
            Some((k, f)) => (k.to_string(), f.to_string()),
            None => (keypart.to_string(), String::new()),
        };
        let caption = caption.trim().to_string();
        let bytes = self.inbox_bytes(&key);
        match tag {
            "img" => {
                if let Some(bytes) = &bytes {
                    if images::show_bytes(
                        ui,
                        &mut self.tex,
                        &key,
                        bytes,
                        Vec2::new(ui.available_width().min(280.0), 140.0),
                    )
                    .on_hover_text("Click to zoom")
                    .clicked()
                    {
                        self.zoom_key = Some(key.clone());
                        self.zoom_path = None;
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    wrap_text(ui, &caption, CYAN, 12.0);
                    if theme::neon_btn(ui, "Download").clicked() {
                        self.download_inbox(&key);
                    }
                });
            }
            "aud" => {
                ui.horizontal_wrapped(|ui| {
                    if theme::neon_btn(ui, "Play").clicked() {
                        if let Some(b) = &bytes {
                            self.mixer.play_bytes(b);
                        }
                    }
                    if theme::neon_btn(ui, "Download").clicked() {
                        self.download_inbox(&key);
                    }
                    wrap_text(ui, &caption, CYAN, 12.0);
                });
            }
            "vid" => {
                ui.horizontal_wrapped(|ui| {
                    if theme::neon_btn(ui, "Open video").clicked() {
                        if let Some(b) = &bytes {
                            open_media(b, sniff_ext(b));
                        }
                    }
                    if theme::neon_btn(ui, "Download").clicked() {
                        self.download_inbox(&key);
                    }
                    wrap_text(ui, &caption, CYAN, 12.0);
                });
            }
            "file" => {
                ui.horizontal_wrapped(|ui| {
                    wrap_text(
                        ui,
                        if filename.is_empty() {
                            &caption
                        } else {
                            &filename
                        },
                        CYAN,
                        12.0,
                    );
                    if theme::neon_btn(ui, "Download").clicked() {
                        self.download_inbox(&key);
                    }
                    wrap_text(ui, &caption, MUTED, 11.0);
                });
            }
            _ => return false,
        }
        true
    }

    fn ui_zoom(&mut self, ctx: &egui::Context) {
        if self.zoom_path.is_none() && self.zoom_key.is_none() {
            return;
        }
        let mut dismiss = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::Area::new(egui::Id::new("zoom-view"))
            .order(egui::Order::Foreground)
            .fixed_pos(ctx.screen_rect().min)
            .show(ctx, |ui| {
                let r = ctx.screen_rect();
                let resp = ui.allocate_rect(r, egui::Sense::click());
                ui.painter().rect_filled(
                    r,
                    0.0,
                    Color32::from_rgba_unmultiplied(8, 8, 6, 236),
                );
                let frame = r.shrink(18.0);
                ui.painter().rect_stroke(
                    frame,
                    0.0,
                    egui::Stroke::new(2.0, CYAN),
                    egui::StrokeKind::Inside,
                );
                theme::brackets(ui, frame, CYAN, 16.0);
                ui.painter().text(
                    frame.left_top() + Vec2::new(16.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    "ZOOM  ·  CLICK TO CLOSE",
                    FontId::new(12.0, theme::mono()),
                    CYAN,
                );
                let inner = frame.shrink2(Vec2::new(16.0, 36.0));
                let tex = if let Some(p) = &self.zoom_path {
                    self.tex.get(ui.ctx(), p)
                } else if let Some(k) = &self.zoom_key {
                    self.inbox_bytes(k)
                        .and_then(|b| self.tex.from_bytes(ui.ctx(), k, &b))
                } else {
                    None
                };
                if let Some(tex) = tex {
                    let sz = tex.size_vec2();
                    if sz.x > 0.0 && sz.y > 0.0 {
                        let scale = (inner.width() / sz.x).min(inner.height() / sz.y);
                        let dest = Rect::from_center_size(inner.center(), sz * scale);
                        ui.painter().image(
                            tex.id(),
                            dest,
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
                if resp.clicked() {
                    dismiss = true;
                }
            });
        if dismiss {
            self.zoom_path = None;
            self.zoom_key = None;
        }
    }

    fn ui_nethooks(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(
            ui,
            "NETHOOKS://GRID",
            "NETHOOKS",
            "Write a page · Post to table · Board on TABLE rail",
        );
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "← INDEX").clicked() {
                self.page = Page::Index;
                self.hook_edit = false;
            }
            if theme::neon_btn(ui, "TABLE").clicked() {
                self.page = Page::Table;
                self.hook_edit = false;
            }
            if theme::neon_btn(ui, "Handout").clicked() {
                self.add_starter("handout");
            }
            if theme::neon_btn(ui, "Rumor").clicked() {
                self.add_starter("rumor");
            }
            if theme::neon_btn(ui, "Job").clicked() {
                self.add_starter("job");
            }
            if theme::neon_btn(ui, "Recap").clicked() {
                self.add_starter("recap");
            }
            if theme::neon_btn(ui, "+ New nethook").clicked() {
                let h = crate::nethook::Nethook::fresh(&self.net.self_id, &self.handle);
                self.hook_draft_title = h.title.clone();
                self.hook_draft_html = h.html.clone();
                let at = if self
                    .nethooks
                    .first()
                    .map(|x| x.pinned())
                    .unwrap_or(false)
                {
                    1
                } else {
                    0
                };
                self.nethooks.insert(at, h);
                self.hook_i = at;
                self.hook_edit = true;
            }
        });
        ui.add_space(4.0);
        let rest = ui.available_rect_before_wrap();
        let _ = ui.allocate_rect(rest, egui::Sense::hover());
        let split = rest.left() + rest.width() * 0.25;
        let (left, right) = rest.split_left_right_at_x(split);
        ui.painter().vline(
            split,
            rest.y_range(),
            egui::Stroke::new(1.5, CYAN),
        );
        let mut list_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left.shrink2(Vec2::new(8.0, 4.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let mut page_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right.shrink2(Vec2::new(10.0, 4.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let list_h = list_ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
                .id_salt("nethook-list")
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(&mut list_ui, |ui| {
                    ui.label(
                        RichText::new("DIRECTORY")
                            .family(theme::mono())
                            .size(11.0)
                            .color(CYAN),
                    );
                    if self.nethooks.is_empty() {
                        wrap_text(ui, "No nethooks on this deck yet. Write one.", MUTED, 13.0);
                    }
                    let mine = self.net.self_id.clone();
                    for i in 0..self.nethooks.len() {
                        let title = self.nethooks[i].title.clone();
                        let owner = self.nethooks[i].owner_name.clone();
                        let pinned = self.nethooks[i].pinned();
                        let yours = self.nethooks[i].owner_id == mine && !pinned;
                        let on = self.hook_i == i;
                        let kind = self.nethooks[i].kind.clone();
                        let posted = self.nethooks[i].posted;
                        let extra = if pinned {
                            "LESSON · PINNED".into()
                        } else if posted {
                            format!("{kind} · ON TABLE")
                        } else if yours {
                            format!("{kind} · {owner}")
                        } else {
                            format!("{kind} · {owner}")
                        };
                        if theme::wide_btn(ui, &title, &extra, on).clicked() {
                            self.hook_i = i;
                            self.hook_edit = false;
                        }
                    }
                });
        if self.hook_edit {
            self.ui_nethook_page(&mut page_ui);
        } else {
            let page_h = page_ui.available_height().max(40.0);
            egui::ScrollArea::vertical()
                .id_salt("nethook-page")
                .max_height(page_h)
                .auto_shrink([false, false])
                .show(&mut page_ui, |ui| {
                    self.ui_nethook_page(ui);
                });
        }
    }

    fn ui_recon(&mut self, ui: &mut egui::Ui) {
        let mut sent = None;
        let changed = crate::recon::paint(
            ui,
            &self.root,
            &mut self.recon,
            &mut self.recon_i,
            &mut self.recon_q,
            &mut self.recon_arm,
            &mut self.tex,
            &mut self.zoom_path,
            self.blight,
            &mut sent,
        );
        if let Some(file) = sent {
            let label = file.title();
            self.open_share("recon", &label, pack_recon(&self.root, &file));
        }
        if self.recon.is_empty() {
            self.recon_i = 0;
        } else if self.recon_i >= self.recon.len() {
            self.recon_i = self.recon.len() - 1;
        }
        if changed {
            crate::recon::save(&self.root, &self.recon);
        }
    }


    fn netspace_ping_ms(&self) -> Option<u128> {
        if !self.node_live {
            return None;
        }
        let mut sum = 0u128;
        let mut n = 0u32;
        for (id, ms) in &self.ping_ms {
            let fresh = self
                .ping_at
                .get(id)
                .map(|t| t.elapsed() < Duration::from_secs(6))
                .unwrap_or(false);
            if fresh {
                sum += *ms;
                n += 1;
            }
        }
        if n == 0 {
            None
        } else {
            Some(sum / u128::from(n))
        }
    }

    fn ui_tree(&mut self, ui: &mut egui::Ui) {
        if !self.tree_loaded {
            self.tree_loaded = true;
            self.tree_reload();
        }
        self.poll_tree_tags();
        if self.tree_log_at.elapsed() >= Duration::from_secs(2) {
            self.tree_log_at = Instant::now();
            self.tree_log_tail = crate::tree::tail_daemon_log(&self.root, 24);
        }
        let root = self.root.clone();
        let at = self.tree_at.clone();
        let drives = self.tree_drives;
        let rows = self.tree_rows.clone();
        let sel = self.tree_sel.clone();
        let arm = self.tree_arm.clone();
        let deep = self.tree_deep;
        let err = self.tree_err.clone();
        let role = match self.net.role {
            Role::Host => "HOST",
            Role::Guest => "GUEST",
            Role::Presence => "ONLINE",
            Role::Idle => "LOCAL",
        };
        let invite = if matches!(self.net.role, Role::Host) {
            self.net.paste_link()
        } else {
            String::new()
        };
        let pulse = crate::tree::DeckPulse {
            cpu: self.machine.cpu,
            ram_used: self.machine.ram_used,
            ram_total: self.machine.ram_total,
            disk_free: self.machine.disk_free,
            disk_total: self.machine.disk_total,
            node_up: self.node_live,
            role: role.into(),
            peers: self.net.peers.len(),
            invite,
            log_tail: self.tree_log_tail.clone(),
        };
        let tags_note = self.tree_tags_note.clone();
        let tags_owned = if self
            .tree_tags_for
            .as_ref()
            .zip(sel.as_ref())
            .is_some_and(|(a, b)| a == b)
        {
            Some(self.tree_tags.clone())
        } else {
            None
        };
        let acts = crate::tree::paint(
            ui,
            &mut self.tree_chrome,
            &root,
            &at,
            drives,
            &rows,
            sel.as_deref(),
            arm.as_deref(),
            deep,
            &mut self.tree_name,
            &err,
            &pulse,
            tags_owned.as_ref(),
            &tags_note,
        );
        for act in acts {
            match act {
                crate::tree::Act::Home => {
                    if let Some(home) = crate::tree::home_dir() {
                        self.tree_enter(home);
                    } else {
                        self.tree_err = "Home is not on this computer.".into();
                    }
                }
                crate::tree::Act::Computer => {
                    let roots = crate::tree::computer_roots();
                    if roots.len() == 1 {
                        self.tree_enter(roots[0].clone());
                    } else {
                        self.tree_drives = true;
                        self.tree_sel = None;
                        self.tree_arm = None;
                        self.tree_reload();
                    }
                }
                crate::tree::Act::Up => self.tree_up(),
                crate::tree::Act::Refresh => self.tree_reload(),
                crate::tree::Act::NewFile => self.tree_make(false),
                crate::tree::Act::NewFolder => self.tree_make(true),
                crate::tree::Act::Open(path) => self.tree_open(path),
                crate::tree::Act::Enter(path) | crate::tree::Act::Jump(path) | crate::tree::Act::Bookmark(path) => {
                    if path.is_dir() {
                        self.tree_enter(path);
                    } else if path.exists() {
                        self.tree_err = "That is not a folder.".into();
                    } else {
                        self.tree_err = "That path is not there.".into();
                    }
                }
                crate::tree::Act::Select(path) => {
                    if self.tree_sel.as_ref() != Some(&path) {
                        self.tree_arm = None;
                    }
                    self.tree_sel = Some(path.clone());
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        self.tree_chrome.rename_to = name.to_string();
                    }
                    if path.is_file() && crate::tree::is_media_path(&path) {
                        self.queue_tree_tags(path);
                    } else {
                        self.tree_tags_for = None;
                        self.tree_tags_note.clear();
                    }
                }
                crate::tree::Act::Delete => self.tree_delete(),
                crate::tree::Act::CopyPath(path) => {
                    ui.ctx().copy_text(path.display().to_string());
                    self.tree_err = "Path copied.".into();
                }
                crate::tree::Act::CopyName(path) => {
                    let name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    ui.ctx().copy_text(name);
                    self.tree_err = "Name copied.".into();
                }
                crate::tree::Act::TogglePin(path) => {
                    if !self.tree_chrome.pins.remove(&path) {
                        self.tree_chrome.pins.insert(path);
                    }
                }
                crate::tree::Act::Rename => {
                    let Some(sel) = self.tree_sel.clone() else {
                        self.tree_err = "Choose a file first.".into();
                        continue;
                    };
                    match crate::tree::rename_entry(&sel, &self.tree_chrome.rename_to) {
                        Ok(dest) => {
                            self.tree_sel = Some(dest);
                            self.tree_arm = None;
                            self.tree_err.clear();
                            self.tree_reload();
                        }
                        Err(err) => self.tree_err = err,
                    }
                }
                crate::tree::Act::CopyHere => {
                    let Some(src) = self.tree_chrome.clip.clone() else {
                        self.tree_err = "Clip a file first.".into();
                        continue;
                    };
                    if self.tree_drives {
                        self.tree_err = "Open a folder first.".into();
                        continue;
                    }
                    match crate::tree::copy_entry(&src, &self.tree_at) {
                        Ok(_) => {
                            self.tree_err.clear();
                            self.tree_reload();
                        }
                        Err(err) => self.tree_err = err,
                    }
                }
                crate::tree::Act::MoveHere => {
                    let Some(src) = self.tree_chrome.clip.clone() else {
                        self.tree_err = "Clip a file first.".into();
                        continue;
                    };
                    if self.tree_drives {
                        self.tree_err = "Open a folder first.".into();
                        continue;
                    }
                    match crate::tree::move_entry(&src, &self.tree_at) {
                        Ok(dest) => {
                            self.tree_chrome.clip = None;
                            self.tree_sel = Some(dest);
                            self.tree_arm = None;
                            self.tree_err.clear();
                            self.tree_reload();
                        }
                        Err(err) => self.tree_err = err,
                    }
                }
                crate::tree::Act::OpenText(path) => {
                    self.load_text(&path);
                    self.shell = ShellPanel::Player;
                    self.add_deck_paths(vec![path.clone()]);
                    if let Some(i) = self.deck_list.iter().position(|p| p == &path) {
                        self.deck_i = i;
                    }
                }
                crate::tree::Act::OpenTerminal => {
                    let cwd = if self.tree_drives {
                        self.root.clone()
                    } else {
                        self.tree_at.clone()
                    };
                    self.term_cwd = Some(cwd);
                    self.term = None;
                    self.page = Page::Terminal;
                }
                crate::tree::Act::ToggleConsole => {
                    self.tree_chrome.console_open = !self.tree_chrome.console_open;
                    if self.tree_chrome.console_open {
                        self.tree_log_tail = crate::tree::tail_daemon_log(&self.root, 24);
                        self.tree_log_at = Instant::now();
                    }
                }
                crate::tree::Act::SendFile => {
                    let Some(path) = self.tree_sel.clone() else {
                        self.tree_err = "Choose a file first.".into();
                        continue;
                    };
                    if !path.is_file() {
                        self.tree_err = "Choose a file first.".into();
                        continue;
                    }
                    if !self.node_live {
                        self.tree_err = "Press Online first. Then you can send it.".into();
                        continue;
                    }
                    self.send_path = Some(path);
                    self.send_ids.clear();
                }
                crate::tree::Act::ReconPortrait => {
                    let Some(path) = self.tree_sel.clone() else {
                        self.tree_err = "Choose an image first.".into();
                        continue;
                    };
                    if !crate::tree::is_image_path(&path) {
                        self.tree_err = "Choose an image first.".into();
                        continue;
                    }
                    if self.recon.is_empty() {
                        self.tree_err = "Open RECON and make a file first.".into();
                        continue;
                    }
                    let i = self.recon_i.min(self.recon.len() - 1);
                    let id = self.recon[i].id.clone();
                    match crate::recon::store_picture(&self.root, &id, &path) {
                        Ok(rel) => {
                            self.recon[i].portrait = rel;
                            crate::recon::save(&self.root, &self.recon);
                            self.tree_err = "Portrait set on RECON file.".into();
                        }
                        Err(err) => self.tree_err = err,
                    }
                }
                crate::tree::Act::ProbeTags => {
                    let Some(path) = self.tree_sel.clone() else {
                        self.tree_err = "Choose a media file first.".into();
                        continue;
                    };
                    if !crate::tree::is_media_path(&path) {
                        self.tree_err = "That file has no media tags.".into();
                        continue;
                    }
                    self.queue_tree_tags(path);
                }
            }
        }
    }

    fn queue_tree_tags(&mut self, path: PathBuf) {
        if self.tree_tags_for.as_ref() == Some(&path) && self.tree_tags_rx.is_none() {
            return;
        }
        self.tree_tags_gen = self.tree_tags_gen.wrapping_add(1);
        let gen = self.tree_tags_gen;
        let (tx, rx) = std::sync::mpsc::channel();
        self.tree_tags_rx = Some(rx);
        self.tree_tags_for = Some(path.clone());
        self.tree_tags_note = "Reading tags…".into();
        std::thread::spawn(move || {
            let result = crate::deskfile::read_tags(&path);
            let _ = tx.send((gen, result));
        });
    }

    fn poll_tree_tags(&mut self) {
        let Some(rx) = self.tree_tags_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok((gen, Ok(tags))) => {
                if gen == self.tree_tags_gen {
                    self.tree_tags = tags;
                    self.tree_tags_note.clear();
                }
            }
            Ok((gen, Err(err))) => {
                if gen == self.tree_tags_gen {
                    self.tree_tags_note = err;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.tree_tags_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }

    fn tree_reload(&mut self) {
        if self.tree_drives {
            self.tree_rows = crate::tree::computer_roots()
                .into_iter()
                .map(|path| crate::tree::Entry {
                    name: path.display().to_string(),
                    path,
                    dir: true,
                    bytes: 0,
                })
                .collect();
            self.tree_err.clear();
            return;
        }
        match crate::tree::list_dir(&self.tree_at) {
            Ok(rows) => {
                self.tree_rows = rows;
                self.tree_err.clear();
            }
            Err(err) => {
                self.tree_rows.clear();
                self.tree_err = err;
            }
        }
    }

    fn tree_enter(&mut self, path: PathBuf) {
        self.tree_drives = false;
        self.tree_at = path;
        self.tree_sel = None;
        self.tree_arm = None;
        self.tree_deep = false;
        self.tree_reload();
    }

    fn tree_up(&mut self) {
        if self.tree_drives {
            return;
        }
        if crate::tree::is_fs_root(&self.tree_at) {
            let roots = crate::tree::computer_roots();
            if roots.len() > 1 {
                self.tree_drives = true;
                self.tree_sel = None;
                self.tree_arm = None;
                self.tree_reload();
            }
            return;
        }
        if let Some(parent) = self.tree_at.parent() {
            if parent.as_os_str().is_empty() {
                return;
            }
            self.tree_enter(parent.to_path_buf());
        }
    }

    fn tree_make(&mut self, folder: bool) {
        if self.tree_drives {
            self.tree_err = "Open a folder first.".into();
            return;
        }
        let made = if folder {
            crate::tree::create_dir(&self.tree_at, &self.tree_name)
        } else {
            crate::tree::create_file(&self.tree_at, &self.tree_name)
        };
        match made {
            Ok(_) => {
                self.tree_name.clear();
                self.tree_err.clear();
                self.tree_reload();
            }
            Err(err) => self.tree_err = err,
        }
    }

    fn tree_delete(&mut self) {
        let Some(sel) = self.tree_sel.clone() else {
            self.tree_err = "Choose a file first.".into();
            return;
        };
        if self.tree_arm.as_ref() != Some(&sel) {
            self.tree_arm = Some(sel.clone());
            let link = std::fs::symlink_metadata(&sel)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            self.tree_deep = !link && sel.is_dir() && crate::tree::dir_has_child(&sel);
            self.tree_err = if self.tree_deep {
                "This folder is not empty. Press Delete again to remove it and everything inside.".into()
            } else {
                "Press Delete again to remove this.".into()
            };
            return;
        }
        match crate::tree::remove_path(&sel, &self.root) {
            Ok(()) => {
                self.tree_sel = None;
                self.tree_arm = None;
                self.tree_deep = false;
                self.tree_err.clear();
                self.tree_reload();
            }
            Err(err) => {
                self.tree_arm = None;
                self.tree_err = err;
            }
        }
    }

    fn tree_open(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.tree_enter(path);
            return;
        }
        if is_library_file(&path) {
            self.add_deck_paths(vec![path.clone()]);
            if let Some(i) = self.deck_list.iter().position(|p| p == &path) {
                self.deck_i = i;
                self.deck_base = 0.0;
                self.media_offset = 0.0;
                self.seek_drag = None;
                self.media_page = 1;
                self.deck_note.clear();
                self.deck_play_current();
                self.shell = ShellPanel::Player;
            }
            return;
        }
        if !crate::sys::open_path(&path) {
            self.tree_err = "This computer did not open that file.".into();
        }
    }

    fn ui_terminal(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
        theme::page_chrome(
            ui,
            "TERM://LOCAL",
            "TERMINAL",
            "Type or click a command · nothing leaves this deck",
        );
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, egui::Sense::hover());
        let side_w = 240.0_f32.min(rect.width() * 0.32).max(140.0);
        let (main, side) = rect.split_left_right_at_x(rect.right() - side_w);
        let main = main.shrink2(Vec2::new(6.0, 4.0));
        let side = side.shrink2(Vec2::new(8.0, 4.0));
        ui.painter().vline(side.left(), rect.y_range(), egui::Stroke::new(1.0, theme::HOT));
        let cw = 8.4_f32;
        let ch = 16.0_f32;
        let cols = (main.width() / cw).floor() as u16;
        let rows = (main.height() / ch).floor() as u16;
        if self.term.is_none() {
            let cwd = self
                .term_cwd
                .clone()
                .filter(|p| p.is_dir())
                .unwrap_or_else(|| self.root.clone());
            self.term = Some(crate::term::Shell::spawn_in(cols, rows, &cwd));
            self.term_cmds = crate::term::list_commands();
        }
        if let Some(shell) = self.term.as_mut() {
            shell.resize(cols, rows);
            shell.poll();
            if !shell.err.is_empty() {
                wrap_text(ui, &shell.err, KILL, 13.0);
            }
            shell.paint(ui, main);
            let id = egui::Id::new("term-grid");
            let resp = ui.interact(main, id, egui::Sense::click());
            if resp.clicked() {
                resp.request_focus();
            }
            if resp.hovered() {
                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                if dy.abs() > 1.0 {
                    shell.scroll_by(if dy > 0.0 { 3 } else { -3 });
                }
            }
            if resp.has_focus() {
                let events = ui.input(|i| i.events.clone());
                for event in &events {
                    crate::term::handle_key(shell, event);
                }
            }
        }
        let mut side_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(side)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        side_ui.set_clip_rect(side);
        side_ui.label(
            RichText::new("COMMANDS")
                .family(theme::mono())
                .size(11.0)
                .color(theme::ACID),
        );
        wrap_text(
            &mut side_ui,
            "Every command on this computer. Click types it. Enter runs it. Nothing here is sent to the table.",
            MUTED,
            11.0,
        );
        side_ui.add(
            egui::TextEdit::singleline(&mut self.term_filter)
                .hint_text("Filter…")
                .desired_width(side_ui.available_width()),
        );
        if theme::neon_btn(&mut side_ui, "Rescan").clicked() {
            self.term_cmds = crate::term::list_commands();
        }
        let q = self.term_filter.to_lowercase();
        let shown: Vec<usize> = self
            .term_cmds
            .iter()
            .enumerate()
            .filter(|(_, c)| q.is_empty() || c.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("term-cmds")
            .auto_shrink([false, false])
            .show_rows(&mut side_ui, 36.0, shown.len(), |ui, range| {
                for i in range {
                    let name = self.term_cmds[shown[i]].clone();
                    if theme::wide_btn(ui, &name, "", false).clicked() {
                        if let Some(shell) = self.term.as_mut() {
                            shell.write_str(&format!("{name} "));
                        }
                    }
                }
            });
    }

    fn ui_rotn(&mut self, ui: &mut egui::Ui) {
        let desk = crate::rotn::Desk {
            world: if self.blight { "Blight".into() } else { "Hearthsong".into() },
            place: self.place_name(),
            inside: self.inside,
            period: self.time.to_string(),
            date: format!("{:04}-{:02}-{:02}", self.cal_y, self.cal_m, self.cal_d),
            clock: format_clock(self.clock),
            scene: if self.scene.is_empty() {
                "none".into()
            } else {
                self.scene.clone()
            },
            seats: self
                .net
                .peers
                .iter()
                .map(|p| {
                    if p.name.trim().is_empty() {
                        p.id.clone()
                    } else {
                        p.name.clone()
                    }
                })
                .collect(),
            sheets: self
                .chars
                .iter()
                .map(|c| format!("{} {}/{}", c.name, c.hp, c.hp_max))
                .collect(),
        };
        let root = self.root.clone();
        crate::rotn::paint(ui, &root, &mut self.rotn, &desk);
        if let Some((kind, title, body)) = self.rotn.post.take() {
            self.spawn_hook(&kind, &title, &body);
        }
    }

    fn spawn_hook(&mut self, kind: &str, title: &str, body: &str) {
        let h = crate::nethook::Nethook::from_text(
            kind,
            title,
            body,
            &self.net.self_id,
            &self.handle,
        );
        let id = h.id.clone();
        crate::nethook::save_one(&self.root, &h);
        if self.live_link() {
            self.send_hook(&h);
        }
        let at = self
            .nethooks
            .iter()
            .position(|x| !x.pinned())
            .unwrap_or(self.nethooks.len());
        self.nethooks.insert(at, h);
        self.hook_i = self.nethooks.iter().position(|x| x.id == id).unwrap_or(at);
        self.hook_edit = false;
        self.page = Page::Nethooks;
        self.chat.push("Posted from the fixer. It is yours. Press Post to table when the table should read it.".into());
    }

    fn ui_netspace(&mut self, ui: &mut egui::Ui) {
        theme::page_chrome(
            ui,
            "NETSPACE://GRID",
            "NETSPACE",
            "Click look · WASD move · seats when Online",
        );
        self.netspace.apply_table_seed(&self.net.table_key);
        let ping = self.netspace_ping_ms();
        let t = self.jack_at.elapsed().as_secs_f32();
        crate::netspace::paint(ui, &mut self.netspace, t, true, ping);
    }

    fn stop_hook_video(&mut self) {
        if let Some(mut child) = self.hook_vid.take() {
            let _ = child.kill();
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }

    fn toggle_hook_audio(&mut self, path: &Path) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if self.hook_sound == name {
            self.mixer.stop("__hook");
            self.hook_sound.clear();
            return;
        }
        if self.mixer.play_once("__hook", path, 0.7).is_ok() {
            self.hook_sound = name;
        }
    }

    fn toggle_hook_video(&mut self, path: &Path) {
        self.stop_hook_video();
        let Some(bin) = crate::sys::ffmpeg_bin() else {
            self.chat.push("FFmpeg is missing. Open the video outside.".into());
            return;
        };
        let dest = crate::nethook::video_frame(path);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        self.tex.forget(dest.to_string_lossy().as_ref());
        let mut cmd = std::process::Command::new(bin);
        crate::sys::hide(&mut cmd);
        cmd.args(["-y", "-re", "-i"])
            .arg(path)
            .arg("-an")
            .args(["-vf", "fps=4,scale=960:-2"])
            .args(["-f", "image2", "-update", "1"])
            .arg(&dest)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Ok(child) = cmd.spawn() {
            self.hook_vid = Some(child);
        }
    }

    fn paint_hook(&mut self, ui: &mut egui::Ui, id: &str, html: &str) {
        let playing = self.hook_sound.clone();
        let mut face = crate::nethook::HookFace {
            root: &self.root,
            hook_id: id,
            tex: &mut self.tex,
            playing: &playing,
        };
        let acts = crate::nethook::paint(ui, html, Some(&mut face));
        drop(face);
        for act in acts {
            match act {
                crate::nethook::HookAct::Audio(p) => self.toggle_hook_audio(&p),
                crate::nethook::HookAct::Open(p) => {
                    if !crate::sys::open_path(&p) {
                        self.chat.push("This computer did not open that file.".into());
                    }
                }
                crate::nethook::HookAct::Video(p) => self.toggle_hook_video(&p),
            }
        }
    }

    fn attach_hook_file(&mut self) {
        let Some(id) = self.nethooks.get(self.hook_i).map(|h| h.id.clone()) else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Media",
                &["png", "jpg", "jpeg", "webp", "ogg", "mp3", "wav", "flac", "opus", "mp4", "webm", "mkv"],
            )
            .set_title("Attach a picture, sound, or video")
            .pick_file()
        else {
            return;
        };
        match crate::nethook::store_attachment(&self.root, &id, &path) {
            Ok(file) => {
                let tag = crate::nethook::tag_for(&file);
                if let Some(h) = self.nethooks.get_mut(self.hook_i) {
                    if !h.files.iter().any(|x| x.name == file.name) {
                        h.files.push(file);
                    }
                }
                if !self.hook_draft_html.contains(&tag) {
                    self.hook_draft_html.push_str(&tag);
                }
                self.hook_blocks = crate::nethook::html_to_blocks(&self.hook_draft_html);
                self.save_hook_draft();
            }
            Err(e) => self.chat.push(e),
        }
    }

    fn sync_hook_blocks(&mut self) {
        self.hook_draft_html = crate::nethook::blocks_to_html(&self.hook_blocks);
    }

    fn ui_hook_build(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, block) in [
                ("Heading", crate::nethook::Block::Heading { level: 1, text: "Title".into() }),
                ("Text", crate::nethook::Block::Paragraph("Write here.".into())),
                ("List", crate::nethook::Block::List("One item".into())),
                ("Quote", crate::nethook::Block::Quote("A line someone said.".into())),
                ("Rule", crate::nethook::Block::Rule),
                ("Columns", crate::nethook::Block::Columns("Left".into(), "Right".into())),
            ] {
                if theme::neon_btn(ui, label).clicked() {
                    self.hook_blocks.push(block);
                    self.hook_block_i = self.hook_blocks.len() - 1;
                    self.sync_hook_blocks();
                }
            }
            if theme::neon_btn(ui, "Attach").clicked() {
                self.attach_hook_file();
            }
        });
        let names: Vec<String> = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.files.iter().map(|f| format!("{} ({})", f.name, f.kind)).collect())
            .unwrap_or_default();
        if !names.is_empty() {
            wrap_text(ui, &names.join("  ·  "), DIM, 11.0);
        }
        let len = self.hook_blocks.len();
        if len == 0 {
            wrap_text(ui, "Add a heading or a paragraph. The preview updates on the right.", MUTED, 12.0);
            return;
        }
        self.hook_block_i = self.hook_block_i.min(len - 1);
        let labels: Vec<String> = self
            .hook_blocks
            .iter()
            .map(|b| match b {
                crate::nethook::Block::Heading { text, .. } => format!("Heading · {text}"),
                crate::nethook::Block::Paragraph(t) => format!("Text · {t}"),
                crate::nethook::Block::List(_) => "List".into(),
                crate::nethook::Block::Quote(t) => format!("Quote · {t}"),
                crate::nethook::Block::Rule => "Rule".into(),
                crate::nethook::Block::Picture(n) => format!("Picture · {n}"),
                crate::nethook::Block::Sound(n) => format!("Sound · {n}"),
                crate::nethook::Block::Film(n) => format!("Video · {n}"),
                crate::nethook::Block::Columns(_, _) => "Columns".into(),
            })
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("hook-blocks")
            .max_height(120.0)
            .show(ui, |ui| {
                for (i, label) in labels.iter().enumerate() {
                    if theme::wide_btn(ui, label, "", self.hook_block_i == i).clicked() {
                        self.hook_block_i = i;
                    }
                }
            });
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Up").clicked() && self.hook_block_i > 0 {
                let i = self.hook_block_i;
                self.hook_blocks.swap(i, i - 1);
                self.hook_block_i -= 1;
                self.sync_hook_blocks();
            }
            if theme::neon_btn(ui, "Down").clicked() && self.hook_block_i + 1 < self.hook_blocks.len() {
                let i = self.hook_block_i;
                self.hook_blocks.swap(i, i + 1);
                self.hook_block_i += 1;
                self.sync_hook_blocks();
            }
            if theme::neon_btn_color(ui, "Remove", KILL, false).clicked() {
                let i = self.hook_block_i;
                self.hook_blocks.remove(i);
                self.hook_block_i = self.hook_block_i.saturating_sub(1);
                self.sync_hook_blocks();
            }
        });
        let files: Vec<String> = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.files.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        let i = self.hook_block_i.min(self.hook_blocks.len().saturating_sub(1));
        if let Some(block) = self.hook_blocks.get_mut(i) {
            let mut changed = false;
            match block {
                crate::nethook::Block::Heading { level, text } => {
                    ui.horizontal_wrapped(|ui| {
                        if theme::neon_btn_color(ui, "H1", CYAN, *level == 1).clicked() {
                            *level = 1;
                            changed = true;
                        }
                        if theme::neon_btn_color(ui, "H2", CYAN, *level == 2).clicked() {
                            *level = 2;
                            changed = true;
                        }
                        if theme::neon_btn_color(ui, "H3", CYAN, *level >= 3).clicked() {
                            *level = 3;
                            changed = true;
                        }
                    });
                    if ui.add(egui::TextEdit::singleline(text).desired_width(ui.available_width())).changed() {
                        changed = true;
                    }
                }
                crate::nethook::Block::Paragraph(t)
                | crate::nethook::Block::Quote(t)
                | crate::nethook::Block::List(t) => {
                    if ui
                        .add(egui::TextEdit::multiline(t).desired_rows(4).desired_width(ui.available_width()))
                        .changed()
                    {
                        changed = true;
                    }
                }
                crate::nethook::Block::Picture(n)
                | crate::nethook::Block::Sound(n)
                | crate::nethook::Block::Film(n) => {
                    for name in &files {
                        if theme::neon_btn_color(ui, name, CYAN, n == name).clicked() {
                            *n = name.clone();
                            changed = true;
                        }
                    }
                    if files.is_empty() {
                        wrap_text(ui, "Attach a file, then pick it here.", MUTED, 12.0);
                    }
                }
                crate::nethook::Block::Columns(a, b) => {
                    if ui.add(egui::TextEdit::singleline(a).hint_text("Left").desired_width(ui.available_width())).changed() {
                        changed = true;
                    }
                    if ui.add(egui::TextEdit::singleline(b).hint_text("Right").desired_width(ui.available_width())).changed() {
                        changed = true;
                    }
                }
                crate::nethook::Block::Rule => {
                    wrap_text(ui, "A line between sections.", MUTED, 12.0);
                }
            }
            if changed {
                self.sync_hook_blocks();
            }
        }
    }

    fn ui_nethook_page(&mut self, ui: &mut egui::Ui) {
        if self.nethooks.is_empty() {
            wrap_text(ui, "Press + New nethook to put a site on the grid.", MUTED, 14.0);
            return;
        }
        self.hook_i = self.hook_i.min(self.nethooks.len() - 1);
        let mine = self.net.self_id.clone();
        let pinned = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.pinned())
            .unwrap_or(false);
        let owned = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.owner_id == mine && !h.pinned())
            .unwrap_or(false);
        if self.hook_edit && owned {
            let area = ui.available_rect_before_wrap();
            ui.allocate_rect(area, egui::Sense::hover());
            let mid = area.left() + area.width() * 0.5;
            let (ed, prev) = area.split_left_right_at_x(mid);
            let mut ed_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(ed.shrink2(Vec2::new(6.0, 4.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            ed_ui.label(
                RichText::new("EDITOR")
                    .family(theme::mono())
                    .size(11.0)
                    .color(CYAN),
            );
            wrap_text(
                &mut ed_ui,
                "HTML and one style block. See the pinned HTML and CSS pages. Scripts are stripped. The preview is live.",
                MUTED,
                12.0,
            );
            ed_ui.label(RichText::new("Title").family(theme::mono()).size(10.0).color(DIM));
            ed_ui.add(
                egui::TextEdit::singleline(&mut self.hook_draft_title)
                    .desired_width(ed_ui.available_width())
                    .font(FontId::new(16.0, theme::display()))
                    .text_color(CYAN),
            );
            ed_ui.horizontal_wrapped(|ui| {
                if theme::neon_btn_color(ui, "Code", CYAN, !self.hook_build).clicked() {
                    self.hook_build = false;
                }
                if theme::neon_btn_color(ui, "Build", CYAN, self.hook_build).clicked() {
                    self.hook_blocks = crate::nethook::html_to_blocks(&self.hook_draft_html);
                    self.hook_block_i = 0;
                    self.hook_build = true;
                }
                if theme::neon_btn(ui, "Attach").clicked() {
                    self.attach_hook_file();
                }
            });
            if self.hook_build {
                self.ui_hook_build(&mut ed_ui);
            } else {
                let rows = ((ed_ui.available_height() - 78.0) / 18.0).clamp(8.0, 48.0) as usize;
                egui::ScrollArea::vertical()
                    .id_salt("hook-code")
                    .max_height((ed_ui.available_height() - 70.0).max(80.0))
                    .show(&mut ed_ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.hook_draft_html)
                                .desired_width(ui.available_width())
                                .desired_rows(rows)
                                .font(FontId::new(14.0, theme::mono()))
                                .text_color(CREAM),
                        );
                    });
            }
            ed_ui.horizontal_wrapped(|ui| {
                if theme::neon_btn(ui, "Save").clicked() {
                    if self.hook_build {
                        self.sync_hook_blocks();
                    }
                    self.save_hook_draft();
                }
                if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
                    self.hook_build = false;
                    self.cancel_hook_edit();
                }
            });
            let mut prev_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(prev.shrink2(Vec2::new(8.0, 4.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            prev_ui.label(
                RichText::new("LIVE PREVIEW")
                    .family(theme::mono())
                    .size(11.0)
                    .color(CYAN),
            );
            let title = self.hook_draft_title.clone();
            let html = self.hook_draft_html.clone();
            let who = self.handle.clone();
            let prev_h = prev_ui.available_height().max(40.0);
            egui::ScrollArea::vertical()
                .id_salt("hook-live")
                .max_height(prev_h)
                .auto_shrink([false, false])
                .show(&mut prev_ui, |ui| {
                    let id = self
                        .nethooks
                        .get(self.hook_i)
                        .map(|h| h.id.clone())
                        .unwrap_or_default();
                    crate::nethook::chrome_frame(ui, &title, &who, |ui| {
                        self.paint_hook(ui, &id, &html);
                    });
                });
            return;
        }
        let title = self.nethooks[self.hook_i].title.clone();
        let owner = self.nethooks[self.hook_i].owner_name.clone();
        let html = self.nethooks[self.hook_i].html.clone();
        let mut gone = false;
        ui.horizontal_wrapped(|ui| {
            if owned && !pinned && theme::neon_btn(ui, "Send").clicked() {
                if let Some(h) = self.nethooks.get(self.hook_i) {
                    if let Ok(body) = serde_json::to_string(h) {
                        let label = h.title.clone();
                        self.open_share("nethook", &label, body);
                    }
                }
            }
            if owned && theme::neon_btn(ui, "Edit").clicked() {
                self.hook_draft_title = title.clone();
                self.hook_draft_html = html.clone();
                self.hook_edit = true;
            }
            if owned && theme::neon_btn(ui, if self.nethooks[self.hook_i].posted { "Take down" } else { "Post to table" }).clicked() {
                self.toggle_post_hook();
            }
            if owned && theme::neon_btn_color(ui, "Delete", KILL, false).clicked() {
                self.delete_current_hook();
                gone = true;
            }
            if pinned {
                wrap_text(
                    ui,
                    "Pinned lesson. It stays. You cannot edit or delete it.",
                    MUTED,
                    12.0,
                );
            } else if !owned {
                wrap_text(ui, "Read only. You did not write this site.", MUTED, 12.0);
            }
        });
        if gone || self.nethooks.is_empty() {
            return;
        }
        let id = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.id.clone())
            .unwrap_or_default();
        crate::nethook::chrome_frame(ui, &title, &owner, |ui| {
            self.paint_hook(ui, &id, &html);
        });
    }

    fn save_hook_draft(&mut self) {
        let Some(h) = self.nethooks.get_mut(self.hook_i) else {
            return;
        };
        if h.pinned() || h.owner_id != self.net.self_id {
            return;
        }
        let mut title = self.hook_draft_title.trim().to_string();
        if title.is_empty() {
            title = "Untitled".into();
        }
        title.truncate(80);
        let mut html = crate::nethook::strip_danger(&self.hook_draft_html);
        if html.len() > 48_000 {
            html.truncate(48_000);
        }
        h.title = title;
        h.html = html;
        h.owner_name = self.handle.clone();
        h.touch();
        let saved = h.clone();
        crate::nethook::save_one(&self.root, &saved);
        self.hook_edit = false;
        if self.live_link() {
            self.send_hook(&saved);
        }
        self.chat.push("Nethook saved. Live tables pick it up.".into());
    }

    fn cancel_hook_edit(&mut self) {
        self.hook_edit = false;
        let Some(h) = self.nethooks.get(self.hook_i) else {
            return;
        };
        if h.pinned() || h.owner_id != self.net.self_id {
            return;
        }
        let path = crate::nethook::dir(&self.root).join(format!("{}.json", h.id));
        if !path.is_file() {
            self.nethooks.remove(self.hook_i);
            if self.hook_i >= self.nethooks.len() {
                self.hook_i = self.nethooks.len().saturating_sub(1);
            }
        }
    }

    fn delete_current_hook(&mut self) {
        let Some(h) = self.nethooks.get(self.hook_i) else {
            return;
        };
        if h.pinned() || h.owner_id != self.net.self_id {
            return;
        }
        let id = h.id.clone();
        let owner = h.owner_id.clone();
        crate::nethook::delete_one(&self.root, &id);
        self.nethooks.remove(self.hook_i);
        if self.hook_i >= self.nethooks.len() {
            self.hook_i = self.nethooks.len().saturating_sub(1);
        }
        self.hook_edit = false;
        if self.live_link() {
            self.net.send_nethook_del(&id, &owner);
        }
        self.chat.push("Nethook pulled from the grid.".into());
    }

    fn add_starter(&mut self, kind: &str) {
        let date = format!("{:04}-{:02}-{:02}", self.cal_y, self.cal_m, self.cal_d);
        let scene = if self.scene.is_empty() {
            "no scene".into()
        } else {
            self.scene.clone()
        };
        let h = crate::nethook::Nethook::starter(
            kind,
            &self.net.self_id,
            &self.handle,
            &date,
            &self.place_name(),
            &scene,
        );
        self.hook_draft_title = h.title.clone();
        self.hook_draft_html = h.html.clone();
        let at = self
            .nethooks
            .iter()
            .position(|x| !x.pinned())
            .unwrap_or(self.nethooks.len());
        self.nethooks.insert(at, h);
        self.hook_i = at;
        self.hook_edit = true;
    }

    fn toggle_post_hook(&mut self) {
        let Some(cur) = self.nethooks.get(self.hook_i) else {
            return;
        };
        if cur.pinned() || cur.owner_id != self.net.self_id {
            return;
        }
        let id = cur.id.clone();
        let turn_on = !cur.posted;
        let mut changed = Vec::new();
        for h in &mut self.nethooks {
            if h.pinned() {
                continue;
            }
            let want = turn_on && h.id == id;
            if h.posted != want {
                h.posted = want;
                h.touch();
                changed.push(h.clone());
            }
        }
        for h in &changed {
            crate::nethook::save_one(&self.root, h);
            if self.live_link() {
                self.send_hook(h);
            }
        }
        self.board_open = turn_on;
        if turn_on {
            if !self.panel_on(Overlay::Board) {
                self.open.push(Overlay::Board);
            }
        } else {
            self.open.retain(|o| *o != Overlay::Board);
        }
    }

    fn ui_table_board(&mut self, ui: &mut egui::Ui) {
        if theme::overlay_chrome(
            ui,
            "TABLE://BOARD",
            "BOARD",
            "Posted nethooks · write in NETHOOKS then Post to table",
        ) {
            self.toggle_overlay(Overlay::Board);
            return;
        }
        let posted = self.nethooks.iter().find(|h| h.posted).cloned();
        let avail = ui.available_height();
        let board_h = if avail.is_finite() {
            avail.clamp(64.0, 640.0)
        } else {
            180.0
        };
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), board_h),
            egui::Sense::hover(),
        );
        theme::plate(ui, rect);
        let inner = rect.shrink(8.0);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(inner);
        match posted {
            Some(h) => {
                child.label(
                    RichText::new("ON THE TABLE")
                        .family(theme::mono())
                        .size(11.0)
                        .color(theme::chrome().acid),
                );
                let id = h.id.clone();
                let html = h.html.clone();
                egui::ScrollArea::vertical()
                    .id_salt("table-board")
                    .max_height((inner.height() - 24.0).max(40.0))
                    .show(&mut child, |ui| {
                        self.paint_hook(ui, &id, &html);
                    });
            }
            None => {
                wrap_text(
                    &mut child,
                    "Nothing is posted. In NETHOOKS, open a page you wrote and press Post to table.",
                    MUTED,
                    13.0,
                );
            }
        }
    }

    fn draw_tour(&mut self, ctx: &egui::Context) {
        let Some(step) = self.tour else {
            return;
        };
        let step = step.min(TOUR.len() - 1);
        let card = &TOUR[step];
        egui::Area::new(egui::Id::new("tour-card"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -48.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(12, 12, 8))
                    .stroke(egui::Stroke::new(2.0, theme::ACID))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        let w = (ui.ctx().screen_rect().width() - 48.0).clamp(220.0, 560.0);
                        ui.set_max_width(w);
                        ui.set_width(w);
                        ui.label(
                            RichText::new(format!("TOUR {} / {}", step + 1, TOUR.len()))
                                .family(theme::mono())
                                .size(11.0)
                                .color(theme::ACID),
                        );
                        ui.label(
                            RichText::new(card.title)
                                .family(theme::display())
                                .size(22.0)
                                .color(CYAN),
                        );
                        wrap_text(ui, card.body, CREAM, 14.0);
                        ui.horizontal_wrapped(|ui| {
                            if theme::neon_btn(ui, "Back").clicked() && step > 0 {
                                self.tour = Some(step - 1);
                                self.jump_tour(step - 1);
                            }
                            if theme::neon_btn(ui, "Next").clicked() {
                                if step + 1 >= TOUR.len() {
                                    self.end_tour();
                                } else {
                                    self.tour = Some(step + 1);
                                    self.jump_tour(step + 1);
                                }
                            }
                            if theme::neon_btn_color(ui, "Skip", KILL, false).clicked() {
                                self.end_tour();
                            }
                        });
                    });
            });
    }

    fn jump_tour(&mut self, step: usize) {
        let Some(card) = TOUR.get(step) else {
            return;
        };
        let page = card.page;
        let dock = card.dock;
        let open = card.open;
        self.page = page;
        self.shell = dock;
        self.player_full = page == Page::Player;
        self.watch_open = false;
        self.place_open = false;
        self.open.clear();
        self.viewing = None;
        match open {
            TourOpen::None => {}
            TourOpen::Panel(Overlay::Vendors) => {
                self.kit_filter.clear();
                self.restock_vendor();
                self.open.push(Overlay::Vendors);
            }
            TourOpen::Panel(Overlay::Armory) => {
                self.kit_filter.clear();
                self.open.push(Overlay::Armory);
            }
            TourOpen::Panel(panel) => self.open.push(panel),
            TourOpen::Games => {
                self.open.push(Overlay::Blackjack);
                if self.blight {
                    self.open.push(Overlay::Chess);
                }
            }
            TourOpen::Book => self.open_tour_book(),
            TourOpen::Seat => self.open.push(Overlay::Chars),
        }
        if page == Page::Netspace {
            self.jack_at = Instant::now();
        }
        if page == Page::Nethooks {
            self.hook_edit = false;
        }
    }

    fn open_tour_book(&mut self) {
        let (name, file) = if self.blight {
            ("Datashard", "datashard.json")
        } else {
            ("Bestiary", "bestiary.json")
        };
        self.catalog_rows = load_list(&self.root, file);
        self.catalog_pick = 0;
        self.catalog_q.clear();
        self.overlay_cat = name;
        self.open.push(Overlay::Catalog);
    }

    fn end_tour(&mut self) {
        self.tour = None;
        self.open.clear();
        self.viewing = None;
        self.shell = ShellPanel::None;
        self.watch_open = false;
        self.place_open = false;
    }

    fn ui_tutorial(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "← INDEX").clicked() {
                self.page = Page::Index;
            }
        });
        ui.add_space(6.0);
        ui.label(
            RichText::new("FIELD MANUAL")
                .family(theme::display())
                .size(32.0)
                .color(theme::ACID),
        );
        if theme::neon_btn(ui, "Start tour").clicked() {
            self.tour = Some(0);
            self.jump_tour(0);
        }
        wrap_text(
            ui,
            "The tour opens the real window, one part at a time. Each card is the desk, a table tool, talk, the player, the folders, the shell, recon, a page, the city, or the fixer.",
            MUTED,
            13.0,
        );
        theme::kicker(ui, "NETDIR://TUTORIAL");
        let (rule, _) = ui.allocate_exact_size(Vec2::new(120.0, 2.0), egui::Sense::hover());
        ui.painter().rect_filled(rule, 0.0, CYAN);
        ui.add_space(10.0);
        let manual_h = ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("field-manual")
            .max_height(manual_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, card) in TOUR.iter().enumerate() {
                    let id = format!("{:02}", i + 1);
                    egui::Frame::NONE
                        .fill(PANEL)
                        .stroke(egui::Stroke::new(1.0, theme::fade(theme::HOT, 140)))
                        .inner_margin(egui::Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&id)
                                        .family(theme::mono())
                                        .size(13.0)
                                        .color(theme::ACID),
                                );
                                ui.label(
                                    RichText::new(card.title)
                                        .family(theme::display())
                                        .size(18.0)
                                        .color(CREAM),
                                );
                            });
                            ui.add_space(4.0);
                            wrap_text(ui, card.body, CREAM, 14.0);
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn ui_audio(&mut self, ui: &mut egui::Ui) {
        let h = ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("audio-page")
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
            ui.set_width((ui.available_width() - 12.0).max(40.0));
            if theme::neon_btn(ui, "← INDEX").clicked() {
                self.page = Page::Index;
            }
            ui.heading(RichText::new("Audio").family(theme::display()).color(theme::ACID));
            ui.label("Mic send is how loud you go out. Listen levels are local — they never change someone else for the table.");
            ui.horizontal(|ui| {
                ui.label("Mic send");
                ui.add(egui::Slider::new(&mut self.mic_gain, 0.0..=2.0).suffix("x"));
            });
            ui.label(RichText::new("Voice is on the top row. Mic send is local — it never changes someone else's table.").color(MUTED).small());
            if theme::neon_btn(ui, "Open Voice").clicked() {
                self.shell = ShellPanel::Voice;
                self.ensure_mic();
            }
        });
    }

    fn ui_bj(&mut self, ui: &mut egui::Ui) {
        let h = ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("bj-page")
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.ui_bj_body(ui, theme::index_palette());
            });
    }
}

fn start_mic(want: Option<&str>) -> Option<(cpal::Stream, Receiver<Vec<f32>>, u32)> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let mut aliases = Vec::new();
    if let Some(name) = want {
        if !name.is_empty() {
            aliases.push(name.to_string());
            for d in crate::audio::list_inputs() {
                if d.id == name || d.label == name || d.alt == name {
                    aliases.push(d.id);
                    aliases.push(d.label);
                    aliases.push(d.alt);
                }
            }
        }
    }
    let mut dev = None;
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(n) = d.name() {
                if aliases.iter().any(|a| a == &n) {
                    dev = Some(d);
                    break;
                }
            }
        }
    }
    let dev = dev.or_else(|| host.default_input_device())?;
    let cfg = dev.default_input_config().ok()?;
    let rate = cfg.sample_rate().0;
    let (tx, rx) = std::sync::mpsc::channel();
    let err_fn = |e| eprintln!("mic: {e}");
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => {
            let conf: cpal::StreamConfig = cfg.into();
            let ch = conf.channels.max(1) as usize;
            dev.build_input_stream(
                &conf,
                move |data: &[f32], _| {
                    let mut mono = Vec::with_capacity(data.len() / ch);
                    if ch <= 1 {
                        mono.extend_from_slice(data);
                    } else {
                        for frame in data.chunks(ch) {
                            mono.push(frame.iter().copied().sum::<f32>() / ch as f32);
                        }
                    }
                    let _ = tx.send(mono);
                },
                err_fn,
                None,
            )
            .ok()?
        }
        cpal::SampleFormat::I16 => {
            let conf: cpal::StreamConfig = cfg.into();
            let ch = conf.channels.max(1) as usize;
            dev.build_input_stream(
                &conf,
                move |data: &[i16], _| {
                    let mut mono = Vec::with_capacity(data.len() / ch);
                    if ch <= 1 {
                        mono.extend(data.iter().map(|s| *s as f32 / 32767.0));
                    } else {
                        for frame in data.chunks(ch) {
                            let v = frame.iter().map(|s| *s as f32).sum::<f32>()
                                / ch as f32
                                / 32767.0;
                            mono.push(v);
                        }
                    }
                    let _ = tx.send(mono);
                },
                err_fn,
                None,
            )
            .ok()?
        }
        cpal::SampleFormat::I32 => {
            let conf: cpal::StreamConfig = cfg.into();
            let ch = conf.channels.max(1) as usize;
            dev.build_input_stream(
                &conf,
                move |data: &[i32], _| {
                    let mut mono = Vec::with_capacity(data.len() / ch);
                    if ch <= 1 {
                        mono.extend(data.iter().map(|s| *s as f32 / 2147483648.0));
                    } else {
                        for frame in data.chunks(ch) {
                            let v = frame.iter().map(|s| *s as f32).sum::<f32>()
                                / ch as f32
                                / 2147483648.0;
                            mono.push(v);
                        }
                    }
                    let _ = tx.send(mono);
                },
                err_fn,
                None,
            )
            .ok()?
        }
        _ => return None,
    };
    stream.play().ok()?;
    Some((stream, rx, rate))
}

fn rail_block(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let ch = theme::chrome();
    ui.label(
        RichText::new(label)
            .family(theme::mono())
            .size(10.0)
            .color(theme::fade(ch.cyan, 160)),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
        add(ui);
    });
    ui.add_space(8.0);
}

fn media_tag(mime: &str) -> &'static str {
    if mime.starts_with("audio/") {
        "aud"
    } else if mime.starts_with("video/") {
        "vid"
    } else if mime.starts_with("image/") {
        "img"
    } else {
        "file"
    }
}

fn media_caption(tag: &str, who: &str) -> String {
    match tag {
        "aud" => format!("{who} sent audio"),
        "vid" => format!("{who} sent video"),
        "file" => format!("{who} sent a file"),
        _ => format!("{who} sent a picture"),
    }
}

fn mime_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ogg" | "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "m4a" | "aac" => "audio/mp4",
        "mp4" | "m4v" | "mov" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "json" => "application/json",
        "txt" | "md" | "log" | "csv" | "toml" => "text/plain",
        _ => "application/octet-stream",
    }
}

fn sniff_ext(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return "mp4";
    }
    if bytes.len() >= 4 && bytes[0] == 0x1A && bytes[1] == 0x45 {
        return "webm";
    }
    if bytes.starts_with(b"RIFF") {
        if bytes.len() >= 12 && &bytes[8..12] == b"WAVE" {
            return "wav";
        }
        if bytes.len() >= 12 && &bytes[8..12] == b"AVI " {
            return "avi";
        }
    }
    if bytes.starts_with(b"OggS") {
        return "ogg";
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        return "mp3";
    }
    "mp4"
}

fn open_media(bytes: &[u8], ext: &str) {
    let dir = std::env::temp_dir().join("blightnet-media");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("clip-{:08x}.{ext}", rand::random::<u32>()));
    if std::fs::write(&path, bytes).is_err() {
        return;
    }
    let _ = crate::sys::open_path(&path);
}

fn collect_music(dir: &Path, out: &mut Vec<PathBuf>, depth: u8) {
    if depth == 0 || out.len() >= 400 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        if out.len() >= 400 {
            break;
        }
        let p = e.path();
        if p.is_dir() {
            collect_music(&p, out, depth.saturating_sub(1));
        } else if is_library_file(&p) {
            out.push(p);
        }
    }
}

fn load_deck_lib(root: &Path) -> Vec<PathBuf> {
    let raw = std::fs::read_to_string(root.join("data/deck-library.json")).ok();
    let Some(s) = raw else {
        return Vec::new();
    };
    let Ok(paths) = serde_json::from_str::<Vec<String>>(&s) else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_file() && crate::audio::is_music(p))
        .collect()
}

fn save_deck_lib(root: &Path, list: &[PathBuf]) {
    let paths: Vec<String> = list
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    if let Ok(s) = serde_json::to_string_pretty(&paths) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/deck-library.json"), s);
    }
}

fn load_deck_vol(root: &Path) -> Option<f32> {
    std::fs::read_to_string(root.join("data/deck-vol.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .map(|v: f32| v.clamp(0.0, 1.0))
}

fn save_deck_vol(root: &Path, vol: f32) {
    let _ = std::fs::create_dir_all(root.join("data"));
    let _ = std::fs::write(root.join("data/deck-vol.txt"), format!("{vol:.3}"));
}

fn safe_filename(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.replace('|', "_").replace(']', "_");
    if s.is_empty() {
        "file".into()
    } else {
        s
    }
}

fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn load_chat(root: &Path) -> Option<Vec<String>> {
    let s = std::fs::read_to_string(root.join("data/chat.json")).ok()?;
    serde_json::from_str(&s).ok()
}

fn save_chat(root: &Path, chat: &[String]) {
    if let Ok(s) = serde_json::to_string_pretty(chat) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/chat.json"), s);
    }
}

fn load_inbox(root: &Path) -> HashMap<String, InboxFile> {
    let mut out = HashMap::new();
    let dir = root.join("data/inbox");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let key = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        let mime = mime_of(&p);
        out.insert(
            key.clone(),
            InboxFile {
                mime: mime.into(),
                filename: key,
                path: p,
            },
        );
    }
    out
}

fn token_sig(toks: &[maps::MapTok]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    toks.len().hash(&mut h);
    for t in toks {
        t.id.hash(&mut h);
        t.x.to_bits().hash(&mut h);
        t.y.to_bits().hash(&mut h);
        t.size.to_bits().hash(&mut h);
        t.name.hash(&mut h);
        t.image.hash(&mut h);
        t.sheet.hash(&mut h);
    }
    h.finish()
}

fn sheet_sig(rows: &[Character]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if let Ok(b) = serde_json::to_vec(rows) {
        h.write(&b);
    } else {
        rows.len().hash(&mut h);
    }
    h.finish()
}

const GITHUB_REPO: &str = "https://github.com/Steelworth/Blightnet.git";

fn git_at(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(["-c", "safe.directory=*"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("LD_PRELOAD")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|_| "Git is not on PATH. Install Git, then press Update again.".to_string())?;
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let ok = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut text = String::new();
    if !ok.is_empty() {
        text.push_str(&ok);
    }
    if !err.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&err);
    }
    if out.status.success() {
        Ok(if text.is_empty() {
            "Already up to date.".into()
        } else {
            text
        })
    } else if text.is_empty() {
        Err(format!("Git exited with {}.", out.status))
    } else {
        Err(text)
    }
}

fn local_edits_block(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("local changes")
        || m.contains("would be overwritten")
        || m.contains("please commit your changes")
        || m.contains("please move or remove")
}

fn github_update(root: &Path) -> Result<String, String> {
    github_update_from(root, GITHUB_REPO)
}

fn github_update_from(root: &Path, url: &str) -> Result<String, String> {
    if !root.join("data").join("mixer-catalog.json").is_file() {
        return Err("This folder is not a Blightnet deck.".into());
    }
    if !root.join(".git").exists() {
        git_at(root, &["init", "-b", "main"])?;
        let _ = git_at(root, &["remote", "remove", "origin"]);
        git_at(root, &["remote", "add", "origin", url])?;
        git_at(root, &["fetch", "--depth", "1", "origin", "main"])?;
        git_at(root, &["checkout", "-f", "-B", "main", "FETCH_HEAD"])?;
        return Ok("Downloaded the latest Blightnet.".into());
    }
    let fetched = match git_at(root, &["fetch", url, "main"]) {
        Ok(text) => text,
        Err(https_err) => match git_at(root, &["fetch", "origin", "main"]) {
            Ok(text) => text,
            Err(origin_err) => {
                return Err(format!("{https_err}\n{origin_err}"));
            }
        },
    };
    match git_at(root, &["merge", "--ff-only", "FETCH_HEAD"]) {
        Ok(merged) => Ok(format!("{fetched}\n{merged}")),
        Err(err) if local_edits_block(&err) => {
            let _ = git_at(root, &["stash", "push", "-u", "-m", "blightnet-update"]);
            let merged = git_at(root, &["merge", "--ff-only", "FETCH_HEAD"])?;
            match git_at(root, &["stash", "pop"]) {
                Ok(_) => Ok(format!(
                    "{merged}\nYour local edits were set aside and put back."
                )),
                Err(pop) => Ok(format!(
                    "{merged}\nThe update is in. Your local edits are still in the git stash. {pop}"
                )),
            }
        }
        Err(err) => Err(err),
    }
}

fn pack_recon(root: &Path, file: &crate::recon::Dossier) -> String {
    let mut portrait_b64 = String::new();
    let mut ext = String::new();
    if !file.portrait.is_empty() {
        if let Ok(bytes) = std::fs::read(root.join(&file.portrait)) {
            if bytes.len() < 4_000_000 {
                portrait_b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                ext = std::path::Path::new(&file.portrait)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("png")
                    .to_string();
            }
        }
    }
    serde_json::json!({
        "file": file,
        "portrait_b64": portrait_b64,
        "ext": ext,
    })
    .to_string()
}

fn brief_bytes(n: u64) -> String {
    const G: f64 = 1024.0 * 1024.0 * 1024.0;
    const M: f64 = 1024.0 * 1024.0;
    if n as f64 >= G {
        format!("{:.1}G", n as f64 / G)
    } else if n as f64 >= M {
        format!("{:.0}M", n as f64 / M)
    } else {
        format!("{:.0}K", (n as f64 / 1024.0).max(0.0))
    }
}

fn meter_pair(ui: &mut egui::Ui, k: &str, v: &str, hot: bool, bad: bool) {
    let col = if bad {
        KILL
    } else if hot {
        theme::ACID
    } else {
        CYAN
    };
    ui.label(
        RichText::new(k)
            .family(theme::mono())
            .size(10.0)
            .color(DIM),
    )
    .on_hover_text("This computer. Not sent to the table.");
    ui.label(
        RichText::new(v)
            .family(theme::mono())
            .size(11.0)
            .color(col),
    );
}

fn wrap_text(ui: &mut egui::Ui, text: &str, color: Color32, size: f32) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(color)
                .size(size)
                .family(theme::ui_font()),
        )
        .wrap(),
    );
}

fn load_devices(root: &Path) -> DevicePref {
    std::fs::read_to_string(root.join("data/devices.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_devices(root: &Path, pref: &DevicePref) {
    if let Ok(s) = serde_json::to_string_pretty(pref) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/devices.json"), s);
    }
}

const UI_SCALE_MIN: f32 = 0.85;
const UI_SCALE_MAX: f32 = 1.75;
const UI_SCALE_DEFAULT: f32 = 1.0;

fn clamp_ui_scale(s: f32) -> f32 {
    let c = s.clamp(UI_SCALE_MIN, UI_SCALE_MAX);
    (c * 20.0).round() / 20.0
}

fn load_chrome(root: &Path) -> (theme::ChromeTheme, f32) {
    let v = std::fs::read_to_string(root.join("data/chrome.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    let theme = v
        .as_ref()
        .and_then(|v| v.get("theme").and_then(|t| t.as_str()))
        .map(theme::ChromeTheme::from_id)
        .unwrap_or(theme::ChromeTheme::NeonDeck);
    let ui_scale = v
        .as_ref()
        .and_then(|v| v.get("ui_scale").and_then(|t| t.as_f64()))
        .map(|f| clamp_ui_scale(f as f32))
        .unwrap_or(UI_SCALE_DEFAULT);
    (theme, ui_scale)
}

fn save_chrome(root: &Path, theme: theme::ChromeTheme, ui_scale: f32) {
    let v = serde_json::json!({
        "theme": theme.id(),
        "ui_scale": clamp_ui_scale(ui_scale),
    });
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/chrome.json"), s);
    }
}

fn load_crews(root: &Path) -> Vec<Crew> {
    std::fs::read_to_string(root.join("data/crews.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_crews(root: &Path, rows: &[Crew]) {
    if let Ok(s) = serde_json::to_string_pretty(rows) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/crews.json"), s);
    }
}

fn downsample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from <= to || samples.is_empty() {
        return samples.to_vec();
    }
    let step = (from as f32 / to as f32).max(1.0);
    let mut out = Vec::new();
    let mut i = 0.0;
    while (i as usize) < samples.len() {
        out.push(samples[i as usize]);
        i += step;
    }
    out
}

fn tab(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    theme::chrome_tab(ui, label, on)
}

fn quiet_btn(ui: &mut egui::Ui, label: &str, color: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .family(theme::mono())
                .size(10.0)
                .color(color),
        )
        .frame(false),
    )
}

fn status_pair(ui: &mut egui::Ui, k: &str, v: &str, value: Color32) {
    ui.label(RichText::new(k).family(theme::mono()).size(10.0).color(DIM));
    ui.label(RichText::new(v).family(theme::mono()).size(10.0).color(value));
}


fn session_chip(ui: &mut egui::Ui, k: &str, v: &str, value: Color32) {
    egui::Frame::NONE
        .fill(theme::glass())
        .stroke(egui::Stroke::new(1.0, theme::fade(value, 200)))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).family(theme::mono()).size(10.0).color(DIM));
                ui.label(RichText::new(v).family(theme::mono()).size(10.0).color(value));
            });
        });
}

/// Parse Host path-health flags already pushed into `status` by net (UPnP/public IP/mesh).
fn path_health_flags(status: &str) -> Option<(bool, bool, bool, bool)> {
    if !status.contains("path health") {
        return None;
    }
    let flag = |key: &str| -> Option<bool> {
        let i = status.find(key)?;
        let rest = status[i + key.len()..].trim_start();
        match rest.chars().next() {
            Some('Y') | Some('y') => Some(true),
            Some('N') | Some('n') => Some(false),
            _ => None,
        }
    };
    Some((
        flag("UPnP TCP")?,
        flag("UPnP UDP")?,
        flag("public IP")?,
        flag("udp mesh")?,
    ))
}

fn session_path_readout(
    node_live: bool,
    role: Role,
    internet: bool,
    status: &str,
    cached: &str,
) -> (String, Color32) {
    if !node_live {
        return ("—".into(), MUTED);
    }
    match role {
        Role::Host if internet => {
            let flags = path_health_flags(status).or_else(|| path_health_flags(cached));
            if let Some((tcp, udp, ip, mesh)) = flags {
                let yn = |b: bool| if b { "Y" } else { "N" };
                let s = format!(
                    "UPnP {}/{} · IP {} · mesh {}",
                    yn(tcp),
                    yn(udp),
                    yn(ip),
                    yn(mesh)
                );
                let ok = tcp || udp || ip || mesh;
                (s, if ok { theme::ACID } else { theme::ORANGE })
            } else if status.contains("building invite") {
                ("probing…".into(), CYAN)
            } else {
                ("internet…".into(), CYAN)
            }
        }
        Role::Host => ("LAN".into(), CYAN),
        Role::Guest => ("join".into(), MUTED),
        Role::Presence | Role::Idle => ("—".into(), MUTED),
    }
}

fn meta_c(ui: &mut egui::Ui, k: &str, v: &str, value: Color32) {
    egui::Frame::NONE
        .fill(theme::glass())
        .stroke(egui::Stroke::new(1.0, theme::fade(theme::HOT, 170)))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).family(theme::mono()).size(10.0).color(DIM));
                ui.label(RichText::new(v).family(theme::mono()).size(10.0).color(value));
            });
        });
}

fn pretty(v: &serde_json::Value) -> String {
    let mut head = String::new();
    let mut body = String::new();
    if let Some(o) = v.as_object() {
        const LONG: &[&str] = &[
            "text", "origin", "blurb", "hooks", "desc", "description", "lore", "notes",
        ];
        const SKIP: &[&str] = &["id", "file", "files", "image", "portrait"];
        for (k, val) in o {
            if SKIP.contains(&k.as_str()) {
                continue;
            }
            if let Some(s) = val.as_str() {
                if s.trim().is_empty() {
                    continue;
                }
                if LONG.contains(&k.as_str()) || s.len() > 48 {
                    body.push_str(&format!("\n{s}\n"));
                } else {
                    head.push_str(&format!("{k}: {s}\n"));
                }
            } else if let Some(n) = val.as_i64() {
                head.push_str(&format!("{k}: {n}\n"));
            } else if let Some(n) = val.as_f64() {
                head.push_str(&format!("{k}: {n}\n"));
            } else if let Some(b) = val.as_bool() {
                head.push_str(&format!("{k}: {b}\n"));
            }
        }
    }
    format!("{head}{body}")
}

fn kit_blurb(v: &serde_json::Value) -> String {
    let mut bits = Vec::new();
    for k in [
        "kind", "cat", "cost", "dmg", "props", "wt", "rof", "shots", "ac", "rarity", "hl", "lv",
    ] {
        match v.get(k) {
            Some(serde_json::Value::String(s)) if !s.is_empty() => bits.push(s.clone()),
            Some(serde_json::Value::Number(n)) => bits.push(format!("{k} {n}")),
            _ => {}
        }
    }
    let text = v
        .get("text")
        .or_else(|| v.get("blurb"))
        .or_else(|| v.get("desc"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    let mut out = bits.join(" · ");
    if !text.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(text);
    }
    out
}

fn notes_key(handle: &str) -> String {
    let h = handle.trim();
    if h.is_empty() {
        "local".into()
    } else {
        h.to_string()
    }
}

fn load_notes(root: &Path, handle: &str) -> String {
    let raw = std::fs::read_to_string(root.join("data/notes.json")).unwrap_or_default();
    let map: HashMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();
    map.get(&notes_key(handle)).cloned().unwrap_or_default()
}

fn save_notes(root: &Path, handle: &str, text: &str) {
    let path = root.join("data/notes.json");
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    let mut map: HashMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();
    map.insert(notes_key(handle), text.to_string());
    if let Ok(s) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(path, s);
    }
}

fn load_combat_log(root: &Path) -> Vec<String> {
    std::fs::read_to_string(root.join("data/combat-log.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_combat_log(root: &Path, log: &[String]) {
    if let Ok(s) = serde_json::to_string_pretty(log) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/combat-log.json"), s);
    }
}

fn changelog_day(heading: &str) -> String {
    let t = heading.trim().trim_start_matches('#').trim();
    let day = t.split([' ', '—', '–', ':']).next().unwrap_or(t).trim();
    if day.len() >= 10 && day.as_bytes().get(4) == Some(&b'-') && day.as_bytes().get(7) == Some(&b'-')
    {
        day[..10].to_string()
    } else if !day.is_empty() {
        day.to_string()
    } else {
        t.to_string()
    }
}

fn load_changelog(root: &Path) -> Vec<(String, Vec<String>)> {
    let raw = std::fs::read_to_string(root.join("data/changelog.md")).unwrap_or_default();
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut day = String::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            day = changelog_day(t);
            if !out.iter().any(|(d, _)| d == &day) {
                out.push((day.clone(), Vec::new()));
            }
        } else if let Some(rest) = t.strip_prefix("- ") {
            if day.is_empty() {
                continue;
            }
            if let Some((_, bullets)) = out.iter_mut().find(|(d, _)| d == &day) {
                let line = rest.trim().to_string();
                if !line.is_empty() && !bullets.iter().any(|b| b == &line) {
                    bullets.push(line);
                }
            }
        }
    }
    out
}

fn catalog_art(root: &Path, title: &str, id: &str) -> PathBuf {
    let dir = match title {
        "Bestiary" | "NPCs" => "assets/bestiary",
        "Datashard" => "assets/datashard",
        "Gangs" => "assets/gangs",
        "Gods" => "assets/gods",
        "Lore" => "assets/lore",
        "Corps" => "assets/corps",
        "Faces" => "assets/npcs",
        _ => "assets",
    };
    let jpg = root.join(dir).join(format!("{id}.jpg"));
    if jpg.is_file() {
        return jpg;
    }
    let png = root.join(dir).join(format!("{id}.png"));
    if png.is_file() {
        png
    } else {
        jpg
    }
}

impl Drop for Blightnet {
    fn drop(&mut self) {
        self.drop_partials();
        self.stop_media_proc();
        self.stop_hook_video();
        self.term = None;
    }
}

fn is_library_file(path: &std::path::Path) -> bool {
    crate::audio::is_music(path)
        || matches!(media_kind(path), "image" | "video" | "pdf" | "gif" | "text")
}

fn paint_air_legend(ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (label, col) in [
            ("playing", theme::ACID),
            ("on air", CYAN),
            ("off air", KILL),
            ("not checked", DIM),
        ] {
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, col);
            ui.label(
                RichText::new(label)
                    .family(theme::mono())
                    .size(10.0)
                    .color(col),
            );
        }
    });
}

fn station_row(ui: &mut egui::Ui, name: &str, sub: &str, on: bool, mark: Color32) -> egui::Response {
    let w = ui.available_width().max(40.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 40.0), egui::Sense::click());
    let hover = resp.hovered();
    let edge = if on || hover { theme::ACID } else { theme::fade(CYAN, 90) };
    theme::fill_chamfer(
        ui,
        rect,
        6.0,
        PANEL,
        egui::Stroke::new(if on { 1.5 } else { 1.0 }, edge),
    );
    ui.painter()
        .circle_filled(rect.left_center() + Vec2::new(16.0, 0.0), 4.0, mark);
    let clip = Rect::from_min_max(
        rect.left_top() + Vec2::new(28.0, 3.0),
        rect.right_bottom() - Vec2::new(8.0, 3.0),
    );
    let p = ui.painter().with_clip_rect(clip);
    let fg = if on || hover { theme::ACID } else { CREAM };
    p.text(
        clip.left_top(),
        egui::Align2::LEFT_TOP,
        name,
        FontId::new(14.0, theme::ui_font()),
        fg,
    );
    p.text(
        clip.left_bottom(),
        egui::Align2::LEFT_BOTTOM,
        sub,
        FontId::new(11.0, theme::mono()),
        mark,
    );
    resp
}

fn paint_contain(ui: &mut egui::Ui, tex: &egui::TextureHandle, max: Vec2) -> egui::Response {
    let max = Vec2::new(max.x.max(40.0), max.y.max(40.0));
    let (rect, resp) = ui.allocate_exact_size(max, egui::Sense::click());
    ui.painter().rect_filled(rect, 4.0, PANEL);
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, theme::fade(CYAN, 90)),
        egui::StrokeKind::Inside,
    );
    let sz = tex.size_vec2();
    if sz.x > 1.0 && sz.y > 1.0 {
        let fit = rect.shrink(6.0);
        let scale = (fit.width() / sz.x).min(fit.height() / sz.y);
        let dest = Rect::from_center_size(fit.center(), sz * scale);
        ui.painter().with_clip_rect(fit).image(
            tex.id(),
            dest,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    resp
}

fn render_pdf(src: &Path, stem: &Path, page: i32) -> (bool, String) {
    if crate::sys::which("pdftoppm").is_none() {
        return (
            false,
            "pdftoppm is not on this computer, so this PDF cannot be drawn here.".into(),
        );
    }
    let page = page.max(1).to_string();
    let mut cmd = std::process::Command::new("pdftoppm");
    crate::sys::hide(&mut cmd);
    let ok = cmd
        .args(["-f", &page, "-l", &page, "-png", "-singlefile", "-scale-to", "1200"])
        .arg(src)
        .arg(stem)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        (true, String::new())
    } else {
        (false, "That PDF page did not render.".into())
    }
}

fn media_kind(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "webp" => "image",
        "gif" => "gif",
        "txt" | "md" | "log" | "csv" | "json" | "toml" => "text",
        "mp4" | "webm" | "mkv" => "video",
        "pdf" => "pdf",
        _ => "audio",
    }
}

fn pull_live(url: &str, dir: &std::path::Path, id: &str) -> Result<std::path::PathBuf, String> {
    let _ = std::fs::create_dir_all(dir);
    let stream = if url.ends_with(".pls") {
        let text = curl_text(url, 12)?;
        text.lines()
            .find_map(|l| l.trim().strip_prefix("File1=").map(|s| s.trim().to_string()))
            .ok_or_else(|| format!("{id} did not publish a stream."))?
    } else {
        url.to_string()
    };
    let bytes = curl_bin(&stream, 18)?;
    if bytes.len() < 2048 {
        return Err(format!("{id} is off the air."));
    }
    let dest = dir.join(format!("{id}-{}.bin", bytes.len().min(99999)));
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    Ok(dest)
}

fn curl_text(url: &str, secs: u64) -> Result<String, String> {
    let bytes = curl_bin(url, secs)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn curl_bin(url: &str, secs: u64) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            &secs.to_string(),
            "-A",
            "Blightnet",
            "-L",
            url,
        ])
        .output()
        .map_err(|_| "curl is not on this computer, so live stations cannot play.".to_string())?;
    if out.stdout.len() < 32 {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("The station did not answer. {err}"));
    }
    Ok(out.stdout)
}

const HOURS: [&str; 4] = ["morning", "day", "evening", "night"];

fn period_from_clock(mins: u32) -> &'static str {
    let t = mins % 1440;
    if t >= 21 * 60 || t < 6 * 60 {
        "night"
    } else if t < 11 * 60 {
        "morning"
    } else if t < 17 * 60 {
        "day"
    } else {
        "evening"
    }
}

fn clock_from_period(period: &str) -> u32 {
    match period {
        "morning" => 7 * 60,
        "evening" => 18 * 60 + 30,
        "night" => 23 * 60,
        _ => 13 * 60,
    }
}

fn weekday_sun0(y: i32, m: u32, d: u32) -> usize {
    let mut y = y;
    let mut m = m as i32;
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (d as i32 + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j).rem_euclid(7);
    ((h + 6) % 7) as usize
}

fn calc_apply(a: f64, op: char, b: f64) -> f64 {
    match op {
        '+' => a + b,
        '−' | '-' => a - b,
        '×' | '*' => a * b,
        '÷' | '/' => {
            if b.abs() < f64::EPSILON {
                0.0
            } else {
                a / b
            }
        }
        _ => b,
    }
}

fn fmt_calc(n: f64) -> String {
    if (n - n.round()).abs() < 1e-9 {
        format!("{:.0}", n.round())
    } else {
        format!("{n:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn days_in_month(y: i32, m: i32) -> i32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 400 == 0 || (y % 4 == 0 && y % 100 != 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn load_calendar(root: &Path) -> Option<(i32, u32, u32)> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("data/calendar.json")).ok()?).ok()?;
    Some((
        v.get("y")?.as_i64()? as i32,
        v.get("m")?.as_u64()? as u32,
        v.get("d")?.as_u64()? as u32,
    ))
}

fn save_calendar(root: &Path, y: i32, m: u32, d: u32) {
    let _ = std::fs::create_dir_all(root.join("data"));
    let _ = std::fs::write(
        root.join("data/calendar.json"),
        format!("{{\"y\":{y},\"m\":{m},\"d\":{d}}}"),
    );
}

fn paint_deck_viz(ui: &mut egui::Ui, samples: &[f32; 128]) {
    let w = ui.available_width().max(80.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 168.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6.0, PANEL);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, theme::fade(theme::HOT, 170)),
        egui::StrokeKind::Inside,
    );
    let bands = 16;
    let step = samples.len() / bands;
    let bw = rect.width() / bands as f32;
    for i in 0..bands {
        let mut energy = 0.0;
        for s in &samples[i * step..(i + 1) * step] {
            energy += s * s;
        }
        let amp = (energy / step as f32).sqrt().clamp(0.0, 1.0);
        let h = 6.0 + amp * (rect.height() - 28.0);
        let bar = Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * bw + 3.0, rect.bottom() - 10.0 - h),
            Vec2::new((bw - 6.0).max(2.0), h),
        );
        let alpha = (36.0 + amp * 190.0) as u8;
        ui.painter()
            .rect_filled(bar, 2.0, theme::fade(theme::ACID, alpha));
    }
    let mid = rect.center().y;
    let span = rect.height() * 0.36;
    for i in 0..127 {
        let x0 = rect.left() + rect.width() * (i as f32 / 127.0);
        let x1 = rect.left() + rect.width() * ((i + 1) as f32 / 127.0);
        let y0 = mid - samples[i].clamp(-1.0, 1.0) * span;
        let y1 = mid - samples[i + 1].clamp(-1.0, 1.0) * span;
        ui.painter().line_segment(
            [egui::pos2(x0, y0), egui::pos2(x1, y1)],
            egui::Stroke::new(1.6, CYAN),
        );
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 2.0,
        egui::Stroke::new(1.5, theme::HOT),
    );
}

fn paint_viz(ui: &mut egui::Ui, w: f32, h: f32, energy: f32, phase: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w.max(40.0), h), egui::Sense::hover());
    let n = 28;
    let gap = 2.0;
    let bw = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    for i in 0..n {
        let wobble = ((i as f32 * 0.45 + phase).sin() * 0.5 + 0.5).clamp(0.15, 1.0);
        let bh = (8.0 + energy * wobble * (rect.height() - 10.0)).min(rect.height());
        let x = rect.left() + i as f32 * (bw + gap);
        let bar = Rect::from_min_size(egui::pos2(x, rect.bottom() - bh), Vec2::new(bw.max(1.0), bh));
        let col = if i % 5 == 0 { CYAN } else { theme::ACID };
        ui.painter().rect_filled(bar, 0.0, col.gamma_multiply(0.85));
    }
}

fn format_clock(mins: u32) -> String {
    let t = mins % 1440;
    format!("{:02}:{:02}", t / 60, t % 60)
}

fn period_label(time: &str) -> &'static str {
    match time {
        "morning" => "Morning",
        "evening" => "Evening",
        "night" => "Night",
        _ => "Day",
    }
}

enum Space {
    In,
    Out,
    Both,
}

fn space_of(layer: &Layer) -> Space {
    if layer.category == "music" {
        return Space::Both;
    }
    if layer.category == "weather" {
        return Space::Out;
    }
    if layer.category == "animals" {
        const BOTH: &[&str] = &[
            "cats", "hounds", "cattle", "rooster", "farmyard", "purring", "donkeys", "chickens",
            "goats",
        ];
        if BOTH.contains(&layer.id.as_str()) {
            return Space::Both;
        }
        return Space::Out;
    }
    const INDOOR: &[&str] = &[
        "tavern_hall",
        "dungeon",
        "cavern",
        "temple",
        "forge",
        "torch",
        "magic",
        "market",
        "kitchen",
        "library",
        "sewers",
        "drips",
        "fireplace",
        "crowded_pub",
        "tomb",
        "clock",
        "horror",
        "big_fire",
    ];
    if layer.id == "church_bells"
        || matches!(
            layer.id.as_str(),
            "underwater" | "diving" | "deep_hum" | "sinking" | "bubbles"
        )
    {
        return Space::Both;
    }
    if INDOOR.contains(&layer.id.as_str()) {
        Space::In
    } else {
        Space::Out
    }
}

fn parse_coins(cost: &str) -> i32 {
    cost.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn load_saved(root: &Path) -> Vec<SavedMix> {
    std::fs::read_to_string(root.join("data/saved-mixes.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn transfer_clock(done: u64, total: u64, elapsed: f32) -> Option<(f32, f32)> {
    if total == 0 || done == 0 || done > total || elapsed < 0.05 {
        return None;
    }
    if done < 256 * 1024 && elapsed < 0.5 {
        return None;
    }
    let rate = done as f32 / elapsed;
    if rate <= 0.0 {
        return None;
    }
    let left = (total - done) as f32 / rate;
    Some((elapsed + left, left))
}

fn transfer_line(prefix: &str, done: u64, total: u64, elapsed: f32) -> String {
    let amounts = format!("{} of {}", brief_bytes(done), brief_bytes(total));
    match transfer_clock(done, total, elapsed) {
        Some((about, left)) => format!(
            "{prefix} · {amounts} · About {} · {} left",
            fmt_media(about),
            fmt_media(left)
        ),
        None => format!("{prefix} · {amounts}"),
    }
}

fn fmt_media(secs: f32) -> String {
    let total = secs.max(0.0) as u32;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn parse_ffmpeg_duration(text: &str) -> Option<f32> {
    let rest = text.split("Duration:").nth(1)?;
    let stamp = rest.trim().split([',', ' ']).next()?.trim();
    let mut parts = stamp.split(':');
    let h: f32 = parts.next()?.parse().ok()?;
    let m: f32 = parts.next()?.parse().ok()?;
    let s: f32 = parts.next()?.parse().ok()?;
    let secs = h * 3600.0 + m * 60.0 + s;
    (secs.is_finite() && secs > 0.0).then_some(secs)
}

fn probe_media_len(path: &Path) -> Option<f32> {
    if let Some(bin) = crate::sys::which("ffprobe") {
        if let Ok(out) = std::process::Command::new(bin)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Ok(secs) = text.trim().parse::<f32>() {
                if secs.is_finite() && secs > 0.05 {
                    return Some(secs);
                }
            }
        }
    }
    let bin = crate::sys::ffmpeg_bin()?;
    let out = std::process::Command::new(bin)
        .args(["-hide_banner", "-i"])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    parse_ffmpeg_duration(&text)
}

fn refill_shuffle(bag: &mut Vec<usize>, playable: &[usize], current: usize) -> bool {
    bag.retain(|i| playable.contains(i) && *i != current);
    if !bag.is_empty() {
        return false;
    }
    *bag = playable.iter().copied().filter(|i| *i != current).collect();
    true
}

fn draw_shuffle(bag: &mut Vec<usize>, playable: &[usize], current: usize) -> Option<usize> {
    if !playable.iter().any(|i| *i != current) {
        return None;
    }
    if refill_shuffle(bag, playable, current) {
        bag.shuffle(&mut rand::thread_rng());
    }
    bag.pop()
}

fn save_saved(root: &Path, mixes: &[SavedMix]) {
    if let Ok(s) = serde_json::to_string_pretty(mixes) {
        let _ = std::fs::write(root.join("data/saved-mixes.json"), s);
    }
}

fn kill_child(child: &mut Option<std::process::Child>) {
    if let Some(mut child) = child.take() {
        let _ = child.kill();
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

fn take_exit(child: &mut Option<std::process::Child>) -> Option<std::process::ExitStatus> {
    let status = child.as_mut()?.try_wait().ok()??;
    *child = None;
    Some(status)
}

fn film_video_args(sec: &str, path: &Path) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-ss".into(),
        sec.into(),
        "-re".into(),
        "-i".into(),
        path.display().to_string(),
        "-an".into(),
        "-vf".into(),
        "scale=960:540:force_original_aspect_ratio=decrease,pad=960:540:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=24".into(),
        "-pix_fmt".into(),
        "rgb24".into(),
        "-f".into(),
        "rawvideo".into(),
        "pipe:1".into(),
    ]
}

fn film_audio_args(sec: &str, path: &Path) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-ss".into(),
        sec.into(),
        "-re".into(),
        "-i".into(),
        path.display().to_string(),
        "-vn".into(),
        "-ac".into(),
        "1".into(),
        "-ar".into(),
        "48000".into(),
        "-f".into(),
        "f32le".into(),
        "pipe:1".into(),
    ]
}

fn spawn_ffmpeg(bin: &Path, args: &[String]) -> Result<std::process::Child, String> {
    let mut cmd = std::process::Command::new(bin);
    crate::sys::hide(&mut cmd);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    cmd.spawn().map_err(|_| "ffmpeg did not start.".to_string())
}

fn read_full(reader: &mut impl std::io::Read, buf: &mut [u8]) -> bool {
    let mut got = 0;
    while got < buf.len() {
        match reader.read(&mut buf[got..]) {
            Ok(0) => return false,
            Ok(n) => got += n,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return false,
        }
    }
    true
}

fn pcm_le(buf: &[u8]) -> Vec<f32> {
    buf.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn pump_film_frames(mut stdout: impl std::io::Read + Send, slot: std::sync::Arc<FilmSlot>) {
    let mut buf = vec![0u8; FILM_BYTES];
    let mut n = 0u64;
    loop {
        if !read_full(&mut stdout, &mut buf) {
            break;
        }
        n += 1;
        if let Ok(mut guard) = slot.rgb.lock() {
            let spare = guard
                .take()
                .map(|(_, old)| old)
                .filter(|old| old.len() == FILM_BYTES);
            let frame = std::mem::replace(&mut buf, spare.unwrap_or_else(|| vec![0u8; FILM_BYTES]));
            *guard = Some((n, frame));
        }
    }
}

fn pump_film_audio(mut stdout: impl std::io::Read + Send, tx: std::sync::mpsc::SyncSender<Vec<f32>>) {
    let mut buf = [0u8; 4096 * 4];
    loop {
        let mut got = 0;
        let mut end = false;
        while got < buf.len() {
            match stdout.read(&mut buf[got..]) {
                Ok(0) => {
                    end = true;
                    break;
                }
                Ok(n) => got += n,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return,
            }
        }
        let n = got - (got % 4);
        if n >= 4 && tx.send(pcm_le(&buf[..n])).is_err() {
            return;
        }
        if end {
            return;
        }
    }
}

fn edit_copy_path(src: &Path) -> PathBuf {
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("picture");
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
    for n in 0..100 {
        let name = if n == 0 {
            format!("{stem}-edit.{ext}")
        } else {
            format!("{stem}-edit-{n}.{ext}")
        };
        let path = parent.join(name);
        if !path.exists() {
            return path;
        }
    }
    parent.join(format!("{stem}-edit-now.{ext}"))
}

fn bake_picture(src: &Path, edit: &PicEdit, dest: Option<&Path>) -> Result<Vec<u8>, String> {
    let img = image::open(src).map_err(|_| "The picture did not open.".to_string())?;
    let img = apply_edit(img, edit);
    if let Some(dest) = dest {
        write_picture(&img, dest)?;
        return Ok(Vec::new());
    }
    let preview = fit_preview(img);
    let rgb = preview.to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|_| "The picture did not open.".to_string())?;
    Ok(out)
}

fn apply_edit(mut img: image::DynamicImage, edit: &PicEdit) -> image::DynamicImage {
    img = match edit.turns % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img,
    };
    if edit.flip {
        img = img.fliph();
    }
    img = crop_edges(img, edit);
    if edit.bright != 0 {
        img = img.brighten(edit.bright);
    }
    if edit.contrast != 0 {
        let rgb = img.to_rgb8();
        img = image::DynamicImage::ImageRgb8(image::imageops::contrast(&rgb, edit.contrast as f32));
    }
    img
}

fn crop_edges(img: image::DynamicImage, edit: &PicEdit) -> image::DynamicImage {
    let left = edit.left.clamp(0, 45) as u32;
    let top = edit.top.clamp(0, 45) as u32;
    let right = edit.right.clamp(0, 45) as u32;
    let bottom = edit.bottom.clamp(0, 45) as u32;
    if left + top + right + bottom == 0 {
        return img;
    }
    let w = img.width();
    let h = img.height();
    let x = w * left / 100;
    let y = h * top / 100;
    let x2 = w * right / 100;
    let y2 = h * bottom / 100;
    let nw = w.saturating_sub(x + x2).max(1);
    let nh = h.saturating_sub(y + y2).max(1);
    let x = x.min(w.saturating_sub(nw));
    let y = y.min(h.saturating_sub(nh));
    img.crop_imm(x, y, nw, nh)
}

fn fit_preview(img: image::DynamicImage) -> image::DynamicImage {
    if img.width() <= 1280 && img.height() <= 1280 {
        img
    } else {
        img.resize(1280, 1280, image::imageops::FilterType::Triangle)
    }
}

fn write_picture(img: &image::DynamicImage, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ext = dest
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let fail = "The picture did not save.";
    match ext.as_str() {
        "png" => img.save(dest).map_err(|_| fail.to_string()),
        "webp" => {
            let mut file = std::fs::File::create(dest).map_err(|_| fail.to_string())?;
            let rgba = img.to_rgba8();
            image::codecs::webp::WebPEncoder::new_lossless(&mut file)
                .encode(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|_| fail.to_string())
        }
        _ => {
            let mut file = std::fs::File::create(dest).map_err(|_| fail.to_string())?;
            let rgb = img.to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90)
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|_| fail.to_string())
        }
    }
}

fn recv_is_quiet(elapsed: Duration) -> bool {
    elapsed >= RECV_QUIET
}

fn boot_open(elapsed: f32, ready: bool, skip: bool) -> bool {
    if !ready {
        return false;
    }
    if skip && elapsed > BOOT_SKIP {
        return true;
    }
    elapsed >= BOOT_HOLD
}

fn boot_frac(done: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (done as f32 / total as f32).clamp(0.0, 1.0)
}

fn boot_line(step: &str) -> &'static str {
    match step {
        "catalog" => "catalog    mix, places, scenes",
        "devices" => "devices    mics and speakers",
        "mixer" => "mixer      local sound",
        "records" => "records    sheets, notes, pages",
        "city" => "city       netspace",
        _ => "load       working",
    }
}

enum BootMsg {
    Step(&'static str),
    NeedMixer(String),
    Desk(Desk),
    Failed(String),
}

pub struct Launch {
    root: PathBuf,
    boot_at: Instant,
    skip: bool,
    steps: Vec<&'static str>,
    err: String,
    mixer: Option<Mixer>,
    app: Option<Blightnet>,
    rx: Receiver<BootMsg>,
    visuals: bool,
}

impl Launch {
    pub fn start(root: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let folder = root.clone();
        std::thread::spawn(move || {
            let tx_note = tx.clone();
            let loaded = Blightnet::prepare(folder, move |msg| {
                let _ = tx_note.send(msg);
            });
            match loaded {
                Ok(desk) => {
                    let _ = tx.send(BootMsg::Desk(desk));
                }
                Err(err) => {
                    let _ = tx.send(BootMsg::Failed(err));
                }
            }
        });
        Self {
            root,
            boot_at: Instant::now(),
            skip: false,
            steps: Vec::new(),
            err: String::new(),
            mixer: None,
            app: None,
            rx,
            visuals: false,
        }
    }

    fn open_mixer(&mut self, speaker: String) {
        if self.mixer.is_some() || !self.err.is_empty() {
            return;
        }
        let chosen = if speaker.is_empty() { None } else { Some(speaker.as_str()) };
        match Mixer::with_output(self.root.clone(), chosen).or_else(|_| Mixer::new(self.root.clone()))
        {
            Ok(mixer) => {
                self.mixer = Some(mixer);
                if !self.steps.contains(&"mixer") {
                    self.steps.push("mixer");
                }
            }
            Err(err) => self.err = err,
        }
    }

    fn drain(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(BootMsg::Step(step)) => {
                    if !self.steps.contains(&step) {
                        self.steps.push(step);
                    }
                }
                Ok(BootMsg::NeedMixer(speaker)) => self.open_mixer(speaker),
                Ok(BootMsg::Desk(desk)) => {
                    if self.mixer.is_none() {
                        self.open_mixer(desk.speaker_name.clone());
                    }
                    if self.err.is_empty() {
                        if let Some(mixer) = self.mixer.take() {
                            match Blightnet::from_parts(desk, mixer) {
                                Ok(mut app) => {
                                    app.page = Page::Index;
                                    self.app = Some(app);
                                }
                                Err(err) => self.err = err,
                            }
                        }
                    }
                }
                Ok(BootMsg::Failed(err)) => {
                    self.err = err;
                    if self.app.is_none() {
                        self.mixer = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.app.is_none() && self.err.is_empty() {
                        self.err = "The load stopped.".into();
                    }
                    if self.app.is_none() {
                        self.mixer = None;
                    }
                    break;
                }
            }
        }
    }

    fn paint(&self, ctx: &egui::Context) {
        let ready = self.app.is_some();
        let frac = if ready {
            1.0
        } else {
            boot_frac(self.steps.len() as u32, BOOT_STEPS)
        };
        let t = self.boot_at.elapsed().as_secs_f32();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, theme::BG);
                theme::holo_grid(ui, r);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(1.0, theme::HOT),
                    egui::StrokeKind::Inside,
                );
                theme::hud_ticks(ui, r.shrink(10.0), CYAN, 16.0);
                let plate = Rect::from_center_size(
                    egui::pos2(r.center().x, r.center().y - 8.0),
                    Vec2::new((r.width() - 80.0).clamp(420.0, 680.0), 380.0),
                );
                theme::holo_frame(ui, plate, 12.0);
                let anchor = plate.center();
                ui.painter().text(
                    anchor + Vec2::new(0.0, -140.0),
                    egui::Align2::CENTER_CENTER,
                    "BLIGHTNET",
                    FontId::new(42.0, theme::display()),
                    theme::ACID,
                );
                ui.painter().rect_filled(
                    Rect::from_center_size(anchor + Vec2::new(0.0, -112.0), Vec2::new(120.0, 2.0)),
                    0.0,
                    CYAN,
                );
                ui.painter().text(
                    anchor + Vec2::new(0.0, -90.0),
                    egui::Align2::CENTER_CENTER,
                    "LOCAL NODE",
                    FontId::new(13.0, theme::mono()),
                    CYAN,
                );
                for (i, step) in self.steps.iter().enumerate() {
                    let y = anchor.y - 58.0 + i as f32 * 22.0;
                    ui.painter().text(
                        egui::pos2(anchor.x - 220.0, y),
                        egui::Align2::LEFT_TOP,
                        "ok",
                        FontId::new(14.0, theme::mono()),
                        CYAN,
                    );
                    ui.painter().text(
                        egui::pos2(anchor.x - 184.0, y),
                        egui::Align2::LEFT_TOP,
                        boot_line(step),
                        FontId::new(14.0, theme::mono()),
                        CREAM,
                    );
                }
                ui.painter().text(
                    egui::pos2(
                        anchor.x - 220.0,
                        anchor.y - 58.0 + self.steps.len() as f32 * 22.0,
                    ),
                    egui::Align2::LEFT_TOP,
                    if ready {
                        "ok   node       waiting · press Online"
                    } else {
                        "     node       waiting · press Online"
                    },
                    FontId::new(14.0, theme::mono()),
                    if ready { theme::ACID } else { DIM },
                );
                if !self.err.is_empty() {
                    ui.painter().text(
                        egui::pos2(anchor.x, plate.bottom() - 86.0),
                        egui::Align2::CENTER_CENTER,
                        &self.err,
                        FontId::new(13.0, theme::mono()),
                        KILL,
                    );
                }
                let track = Rect::from_center_size(
                    egui::pos2(anchor.x, plate.bottom() - 52.0),
                    Vec2::new(440.0, 4.0),
                );
                ui.painter()
                    .rect_filled(track, 0.0, theme::fade(KILL, 110));
                let mut fill = track;
                fill.max.x = track.left() + track.width() * frac;
                ui.painter().rect_filled(fill, 0.0, CYAN);
                if frac > 0.0 {
                    let scan = r.top() + frac * (r.bottom() - r.top() - 2.0);
                    ui.painter().hline(
                        r.x_range(),
                        scan,
                        egui::Stroke::new(8.0, Color32::from_rgba_unmultiplied(77, 232, 255, 28)),
                    );
                    ui.painter().hline(r.x_range(), scan, egui::Stroke::new(1.0, CYAN));
                }
                let hint = if !self.err.is_empty() {
                    "THE LOAD STOPPED"
                } else if ready {
                    "OPENING"
                } else if self.skip || t <= BOOT_SKIP {
                    "LOADING"
                } else {
                    "CLICK OR PRESS ANY KEY TO SKIP"
                };
                ui.painter().text(
                    egui::pos2(anchor.x, plate.bottom() - 28.0),
                    egui::Align2::CENTER_CENTER,
                    hint,
                    FontId::new(12.0, theme::mono()),
                    DIM,
                );
            });
    }
}

impl eframe::App for Launch {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.drain();
        let t = self.boot_at.elapsed().as_secs_f32();
        let pressed = t > BOOT_SKIP
            && ctx.input(|i| {
                i.pointer.any_click()
                    || i.events.iter().any(|e| {
                        matches!(
                            e,
                            egui::Event::Key { pressed: true, .. }
                                | egui::Event::PointerButton { pressed: true, .. }
                        )
                    })
            });
        if pressed {
            self.skip = true;
        }
        if self.err.is_empty() && boot_open(t, self.app.is_some(), self.skip) {
            if let Some(app) = self.app.as_mut() {
                app.update(ctx, frame);
            }
            return;
        }
        if !self.visuals {
            ctx.set_visuals(theme::visuals());
            ctx.style_mut(|s| {
                s.interaction.tooltip_delay = 0.08;
                s.interaction.tooltip_grace_time = 0.12;
            });
            self.visuals = true;
        }
        ctx.request_repaint_after(FRAME);
        self.paint(ctx);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn path_health_flags_reads_status_string() {
        let s = "Hosting · path health · UPnP TCP Y · UPnP UDP N · public IP Y · udp mesh Y · UPnP lease best-effort (reachability only, not trust)";
        assert_eq!(super::path_health_flags(s), Some((true, false, true, true)));
        assert_eq!(super::path_health_flags("Hosting · local network"), None);
        let (v, _) = super::session_path_readout(true, super::Role::Host, false, "Hosting · local network", "");
        assert_eq!(v, "LAN");
        let (v, _) = super::session_path_readout(false, super::Role::Host, true, s, "");
        assert_eq!(v, "—");
        let (v, _) = super::session_path_readout(true, super::Role::Host, true, "Hosting · invite ready", s);
        assert!(v.contains("UPnP Y/N"), "{v}");
    }

    use super::*;


    #[test]
    fn mute_and_deaf_are_independent() {
        // Mute gates TX only; deaf gates RX only.
        assert!(voice_tx_allowed(true, false, true));
        assert!(!voice_tx_allowed(true, true, true));
        assert!(!voice_tx_allowed(false, false, true));
        assert!(!voice_tx_allowed(true, false, false));
        assert!(voice_rx_allowed(false, false));
        assert!(!voice_rx_allowed(false, true));
        assert!(!voice_rx_allowed(true, false));
        // Mute on + deaf off ⇒ no TX, still hear.
        assert!(!voice_tx_allowed(true, true, true));
        assert!(voice_rx_allowed(false, false));
        // Mute off + deaf on ⇒ TX ok, no RX.
        assert!(voice_tx_allowed(true, false, true));
        assert!(!voice_rx_allowed(false, true));
    }

    #[test]
    fn blackjack_totals_and_labels() {
        assert_eq!(total(&[0, 10]), 21);
        assert_eq!(total(&[0, 0, 8]), 21);
        assert_eq!(total(&[0, 0, 0, 10]), 13);
        assert_eq!(total(&[12, 11]), 20);
        assert_eq!(card_label(0), "A");
        assert_eq!(card_label(10), "J");
        assert_eq!(card_label(12), "K");
        assert_eq!(card_label(6), "7");
        for _ in 0..40 {
            let c = card();
            assert!(c < 52);
        }
    }

    #[test]
    fn clock_wraps_and_maps_periods() {
        assert_eq!(period_from_clock(0), "night");
        assert_eq!(period_from_clock(7 * 60), "morning");
        assert_eq!(period_from_clock(13 * 60), "day");
        assert_eq!(period_from_clock(19 * 60), "evening");
        assert_eq!(period_from_clock(22 * 60), "night");
        assert_eq!(format_clock(13 * 60 + 5), "13:05");
        assert_eq!(clock_from_period("night"), 23 * 60);
        for p in HOURS {
            assert_eq!(period_from_clock(clock_from_period(p)), p);
        }
    }

    #[test]
    fn parse_coins_reads_leading_digits() {
        assert_eq!(parse_coins("15 gp"), 15);
        assert_eq!(parse_coins("500 eb"), 500);
        assert_eq!(parse_coins("1 sp"), 1);
        assert_eq!(parse_coins("free"), 0);
    }

    #[test]
    fn media_tags_and_sniff() {
        assert_eq!(media_tag("image/jpeg"), "img");
        assert_eq!(media_tag("audio/wav"), "aud");
        assert_eq!(media_tag("video/mp4"), "vid");
        assert_eq!(media_tag("application/zip"), "file");
        assert_eq!(sniff_ext(b"OggS...."), "ogg");
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0, 0, 0, 0]);
        wav.extend_from_slice(b"WAVE");
        assert_eq!(sniff_ext(&wav), "wav");
        assert_eq!(mime_of(Path::new("clip.webm")), "video/webm");
        assert_eq!(mime_of(Path::new("note.wav")), "audio/wav");
    }

    #[test]
    fn deck_library_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bn-deck-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("tune.ogg");
        std::fs::write(&a, b"not really audio").unwrap();
        save_deck_lib(&dir, &[a.clone()]);
        let loaded = load_deck_lib(&dir);
        assert_eq!(loaded, vec![a]);
        save_deck_vol(&dir, 0.42);
        assert!((load_deck_vol(&dir).unwrap() - 0.42).abs() < 0.01);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chat_persists_until_wipe() {
        let dir = std::env::temp_dir().join(format!("bn-chat-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        save_chat(&dir, &["hello".into(), "there".into()]);
        let loaded = load_chat(&dir).unwrap();
        assert_eq!(loaded, vec!["hello", "there"]);
        save_chat(&dir, &[]);
        assert_eq!(load_chat(&dir).unwrap().len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn downsample_48000_to_16000() {
        let s: Vec<f32> = (0..48000).map(|i| i as f32).collect();
        let d = downsample(&s, 48000, 16000);
        assert!((d.len() as i32 - 16000).abs() <= 2);
        assert_eq!(downsample(&s, 8000, 16000).len(), s.len());
        let d44 = downsample(&s[..44100], 44100, 16000);
        assert!((d44.len() as i32 - 16000).abs() < 400);
    }

    #[test]
    fn overlay_toggle_and_radio_roster() {
        let radio = crate::stations::all();
        assert!(radio.len() >= 40);
        assert!(radio.iter().any(|s| s.id == "rebellious" && s.call == "RIOT" && s.url.is_none()));
        assert!(radio.iter().any(|s| s.id == "brutal" && s.call == "RAVE"));
        assert!(radio.iter().any(|s| s.id == "afterlife"));
        assert!(radio.iter().any(|s| s.id == "soma-groove" && s.url.is_some()));
        assert!(radio.iter().any(|s| s.id == "nr-main"));
        assert_ne!(Overlay::Log, Overlay::Chars);
        assert_ne!(Overlay::Log, Overlay::Maps);
        assert_ne!(Overlay::Notes, Overlay::Log);
        assert_eq!(MAP_WIRE, "__table-map");
        let mut c = Character::new("hearthsong");
        let a = sheet_sig(&[c.clone()]);
        c.hp = 3;
        assert_ne!(a, sheet_sig(&[c]));
        assert!(load_changelog(&PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .iter()
            .any(|(t, _)| t.contains("2026")));
        let days: Vec<_> = load_changelog(&PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .into_iter()
            .map(|(d, _)| d)
            .collect();
        let mut seen = std::collections::HashSet::new();
        for d in &days {
            assert!(seen.insert(d.clone()), "duplicate changelog day {d}");
            assert!(!d.contains('—') && !d.contains("Sharper"), "{d}");
        }
    }

    #[test]
    fn changelog_merges_split_headings_for_one_day() {
        let dir = std::env::temp_dir().join(format!("bn-log-{}", rand::random::<u32>()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(
            dir.join("data/changelog.md"),
            "## 2026-09-19 — First\n- alpha\n\n## 2026-09-19 — Second\n- beta\n- alpha\n\n## 2026-09-18\n- old\n",
        )
        .unwrap();
        let log = load_changelog(&dir);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].0, "2026-09-19");
        assert_eq!(log[0].1, vec!["alpha", "beta"]);
        assert_eq!(log[1].0, "2026-09-18");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_downloads_without_a_git_checkout_and_past_local_edits() {
        let base = std::env::temp_dir().join(format!("bn-upd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let bare = base.join("bare");
        let seed = base.join("seed");
        let work = base.join("work");
        let fresh = base.join("fresh");
        let run = |dir: &Path, args: &[&str]| {
            let out = std::process::Command::new("git")
                .current_dir(dir)
                .args(args)
                .env("GIT_AUTHOR_NAME", "Ada")
                .env("GIT_AUTHOR_EMAIL", "ada@example.com")
                .env("GIT_COMMITTER_NAME", "Ada")
                .env("GIT_COMMITTER_EMAIL", "ada@example.com")
                .output()
                .expect("git");
            assert!(
                out.status.success(),
                "{} {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        };
        std::fs::create_dir_all(&bare).unwrap();
        run(&bare, &["init", "--bare", "-b", "main"]);
        std::fs::create_dir_all(seed.join("data")).unwrap();
        std::fs::write(seed.join("data/mixer-catalog.json"), "{}").unwrap();
        std::fs::write(seed.join("hello.txt"), "v1").unwrap();
        run(&seed, &["init", "-b", "main"]);
        run(&seed, &["add", "."]);
        run(&seed, &["commit", "-m", "v1"]);
        run(&seed, &["remote", "add", "origin", bare.to_str().unwrap()]);
        run(&seed, &["push", "origin", "main"]);
        run(&base, &["clone", bare.to_str().unwrap(), "work"]);
        std::fs::write(seed.join("hello.txt"), "v2").unwrap();
        run(&seed, &["add", "hello.txt"]);
        run(&seed, &["commit", "-m", "v2"]);
        run(&seed, &["push", "origin", "main"]);
        std::fs::write(work.join("hello.txt"), "local edit").unwrap();
        let url = bare.to_str().unwrap();
        let updated = github_update_from(&work, url).expect("update behind a local edit");
        assert!(
            updated.contains("v2") || updated.contains("Fast-forward") || updated.contains("stash") || updated.contains("Updating"),
            "{updated}"
        );
        let head = std::process::Command::new("git")
            .current_dir(&work)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let want = std::process::Command::new("git")
            .current_dir(&seed)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert_eq!(head.stdout, want.stdout, "{updated}");
        std::fs::create_dir_all(fresh.join("data")).unwrap();
        std::fs::write(fresh.join("data/mixer-catalog.json"), "{}").unwrap();
        github_update_from(&fresh, url).expect("update a folder that is not a git repo");
        let hello = std::fs::read_to_string(fresh.join("hello.txt")).unwrap_or_default();
        assert_eq!(hello, "v2", "fresh download did not get the file");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn init_roster_includes_pcs_and_token_sheets_sorted() {
        let mut pc = Character::new("hearthsong");
        pc.id = "pc1".into();
        pc.name = "Ada".into();
        pc.npc = false;
        let mut npc = Character::new("blight");
        npc.id = "npc1".into();
        npc.name = "Boost".into();
        npc.npc = true;
        npc.init = 7;
        let toks = vec![maps::MapTok {
            id: "t1".into(),
            name: "Boost".into(),
            x: 0.5,
            y: 0.5,
            size: 0.08,
            image: String::new(),
            sheet: "npc1".into(),
        }];
        let mut rows = build_init_roster(&[pc, npc], &toks);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.id == "pc1"));
        assert!(rows.iter().any(|r| r.id == "npc1"));
        rows[0].score = 5;
        rows[1].score = 12;
        sort_init_roster(&mut rows);
        assert_eq!(rows[0].score, 12);
        assert_eq!(rows[1].score, 5);
    }

    #[test]
    fn init_wire_pack_unpack_preserves_order_and_turn() {
        let rows = vec![
            InitRow {
                id: "a".into(),
                name: "Ada".into(),
                score: 18,
                ready: true,
            },
            InitRow {
                id: "b".into(),
                name: "Boost".into(),
                score: 12,
                ready: true,
            },
        ];
        let body = pack_init(&rows, 1);
        let (got, turn) = unpack_init(&body).expect("init wire");
        assert_eq!(turn, 1);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id, "a");
        assert_eq!(got[0].score, 18);
        assert_eq!(got[1].name, "Boost");
        assert!(unpack_init("ask").is_none());
        let (empty, t0) = unpack_init("{\"rows\":[],\"turn\":9}").unwrap();
        assert!(empty.is_empty());
        assert_eq!(t0, 0);
    }

    #[test]
    fn resolve_roll_victim_prefers_aim_over_open_sheet() {
        assert_eq!(
            resolve_roll_victim(Some("victim"), Some("open"), "attacker"),
            "victim"
        );
        assert_eq!(
            resolve_roll_victim(Some("attacker"), Some("open"), "attacker"),
            "open"
        );
        assert_eq!(
            resolve_roll_victim(None, Some("open"), "attacker"),
            "open"
        );
        assert_eq!(
            resolve_roll_victim(Some("attacker"), Some("attacker"), "attacker"),
            ""
        );
        assert_eq!(resolve_roll_victim(None, None, "attacker"), "");
    }

    #[test]
    fn combat_mark_and_private_notes_stay_local() {
        assert!(COMBAT_MARK.starts_with('\u{2060}'));
        let line = format!("{COMBAT_MARK}Ada rolls 40");
        assert_eq!(line.strip_prefix(COMBAT_MARK), Some("Ada rolls 40"));
        let dir = std::env::temp_dir().join(format!("bn-notes-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("data"));
        save_notes(&dir, "Ada", "don't tell the table");
        assert_eq!(load_notes(&dir, "Ada"), "don't tell the table");
        assert!(load_notes(&dir, "Bob").is_empty());
        save_combat_log(&dir, &["hit 12".into()]);
        assert_eq!(load_combat_log(&dir), vec!["hit 12".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn catalog_art_paths_match_folders() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = catalog_art(&root, "Bestiary", "goblin");
        assert!(p.ends_with("assets/bestiary/goblin.jpg") || p.is_file());
        assert!(catalog_art(&root, "Datashard", "assassin").is_file()
            || catalog_art(&root, "Datashard", "assassin")
                .to_string_lossy()
                .contains("datashard"));
    }

    #[test]
    fn devices_and_crews_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bn-dev-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        save_devices(
            &dir,
            &DevicePref {
                mic: "mic1".into(),
                speaker: "spk1".into(),
                camera: "/dev/video0".into(),
            },
        );
        let d = load_devices(&dir);
        assert_eq!(d.mic, "mic1");
        assert_eq!(d.camera, "/dev/video0");
        save_crews(
            &dir,
            &[Crew {
                id: "crew-1".into(),
                name: "Aldecaldos".into(),
                members: vec!["a".into()],
            }],
        );
        let c = load_crews(&dir);
        assert_eq!(c[0].name, "Aldecaldos");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn film_args_keep_picture_and_sound() {
        let path = Path::new("clip.mp4");
        let video = film_video_args("1.25", path).join(" ");
        assert!(video.contains("-an"));
        assert!(video.contains("fps=24"));
        assert!(video.contains("rgb24"));
        assert!(video.contains("rawvideo"));
        assert!(!video.contains("image2"));
        assert!(!video.contains("fps=4"));
        let audio = film_audio_args("1.25", path).join(" ");
        assert!(audio.contains("-vn"));
        assert!(audio.contains("f32le"));
        assert!(audio.contains("48000"));
        assert!(!audio.contains(" -an"));
    }

    #[test]
    fn picture_rotate_and_crop_save() {
        let dir = std::env::temp_dir().join(format!("bn-pic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("shot.png");
        image::RgbImage::from_fn(20, 10, |x, y| image::Rgb([x as u8, y as u8, 40]))
            .save(&src)
            .unwrap();
        let edit = PicEdit {
            turns: 1,
            ..PicEdit::default()
        };
        let dest = dir.join("turned.jpg");
        bake_picture(&src, &edit, Some(&dest)).unwrap();
        let out = image::open(&dest).unwrap();
        assert_eq!((out.width(), out.height()), (10, 20));
        let wide = dir.join("wide.png");
        image::RgbImage::from_pixel(100, 40, image::Rgb([8, 8, 8]))
            .save(&wide)
            .unwrap();
        let crop = PicEdit {
            left: 50,
            ..PicEdit::default()
        };
        let cropped = dir.join("crop.png");
        bake_picture(&wide, &crop, Some(&cropped)).unwrap();
        let out = image::open(&cropped).unwrap();
        assert_eq!((out.width(), out.height()), (55, 40));
        assert_eq!(
            edit_copy_path(&src).file_name().unwrap().to_str(),
            Some("shot-edit.png")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn film_pipe_carries_a_frame_and_sound() {
        let Some(bin) = crate::sys::ffmpeg_bin() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("bn-film-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("clip.mp4");
        let make = std::process::Command::new(&bin)
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x120:rate=24:duration=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=1",
                "-shortest",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&clip)
            .output()
            .expect("ffmpeg");
        if !make.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        let mut video = spawn_ffmpeg(&bin, &film_video_args("0", &clip)).unwrap();
        let mut audio = spawn_ffmpeg(&bin, &film_audio_args("0", &clip)).unwrap();
        let mut vout = video.stdout.take().unwrap();
        let mut aout = audio.stdout.take().unwrap();
        let (vtx, vrx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; FILM_BYTES];
            let ok = read_full(&mut vout, &mut buf);
            let _ = vtx.send((ok, buf));
        });
        let frame = vrx.recv_timeout(std::time::Duration::from_secs(12));
        let (atx, arx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let n = std::io::Read::read(&mut aout, &mut buf).unwrap_or(0);
            let _ = atx.send((n, buf));
        });
        let audio_got = arx.recv_timeout(std::time::Duration::from_secs(12));
        let _ = video.kill();
        let _ = audio.kill();
        let _ = video.wait();
        let _ = audio.wait();
        let _ = std::fs::remove_dir_all(&dir);
        let (ok, buf) = frame.expect("timed out waiting for a frame");
        assert!(ok, "the frame was short");
        assert_eq!(buf.len(), FILM_BYTES);
        assert!(buf.iter().any(|b| *b != 0), "the frame was blank");
        let (n, samples) = audio_got.expect("timed out waiting for sound");
        assert!(n >= 4, "no sound bytes");
        let pcm = pcm_le(&samples[..n - (n % 4)]);
        assert!(pcm.iter().any(|s| s.abs() > 0.01), "the sound was silent");
    }

    #[test]
    fn transfer_clock_estimates_the_rest() {
        assert!(transfer_clock(1000, 10_000, 0.1).is_none());
        let (about, left) = transfer_clock(50_000_000, 100_000_000, 2.0).unwrap();
        assert!((left - 2.0).abs() < 0.01);
        assert!((about - 4.0).abs() < 0.01);
        let line = transfer_line("Receiving clip.mp4", 50_000_000, 100_000_000, 2.0);
        assert!(line.contains("0:02 left"));
        assert!(line.contains("About 0:04"));
    }

    #[test]
    fn media_clock_and_duration_parse() {
        assert_eq!(fmt_media(5.0), "0:05");
        assert_eq!(fmt_media(62.2), "1:02");
        assert_eq!(fmt_media(3661.0), "1:01:01");
        assert_eq!(
            parse_ffmpeg_duration("Duration: 00:01:02.50, start: 0.000000"),
            Some(62.5)
        );
        assert!(parse_ffmpeg_duration("no clock here").is_none());
    }

    #[test]
    fn a_quiet_download_is_dropped() {
        assert!(!recv_is_quiet(Duration::from_secs(19)));
        assert!(recv_is_quiet(Duration::from_secs(20)));
        assert!(recv_is_quiet(Duration::from_secs(21)));
    }

    #[test]
    fn boot_waits_for_the_load() {
        assert!(!boot_open(2.0, false, true));
        assert!(!boot_open(0.1, true, true));
        assert!(boot_open(0.3, true, true));
        assert!(!boot_open(1.0, true, false));
        assert!(boot_open(1.6, true, false));
        assert_eq!(boot_frac(0, 5), 0.0);
        assert!((boot_frac(2, 5) - 0.4).abs() < 0.001);
        assert_eq!(boot_frac(5, 5), 1.0);
    }

    #[test]
    fn tour_opens_each_part_of_the_desk() {
        assert_eq!(TOUR.len(), 31);
        assert_eq!(TOUR.first().map(|s| s.title), Some("INDEX"));
        assert_eq!(TOUR.last().map(|s| s.title), Some("ROTN"));
        let mut seen = std::collections::HashSet::new();
        for step in TOUR {
            assert!(seen.insert(step.title), "{}", step.title);
            assert!(!step.body.is_empty());
        }
        let host = TOUR.iter().find(|s| s.title == "HOST").unwrap();
        assert!(host.page == Page::Index);
        assert!(host.dock == ShellPanel::Host);
        let maps = TOUR.iter().find(|s| s.title == "MAPS").unwrap();
        assert!(maps.page == Page::Table);
        assert!(maps.open == TourOpen::Panel(Overlay::Maps));
        let player = TOUR.iter().find(|s| s.title == "PLAYER").unwrap();
        assert!(player.page == Page::Player);
        assert!(player.dock == ShellPanel::None);
        assert!(TOUR.iter().any(|s| s.page == Page::Tree && s.title == "TREE"));
        assert!(TOUR.iter().any(|s| s.page == Page::Terminal));
        assert!(TOUR.iter().any(|s| s.page == Page::Recon));
        assert!(TOUR.iter().any(|s| s.page == Page::Audio));
        assert!(TOUR.iter().any(|s| matches!(s.open, TourOpen::Book)));
        assert!(TOUR.iter().any(|s| matches!(s.open, TourOpen::Seat)));
        assert!(TOUR.iter().any(|s| matches!(s.open, TourOpen::Games)));
        assert!(TOUR.iter().any(|s| s.dock == ShellPanel::Join));
        assert!(TOUR.iter().any(|s| s.dock == ShellPanel::Chat));
        assert!(TOUR.iter().any(|s| s.open == TourOpen::Panel(Overlay::Board)));
    }

    #[test]
    fn shuffle_bag_uses_every_other_file_before_repeating() {
        let playable = [0usize, 1, 2];
        let mut bag = Vec::new();
        assert!(refill_shuffle(&mut bag, &playable, 0));
        bag.sort();
        assert_eq!(bag, vec![1, 2]);
        let first = bag.pop().unwrap();
        let second = bag.pop().unwrap();
        assert_ne!(first, second);
        assert!(refill_shuffle(&mut bag, &playable, second));
        assert!(!bag.contains(&second));
        assert!(bag.contains(&0));
        assert!(draw_shuffle(&mut Vec::new(), &[4], 4).is_none());
    }
}
