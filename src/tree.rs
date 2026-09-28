//! One folder at a time. Nothing here is sent to the table.

use crate::theme::{self, CREAM, CYAN, DIM, KILL, MUTED};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub dir: bool,
    pub bytes: u64,
}


pub fn rename_entry(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    let name = safe_name(new_name).ok_or_else(|| "Use a name without a slash.".to_string())?;
    let parent = path.parent().ok_or_else(|| "That path has no folder.".to_string())?;
    let dest = parent.join(name);
    if dest.exists() {
        return Err("That name is already there.".into());
    }
    std::fs::rename(path, &dest).map_err(|_| "Rename failed.".to_string())?;
    Ok(dest)
}

pub fn copy_entry(path: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    if !dest_dir.is_dir() {
        return Err("Open a folder first.".into());
    }
    let name = path
        .file_name()
        .ok_or_else(|| "That has no name.".to_string())?;
    let mut dest = dest_dir.join(name);
    if dest.exists() {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("copy");
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let tagged = if ext.is_empty() {
            format!("{stem}-copy")
        } else {
            format!("{stem}-copy.{ext}")
        };
        dest = dest_dir.join(tagged);
    }
    if path.is_dir() {
        copy_dir_recursive(path, &dest)?;
    } else {
        std::fs::copy(path, &dest).map_err(|_| "Copy failed.".to_string())?;
    }
    Ok(dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|_| "Copy failed.".to_string())?;
    for ent in std::fs::read_dir(src).map_err(|_| "Copy failed.".to_string())? {
        let ent = ent.map_err(|_| "Copy failed.".to_string())?;
        let from = ent.path();
        let to = dest.join(ent.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|_| "Copy failed.".to_string())?;
        }
    }
    Ok(())
}

pub fn move_entry(path: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    if !dest_dir.is_dir() {
        return Err("Open a folder first.".into());
    }
    let name = path
        .file_name()
        .ok_or_else(|| "That has no name.".to_string())?;
    let dest = dest_dir.join(name);
    if dest.exists() {
        return Err("That name is already there.".into());
    }
    if path.is_dir() && dest_dir.starts_with(path) {
        return Err("Cannot move a folder into itself.".into());
    }
    std::fs::rename(path, &dest).map_err(|_| "Move failed.".to_string())?;
    Ok(dest)
}

pub fn mtime_label(path: &Path) -> String {
    let Ok(meta) = std::fs::metadata(path) else {
        return "—".into();
    };
    let Ok(modified) = meta.modified() else {
        return "—".into();
    };
    let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return "—".into();
    };
    let secs = dur.as_secs();
    let days = secs / 86400;
    let tod = secs % 86400;
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    format!("d{days} {hh:02}:{mm:02}")
}

pub struct Bookmark {
    pub label: &'static str,
    pub path: PathBuf,
}

pub fn bookmarks(root: &Path) -> Vec<Bookmark> {
    let mut out = vec![
        Bookmark { label: "This deck", path: root.to_path_buf() },
        Bookmark { label: "data", path: root.join("data") },
        Bookmark { label: "audio", path: root.join("audio") },
        Bookmark { label: "assets", path: root.join("assets") },
        Bookmark { label: "inbox", path: root.join("data/inbox") },
    ];
    out.retain(|b| b.path.is_dir() || b.label == "inbox");
    out
}

/// Session-only chrome for the Tree tab. Pins are not persisted.
#[derive(Default)]
pub struct Chrome {
    pub filter: String,
    pub pins: HashSet<PathBuf>,
    pub path_edit: String,
    pub rename_to: String,
    pub sort_size: bool,
    pub clip: Option<PathBuf>,
    pub console_open: bool,
}

impl Chrome {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            pins: HashSet::new(),
            path_edit: String::new(),
            rename_to: String::new(),
            sort_size: false,
            clip: None,
            console_open: false,
        }
    }
}

/// Local-only deck pulse for Tree chrome (meters + node drawer). Not mesh state.
#[derive(Clone, Default)]
pub struct DeckPulse {
    pub cpu: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub disk_free: u64,
    pub disk_total: u64,
    pub node_up: bool,
    pub role: String,
    pub peers: usize,
    pub invite: String,
    pub log_tail: String,
}

#[derive(Clone, Debug)]
pub enum Act {
    Home,
    Computer,
    Up,
    Refresh,
    NewFile,
    NewFolder,
    Open(PathBuf),
    Enter(PathBuf),
    Select(PathBuf),
    Delete,
    CopyPath(PathBuf),
    CopyName(PathBuf),
    TogglePin(PathBuf),
    Jump(PathBuf),
    Rename,
    CopyHere,
    MoveHere,
    OpenText(PathBuf),
    Bookmark(PathBuf),
    OpenTerminal,
    ToggleConsole,
    SendFile,
    ReconPortrait,
    ProbeTags,
}

