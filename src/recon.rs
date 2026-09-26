//! Local case files. Nothing here is sent on the wire.

use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED, PANEL};
use eframe::egui::{self, Color32, RichText, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Dossier {
    pub id: String,
    #[serde(default = "kind_person")]
    pub kind: String,
    #[serde(default)]
    pub file_no: String,
    #[serde(default = "class_open")]
    pub class: String,
    #[serde(default = "status_active")]
    pub status: String,
    #[serde(default)]
    pub portrait: String,
    #[serde(default)]
    pub company_id: String,
    #[serde(default)]
    pub fields: HashMap<String, String>,
}

fn kind_person() -> String {
    "person".into()
}
fn class_open() -> String {
    "OPEN".into()
}
fn status_active() -> String {
    "ACTIVE".into()
}

impl Dossier {
    pub fn person(n: u32) -> Self {
        Self::fresh("person", n)
    }

    pub fn company(n: u32) -> Self {
        Self::fresh("company", n)
    }

    fn fresh(kind: &str, n: u32) -> Self {
        Self {
            id: format!("rc-{:08x}", rand::random::<u32>()),
            kind: kind.into(),
            file_no: format!("R-{n:04}"),
            class: "OPEN".into(),
            status: "ACTIVE".into(),
            portrait: String::new(),
            company_id: String::new(),
            fields: HashMap::new(),
        }
    }

    pub fn title(&self) -> String {
        let key = if self.kind == "company" { "legal" } else { "name" };
        let name = self.fields.get(key).map(|s| s.trim()).unwrap_or("");
        if name.is_empty() {
            if self.kind == "company" {
                "UNNAMED COMPANY".into()
            } else {
                "UNNAMED PERSON".into()
            }
        } else {
            name.to_uppercase()
        }
    }

    pub fn is_company(&self) -> bool {
        self.kind == "company"
    }

    fn hit(&self, q: &str) -> bool {
        if q.is_empty() {
            return true;
        }
        let extra = ["handle", "aliases", "street", "legal", "ticker", "role", "gang"]
            .into_iter()
            .filter_map(|k| self.fields.get(k).map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{} {} {} {} {}",
            self.file_no,
            self.title(),
            self.class,
            self.status,
            extra
        )
        .to_lowercase()
        .contains(q)
    }
}

pub fn load(root: &Path) -> Vec<Dossier> {
    let path = root.join("data/recon.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(root: &Path, files: &[Dossier]) {
    let dir = root.join("data");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(text) = serde_json::to_string_pretty(files) {
        let _ = std::fs::write(dir.join("recon.json"), text);
    }
}

pub fn next_no(files: &[Dossier]) -> u32 {
    files
        .iter()
        .filter_map(|d| d.file_no.strip_prefix("R-"))
        .filter_map(|s| s.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

pub fn store_picture(root: &Path, id: &str, src: &Path) -> Result<String, String> {
    let raw = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let ext = match raw.as_str() {
        "png" | "jpg" | "webp" => raw,
        "jpeg" => "jpg".to_string(),
        _ => "png".to_string(),
    };
    let dir = root.join("data/recon");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("{id}.{ext}");
    std::fs::copy(src, dir.join(&name)).map_err(|e| e.to_string())?;
    for old in ["png", "jpg", "webp"] {
        if old != ext {
            let _ = std::fs::remove_file(dir.join(format!("{id}.{old}")));
        }
    }
    Ok(format!("data/recon/{name}"))
}

const PERSON: &[(&str, &[(&str, &str, bool)])] = &[
    (
        "IDENTITY",
        &[
            ("name", "Name", false),
            ("aliases", "Aliases", false),
            ("handle", "Handle", false),
            ("born", "Born", false),
            ("age", "Age", false),
            ("gender", "Presented gender", false),
            ("ancestry", "Ancestry / body", false),
            ("role", "Role", false),
        ],
    ),
    (
        "AFFILIATION",
        &[
            ("employer", "Employer", false),
            ("gang", "Gang or crew", false),
        ],
    ),
    (
        "LAST SEEN",
        &[
            ("district", "District", false),
            ("street", "Street", false),
            ("address", "Address", true),
        ],
    ),
    (
        "REACH",
        &[
            ("comm", "Comm", false),
            ("drop", "Dead drop", true),
        ],
    ),
    (
        "LOOK",
        &[
            ("height", "Height", false),
            ("build", "Build", false),
            ("hair", "Hair", false),
            ("eyes", "Eyes", false),
            ("marks", "Marks", true),
            ("chrome", "Chrome or scars", true),
        ],
    ),
    (
        "CAPABLE",
        &[
            ("skills", "Skills", true),
            ("weapons", "Weapons", true),
            ("tricks", "Known tricks", true),
        ],
    ),
    (
        "HOLDINGS",
        &[
            ("money", "Money", false),
            ("property", "Property", true),
            ("assets", "Other assets", true),
        ],
    ),
    (
        "PEOPLE",
        &[
            ("associates", "Associates", true),
            ("enemies", "Enemies", true),
            ("family", "Family", true),
        ],
    ),
    ("HISTORY", &[("history", "Work history", true)]),
    (
        "LEVER",
        &[
            ("vice", "Vice", false),
            ("tell", "Tell", false),
            ("want", "What they want", true),
            ("turn", "How to turn them", true),
        ],
    ),
    (
        "ASSESSMENT",
        &[
            ("threat", "Threat", true),
            ("source", "How sure the source is", false),
        ],
    ),
    ("NOTES", &[("notes", "Notes", true)]),
];

const COMPANY: &[(&str, &[(&str, &str, bool)])] = &[
    (
        "IDENTITY",
        &[
            ("legal", "Legal name", false),
            ("street", "Street name", false),
            ("ticker", "Ticker", false),
            ("trade", "Trade", false),
        ],
    ),
    (
        "GROUND",
        &[
            ("hq", "Headquarters", false),
            ("districts", "Districts", true),
        ],
    ),
    (
        "SIZE",
        &[
            ("size", "Size", false),
            ("headcount", "People on the books", false),
            ("money", "Money", false),
        ],
    ),
    (
        "LEAD",
        &[("leadership", "Who runs it", true)],
    ),
    (
        "FACE",
        &[
            ("public", "What they say they are", true),
            ("actual", "What they actually do", true),
        ],
    ),
    (
        "WORK",
        &[
            ("products", "What they sell", true),
            ("subsidiaries", "Subsidiaries", true),
            ("rivals", "Rivals", true),
        ],
    ),
    (
        "SECURITY",
        &[
            ("security", "How hard to walk into", true),
            ("trouble", "Legal trouble", true),
        ],
    ),
    (
        "INSIDE",
        &[
            ("talkers", "Who inside will talk", true),
            ("rumors", "Rumors", true),
        ],
    ),
    (
        "ASSESSMENT",
        &[
            ("threat", "Threat to this table", true),
            ("source", "How sure the source is", false),
        ],
    ),
    ("NOTES", &[("notes", "Notes", true)]),
];

pub fn paint(
    ui: &mut egui::Ui,
    root: &Path,
    files: &mut Vec<Dossier>,
    index: &mut usize,
    query: &mut String,
    arm: &mut String,
    tex: &mut crate::images::TexCache,
    zoom: &mut Option<PathBuf>,
    blight: bool,
    share: &mut Option<Dossier>,
) -> bool {
    let mut changed = false;
    let rect = ui.available_rect_before_wrap();
    ui.allocate_rect(rect, egui::Sense::hover());
    let side_w = (rect.width() * 0.30).clamp(200.0, 320.0);
    let (left, right) = rect.split_left_right_at_x(rect.left() + side_w);
    let left = left.shrink2(Vec2::new(8.0, 6.0));
    let right = right.shrink2(Vec2::new(10.0, 6.0));
    ui.painter().vline(
        right.left(),
        rect.y_range(),
        egui::Stroke::new(1.0, theme::HOT),
    );
    let mut index_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    index_ui.set_clip_rect(left);
    let before_i = *index;
    if paint_index(&mut index_ui, files, index, query) {
        changed = true;
    }
    if *index != before_i {
        arm.clear();
    }
    let mut file_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    file_ui.set_clip_rect(right);
    let h = right.height().max(40.0);
    egui::ScrollArea::vertical()
        .id_salt("recon-file")
        .max_height(h)
        .auto_shrink([false, false])
        .show(&mut file_ui, |ui| {
            ui.set_width((right.width() - 18.0).max(40.0));
            if files.is_empty() {
                ui.label(
                    RichText::new("NO OPEN FILE")
                        .family(theme::display())
                        .size(28.0)
                        .color(theme::ACID),
                );
                theme::kicker(ui, "RECON://EMPTY");
                ui.add(
                    egui::Label::new(
                        RichText::new("New person or New company. The file stays on this computer.")
                            .color(MUTED)
                            .size(13.0),
                    )
                    .wrap(),
                );
                return;
            }
            *index = (*index).min(files.len() - 1);
            if paint_file(ui, root, files, index, arm, tex, zoom, blight, share) {
                changed = true;
            }
        });
    changed
}

fn paint_index(ui: &mut egui::Ui, files: &mut Vec<Dossier>, index: &mut usize, query: &mut String) -> bool {
    let mut changed = false;
    ui.label(
        RichText::new("RECON")
            .family(theme::display())
            .size(26.0)
            .color(theme::ACID),
    );
    theme::kicker(ui, "CASE://LOCAL");
    ui.label(
        RichText::new(format!("{:02} FILES ON THIS DECK", files.len()))
            .family(theme::mono())
            .size(11.0)
            .color(DIM),
    );
    ui.add(
        egui::TextEdit::singleline(query)
            .hint_text("Find a file…")
            .desired_width(ui.available_width()),
    );
    ui.horizontal_wrapped(|ui| {
        if theme::neon_btn(ui, "New person").clicked() {
            let n = next_no(files);
            files.push(Dossier::person(n));
            *index = files.len() - 1;
            query.clear();
            changed = true;
        }
        if theme::neon_btn(ui, "New company").clicked() {
            let n = next_no(files);
            files.push(Dossier::company(n));
            *index = files.len() - 1;
            query.clear();
            changed = true;
        }
    });
    let q = query.to_lowercase();
    let rows: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, d)| d.hit(&q))
        .map(|(i, _)| i)
        .collect();
    let h = (ui.available_height() - 4.0).max(40.0);
    egui::ScrollArea::vertical()
        .id_salt("recon-index")
        .max_height(h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for i in rows {
                let title = files[i].title();
                let sub = format!(
                    "{}  ·  {}  ·  {}  ·  {}",
                    files[i].file_no,
                    if files[i].is_company() { "COMPANY" } else { "PERSON" },
                    files[i].class,
                    files[i].status
                );
                if theme::wide_btn(ui, &title, &sub, *index == i).clicked() {
                    *index = i;
                }
            }
        });
    changed
}

fn paint_file(
    ui: &mut egui::Ui,
    root: &Path,
    files: &mut Vec<Dossier>,
    index: &mut usize,
    arm: &mut String,
    tex: &mut crate::images::TexCache,
    zoom: &mut Option<PathBuf>,
    blight: bool,
    share: &mut Option<Dossier>,
) -> bool {
    let i = *index;
    if i >= files.len() {
        return false;
    }
    let mut changed = false;
    let id = files[i].id.clone();
    let company = files[i].is_company();
    let file_no = files[i].file_no.clone();
    let class = files[i].class.clone();
    let status = files[i].status.clone();
    let portrait = files[i].portrait.clone();
    let subject = files[i].title();
    let company_id = files[i].company_id.clone();
    let filled = filled_lines(&files[i]);
    let money = if blight { "Eddies" } else { "Gold" };
    let company_line = if company {
        None
    } else {
        files
            .iter()
            .find(|d| d.id == company_id)
            .map(|d| format!("{}  {}", d.file_no, d.title()))
    };
    let edge = if class == "BURN" {
        KILL
    } else if class == "RESTRICTED" {
        theme::ACID
    } else {
        theme::fade(theme::HOT, 200)
    };
    let head = egui::Frame::NONE
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, edge))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let path = if portrait.is_empty() {
                        None
                    } else {
                        Some(root.join(&portrait))
                    };
                    if let Some(path) = path {
                        if path.is_file()
                            && crate::images::show_fit(ui, tex, &path, Vec2::new(96.0, 120.0)).clicked()
                        {
                            *zoom = Some(path);
                        }
                    } else {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(96.0, 120.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 0.0, Color32::from_rgb(8, 10, 12));
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.0, CYAN),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            if company { "MARK" } else { "FACE" },
                            egui::FontId::new(12.0, theme::mono()),
                            DIM,
                        );
                    }
                    let upload = if company { "Upload mark" } else { "Upload portrait" };
                    if theme::neon_btn(ui, upload).clicked() {
                        if let Some(src) = rfd::FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                            .set_title(upload)
                            .pick_file()
                        {
                            if let Ok(rel) = store_picture(root, &id, &src) {
                                files[i].portrait = rel;
                                changed = true;
                                arm.clear();
                            }
                        }
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 8.0).max(40.0));
                    ui.label(
                        RichText::new(&file_no)
                            .family(theme::mono())
                            .size(22.0)
                            .color(theme::ACID),
                    );
                    theme::kicker(
                        ui,
                        if company {
                            "FILE://COMPANY  ·  NOT ON THE WIRE"
                        } else {
                            "FILE://PERSON  ·  NOT ON THE WIRE"
                        },
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(&subject)
                                .family(theme::display())
                                .size(26.0)
                                .color(CREAM),
                        )
                        .wrap(),
                    );
                    if let Some(line) = &company_line {
                        ui.label(
                            RichText::new(format!("WITH  {line}"))
                                .family(theme::mono())
                                .size(12.0)
                                .color(CYAN),
                        );
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("CLASS")
                        .family(theme::mono())
                        .size(10.0)
                        .color(DIM),
                );
                for word in ["OPEN", "RESTRICTED", "BURN"] {
                    let on = files[i].class == word;
                    let col = if word == "BURN" { KILL } else { CYAN };
                    if theme::neon_btn_color(ui, word, col, on).clicked() && !on {
                        files[i].class = word.into();
                        changed = true;
                        arm.clear();
                    }
                }
                ui.label(
                    RichText::new("STATUS")
                        .family(theme::mono())
                        .size(10.0)
                        .color(DIM),
                );
                for word in ["ACTIVE", "WATCH", "CLOSED"] {
                    let on = files[i].status == word;
                    if theme::neon_btn_color(ui, word, CYAN, on).clicked() && !on {
                        files[i].status = word.into();
                        changed = true;
                        arm.clear();
                    }
                }
            });
            ui.label(
                RichText::new(format!(
                    "{filled} LINES  ·  {class}  ·  {status}  ·  LOCAL COPY"
                ))
                .family(theme::mono())
                .size(11.0)
                .color(DIM),
            );
        });
    let tick = head.response.rect;
    ui.painter().hline(
        (tick.left() + 10.0)..=(tick.left() + 48.0),
        tick.top() + 1.0,
        egui::Stroke::new(2.0, theme::ACID),
    );

    let sections: &[(&str, &[(&str, &str, bool)])] = if company { COMPANY } else { PERSON };
    for (n, (title, rows)) in sections.iter().enumerate() {
        ui.add_space(8.0);
        theme::pane().show(ui, |ui| {
            theme::section_head(ui, &format!("{:02}", n + 1), title);
            ui.add_space(4.0);
            let mut k = 0;
            while k < rows.len() {
                let pair = !rows[k].2
                    && k + 1 < rows.len()
                    && !rows[k + 1].2
                    && ui.available_width() > 520.0;
                if pair {
                    let a = rows[k];
                    let b = rows[k + 1];
                    let mut dirty = false;
                    ui.columns(2, |cols| {
                        if put_field(&mut cols[0], files, i, a.0, a.1, false, money) {
                            dirty = true;
                        }
                        if put_field(&mut cols[1], files, i, b.0, b.1, false, money) {
                            dirty = true;
                        }
                    });
                    if dirty {
                        changed = true;
                        arm.clear();
                    }
                    if a.0 == "trade" || b.0 == "trade" {
                        if trade_row(ui, files, i) {
                            changed = true;
                            arm.clear();
                        }
                    }
                    k += 2;
                } else {
                    let (key, label, tall) = rows[k];
                    if put_field(ui, files, i, key, label, tall, money) {
                        changed = true;
                        arm.clear();
                    }
                    if key == "trade" && trade_row(ui, files, i) {
                        changed = true;
                        arm.clear();
                    }
                    k += 1;
                }
            }
        });
    }

    if !company {
        ui.add_space(8.0);
        theme::pane().show(ui, |ui| {
            theme::section_head(ui, "LINK", "COMPANY");
            ui.add_space(4.0);
            let names: Vec<(String, String)> = files
                .iter()
                .filter(|d| d.is_company())
                .map(|d| (d.id.clone(), format!("{}  {}", d.file_no, d.title())))
                .collect();
            if theme::neon_btn_color(ui, "None", CYAN, company_id.is_empty()).clicked()
                && !company_id.is_empty()
            {
                files[i].company_id.clear();
                changed = true;
                arm.clear();
            }
            for (cid, name) in &names {
                let on = &company_id == cid;
                if theme::wide_btn(ui, name, if on { "LINKED" } else { "FILE" }, on).clicked() && !on
                {
                    files[i].company_id = cid.clone();
                    changed = true;
                    arm.clear();
                }
            }
            if names.is_empty() {
                ui.label(
                    RichText::new("No company file yet. New company is on the left.")
                        .color(DIM)
                        .size(12.0),
                );
            } else if !company_id.is_empty() && !names.iter().any(|(cid, _)| cid == &company_id) {
                ui.label(
                    RichText::new("The linked company file is gone.")
                        .color(KILL)
                        .size(12.0),
                );
            }
        });
    } else {
        ui.add_space(8.0);
        let mut jump = None;
        theme::pane().show(ui, |ui| {
            theme::section_head(ui, "ROLL", "PEOPLE ON FILE");
            ui.add_space(4.0);
            let people: Vec<(usize, String)> = files
                .iter()
                .enumerate()
                .filter(|(_, d)| !d.is_company() && d.company_id == id)
                .map(|(n, d)| (n, format!("{}  {}", d.file_no, d.title())))
                .collect();
            if people.is_empty() {
                ui.label(
                    RichText::new("No person file points here. Open a person and link this company.")
                        .color(DIM)
                        .size(12.0),
                );
            }
            for (n, name) in people {
                if theme::wide_btn(ui, &name, "OPEN FILE", false).clicked() {
                    jump = Some(n);
                }
            }
        });
        if let Some(n) = jump {
            *index = n;
        }
    }

    ui.add_space(12.0);
    if theme::neon_btn(ui, "Send").clicked() {
        if let Some(file) = files.get(i) {
            *share = Some(file.clone());
        }
    }
    ui.add_space(6.0);
    let armed = *arm == id;
    if theme::neon_btn_color(ui, if armed { "Delete this file" } else { "Delete" }, KILL, armed)
        .clicked()
    {
        if armed {
            if let Some(rel) = files.iter().find(|d| d.id == id).map(|d| d.portrait.clone()) {
                if rel.starts_with("data/recon/") {
                    let _ = std::fs::remove_file(root.join(rel));
                }
            }
            files.retain(|d| d.id != id);
            arm.clear();
            return true;
        }
        *arm = id.clone();
    }
    if *arm == id {
        ui.label(
            RichText::new("Press delete again. This removes the file from this computer.")
                .color(KILL)
                .size(12.0),
        );
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new(format!("END OF FILE  ·  {file_no}"))
            .family(theme::mono())
            .size(10.0)
            .color(DIM),
    );
    changed
}

