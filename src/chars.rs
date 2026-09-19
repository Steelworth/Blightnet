use crate::images::TexCache;
use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED, ORANGE};
use eframe::egui::{self, Color32, FontId, RichText, Vec2};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const HEARTH_SKILLS: &[(&str, &str, &str)] = &[
    ("acrobatics", "Acrobatics", "dex"),
    ("animalHandling", "Animal Handling", "wis"),
    ("arcana", "Arcana", "int"),
    ("athletics", "Athletics", "str"),
    ("deception", "Deception", "cha"),
    ("history", "History", "int"),
    ("insight", "Insight", "wis"),
    ("intimidation", "Intimidation", "cha"),
    ("investigation", "Investigation", "int"),
    ("medicine", "Medicine", "wis"),
    ("nature", "Nature", "int"),
    ("perception", "Perception", "wis"),
    ("performance", "Performance", "cha"),
    ("persuasion", "Persuasion", "cha"),
    ("religion", "Religion", "int"),
    ("sleightOfHand", "Sleight of Hand", "dex"),
    ("stealth", "Stealth", "dex"),
    ("survival", "Survival", "wis"),
];

const ABILS: &[(&str, &str)] = &[
    ("str", "STR"),
    ("dex", "DEX"),
    ("con", "CON"),
    ("int", "INT"),
    ("wis", "WIS"),
    ("cha", "CHA"),
];

const CP_STATS: &[(&str, &str)] = &[
    ("int", "INT"),
    ("ref", "REF"),
    ("dex", "DEX"),
    ("tech", "TECH"),
    ("cool", "COOL"),
    ("will", "WILL"),
    ("luck", "LUCK"),
    ("move", "MOVE"),
    ("body", "BODY"),
    ("emp", "EMP"),
];

const CP_SKILLS: &[(&str, &str, &str)] = &[
    ("athletics", "Athletics", "dex"),
    ("brawling", "Brawling", "dex"),
    ("evasion", "Evasion", "dex"),
    ("melee", "Melee", "dex"),
    ("handgun", "Handgun", "ref"),
    ("autofire", "Autofire", "ref"),
    ("stealth", "Stealth", "dex"),
    ("perception", "Perception", "int"),
    ("conversation", "Conversation", "emp"),
    ("persuasion", "Persuasion", "cool"),
    ("streetwise", "Streetwise", "cool"),
    ("concentration", "Concentration", "will"),
    ("education", "Education", "int"),
    ("firstAid", "First Aid", "tech"),
    ("basicTech", "Basic Tech", "tech"),
    ("cybertech", "Cybertech", "tech"),
    ("driveLand", "Drive Land", "ref"),
    ("conceal", "Conceal/Reveal", "int"),
];

const CLASSES: &[&str] = &[
    "Barbarian", "Bard", "Cleric", "Druid", "Fighter", "Monk", "Paladin", "Ranger",
    "Rogue", "Sorcerer", "Warlock", "Wizard", "Artificer",
];

const RACES: &[&str] = &[
    "Human", "Elf", "Dwarf", "Halfling", "Gnome", "Half-Elf", "Half-Orc", "Tiefling",
    "Dragonborn", "Aasimar", "Goblin", "Orc",
];

const ALIGNMENTS: &[&str] = &[
    "Lawful Good", "Neutral Good", "Chaotic Good", "Lawful Neutral", "True Neutral",
    "Chaotic Neutral", "Lawful Evil", "Neutral Evil", "Chaotic Evil", "Unaligned",
];

const CP_ROLES: &[&str] = &[
    "Rockerboy", "Solo", "Netrunner", "Tech", "Medtech", "Media", "Exec", "Lawman",
    "Fixer", "Nomad", "Gamemaster",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attack {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub bonus: String,
    #[serde(default)]
    pub damage: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KitItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub catalog_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub dmg: String,
    #[serde(default)]
    pub cost: String,
    #[serde(default)]
    pub cat: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub ac: String,
    #[serde(default)]
    pub props: String,
    #[serde(default)]
    pub rarity: String,
    #[serde(default)]
    pub hl: i32,
    #[serde(default)]
    pub lv: i32,
    #[serde(default)]
    pub equipped: bool,
    #[serde(default)]
    pub bonuses: HashMap<String, i32>,
}

#[derive(Clone)]
pub struct KitSpec {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub dmg: String,
    pub cost: String,
    pub detail: String,
    pub ac: String,
    pub props: String,
    pub text: String,
    pub cat: String,
    pub hl: i32,
    pub lv: i32,
    pub rarity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Character {
    #[serde(default)]
    pub id: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub last: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub world: String,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub portrait: String,
    #[serde(default)]
    pub fullbody: String,
    #[serde(default)]
    pub race: String,
    #[serde(default)]
    pub class_name: String,
    #[serde(default)]
    pub subclass: String,
    #[serde(default = "one")]
    pub level: i32,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub alignment: String,
    #[serde(default)]
    pub xp: i32,
    #[serde(default)]
    pub age: String,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub weight: String,
    #[serde(default)]
    pub eyes: String,
    #[serde(default)]
    pub skin: String,
    #[serde(default)]
    pub hair: String,
    #[serde(default)]
    pub abilities: HashMap<String, i32>,
    #[serde(default)]
    pub save_prof: HashMap<String, bool>,
    #[serde(default)]
    pub skill_prof: HashMap<String, bool>,
    #[serde(default)]
    pub skill_expert: HashMap<String, bool>,
    #[serde(default = "ten")]
    pub ac: i32,
    #[serde(default)]
    pub speed: String,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub hp_max: i32,
    #[serde(default)]
    pub hp_temp: i32,
    #[serde(default)]
    pub hit_dice: String,
    #[serde(default)]
    pub hd_used: i32,
    #[serde(default)]
    pub death_ok: i32,
    #[serde(default)]
    pub death_fail: i32,
    #[serde(default)]
    pub downed: bool,
    #[serde(default)]
    pub dead: bool,
    #[serde(default)]
    pub inspiration: bool,
    #[serde(default)]
    pub exhaustion: i32,
    #[serde(default)]
    pub attacks: Vec<Attack>,
    #[serde(default)]
    pub features: String,
    #[serde(default)]
    pub feats: String,
    #[serde(default)]
    pub proficiencies: String,
    #[serde(default)]
    pub languages: String,
    #[serde(default)]
    pub equipment: String,
    #[serde(default)]
    pub gear: Vec<String>,
    #[serde(default)]
    pub kit: Vec<KitItem>,
    #[serde(default)]
    pub npc: bool,
    #[serde(default)]
    pub cp: i32,
    #[serde(default)]
    pub sp: i32,
    #[serde(default)]
    pub ep: i32,
    #[serde(default)]
    pub gp: i32,
    #[serde(default)]
    pub pp: i32,
    #[serde(default)]
    pub eddies: i32,
    #[serde(default)]
    pub spell_ability: String,
    #[serde(default)]
    pub cantrips: String,
    #[serde(default)]
    pub slots_max: Vec<i32>,
    #[serde(default)]
    pub slots_used: Vec<i32>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub ideals: String,
    #[serde(default)]
    pub bonds: String,
    #[serde(default)]
    pub flaws: String,
    #[serde(default)]
    pub backstory: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub allies: String,
    #[serde(default)]
    pub deity: String,
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub role: String,
    #[serde(default = "four")]
    pub role_rank: i32,
    #[serde(default)]
    pub ip: i32,
    #[serde(default)]
    pub stats_red: HashMap<String, i32>,
    #[serde(default)]
    pub red_skills: HashMap<String, i32>,
    #[serde(default = "fifty")]
    pub humanity: i32,
    #[serde(default)]
    pub cyberware: String,
    #[serde(default)]
    pub chrome_eyes: bool,
    #[serde(default)]
    pub chrome_jack: bool,
    #[serde(default)]
    pub chrome_arm: bool,
    #[serde(default)]
    pub chrome_dermal: bool,
    #[serde(default)]
    pub chrome_speed: bool,
    #[serde(default)]
    pub lifepath: String,
    #[serde(default)]
    pub owner: String,
    #[serde(skip)]
    pub tab: usize,
    #[serde(skip)]
    pub leveling: bool,
    #[serde(skip)]
    pub pending_asi: HashMap<String, i32>,
    #[serde(skip)]
    pub pending_ip: HashMap<String, i32>,
}

fn default_name() -> String {
    "New character".into()
}
fn one() -> i32 {
    1
}
fn ten() -> i32 {
    10
}
fn four() -> i32 {
    4
}
fn fifty() -> i32 {
    50
}

impl Character {
    pub fn new(world: &str) -> Self {
        let blight = world == "blight";
        let mut abilities = HashMap::new();
        for (id, _) in ABILS {
            abilities.insert((*id).into(), 10);
        }
        let mut stats_red = HashMap::new();
        for (id, _) in CP_STATS {
            stats_red.insert((*id).into(), 6);
        }
        Self {
            id: format!("c_{}", rand::random::<u32>()),
            name: "New character".into(),
            first: String::new(),
            last: String::new(),
            gender: "female".into(),
            world: world.into(),
            player: String::new(),
            portrait: String::new(),
            fullbody: String::new(),
            race: if blight { "Human".into() } else { String::new() },
            class_name: String::new(),
            subclass: String::new(),
            level: 1,
            background: String::new(),
            alignment: String::new(),
            xp: 0,
            age: String::new(),
            height: String::new(),
            weight: String::new(),
            eyes: String::new(),
            skin: String::new(),
            hair: String::new(),
            abilities,
            save_prof: HashMap::new(),
            skill_prof: HashMap::new(),
            skill_expert: HashMap::new(),
            ac: if blight { 11 } else { 10 },
            speed: if blight { "6 m".into() } else { "30 ft.".into() },
            hp: if blight { 40 } else { 8 },
            hp_max: if blight { 40 } else { 8 },
            hp_temp: 0,
            hit_dice: "1d8".into(),
            hd_used: 0,
            death_ok: 0,
            death_fail: 0,
            downed: false,
            dead: false,
            inspiration: false,
            exhaustion: 0,
            attacks: vec![
                Attack {
                    name: "Punch".into(),
                    bonus: String::new(),
                    damage: if blight { "1d6".into() } else { "1d4".into() },
                },
                Attack {
                    name: "Kick".into(),
                    bonus: String::new(),
                    damage: if blight { "1d6".into() } else { "1d4".into() },
                },
                Attack {
                    name: "Headbutt".into(),
                    bonus: String::new(),
                    damage: "1d4".into(),
                },
                Attack {
                    name: "Bite".into(),
                    bonus: String::new(),
                    damage: "1d4".into(),
                },
            ],
            features: String::new(),
            feats: String::new(),
            proficiencies: String::new(),
            languages: String::new(),
            equipment: String::new(),
            kit: vec![],
            npc: false,
            gear: vec![],
            cp: 0,
            sp: 0,
            ep: 0,
            gp: if blight { 0 } else { 50 },
            pp: 0,
            eddies: if blight { 500 } else { 0 },
            spell_ability: "int".into(),
            cantrips: String::new(),
            slots_max: vec![0; 9],
            slots_used: vec![0; 9],
            spells: vec![String::new(); 9],
            personality: String::new(),
            ideals: String::new(),
            bonds: String::new(),
            flaws: String::new(),
            backstory: String::new(),
            notes: String::new(),
            allies: String::new(),
            deity: String::new(),
            handle: String::new(),
            role: "Solo".into(),
            role_rank: 4,
            ip: 0,
            stats_red,
            red_skills: HashMap::new(),
            humanity: 50,
            cyberware: String::new(),
            chrome_eyes: false,
            chrome_jack: false,
            chrome_arm: false,
            chrome_dermal: false,
            chrome_speed: false,
            lifepath: String::new(),
            owner: String::new(),
            tab: 0,
            leveling: false,
            pending_asi: HashMap::new(),
            pending_ip: HashMap::new(),
        }
    }

    pub fn is_blight(&self) -> bool {
        self.world == "blight"
    }

    fn abil(&self, id: &str) -> i32 {
        *self.abilities.get(id).unwrap_or(&10)
    }

    fn red(&self, id: &str) -> i32 {
        *self.stats_red.get(id).unwrap_or(&5)
    }

    pub fn bonus_for(&self, id: &str) -> i32 {
        self.kit
            .iter()
            .filter(|k| k.equipped)
            .map(|k| *k.bonuses.get(id).unwrap_or(&0) + *k.bonuses.get("all").unwrap_or(&0))
            .sum()
    }

    pub fn abil_now(&self, id: &str) -> i32 {
        (self.abil(id) + self.bonus_for(id)).clamp(1, 30)
    }

    pub fn red_now(&self, id: &str) -> i32 {
        (self.red(id) + self.bonus_for(id)).clamp(1, 15)
    }

    pub fn ac_now(&self) -> i32 {
        let blight = self.is_blight();
        let mut best: Option<i32> = None;
        let mut shield = 0;
        for k in self.kit.iter().filter(|k| k.equipped) {
            let kind = k.kind.to_lowercase();
            let cat = k.cat_or_props();
            let raw = k.ac.trim();
            if raw.is_empty() && kind != "armor" && !cat.contains("shield") {
                continue;
            }
            let n = raw
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<i32>()
                .unwrap_or(0);
            if cat.contains("shield") || k.name.to_lowercase().contains("shield") {
                shield += if n > 0 { n } else { 2 };
                continue;
            }
            if n > 0 {
                let mut ac = n;
                if !blight {
                    let dex = Self::modifier(self.abil_now("dex"));
                    if raw.to_lowercase().contains("max 2") {
                        ac += dex.min(2);
                    } else if raw.to_lowercase().contains("dex") {
                        ac += dex;
                    }
                }
                best = Some(best.map(|b| b.max(ac)).unwrap_or(ac));
            }
        }
        best.unwrap_or(self.ac) + shield
    }

    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }

    pub fn prof(level: i32) -> i32 {
        (level.clamp(1, 20) - 1) / 4 + 2
    }
}

impl KitItem {
    fn cat_or_props(&self) -> String {
        format!("{} {} {}", self.cat, self.props, self.detail).to_lowercase()
    }
}

pub fn load(root: &Path) -> Vec<Character> {
    std::fs::read_to_string(root.join("data/characters.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(root: &Path, rows: &[Character]) {
    if let Ok(s) = serde_json::to_string_pretty(rows) {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(root.join("data/characters.json"), s);
    }
}

pub fn import_picture(root: &Path, ch: &mut Character, closeup: bool) -> Result<(), String> {
    let Some(src) = rfd::FileDialog::new()
        .add_filter("Image", &["jpg", "jpeg", "png", "webp"])
        .set_title(if closeup {
            "Close-up portrait"
        } else {
            "Full-body picture"
        })
        .pick_file()
    else {
        return Ok(());
    };
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let ext = if matches!(ext.as_str(), "png" | "webp" | "jpeg" | "jpg") {
        if ext == "jpeg" { "jpg" } else { &ext }
    } else {
        "jpg"
    };
    let dir = root.join("data/chars");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let kind = if closeup { "portrait" } else { "fullbody" };
    let dest = dir.join(format!("{}-{kind}.{ext}", ch.id));
    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    let rel = format!("data/chars/{}-{kind}.{ext}", ch.id);
    if closeup {
        ch.portrait = rel;
    } else {
        ch.fullbody = rel;
    }
    Ok(())
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(label)
                .family(theme::mono())
                .size(10.0)
                .color(CYAN),
        );
        let w = ui.available_width().clamp(80.0, 280.0);
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(w)
                .text_color(ORANGE),
        );
    });
}