pub fn home_dir() -> Option<PathBuf> {
    let raw = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let path = PathBuf::from(raw);
    path.is_dir().then_some(path)
}

pub fn computer_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut roots = Vec::new();
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", letter as char));
            if path.is_dir() {
                roots.push(path);
            }
        }
        if roots.is_empty() {
            roots.push(PathBuf::from("C:\\"));
        }
        return roots;
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/")]
    }
}

pub fn list_dir(path: &Path) -> Result<Vec<Entry>, String> {
    let rd = std::fs::read_dir(path).map_err(|_| "This folder did not open.".to_string())?;
    let mut rows = Vec::new();
    for ent in rd.flatten() {
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let meta = ent.metadata().ok();
        let dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let bytes = if dir { 0 } else { meta.map(|m| m.len()).unwrap_or(0) };
        rows.push(Entry {
            name,
            path,
            dir,
            bytes,
        });
    }
    rows.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(rows)
}

pub fn safe_name(name: &str) -> Option<&str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }
    Some(name)
}

pub fn create_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = safe_name(name).ok_or_else(|| "Use a name without a slash.".to_string())?;
    let path = dir.join(name);
    if path.exists() {
        return Err("That name is already there.".into());
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "The file was not created.".to_string())?;
    Ok(path)
}

pub fn create_dir(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = safe_name(name).ok_or_else(|| "Use a name without a slash.".to_string())?;
    let path = dir.join(name);
    if path.exists() {
        return Err("That name is already there.".into());
    }
    std::fs::create_dir(&path).map_err(|_| "The folder was not created.".to_string())?;
    Ok(path)
}

pub fn is_fs_root(path: &Path) -> bool {
    if path == Path::new("/") {
        return true;
    }
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    if bytes.len() == 2 && bytes[1] == b':' {
        return true;
    }
    if bytes.len() == 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        return true;
    }
    false
}

fn same_place(path: &Path, other: &Path) -> bool {
    match (path.canonicalize(), other.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => path == other,
    }
}

pub fn protected(path: &Path, app_root: &Path) -> bool {
    if is_fs_root(path) || same_place(path, app_root) {
        return true;
    }
    // A parent of this install would take the program with it.
    app_root.starts_with(path)
}

pub fn dir_has_child(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

pub fn remove_path(path: &Path, app_root: &Path) -> Result<(), String> {
    if protected(path, app_root) {
        return Err("That folder stays.".into());
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| "That is not there.".to_string())?;
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path).map_err(|_| "The file did not delete.".to_string())
    } else if meta.is_dir() {
        std::fs::remove_dir_all(path).map_err(|_| "The folder did not delete.".to_string())
    } else {
        Err("That is not there.".into())
    }
}

pub fn brief_bytes(n: u64) -> String {
    const G: f64 = 1024.0 * 1024.0 * 1024.0;
    const M: f64 = 1024.0 * 1024.0;
    if n as f64 >= G {
        format!("{:.1}G", n as f64 / G)
    } else if n as f64 >= M {
        format!("{:.0}M", n as f64 / M)
    } else if n >= 1024 {
        format!("{:.0}K", (n as f64 / 1024.0).max(0.0))
    } else {
        format!("{n} B")
    }
}

pub fn kind_tag(ent: &Entry) -> &'static str {
    if ent.dir {
        return "DIR";
    }
    let ext = ent
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" | "toml" | "json" | "md" | "txt" | "log" => "TXT",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => "IMG",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => "AUD",
        "mp4" | "webm" | "mkv" | "mov" => "VID",
        "pdf" => "PDF",
        "zip" | "gz" | "7z" | "tar" => "ARC",
        "exe" | "bin" | "so" | "dll" | "appimage" => "BIN",
        "" => "FILE",
        _ => "FILE",
    }
}

pub fn filter_rows<'a>(rows: &'a [Entry], filter: &str) -> Vec<&'a Entry> {
    let q = filter.trim().to_lowercase();
    if q.is_empty() {
        return rows.iter().collect();
    }
    rows.iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.path
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.to_lowercase().contains(&q))
                    .unwrap_or(false)
                || kind_tag(e).to_lowercase().contains(&q)
        })
        .collect()
}