fn filled_lines(d: &Dossier) -> usize {
    d.fields.values().filter(|s| !s.trim().is_empty()).count()
        + usize::from(!d.portrait.is_empty())
        + usize::from(!d.company_id.is_empty())
}

fn put_field(
    ui: &mut egui::Ui,
    files: &mut [Dossier],
    index: usize,
    key: &str,
    label: &str,
    tall: bool,
    money: &str,
) -> bool {
    let shown = if key == "money" {
        format!("{label} ({money})")
    } else {
        label.to_string()
    };
    ui.add_space(2.0);
    ui.label(
        RichText::new(shown)
            .family(theme::mono())
            .size(10.0)
            .color(CYAN),
    );
    let mut buf = files[index].fields.get(key).cloned().unwrap_or_default();
    let w = ui.available_width().clamp(1.0, 720.0);
    let hint = field_hint(key);
    let resp = if tall {
        let mut edit = egui::TextEdit::multiline(&mut buf)
            .desired_rows(3)
            .desired_width(w)
            .text_color(CREAM);
        if !hint.is_empty() {
            edit = edit.hint_text(hint);
        }
        ui.add(edit)
    } else {
        let mut edit = egui::TextEdit::singleline(&mut buf)
            .desired_width(w)
            .text_color(CREAM);
        if !hint.is_empty() {
            edit = edit.hint_text(hint);
        }
        ui.add(edit)
    };
    if !resp.changed() {
        return false;
    }
    if buf.trim().is_empty() {
        files[index].fields.remove(key);
    } else {
        files[index].fields.insert(key.to_string(), buf);
    }
    true
}