fn sheet_card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(8, 8, 5))
        .stroke(egui::Stroke::new(1.0, ORANGE))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .family(theme::mono())
                    .size(10.0)
                    .color(ORANGE),
            );
            ui.add_space(4.0);
            add(ui);
        });
}

fn hp_bar(ui: &mut egui::Ui, hp: i32, max: i32) {
    let max = max.max(1);
    let t = (hp as f32 / max as f32).clamp(0.0, 1.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().min(220.0), 10.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, Color32::from_rgb(12, 12, 8));
    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, ORANGE), egui::StrokeKind::Inside);
    if t > 0.0 {
        let fill = egui::Rect::from_min_size(r.min, Vec2::new((r.width() * t).max(2.0), r.height()));
        let col = if t > 0.5 {
            ORANGE
        } else if t > 0.25 {
            CYAN
        } else {
            KILL
        };
        ui.painter().rect_filled(fill.shrink(1.0), 0.0, col);
    }
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        format!("{hp}/{max}"),
        FontId::new(10.0, theme::mono()),
        CREAM,
    );
}

fn wrap(ui: &mut egui::Ui, text: &str, color: Color32, size: f32) {
    ui.add(
        egui::Label::new(RichText::new(text).color(color).size(size).family(theme::ui_font()))
            .wrap(),
    );
}

fn signed(n: i32) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

fn pic(ui: &mut egui::Ui, tex: &mut TexCache, root: &Path, rel: &str, max: Vec2) -> Option<PathBuf> {
    if rel.is_empty() {
        ui.allocate_ui(max, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No picture").color(DIM).small());
            });
        });
        return None;
    }
    let path = crate::images::resolve_rel(root, rel);
    if path.is_file() {
        if crate::images::show_fit(ui, tex, &path, max)
            .on_hover_text("Click to zoom")
            .clicked()
        {
            return Some(path);
        }
        None
    } else {
        ui.label(RichText::new("Missing file").color(DIM).small());
        None
    }
}

