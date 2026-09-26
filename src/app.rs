use crate::audio::Mixer;
use crate::catalog::{load_list, Catalog, Layer};
use crate::chars::{Character, KitSpec};
use crate::dice::{Luck, Roll};
use crate::images::{self, TexCache};
use crate::maps::{self, MapBoard, MapOp, TokenSpec};
use crate::names::Names;
use crate::net::{self, Contact, NetEvent, NetHub, Role};
use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED, PANEL, RAIL};
use eframe::egui::{self, Color32, FontId, Rect, RichText, Vec2};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Wake at most every 16.67ms. Do not sleep on the UI thread to enforce this.
const FRAME: Duration = Duration::from_nanos(16_666_667);
const COMBAT_MARK: &str = "\u{2060}C|";
const FILE_CAP: usize = 96 * 1024 * 1024;
const MAP_WIRE: &str = "__table-map";

struct MediaJob {
    gen: u64,
    ok: bool,
    msg: String,
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
}

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
    buf: Vec<u8>,
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
    boot_at: Instant,
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
    viewing: Option<String>,
    remote_chars: HashMap<String, Vec<Character>>,
    token_sig: u64,
    sheet_dirty: bool,
    sheet_at: Instant,
    mix_dirty: bool,
    mix_at: Instant,
    luck: Luck,
    roll: Option<Roll>,
    names: Names,
    jack_at: Instant,
    netspace: crate::netspace::Netspace,
    cameras: Vec<crate::video::Camera>,
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
    node_live: bool,
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