fn trade_row(ui: &mut egui::Ui, files: &mut [Dossier], index: usize) -> bool {
    let current = files[index]
        .fields
        .get("trade")
        .cloned()
        .unwrap_or_default();
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for word in [
            "security",
            "biotech",
            "finance",
            "media",
            "manufacture",
            "transport",
            "vice",
            "other",
        ] {
            let on = current.eq_ignore_ascii_case(word);
            if theme::neon_btn_color(ui, word, CYAN, on).clicked() && !on {
                picked = Some(word);
            }
        }
    });
    if let Some(word) = picked {
        files[index].fields.insert("trade".into(), word.into());
        true
    } else {
        false
    }
}

fn field_hint(key: &str) -> &'static str {
    match key {
        "name" => "Name they answer to",
        "aliases" => "Other names, separated",
        "handle" => "Net handle",
        "born" => "Year or date",
        "age" => "Years, or unknown",
        "gender" => "How they present",
        "ancestry" => "Body, line, or build-origin",
        "role" => "What they do",
        "employer" => "Who pays them",
        "gang" => "Crew, gang, or circle",
        "district" => "Last district",
        "street" => "Last street, or the company's street name",
        "address" => "Door, floor, or drop point",
        "comm" => "How to reach them",
        "drop" => "Where a message can wait",
        "height" => "Height",
        "build" => "Build",
        "hair" => "Hair",
        "eyes" => "Eyes",
        "marks" => "Scars, ink, tells",
        "chrome" => "Visible chrome, prosthetics, scars",
        "skills" => "What they can do",
        "weapons" => "What they carry",
        "tricks" => "Known tricks",
        "money" => "What they hold",
        "property" => "Rooms, vehicles, leases",
        "assets" => "Accounts, keys, gear",
        "associates" => "Who they stand with",
        "enemies" => "Who wants them hurt",
        "family" => "Blood or chosen",
        "history" => "Work, one line at a time",
        "vice" => "What they cannot leave",
        "tell" => "What gives them away",
        "want" => "What they are chasing",
        "turn" => "How to turn them",
        "threat" => "What they can do to this table",
        "source" => "Witness, rumor, file, or your own eyes",
        "notes" => "Anything that does not fit a line",
        "legal" => "Name on the papers",
        "ticker" => "Letters, if they trade",
        "trade" => "security, biotech, finance, media, manufacture, transport, vice",
        "hq" => "Where the door is",
        "districts" => "Where they work",
        "size" => "Shop, firm, or tower",
        "headcount" => "People on the books",
        "leadership" => "Who decides. Link those people from their own file.",
        "public" => "What the sign says",
        "actual" => "What they actually do",
        "products" => "What they sell",
        "subsidiaries" => "Names they hide behind",
        "rivals" => "Who they fight for the street",
        "security" => "Guards, locks, and who gets in",
        "trouble" => "Cases, fines, warrants",
        "talkers" => "Who inside will talk, and the price",
        "rumors" => "What the street says",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_climb_and_a_file_roundtrips() {
        let mut files = vec![Dossier::person(1)];
        files[0].fields.insert("name".into(), "Ada".into());
        files[0].fields.insert("handle".into(), "glass".into());
        assert!(files[0].hit("glass"));
        assert!(!files[0].hit("missing-file"));
        assert_eq!(next_no(&files), 2);
        files.push(Dossier::company(next_no(&files)));
        files[1].fields.insert("legal".into(), "Northline".into());
        assert_eq!(files[1].title(), "NORTHLINE");
        assert!(files[1].is_company());
        let dir = std::env::temp_dir().join(format!("blight-recon-{}", rand::random::<u32>()));
        let _ = std::fs::create_dir_all(dir.join("data"));
        save(&dir, &files);
        let back = load(&dir);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].fields.get("name").map(String::as_str), Some("Ada"));
        assert_eq!(back[0].class, "OPEN");
        assert_eq!(back[1].file_no, "R-0002");
        assert!(back[1].is_company());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