pub fn ui_sheet(
    ui: &mut egui::Ui,
    root: &Path,
    tex: &mut TexCache,
    chars: &mut Vec<Character>,
    char_i: &mut usize,
    blight: bool,
    dice: &mut String,
    log: &mut Vec<String>,
    is_gm: bool,
    target: &mut Option<String>,
    luck: &mut crate::dice::Luck,
    roll: &mut Option<crate::dice::Roll>,
    names: &crate::names::Names,
    zoom: &mut Option<PathBuf>,
) {
    if is_gm {
        let drop = ui.interact(ui.clip_rect(), egui::Id::new("sheet-drop"), egui::Sense::hover());
        if let Some(spec) = drop.dnd_release_payload::<KitSpec>() {
            if let Some(c) = chars.get_mut(*char_i) {
                receive_kit(c, &spec);
                save(root, chars);
            }
        }
        if let Some(spec) = drop.dnd_release_payload::<crate::maps::TokenSpec>() {
            if spec.cat == "Gods" {
                if let Some(c) = chars.get_mut(*char_i) {
                    c.deity = spec.name.clone();
                    save(root, chars);
                }
            }
        }
    }
    theme::section_head(ui, "05", "CHARACTER SHEET");
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if theme::neon_btn(ui, "+ New").clicked() {
            chars.push(Character::new(if blight { "blight" } else { "hearthsong" }));
            *char_i = chars.len() - 1;
            save(root, chars);
        }
        if theme::neon_btn_color(ui, "Norm", ORANGE, *luck == crate::dice::Luck::Norm).clicked() {
            *luck = crate::dice::Luck::Norm;
        }
        if theme::neon_btn_color(ui, "Adv", ORANGE, *luck == crate::dice::Luck::Adv).clicked() {
            *luck = crate::dice::Luck::Adv;
        }
        if theme::neon_btn_color(ui, "Dis", ORANGE, *luck == crate::dice::Luck::Dis).clicked() {
            *luck = crate::dice::Luck::Dis;
        }
        if theme::neon_btn(ui, "d%").clicked() {
            let name = chars
                .get(*char_i)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Someone".into());
            let r = crate::dice::fire(&name, "", false, *luck, target.as_deref().unwrap_or(""));
            *dice = format!("{} — {}", r.pct, r.grade);
            log.push(format!("{} rolls {} ({})", name, r.pct, r.grade));
            *roll = Some(r);
        }
        if !chars.is_empty() && theme::neon_btn_color(ui, "Delete", KILL, false).clicked() {
            let i = (*char_i).min(chars.len() - 1);
            chars.remove(i);
            if *char_i >= chars.len() && !chars.is_empty() {
                *char_i = chars.len() - 1;
            }
            save(root, chars);
        }
        if theme::neon_btn(ui, "Save").clicked() {
            save(root, chars);
        }
        let mut did_rest = false;
        let i = (*char_i).min(chars.len().saturating_sub(1));
        if let Some(c) = chars.get_mut(i) {
            if theme::neon_btn(ui, "Short rest").clicked() {
                short_rest(c, log);
                did_rest = true;
            }
        }
        if did_rest {
            save(root, chars);
        }
    });
    ui.horizontal_wrapped(|ui| {
        let mut did_rest = false;
        let i = (*char_i).min(chars.len().saturating_sub(1));
        if let Some(c) = chars.get_mut(i) {
            if theme::neon_btn(ui, "Long rest").clicked() {
                long_rest(c, log);
                did_rest = true;
            }
            if (c.downed || c.hp <= 0) && !c.dead && theme::neon_btn(ui, "Death save").clicked() {
                death_save(c, *luck, dice, log, roll);
                did_rest = true;
            }
        }
        if did_rest {
            save(root, chars);
        }
    });
    if !dice.is_empty() {
        ui.label(
            RichText::new(dice.as_str())
                .family(theme::mono())
                .size(13.0)
                .color(CYAN),
        );
    }
    if chars.is_empty() {
        wrap(ui, "No characters. Press + New to open a full sheet.", MUTED, 13.0);
        return;
    }
    sheet_card(ui, "ROSTER", |ui| {
        egui::ScrollArea::vertical()
            .id_salt("char-roster")
            .max_height(88.0)
            .show(ui, |ui| {
                for (i, c) in chars.iter().enumerate() {
                    let mark = if c.dead {
                        "†"
                    } else if c.downed {
                        "↓"
                    } else {
                        "●"
                    };
                    let aimed = target.as_deref() == Some(c.id.as_str());
                    let on = *char_i == i;
                    ui.horizontal_wrapped(|ui| {
                        crate::maps::drag_source(
                            ui,
                            ("char-drag", c.id.clone()),
                            crate::maps::TokenSpec {
                                name: c.name.clone(),
                                image: c.portrait.clone(),
                                sheet: c.id.clone(),
                                cat: String::new(),
                                src: String::new(),
                            },
                            |ui| {
                                if theme::neon_btn_color(
                                    ui,
                                    &format!(
                                        "{}{mark} {}  {}/{}",
                                        if aimed { "▸ " } else { "" },
                                        c.name,
                                        c.hp,
                                        c.hp_max
                                    ),
                                    ORANGE,
                                    on,
                                )
                                .clicked()
                                {
                                    *char_i = i;
                                }
                            },
                        );
                        if theme::neon_btn_color(ui, "Aim", CYAN, aimed).clicked() {
                            *target = Some(c.id.clone());
                        }
                    });
                }
            });
    });
    ui.add_space(6.0);
    let i = (*char_i).min(chars.len() - 1);
    *char_i = i;
    let mut pic_close = false;
    let mut pic_full = false;
    if let Some(c) = chars.get(i) {
        sheet_card(ui, "PORTRAIT", |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(p) = pic(ui, tex, root, &c.portrait, Vec2::new(72.0, 72.0)) {
                    *zoom = Some(p);
                }
                ui.vertical(|ui| {
                    wrap(ui, "Close-up", DIM, 10.0);
                    if theme::neon_btn(ui, "Upload close-up").clicked() {
                        pic_close = true;
                    }
                    ui.add_space(4.0);
                    hp_bar(ui, c.hp, c.hp_max);
                    ui.label(
                        RichText::new(format!(
                            "LV {} · {}",
                            c.level,
                            if blight { "BLIGHT" } else { "HEARTH" }
                        ))
                        .family(theme::mono())
                        .size(11.0)
                        .color(CYAN),
                    );
                });
                if let Some(p) = pic(ui, tex, root, &c.fullbody, Vec2::new(52.0, 84.0)) {
                    *zoom = Some(p);
                }
                ui.vertical(|ui| {
                    wrap(ui, "Full body", DIM, 10.0);
                    if theme::neon_btn(ui, "Upload full body").clicked() {
                        pic_full = true;
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
    if pic_close {
        if let Some(c) = chars.get_mut(i) {
            if let Err(e) = import_picture(root, c, true) {
                *dice = e;
            }
        }
        save(root, chars);
    }
    if pic_full {
        if let Some(c) = chars.get_mut(i) {
            if let Err(e) = import_picture(root, c, false) {
                *dice = e;
            }
        }
        save(root, chars);
    }
    let i = (*char_i).min(chars.len().saturating_sub(1));
    let Some(c) = chars.get_mut(i) else {
        return;
    };
    let blight_sheet = c.is_blight();
    ui.horizontal_wrapped(|ui| {
        let aimed = target.as_deref() == Some(c.id.as_str());
        if theme::neon_btn_color(ui, "Target this sheet", CYAN, aimed).clicked() {
            *target = Some(c.id.clone());
        }
        if (is_gm || c.owner.is_empty()) && theme::neon_btn(ui, "Level up").clicked() {
            c.leveling = true;
            c.pending_asi.clear();
            c.pending_ip.clear();
        }
    });
    if c.leveling {
        ui_level_up(ui, c, blight_sheet);
    }
    let tabs: &[&str] = if blight_sheet {
        &["Bio", "Stats", "Combat", "Skills", "Chrome", "Gear", "Life"]
    } else {
        &["Bio", "Stats", "Combat", "Magic", "Features", "Gear", "Story"]
    };
    ui.horizontal_wrapped(|ui| {
        for (t, lab) in tabs.iter().enumerate() {
            if theme::neon_btn_color(ui, lab, ORANGE, c.tab == t).clicked() {
                c.tab = t;
            }
        }
    });
    ui.add_space(4.0);
    match (blight_sheet, c.tab) {
        (_, 0) => ui_bio(ui, c, blight_sheet, names),
        (false, 1) => ui_stats_5e(ui, c),
        (true, 1) => ui_stats_red(ui, c),
        (_, 2) => ui_combat(ui, c, blight_sheet, dice, log, luck, roll, target),
        (false, 3) => ui_magic(ui, c),
        (true, 3) => ui_red_skills(ui, c),
        (false, 4) => ui_features(ui, c),
        (true, 4) => ui_chrome(ui, c),
        (_, 5) => ui_gear(ui, c, blight_sheet),
        (_, _) => ui_story(ui, c, blight_sheet),
    }
}

fn compose_name(c: &mut Character) {
    let n = format!("{} {}", c.first.trim(), c.last.trim())
        .trim()
        .to_string();
    if !n.is_empty() {
        c.name = n;
    }
}

fn ui_bio(ui: &mut egui::Ui, c: &mut Character, blight: bool, names: &crate::names::Names) {
    wrap(
        ui,
        "Shuffle first and last names separately. About 500 given names for each, plus 500 family names.",
        MUTED,
        11.0,
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if theme::neon_btn_color(ui, "Female", ORANGE, c.gender != "male").clicked() {
            c.gender = "female".into();
        }
        if theme::neon_btn_color(ui, "Male", ORANGE, c.gender == "male").clicked() {
            c.gender = "male".into();
        }
        let female = c.gender != "male";
        if theme::neon_btn(ui, "Shuffle first").clicked() {
            c.first = names.first(female);
            compose_name(c);
        }
        if theme::neon_btn(ui, "Shuffle last").clicked() {
            c.last = names.last();
            compose_name(c);
        }
        if theme::neon_btn(ui, "Shuffle both").clicked() {
            c.first = names.first(female);
            c.last = names.last();
            compose_name(c);
        }
    });
    let before = (c.first.clone(), c.last.clone());
    field(ui, "First", &mut c.first);
    field(ui, "Last", &mut c.last);
    if (c.first.clone(), c.last.clone()) != before {
        compose_name(c);
    }
    field(ui, "Name", &mut c.name);
    field(ui, "Player", &mut c.player);
    if blight {
        field(ui, "Handle", &mut c.handle);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Role").color(DIM).small());
            egui::ComboBox::from_id_salt("role")
                .selected_text(&c.role)
                .show_ui(ui, |ui| {
                    for r in CP_ROLES {
                        ui.selectable_value(&mut c.role, (*r).into(), *r);
                    }
                });
            ui.label("Rank");
            ui.add(egui::DragValue::new(&mut c.role_rank).range(1..=10));
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ancestry").color(DIM).small());
            egui::ComboBox::from_id_salt("race")
                .selected_text(if c.race.is_empty() { "—" } else { &c.race })
                .show_ui(ui, |ui| {
                    for r in RACES {
                        ui.selectable_value(&mut c.race, (*r).into(), *r);
                    }
                });
            ui.label(RichText::new("Class").color(DIM).small());
            egui::ComboBox::from_id_salt("class")
                .selected_text(if c.class_name.is_empty() {
                    "—"
                } else {
                    &c.class_name
                })
                .show_ui(ui, |ui| {
                    for r in CLASSES {
                        ui.selectable_value(&mut c.class_name, (*r).into(), *r);
                    }
                });
        });
        field(ui, "Subclass", &mut c.subclass);
        field(ui, "Background", &mut c.background);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Alignment").color(DIM).small());
            egui::ComboBox::from_id_salt("align")
                .selected_text(if c.alignment.is_empty() {
                    "—"
                } else {
                    &c.alignment
                })
                .show_ui(ui, |ui| {
                    for a in ALIGNMENTS {
                        ui.selectable_value(&mut c.alignment, (*a).into(), *a);
                    }
                });
        });
        field(ui, "Deity", &mut c.deity);
    }
    ui.horizontal(|ui| {
        ui.label(if blight { "Rank" } else { "Level" });
        ui.add(egui::DragValue::new(&mut c.level).range(1..=20));
        if !blight {
            ui.label("XP");
            ui.add(egui::DragValue::new(&mut c.xp).range(0..=355_000));
        } else {
            ui.label("IP");
            ui.add(egui::DragValue::new(&mut c.ip).range(0..=9999));
        }
    });
    ui.columns(3, |cols| {
        field(&mut cols[0], "Age", &mut c.age);
        field(&mut cols[1], "Height", &mut c.height);
        field(&mut cols[2], "Weight", &mut c.weight);
    });
    ui.columns(3, |cols| {
        field(&mut cols[0], "Eyes", &mut c.eyes);
        field(&mut cols[1], "Skin", &mut c.skin);
        field(&mut cols[2], "Hair", &mut c.hair);
    });
}

