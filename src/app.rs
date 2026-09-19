use crate::audio::Mixer;
use crate::catalog::{load_list, Catalog, Layer};
use crate::chars::{Character, KitSpec};
use crate::dice::{Luck, Roll};
use crate::images::{self, TexCache};
use crate::maps::{self, MapBoard, MapOp, TokenSpec};
use crate::names::Names;
use crate::net::{self, Contact, NetEvent, NetHub, Role};
use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED, ORANGE, PANEL};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Catalog,
    Chars,
    Maps,
    Blackjack,
    Armory,
    Vendors,
    Jackin,
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
    vendor_stock: Vec<serde_json::Value>,
    custom: Vec<Layer>,
    kit_filter: String,
    tex: TexCache,
    net: NetHub,
    contacts: Vec<Contact>,
    shell: ShellPanel,
    join_in: String,
    voice_on: bool,
    voice_mute: bool,
    incoming: Option<(String, String)>,
    call_id: Option<String>,
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
    hook_draft_title: String,
    hook_draft_html: String,
    deck_list: Vec<PathBuf>,
    deck_i: usize,
    deck_on: bool,
    deck_vol: f32,
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
        1 => ("H", ORANGE),
        2 => ("D", ORANGE),
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
            egui::Stroke::new(2.0, ORANGE),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "BN",
            FontId::new(18.0, theme::display()),
            ORANGE,
        );
        return;
    }
    ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(22, 18, 10));
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(2.0, ORANGE),
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
            vendor_stock: vec![],
            custom: vec![],
            kit_filter: String::new(),
            tex: TexCache::default(),
            net: NetHub::new("Traveller".into(), &root),
            contacts: net::load_contacts(&root),
            shell: ShellPanel::None,
            join_in: String::new(),
            voice_on: false,
            voice_mute: false,
            incoming: None,
            call_id: None,
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
            hook_draft_title: String::new(),
            hook_draft_html: String::new(),
            deck_list: vec![],
            deck_i: 0,
            deck_on: false,
            deck_vol: 0.7,
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
        self.time = period_from_clock(self.clock);
        self.refresh_presence();
    }

    fn set_period(&mut self, time: &'static str) {
        self.set_clock(clock_from_period(time));
    }

    fn set_inside(&mut self, inside: bool) {
        self.inside = inside;
        self.refresh_presence();
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
            return;
        }
        if let Some(l) = self.layer_any(id) {
            if let Some(file) = l.audio_file() {
                if let Err(e) = self.mixer.play(id, file, 0.45) {
                    self.err = e;
                } else {
                    let p = self.presence_of(id);
                    self.mixer.set_presence(id, p);
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
            }
            return;
        }
        self.held_mix = Some(self.mixer.snapshot());
        self.mixer.silence();
        self.mix_msg = "Table faded. Fade in brings it back.".into();
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
        }
    }

    fn cycle_radio(&mut self) {
        if !self.radio_on || self.radio_station.is_empty() {
            self.tune_station(RADIO[0].id);
            return;
        }
        let i = RADIO
            .iter()
            .position(|s| s.id == self.radio_station)
            .unwrap_or(0);
        let next = (i + 1) % (RADIO.len() + 1);
        if next == RADIO.len() {
            self.mixer.stop("__radio");
            self.radio_on = false;
            self.radio_track.clear();
            self.radio_station.clear();
        } else {
            self.tune_station(RADIO[next].id);
        }
    }

    fn tune_station(&mut self, id: &str) {
        if self.radio_on && self.radio_station == id {
            self.play_radio_track();
            return;
        }
        self.radio_station = id.into();
        self.play_radio_track();
    }

    fn play_radio_track(&mut self) {
        let st = RADIO.iter().find(|s| s.id == self.radio_station);
        let mood = st.map(|s| s.mood).unwrap_or(RADIO[0].mood);
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
        self.net.host(internet, self.root.clone());
        self.net.internet = internet;
        if internet {
            self.chat.push(
                "Table open. You are the Gamemaster. Opening a Cloudflare invite link for other networks…".into(),
            );
            self.status = "Hosting · opening internet link…".into();
        } else {
            self.chat.push(
                "Table open on the local network. Same-house friends Join with Copy address.".into(),
            );
            self.status = "Hosting · local network".into();
        }
        self.shell = ShellPanel::Host;
    }

    fn do_join(&mut self, addr: &str) {
        self.net.join(addr);
        self.chat.push(format!("Joining {addr}…"));
        self.shell = ShellPanel::Chat;
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
        self.net.go_online();
        for c in &self.contacts {
            if !c.addr.is_empty() {
                self.net.dial(&c.addr);
            }
        }
        self.chat.push("You are online. Contacts can see you and DM without a table invite.".into());
        self.status = "Online".into();
        self.announce_hooks();
    }

    fn live_link(&self) -> bool {
        self.net.presence || self.net.role != Role::Idle
    }

    fn push_owned_hooks(&self) {
        if !self.live_link() {
            return;
        }
        for h in &self.nethooks {
            if h.owner_id == self.net.self_id {
                self.net.send_nethook_put(h.clone());
            }
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
        crate::nethook::merge(&mut self.nethooks, hook.clone());
        if let Some(h) = self.nethooks.iter().find(|h| h.id == id) {
            crate::nethook::save_one(&self.root, h);
        }
        if fresh && hook.owner_id != self.net.self_id {
            self.chat.push(format!("Nethook online: {title} · {owner}"));
        }
    }

    fn apply_hook_del(&mut self, id: &str, owner_id: &str) {
        let ok = self
            .nethooks
            .iter()
            .any(|h| h.id == id && h.owner_id == owner_id);
        if !ok {
            return;
        }
        self.nethooks.retain(|h| h.id != id);
        crate::nethook::delete_one(&self.root, id);
        if self.hook_i >= self.nethooks.len() {
            self.hook_i = self.nethooks.len().saturating_sub(1);
        }
        self.hook_edit = false;
    }

    fn go_offline(&mut self) {
        self.net.leave();
        self.chat.push("You went offline.".into());
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
                NetEvent::Status(s) => self.status = s,
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
                }
                NetEvent::Relay { url } => {
                    self.chat.push(format!(
                        "Friends on any network paste this into Join:\n{url}"
                    ));
                    self.status = "Hosting · internet link ready".into();
                    self.shell = ShellPanel::Host;
                }
                NetEvent::Joined { addr } => {
                    self.chat.push(format!("Joined {addr}"));
                    self.status = "Joined".into();
                    self.page = Page::Table;
                    self.announce_hooks();
                    self.net.send_map_marks_ask();
                }
                NetEvent::Left => {
                    self.status = "Offline".into();
                    self.voice_on = false;
                }
                NetEvent::Peers(p) => {
                    for peer in &p {
                        if peer.id != self.net.self_id
                            && !self.contacts.iter().any(|c| c.id == peer.id)
                        {
                            // listed in contacts panel as table people
                        }
                    }
                    let _ = p;
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
                        "accept" => {
                            self.call_id = Some(from);
                            if crew.is_some() {
                                self.call_crew = crew.clone();
                            }
                            self.voice_on = true;
                            self.ensure_mic();
                            self.chat.push(format!("{name} picked up."));
                        }
                        "decline" | "hangup" => {
                            self.remote_vid.remove(&from);
                            if self.call_crew.is_none()
                                && (self.call_id.as_deref() == Some(&from)
                                    || self.incoming.as_ref().map(|i| i.0.as_str())
                                        == Some(from.as_str()))
                            {
                                self.end_call_local();
                            } else if action == "decline" {
                                self.incoming = None;
                                self.incoming_video = None;
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
                    if self.net.role == Role::Guest {
                        if self.blight != blight {
                            self.set_world(blight);
                        }
                        self.place = place;
                        self.inside = inside;
                        if let Some(t) = HOURS.iter().copied().find(|h| *h == time.as_str()) {
                            self.set_period(t);
                        }
                        let files = self.files();
                        let _ = self.mixer.apply_scene(&layers, &files);
                        self.refresh_presence();
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
                            self.call_id = Some(from.clone());
                            if crew.is_some() {
                                self.call_crew = crew;
                            }
                            self.video_on = true;
                            self.voice_on = true;
                            self.ensure_mic();
                            self.start_cam();
                            self.chat.push(format!("{name} joined the video call."));
                            self.shell = ShellPanel::Video;
                        }
                        "decline" | "hangup" => {
                            self.remote_vid.remove(&from);
                            if self.call_crew.is_none()
                                && (self.incoming_video.as_ref().map(|i| i.0.as_str())
                                    == Some(from.as_str())
                                    || self.call_id.as_deref() == Some(&from))
                            {
                                self.end_call_local();
                            } else if action == "decline" {
                                self.incoming_video = None;
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
        let talk = self.voice_on
            && !self.voice_mute
            && (self.net.presence || self.net.role != Role::Idle);
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
        self.devices_on = true;
        self.refresh_devices();
        self.chat.push(format!(
            "Devices updated · {} mics · {} speakers · {} cameras.",
            self.inputs.len(),
            self.outputs.len(),
            self.cameras.len()
        ));
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

    fn deck_play_current(&mut self) {
        let Some(p) = self.deck_list.get(self.deck_i).cloned() else {
            self.deck_on = false;
            self.mixer.deck_stop();
            return;
        };
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
            self.chat.push("Add music files to the player library.".into());
            return;
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
        self.deck_i = (self.deck_i + 1) % self.deck_list.len();
        if force || self.deck_on {
            self.deck_play_current();
        }
        save_deck_lib(&self.root, &self.deck_list);
    }

    fn deck_prev(&mut self) {
        if self.deck_list.is_empty() {
            return;
        }
        if self.deck_i == 0 {
            self.deck_i = self.deck_list.len() - 1;
        } else {
            self.deck_i -= 1;
        }
        if self.deck_on {
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
            if !crate::audio::is_music(&p) || !p.is_file() {
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
            self.chat.push(format!("Player: added {n} track{}.", if n == 1 { "" } else { "s" }));
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
        if let Some(cid) = &self.call_crew {
            self.crews
                .iter()
                .find(|c| &c.id == cid)
                .map(|c| {
                    c.members
                        .iter()
                        .filter(|id| *id != &self.net.self_id)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        } else if let Some(id) = &self.call_id {
            vec![id.clone()]
        } else {
            vec![]
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
        let targets = self.call_targets();
        if self.cam_on {
            if let Some(cap) = self.cam_cap.as_ref() {
                if let Some(f) = cap.latest() {
                    self.local_cam = f;
                    if !targets.is_empty() {
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
                    if !targets.is_empty() {
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
                self.call_id = Some(id);
                self.incoming = None;
                self.voice_on = true;
                self.ensure_mic();
            }
            return;
        };
        self.call_id = Some(id.clone());
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
        self.shell = ShellPanel::Video;
    }

    fn broadcast_mix(&self) {
        if self.net.role == Role::Host {
            self.net.send_mix(
                self.mixer.snapshot(),
                self.blight,
                self.place.clone(),
                self.time.to_string(),
                self.inside,
            );
        }
    }
}

impl eframe::App for Blightnet {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick_fps();
        self.poll_net();
        self.trim_logs();
        self.flush_notes();
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
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(7, 7, 4)))
            .show(ctx, |ui| {
                theme::scanlines(ui, ui.max_rect());
                let win = ui.max_rect().shrink(10.0);
                ui.painter().rect_filled(
                    win.translate(Vec2::new(10.0, 10.0)),
                    0.0,
                    Color32::from_rgba_unmultiplied(255, 106, 18, 40),
                );
                ui.painter()
                    .rect_filled(win, 0.0, Color32::from_rgb(12, 12, 8));
                ui.painter()
                    .rect_stroke(win, 0.0, egui::Stroke::new(2.0, ORANGE), egui::StrokeKind::Inside);
                ui.painter().rect_stroke(
                    win.shrink(3.0),
                    0.0,
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(77, 232, 255, 70)),
                    egui::StrokeKind::Inside,
                );
                theme::brackets(ui, win, ORANGE, 18.0);
                theme::brackets(ui, win.shrink(6.0), Color32::from_rgba_unmultiplied(77, 232, 255, 90), 10.0);
                let mut ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(win.shrink2(Vec2::new(2.0, 2.0)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                self.draw_titlebar(&mut ui);
                self.draw_tabs(&mut ui);
                self.draw_navbar(&mut ui);
                let rest = ui.available_rect_before_wrap();
                let (body, status) = rest.split_top_bottom_at_y(rest.bottom() - 32.0);
                let dock_open = self.shell != ShellPanel::None;
                let dock_w = if dock_open {
                    (body.width() * 0.34).clamp(300.0, 430.0).min(body.width() * 0.48)
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
                    Page::Boot => {}
                }
                if dock_open {
                    ui.painter().vline(
                        dock.left(),
                        body.y_range(),
                        egui::Stroke::new(2.0, ORANGE),
                    );
                    theme::plate(&ui, dock.shrink(4.0));
                    let mut dock_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(dock.shrink2(Vec2::new(12.0, 10.0)))
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    self.ui_dock(&mut dock_ui);
                }
                self.ui_zoom(ui.ctx());
                let mut st = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(status)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                st.painter().rect_filled(status, 0.0, Color32::from_rgb(8, 8, 6));
                st.painter().hline(
                    status.x_range(),
                    status.top(),
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 80)),
                );
                st.horizontal_wrapped(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("LINK")
                            .family(theme::mono())
                            .size(10.0)
                            .color(DIM),
                    );
                    ui.label(
                        RichText::new(match self.net.role {
                            Role::Host => "HOSTING",
                            Role::Guest => "JOINED",
                            Role::Presence => "ONLINE",
                            Role::Idle => "LOCAL",
                        })
                        .family(theme::mono())
                        .size(10.0)
                        .color(if self.net.role == Role::Idle {
                            ORANGE
                        } else {
                            CYAN
                        }),
                    );
                    ui.label(
                        RichText::new("PAGE")
                            .family(theme::mono())
                            .size(10.0)
                            .color(DIM),
                    );
                    let page = match self.page {
                        Page::Index => "INDEX",
                        Page::Table => "TABLE",
                        Page::Chars => "CHARS",
                        Page::Tutorial => "TUTORIAL",
                        Page::Audio => "AUDIO",
                        Page::Blackjack => "BLACKJACK",
                        Page::Nethooks => "NETHOOKS",
                        Page::Catalog(n) => n,
                        Page::Boot => "BOOT",
                    };
                    ui.label(
                        RichText::new(page)
                            .family(theme::mono())
                            .size(10.0)
                            .color(ORANGE),
                    );
                    ui.label(
                        RichText::new("PROTO  BLIGHTNET")
                            .family(theme::mono())
                            .size(10.0)
                            .color(DIM),
                    );
                    ui.label(
                        RichText::new(&self.status)
                            .family(theme::mono())
                            .size(10.0)
                            .color(CYAN),
                    );
                    if let Some(name) = self.deck_track_name() {
                        ui.label(
                            RichText::new(if self.deck_on { "PLAY" } else { "DECK" })
                                .family(theme::mono())
                                .size(10.0)
                                .color(DIM),
                        );
                        ui.label(
                            RichText::new(name)
                                .family(theme::mono())
                                .size(10.0)
                                .color(CYAN),
                        );
                    }
                });
            });
    }

    fn draw_titlebar(&mut self, ui: &mut egui::Ui) {
        let shown = egui::Frame::NONE
            .fill(Color32::from_rgb(17, 17, 8))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
                ui.horizontal(|ui| {
                    let (net, _) = ui.allocate_exact_size(Vec2::new(44.0, 22.0), egui::Sense::hover());
                    theme::fill_chamfer(ui, net, 6.0, ORANGE, egui::Stroke::NONE);
                    ui.painter().text(
                        net.center(),
                        egui::Align2::CENTER_CENTER,
                        "NET",
                        FontId::new(11.0, theme::display()),
                        Color32::from_rgb(17, 17, 17),
                    );
                    ui.label(
                        RichText::new("BLIGHTNET")
                            .family(theme::display())
                            .size(13.0)
                            .color(ORANGE),
                    );
                    ui.label(
                        RichText::new("KEYSTONE // LOCAL NODE")
                            .family(theme::mono())
                            .size(10.0)
                            .color(MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::neon_btn_color(ui, "×", KILL, true).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let maxed = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(true));
                        if theme::neon_btn(ui, if maxed { "❐" } else { "□" }).clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
                        }
                        ui.label(
                            RichText::new(format!("{:>3.0} FPS", self.fps))
                                .family(theme::mono())
                                .size(13.0)
                                .color(if self.fps >= 59.0 { CYAN } else { ORANGE }),
                        );
                        let drag_w = ui.available_width().max(16.0);
                        let (_, drag) = ui.allocate_exact_size(
                            Vec2::new(drag_w, 22.0),
                            egui::Sense::click_and_drag(),
                        );
                        if drag.drag_started() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        if drag.double_clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maxed));
                        }
                    });
                });
            });
        theme::hatch(
            ui,
            shown.response.rect,
            Color32::from_rgba_unmultiplied(255, 106, 18, 10),
        );
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(2.0, ORANGE),
        );
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom() - 1.0,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(77, 232, 255, 70)),
        );
    }

    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = egui::Frame::NONE
            .fill(Color32::from_rgb(8, 8, 6))
            .inner_margin(egui::Margin::symmetric(8, 2))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                ui.horizontal_wrapped(|ui| {
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
                    ui.label(RichText::new("+").color(DIM).family(theme::mono()));
                    ui.label(
                        RichText::new(if self.net.role == Role::Idle {
                            "1 SESSION".into()
                        } else {
                            format!("{} ONLINE", self.net.peers.len().max(1))
                        })
                        .family(theme::mono())
                        .size(10.0)
                        .color(DIM),
                    );
                });
            });
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(2.0, ORANGE),
        );
    }

    fn draw_navbar(&mut self, ui: &mut egui::Ui) {
        let loc = match self.page {
            Page::Table => "blightnet://blightnexus",
            Page::Tutorial => "blightnet://tutorial",
            Page::Audio => "blightnet://audio",
            Page::Blackjack => "blightnet://blackjack",
            Page::Nethooks => "blightnet://nethooks",
            _ => "blightnet://start",
        };
        egui::Frame::NONE
            .fill(Color32::from_rgb(10, 10, 6))
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("LOC  {loc}"))
                            .family(theme::mono())
                            .size(11.0)
                            .color(ORANGE),
                    );
                    ui.label(RichText::new("Handle").family(theme::mono()).size(10.0).color(MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.handle)
                            .desired_width(120.0)
                            .font(FontId::new(13.0, theme::ui_font()))
                            .text_color(ORANGE),
                    );
                    if theme::neon_btn(ui, if self.blight { "Blight" } else { "Hearthsong" }).clicked()
                    {
                        self.set_world(!self.blight);
                    }
                    ui.label(
                        RichText::new(&self.status)
                            .family(theme::mono())
                            .size(11.0)
                            .color(CYAN),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    cluster(ui, "DECK", |ui| {
                        if theme::neon_btn_color(ui, "Update", CYAN, true).clicked() {
                            self.do_update();
                        }
                    });
                    cluster(ui, "NET", |ui| {
                        match self.net.role {
                            Role::Idle | Role::Presence => {
                                if theme::neon_btn_color(
                                    ui,
                                    "Host",
                                    ORANGE,
                                    self.shell == ShellPanel::Host,
                                )
                                .clicked()
                                {
                                    self.toggle_shell(ShellPanel::Host);
                                }
                                if theme::neon_btn_color(
                                    ui,
                                    "Join",
                                    ORANGE,
                                    self.shell == ShellPanel::Join,
                                )
                                .clicked()
                                {
                                    self.toggle_shell(ShellPanel::Join);
                                }
                            }
                            Role::Host => {
                                if theme::neon_btn(ui, "Copy address").clicked() {
                                    let link = self.net.paste_link();
                                    ui.ctx().copy_text(link.clone());
                                    self.chat.push(format!("Copied {link}"));
                                }
                                if theme::neon_btn_color(ui, "Leave", KILL, false).clicked() {
                                    self.leave_table();
                                }
                            }
                            Role::Guest => {
                                if theme::neon_btn_color(ui, "Leave", KILL, false).clicked() {
                                    self.leave_table();
                                }
                            }
                        }
                        if self.net.presence {
                            if theme::neon_btn_color(ui, "Online", CYAN, true).clicked() {
                                self.go_offline();
                            }
                        } else if theme::neon_btn(ui, "Go online").clicked() {
                            self.go_online();
                        }
                    });
                    cluster(ui, "TALK", |ui| {
                        if theme::neon_btn_color(ui, "Chat", ORANGE, self.shell == ShellPanel::Chat)
                            .clicked()
                        {
                            self.toggle_shell(ShellPanel::Chat);
                        }
                        if theme::neon_btn_color(
                            ui,
                            "Contacts",
                            ORANGE,
                            self.shell == ShellPanel::Contacts,
                        )
                        .clicked()
                        {
                            self.toggle_shell(ShellPanel::Contacts);
                        }
                        if theme::neon_btn_color(
                            ui,
                            "Voice",
                            ORANGE,
                            self.shell == ShellPanel::Voice,
                        )
                        .clicked()
                        {
                            self.toggle_shell(ShellPanel::Voice);
                        }
                        if theme::neon_btn_color(
                            ui,
                            "Video",
                            CYAN,
                            self.shell == ShellPanel::Video,
                        )
                        .clicked()
                        {
                            self.toggle_shell(ShellPanel::Video);
                        }
                    });
                    cluster(ui, "PLAY", |ui| {
                        if theme::neon_btn(ui, "Prev").clicked() {
                            self.deck_prev();
                        }
                        let playing = self.deck_on && self.mixer.deck_live();
                        if theme::neon_btn_color(
                            ui,
                            if playing { "Pause" } else { "Play" },
                            CYAN,
                            playing,
                        )
                        .clicked()
                        {
                            self.deck_toggle();
                        }
                        if theme::neon_btn(ui, "Next").clicked() {
                            self.deck_next(false);
                        }
                        if theme::neon_btn_color(
                            ui,
                            "Library",
                            ORANGE,
                            self.shell == ShellPanel::Player,
                        )
                        .clicked()
                        {
                            self.toggle_shell(ShellPanel::Player);
                        }
                        let name = self
                            .deck_track_name()
                            .unwrap_or_else(|| "NO TRACK".into());
                        ui.label(
                            RichText::new(name)
                                .family(theme::mono())
                                .size(11.0)
                                .color(if self.deck_on { CYAN } else { MUTED }),
                        );
                    });
                });
            });
        let y = ui.min_rect().bottom();
        ui.painter().hline(
            ui.max_rect().x_range(),
            y,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 110)),
        );
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
        if skip || t > 8.2 {
            self.page = Page::Index;
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(12, 12, 8)))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().rect_filled(r, 0.0, Color32::from_rgb(12, 12, 8));
                theme::scanlines(ui, r);
                ui.painter().text(
                    r.right_top() + Vec2::new(-14.0, 10.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{:>3.0} FPS", self.fps),
                    FontId::new(13.0, theme::mono()),
                    if self.fps >= 59.0 { CYAN } else { ORANGE },
                );
                let win = r.shrink(18.0);
                ui.painter()
                    .rect_filled(win, 0.0, Color32::from_rgb(10, 10, 6));
                ui.painter().rect_stroke(
                    win,
                    0.0,
                    egui::Stroke::new(2.0, ORANGE),
                    egui::StrokeKind::Inside,
                );
                theme::brackets(ui, win, ORANGE, 16.0);
                let mut body = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(win.shrink2(Vec2::new(18.0, 14.0)))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                body.horizontal_wrapped(|ui| {
                    let icon = self.root.join("assets/icon.png");
                    images::show_fit(ui, &mut self.tex, &icon, Vec2::splat(56.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("NETDIR://LOCAL · COLD BOOT")
                                .family(theme::mono())
                                .size(11.0)
                                .color(MUTED),
                        );
                        ui.label(
                            RichText::new("BLIGHTNET")
                                .family(theme::display())
                                .size(28.0)
                                .color(ORANGE),
                        );
                        ui.label(
                            RichText::new("RUNNER SHELL  ·  NODE 8766")
                                .family(theme::mono())
                                .size(12.0)
                                .color(CYAN),
                        );
                    });
                });
                body.add_space(8.0);
                body.add(egui::Separator::default());
                body.add_space(6.0);
                wrap_text(
                    &mut body,
                    "Deck coming up. You are the runner. The jack is warm. Blightnet is on the other side of this handshake.",
                    theme::CREAM,
                    14.0,
                );
                body.add_space(8.0);
                let lines: &[(&str, f32, bool)] = &[
                    ("$ blightnet --node local", 0.15, false),
                    ("ok  bios  oxanium / share-tech-mono", 0.55, true),
                    ("$ ice.handshake --port 8766", 1.0, false),
                    ("ok  ice  CLEAR", 1.45, true),
                    ("$ mount blightnet://netdir", 1.9, false),
                    ("ok  shards  characters maps radio", 2.35, true),
                    ("$ load runner.deck --handle Traveller", 2.8, false),
                    ("ok  deck  seated", 3.25, true),
                    ("$ jack --brain --quiet", 3.7, false),
                    ("ok  neural link  ready", 4.2, true),
                    ("$ dive blightnet", 4.7, false),
                    ("ok  SHELL READY  ·  you are in", 5.3, true),
                ];
                for (text, at, ok) in lines {
                    if t >= *at {
                        body.label(
                            RichText::new(*text)
                                .family(theme::mono())
                                .size(13.0)
                                .color(if *ok { CYAN } else { ORANGE }),
                        );
                    }
                }
                body.add_space(10.0);
                let bars: &[(&str, f32, f32)] = &[
                    ("BIOS", 0.4, 1.2),
                    ("ICE", 1.3, 2.1),
                    ("SHARDS", 2.2, 3.1),
                    ("JACK", 3.6, 5.0),
                    ("DIVE", 5.0, 7.4),
                ];
                for (name, a, b) in bars {
                    if t < *a {
                        continue;
                    }
                    let p = ((t - *a) / (b - a)).clamp(0.0, 1.0);
                    body.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{name:8}"))
                                .family(theme::mono())
                                .size(11.0)
                                .color(DIM),
                        );
                        let (bar, _) = ui.allocate_exact_size(Vec2::new(220.0, 12.0), egui::Sense::hover());
                        ui.painter().rect_filled(bar, 0.0, Color32::from_rgb(17, 17, 8));
                        ui.painter().rect_stroke(
                            bar,
                            0.0,
                            egui::Stroke::new(1.0, ORANGE),
                            egui::StrokeKind::Inside,
                        );
                        if p > 0.0 {
                            let fill = Rect::from_min_size(
                                bar.min,
                                Vec2::new(bar.width() * p, bar.height()),
                            );
                            ui.painter().rect_filled(fill, 0.0, ORANGE);
                        }
                        ui.label(
                            RichText::new(format!("{:>3}%", (p * 100.0) as i32))
                                .family(theme::mono())
                                .size(11.0)
                                .color(CYAN),
                        );
                    });
                }
                body.add_space(12.0);
                body.label(
                    RichText::new("CLICK OR PRESS ANY KEY TO SKIP")
                        .family(theme::mono())
                        .size(11.0)
                        .color(DIM),
                );
            });
    }

    fn ui_index(&mut self, ui: &mut egui::Ui, t: f32) {
        egui::ScrollArea::vertical()
            .id_salt("index")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(12.0);
                    let icon = self.root.join("assets/icon.png");
                    images::show_fit(ui, &mut self.tex, &icon, Vec2::splat(48.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("NETDIR://LOCAL · BLIGHTNET DECK")
                                .family(theme::mono())
                                .size(11.0)
                                .color(MUTED),
                        );
                        ui.label(
                            RichText::new("DECK")
                                .family(theme::display())
                                .size(36.0)
                                .color(ORANGE),
                        );
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(12.0);
                    meta(ui, "CLK", &format_clock(self.clock));
                    meta(
                        ui,
                        "LINK",
                        match self.net.role {
                            Role::Host => "HOST",
                            Role::Guest => "JOIN",
                            Role::Presence => "ONLINE",
                            Role::Idle => "LOCAL",
                        },
                    );
                    meta(ui, "ICE", "CLEAR");
                    meta(ui, "NODE", "8766");
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
        egui::Frame::NONE
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, ORANGE))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                self.ui_index_link_inner(ui, t);
            });
    }

    fn ui_index_link_inner(&mut self, ui: &mut egui::Ui, t: f32) {
        ui.label(
            RichText::new("PRIMARY LINK")
                .family(theme::mono())
                .size(11.0)
                .color(ORANGE),
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
        egui::Frame::NONE
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, ORANGE))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                self.ui_index_systems_inner(ui);
            });
    }

    fn ui_index_systems_inner(&mut self, ui: &mut egui::Ui) {
        self.ensure_devices();
        self.ensure_cameras();
        ui.label(
            RichText::new("DECK SYSTEMS")
                .family(theme::mono())
                .size(11.0)
                .color(ORANGE),
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
                .color(ORANGE),
        );
        wrap_text(
            ui,
            &format!(
                "Auto-detected {} mic{}, {} speaker{}. Update rescans Pulse/PipeWire/ALSA.",
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
        wrap_text(
            ui,
            "UPDATE sits on the top bar next to NET — cyan, always in reach.",
            MUTED,
            11.0,
        );
        ui.horizontal_wrapped(|ui| {
            if self.net.presence {
                if theme::neon_btn_color(ui, "Go offline", KILL, false).clicked() {
                    self.go_offline();
                }
            } else if theme::neon_btn(ui, "Go online").clicked() {
                self.go_online();
            }
        });
        ui.add_space(8.0);
        if theme::sys_tile(ui, "00", "DISCONNECT", "SHUT DOWN BLIGHTNET", "KILL", true).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui_index_log(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, ORANGE))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("DECK LOG")
                        .family(theme::mono())
                        .size(11.0)
                        .color(ORANGE),
                );
                ui.label(
                    RichText::new("data/changelog.md · newest first · scrolls with the deck")
                        .family(theme::mono())
                        .size(11.0)
                        .color(DIM),
                );
                ui.add_space(6.0);
                if self.changelog.is_empty() {
                    wrap_text(ui, "No changelog on this deck.", MUTED, 14.0);
                }
                for (i, (title, bullets)) in self.changelog.iter().take(4).enumerate() {
                    if i > 0 {
                        ui.add_space(10.0);
                    }
                    ui.label(
                        RichText::new(title)
                            .family(theme::display())
                            .size(16.0)
                            .color(CYAN),
                    );
                    for b in bullets {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("▸").color(ORANGE).size(14.0));
                            wrap_text(ui, b, CREAM, 14.0);
                        });
                    }
                }
            });
    }

    fn ui_index_protocol(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(Color32::from_rgb(10, 10, 6))
            .stroke(egui::Stroke::new(1.0, ORANGE))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("RUN PROTOCOL")
                        .family(theme::mono())
                        .size(11.0)
                        .color(ORANGE),
                );
                ui.add_space(6.0);
                for step in [
                    "Stamp a Handle in the top bar.",
                    "Go Online so saved contacts can DM, call, and send pictures without a new invite.",
                    "Host: local network or internet (Cloudflare invite) when you want a mix table.",
                    "Press 01 BLIGHTNEXUS — JACK IN to mix.",
                    "Video Call a contact or a crew. Share camera and screen.",
                    "Chat, Voice, and Crews live in the dock. Text wraps when the deck resizes.",
                ] {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 0.0, ORANGE);
                        wrap_text(ui, step, theme::CREAM, 13.0);
                    });
                    ui.add_space(4.0);
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
                            .color(ORANGE),
                    );
                    wrap_text(ui, last, CYAN, 11.0);
                }
            });
    }

    fn ui_dock(&mut self, ui: &mut egui::Ui) {
        let title = match self.shell {
            ShellPanel::Host => "HOST",
            ShellPanel::Join => "JOIN",
            ShellPanel::Chat => "CHAT",
            ShellPanel::Contacts => "CONTACTS",
            ShellPanel::Voice => "VOICE",
            ShellPanel::Video => "VIDEO",
            ShellPanel::Player => "PLAYER",
            ShellPanel::None => "",
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(title)
                    .family(theme::display())
                    .size(18.0)
                    .color(ORANGE),
            );
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.shell = ShellPanel::None;
            }
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("dock")
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

    fn ui_player_panel(&mut self, ui: &mut egui::Ui) {
        theme::kicker(ui, "NETDIR://PLAYER");
        wrap_text(
            ui,
            "Your files, this deck. Independent of the table mix and radio. Play / Pause / Next sit on the top bar from every page.",
            MUTED,
            12.0,
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("VOL")
                    .family(theme::mono())
                    .size(10.0)
                    .color(ORANGE),
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
                    .add_filter("Audio", &["ogg", "mp3", "wav", "flac", "opus", "m4a", "aac"])
                    .set_title("Add music")
                    .pick_files()
                {
                    self.add_deck_paths(files);
                }
            }
            if theme::neon_btn(ui, "Add folder").clicked() {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Add a music folder")
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
                self.deck_list.clear();
                self.deck_i = 0;
                save_deck_lib(&self.root, &self.deck_list);
            }
        });
        ui.add_space(8.0);
        if self.deck_list.is_empty() {
            wrap_text(
                ui,
                "No tracks yet. Add files or a folder from this machine.",
                MUTED,
                13.0,
            );
            return;
        }
        let n = self.deck_list.len();
        wrap_text(
            ui,
            &format!("{n} TRACK{}", if n == 1 { "" } else { "S" }),
            ORANGE,
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
        for (i, name) in names {
            let on = self.deck_i == i && self.deck_on;
            if theme::wide_btn(
                ui,
                &name,
                if self.deck_i == i {
                    if self.deck_on {
                        "NOW PLAYING"
                    } else {
                        "SELECTED"
                    }
                } else {
                    ""
                },
                on,
            )
            .clicked()
            {
                self.deck_i = i;
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
        ui.add_space(8.0);
        if self.net.role == Role::Idle {
            ui.label(
                RichText::new("LOCAL NETWORK")
                    .family(theme::mono())
                    .size(11.0)
                    .color(ORANGE),
            );
            ui.label(
                RichText::new(
                    "Same house or the same Wi-Fi. Friends Join with a blightnet:// LAN address.",
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
                    .color(ORANGE),
            );
            ui.label(
                RichText::new("Other networks. Opens a free Cloudflare tunnel and gives you an https invite link. No account. Friends paste that link into Join.")
                    .color(CREAM)
                    .small(),
            );
            if theme::neon_btn(ui, "Host on the internet").clicked() {
                self.start_host(true);
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
                    .color(ORANGE),
            );
            if theme::neon_btn(ui, "Copy LAN address").clicked() {
                ui.ctx().copy_text(lan.clone());
                self.chat.push(format!("Copied {lan}"));
            }
            if self.net.internet {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Invite link (any network)")
                        .family(theme::mono())
                        .size(10.0)
                        .color(DIM),
                );
                if self.net.public_url.is_empty() {
                    ui.label(
                        RichText::new("Waiting on Cloudflare… keep this open.")
                            .color(CYAN),
                    );
                } else {
                    ui.label(
                        RichText::new(&self.net.public_url)
                            .family(theme::mono())
                            .size(12.0)
                            .color(CYAN),
                    );
                    if theme::neon_btn(ui, "Copy invite link").clicked() {
                        ui.ctx().copy_text(self.net.public_url.clone());
                        self.chat.push(format!(
                            "Copied. Friends on any network paste this into Join:\n{}",
                            self.net.public_url
                        ));
                    }
                }
            }
            ui.add_space(8.0);
            if theme::neon_btn_color(ui, "Leave table", KILL, true).clicked() {
                self.leave_table();
            }
        }
    }

    fn ui_join_panel(&mut self, ui: &mut egui::Ui) {
        theme::kicker(ui, "NETDIR://JOIN");
        ui.label(
            RichText::new(
                "Paste a blightnet:// LAN address, or an https:// invite from Internet Host.",
            )
            .color(MUTED)
            .small(),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.join_in)
                .hint_text("blightnet://192.168.0.12:8766  or  https://….trycloudflare.com")
                .desired_width(ui.available_width()),
        );
        ui.horizontal(|ui| {
            if theme::neon_btn(ui, "Connect").clicked() {
                let addr = self.join_in.clone();
                self.do_join(&addr);
            }
        });
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
            if theme::neon_btn_color(ui, "Table", ORANGE, self.chat_target == ChatTarget::Table)
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
                if theme::neon_btn_color(ui, &lab, if self.contact_online(&c.id) { CYAN } else { ORANGE }, on)
                    .clicked()
                {
                    self.chat_target = ChatTarget::Dm(c.id.clone());
                    self.whisper_to = Some(c.id.clone());
                }
            }
            let crews = self.crews.clone();
            for crew in &crews {
                let on = matches!(&self.chat_target, ChatTarget::Crew(id) if id == &crew.id);
                if theme::neon_btn_color(ui, &format!("crew:{}", crew.name), ORANGE, on).clicked() {
                    self.chat_target = ChatTarget::Crew(crew.id.clone());
                }
            }
        });
        ui.add_space(6.0);
        let chat_n = self.chat.len();
        let start = chat_n.saturating_sub(500);
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
        ui.label(RichText::new("NEARBY / TABLE").family(theme::mono()).size(10.0).color(ORANGE));
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
        ui.label(RichText::new("SAVED").family(theme::mono()).size(10.0).color(ORANGE));
        if self.contacts.is_empty() {
            wrap_text(ui, "No contacts yet. Add someone at a table or nearby.", MUTED, 11.0);
        }
        let saved = self.contacts.clone();
        for (i, c) in saved.iter().enumerate() {
            let on = self.contact_online(&c.id);
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&c.name).color(ORANGE));
                    wrap_text(
                        ui,
                        if on { "online" } else { "offline" },
                        if on { CYAN } else { DIM },
                        11.0,
                    );
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
        ui.label(RichText::new("CREWS").family(theme::mono()).size(10.0).color(ORANGE));
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
            wrap_text(ui, &format!("▸ {}", crew.name), ORANGE, 14.0);
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
            if theme::neon_btn_color(ui, lab, ORANGE, self.voice_on).clicked() {
                self.voice_on = !self.voice_on;
                if self.voice_on {
                    self.ensure_mic();
                    self.net.send_voice("table-on", None);
                } else {
                    self.net.send_voice("table-off", None);
                }
            }
            if theme::neon_btn_color(ui, if self.voice_mute { "Muted" } else { "Mute" }, ORANGE, self.voice_mute)
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
                    self.call_id = Some(id.clone());
                    self.incoming = None;
                    self.voice_on = true;
                    self.ensure_mic();
                }
                if theme::neon_btn_color(ui, "Decline", KILL, true).clicked() {
                    self.net.send_voice("decline", Some(id.clone()));
                    self.incoming = None;
                }
            });
        }
        if self.call_id.is_some() || self.call_crew.is_some() {
            if theme::neon_btn_color(ui, "Hang up", KILL, true).clicked() {
                self.hang_up();
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new("PEOPLE").family(theme::mono()).size(10.0).color(ORANGE));
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
                .color(ORANGE),
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
                if theme::neon_btn_color(ui, "Camera", ORANGE, self.cam_on).clicked() {
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
                if theme::neon_btn_color(ui, if self.voice_mute { "Muted" } else { "Mute" }, ORANGE, self.voice_mute)
                    .clicked()
                {
                    self.voice_mute = !self.voice_mute;
                }
                if theme::neon_btn_color(ui, "Hang up", KILL, true).clicked() {
                    self.hang_up();
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("YOU")
                    .family(theme::mono())
                    .size(10.0)
                    .color(DIM),
            );
            ui.horizontal_wrapped(|ui| {
                images::show_bytes(ui, &mut self.tex, "local-cam", &self.local_cam, Vec2::new(220.0, 160.0));
                if !self.local_screen.is_empty() {
                    images::show_bytes(
                        ui,
                        &mut self.tex,
                        "local-screen",
                        &self.local_screen,
                        Vec2::new(280.0, 160.0),
                    );
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("THEM")
                    .family(theme::mono())
                    .size(10.0)
                    .color(ORANGE),
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
                ui.horizontal_wrapped(|ui| {
                    if let Some(feed) = self.remote_vid.get(id) {
                        images::show_bytes(
                            ui,
                            &mut self.tex,
                            &format!("cam-{id}"),
                            &feed.cam,
                            Vec2::new(220.0, 160.0),
                        );
                    }
                    if self
                        .remote_vid
                        .get(id)
                        .map(|f| !f.screen.is_empty())
                        .unwrap_or(false)
                    {
                        if let Some(feed) = self.remote_vid.get(id) {
                            images::show_bytes(
                                ui,
                                &mut self.tex,
                                &format!("scr-{id}"),
                                &feed.screen,
                                Vec2::new(280.0, 160.0),
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
                .color(ORANGE),
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
                .color(ORANGE),
        );
        let crews = self.crews.clone();
        for crew in &crews {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&crew.name).color(ORANGE));
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
            }
        }
        let pal = self.pal();
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                self.ui_table_top(ui, pal);
                if self.watch_open || self.place_open {
                    self.ui_table_expand(ui, pal);
                }
                self.ui_table_bar(ui, pal);
                let chars_on = self.panel_on(Overlay::Chars);
                let rest = ui.available_rect_before_wrap();
                ui.allocate_rect(rest, egui::Sense::hover());
                let char_w = if chars_on {
                    (rest.width() * 0.38)
                        .clamp(320.0, 460.0)
                        .min(rest.width() * 0.48)
                } else {
                    0.0
                };
                let (main, sheet) = if chars_on && char_w > 120.0 {
                    rest.split_left_right_at_x(rest.right() - char_w)
                } else {
                    (rest, Rect::from_min_size(rest.right_top(), Vec2::ZERO))
                };
                let log_h = 168.0_f32.min(main.height() * 0.34).max(120.0);
                let (play, logs) = main.split_top_bottom_at_y(main.bottom() - log_h);
                let mut main_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(play)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let tiles = self.open.iter().any(|o| *o != Overlay::Chars);
                let jack_dive = self.panel_on(Overlay::Jackin)
                    && self.open.iter().all(|o| *o == Overlay::Jackin || *o == Overlay::Chars);
                if tiles {
                    self.ui_tiled_panels(&mut main_ui, pal);
                }
                if !jack_dive {
                    self.ui_table_sky(&mut main_ui, pal);
                    self.ui_table_stage(&mut main_ui, pal);
                }
                self.ui_dice_fx(&mut main_ui);
                self.ui_table_logs(ui, logs);
                if chars_on && sheet.width() > 80.0 {
                    ui.painter().vline(
                        sheet.left(),
                        rest.y_range(),
                        egui::Stroke::new(2.0, ORANGE),
                    );
                    theme::plate(ui, sheet.shrink(4.0));
                    let mut sheet_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(sheet.shrink2(Vec2::new(10.0, 8.0)))
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    self.ui_chars_panel(&mut sheet_ui, pal);
                }
            },
        );
    }

    fn ui_tiled_panels(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let panels: Vec<Overlay> = self
            .open
            .iter()
            .copied()
            .filter(|o| *o != Overlay::Chars)
            .collect();
        let n = panels.len();
        if n == 0 {
            return;
        }
        let jack = panels.iter().any(|o| *o == Overlay::Jackin);
        let frac = if jack && n == 1 {
            0.97
        } else if jack {
            0.88
        } else {
            (0.36 + 0.10 * n.min(3) as f32).min(0.72)
        };
        let h = ui.available_height() * frac;
        let rect = ui.available_rect_before_wrap();
        let board = Rect::from_min_size(rect.min, Vec2::new(rect.width(), h.max(160.0)));
        ui.allocate_rect(board, egui::Sense::hover());
        let inner = board.shrink2(Vec2::new(6.0, 4.0));
        let cells: Vec<Rect> = match n {
            0 => vec![],
            1 => vec![inner],
            2 => {
                let (a, b) = inner.split_left_right_at_x(inner.left() + inner.width() * 0.5);
                vec![a.shrink(4.0), b.shrink(4.0)]
            }
            _ => {
                let (top, bot) = inner.split_top_bottom_at_y(inner.top() + inner.height() * 0.55);
                let (a, b) = top.split_left_right_at_x(top.left() + top.width() * 0.5);
                let mut v = vec![a.shrink(3.0), b.shrink(3.0)];
                if n == 3 {
                    v.push(bot.shrink(3.0));
                } else {
                    let (c, d) = bot.split_left_right_at_x(bot.left() + bot.width() * 0.5);
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
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell.shrink(8.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            match kind {
                Overlay::Chars => {}
                Overlay::Maps => self.ui_overlay_maps(&mut child, pal),
                Overlay::Catalog => self.ui_overlay_catalog(&mut child, pal),
                Overlay::Blackjack => self.ui_overlay_bj(&mut child, pal),
                Overlay::Armory => self.ui_overlay_kit(&mut child, pal, false),
                Overlay::Vendors => self.ui_overlay_kit(&mut child, pal, true),
                Overlay::Jackin => self.ui_overlay_jackin(&mut child, pal),
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
                ui.horizontal_wrapped(|ui| {
                    let icon = self.root.join("assets/icon.png");
                    images::show_fit(ui, &mut self.tex, &icon, Vec2::splat(28.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(if self.blight { "BLIGHT" } else { "HEARTHSONG" })
                                .family(theme::display())
                                .size(15.0)
                                .color(ORANGE),
                        );
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
                    if theme::neon_btn_color(ui, &self.place_name(), ORANGE, true).clicked() {
                        self.place_open = !self.place_open;
                        self.watch_open = false;
                    }
                    if theme::neon_btn_color(ui, "Outside", ORANGE, !self.inside).clicked() {
                        self.set_inside(false);
                    }
                    if theme::neon_btn_color(ui, "Inside", ORANGE, self.inside).clicked() {
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
                                .color(ORANGE),
                        );
                        ui.label(
                            RichText::new(period_label(self.time))
                                .family(theme::mono())
                                .size(10.0)
                                .color(CYAN),
                        );
                    });
                    for (id, lab) in [
                        ("morning", "Morning"),
                        ("day", "Day"),
                        ("evening", "Evening"),
                        ("night", "Night"),
                    ] {
                        if theme::neon_btn_color(ui, lab, ORANGE, self.time == id).clicked() {
                            self.set_period(id);
                        }
                    }
                    ui.label(
                        RichText::new("MASTER")
                            .family(theme::mono())
                            .size(10.0)
                            .color(DIM),
                    );
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
                        ORANGE,
                        faded,
                    )
                    .clicked()
                    {
                        self.fade_mix();
                    }
                    if theme::neon_btn_color(ui, "Silence", KILL, false).clicked() {
                        self.mixer.silence();
                        self.mix_msg = "Silent.".into();
                    }
                    ui.label(RichText::new("SEAT").family(theme::mono()).size(10.0).color(DIM));
                    if theme::neon_btn_color(ui, "Player", ORANGE, !self.is_gm).clicked() {
                        self.is_gm = false;
                    }
                    if theme::neon_btn_color(ui, "GM", ORANGE, self.is_gm).clicked() {
                        self.is_gm = true;
                    }
                });
            });
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(2.0, ORANGE),
        );
    }

    fn ui_table_bar(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        let shown = egui::Frame::NONE
            .fill(Color32::from_rgb(8, 8, 6))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                ui.horizontal_wrapped(|ui| {
                    let overlay_cat = self.overlay_cat;
                    let catalog_open = self.panel_on(Overlay::Catalog);
                    let cat_on = |name: &str| catalog_open && overlay_cat == name;
                    cluster(ui, "SHEET", |ui| {
                        if theme::neon_btn_color(
                            ui,
                            "Characters",
                            ORANGE,
                            self.panel_on(Overlay::Chars),
                        )
                        .clicked()
                        {
                            self.toggle_overlay(Overlay::Chars);
                        }
                    });
                    cluster(ui, "BOOKS", |ui| {
                        if self.blight {
                            if theme::neon_btn_color(ui, "Datashard", ORANGE, cat_on("Datashard"))
                                .clicked()
                            {
                                self.open_table_catalog("Datashard", "datashard.json");
                            }
                            if theme::neon_btn_color(ui, "Faces", ORANGE, cat_on("Faces")).clicked()
                            {
                                self.open_table_catalog("Faces", "npcs.json");
                            }
                            if theme::neon_btn_color(ui, "Corps", ORANGE, cat_on("Corps")).clicked()
                            {
                                self.open_table_catalog("Corps", "corps.json");
                            }
                            if theme::neon_btn_color(ui, "Gangs", ORANGE, cat_on("Gangs")).clicked()
                            {
                                self.open_table_catalog("Gangs", "gangs.json");
                            }
                            if theme::neon_btn_color(ui, "Lore", ORANGE, cat_on("Lore")).clicked() {
                                self.open_table_catalog("Lore", "lore.json");
                            }
                        } else {
                            if theme::neon_btn_color(ui, "Bestiary", ORANGE, cat_on("Bestiary"))
                                .clicked()
                            {
                                self.open_table_catalog("Bestiary", "bestiary.json");
                            }
                            if theme::neon_btn_color(ui, "NPCs", ORANGE, cat_on("NPCs")).clicked() {
                                self.open_table_catalog("NPCs", "srd-npcs.json");
                            }
                            if theme::neon_btn_color(ui, "Gods", ORANGE, cat_on("Gods")).clicked() {
                                self.open_table_catalog("Gods", "gods.json");
                            }
                        }
                    });
                    cluster(ui, "GEAR", |ui| {
                        if theme::neon_btn_color(
                            ui,
                            "Armory",
                            ORANGE,
                            self.panel_on(Overlay::Armory),
                        )
                        .clicked()
                        {
                            self.kit_filter.clear();
                            self.toggle_overlay(Overlay::Armory);
                        }
                        let vendor = if self.blight { "Night Market" } else { "Vendors" };
                        if theme::neon_btn_color(ui, vendor, ORANGE, self.panel_on(Overlay::Vendors))
                            .clicked()
                        {
                            self.kit_filter.clear();
                            self.toggle_overlay(Overlay::Vendors);
                        }
                    });
                    cluster(ui, "MORE", |ui| {
                        if self.blight
                            && theme::neon_btn_color(
                                ui,
                                "21",
                                ORANGE,
                                self.panel_on(Overlay::Blackjack),
                            )
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Blackjack);
                        }
                        if theme::neon_btn(ui, "Add sound").clicked() {
                            self.pick_sound();
                        }
                        if theme::neon_btn_color(ui, "Maps", ORANGE, self.panel_on(Overlay::Maps))
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Maps);
                        }
                        if self.blight
                            && theme::neon_btn_color(
                                ui,
                                "Jack-in",
                                CYAN,
                                self.panel_on(Overlay::Jackin),
                            )
                            .clicked()
                        {
                            self.toggle_overlay(Overlay::Jackin);
                        }
                        if theme::neon_btn_color(
                            ui,
                            "Nethooks",
                            ORANGE,
                            self.page == Page::Nethooks,
                        )
                        .clicked()
                        {
                            self.page = Page::Nethooks;
                            self.hook_edit = false;
                        }
                    });
                });
            });
        ui.painter().hline(
            shown.response.rect.x_range(),
            shown.response.rect.bottom(),
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 110)),
        );
    }

    #[allow(dead_code)]
    fn ui_table_overlay(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = (ui, pal);
    }

    fn ui_overlay_catalog(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "06", &self.overlay_cat.to_uppercase());
            ui.add(
                egui::TextEdit::singleline(&mut self.catalog_q)
                    .hint_text("Find…")
                    .desired_width(180.0),
            );
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.close_panel(Overlay::Catalog);
            }
        });
        self.refresh_cat_cache();
        ui.columns(2, |cols| {
            let n = self.cat_cache.len();
            egui::ScrollArea::vertical()
                .id_salt("ov-cat-list")
                .show_rows(&mut cols[0], 50.0, n, |ui, range| {
                    for row in range {
                        let (i, name, extra) = self.cat_cache[row].clone();
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
                        maps::drag_source(
                            ui,
                            ("cat", i),
                            TokenSpec {
                                name: name.clone(),
                                image: art,
                                sheet: sheet.clone(),
                                cat: self.overlay_cat.to_string(),
                                src: sheet,
                            },
                            |ui| {
                                if theme::wide_btn(ui, &name, &extra, on).clicked() {
                                    self.catalog_pick = i;
                                }
                            },
                        );
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
                                    .color(ORANGE),
                            );
                        }
                        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
                            let art = catalog_art(&self.root, self.overlay_cat, id);
                            if art.is_file() {
                                if images::show_fit(
                                    ui,
                                    &mut self.tex,
                                    &art,
                                    Vec2::new(220.0, 140.0),
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
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.close_panel(Overlay::Jackin);
            }
        });
        crate::netspace::paint(ui, &mut self.netspace, self.jack_at.elapsed().as_secs_f32());
    }

    fn ui_overlay_maps(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.close_panel(Overlay::Maps);
            }
        });
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
        if self.live_link() {
            for op in ops {
                match op {
                    MapOp::Add(mark) => self.net.send_map_mark(mark),
                    MapOp::Del(id) => self.net.send_map_mark_del(&id),
                    MapOp::Clear => self.net.send_map_marks_clear(),
                }
            }
        }
    }

    fn ui_overlay_bj(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        ui.horizontal_wrapped(|ui| {
            theme::section_head(ui, "04", "HOUSE 21");
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.close_panel(Overlay::Blackjack);
            }
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
            if vendor && theme::neon_btn(ui, "Shuffle stall").clicked() {
                self.restock_vendor();
            }
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                self.close_panel(if vendor {
                    Overlay::Vendors
                } else {
                    Overlay::Armory
                });
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
        let rows = if vendor {
            if self.vendor_stock.is_empty() {
                self.restock_vendor();
            }
            self.vendor_stock.clone()
        } else {
            load_list(&self.root, file)
        };
        let q = self.kit_filter.to_lowercase();
        let mut bought: Option<KitSpec> = None;
        let mut paid = 0;
        let mut fail = String::new();
        egui::ScrollArea::vertical()
            .id_salt("kit")
            .show(ui, |ui| {
                for v in &rows {
                    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    let cat = v.get("cat").and_then(|x| x.as_str()).unwrap_or("");
                    let cost = v.get("cost").and_then(|x| x.as_str()).unwrap_or("");
                    let dmg = v.get("dmg").and_then(|x| x.as_str()).unwrap_or("");
                    if !q.is_empty() {
                        let blob = format!("{} {} {}", name, cat, kit_blurb(v)).to_lowercase();
                        if !blob.contains(&q) {
                            continue;
                        }
                    }
                    let spec = crate::chars::kit_spec_from_json(v);
                    let blurb = kit_blurb(v);
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let id = spec.id.clone();
                        if let Some(art) = images::kit_art(&self.root, &id, name) {
                            if images::show_fit(ui, &mut self.tex, &art, Vec2::splat(56.0))
                                .on_hover_text("Click to zoom")
                                .clicked()
                            {
                                self.zoom_path = Some(art);
                                self.zoom_key = None;
                            }
                        }
                        ui.vertical(|ui| {
                            let row = |ui: &mut egui::Ui| {
                                ui.label(
                                    RichText::new(name)
                                        .color(ORANGE)
                                        .size(16.0)
                                        .family(theme::ui_font()),
                                );
                                if !cat.is_empty() || !cost.is_empty() || !dmg.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            [cat, cost, dmg]
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
                                let _ = ui.dnd_drag_source(
                                    egui::Id::new(("kit-drag", spec.id.clone())),
                                    spec.clone(),
                                    |ui| row(ui),
                                );
                            } else {
                                row(ui);
                            }
                            if !blurb.is_empty() {
                                wrap_text(ui, &blurb, CREAM, 14.0);
                            }
                        });
                        if vendor {
                            if theme::neon_btn(ui, "Buy").clicked() {
                                let price = parse_coins(cost);
                                let lvl = crate::chars::item_min_level(v);
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

    fn ui_table_sky(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        let _ = pal;
        let h = 200.0;
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
        theme::brackets(ui, inner.shrink(6.0), ORANGE, 14.0);
        let paint = ui.painter().with_clip_rect(inner);
        let mut top_y = inner.top() + 12.0;
        if self.blight {
            let chip = Rect::from_min_size(inner.left_top() + Vec2::new(12.0, 12.0), Vec2::new(176.0, 36.0));
            theme::fill_chamfer(ui, chip, 6.0, PANEL, egui::Stroke::new(1.0, CYAN));
            let st = RADIO
                .iter()
                .find(|s| s.id == self.radio_station)
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
            ORANGE,
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
            ORANGE,
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
                for st in RADIO {
                    let on = self.radio_on && self.radio_station == st.id;
                    if theme::neon_btn_color(ui, &format!("{} {}", st.freq, st.call), CYAN, on)
                        .clicked()
                    {
                        if on {
                            self.play_radio_track();
                        } else {
                            self.tune_station(st.id);
                        }
                    }
                }
                if self.radio_on && theme::neon_btn_color(ui, "Off air", KILL, false).clicked() {
                    self.mixer.stop("__radio");
                    self.radio_on = false;
                    self.radio_track.clear();
                    self.radio_station.clear();
                }
            });
            if self.radio_on {
                let name = RADIO
                    .iter()
                    .find(|s| s.id == self.radio_station)
                    .map(|s| s.name)
                    .unwrap_or("Station");
                let track = self
                    .catalog
                    .layer(true, &self.radio_track)
                    .map(|l| l.name.as_str())
                    .unwrap_or("…");
                wrap_text(ui, &format!("{name} · {track}"), CYAN, 11.0);
            }
        }
    }

    fn ui_table_stage(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        ui.add_space(8.0);
        if ui.available_width() < 700.0 {
            self.ui_scenes_panel(ui, pal);
            ui.add_space(8.0);
            self.ui_mix_panel(ui, pal);
        } else {
            ui.columns(2, |cols| {
                self.ui_scenes_panel(&mut cols[0], pal);
                self.ui_mix_panel(&mut cols[1], pal);
            });
        }
    }

    fn ui_scenes_panel(&mut self, ui: &mut egui::Ui, pal: theme::Palette) {
        theme::plate(ui, ui.max_rect());
        ui.add_space(10.0);
        theme::section_head(ui, "01", "SCENES");
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
        let scenes: Vec<_> = self
            .catalog
            .scenes(self.blight)
            .iter()
            .filter(|s| {
                q.is_empty()
                    || s.name.to_lowercase().contains(&q)
                    || s.blurb.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("scenes")
            .show(ui, |ui| {
                for (i, name) in &saved {
                    if theme::wide_btn(ui, &format!("★ {name}"), "saved mix", false).clicked() {
                        self.apply_saved(*i);
                    }
                }
                for s in scenes {
                    let on = self.scene == s.id;
                    if theme::wide_btn(ui, &s.name, &s.blurb, on).clicked() {
                        self.apply_scene(&s.id);
                    }
                }
            });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
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
                if theme::neon_btn_color(ui, c, ORANGE, self.cat_filter == c).clicked() {
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
                        if theme::neon_btn_color(ui, &name, ORANGE, on).clicked() {
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

    fn ui_table_logs(&mut self, ui: &mut egui::Ui, rect: Rect) {
        theme::plate(ui, rect.shrink(2.0));
        let inner = rect.shrink2(Vec2::new(10.0, 6.0));
        let mid = inner.top() + inner.height() * 0.52;
        let (top, bot) = inner.split_top_bottom_at_y(mid);
        let mut combat_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(top)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        combat_ui.label(
            RichText::new("COMBAT LOG")
                .family(theme::mono())
                .size(11.0)
                .color(ORANGE),
        );
        egui::ScrollArea::vertical()
            .id_salt("combat-log")
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(&mut combat_ui, |ui| {
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
        ui.painter().hline(
            bot.x_range(),
            bot.top(),
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 90)),
        );
        let mut notes_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bot.shrink2(Vec2::new(0.0, 2.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        notes_ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("PRIVATE NOTES")
                    .family(theme::mono())
                    .size(11.0)
                    .color(CYAN),
            );
            ui.label(
                RichText::new("this deck only · other players never see this")
                    .family(theme::mono())
                    .size(10.0)
                    .color(DIM),
            );
        });
        let nw = notes_ui.available_width().max(80.0);
        let resp = notes_ui.add(
            egui::TextEdit::multiline(&mut self.notes)
                .desired_width(nw)
                .desired_rows(3)
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
        ui.horizontal_wrapped(|ui| {
            if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                crate::chars::save(&self.root, &self.chars);
                self.close_panel(Overlay::Chars);
            }
        });
        self.run_sheet(ui);
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
                            self.set_clock(self.clock.saturating_sub(15));
                        }
                        ui.label(
                            RichText::new(format_clock(self.clock))
                                .family(theme::mono())
                                .size(16.0)
                                .color(ORANGE),
                        );
                        if theme::neon_btn(ui, "+").clicked() {
                            self.set_clock(self.clock + 15);
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
                        if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                            self.watch_open = false;
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
                            if theme::neon_btn_color(ui, name, ORANGE, self.place == *id).clicked() {
                                self.place = id.clone();
                            }
                        }
                        if theme::neon_btn_color(ui, "Close", KILL, true).clicked() {
                            self.place_open = false;
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
            ui.painter().rect_filled(
                felt,
                0.0,
                Color32::from_rgb(10, 10, 6),
            );
            ui.label(
                RichText::new("HOUSE 21")
                    .family(theme::display())
                    .size(22.0)
                    .color(ORANGE),
            );
            ui.label(
                RichText::new(&self.bj.msg)
                    .family(theme::mono())
                    .size(14.0)
                    .color(CYAN),
            );
            ui.label(
                RichText::new(format!("BANK {} eb   BET {}", self.bj.bank, self.bj.bet))
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
                            .color(ORANGE),
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
                    if self.bj.bet > self.bj.bank {
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
                            self.bj.msg = "Hit or stand.".into();
                        }
                    }
                }
                if self.bj.live && theme::neon_btn(ui, "Hit").clicked() {
                    self.bj.player.push(card());
                    if total(&self.bj.player) > 21 {
                        self.bj.live = false;
                        self.bj.msg = "Bust.".into();
                    }
                }
                if self.bj.live && theme::neon_btn(ui, "Stand").clicked() {
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
                }
            });
    }

    fn ui_catalog(&mut self, ui: &mut egui::Ui) {
        let title = match self.page {
            Page::Catalog(n) => n,
            _ => "Catalog",
        };
        ui.horizontal(|ui| {
            ui.heading(RichText::new(title).family(theme::display()).color(ORANGE));
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
            ui.columns(2, |cols| {
                egui::ScrollArea::vertical().show(&mut cols[0], |ui| {
                    for (i, name, extra) in &rows {
                        let on = self.catalog_pick == *i;
                        if ui.selectable_label(on, format!("{name}\n{extra}")).clicked() {
                            self.catalog_pick = *i;
                        }
                    }
                });
                egui::ScrollArea::vertical().show(&mut cols[1], |ui| {
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
                                    Vec2::new(280.0, 220.0),
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
        self.run_sheet(ui);
    }

    fn run_sheet(&mut self, ui: &mut egui::Ui) {
        let before = self.combat_log.len();
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
        );
        if self.zoom_path.is_some() {
            self.zoom_key = None;
        }
        if self.combat_log.len() > before {
            let fresh: Vec<String> = self.combat_log[before..].to_vec();
            for line in &fresh {
                self.net.send_chat(&format!("{COMBAT_MARK}{line}"), None, false);
            }
            save_combat_log(&self.root, &self.combat_log);
        }
        self.apply_roll_hp();
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
        let size = Vec2::new(260.0, 128.0);
        let pad = Vec2::new(16.0, 16.0);
        let boxr = Rect::from_min_size(
            egui::pos2(
                (rect.right() - size.x - pad.x).max(rect.left() + 8.0),
                (rect.bottom() - size.y - pad.y).max(rect.top() + 8.0),
            ),
            size,
        );
        ui.painter().rect_filled(
            boxr,
            8.0,
            Color32::from_rgba_unmultiplied(12, 12, 8, 230),
        );
        ui.painter().rect_stroke(
            boxr,
            8.0,
            egui::Stroke::new(2.0, ORANGE),
            egui::StrokeKind::Outside,
        );
        let shown = r.display_pct();
        let color = match r.grade {
            "critical success" if r.done() => CYAN,
            "critical failure" if r.done() => KILL,
            _ => ORANGE,
        };
        ui.painter().text(
            boxr.center() + Vec2::new(0.0, -28.0),
            egui::Align2::CENTER_CENTER,
            format!("{shown}"),
            FontId::new(56.0, theme::display()),
            color,
        );
        if r.done() {
            ui.painter().text(
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
                ui.painter().text(
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
                    egui::Stroke::new(2.0, ORANGE),
                    egui::StrokeKind::Inside,
                );
                theme::brackets(ui, frame, ORANGE, 16.0);
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
            if theme::neon_btn(ui, "+ New nethook").clicked() {
                let h = crate::nethook::Nethook::fresh(&self.net.self_id, &self.handle);
                self.hook_draft_title = h.title.clone();
                self.hook_draft_html = h.html.clone();
                self.nethooks.insert(0, h);
                self.hook_i = 0;
                self.hook_edit = true;
            }
        });
        ui.label(
            RichText::new("NETHOOKS")
                .family(theme::display())
                .size(28.0)
                .color(ORANGE),
        );
        wrap_text(
            ui,
            "User sites on the grid. Same INDEX chrome. Host, Join, or Go Online and every table sees new pages as they go up. Only the original runner can edit or delete theirs.",
            MUTED,
            13.0,
        );
        ui.add_space(8.0);
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("nethook-list")
                .show(&mut cols[0], |ui| {
                    ui.label(
                        RichText::new("DIRECTORY")
                            .family(theme::mono())
                            .size(11.0)
                            .color(ORANGE),
                    );
                    if self.nethooks.is_empty() {
                        wrap_text(ui, "No nethooks on this deck yet. Write one.", MUTED, 13.0);
                    }
                    let mine = self.net.self_id.clone();
                    for i in 0..self.nethooks.len() {
                        let title = self.nethooks[i].title.clone();
                        let owner = self.nethooks[i].owner_name.clone();
                        let yours = self.nethooks[i].owner_id == mine;
                        let on = self.hook_i == i;
                        let extra = if yours {
                            format!("{owner} · YOURS")
                        } else {
                            owner
                        };
                        if theme::wide_btn(ui, &title, &extra, on).clicked() {
                            self.hook_i = i;
                            self.hook_edit = false;
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("nethook-page")
                .show(&mut cols[1], |ui| {
                    self.ui_nethook_page(ui);
                });
        });
    }

    fn ui_nethook_page(&mut self, ui: &mut egui::Ui) {
        if self.nethooks.is_empty() {
            wrap_text(ui, "Press + New nethook to put a site on the grid.", MUTED, 14.0);
            return;
        }
        self.hook_i = self.hook_i.min(self.nethooks.len() - 1);
        let mine = self.net.self_id.clone();
        let owned = self
            .nethooks
            .get(self.hook_i)
            .map(|h| h.owner_id == mine)
            .unwrap_or(false);
        if self.hook_edit && owned {
            ui.label(
                RichText::new("EDITOR")
                    .family(theme::mono())
                    .size(11.0)
                    .color(CYAN),
            );
            wrap_text(
                ui,
                "Simple HTML: h1 h2 h3 p ul li br hr. Scripts are stripped. INDEX chrome on the page.",
                MUTED,
                12.0,
            );
            ui.label(RichText::new("Title").family(theme::mono()).size(10.0).color(DIM));
            ui.add(
                egui::TextEdit::singleline(&mut self.hook_draft_title)
                    .desired_width(ui.available_width())
                    .font(FontId::new(16.0, theme::display()))
                    .text_color(ORANGE),
            );
            ui.label(RichText::new("HTML").family(theme::mono()).size(10.0).color(DIM));
            ui.add(
                egui::TextEdit::multiline(&mut self.hook_draft_html)
                    .desired_width(ui.available_width())
                    .desired_rows(12)
                    .font(FontId::new(13.0, theme::mono()))
                    .text_color(CREAM),
            );
            ui.horizontal_wrapped(|ui| {
                if theme::neon_btn(ui, "Save").clicked() {
                    self.save_hook_draft();
                }
                if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
                    self.cancel_hook_edit();
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new("PREVIEW")
                    .family(theme::mono())
                    .size(11.0)
                    .color(ORANGE),
            );
            let title = self.hook_draft_title.clone();
            let html = self.hook_draft_html.clone();
            crate::nethook::chrome_frame(ui, &title, &self.handle, |ui| {
                crate::nethook::paint(ui, &html);
            });
            return;
        }
        let title = self.nethooks[self.hook_i].title.clone();
        let owner = self.nethooks[self.hook_i].owner_name.clone();
        let html = self.nethooks[self.hook_i].html.clone();
        let mut gone = false;
        ui.horizontal_wrapped(|ui| {
            if owned && theme::neon_btn(ui, "Edit").clicked() {
                self.hook_draft_title = title.clone();
                self.hook_draft_html = html.clone();
                self.hook_edit = true;
            }
            if owned && theme::neon_btn_color(ui, "Delete", KILL, false).clicked() {
                self.delete_current_hook();
                gone = true;
            }
            if !owned {
                wrap_text(ui, "Read only. You did not write this site.", MUTED, 12.0);
            }
        });
        if gone || self.nethooks.is_empty() {
            return;
        }
        crate::nethook::chrome_frame(ui, &title, &owner, |ui| {
            crate::nethook::paint(ui, &html);
        });
    }

    fn save_hook_draft(&mut self) {
        let Some(h) = self.nethooks.get_mut(self.hook_i) else {
            return;
        };
        if h.owner_id != self.net.self_id {
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
            self.net.send_nethook_put(saved);
        }
        self.chat.push("Nethook saved. Live tables pick it up.".into());
    }

    fn cancel_hook_edit(&mut self) {
        self.hook_edit = false;
        let Some(h) = self.nethooks.get(self.hook_i) else {
            return;
        };
        if h.owner_id != self.net.self_id {
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
        if h.owner_id != self.net.self_id {
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

    fn ui_tutorial(&mut self, ui: &mut egui::Ui) {
            if theme::neon_btn(ui, "← INDEX").clicked() {
                self.page = Page::Index;
            }
            ui.heading(RichText::new("Field manual").family(theme::display()).color(ORANGE));
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label("This is the native Blightnet window. There is no web browser.");
                ui.label("INDEX 01 TABLE opens the mixer. Press a scene. Sounds loop until you silence them.");
                ui.label("Stamp a Handle. Host asks local network or internet. Local: blightnet:// LAN address. Internet: a Cloudflare https invite. Friends press Join and paste it.");
                ui.label("Chat and Voice use that same link. Contacts remember people you add at a table. Chat can send any file. Incoming media streams; Download keeps a copy. Chat is permanent until Wipe chat.");
                ui.label("On Maps the gamemaster can ink, stamp filled circles and squares to fog the board, and erase those drawings.");
                ui.label("INDEX Nethooks are user sites on the grid. There is a NETHOOKS tab next to INDEX and TABLE. Write simple HTML in the built-in editor. Host, Join, or Go Online and everyone else sees new nethooks. Only the original author can edit or delete theirs.");
                ui.label("PLAY on the top bar is your music player: files from this machine, independent of the table mix. Library adds tracks. UPDATE is the cyan button next to NET.");
                ui.label("Master is local. Place changes the painting. Time is the watch.");
                ui.label("Hearthsong is the fantasy table. Blight is the Night City table. Catalogs live on TABLE.");
                ui.label(RichText::new("Assets load from the same folder: audio/, assets/, data/.").color(MUTED));
            });
    }

    fn ui_audio(&mut self, ui: &mut egui::Ui) {
            if theme::neon_btn(ui, "← INDEX").clicked() {
                self.page = Page::Index;
            }
            ui.heading(RichText::new("Audio").family(theme::display()).color(ORANGE));
            ui.label("Mic send is how loud you go out. Listen levels are local — they never change someone else for the table.");
            ui.horizontal(|ui| {
                ui.label("Mic send");
                ui.add(egui::Slider::new(&mut self.mic_gain, 0.0..=2.0).suffix("x"));
            });
            ui.label(RichText::new("Voice is under the Voice button on the top bar. It rides Host/Join. Mic send is local — it never changes someone else's table.").color(MUTED).small());
            if theme::neon_btn(ui, "Open Voice").clicked() {
                self.shell = ShellPanel::Voice;
                self.ensure_mic();
            }
    }

    fn ui_bj(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("bj-page")
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

fn cluster(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let shown = egui::Frame::NONE
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, ORANGE))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(label)
                        .family(theme::mono())
                        .size(10.0)
                        .color(ORANGE),
                );
                add(ui);
            });
        });
    theme::brackets(
        ui,
        shown.response.rect.shrink(2.0),
        Color32::from_rgba_unmultiplied(77, 232, 255, 80),
        5.0,
    );
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
        } else if crate::audio::is_music(&p) {
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
        FontId::new(13.0, theme::ui_font()),
        if on { Color32::from_rgb(17, 17, 17) } else { ORANGE },
    );
    let size = Vec2::new(galley.size().x + 36.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover = resp.hovered();
    let fill = if on {
        ORANGE
    } else if hover {
        Color32::from_rgba_unmultiplied(255, 106, 18, 28)
    } else {
        Color32::from_rgba_unmultiplied(255, 106, 18, 8)
    };
    let pts = vec![
        rect.left_top() + Vec2::new(10.0, 0.0),
        rect.right_top(),
        rect.right_bottom() + Vec2::new(-10.0, 0.0),
        rect.left_bottom(),
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        fill,
        egui::Stroke::new(1.0, if on { ORANGE } else { Color32::from_rgba_unmultiplied(255, 106, 18, 140) }),
    ));
    if on {
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 1.0,
            egui::Stroke::new(2.0, CYAN),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(13.0, theme::ui_font()),
        if on { Color32::from_rgb(17, 17, 17) } else { ORANGE },
    );
    resp
}

fn meta(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(k).family(theme::mono()).size(10.0).color(DIM));
        ui.label(RichText::new(v).family(theme::mono()).size(10.0).color(ORANGE));
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

fn load_changelog(root: &Path) -> Vec<(String, Vec<String>)> {
    let raw = std::fs::read_to_string(root.join("data/changelog.md")).unwrap_or_default();
    let mut out = Vec::new();
    let mut title = String::new();
    let mut bullets = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            if !title.is_empty() {
                out.push((title, bullets));
            }
            title = t.trim_start_matches('#').trim().to_string();
            bullets = Vec::new();
        } else if let Some(rest) = t.strip_prefix("- ") {
            if !title.is_empty() {
                bullets.push(rest.to_string());
            }
        }
    }
    if !title.is_empty() {
        out.push((title, bullets));
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

struct RadioSt {
    id: &'static str,
    call: &'static str,
    freq: &'static str,
    name: &'static str,
    mood: &'static str,
    needle: &'static str,
}

const RADIO: [RadioSt; 16] = [
    RadioSt { id: "rebellious", call: "RIOT", freq: "104.4", name: "Riot FM", mood: "rebellious", needle: "" },
    RadioSt { id: "melancholic", call: "GLOOM", freq: "91.3", name: "Gloom Wire", mood: "melancholic", needle: "" },
    RadioSt { id: "relaxing", call: "DUSK", freq: "96.1", name: "Dusk Channel", mood: "relaxing", needle: "" },
    RadioSt { id: "brutal", call: "RAVE", freq: "88.1", name: "Warehouse", mood: "brutal", needle: "" },
    RadioSt { id: "afterlife", call: "AFTER", freq: "107.9", name: "Afterlife", mood: "melancholic", needle: "night|club|neon|ether" },
    RadioSt { id: "bodyheat", call: "HEAT", freq: "102.2", name: "Body Heat", mood: "relaxing", needle: "chill|wave|lounge|flow" },
    RadioSt { id: "trauma", call: "TRAU", freq: "89.7", name: "Trauma Tunes", mood: "brutal", needle: "metal|rock|burn|aggress" },
    RadioSt { id: "netwatch", call: "WATCH", freq: "95.5", name: "NetWatch", mood: "rebellious", needle: "cyber|digital|net|bit|shift" },
    RadioSt { id: "combatz", call: "ZONE", freq: "90.1", name: "Combat Zone", mood: "brutal", needle: "rave|dance|edm|laser|trance" },
    RadioSt { id: "pacifica", call: "PACI", freq: "97.3", name: "Pacifica", mood: "relaxing", needle: "cloud|beauty|equator|ambler" },
    RadioSt { id: "watson", call: "WAT", freq: "101.7", name: "Watson Drive", mood: "rebellious", needle: "ninja|space|fighter|ace" },
    RadioSt { id: "chromeam", call: "CHRM", freq: "66.0", name: "Chrome AM", mood: "melancholic", needle: "horizon|groove|lemon|wave" },
    RadioSt { id: "morro", call: "MORR", freq: "103.5", name: "Morro Rock", mood: "brutal", needle: "rock|high|ace|burn" },
    RadioSt { id: "samizdat", call: "SAMI", freq: "94.2", name: "Samizdat", mood: "rebellious", needle: "reform|shift|blip|bit" },
    RadioSt { id: "ritual", call: "RIT", freq: "98.8", name: "Ritual FM", mood: "melancholic", needle: "waltz|newer|brain|night" },
    RadioSt { id: "growl", call: "GROWL", freq: "106.6", name: "Growl FM", mood: "brutal", needle: "monster|energy|pack|rave" },
];

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
        assert!(RADIO.len() >= 12);
        assert!(RADIO.iter().any(|s| s.id == "rebellious" && s.call == "RIOT"));
        assert!(RADIO.iter().any(|s| s.id == "brutal" && s.call == "RAVE"));
        assert!(RADIO.iter().any(|s| s.id == "afterlife"));
        assert!(load_changelog(&PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .iter()
            .any(|(t, _)| t.contains("2026")));
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