fn crumbs(path: &Path, drives: bool) -> Vec<(String, PathBuf)> {
    if drives {
        return vec![("Computer".into(), PathBuf::new())];
    }
    let mut out = Vec::new();
    let mut acc = PathBuf::new();
    let text = path.to_string_lossy();
    if text.starts_with('/') {
        out.push(("/".into(), PathBuf::from("/")));
        acc = PathBuf::from("/");
    }
    for part in path.components() {
        match part {
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(p) => {
                let s = p.as_os_str().to_string_lossy().into_owned();
                acc = PathBuf::from(&s);
                if !s.ends_with('\\') && !s.ends_with('/') {
                    acc.push("");
                }
                out.push((s, acc.clone()));
            }
            std::path::Component::Normal(s) => {
                let label = s.to_string_lossy().into_owned();
                acc.push(s);
                out.push((label, acc.clone()));
            }
            _ => {}
        }
    }
    if out.is_empty() {
        out.push((path.display().to_string(), path.to_path_buf()));
    }
    out
}

fn row_meta(ent: &Entry) -> String {
    if ent.dir {
        "folder".into()
    } else {
        brief_bytes(ent.bytes)
    }
}


pub fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
    )
}

pub fn is_media_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            | "mp3" | "wav" | "flac" | "ogg" | "m4a"
            | "mp4" | "webm" | "mkv" | "mov"
    )
}

pub fn tail_daemon_log(root: &Path, max_lines: usize) -> String {
    let path = root.join("data/daemon.log");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return "No daemon.log yet.".into();
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return "daemon.log is empty.".into();
    }
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn paint_compact_meters(ui: &mut egui::Ui, pulse: &DeckPulse) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("DECK")
                .family(theme::mono())
                .size(10.0)
                .color(theme::ACID),
        );
        meter_chip(ui, "CPU", &format!("{:.0}%", pulse.cpu), pulse.cpu / 100.0);
        let ram_frac = if pulse.ram_total == 0 {
            0.0
        } else {
            pulse.ram_used as f32 / pulse.ram_total as f32
        };
        let ram = if pulse.ram_total == 0 {
            "—".into()
        } else {
            format!("{}/{}", brief_bytes(pulse.ram_used), brief_bytes(pulse.ram_total))
        };
        meter_chip(ui, "RAM", &ram, ram_frac);
        let disk_used_frac = if pulse.disk_total == 0 {
            0.0
        } else {
            1.0 - (pulse.disk_free as f32 / pulse.disk_total as f32)
        };
        let disk = if pulse.disk_total == 0 {
            "—".into()
        } else {
            format!("{} free", brief_bytes(pulse.disk_free))
        };
        meter_chip(ui, "DISK", &disk, disk_used_frac);
    });
}

fn meter_chip(ui: &mut egui::Ui, label: &str, value: &str, frac: f32) {
    let frac = frac.clamp(0.0, 1.0);
    let hot = frac >= 0.75;
    let bad = frac >= 0.90;
    let col = if bad {
        KILL
    } else if hot {
        theme::ACID
    } else {
        CYAN
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(108.0, 28.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        3.0,
        Color32::from_rgba_unmultiplied(8, 10, 14, 160),
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, theme::fade(col, 160)),
        egui::StrokeKind::Inside,
    );
    let bar = Rect::from_min_max(
        Pos2::new(rect.left() + 4.0, rect.bottom() - 6.0),
        Pos2::new(rect.right() - 4.0, rect.bottom() - 3.0),
    );
    ui.painter().rect_filled(bar, 1.0, theme::fade(DIM, 80));
    let fill_w = (bar.width() * frac).max(if frac > 0.0 { 2.0 } else { 0.0 });
    let fill = Rect::from_min_size(bar.min, Vec2::new(fill_w, bar.height()));
    ui.painter().rect_filled(fill, 1.0, col);
    ui.painter().text(
        Pos2::new(rect.left() + 6.0, rect.top() + 5.0),
        Align2::LEFT_TOP,
        label,
        FontId::new(9.0, theme::mono()),
        DIM,
    );
    ui.painter().text(
        Pos2::new(rect.right() - 6.0, rect.top() + 5.0),
        Align2::RIGHT_TOP,
        value,
        FontId::new(10.0, theme::mono()),
        col,
    );
}