fn ui_stats_5e(ui: &mut egui::Ui, c: &mut Character) {
    wrap(ui, "Ability scores. Proficiency is automatic from level. Tick P for proficient, E for expertise.", MUTED, 11.0);
    ui.horizontal_wrapped(|ui| {
        for (id, lab) in ABILS {
            let mut v = c.abil(id);
            let boost = c.bonus_for(id);
            ui.vertical(|ui| {
                ui.label(RichText::new(*lab).family(theme::mono()).color(ORANGE));
                ui.add(egui::DragValue::new(&mut v).range(1..=30));
                let shown = (v + boost).clamp(1, 30);
                ui.label(
                    RichText::new(if boost != 0 {
                        format!("{} ({})", signed(Character::modifier(shown)), signed(boost))
                    } else {
                        signed(Character::modifier(shown))
                    })
                    .color(CYAN)
                    .family(theme::mono()),
                );
            });
            c.abilities.insert((*id).into(), v);
        }
        ui.vertical(|ui| {
            ui.label(RichText::new("PROF").family(theme::mono()).color(ORANGE));
            ui.label(RichText::new(signed(Character::prof(c.level))).color(CYAN).size(18.0));
            ui.checkbox(&mut c.inspiration, "Inspiration");
        });
    });
    ui.add_space(6.0);
    ui.label(RichText::new("SAVING THROWS").family(theme::mono()).size(10.0).color(ORANGE));
    ui.columns(3, |cols| {
        for (i, (id, lab)) in ABILS.iter().enumerate() {
            let ui = &mut cols[i % 3];
            let mut on = *c.save_prof.get(*id).unwrap_or(&false);
            let bonus = Character::modifier(c.abil_now(id)) + if on { Character::prof(c.level) } else { 0 };
            ui.horizontal(|ui| {
                ui.checkbox(&mut on, "");
                ui.label(format!("{lab} {}", signed(bonus)));
            });
            c.save_prof.insert((*id).into(), on);
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new("SKILLS").family(theme::mono()).size(10.0).color(ORANGE));
    ui.columns(2, |cols| {
        for (i, (id, name, abil)) in HEARTH_SKILLS.iter().enumerate() {
            let ui = &mut cols[i % 2];
            let mut p = *c.skill_prof.get(*id).unwrap_or(&false);
            let mut e = *c.skill_expert.get(*id).unwrap_or(&false);
            let bonus = {
                let m = Character::modifier(c.abil_now(abil));
                let pr = Character::prof(c.level);
                m + if e { pr * 2 } else if p { pr } else { 0 }
            };
            ui.horizontal(|ui| {
                ui.checkbox(&mut p, "P");
                ui.checkbox(&mut e, "E");
                ui.label(format!("{name} ({abil}) {}", signed(bonus)));
            });
            c.skill_prof.insert((*id).into(), p);
            c.skill_expert.insert((*id).into(), e);
        }
    });
}

fn ui_stats_red(ui: &mut egui::Ui, c: &mut Character) {
    wrap(ui, "Cyberpunk RED stats (1–10 typical). Humanity drops as chrome goes in.", MUTED, 11.0);
    ui.horizontal_wrapped(|ui| {
        for (id, lab) in CP_STATS {
            let mut v = c.red(id);
            let boost = c.bonus_for(id);
            ui.vertical(|ui| {
                ui.label(RichText::new(*lab).family(theme::mono()).color(ORANGE).size(11.0));
                ui.add(egui::DragValue::new(&mut v).range(1..=15));
                if boost != 0 {
                    ui.label(
                        RichText::new(format!("→ {}", (v + boost).clamp(1, 15)))
                            .color(CYAN)
                            .family(theme::mono())
                            .size(11.0),
                    );
                }
            });
            c.stats_red.insert((*id).into(), v);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Humanity");
        ui.add(egui::DragValue::new(&mut c.humanity).range(0..=100));
        ui.label("IP");
        ui.add(egui::DragValue::new(&mut c.ip).range(0..=9999));
    });
}

fn ui_combat(
    ui: &mut egui::Ui,
    c: &mut Character,
    blight: bool,
    dice: &mut String,
    log: &mut Vec<String>,
    luck: &mut crate::dice::Luck,
    roll: &mut Option<crate::dice::Roll>,
    target: &mut Option<String>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("HP");
        ui.add(egui::DragValue::new(&mut c.hp).range(0..=999));
        ui.label("/");
        ui.add(egui::DragValue::new(&mut c.hp_max).range(1..=999));
        ui.label("temp");
        ui.add(egui::DragValue::new(&mut c.hp_temp).range(0..=999));
        ui.label("AC");
        ui.add(egui::DragValue::new(&mut c.ac).range(0..=40));
        let worn = c.ac_now();
        if worn != c.ac {
            ui.label(RichText::new(format!("worn {worn}")).color(CYAN).family(theme::mono()));
        }
        ui.label("Speed");
        ui.add(egui::TextEdit::singleline(&mut c.speed).desired_width(70.0));
    });
    if !blight {
        ui.horizontal(|ui| {
            ui.label("Hit dice");
            ui.add(egui::TextEdit::singleline(&mut c.hit_dice).desired_width(60.0));
            ui.label("used");
            ui.add(egui::DragValue::new(&mut c.hd_used).range(0..=20));
            ui.label("Exhaustion");
            ui.add(egui::DragValue::new(&mut c.exhaustion).range(0..=6));
        });
    }
    ui.horizontal_wrapped(|ui| {
        if theme::neon_btn(ui, "Short rest").clicked() {
            short_rest(c, log);
        }
        if theme::neon_btn(ui, "Long rest").clicked() {
            long_rest(c, log);
        }
        ui.checkbox(&mut c.downed, "Downed");
        ui.checkbox(&mut c.dead, "Dead");
    });
    if c.hp <= 0 && !c.dead {
        c.downed = true;
        wrap(ui, "At 0 HP. Roll death saves. Three successes: stand at 1 HP. Three failures: dead. A heal stands them.", CYAN, 12.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Death saves  ok {}  fail {}", c.death_ok, c.death_fail));
            if theme::neon_btn(ui, "Death save").clicked() {
                death_save(c, *luck, dice, log, roll);
            }
        });
    }
    wrap(
        ui,
        "Unarmed: Punch, Kick, Headbutt, Bite. Damage and to-hit scale with the used stat plus level (or role rank). Equip weapons on Gear.",
        MUTED,
        11.0,
    );
    let mut remove = None;
    let mut fire_i = None;
    let n = c.attacks.len();
    for i in 0..n {
        let (hit, scaled_dmg) = {
            let atk = &c.attacks[i];
            let (h, d, _) = scaled_attack(c, atk);
            (h, d)
        };
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut c.attacks[i].name)
                    .desired_width(110.0)
                    .hint_text("Attack"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut c.attacks[i].bonus)
                    .desired_width(50.0)
                    .hint_text("to-hit"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut c.attacks[i].damage)
                    .desired_width(70.0)
                    .hint_text("damage"),
            );
            ui.label(
                RichText::new(format!("hit {hit}  {scaled_dmg}"))
                    .color(CYAN)
                    .family(theme::mono())
                    .size(11.0),
            );
            if theme::neon_btn(ui, "Roll").clicked() {
                fire_i = Some((i, scaled_dmg.clone()));
            }
            if theme::neon_btn_color(ui, "×", KILL, false).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some((i, dmg)) = fire_i {
        let name = c.attacks[i].name.clone();
        let low = name.to_lowercase();
        let heal = low.contains("heal") || low.contains("cure") || low.contains("aid");
        let r = crate::dice::fire(
            &name,
            &dmg,
            heal,
            *luck,
            target.as_deref().unwrap_or(""),
        );
        *dice = format!(
            "{}: {}% {}{}",
            name,
            r.pct,
            r.grade,
            r.damage.map(|d| format!(" · {d}")).unwrap_or_default()
        );
        log.push(format!(
            "{} uses {} → {} ({}){}",
            c.name,
            name,
            if r.target.is_empty() {
                "—"
            } else {
                &r.target
            },
            r.grade,
            r.damage.map(|d| format!(" {d}")).unwrap_or_default()
        ));
        *roll = Some(r);
    }
    if let Some(i) = remove {
        c.attacks.remove(i);
    }
    if theme::neon_btn(ui, "+ Attack").clicked() {
        c.attacks.push(Attack {
            name: String::new(),
            bonus: String::new(),
            damage: String::new(),
        });
    }
}

fn ui_magic(ui: &mut egui::Ui, c: &mut Character) {
    ui.horizontal(|ui| {
        ui.label("Spellcasting ability");
        for id in ["int", "wis", "cha"] {
            if theme::neon_btn_color(ui, id, ORANGE, c.spell_ability == id).clicked() {
                c.spell_ability = id.into();
            }
        }
        let m = Character::modifier(c.abil_now(&c.spell_ability));
        ui.label(RichText::new(format!("mod {}", signed(m))).color(CYAN));
    });
    ui.label(RichText::new("Cantrips").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.cantrips).desired_rows(3).desired_width(ui.available_width()));
    if c.slots_max.len() < 9 {
        c.slots_max.resize(9, 0);
    }
    if c.slots_used.len() < 9 {
        c.slots_used.resize(9, 0);
    }
    if c.spells.len() < 9 {
        c.spells.resize(9, String::new());
    }
    for lv in 0..9 {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("L{}", lv + 1)).family(theme::mono()).color(ORANGE));
            ui.label("slots");
            ui.add(egui::DragValue::new(&mut c.slots_max[lv]).range(0..=9));
            ui.label("used");
            ui.add(egui::DragValue::new(&mut c.slots_used[lv]).range(0..=9));
        });
        ui.add(
            egui::TextEdit::multiline(&mut c.spells[lv])
                .desired_rows(2)
                .desired_width(ui.available_width())
                .hint_text("Prepared / known spells"),
        );
    }
}

fn ui_red_skills(ui: &mut egui::Ui, c: &mut Character) {
    wrap(ui, "Skill ranks added to the linked STAT when you roll.", MUTED, 11.0);
    ui.columns(2, |cols| {
        for (i, (id, name, stat)) in CP_SKILLS.iter().enumerate() {
            let ui = &mut cols[i % 2];
            let mut v = *c.red_skills.get(*id).unwrap_or(&0);
            ui.horizontal(|ui| {
                ui.label(format!("{name} ({stat})"));
                ui.add(egui::DragValue::new(&mut v).range(0..=10));
                let total = c.red_now(stat) + v;
                ui.label(RichText::new(format!("= {total}")).color(CYAN).family(theme::mono()));
            });
            c.red_skills.insert((*id).into(), v);
        }
    });
}

fn ui_features(ui: &mut egui::Ui, c: &mut Character) {
    ui.label(RichText::new("Class features & racial traits").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.features).desired_rows(6).desired_width(ui.available_width()));
    ui.label(RichText::new("Feats").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.feats).desired_rows(3).desired_width(ui.available_width()));
    ui.label(RichText::new("Proficiencies").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.proficiencies).desired_rows(3).desired_width(ui.available_width()));
    field(ui, "Languages", &mut c.languages);
}

fn ui_chrome(ui: &mut egui::Ui, c: &mut Character) {
    wrap(ui, "Tick installed chrome. Write the rest in the cyberware file.", MUTED, 11.0);
    ui.checkbox(&mut c.chrome_eyes, "Cybereyes");
    ui.checkbox(&mut c.chrome_jack, "Neural jack / interface");
    ui.checkbox(&mut c.chrome_arm, "Cyberarm");
    ui.checkbox(&mut c.chrome_dermal, "Dermal / skin weave");
    ui.checkbox(&mut c.chrome_speed, "Speedware (Sandevistan etc.)");
    ui.label(RichText::new("Cyberware list").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.cyberware).desired_rows(8).desired_width(ui.available_width()));
    ui.horizontal(|ui| {
        ui.label("Humanity");
        ui.add(egui::DragValue::new(&mut c.humanity).range(0..=100));
    });
}