impl Blightnet {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let catalog = Catalog::load(&root)?;
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
        let mixer = Mixer::with_output(
            root.clone(),
            Some(speaker_name.as_str()).filter(|s| !s.is_empty()),
        )
        .or_else(|_| Mixer::new(root.clone()))?;
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
            boot_at: Instant::now(),
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
            chat: load_chat(&root).unwrap_or_else(|| {
                vec!["INDEX // stamp a Handle, then Host or Join, then JACK IN.".into()]
            }),
            chat_in: String::new(),
            chars: {
                let mut rows = crate::chars::load(&root);
                if rows.is_empty() {
                    let mut ada = Character::new("hearthsong");
                    ada.name = "Ada".into();
                    ada.hp = 12;
                    ada.hp_max = 12;
                    ada.gp = 80;
                    rows.push(ada);
                }
                rows
            },
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
            saved: load_saved(&root),
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
            vendor_stock: vec![],
            custom: vec![],
            kit_filter: String::new(),
            kit_focus: String::new(),
            tex: TexCache::default(),
            net: NetHub::new("Traveller".into(), &root),
            probe_n: 0,
            probe_at: Instant::now(),
            probe_sent: HashMap::new(),
            ping_ms: HashMap::new(),
            ping_at: HashMap::new(),
            contacts: net::load_contacts(&root),
            shell: ShellPanel::None,
            join_in: String::new(),
            voice_on: false,
            voice_mute: false,
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
            crews: load_crews(&root),
            crew_name: String::new(),
            chat_target: ChatTarget::Table,
            mic_name,
            speaker_name,
            inputs,
            outputs,
            inbox: load_inbox(&root),
            file_in: HashMap::new(),
            chat_saved: 0,
            is_gm: true,
            target: None,
            viewing: None,
            remote_chars: HashMap::new(),
            token_sig: 0,
            sheet_dirty: false,
            sheet_at: Instant::now(),
            mix_dirty: false,
            mix_at: Instant::now(),
            luck: Luck::Norm,
            roll: None,
            names: Names::load(&root),
            jack_at: Instant::now(),
            netspace: crate::netspace::Netspace::new(),
            cameras: vec![],
            cam_name: pref.camera.clone(),
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
            node_live: false,
            node_at: Instant::now(),
            clock_acc: 0.0,
            net_pos_at: Instant::now(),
            tour: None,
            mix_cache: vec![],
            mix_cache_key: String::new(),
            cat_cache: vec![],
            cat_cache_key: String::new(),
            devices_on: false,
            combat_log: load_combat_log(&root),
            notes: load_notes(&root, "Traveller"),
            notes_dirty: false,
            notes_at: Instant::now(),
            changelog: load_changelog(&root),
            zoom_path: None,
            zoom_key: None,
            rec_on: false,
            rec: vec![],
            nethooks: crate::nethook::load_all(&root),
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
            rotn: crate::rotn::Rotn::load(&root),
            chess: crate::chess::Game::new(),
            term: None,
            term_filter: String::new(),
            term_cmds: Vec::new(),
            share_pick: None,
            calc_acc: None,
            calc_op: None,
            calc_entry: "0".into(),
            calc_fresh: true,
            recon: crate::recon::load(&root),
            recon_i: 0,
            recon_q: String::new(),
            recon_arm: String::new(),
            cpu_tick: crate::sys::CpuTick::default(),
            machine: crate::sys::Machine::default(),
            meter_at: Instant::now() - Duration::from_secs(2),
        };
        app.chat_saved = app.chat.len();
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
        let Ok(bytes) = std::fs::read(&path) else {
            self.chat.push("Could not read that file.".into());
            return;
        };
        if bytes.len() > FILE_CAP {
            self.chat.push(format!(
                "That file is too large (max about {} MB).",
                FILE_CAP / 1_000_000
            ));
            return;
        }
        let mime = mime_of(&path);
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into());
        self.ingest_outgoing(mime, &safe_filename(&filename), bytes);
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
        self.chat.clear();
        self.file_in.clear();
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
                NetEvent::VoicePcm { from, samples } => {
                    if from != self.net.self_id && !self.voice_mute {
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
                    if size > FILE_CAP as u64 {
                        self.chat.push(format!("{name} sent a file that is too large."));
                        continue;
                    }
                    self.status = format!("Receiving {filename}…");
                    self.file_in.insert(
                        id,
                        FileIn {
                            from,
                            name,
                            mime,
                            filename,
                            size,
                            buf: Vec::with_capacity(size.min(8_000_000) as usize),
                        },
                    );
                }
                NetEvent::FileChunk { id, data } => {
                    if let Some(f) = self.file_in.get_mut(&id) {
                        if f.buf.len() + data.len() <= FILE_CAP {
                            f.buf.extend_from_slice(&data);
                        }
                    }
                }
                NetEvent::FileDone { id } => {
                    if let Some(f) = self.file_in.remove(&id) {
                        if f.filename.starts_with(MAP_WIRE) {
                            self.install_map_image(&f.filename, &f.buf);
                            continue;
                        }
                        if let Some(rest) = f.filename.strip_prefix("__hook|") {
                            if let Some((id, name)) = rest.split_once('|') {
                                if let Some(name) = crate::nethook::safe_file_name(name) {
                                    let dir = crate::nethook::file_dir(&self.root, id);
                                    let _ = std::fs::create_dir_all(&dir);
                                    let _ = std::fs::write(dir.join(name), &f.buf);
                                }
                            }
                            continue;
                        }
                        let tag = media_tag(&f.mime);
                        let key = id;
                        self.store_inbox(&key, &f.mime, &f.filename, &f.buf);
                        self.chat.push(format!(
                            "[{tag}:{key}|{}] {}",
                            f.filename,
                            media_caption(tag, &f.name)
                        ));
                        self.auto_stream(tag, &f.buf);
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
        if self.voice_on && self.voice_mute && live {
            self.net.send_pcm(&[0.0; 160]);
        }
        let talk = self.voice_on && !self.voice_mute && live;
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
            self.net.send_pcm(&down);
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

    fn ensure_devices(&mut self) {
        if self.devices_on {
            return;
        }
        self.devices_on = true;
        self.refresh_devices();
    }

    fn do_update(&mut self) {
        let root = self.root.clone();
        let out = std::process::Command::new("git")
            .args(["-C"])
            .arg(&root)
            .args(["pull", "--ff-only", "origin", "main"])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                self.chat.push(format!(
                    "Pulled from GitHub.\n{}\nRestart Blightnet to run the new build.",
                    text.trim()
                ));
                self.status = "Update pulled".into();
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                self.chat.push(format!("Git pull failed. {}", err.trim()));
            }
            Err(_) => {
                self.chat.push("Git is not on PATH. Install Git, then press UPDATE again.".into());
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
        if let Some(mut child) = self.media_child.take() {
            let _ = child.kill();
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }

    fn poll_media_proc(&mut self) {
        let finished = match self.media_child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => {
                    if self.media_run && !status.success() && self.media_msg.is_empty() {
                        self.media_msg = "The video stopped.".into();
                    }
                    self.media_run = false;
                    true
                }
                Ok(None) => false,
                Err(_) => {
                    self.media_run = false;
                    true
                }
            },
            None => false,
        };
        if finished {
            self.media_child = None;
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
        let Some(bin) = crate::sys::ffmpeg_bin() else {
            self.media_run = false;
            self.media_msg = "ffmpeg is not on this computer. The video can still open in the system player.".into();
            return;
        };
        let dest = self.root.join("data/media-frame.png");
        let _ = std::fs::create_dir_all(self.root.join("data"));
        self.tex.forget("media-stage");
        self.media_stamp = None;
        let sec = format!("{:.2}", self.media_offset.max(0.0));
        let mut cmd = std::process::Command::new(bin);
        crate::sys::hide(&mut cmd);
        cmd.arg("-y")
            .arg("-ss")
            .arg(sec)
            .arg("-re")
            .arg("-i")
            .arg(path)
            .arg("-an")
            .args(["-vf", "fps=4,scale=960:-2"])
            .args(["-f", "image2", "-update", "1"])
            .arg(&dest)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        match cmd.spawn() {
            Ok(child) => {
                self.media_child = Some(child);
                self.media_run = true;
                self.media_started = Instant::now();
                self.media_msg.clear();
            }
            Err(_) => {
                self.media_run = false;
                self.media_msg = "ffmpeg did not start.".into();
            }
        }
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
        self.poll_media_proc();
        let kind = self.current_kind();
        if kind == "video" || kind == "pdf" {
            self.refresh_media_frame(ctx);
        }
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
        if kind == "audio" {
            return;
        }
        if !self.media_msg.is_empty() {
            wrap_text(ui, &self.media_msg, CYAN, 12.0);
        }
        let max = Self::stage_size(ui, full);
        if kind == "image" {
            let path = p.clone();
            if let Some(tex) = self.tex.get(ui.ctx(), &path) {
                if paint_contain(ui, &tex, max).clicked() {
                    self.zoom_path = Some(path);
                }
            } else {
                let _ = images::show_fit(ui, &mut self.tex, &path, max);
            }
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
            if kind == "video" {
                self.start_video(&p);
            } else if kind == "pdf" {
                self.queue_pdf(&p);
            } else {
                self.stop_media_proc();
                self.media_run = false;
                self.media_rx = None;
            }
            return;
        }
        self.stop_media_proc();
        self.media_run = false;
        self.media_rx = None;
        self.media_msg.clear();
        match self.mixer.deck_play_path(&p) {
            Ok(()) => {
                self.deck_on = true;
                self.mixer.set_deck_vol(self.deck_vol);
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
            "image" | "pdf" => return,
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
        let keep = force || self.deck_on || self.media_run;
        self.deck_i = (self.deck_i + 1) % self.deck_list.len();
        if keep || self.current_kind() != "audio" {
            self.deck_play_current();
        }
        save_deck_lib(&self.root, &self.deck_list);
    }

    fn deck_prev(&mut self) {
        if self.deck_list.is_empty() {
            return;
        }
        let keep = self.deck_on || self.media_run;
        if self.deck_i == 0 {
            self.deck_i = self.deck_list.len() - 1;
        } else {
            self.deck_i -= 1;
        }
        if keep || self.current_kind() != "audio" {
            self.deck_play_current();
        }
        save_deck_lib(&self.root, &self.deck_list);
    }

    fn pump_deck(&mut self) {
        if self.deck_on && self.mixer.deck_done() && !self.deck_list.is_empty() {
            self.deck_next(true);
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

    fn set_camera(&mut self, path: String) {
        self.cam_name = path;
        self.persist_devices();
        if self.cam_on {
            self.start_cam();
        }
    }

    fn start_cam(&mut self) {
        self.cam_cap = None;
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
        match crate::video::start_screen() {
            Some(cap) => {
                self.screen_cap = Some(cap);
                self.screen_on = true;
            }
            None => {
                self.chat.push("Could not share the screen.".into());
                self.screen_on = false;
            }
        }
    }

    fn call_targets(&self) -> Vec<String> {
        let mut v = Vec::new();
        let mut push = |v: &mut Vec<String>, id: &str| {
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
        if self.cam_on {
            if let Some(cap) = self.cam_cap.as_ref() {
                if let Some(f) = cap.latest() {
                    self.local_cam = f;
                    if self.local_cam.len() <= 120_000 && !targets.is_empty() {
                        let crew = self.call_crew.clone();
                        for id in &targets {
                            self.net.send_video_frame("cam", Some(id.clone()), crew.clone(), &self.local_cam);
                        }
                    }
                }
            }
        }
        if self.screen_on {
            if let Some(cap) = self.screen_cap.as_ref() {
                if let Some(f) = cap.latest() {
                    self.local_screen = f;
                    if self.local_screen.len() <= 120_000 && !targets.is_empty() {
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
        self.tick_fps();
        self.poll_net();
        self.poll_meters();
        self.poll_probe();
        self.poll_radio();
        self.step_media(ctx);
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
        // Never sleep on this thread: Wayland frame callbacks would stall and
        // the boot screen would freeze. Cap rate with a delayed wake instead.
        ctx.request_repaint_after(FRAME);
        if self.page == Page::Boot {
            self.ui_boot(ctx);
            return;
        }
        self.ui_shell(ctx);
    }
}

impl Blightnet {
    fn ui_shell(&mut self, ctx: &egui::Context) {
        let t = ctx.input(|i| i.time) as f32;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ctx, |ui| {
                theme::scanlines(ui, ui.max_rect());
                let win = ui.max_rect();
                ui.painter().rect_filled(win, 0.0, theme::BG);
                ui.painter().rect_stroke(
                    win,
                    0.0,
                    egui::Stroke::new(1.0, theme::HOT),
                    egui::StrokeKind::Inside,
                );
                theme::hud_ticks(ui, win.shrink(8.0), CYAN, 12.0);
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
            Page::Table => "blightnet://blightnexus",
            Page::Tutorial => "blightnet://tutorial",
            Page::Audio => "blightnet://audio",
            Page::Blackjack => "blightnet://blackjack",
            Page::Nethooks => "blightnet://nethooks",
            Page::Netspace => "blightnet://netspace",
            Page::Rotn => "blightnet://rotn",
            Page::Player => "blightnet://player",
            Page::Terminal => "blightnet://terminal",
            Page::Recon => "blightnet://recon",
            _ => "blightnet://start",
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
        for (panel, label, tip) in [
            (
                ShellPanel::Player,
                "PLAYER",
                "Pictures, video, PDF, and music on this deck. Press again to close the side rail.",
            ),
            (
                ShellPanel::Video,
                "VIDEO",
                "Open video. Press again to close.",
            ),
            (
                ShellPanel::Voice,
                "VOICE",
                "Open the mic and calls. Press again to close.",
            ),
            (
                ShellPanel::Contacts,
                "CONTACTS",
                "People saved on this deck. Press again to close.",
            ),
            (
                ShellPanel::Chat,
                "CHAT",
                "Table talk, DMs, and files. Press again to close.",
            ),
        ] {
            let on = self.shell == panel;
            if tab(ui, label, on).on_hover_text(tip).clicked() {
                if on {
                    self.shell = ShellPanel::None;
                } else {
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

    fn draw_command_bar(&mut self, ui: &mut egui::Ui) {
        let h = 44.0;
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(1.0), h),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, RAIL);
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 1.0,
            egui::Stroke::new(1.0, theme::ACID),
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
        bar.painter().text(
            mark.left_center() + Vec2::new(0.0, -3.0),
            egui::Align2::LEFT_CENTER,
            "BLIGHTNET",
            FontId::new(15.0, theme::display()),
            theme::ACID,
        );
        bar.painter().rect_filled(
            Rect::from_min_size(
                mark.left_bottom() + Vec2::new(0.0, -8.0),
                Vec2::new(64.0, 2.0),
            ),
            0.0,
            CYAN,
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
                if theme::neon_btn_color(ui, "Online", theme::ACID, true).clicked() {
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
                    egui::Stroke::new(1.0, theme::fade(theme::ACID, 150)),
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
            egui::ScrollArea::horizontal()
                .id_salt("cmd-tabs")
                .auto_shrink([false, true])
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden,
                )
                .show(&mut tabs, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        let index_on = matches!(self.page, Page::Index);
                        if tab(ui, "INDEX", index_on).clicked() {
                            self.page = Page::Index;
                        }
                        if tab(ui, "TABLE", self.page == Page::Table).clicked() {
                            self.page = Page::Table;
                        }
                        if tab(ui, "NETHOOKS", self.page == Page::Nethooks).clicked() {
                            self.page = Page::Nethooks;
                            self.hook_edit = false;
                        }
                        if tab(ui, "NETSPACE", self.page == Page::Netspace).clicked() {
                            self.page = Page::Netspace;
                            self.jack_at = Instant::now();
                        }
                        if tab(ui, "ROTN", self.page == Page::Rotn).clicked() {
                            self.page = Page::Rotn;
                        }
                        if tab(ui, "TERMINAL", self.page == Page::Terminal).clicked() {
                            self.page = Page::Terminal;
                        }
                        if tab(ui, "RECON", self.page == Page::Recon).clicked() {
                            self.page = Page::Recon;
                        }
                        if self.player_full && tab(ui, "PLAYER", self.page == Page::Player).clicked() {
                            self.page = Page::Player;
                        }
                    });
                });
        });
    }

    fn paint_boot_city(&self, ui: &egui::Ui, r: Rect) {
        let _ = self;
        let horizon_y = r.top() + r.height() * 0.62;
        let vanish = egui::pos2(r.center().x, horizon_y);
        let p = ui.painter();
        let ground = Rect::from_min_max(egui::pos2(r.left(), horizon_y), r.right_bottom());
        p.rect_filled(ground, 0.0, Color32::from_rgb(5, 7, 12));
        let dim_c = theme::fade(CYAN, 42);
        for i in 0..9 {
            let x = r.left() + r.width() * (i as f32 / 8.0);
            p.line_segment(
                [vanish, egui::pos2(x, r.bottom())],
                egui::Stroke::new(1.0, dim_c),
            );
        }
        let band = (r.bottom() - horizon_y).max(40.0);
        for i in 1..5 {
            let y = horizon_y + band * (i as f32 / 4.0);
            p.hline(r.x_range(), y, egui::Stroke::new(1.0, theme::fade(CYAN, 28)));
        }
        let towers: [(f32, f32, f32, bool); 9] = [
            (0.03, 0.045, 0.38, false),
            (0.10, 0.07, 0.72, true),
            (0.19, 0.05, 0.48, true),
            (0.27, 0.09, 0.88, false),
            (0.40, 0.06, 0.55, true),
            (0.52, 0.08, 0.78, false),
            (0.64, 0.05, 0.44, true),
            (0.74, 0.09, 0.92, true),
            (0.86, 0.06, 0.58, false),
        ];
        let base = r.bottom() - 6.0;
        for (xf, wf, hf, cyan_edge) in towers {
            let w = r.width() * wf;
            let h = band * hf;
            let x = r.left() + r.width() * xf;
            let rect = Rect::from_min_max(egui::pos2(x, base - h), egui::pos2((x + w).min(r.right() - 4.0), base));
            let edge = if cyan_edge { CYAN } else { theme::HOT };
            theme::fill_chamfer(
                ui,
                rect,
                5.0,
                Color32::from_rgb(7, 9, 14),
                egui::Stroke::new(1.0, theme::fade(edge, 190)),
            );
            let mark = if cyan_edge {
                theme::fade(CYAN, 210)
            } else {
                theme::fade(theme::ACID, 200)
            };
            p.rect_filled(
                Rect::from_center_size(rect.center() + Vec2::new(0.0, -h * 0.14), Vec2::splat(3.0)),
                0.0,
                mark,
            );
            p.rect_filled(
                Rect::from_center_size(rect.center() + Vec2::new(0.0, h * 0.16), Vec2::splat(3.0)),
                0.0,
                theme::fade(edge, 150),
            );
        }
    }

    fn ui_boot(&mut self, ctx: &egui::Context) {
        let t = self.boot_at.elapsed().as_secs_f32();
        let skip = t > 0.2
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
        if skip || t > 8.6 {
            self.page = Page::Index;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, theme::BG);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(1.0, theme::HOT),
                    egui::StrokeKind::Inside,
                );
                theme::hud_ticks(ui, r.shrink(10.0), CYAN, 16.0);
                self.paint_boot_city(ui, r);
                let grow = (t / 2.4).clamp(0.0, 1.0);
                let anchor = egui::pos2(r.center().x, r.top() + r.height() * 0.34);
                let frame = Rect::from_center_size(
                    anchor,
                    Vec2::new(
                        r.width() * (0.50 + 0.18 * grow),
                        r.height() * (0.42 + 0.10 * grow),
                    ),
                );
                let edge = mix_rgb(theme::HOT, CYAN, grow);
                theme::fill_chamfer(
                    ui,
                    frame,
                    16.0,
                    Color32::from_rgba_unmultiplied(6, 8, 12, 230),
                    egui::Stroke::new(1.5, edge),
                );
                theme::hud_ticks(ui, frame.shrink(10.0), CYAN, 12.0);
                if t < 3.2 {
                    let scan = r.top() + (t / 3.2) * r.height();
                    ui.painter().hline(
                        r.x_range(),
                        scan,
                        egui::Stroke::new(8.0, Color32::from_rgba_unmultiplied(77, 232, 255, 22)),
                    );
                    ui.painter().hline(r.x_range(), scan, egui::Stroke::new(1.0, CYAN));
                }
                ui.painter().text(
                    anchor + Vec2::new(0.0, -72.0),
                    egui::Align2::CENTER_CENTER,
                    "BLIGHTNET",
                    FontId::new(42.0, theme::display()),
                    theme::ACID,
                );
                ui.painter().rect_filled(
                    Rect::from_center_size(anchor + Vec2::new(0.0, -44.0), Vec2::new(120.0, 2.0)),
                    0.0,
                    CYAN,
                );
                ui.painter().text(
                    anchor + Vec2::new(0.0, -26.0),
                    egui::Align2::CENTER_CENTER,
                    "LOCAL NODE",
                    FontId::new(13.0, theme::mono()),
                    CYAN,
                );
                let lines: &[(&str, &str, f32, Color32)] = &[
                    ("ok", "lock        data/daemon.lock", 0.55, CREAM),
                    ("ok", "handshake   x25519 · chacha20", 1.45, CREAM),
                    ("ok", "listen      127.0.0.1:18766", 2.35, CYAN),
                    ("ok", "node        WAITING · press Online", 3.25, CREAM),
                    ("ok", "shell       ready", 4.15, theme::ACID),
                ];
                for (i, (ok, rest, at, color)) in lines.iter().enumerate() {
                    if t < *at {
                        continue;
                    }
                    let y = anchor.y + 8.0 + i as f32 * 22.0;
                    let x = anchor.x - 210.0;
                    ui.painter().text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        *ok,
                        FontId::new(14.0, theme::mono()),
                        CYAN,
                    );
                    ui.painter().text(
                        egui::pos2(x + 36.0, y),
                        egui::Align2::LEFT_TOP,
                        *rest,
                        FontId::new(14.0, theme::mono()),
                        *color,
                    );
                }
                ui.painter().text(
                    egui::pos2(r.center().x, r.top() + r.height() * 0.585),
                    egui::Align2::CENTER_CENTER,
                    "CLICK OR PRESS ANY KEY TO SKIP",
                    FontId::new(12.0, theme::mono()),
                    DIM,
                );
            });
    }


    fn ui_index(&mut self, ui: &mut egui::Ui, t: f32) {
        let page_h = ui.available_height().max(40.0);
        egui::ScrollArea::vertical()
            .id_salt("index")
            .max_height(page_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                ui.add_space(8.0);
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
                        egui::Stroke::new(1.5, theme::ACID),
                    );
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(badge.shrink(4.0))
                            .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)),
                    );
                    images::show_fit(&mut child, &mut self.tex, &icon, Vec2::splat(46.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("NETDIR://LOCAL · BLIGHTNET DECK")
                                .family(theme::mono())
                                .size(11.0)
                                .color(CYAN),
                        );
                        ui.label(
                            RichText::new("DECK")
                                .family(theme::display())
                                .size(38.0)
                                .color(theme::ACID),
                        );
                        let (line, _) =
                            ui.allocate_exact_size(Vec2::new(88.0, 2.0), egui::Sense::hover());
                        ui.painter().rect_filled(line, 0.0, CYAN);
                        let world = if self.blight { "BLIGHT" } else { "HEARTHSONG" };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(world)
                                        .family(theme::display())
                                        .size(14.0)
                                        .color(CYAN),
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
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(12.0);
                    meta_c(ui, "CLK", &format_clock(self.clock), CYAN);
                    meta_c(
                        ui,
                        "LINK",
                        match self.net.role {
                            Role::Host => "HOST",
                            Role::Guest => "JOIN",
                            Role::Presence => "ONLINE",
                            Role::Idle => "LOCAL",
                        },
                        CYAN,
                    );
                    meta(ui, "ICE", "CLEAR");
                    meta_c(ui, "NODE", "8766", CYAN);
                });
                ui.add(egui::Separator::default());
                if !self.err.is_empty() {
                    wrap_text(ui, &self.err.clone(), CYAN, 13.0);
                }
                ui.add_space(4.0);
                wrap_text(
                    ui,
                    "Local deck. Stamp a Handle. Go Online so contacts can reach you. Host or Join a Blightnexus table to mix. Video Call sits on the systems rail. Catalogs live on Blightnexus.",
                    theme::CREAM,
                    15.0,
                );
                ui.add_space(12.0);
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
                ui.add_space(12.0);
                self.ui_index_log(ui);
                ui.add_space(16.0);
            });
    }

    fn ui_index_link(&mut self, ui: &mut egui::Ui, t: f32) {
        self.ui_index_link_inner(ui, t);
    }

    fn ui_index_link_inner(&mut self, ui: &mut egui::Ui, t: f32) {
        ui.label(
            RichText::new("PRIMARY LINK")
                .family(theme::mono())
                .size(11.0)
                .color(theme::ACID),
        );
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
            wrap_text(ui, &self.net.paste_link(), CYAN, 11.0);
        }
    }

    fn ui_index_systems(&mut self, ui: &mut egui::Ui) {
        self.ui_index_systems_inner(ui);
    }

    fn ui_index_systems_inner(&mut self, ui: &mut egui::Ui) {
        self.ensure_devices();
        self.ensure_cameras();
        ui.label(
            RichText::new("DECK SYSTEMS")
                .family(theme::mono())
                .size(11.0)
                .color(theme::ACID),
        );
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
        ui.add_space(8.0);
        ui.label(
            RichText::new("DISPLAY ARRAY")
                .family(theme::mono())
                .size(11.0)
                .color(MUTED),
        );
        wrap_text(ui, "This native window · borderless", CREAM, 11.0);
        ui.add_space(8.0);
        ui.label(
            RichText::new("DEVICES")
                .family(theme::mono())
                .size(11.0)
                .color(theme::ACID),
        );
        wrap_text(
            ui,
            &format!(
                "Auto-detected {} mic{}, {} speaker{}. Rescan devices looks again.",
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
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Rescan devices").clicked() {
                self.refresh_devices();
            }
            if theme::neon_btn(ui, "Update").clicked() {
                self.do_update();
            }
        });
        ui.horizontal_wrapped(|ui| {
            if self.node_live {
                if theme::neon_btn_color(ui, "Go offline", KILL, false).clicked() {
                    self.go_offline();
                }
            } else if theme::neon_btn(ui, "Go online").clicked() {
                self.go_online();
            }
        });
        ui.add_space(8.0);
        if theme::sys_tile(ui, "00", "DISCONNECT", "SHUT DOWN BLIGHTNET", "KILL", true).clicked() {
            self.go_offline();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui_index_log(&mut self, ui: &mut egui::Ui) {
        theme::pane().show(ui, |ui| {
                ui.label(
                    RichText::new("DECK LOG")
                        .family(theme::mono())
                        .size(11.0)
                        .color(theme::ACID),
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
                            .color(theme::ACID),
                    );
                    for b in bullets {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("▸").color(CYAN).size(14.0));
                            wrap_text(ui, b, CREAM, 14.0);
                        });
                    }
                }
            });
    }

    fn ui_index_protocol(&mut self, ui: &mut egui::Ui) {
        theme::pane().show(ui, |ui| {
                ui.label(
                    RichText::new("RUN PROTOCOL")
                        .family(theme::mono())
                        .size(11.0)
                        .color(theme::ACID),
                );
                ui.add_space(6.0);
                for (n, step) in [
                    "Stamp a Handle in the command bar.",
                    "Press Online. Nothing listens until you do. Press it again to stop the node.",
                    "Host or Join from the left rail on TABLE. Closing the window keeps the table. INDEX 00 or daemon-stop ends the node.",
                    "Press 01 BLIGHTNEXUS or the TABLE tab to mix. NETSPACE is its own tab.",
                    "Chat, Contacts, Voice, Video, and Player sit on the top row. Press the open one again to close it.",
                    "Player is the music library. The track name on the status line plays or pauses.",
                ]
                .iter()
                .enumerate()
                {
                    ui.horizontal(|ui| {
                        let (r, _) =
                            ui.allocate_exact_size(Vec2::new(22.0, 18.0), egui::Sense::hover());
                        theme::fill_chamfer(
                            ui,
                            r,
                            3.0,
                            PANEL,
                            egui::Stroke::new(1.0, theme::fade(theme::HOT, 140)),
                        );
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{n:02}"),
                            FontId::new(11.0, theme::mono()),
                            theme::ACID,
                        );
                        wrap_text(ui, step, theme::CREAM, 13.0);
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
                            .color(theme::ACID),
                    );
                    wrap_text(ui, last, CYAN, 11.0);
                }
            });
    }

    fn ui_dock(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, panel) in [
                ("Chat", ShellPanel::Chat),
                ("Contacts", ShellPanel::Contacts),
                ("Voice", ShellPanel::Voice),
                ("Video", ShellPanel::Video),
                ("Library", ShellPanel::Player),
                ("Host", ShellPanel::Host),
                ("Join", ShellPanel::Join),
            ] {
                if theme::neon_btn_color(ui, label, theme::ACID, self.shell == panel).clicked() {
                    self.shell = panel;
                    if panel == ShellPanel::Voice {
                        self.ensure_mic();
                    }
                    if panel == ShellPanel::Video {
                        self.cameras = crate::video::list_cameras();
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
        theme::kicker(ui, "NETDIR://PLAYER");
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
            "Pictures, video, PDF, and music on this deck. Nothing here is sent to the table, and it does not change the mix.",
            MUTED,
            12.0,
        );
        ui.add_space(6.0);
        self.paint_media_stage(ui, full);
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Prev").clicked() {
                self.deck_prev();
            }
            let playing = self.media_run || (self.deck_on && self.mixer.deck_live());
            if theme::neon_btn_color(ui, if playing { "Pause" } else { "Play" }, CYAN, playing)
                .clicked()
            {
                self.deck_toggle();
            }
            if theme::neon_btn(ui, "Next").clicked() {
                self.deck_next(false);
            }
        });
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
                        &["ogg", "mp3", "wav", "flac", "opus", "m4a", "aac", "png", "jpg", "jpeg", "webp", "mp4", "webm", "mkv", "pdf"],
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
            let on = selected && (self.deck_on || self.media_run || kind != "audio");
            let sub = if selected {
                match kind {
                    "video" if self.media_run => "PLAYING",
                    "video" => "VIDEO",
                    "image" => "PICTURE",
                    "pdf" => "PDF",
                    _ if self.deck_on => "NOW PLAYING",
                    _ => "SELECTED",
                }
            } else {
                match kind {
                    "video" => "VIDEO",
                    "image" => "PICTURE",
                    "pdf" => "PDF",
                    _ => "",
                }
            };
            if theme::wide_btn(ui, &name, sub, on).clicked() {
                self.deck_i = i;
                self.media_page = 1;
                self.media_msg.clear();
                self.deck_play_current();
            }
        }
    }

    fn ui_host_panel(&mut self, ui: &mut egui::Ui) {
        theme::kicker(ui, "NETDIR://HOST");
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
        theme::kicker(ui, "NETDIR://JOIN");
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
        theme::kicker(ui, "NETDIR://CHAT");
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
        theme::kicker(ui, "NETDIR://CONTACTS");
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
        theme::kicker(ui, "NETDIR://VOICE");
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
        theme::kicker(ui, "NETDIR://VIDEO");
        wrap_text(
            ui,
            "Call a contact or a crew. Camera is auto-detected. Share your screen on the same call. Go Online first.",
            MUTED,
            11.0,
        );
        if !self.net.presence && self.net.role == Role::Idle {
            wrap_text(ui, "Go Online or Host / Join first.", CYAN, 12.0);
        }
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
                    egui::Stroke::new(2.0, if resp.hovered() || resp.dragged() { theme::ACID } else { theme::HOT }),
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
                    egui::Stroke::new(2.0, if resp.hovered() || resp.dragged() { theme::ACID } else { theme::HOT }),
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
                                        .color(CYAN),
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
                            .color(CYAN),
                        );
                    });
                    if theme::neon_btn_color(ui, &self.place_name(), CYAN, true).clicked() {
                        self.place_open = !self.place_open;
                        self.watch_open = false;
                    }
                    if theme::neon_btn_color(ui, "Outside", CYAN, !self.inside).clicked() {
                        self.set_inside(false);
                    }
                    if theme::neon_btn_color(ui, "Inside", CYAN, self.inside).clicked() {
                        self.set_inside(true);
                    }
                    if theme::analog_watch(ui, self.clock, pal).clicked() {
                        self.watch_open = !self.watch_open;
                        self.place_open = false;
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format_clock(self.clock))
                                .family(theme::mono())
                                .size(13.0)
                                .color(CYAN),
                        );
                        ui.label(
                            RichText::new(period_label(self.time))
                                .family(theme::mono())
                                .size(10.0)
                                .color(CYAN),
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
                                    .color(theme::ACID),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Open the calendar.")
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Calendar);
                        }
                    });
                    if self.is_gm
                        && theme::neon_btn_color(ui, "Run clock", theme::HOT, self.clock_run)
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
                        if theme::neon_btn_color(ui, lab, CYAN, self.time == id).clicked() {
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
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "06", &self.overlay_cat.to_uppercase());
            ui.add(
                egui::TextEdit::singleline(&mut self.catalog_q)
                    .hint_text("Find…")
                    .desired_width(180.0),
            );

        });
        self.refresh_cat_cache();
        ui.columns(2, |cols| {
            let n = self.cat_cache.len();
            egui::ScrollArea::vertical()
                .id_salt("ov-cat-list")
                .show_rows(&mut cols[0], 50.0, n, |ui, range| {
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
                                theme::wide_btn(ui, name, extra, on);
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
                                    .size(22.0)
                                    .color(theme::ACID),
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
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "09", "JACK-IN");
            crate::netspace::hud(ui, &self.netspace, self.jack_at.elapsed().as_secs_f32());

        });
        crate::netspace::paint(
            ui,
            &mut self.netspace,
            self.jack_at.elapsed().as_secs_f32(),
            false,
        );
    }

    fn ui_overlay_maps(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
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
    }

    fn ui_overlay_chess(&mut self, ui: &mut egui::Ui) {
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
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "04", "HOUSE 21");

        });
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
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "08", &title.to_uppercase());
            ui.add(
                egui::TextEdit::singleline(&mut self.kit_filter)
                    .hint_text("Find gear…")
                    .desired_width(160.0),
            );
            if let Some(c) = self.chars.get(self.char_i) {
                ui.label(
                    RichText::new(if self.blight {
                        format!("{} · {} eb", c.name, c.eddies)
                    } else {
                        format!("{} · {} gp", c.name, c.gp)
                    })
                    .color(CYAN)
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
                            let row = |ui: &mut egui::Ui| {
                                ui.label(
                                    RichText::new(&name)
                                        .color(if focused { theme::ACID } else { CREAM })
                                        .size(16.0)
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
                                        .color(CYAN)
                                        .size(13.0)
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
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, egui::Sense::hover());
        let path = self.painting_path();
        images::paint_cover(ui, &mut self.tex, &path, rect);
        ui.painter().text(
            rect.left_top() + Vec2::new(12.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("{}  ·  {}", self.place_name(), period_label(self.time)),
            egui::FontId::new(14.0, theme::display()),
            theme::ACID,
        );
    }

    fn ui_calendar_tile(&mut self, ui: &mut egui::Ui) {
        let month = [
            "", "January", "February", "March", "April", "May", "June", "July", "August",
            "September", "October", "November", "December",
        ];
        let name = month.get(self.cal_m as usize).copied().unwrap_or("");
        ui.horizontal_wrapped(|ui| {
            if self.is_gm && theme::neon_btn(ui, "◀").clicked() {
                self.advance_month(-1);
            }
            ui.label(
                RichText::new(format!("{name} {}", self.cal_y))
                    .family(theme::display())
                    .size(22.0)
                    .color(theme::ACID),
            );
            if self.is_gm && theme::neon_btn(ui, "▶").clicked() {
                self.advance_month(1);
            }
        });
        ui.horizontal(|ui| {
            for day in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
                ui.add_sized(
                    [36.0, 18.0],
                    egui::Label::new(
                        RichText::new(day)
                            .family(theme::mono())
                            .size(11.0)
                            .color(CYAN),
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
                        if theme::neon_btn_color(ui, &n.to_string(), CYAN, on).clicked() && self.is_gm
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
        ui.add_space(10.0);
        theme::section_head(ui, "01", "SCENES");
        let energy = self.mixer.master.max(0.05);
        let phase = ui.input(|i| i.time) as f32 * 0.25;
        paint_viz(ui, ui.available_width().max(40.0), 22.0, energy, phase);
        theme::kicker(ui, "TABLE://PLAYLIST");
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Find a scene…")
                .desired_width(ui.available_width() - 16.0),
        );
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
        ui.add_space(10.0);
        let n = self
            .mixer
            .playing()
            .filter(|(id, _)| !id.starts_with("__"))
            .count();
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                theme::section_head(ui, "02", "THE MIX");
                let note = if n == 0 {
                    "NOTHING PLAYING".into()
                } else if !self.mix_msg.is_empty() {
                    self.mix_msg.clone()
                } else {
                    format!("{n} LAYERS LIVE")
                };
                theme::kicker(ui, &note);
            });
            if theme::neon_btn(ui, "Shuffle").clicked() {
                self.shuffle_music();
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.mix_search)
                .hint_text("Find a sound…")
                .desired_width(ui.available_width() - 12.0),
        );
        ui.horizontal_wrapped(|ui| {
            let cats: Vec<&str> = if self.blight {
                vec!["all", "music", "ambience", "animals"]
            } else {
                vec!["all", "music", "weather", "animals", "ambience"]
            };
            for c in cats {
                if theme::neon_btn_color(ui, c, CYAN, self.cat_filter == c).clicked() {
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
        theme::section_head(ui, "10", "COMBAT LOG");
        wrap_text(
            ui,
            "Shared with everyone at this table. Press Log again to close.",
            MUTED,
            12.0,
        );
        ui.add_space(4.0);
        let combat_h = ui.available_height().max(80.0);
        ui.label(
            RichText::new("ROLLS")
                .family(theme::mono())
                .size(11.0)
                .color(CYAN),
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

    fn ui_rail_notes(&mut self, ui: &mut egui::Ui) {
        theme::section_head(ui, "11", "NOTES");
        wrap_text(
            ui,
            "This deck only. Other players never see this. Press Notes again to close.",
            MUTED,
            12.0,
        );
        ui.add_space(4.0);
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
                                egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { theme::ACID } else { theme::fade(CYAN, 90) }),
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
                                if on { theme::ACID } else { CREAM },
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
            ui.label(
                RichText::new("THE PIT")
                    .family(theme::display())
                    .size(26.0)
                    .color(theme::ACID),
            );
            ui.label(
                RichText::new(&self.bj.msg)
                    .family(theme::mono())
                    .size(14.0)
                    .color(CYAN),
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
        ui.horizontal(|ui| {
            ui.heading(RichText::new(title).family(theme::display()).color(CYAN));
            ui.add(egui::TextEdit::singleline(&mut self.catalog_q).hint_text("search").desired_width(200.0));
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
                        Some((i, name, extra.chars().take(180).collect()))
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
                        if ui.selectable_label(on, format!("{name}\n{extra}")).clicked() {
                            self.catalog_pick = *i;
                        }
                    }
                });
                egui::ScrollArea::vertical().max_height(col_h).show(&mut cols[1], |ui| {
                    let row = self.catalog_rows.get(self.catalog_pick).cloned();
                    if let Some(v) = row {
                        if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                            ui.heading(name);
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
                &mut self.target,
                &mut self.luck,
                &mut self.roll,
                &self.names,
                &mut self.zoom_path,
                &mut None,
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
        for c in &mut self.chars {
            if c.owner.is_empty() {
                c.owner = oid.clone();
            }
        }
        let sheet_before = sheet_sig(&self.chars);
        let mut sheet_send = None;
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
            &mut self.target,
            &mut self.luck,
            &mut self.roll,
            &self.names,
            &mut self.zoom_path,
            &mut sheet_send,
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
        if sheet_sig(&self.chars) != sheet_before {
            self.mark_sheets();
        }
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
        let tid = if r.taken {
            attacker
        } else if r.target.is_empty() {
            return;
        } else {
            r.target.clone()
        };
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
                    c.downed = true;
                }
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
        ui.label(
            RichText::new("NETHOOKS")
                .family(theme::display())
                .size(28.0)
                .color(theme::ACID),
        );
        wrap_text(
            ui,
            "How this works. A page you can show the table. A handout is notes everyone should read. A rumor is gossip. A job is work someone will pay for. Press Post to table, then Board on the left of TABLE. Press Take down when it should go away. The lessons at the top only teach you how to write a page. Your private notes never become a page.",
            MUTED,
            13.0,
        );
        ui.add_space(8.0);
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

    fn ui_terminal(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
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
            self.term = Some(crate::term::Shell::spawn(cols, rows));
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
        crate::netspace::paint(
            ui,
            &mut self.netspace,
            self.jack_at.elapsed().as_secs_f32(),
            true,
        );
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
            self.chat.push("ffmpeg is not on this computer. Open the video outside.".into());
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
                        .color(theme::ACID),
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
        let steps: &[(&str, &str)] = &[
            ("INDEX", "The front desk. Tiles open TABLE, NET, and this tour."),
            ("ONLINE", "The node is off until you press Online. Press it again to stop it."),
            ("TABLE", "The world and the clock stay on top. Host, books, sheets, and maps are on the left rail."),
            ("TIME", "The watch and the date sit together. Past midnight, the calendar moves a day."),
            ("SCENES", "Scenes and Mix are on the left rail. They open a short strip under the world bar."),
            ("SEAT", "Every person at the table is a button. Click to open their sheet. Click again to close."),
            ("NETHOOKS", "Write HTML and CSS. The preview is live. Learn HTML and Learn CSS are pinned lessons."),
            ("NETSPACE", "Walk the city. When Online, other seats appear under their handles."),
            ("ROTN", "The fixer. Press Hour, Table, Place, Rumor, Job, or NPC. Nothing they say is sent away."),
        ];
        let step = step.min(steps.len() - 1);
        let (title, body) = steps[step];
        egui::Area::new(egui::Id::new("tour-card"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -48.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(12, 12, 8))
                    .stroke(egui::Stroke::new(2.0, theme::ACID))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        let w = (ui.ctx().screen_rect().width() - 48.0).clamp(180.0, 420.0);
                        ui.set_max_width(w);
                        ui.set_width(w);
                        ui.label(
                            RichText::new(format!("TOUR {} / {}", step + 1, steps.len()))
                                .family(theme::mono())
                                .size(11.0)
                                .color(theme::ACID),
                        );
                        ui.label(
                            RichText::new(title)
                                .family(theme::display())
                                .size(22.0)
                                .color(CYAN),
                        );
                        ui.label(RichText::new(body).color(CREAM).size(14.0));
                        ui.horizontal_wrapped(|ui| {
                            if theme::neon_btn(ui, "Back").clicked() && step > 0 {
                                self.tour = Some(step - 1);
                            }
                            if theme::neon_btn(ui, "Next").clicked() {
                                if step + 1 >= steps.len() {
                                    self.tour = None;
                                } else {
                                    self.tour = Some(step + 1);
                                    self.jump_tour(step + 1);
                                }
                            }
                            if theme::neon_btn_color(ui, "Skip", KILL, false).clicked() {
                                self.tour = None;
                            }
                        });
                    });
            });
    }

    fn jump_tour(&mut self, step: usize) {
        self.page = match step {
            0 => Page::Index,
            1 => Page::Index,
            2 | 3 | 4 | 5 => Page::Table,
            6 => Page::Nethooks,
            7 => Page::Netspace,
            _ => Page::Rotn,
        };
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
            self.page = Page::Index;
        }
        wrap_text(
            ui,
            "The tour walks the real window. Each card says what the control does. The world bar stays above the painting. Tools stay on the left rail.",
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
                let cards: &[(&str, &str, &str)] = &[
                    ("01", "DECK", "Native window. No browser. Keep audio/, assets/, and data/ next to the binary. Update on the status line pulls from GitHub. Rescan devices is on INDEX."),
                    ("02", "NET", "Stamp a Handle in the command bar. Online starts the node. Host copies a blightnet:// invite. Friends paste it into Join. No Cloudflare. The window X leaves the node running. INDEX 00 shuts it down and closes. daemon-stop does the same from a terminal."),
                    ("03", "TABLE", "INDEX 01 BLIGHTNEXUS or the TABLE tab. Scenes and Mix open as tiles in the center. Master is local. Place is the painting. Time is the watch. The world name switches Hearthsong and Blight. Log and Notes are on the left rail. Notes stay on this computer."),
                    ("04", "TALK", "Chat, Contacts, Voice, and Video are on the top row, next to the tabs. Send any file. Chat stays until Wipe chat."),
                    ("05", "MAPS", "Gamemaster only: Ink, filled Circle, filled Square to fog the board. Erase click. Clear drawings. Marks sync to the table."),
                    ("06", "NETHOOKS", "Pages you can show the table. A handout, a rumor, or a job. Press Post to table, then Board on the left of TABLE. The lessons at the top only teach you how to write a page."),
                    ("09", "FIXER AND BOARD", "The fixer (ROTN) sits on your computer and knows the table. Press Hour, Table, or Place for the facts. Press Rumor, Job, or NPC and they make something up. None of that is sent away. You do not need a model. DeepSeek and Kimi work only if you start them on this computer, then press Rescan. Post rumor or Post job makes a page. Post to table shows it. Board opens it. Take down removes it. Private notes, the fixer's memory, and the model never leave your computer."),
                    ("07", "NETSPACE", "Own tab, full window. WASD, look-drag, Shift to run, C for auto-walk. Radar overlays the city. TABLE Jack-in or the NETSPACE tab opens it."),
                    ("08", "PLAY", "Player on the top row plays files on this machine. The status line shows the track. It is independent of the table mix and radio."),
                ];
                for (id, title, body) in cards {
                    egui::Frame::NONE
                        .fill(PANEL)
                        .stroke(egui::Stroke::new(1.0, theme::fade(theme::HOT, 140)))
                        .inner_margin(egui::Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(*id)
                                        .family(theme::mono())
                                        .size(13.0)
                                        .color(theme::ACID),
                                );
                                ui.label(
                                    RichText::new(*title)
                                        .family(theme::display())
                                        .size(18.0)
                                        .color(CREAM),
                                );
                            });
                            ui.add_space(4.0);
                            wrap_text(ui, body, CREAM, 14.0);
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
    ui.label(
        RichText::new(label)
            .family(theme::mono())
            .size(10.0)
            .color(DIM),
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
        "txt" | "md" => "text/plain",
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
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::new(13.0, theme::display()),
        if on { theme::ACID } else { CREAM },
    );
    let size = Vec2::new(galley.size().x + 22.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover = resp.hovered();
    let fill = if on {
        Color32::from_rgb(18, 20, 8)
    } else if hover {
        theme::fade(theme::ACID, 22)
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if on {
        theme::ACID
    } else if hover {
        CYAN
    } else {
        theme::fade(theme::HOT, 110)
    };
    theme::fill_chamfer(ui, rect, 4.0, fill, egui::Stroke::new(1.0, stroke));
    if on {
        ui.painter().hline(
            (rect.left() + 6.0)..=(rect.right() - 6.0),
            rect.bottom() - 1.0,
            egui::Stroke::new(2.0, theme::HOT),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(13.0, theme::display()),
        if on || hover { theme::ACID } else { CREAM },
    );
    resp
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

fn mix_rgb(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

fn meta(ui: &mut egui::Ui, k: &str, v: &str) {
    meta_c(ui, k, v, CREAM);
}

fn meta_c(ui: &mut egui::Ui, k: &str, v: &str, value: Color32) {
    egui::Frame::NONE
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, theme::fade(theme::HOT, 140)))
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
        self.stop_media_proc();
        self.stop_hook_video();
        self.term = None;
    }
}

fn is_library_file(path: &std::path::Path) -> bool {
    crate::audio::is_music(path) || matches!(media_kind(path), "image" | "video" | "pdf")
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

fn save_saved(root: &Path, mixes: &[SavedMix]) {
    if let Ok(s) = serde_json::to_string_pretty(mixes) {
        let _ = std::fs::write(root.join("data/saved-mixes.json"), s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