fn paint_console_drawer(ui: &mut egui::Ui, pulse: &DeckPulse) {
    theme::kicker(ui, "NODE://LOCAL");
    ui.label(
        RichText::new("Local view only. Directory browse never leaves this deck.")
            .family(theme::ui_font())
            .size(11.0)
            .color(MUTED),
    );
    let status = if pulse.node_up { "UP" } else { "DOWN" };
    let status_col = if pulse.node_up { theme::ACID } else { KILL };
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("daemon {status}"))
                .family(theme::mono())
                .size(12.0)
                .color(status_col),
        );
        ui.label(
            RichText::new(format!("role {}", if pulse.role.is_empty() { "—" } else { &pulse.role }))
                .family(theme::mono())
                .size(12.0)
                .color(CYAN),
        );
        ui.label(
            RichText::new(format!("peers {}", pulse.peers))
                .family(theme::mono())
                .size(12.0)
                .color(MUTED),
        );
    });
    if !pulse.invite.is_empty() {
        ui.label(
            RichText::new("invite")
                .family(theme::mono())
                .size(10.0)
                .color(DIM),
        );
        ui.label(
            RichText::new(&pulse.invite)
                .family(theme::mono())
                .size(10.0)
                .color(CYAN),
        );
    }
    ui.label(
        RichText::new("data/daemon.log")
            .family(theme::mono())
            .size(10.0)
            .color(theme::ACID),
    );
    egui::ScrollArea::vertical()
        .id_salt("tree-daemon-log")
        .max_height(110.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(&pulse.log_tail)
                    .family(theme::mono())
                    .size(10.0)
                    .color(MUTED),
            );
        });
}

fn paint_tags_strip(ui: &mut egui::Ui, tags: &crate::deskfile::TagSet, note: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("TAGS")
                .family(theme::mono())
                .size(10.0)
                .color(theme::ACID),
        );
        let bits: Vec<String> = [
            ("title", tags.title.as_str()),
            ("artist", tags.artist.as_str()),
            ("album", tags.album.as_str()),
            ("year", tags.year.as_str()),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}:{v}"))
        .collect();
        let body = if bits.is_empty() {
            if tags.width > 0 {
                format!("{}×{}", tags.width, tags.height)
            } else if !note.is_empty() {
                note.to_string()
            } else {
                "no tags".into()
            }
        } else {
            let mut s = bits.join(" · ");
            if tags.width > 0 {
                s.push_str(&format!(" · {}×{}", tags.width, tags.height));
            }
            if !tags.comment.is_empty() {
                s.push_str(&format!(" · {}", tags.comment));
            }
            s
        };
        ui.label(
            RichText::new(body)
                .family(theme::mono())
                .size(11.0)
                .color(CYAN),
        );
    });
}