fn ui_gear(ui: &mut egui::Ui, c: &mut Character, blight: bool) {
    if blight {
        ui.horizontal(|ui| {
            ui.label("Eddies");
            ui.add(egui::DragValue::new(&mut c.eddies).range(0..=9_999_999));
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.label("pp");
            ui.add(egui::DragValue::new(&mut c.pp).range(0..=99_999));
            ui.label("gp");
            ui.add(egui::DragValue::new(&mut c.gp).range(0..=999_999));
            ui.label("ep");
            ui.add(egui::DragValue::new(&mut c.ep).range(0..=99_999));
            ui.label("sp");
            ui.add(egui::DragValue::new(&mut c.sp).range(0..=99_999));
            ui.label("cp");
            ui.add(egui::DragValue::new(&mut c.cp).range(0..=99_999));
        });
    }
    ui.label(RichText::new("Equipment notes").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.equipment).desired_rows(4).desired_width(ui.available_width()));
    wrap(
        ui,
        "Equip an item to wear it. Stat bonuses apply only while it is equipped. Unequip to drop them.",
        MUTED,
        11.0,
    );
    let mut toggle = None;
    let mut drop = None;
    for (i, k) in c.kit.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            let mark = if k.equipped { "●" } else { "○" };
            ui.label(
                RichText::new(format!("{mark} {}", k.name))
                    .color(if k.equipped { CYAN } else { ORANGE })
                    .size(14.0),
            );
            if !k.kind.is_empty() {
                ui.label(RichText::new(&k.kind).color(MUTED).size(13.0));
            }
            if !k.dmg.is_empty() {
                ui.label(RichText::new(&k.dmg).color(CYAN).size(13.0));
            }
            if theme::neon_btn(ui, if k.equipped { "Unequip" } else { "Equip" }).clicked() {
                toggle = Some((i, !k.equipped));
            }
            if theme::neon_btn_color(ui, "Drop", KILL, false).clicked() {
                drop = Some(i);
            }
        });
        let mut lines = Vec::new();
        for (lab, val) in [
            ("type", k.kind.as_str()),
            ("cat", k.cat.as_str()),
            ("cost", k.cost.as_str()),
            ("dmg", k.dmg.as_str()),
            ("ac", k.ac.as_str()),
            ("props", k.props.as_str()),
            ("rarity", k.rarity.as_str()),
        ] {
            if !val.is_empty() && lab != "type" {
                lines.push(format!("{lab}: {val}"));
            }
        }
        if !k.detail.is_empty() {
            lines.push(k.detail.clone());
        }
        if !lines.is_empty() {
            wrap(ui, &lines.join(" · "), CREAM, 14.0);
        }
        if !k.bonuses.is_empty() {
            let b = k
                .bonuses
                .iter()
                .map(|(s, n)| format!("{s} {:+}", n))
                .collect::<Vec<_>>()
                .join(" · ");
            wrap(ui, &format!("boost {b}"), CYAN, 13.0);
        }
    }
    if let Some((i, on)) = toggle {
        set_equipped(c, i, on);
    }
    if let Some(i) = drop {
        if c.kit.get(i).map(|k| k.equipped).unwrap_or(false) {
            set_equipped(c, i, false);
        }
        let name = c.kit.get(i).map(|k| k.name.clone()).unwrap_or_default();
        c.kit.remove(i);
        c.gear.retain(|g| g != &name);
    }
    if c.kit.is_empty() && !c.gear.is_empty() {
        wrap(ui, &format!("Stash: {}", c.gear.join(" · ")), CYAN, 12.0);
    }
}

fn ui_story(ui: &mut egui::Ui, c: &mut Character, blight: bool) {
    if blight {
        ui.label(RichText::new("Lifepath").color(DIM).small());
        ui.add(egui::TextEdit::multiline(&mut c.lifepath).desired_rows(5).desired_width(ui.available_width()));
    } else {
        ui.label(RichText::new("Personality").color(DIM).small());
        ui.add(egui::TextEdit::multiline(&mut c.personality).desired_rows(2).desired_width(ui.available_width()));
        ui.label(RichText::new("Ideals").color(DIM).small());
        ui.add(egui::TextEdit::multiline(&mut c.ideals).desired_rows(2).desired_width(ui.available_width()));
        ui.label(RichText::new("Bonds").color(DIM).small());
        ui.add(egui::TextEdit::multiline(&mut c.bonds).desired_rows(2).desired_width(ui.available_width()));
        ui.label(RichText::new("Flaws").color(DIM).small());
        ui.add(egui::TextEdit::multiline(&mut c.flaws).desired_rows(2).desired_width(ui.available_width()));
    }
    ui.label(RichText::new("Backstory").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.backstory).desired_rows(5).desired_width(ui.available_width()));
    ui.label(RichText::new("Allies & notes").color(DIM).small());
    ui.add(egui::TextEdit::multiline(&mut c.allies).desired_rows(3).desired_width(ui.available_width()));
    ui.add(egui::TextEdit::multiline(&mut c.notes).desired_rows(3).desired_width(ui.available_width()));
}

fn class_hd(name: &str) -> i32 {
    match name.to_lowercase().as_str() {
        "barbarian" => 12,
        "fighter" | "paladin" | "ranger" => 10,
        "wizard" | "sorcerer" => 6,
        _ => 8,
    }
}

fn is_asi_level(class: &str, lv: i32) -> bool {
    let fighterish = matches!(class.to_lowercase().as_str(), "fighter");
    let rogue = class.eq_ignore_ascii_case("rogue");
    if fighterish {
        matches!(lv, 4 | 6 | 8 | 12 | 14 | 16 | 19)
    } else if rogue {
        matches!(lv, 4 | 8 | 10 | 12 | 16 | 19)
    } else {
        matches!(lv, 4 | 8 | 12 | 16 | 19)
    }
}

fn features_for(class: &str, lv: i32) -> &'static str {
    match (class.to_lowercase().as_str(), lv) {
        ("fighter", 1) => "Fighting Style. Second Wind.",
        ("fighter", 2) => "Action Surge.",
        ("fighter", 5) => "Extra Attack.",
        ("rogue", 1) => "Sneak Attack. Expertise. Thieves' Cant.",
        ("rogue", 2) => "Cunning Action.",
        ("rogue", 5) => "Uncanny Dodge. Sneak Attack 3d6.",
        ("wizard", 1) => "Spellcasting. Arcane Recovery.",
        ("wizard", 2) => "Arcane Tradition.",
        ("cleric", 1) => "Spellcasting. Divine Domain.",
        ("cleric", 2) => "Channel Divinity.",
        ("barbarian", 1) => "Rage. Unarmored Defense.",
        ("barbarian", 2) => "Reckless Attack. Danger Sense.",
        ("paladin", 1) => "Divine Sense. Lay on Hands.",
        ("paladin", 2) => "Fighting Style. Spellcasting. Divine Smite.",
        ("ranger", 1) => "Favored Enemy. Natural Explorer.",
        ("bard", 1) => "Spellcasting. Bardic Inspiration.",
        ("monk", 1) => "Unarmored Defense. Martial Arts.",
        ("warlock", 1) => "Otherworldly Patron. Pact Magic.",
        ("sorcerer", 1) => "Spellcasting. Sorcerous Origin.",
        ("druid", 1) => "Druidic. Spellcasting.",
        _ if lv % 4 == 0 => "Ability Score Improvement.",
        _ => "Class feature.",
    }
}

fn ui_level_up(ui: &mut egui::Ui, c: &mut Character, blight: bool) {
    let next = (c.level + 1).min(if blight { 10 } else { 20 });
    wrap(
        ui,
        if blight {
            "Spend IP in one pass, then confirm. Rank, stats, and skills you click are queued until you confirm."
        } else {
            "One level at a time. If this level grants ability points, spend every point here before you confirm."
        },
        MUTED,
        12.0,
    );
    if blight {
        let spent: i32 = c.pending_ip.values().sum();
        wrap(ui, &format!("Queued spend {spent} IP  ·  purse {}", c.ip), CYAN, 12.0);
        if c.role_rank < 10 {
            let cost = (c.role_rank + 1) * 20;
            if theme::neon_btn(ui, &format!("Queue role rank → {} ({cost} IP)", c.role_rank + 1)).clicked()
            {
                c.pending_ip.insert("role".into(), cost);
            }
        }
        for (id, lab) in CP_STATS {
            let now = c.red(id);
            let cost = (now + 1) * 10;
            if theme::neon_btn(ui, &format!("{lab} {now}→{} · {cost} IP", now + 1)).clicked() {
                c.pending_ip.insert((*id).into(), cost);
            }
        }
        ui.horizontal(|ui| {
            if theme::neon_btn(ui, "Confirm spend").clicked() {
                let total: i32 = c.pending_ip.values().sum();
                if total <= c.ip {
                    c.ip -= total;
                    if c.pending_ip.contains_key("role") {
                        c.role_rank = (c.role_rank + 1).min(10);
                    }
                    for (id, _) in c.pending_ip.clone() {
                        if id == "role" {
                            continue;
                        }
                        let v = c.red(&id) + 1;
                        c.stats_red.insert(id, v);
                    }
                    let body = c.red("body");
                    c.hp_max += body.max(1);
                    c.hp = (c.hp + body.max(1)).min(c.hp_max);
                    c.level = next;
                    c.pending_ip.clear();
                    c.leveling = false;
                }
            }
            if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
                c.pending_ip.clear();
                c.leveling = false;
            }
        });
        return;
    }
    let asi = is_asi_level(&c.class_name, next);
    let spent: i32 = c.pending_asi.values().copied().sum();
    let need = if asi { 2 } else { 0 };
    wrap(
        ui,
        &format!(
            "Level {} → {}. Hit die d{}. {}",
            c.level,
            next,
            class_hd(&c.class_name),
            features_for(&c.class_name, next)
        ),
        CYAN,
        12.0,
    );
    if asi {
        wrap(ui, &format!("Ability points {spent}/{need}. Put them all in now — two in one score, or split."), ORANGE, 12.0);
        ui.horizontal_wrapped(|ui| {
            for (id, lab) in ABILS {
                let now = c.abil(id);
                let extra = *c.pending_asi.get(*id).unwrap_or(&0);
                if theme::neon_btn(ui, &format!("{lab} {now}{}", if extra > 0 { format!("+{extra}") } else { String::new() })).clicked()
                    && spent < need
                    && now + extra < 20
                {
                    c.pending_asi.insert((*id).into(), extra + 1);
                }
            }
        });
    }
    ui.horizontal(|ui| {
        let ready = !asi || spent == need;
        if ready && theme::neon_btn(ui, "Confirm level").clicked() {
            let hd = class_hd(&c.class_name);
            let con = Character::modifier(c.abil("con"));
            let gain = (hd / 2 + 1 + con).max(1);
            c.level = next;
            c.hp_max += gain;
            c.hp += gain;
            c.hit_dice = format!("{}d{hd}", c.level);
            for (id, n) in c.pending_asi.clone() {
                let v = (c.abil(&id) + n).min(20);
                c.abilities.insert(id, v);
            }
            let feat = features_for(&c.class_name, next);
            if !feat.is_empty() {
                if !c.features.is_empty() {
                    c.features.push('\n');
                }
                c.features.push_str(&format!("Lv{next}: {feat}"));
            }
            c.pending_asi.clear();
            c.leveling = false;
        }
        if theme::neon_btn_color(ui, "Cancel", KILL, false).clicked() {
            c.pending_asi.clear();
            c.leveling = false;
        }
    });
}

fn json_i32(v: &serde_json::Value, k: &str, default: i32) -> i32 {
    match v.get(k) {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(default as i64) as i32,
        Some(serde_json::Value::String(s)) => {
            let digits: String = s
                .chars()
                .skip_while(|c| !c.is_ascii_digit() && *c != '-')
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            digits.parse().unwrap_or(default)
        }
        _ => default,
    }
}

fn json_str(v: &serde_json::Value, k: &str) -> String {
    v.get(k)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn unique_name(list: &[Character], name: &str) -> String {
    let base = name.trim();
    let base = if base.is_empty() { "Creature" } else { base };
    if !list.iter().any(|c| c.name == base) {
        return base.into();
    }
    let mut n = 2;
    loop {
        let cand = format!("{base} {n}");
        if !list.iter().any(|c| c.name == cand) {
            return cand;
        }
        n += 1;
    }
}

fn lines_from(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|r| {
                if let Some(s) = r.as_str() {
                    return Some(s.to_string());
                }
                let n = r.get("n").or_else(|| r.get("name")).and_then(|x| x.as_str()).unwrap_or("");
                let d = r.get("d").or_else(|| r.get("text")).and_then(|x| x.as_str()).unwrap_or("");
                if n.is_empty() && d.is_empty() {
                    None
                } else if d.is_empty() {
                    Some(n.into())
                } else if n.is_empty() {
                    Some(d.into())
                } else {
                    Some(format!("{n}. {d}"))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn parse_action_attack(row: &serde_json::Value) -> Option<Attack> {
    let name = row
        .get("n")
        .or_else(|| row.get("name"))
        .and_then(|x| x.as_str())
        .unwrap_or("Attack")
        .to_string();
    let d = row
        .get("d")
        .or_else(|| row.get("text"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let bonus = d
        .split("to hit")
        .next()
        .and_then(|s| {
            s.rsplit(|c: char| !(c.is_ascii_digit() || c == '+' || c == '-' || c == ' '))
                .next()
                .map(|x| x.trim().replace(' ', ""))
        })
        .filter(|s| s.starts_with('+') || s.starts_with('-'));
    let damage = if let Some(i) = d.to_lowercase().find("hit:") {
        let rest = &d[i + 4..];
        if let Some(a) = rest.find('(') {
            if let Some(b) = rest[a + 1..].find(')') {
                Some(rest[a + 1..a + 1 + b].replace('\n', " ").trim().to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    if bonus.is_none() && damage.is_none() {
        let dice = d.split_whitespace().find(|w| w.contains('d') && w.chars().next().unwrap_or('x').is_ascii_digit());
        dice.map(|w| Attack {
            name,
            bonus: String::new(),
            damage: w.to_string(),
        })
    } else {
        Some(Attack {
            name,
            bonus: bonus.unwrap_or_default(),
            damage: damage.unwrap_or_default(),
        })
    }
}

fn level_from_cr(cr: &str) -> i32 {
    let s = cr.trim();
    if s.is_empty() || s == "0" || s.contains('/') {
        return 1;
    }
    s.parse::<f32>().map(|n| n.round() as i32).unwrap_or(1).clamp(1, 20)
}

fn parse_bonuses(name: &str, text: &str, props: &str) -> HashMap<String, i32> {
    let blob = format!("{name} {text} {props}").to_lowercase();
    let mut out = HashMap::new();
    let keys: &[(&str, &[&str])] = &[
        ("str", &["strength", "str"]),
        ("dex", &["dexterity", "dex"]),
        ("con", &["constitution", "con"]),
        ("int", &["intelligence", "int"]),
        ("wis", &["wisdom", "wis"]),
        ("cha", &["charisma", "cha"]),
        ("body", &["body"]),
        ("ref", &["reflexes", "ref"]),
        ("cool", &["cool"]),
        ("will", &["will"]),
        ("emp", &["empathy", "emp"]),
        ("tech", &["tech"]),
        ("move", &["move", "movement"]),
    ];
    for (id, names) in keys {
        let mut best: i32 = 0;
        for n in *names {
            if let Some(v) = nearby_plus(&blob, n) {
                if v.abs() > best.abs() {
                    best = v;
                }
            }
        }
        if best != 0 {
            out.insert((*id).into(), best);
        }
    }
    out
}

fn nearby_plus(blob: &str, key: &str) -> Option<i32> {
    let mut from = 0;
    while let Some(rel) = blob[from..].find(key) {
        let i = from + rel;
        if i > 0 {
            let prev = blob.as_bytes()[i - 1] as char;
            if prev.is_ascii_alphanumeric() {
                from = i + key.len();
                continue;
            }
        }
        let after_i = i + key.len();
        if after_i < blob.len() {
            let next = blob.as_bytes()[after_i] as char;
            if next.is_ascii_alphanumeric() {
                from = i + key.len();
                continue;
            }
        }
        let lo = i.saturating_sub(10);
        let hi = (after_i + 10).min(blob.len());
        let window = &blob[lo..hi];
        if let Some(n) = plus_in(window) {
            return Some(n);
        }
        from = i + key.len();
    }
    None
}

fn plus_in(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' || bytes[i] == b'-' {
            let sign = if bytes[i] == b'-' { -1 } else { 1 };
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            let start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                if let Ok(n) = s[start..j].parse::<i32>() {
                    return Some(sign * n);
                }
            }
        }
        i += 1;
    }
    None
}

pub fn item_min_level(v: &serde_json::Value) -> i32 {
    if let Some(lv) = v.get("lv").and_then(|x| x.as_i64()) {
        return (lv as i32).max(1);
    }
    let name = json_str(v, "name").to_lowercase();
    if name.contains("+3") {
        return 12;
    }
    if name.contains("+2") {
        return 8;
    }
    if name.contains("+1") {
        return 5;
    }
    let rarity = json_str(v, "rarity").to_lowercase();
    let from_rarity = match rarity.as_str() {
        "legendary" | "artifact" => 17,
        "very rare" => 11,
        "rare" => 7,
        "uncommon" => 3,
        "common" => 1,
        _ => 0,
    };
    if from_rarity > 0 {
        return from_rarity;
    }
    let hl = json_i32(v, "hl", 0);
    if hl >= 14 {
        return 8;
    }
    if hl >= 8 {
        return 5;
    }
    if hl >= 4 {
        return 3;
    }
    let cost_s = json_str(v, "cost");
    let n: i32 = cost_s
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    let blight = cost_s.to_lowercase().contains("eb") || json_str(v, "world") == "blight";
    if blight {
        if n >= 5000 {
            8
        } else if n >= 2000 {
            6
        } else if n >= 500 {
            4
        } else if n >= 100 {
            2
        } else {
            1
        }
    } else if n >= 5000 {
        11
    } else if n >= 1000 {
        8
    } else if n >= 200 {
        5
    } else if n >= 50 {
        3
    } else {
        1
    }
}

pub fn kit_spec_from_json(v: &serde_json::Value) -> KitSpec {
    KitSpec {
        id: json_str(v, "id"),
        name: json_str(v, "name"),
        kind: json_str(v, "kind"),
        dmg: json_str(v, "dmg"),
        cost: json_str(v, "cost"),
        detail: json_str(v, "text"),
        ac: match v.get("ac") {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => String::new(),
        },
        props: json_str(v, "props"),
        text: json_str(v, "text"),
        cat: json_str(v, "cat"),
        hl: json_i32(v, "hl", 0),
        lv: json_i32(v, "lv", 0),
        rarity: json_str(v, "rarity"),
    }
}

pub fn receive_kit(c: &mut Character, spec: &KitSpec) {
    let item = KitItem {
        id: format!("k_{}", rand::random::<u32>()),
        catalog_id: spec.id.clone(),
        name: spec.name.clone(),
        kind: spec.kind.clone(),
        dmg: spec.dmg.clone(),
        cost: spec.cost.clone(),
        cat: spec.cat.clone(),
        detail: if spec.detail.is_empty() {
            spec.text.clone()
        } else {
            spec.detail.clone()
        },
        ac: spec.ac.clone(),
        props: spec.props.clone(),
        rarity: spec.rarity.clone(),
        hl: spec.hl,
        lv: spec.lv,
        equipped: false,
        bonuses: parse_bonuses(&spec.name, &spec.text, &spec.props),
    };
    if !c.gear.iter().any(|g| g == &item.name) {
        c.gear.push(item.name.clone());
    }
    c.kit.push(item);
}

pub fn set_equipped(c: &mut Character, idx: usize, on: bool) {
    let Some(item) = c.kit.get(idx).cloned() else {
        return;
    };
    if on && item.kind.eq_ignore_ascii_case("armor") {
        let others: Vec<String> = c
            .kit
            .iter()
            .enumerate()
            .filter(|(i, k)| {
                *i != idx && k.kind.eq_ignore_ascii_case("armor") && k.equipped
            })
            .map(|(_, k)| k.name.clone())
            .collect();
        for k in c.kit.iter_mut() {
            if others.iter().any(|n| n == &k.name) {
                k.equipped = false;
            }
        }
        for n in &others {
            strip_attack(c, n);
        }
    }
    if let Some(k) = c.kit.get_mut(idx) {
        if k.equipped == on {
            return;
        }
        k.equipped = on;
    }
    if item.kind.eq_ignore_ascii_case("chrome") || item.hl > 0 {
        if on {
            c.humanity = (c.humanity - item.hl).max(0);
        } else {
            c.humanity = (c.humanity + item.hl).min(100);
        }
    }
    let weapon = item.kind.eq_ignore_ascii_case("weapon") || !item.dmg.is_empty();
    if weapon {
        if on {
            if !c.attacks.iter().any(|a| a.name == item.name) {
                c.attacks.push(Attack {
                    name: item.name.clone(),
                    bonus: String::new(),
                    damage: if item.dmg.is_empty() {
                        "1d6".into()
                    } else {
                        item.dmg.clone()
                    },
                });
            }
        } else {
            strip_attack(c, &item.name);
        }
    }
}

fn strip_attack(c: &mut Character, name: &str) {
    c.attacks.retain(|a| a.name != name);
    if c.attacks.is_empty() {
        c.attacks.push(Attack {
            name: "Punch".into(),
            bonus: String::new(),
            damage: "1d4".into(),
        });
    }
}

pub fn short_rest(c: &mut Character, log: &mut Vec<String>) {
    if c.dead {
        log.push(format!("{} is dead. Rest will not raise them.", c.name));
        return;
    }
    if c.is_blight() {
        let body = c.red_now("body");
        c.hp = (c.hp + body).min(c.hp_max);
        log.push(format!("{} takes a short rest (+{body} HP).", c.name));
    } else if c.hd_used < c.level {
        let die = class_hd(&c.class_name);
        let con = Character::modifier(c.abil_now("con")).max(0);
        let roll = rand::thread_rng().gen_range(1..=die) + con;
        c.hp = (c.hp + roll).min(c.hp_max);
        c.hd_used += 1;
        log.push(format!("{} spends a hit die on a short rest (+{roll} HP).", c.name));
    } else {
        log.push(format!("{} has no hit dice left for a short rest.", c.name));
    }
    if c.hp > 0 {
        c.downed = false;
    }
}

pub fn long_rest(c: &mut Character, log: &mut Vec<String>) {
    if c.dead {
        log.push(format!("{} is dead. A long rest will not raise them.", c.name));
        return;
    }
    if c.is_blight() {
        let heal = c.red_now("body") + c.red_now("will");
        c.hp = (c.hp + heal).min(c.hp_max);
        log.push(format!("{} takes a long rest (+{heal} HP).", c.name));
    } else {
        c.hp = c.hp_max;
        let recover = (c.level / 2).max(1);
        c.hd_used = (c.hd_used - recover).max(0);
        c.exhaustion = (c.exhaustion - 1).max(0);
        c.slots_used = vec![0; 9];
        log.push(format!("{} finishes a long rest. HP full, slots returned.", c.name));
    }
    c.death_ok = 0;
    c.death_fail = 0;
    c.downed = false;
}

pub fn death_save(
    c: &mut Character,
    luck: crate::dice::Luck,
    dice: &mut String,
    log: &mut Vec<String>,
    roll: &mut Option<crate::dice::Roll>,
) {
    let r = crate::dice::fire("Death save", "", false, luck, "");
    let n = r.pct;
    if n >= 100 {
        c.death_ok = 3;
        c.hp = 1;
        c.downed = false;
        *dice = format!("{n} — critical: stands at 1 HP");
    } else if n >= 40 {
        c.death_ok = (c.death_ok + 1).min(3);
        *dice = format!("{n} — success ({}/3)", c.death_ok);
        if c.death_ok >= 3 {
            c.hp = 1;
            c.downed = false;
            c.death_ok = 0;
            c.death_fail = 0;
        }
    } else if n == 0 {
        c.death_fail = 3;
        *dice = format!("{n} — critical fail");
    } else {
        c.death_fail = (c.death_fail + 1).min(3);
        *dice = format!("{n} — fail ({}/3)", c.death_fail);
    }
    if c.death_fail >= 3 {
        c.dead = true;
        c.downed = false;
    }
    log.push(format!("{} death save: {}", c.name, dice));
    *roll = Some(r);
}

fn attack_stat<'a>(c: &'a Character, name: &str, dmg: &str) -> &'a str {
    let blob = format!("{name} {dmg}").to_lowercase();
    if c.is_blight() {
        if blob.contains("hack") || blob.contains("program") || blob.contains("net") {
            "int"
        } else if ["bow", "pistol", "rifle", "smg", "shotgun", "gun", "thrown", "dart"]
            .iter()
            .any(|w| blob.contains(w))
        {
            "ref"
        } else {
            "body"
        }
    } else if blob.contains("spell") || blob.contains("cantrip") || blob.contains("ray") || blob.contains("bolt") {
        if c.spell_ability.is_empty() {
            "int"
        } else {
            &c.spell_ability
        }
    } else if blob.contains("finesse")
        || blob.contains("rapier")
        || blob.contains("dagger")
        || blob.contains("scimitar")
        || blob.contains("shortsword")
    {
        if Character::modifier(c.abil_now("dex")) >= Character::modifier(c.abil_now("str")) {
            "dex"
        } else {
            "str"
        }
    } else if ["bow", "crossbow", "dart", "sling", "gun", "pistol", "thrown"]
        .iter()
        .any(|w| blob.contains(w))
    {
        "dex"
    } else {
        "str"
    }
}

fn scaled_attack(c: &Character, atk: &Attack) -> (String, String, i32) {
    let blight = c.is_blight();
    let stat = attack_stat(c, &atk.name, &atk.damage);
    let magic = atk
        .name
        .split('+')
        .nth(1)
        .and_then(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<i32>().ok())
        .unwrap_or(0);
    if blight {
        let score = c.red_now(stat);
        let hit = score + c.role_rank + magic;
        let extra = score / 2 + c.role_rank;
        let dmg = if atk.damage.trim().is_empty() {
            format!("1d6{:+}", extra)
        } else if extra != 0 {
            format!("{}{:+}", atk.damage.trim(), extra)
        } else {
            atk.damage.clone()
        };
        (signed(hit), dmg, extra)
    } else {
        let m = Character::modifier(c.abil_now(stat));
        let hit = m + Character::prof(c.level) + magic;
        let extra = m + c.level / 2;
        let dmg = if atk.damage.trim().is_empty() {
            format!("1d4{:+}", extra)
        } else if extra != 0 {
            format!("{}{:+}", atk.damage.trim(), extra)
        } else {
            atk.damage.clone()
        };
        (signed(hit), dmg, extra)
    }
}

pub fn sheet_from_catalog(
    cat: &str,
    row: &serde_json::Value,
    existing: &[Character],
) -> Character {
    let blight = matches!(cat, "Datashard" | "Faces" | "Gangs" | "Corps");
    match cat {
        "Bestiary" | "NPCs" => sheet_from_5e(row, existing, cat),
        "Gods" => sheet_from_god(row, existing),
        "Datashard" | "Faces" => sheet_from_red(row, existing, cat),
        _ => {
            if blight {
                sheet_from_red(row, existing, cat)
            } else {
                sheet_from_5e(row, existing, cat)
            }
        }
    }
}

fn sheet_from_5e(row: &serde_json::Value, existing: &[Character], cat: &str) -> Character {
    let mut ch = Character::new("hearthsong");
    ch.name = unique_name(existing, &json_str(row, "name"));
    ch.player = "NPC".into();
    ch.npc = true;
    ch.owner = "gm".into();
    ch.race = [json_str(row, "size"), json_str(row, "type"), json_str(row, "sub")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    ch.class_name = {
        let r = json_str(row, "role");
        if r.is_empty() {
            json_str(row, "type")
        } else {
            r
        }
    };
    if ch.class_name.is_empty() {
        ch.class_name = cat.into();
    }
    ch.level = level_from_cr(&json_str(row, "crLabel"));
    if ch.level == 1 {
        ch.level = level_from_cr(&json_str(row, "cr"));
    }
    ch.alignment = json_str(row, "align");
    ch.xp = json_i32(row, "xp", 0);
    ch.ac = json_i32(row, "ac", 10);
    ch.hp = json_i32(row, "hp", 8);
    ch.hp_max = ch.hp;
    ch.speed = {
        let s = json_str(row, "speed");
        if s.is_empty() {
            "30 ft.".into()
        } else {
            s
        }
    };
    let hd = json_str(row, "hd");
    if !hd.is_empty() {
        ch.hit_dice = hd;
    }
    for (id, _) in ABILS {
        if row.get(*id).is_some() {
            ch.abilities.insert((*id).into(), json_i32(row, id, 10));
        }
    }
    let saves = json_str(row, "saves").to_uppercase();
    for (id, lab) in ABILS {
        if saves.contains(lab) {
            ch.save_prof.insert((*id).into(), true);
        }
    }
    let skills = json_str(row, "skills");
    for (id, name, _) in HEARTH_SKILLS {
        if skills.to_lowercase().contains(&name.to_lowercase()) {
            ch.skill_prof.insert((*id).into(), true);
        }
    }
    let mut leftover = String::new();
    let mut attacks = Vec::new();
    let acts = row
        .get("actions")
        .or_else(|| row.get("weapons"))
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    if let serde_json::Value::Array(arr) = acts {
        for a in arr {
            if let Some(atk) = parse_action_attack(&a) {
                attacks.push(atk);
            } else {
                let line = lines_from(&serde_json::Value::Array(vec![a]));
                if !line.is_empty() {
                    leftover.push_str(&line);
                    leftover.push('\n');
                }
            }
        }
    }
    if !attacks.is_empty() {
        ch.attacks = attacks;
    }
    ch.languages = json_str(row, "lang");
    let mut feat = lines_from(row.get("traits").unwrap_or(&serde_json::Value::Null));
    if !leftover.is_empty() {
        feat.push_str("\n\n");
        feat.push_str(&leftover);
    }
    let leg = lines_from(row.get("legendary").unwrap_or(&serde_json::Value::Null));
    if !leg.is_empty() {
        feat.push_str("\n\nLegendary\n");
        feat.push_str(&leg);
    }
    ch.features = feat;
    ch.notes = [
        json_str(row, "blurb"),
        json_str(row, "text"),
        {
            let s = json_str(row, "senses");
            if s.is_empty() {
                String::new()
            } else {
                format!("Senses: {s}")
            }
        },
        {
            let s = json_str(row, "skills");
            if s.is_empty() {
                String::new()
            } else {
                format!("Skills: {s}")
            }
        },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");
    ch
}

fn sheet_from_red(row: &serde_json::Value, existing: &[Character], cat: &str) -> Character {
    let mut ch = Character::new("blight");
    ch.name = unique_name(existing, &json_str(row, "name"));
    ch.handle = json_str(row, "name");
    ch.player = "NPC".into();
    ch.npc = true;
    ch.owner = "gm".into();
    let role = json_str(row, "role");
    ch.role = if role.is_empty() { cat.into() } else { role };
    ch.role_rank = json_i32(row, "rank", 4).clamp(1, 10);
    ch.level = ch.role_rank;
    ch.hp = json_i32(row, "hp", 40);
    ch.hp_max = ch.hp;
    let sp = json_str(row, "sp");
    ch.ac = if sp.contains('/') {
        sp.split('/')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(11)
    } else if !sp.is_empty() {
        json_i32(row, "sp", 11)
    } else {
        json_i32(row, "ac", 11)
    };
    ch.humanity = json_i32(row, "humanity", 50);
    if row.get("move").is_some() {
        ch.speed = format!("{} m", json_i32(row, "move", 6));
    }
    for (id, _) in CP_STATS {
        if row.get(*id).is_some() {
            ch.stats_red.insert((*id).into(), json_i32(row, id, 5));
        }
    }
    if let serde_json::Value::Array(w) = row.get("weapons").cloned().unwrap_or(serde_json::Value::Array(vec![])) {
        let mut attacks = Vec::new();
        for a in w {
            let name = a
                .get("n")
                .or_else(|| a.get("name"))
                .and_then(|x| x.as_str())
                .unwrap_or("Weapon")
                .to_string();
            let d = a.get("d").and_then(|x| x.as_str()).unwrap_or("");
            let damage = d
                .split('·')
                .next()
                .unwrap_or(d)
                .trim()
                .to_string();
            attacks.push(Attack {
                name,
                bonus: String::new(),
                damage,
            });
        }
        if !attacks.is_empty() {
            ch.attacks = attacks;
        }
    }
    ch.cyberware = json_str(row, "chrome");
    ch.equipment = [json_str(row, "gear"), json_str(row, "skills")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    ch.features = lines_from(row.get("traits").unwrap_or(&serde_json::Value::Null));
    ch.lifepath = [
        json_str(row, "blurb"),
        json_str(row, "text"),
        json_str(row, "look"),
        json_str(row, "origin"),
        {
            let d = json_str(row, "district");
            if d.is_empty() {
                String::new()
            } else {
                format!("District: {d}")
            }
        },
        {
            let a = json_str(row, "affiliation");
            if a.is_empty() {
                String::new()
            } else {
                format!("Affiliation: {a}")
            }
        },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");
    ch.notes = ch.lifepath.clone();
    ch
}

fn sheet_from_god(row: &serde_json::Value, existing: &[Character]) -> Character {
    let mut ch = Character::new("hearthsong");
    ch.name = unique_name(existing, &json_str(row, "name"));
    ch.player = "NPC".into();
    ch.npc = true;
    ch.owner = "gm".into();
    ch.class_name = "Deity".into();
    ch.race = json_str(row, "kind");
    ch.alignment = json_str(row, "align");
    ch.deity = json_str(row, "name");
    let rank = json_str(row, "rank").to_lowercase();
    let (hp, ac, lv, score) = if rank.contains("greater") {
        (420, 24, 20, 30)
    } else if rank.contains("inter") {
        (300, 22, 16, 26)
    } else if rank.contains("demi") {
        (180, 20, 12, 22)
    } else {
        (240, 21, 14, 24)
    };
    ch.hp = hp;
    ch.hp_max = hp;
    ch.ac = ac;
    ch.level = lv;
    for (id, _) in ABILS {
        ch.abilities.insert((*id).into(), score);
    }
    ch.speed = "fly 60 ft., walk 40 ft.".into();
    ch.features = [
        json_str(row, "domains"),
        json_str(row, "worship"),
        json_str(row, "symbol"),
        json_str(row, "blurb"),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");
    ch.notes = [json_str(row, "text"), json_str(row, "look"), json_str(row, "aka")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    ch.attacks = vec![
        Attack {
            name: "Divine Strike".into(),
            bonus: signed(Character::modifier(score) + Character::prof(lv)),
            damage: "4d8 radiant".into(),
        },
        Attack {
            name: "Domain Wrath".into(),
            bonus: signed(Character::modifier(score) + Character::prof(lv)),
            damage: "3d10".into(),
        },
    ];
    ch
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::load_list;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn new_sheets_have_unarmed_and_world() {
        let h = Character::new("hearthsong");
        assert!(!h.is_blight());
        assert_eq!(h.attacks.len(), 4);
        assert!(h.attacks.iter().any(|a| a.name == "Punch"));
        let b = Character::new("blight");
        assert!(b.is_blight());
        assert_eq!(b.eddies, 500);
        assert_eq!(b.role_rank, 4);
    }

    #[test]
    fn rest_and_death_work() {
        for _ in 0..8 {
            let mut c = Character::new("hearthsong");
            c.class_name = "Fighter".into();
            c.hp = 2;
            c.hp_max = 20;
            c.level = 4;
            c.hd_used = 0;
            let mut log = vec![];
            short_rest(&mut c, &mut log);
            assert!(c.hp > 2);
            assert_eq!(c.hd_used, 1);
            c.hp = 1;
            long_rest(&mut c, &mut log);
            assert_eq!(c.hp, c.hp_max);
            assert_eq!(c.hd_used, 0);
            c.dead = true;
            c.hp = 0;
            long_rest(&mut c, &mut log);
            assert_eq!(c.hp, 0);

            let mut b = Character::new("blight");
            b.hp = 5;
            b.hp_max = 40;
            short_rest(&mut b, &mut log);
            assert!(b.hp >= 5);
        }
        let mut c = Character::new("hearthsong");
        c.hp = 0;
        c.downed = true;
        let mut dice = String::new();
        let mut log = vec![];
        let mut roll = None;
        for _ in 0..6 {
            if c.dead {
                break;
            }
            death_save(&mut c, crate::dice::Luck::Norm, &mut dice, &mut log, &mut roll);
        }
        assert!(c.dead || c.hp == 1 || c.death_ok > 0 || c.death_fail > 0);
    }

    #[test]
    fn equip_applies_and_clears_stat_boosts() {
        let mut c = Character::new("hearthsong");
        let base = c.abil_now("str");
        receive_kit(
            &mut c,
            &KitSpec {
                id: "gaunt".into(),
                name: "Belt of Strength +2".into(),
                kind: "magic".into(),
                dmg: String::new(),
                cost: "4000 gp".into(),
                detail: "strength +2".into(),
                ac: String::new(),
                props: String::new(),
                text: "grants strength +2".into(),
                cat: "wondrous".into(),
                hl: 0,
                lv: 0,
                rarity: "rare".into(),
            },
        );
        assert_eq!(c.kit.len(), 1);
        assert!(!c.kit[0].equipped);
        set_equipped(&mut c, 0, true);
        assert!(c.kit[0].equipped);
        assert_eq!(c.bonus_for("str"), 2);
        assert_eq!(c.abil_now("str"), base + 2);
        set_equipped(&mut c, 0, false);
        assert_eq!(c.bonus_for("str"), 0);
        assert_eq!(c.abil_now("str"), base);
    }

    #[test]
    fn equip_weapon_adds_attack_unequip_removes() {
        let mut c = Character::new("hearthsong");
        receive_kit(
            &mut c,
            &KitSpec {
                id: "ls".into(),
                name: "Longsword".into(),
                kind: "weapon".into(),
                dmg: "1d8 slashing".into(),
                cost: "15 gp".into(),
                detail: String::new(),
                ac: String::new(),
                props: "versatile".into(),
                text: String::new(),
                cat: "martial melee".into(),
                hl: 0,
                lv: 0,
                rarity: String::new(),
            },
        );
        set_equipped(&mut c, 0, true);
        assert!(c.attacks.iter().any(|a| a.name == "Longsword"));
        set_equipped(&mut c, 0, false);
        assert!(!c.attacks.iter().any(|a| a.name == "Longsword"));
        assert!(c.attacks.iter().any(|a| a.name == "Punch"));
    }

    #[test]
    fn vendor_level_gate() {
        let cheap = serde_json::json!({"name":"Club","cost":"1 sp","world":"hearthsong"});
        let plus = serde_json::json!({"name":"Armor, +2","rarity":"rare"});
        let spell9 = serde_json::json!({"name":"Wish","lv":9});
        let chrome = serde_json::json!({"name":"Sandevistan","cost":"5000 eb","hl":14,"world":"blight"});
        assert_eq!(item_min_level(&cheap), 1);
        assert_eq!(item_min_level(&plus), 8);
        assert_eq!(item_min_level(&spell9), 9);
        assert_eq!(item_min_level(&chrome), 8);
        assert!(item_min_level(&spell9) > 3);
    }

    #[test]
    fn catalog_sheets_copy_stats() {
        let root = root();
        let bestiary = load_list(&root, "bestiary.json");
        let goblin = bestiary
            .iter()
            .find(|r| r.get("id").and_then(|x| x.as_str()) == Some("goblin"))
            .expect("goblin");
        let c = sheet_from_catalog("Bestiary", goblin, &[]);
        assert!(c.npc);
        assert_eq!(c.owner, "gm");
        assert!(c.hp > 0);
        assert!(c.ac > 0);
        assert_eq!(c.abil("str"), json_i32(goblin, "str", 10));

        let gods = load_list(&root, "gods.json");
        let g = sheet_from_catalog("Gods", &gods[0], &[]);
        assert!(g.hp >= 180);
        assert_eq!(g.class_name, "Deity");
        assert!(g.attacks.iter().any(|a| a.name.contains("Divine")));

        let shard = load_list(&root, "datashard.json");
        let d = sheet_from_catalog("Datashard", &shard[0], &[]);
        assert!(d.is_blight());
        assert!(d.npc);
        assert!(d.hp > 0);
    }

    #[test]
    fn unique_names_and_save_roundtrip() {
        let a = Character::new("hearthsong");
        let mut b = Character::new("hearthsong");
        b.name = a.name.clone();
        let n = unique_name(&[a.clone()], &b.name);
        assert_ne!(n, a.name);
        let dir = std::env::temp_dir().join(format!("bn-test-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = Character::new("blight");
        c.name = "Rook".into();
        save(&dir, &[c.clone()]);
        let loaded = load(&dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Rook");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn attack_scales_with_level_and_stats() {
        let mut c = Character::new("hearthsong");
        c.level = 8;
        c.abilities.insert("str".into(), 16);
        let atk = Attack {
            name: "Longsword".into(),
            bonus: String::new(),
            damage: "1d8".into(),
        };
        let (hit, dmg, extra) = scaled_attack(&c, &atk);
        assert!(hit.contains('+'));
        assert!(dmg.contains("1d8"));
        assert!(extra >= 3);
        let mut b = Character::new("blight");
        b.role_rank = 6;
        b.stats_red.insert("body".into(), 8);
        let atk = Attack {
            name: "Punch".into(),
            bonus: String::new(),
            damage: "1d6".into(),
        };
        let (_, _, extra) = scaled_attack(&b, &atk);
        assert_eq!(extra, 8 / 2 + 6);
    }

    #[test]
    fn ac_now_uses_equipped_armor() {
        let mut c = Character::new("hearthsong");
        c.ac = 10;
        receive_kit(
            &mut c,
            &KitSpec {
                id: "plate".into(),
                name: "Plate".into(),
                kind: "armor".into(),
                dmg: String::new(),
                cost: "1500 gp".into(),
                detail: String::new(),
                ac: "18".into(),
                props: String::new(),
                text: String::new(),
                cat: "heavy".into(),
                hl: 0,
                lv: 0,
                rarity: String::new(),
            },
        );
        assert_eq!(c.ac_now(), 10);
        set_equipped(&mut c, 0, true);
        assert_eq!(c.ac_now(), 18);
    }
}