/// Paint the Tree browser. Returns actions for the host to apply.
pub fn paint(
    ui: &mut egui::Ui,
    chrome: &mut Chrome,
    root: &Path,
    at: &Path,
    drives: bool,
    rows: &[Entry],
    sel: Option<&Path>,
    arm: Option<&Path>,
    deep: bool,
    name: &mut String,
    err: &str,
    pulse: &DeckPulse,
    tags: Option<&crate::deskfile::TagSet>,
    tags_note: &str,
) -> Vec<Act> {
    let mut acts = Vec::new();
    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
    theme::page_chrome(
        ui,
        "TREE://LOCAL",
        "TREE",
        "Browse this deck · nothing is sent to the table",
    );

    // Bookmarks
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("MARK")
                .family(theme::mono())
                .size(10.0)
                .color(theme::ACID),
        );
        for b in bookmarks(root) {
            if theme::neon_btn(ui, b.label).clicked() {
                if b.label == "inbox" && !b.path.is_dir() {
                    let _ = std::fs::create_dir_all(&b.path);
                }
                acts.push(Act::Bookmark(b.path));
            }
        }
        if theme::neon_btn(ui, "Home").clicked() {
            acts.push(Act::Home);
        }
        if theme::neon_btn(ui, "Computer").clicked() {
            acts.push(Act::Computer);
        }
        if theme::neon_btn(ui, "Up").clicked() {
            acts.push(Act::Up);
        }
        if theme::neon_btn(ui, "Refresh").clicked() {
            acts.push(Act::Refresh);
        }
    });

    // Deferred utilities: terminal / node / wire send / recon portrait
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("UTIL")
                .family(theme::mono())
                .size(10.0)
                .color(theme::ACID),
        );
        if theme::neon_btn(ui, "Terminal").clicked() {
            acts.push(Act::OpenTerminal);
        }
        let node_l = if chrome.console_open { "Node ▾" } else { "Node" };
        if theme::neon_btn_color(ui, node_l, CYAN, chrome.console_open).clicked() {
            acts.push(Act::ToggleConsole);
        }
        let can_send = sel.map(|p| p.is_file()).unwrap_or(false);
        if can_send && theme::neon_btn(ui, "Send").clicked() {
            acts.push(Act::SendFile);
        }
        let can_portrait = sel.map(|p| p.is_file() && is_image_path(p)).unwrap_or(false);
        if can_portrait && theme::neon_btn(ui, "Portrait").clicked() {
            acts.push(Act::ReconPortrait);
        }
        if sel.map(|p| p.is_file() && is_media_path(p)).unwrap_or(false)
            && theme::neon_btn(ui, "Tags").clicked()
        {
            acts.push(Act::ProbeTags);
        }
    });

    paint_compact_meters(ui, pulse);

    if chrome.console_open {
        paint_console_drawer(ui, pulse);
    }

    // Breadcrumbs
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("PATH")
                .family(theme::mono())
                .size(10.0)
                .color(DIM),
        );
        for (i, (label, path)) in crumbs(at, drives).into_iter().enumerate() {
            if i > 0 {
                ui.label(
                    RichText::new("/")
                        .family(theme::mono())
                        .size(11.0)
                        .color(DIM),
                );
            }
            let short = if label.chars().count() > 18 {
                format!("{}…", label.chars().take(16).collect::<String>())
            } else {
                label
            };
            let on = !drives && path == at;
            let resp = ui.add(
                egui::Label::new(
                    RichText::new(short)
                        .family(theme::mono())
                        .size(12.0)
                        .color(if on { theme::ACID } else { CYAN }),
                )
                .sense(Sense::click()),
            );
            if resp.clicked() {
                if drives || path.as_os_str().is_empty() {
                    acts.push(Act::Computer);
                } else {
                    acts.push(Act::Jump(path));
                }
            }
        }
    });

    // Editable path field
    if chrome.path_edit.is_empty() || !drives {
        let shown = if drives {
            "Computer".to_string()
        } else {
            at.display().to_string()
        };
        if chrome.path_edit != shown && !ui.memory(|m| m.has_focus(egui::Id::new("tree-path"))) {
            chrome.path_edit = shown;
        }
    }
    ui.horizontal(|ui| {
        let te = ui.add(
            egui::TextEdit::singleline(&mut chrome.path_edit)
                .id(egui::Id::new("tree-path"))
                .hint_text("Absolute path…")
                .desired_width(ui.available_width().min(420.0).max(180.0)),
        );
        if (te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || theme::neon_btn(ui, "Go").clicked()
        {
            let p = PathBuf::from(chrome.path_edit.trim());
            if p.is_dir() {
                acts.push(Act::Jump(p));
            } else if p.is_file() {
                if let Some(parent) = p.parent() {
                    acts.push(Act::Jump(parent.to_path_buf()));
                    acts.push(Act::Select(p));
                }
            } else {
                // host will surface via err if jump fails — still attempt
                acts.push(Act::Jump(p));
            }
        }
    });

    if !err.is_empty() {
        ui.label(
            RichText::new(err)
                .family(theme::ui_font())
                .size(12.0)
                .color(CYAN),
        );
    }

    // Filter + create + sort
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut chrome.filter)
                .hint_text("Filter name / kind…")
                .desired_width(150.0),
        );
        ui.add(
            egui::TextEdit::singleline(name)
                .hint_text("New name")
                .desired_width(120.0),
        );
        if theme::neon_btn(ui, "New file").clicked() {
            acts.push(Act::NewFile);
        }
        if theme::neon_btn(ui, "New folder").clicked() {
            acts.push(Act::NewFolder);
        }
        let sort_l = if chrome.sort_size { "Sort: size" } else { "Sort: name" };
        if theme::neon_btn(ui, sort_l).clicked() {
            chrome.sort_size = !chrome.sort_size;
        }
    });

    // Selection actions
    ui.horizontal_wrapped(|ui| {
        let has = sel.is_some();
        if theme::neon_btn(ui, "Open").clicked() {
            if let Some(p) = sel {
                acts.push(Act::Open(p.to_path_buf()));
            }
        }
        if has {
            if let Some(p) = sel {
                if crate::deskfile::is_text_name(p) && theme::neon_btn(ui, "Edit text").clicked() {
                    acts.push(Act::OpenText(p.to_path_buf()));
                }
            }
            if theme::neon_btn(ui, "Copy path").clicked() {
                if let Some(p) = sel {
                    acts.push(Act::CopyPath(p.to_path_buf()));
                }
            }
            if theme::neon_btn(ui, "Copy name").clicked() {
                if let Some(p) = sel {
                    acts.push(Act::CopyName(p.to_path_buf()));
                }
            }
            let pinned = sel.map(|p| chrome.pins.contains(p)).unwrap_or(false);
            let pin_l = if pinned { "Unpin" } else { "Pin" };
            if theme::neon_btn(ui, pin_l).clicked() {
                if let Some(p) = sel {
                    acts.push(Act::TogglePin(p.to_path_buf()));
                }
            }
            if theme::neon_btn(ui, "Clip").clicked() {
                chrome.clip = sel.map(|p| p.to_path_buf());
            }
            if chrome.clip.is_some() && theme::neon_btn(ui, "Paste copy").clicked() {
                acts.push(Act::CopyHere);
            }
            if chrome.clip.is_some() && theme::neon_btn(ui, "Paste move").clicked() {
                acts.push(Act::MoveHere);
            }
        }
        ui.add(
            egui::TextEdit::singleline(&mut chrome.rename_to)
                .hint_text("Rename to…")
                .desired_width(120.0),
        );
        if has && theme::neon_btn(ui, "Rename").clicked() {
            acts.push(Act::Rename);
        }
    });

    // Protected legend before Delete
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("Protected: install root, FS root, and parents of this deck stay.")
                .family(theme::mono())
                .size(10.0)
                .color(DIM),
        );
        let armed = arm.is_some() && arm == sel;
        let label = if armed && deep {
            "Delete all?"
        } else if armed {
            "Delete?"
        } else {
            "Delete"
        };
        if theme::neon_btn_color(ui, label, KILL, armed).clicked() {
            acts.push(Act::Delete);
        }
    });

    if let Some(t) = tags {
        paint_tags_strip(ui, t, tags_note);
    } else if !tags_note.is_empty() {
        ui.label(
            RichText::new(tags_note)
                .family(theme::mono())
                .size(11.0)
                .color(DIM),
        );
    }

    // Pins strip
    if !chrome.pins.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("PIN")
                    .family(theme::mono())
                    .size(10.0)
                    .color(theme::ACID),
            );
            let pins: Vec<PathBuf> = chrome.pins.iter().cloned().collect();
            for p in pins {
                let label = p
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.display().to_string());
                let short = if label.chars().count() > 16 {
                    format!("{}…", label.chars().take(14).collect::<String>())
                } else {
                    label
                };
                if theme::neon_btn(ui, &short).clicked() {
                    if p.is_dir() {
                        acts.push(Act::Enter(p));
                    } else if let Some(parent) = p.parent() {
                        acts.push(Act::Jump(parent.to_path_buf()));
                        acts.push(Act::Select(p));
                    }
                }
            }
        });
    }

    let mut filtered = filter_rows(rows, &chrome.filter);
    if chrome.sort_size {
        filtered.sort_by(|a, b| {
            b.dir
                .cmp(&a.dir)
                .then_with(|| b.bytes.cmp(&a.bytes))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }
    let n = filtered.len();
    let total = rows.len();
    ui.label(
        RichText::new(format!(
            "{} shown · {} in folder{}",
            n,
            total,
            if chrome.filter.trim().is_empty() {
                ""
            } else {
                " · filtered"
            }
        ))
        .family(theme::mono())
        .size(10.0)
        .color(DIM),
    );

    // Dual pane: list + preview
    let full = ui.available_rect_before_wrap();
    let preview_w = (full.width() * 0.32).clamp(160.0, 280.0);
    let (list_rect, preview_rect) = full.split_left_right_at_x(full.right() - preview_w);

    let mut list_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(list_rect.shrink2(Vec2::new(2.0, 0.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    list_ui.set_clip_rect(list_rect);

    // Column header
    {
        let w = list_ui.available_width().max(40.0);
        let (hdr, _) = list_ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::hover());
        list_ui.painter().rect_filled(
            hdr,
            0.0,
            Color32::from_rgba_unmultiplied(77, 232, 255, 10),
        );
        let mono = FontId::new(10.0, theme::mono());
        list_ui.painter().text(
            Pos2::new(hdr.left() + 44.0, hdr.center().y),
            Align2::LEFT_CENTER,
            "NAME",
            mono.clone(),
            DIM,
        );
        list_ui.painter().text(
            Pos2::new(hdr.right() - 150.0, hdr.center().y),
            Align2::LEFT_CENTER,
            "TYPE",
            mono.clone(),
            DIM,
        );
        list_ui.painter().text(
            Pos2::new(hdr.right() - 100.0, hdr.center().y),
            Align2::LEFT_CENTER,
            "SIZE",
            mono.clone(),
            DIM,
        );
        list_ui.painter().text(
            Pos2::new(hdr.right() - 10.0, hdr.center().y),
            Align2::RIGHT_CENTER,
            "MTIME",
            mono,
            DIM,
        );
    }

    let list_h = list_ui.available_height().max(40.0);
    let mut open_dir = None;
    let row_h = 26.0_f32;
    let depth_base = if drives {
        0
    } else {
        at.components().count().saturating_sub(1)
    };

    egui::ScrollArea::vertical()
        .id_salt("tree-rows")
        .max_height(list_h)
        .auto_shrink([false, false])
        .show_rows(&mut list_ui, row_h, n, |ui, range| {
            for i in range {
                let Some(ent) = filtered.get(i).copied() else {
                    continue;
                };
                let on = sel == Some(ent.path.as_path());
                let pinned = chrome.pins.contains(&ent.path);
                let tag = kind_tag(ent);
                let meta = row_meta(ent);
                let mtime = mtime_label(&ent.path);
                let indent = if drives {
                    0.0
                } else {
                    ((depth_base.min(6)) as f32) * 6.0
                };

                let w = ui.available_width().max(40.0);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::click());
                let hover = resp.hovered();
                let ch = theme::chrome();
                let edge = if on || hover {
                    ch.acid
                } else {
                    theme::fade(ch.hot, 120)
                };
                let fill = if on {
                    Color32::from_rgba_unmultiplied(214, 255, 63, 18)
                } else if hover {
                    Color32::from_rgba_unmultiplied(77, 232, 255, 12)
                } else {
                    Color32::from_rgba_unmultiplied(8, 10, 14, 90)
                };
                ui.painter().rect_filled(rect, 2.0, fill);
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(if on { 1.2 } else { 0.8 }, edge),
                    egui::StrokeKind::Inside,
                );

                let guide_x0 = rect.left() + 6.0;
                for g in 0..(depth_base.min(6)) {
                    let gx = guide_x0 + g as f32 * 6.0;
                    ui.painter().vline(
                        gx,
                        rect.y_range(),
                        Stroke::new(1.0, theme::fade(CYAN, 28)),
                    );
                }

                let tag_col = match tag {
                    "DIR" => CYAN,
                    "BIN" => theme::ORANGE,
                    "IMG" | "VID" | "AUD" => theme::ACID,
                    _ => MUTED,
                };
                ui.painter().text(
                    Pos2::new(rect.left() + 10.0 + indent, rect.center().y),
                    Align2::LEFT_CENTER,
                    tag,
                    FontId::new(10.0, theme::mono()),
                    tag_col,
                );

                let name_x = rect.left() + 42.0 + indent;
                let name_col = if on || hover { ch.acid } else { CREAM };
                let pin_mark = if pinned { "★ " } else { "" };
                let shown = format!("{pin_mark}{}", ent.name);
                ui.painter()
                    .with_clip_rect(Rect::from_min_max(
                        Pos2::new(name_x, rect.top()),
                        Pos2::new((rect.right() - 160.0).max(name_x + 40.0), rect.bottom()),
                    ))
                    .text(
                        Pos2::new(name_x, rect.center().y),
                        Align2::LEFT_CENTER,
                        shown,
                        FontId::new(13.0, theme::ui_font()),
                        name_col,
                    );

                ui.painter().text(
                    Pos2::new(rect.right() - 150.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    tag,
                    FontId::new(10.0, theme::mono()),
                    DIM,
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 100.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    meta,
                    FontId::new(10.0, theme::mono()),
                    DIM,
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 10.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    mtime,
                    FontId::new(10.0, theme::mono()),
                    DIM,
                );

                if resp.double_clicked() {
                    if ent.dir {
                        open_dir = Some(ent.path.clone());
                    } else {
                        acts.push(Act::Open(ent.path.clone()));
                    }
                } else if resp.clicked() {
                    if ent.dir {
                        // single click selects dirs too for rename/copy; double/open enters
                        acts.push(Act::Select(ent.path.clone()));
                    } else {
                        acts.push(Act::Select(ent.path.clone()));
                    }
                }
            }
        });

    if let Some(path) = open_dir {
        acts.push(Act::Enter(path));
    }

    // Preview pane
    let mut prev_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(preview_rect.shrink2(Vec2::new(6.0, 2.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    prev_ui.set_clip_rect(preview_rect);
    theme::holo_frame(&prev_ui, preview_rect.shrink2(Vec2::new(2.0, 2.0)), 8.0);
    prev_ui.add_space(8.0);
    prev_ui.label(
        RichText::new("PREVIEW")
            .family(theme::display())
            .size(14.0)
            .color(theme::ACID),
    );
    if let Some(p) = sel {
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.display().to_string());
        prev_ui.label(
            RichText::new(&name)
                .family(theme::ui_font())
                .size(14.0)
                .color(CREAM),
        );
        prev_ui.label(
            RichText::new(p.display().to_string())
                .family(theme::mono())
                .size(10.0)
                .color(DIM),
        );
        let is_dir = p.is_dir();
        let tag = if is_dir {
            "DIR"
        } else {
            kind_tag(&Entry {
                name: name.clone(),
                path: p.to_path_buf(),
                dir: false,
                bytes: std::fs::metadata(p).map(|m| m.len()).unwrap_or(0),
            })
        };
        let bytes = if is_dir {
            "—".into()
        } else {
            brief_bytes(std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        };
        prev_ui.label(
            RichText::new(format!(
                "{tag}  ·  {bytes}  ·  {}",
                mtime_label(p)
            ))
            .family(theme::mono())
            .size(11.0)
            .color(CYAN),
        );
        if protected(p, root) {
            prev_ui.label(
                RichText::new("PROTECTED")
                    .family(theme::mono())
                    .size(11.0)
                    .color(KILL),
            );
        }
        if crate::deskfile::is_text_name(p) {
            match crate::deskfile::read_text(p) {
                Ok(body) => {
                    let clip: String = body.chars().take(600).collect();
                    egui::ScrollArea::vertical()
                        .id_salt("tree-preview")
                        .max_height(prev_ui.available_height().max(40.0))
                        .show(&mut prev_ui, |ui| {
                            ui.label(
                                RichText::new(clip)
                                    .family(theme::mono())
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        });
                }
                Err(e) => {
                    prev_ui.label(
                        RichText::new(e)
                            .family(theme::mono())
                            .size(11.0)
                            .color(DIM),
                    );
                }
            }
        } else if is_dir {
            prev_ui.label(
                RichText::new("Folder — Open / Enter to browse.")
                    .family(theme::ui_font())
                    .size(12.0)
                    .color(MUTED),
            );
            if theme::neon_btn(&mut prev_ui, "Enter").clicked() {
                acts.push(Act::Enter(p.to_path_buf()));
            }
        } else {
            prev_ui.label(
                RichText::new("Binary / media — Open on this computer.")
                    .family(theme::ui_font())
                    .size(12.0)
                    .color(MUTED),
            );
        }
    } else {
        prev_ui.label(
            RichText::new("Select a row.")
                .family(theme::ui_font())
                .size(12.0)
                .color(DIM),
        );
    }

    // Consume layout space so parent doesn't overlap
    ui.allocate_rect(full, Sense::hover());
    acts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_reject_a_slash_and_roots_stay() {
        assert!(safe_name("note").is_some());
        assert!(safe_name("a/b").is_none());
        assert!(safe_name("..").is_none());
        assert!(safe_name("  ").is_none());
        assert!(is_fs_root(Path::new("/")));
        assert!(is_fs_root(Path::new("C:\\")));
        assert!(!is_fs_root(Path::new("/home")));
    }

    #[test]
    fn create_list_and_delete_keep_the_app_folder() {
        let dir = std::env::temp_dir().join(format!("bn-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(protected(&dir, &dir));
        assert!(remove_path(&dir, &dir).is_err());
        let file = create_file(&dir, "note.txt").unwrap();
        assert!(file.is_file());
        assert!(create_file(&dir, "note.txt").is_err());
        let listed = list_dir(&dir).unwrap();
        assert!(listed.iter().any(|e| e.name == "note.txt" && !e.dir));
        let sub = create_dir(&dir, "box").unwrap();
        create_file(&sub, "inside.txt").unwrap();
        assert!(dir_has_child(&sub));
        remove_path(&file, &dir).unwrap();
        assert!(!file.exists());
        remove_path(&sub, &dir).unwrap();
        assert!(!sub.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_parent_of_the_install_stays() {
        let parent = std::env::temp_dir().join(format!("bn-guard-{}", std::process::id()));
        let app = parent.join("blightnet");
        let _ = std::fs::remove_dir_all(&parent);
        std::fs::create_dir_all(&app).unwrap();
        assert!(protected(&parent, &app));
        assert!(protected(&app, &app));
        assert!(remove_path(&parent, &app).is_err());
        assert!(parent.is_dir());
        assert!(app.is_dir());
        let file = create_file(&app, "note.txt").unwrap();
        assert!(!protected(&file, &app));
        remove_path(&file, &app).unwrap();
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn filter_and_kind_tags() {
        let rows = vec![
            Entry {
                name: "notes.txt".into(),
                path: PathBuf::from("/tmp/notes.txt"),
                dir: false,
                bytes: 12,
            },
            Entry {
                name: "assets".into(),
                path: PathBuf::from("/tmp/assets"),
                dir: true,
                bytes: 0,
            },
        ];
        assert_eq!(kind_tag(&rows[0]), "TXT");
        assert_eq!(kind_tag(&rows[1]), "DIR");
        assert_eq!(filter_rows(&rows, "txt").len(), 1);
        assert_eq!(filter_rows(&rows, "DIR").len(), 1);
        assert_eq!(brief_bytes(500), "500 B");
        assert!(brief_bytes(4096).contains('K'));
    }

    #[test]
    fn image_and_media_helpers() {
        assert!(is_image_path(Path::new("shot.png")));
        assert!(is_image_path(Path::new("face.JPEG")));
        assert!(!is_image_path(Path::new("song.mp3")));
        assert!(is_media_path(Path::new("song.mp3")));
        assert!(is_media_path(Path::new("clip.mp4")));
        assert!(!is_media_path(Path::new("note.txt")));
    }
}
