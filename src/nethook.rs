use crate::theme::{self, CREAM, CYAN, ORANGE, PANEL};
use eframe::egui::{self, Color32, FontId, RichText, Vec2};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Nethook {
    pub id: String,
    pub title: String,
    pub owner_id: String,
    pub owner_name: String,
    pub html: String,
    #[serde(default)]
    pub updated: u64,
}

impl Nethook {
    pub fn fresh(owner_id: &str, owner_name: &str) -> Self {
        Self {
            id: format!("nh-{:08x}", rand::random::<u32>()),
            title: "New Nethook".into(),
            owner_id: owner_id.into(),
            owner_name: owner_name.into(),
            html: "<h1>New Nethook</h1>\n<p>Write in the grid. Headings, paragraphs, lists. INDEX chrome only.</p>\n<ul><li>Keep it simple.</li><li>No scripts.</li></ul>\n".into(),
            updated: now_unix(),
        }
    }

    pub fn touch(&mut self) {
        self.updated = now_unix();
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn dir(root: &Path) -> PathBuf {
    root.join("data/nethooks")
}

pub fn load_all(root: &Path) -> Vec<Nethook> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir(root)) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(h) = serde_json::from_str::<Nethook>(&s) {
                out.push(h);
            }
        }
    }
    out.sort_by(|a, b| b.updated.cmp(&a.updated));
    out
}

pub fn save_one(root: &Path, h: &Nethook) {
    let d = dir(root);
    let _ = std::fs::create_dir_all(&d);
    if let Ok(s) = serde_json::to_string_pretty(h) {
        let _ = std::fs::write(d.join(format!("{}.json", h.id)), s);
    }
}

pub fn delete_one(root: &Path, id: &str) {
    let _ = std::fs::remove_file(dir(root).join(format!("{id}.json")));
}

pub fn merge(list: &mut Vec<Nethook>, incoming: Nethook) {
    if let Some(i) = list.iter().position(|x| x.id == incoming.id) {
        if list[i].owner_id != incoming.owner_id {
            return;
        }
        if incoming.updated >= list[i].updated {
            list[i] = incoming;
        }
    } else {
        list.push(incoming);
    }
    list.sort_by(|a, b| b.updated.cmp(&a.updated));
    if list.len() > 80 {
        list.truncate(80);
    }
}

pub fn strip_danger(html: &str) -> String {
    let mut s = html.to_string();
    for tag in ["script", "style", "iframe", "object", "embed"] {
        loop {
            let low = s.to_lowercase();
            let Some(a) = low.find(&format!("<{tag}")) else {
                break;
            };
            let close = format!("</{tag}>");
            let rest = &low[a..];
            let end = rest
                .find(&close)
                .map(|i| a + i + close.len())
                .or_else(|| rest.find('>').map(|i| a + i + 1))
                .unwrap_or(s.len());
            s.replace_range(a..end.min(s.len()), "");
        }
    }
    s
}

fn skip_tag(s: &str, i: usize) -> usize {
    s[i..].find('>').map(|n| i + n + 1).unwrap_or(s.len())
}

fn tag_name(s: &str, i: usize) -> (String, bool, usize) {
    let mut j = i + 1;
    let close = s.as_bytes().get(j) == Some(&b'/');
    if close {
        j += 1;
    }
    let start = j;
    while j < s.len() && s.as_bytes()[j].is_ascii_alphanumeric() {
        j += 1;
    }
    (s[start..j].to_lowercase(), close, skip_tag(s, i))
}

pub fn paint(ui: &mut egui::Ui, html: &str) {
    let clean = strip_danger(html);
    let mut i = 0;
    let mut buf = String::new();
    let mut kind = String::from("p");
    let flush = |ui: &mut egui::Ui, buf: &mut String, kind: &str| {
        let t = buf.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.is_empty() {
            buf.clear();
            return;
        }
        match kind {
            "h1" => {
                ui.label(
                    RichText::new(t.to_uppercase())
                        .family(theme::display())
                        .size(26.0)
                        .color(ORANGE),
                );
            }
            "h2" => {
                ui.label(
                    RichText::new(&t)
                        .family(theme::display())
                        .size(20.0)
                        .color(CYAN),
                );
            }
            "h3" => {
                ui.label(
                    RichText::new(&t)
                        .family(theme::mono())
                        .size(14.0)
                        .color(ORANGE),
                );
            }
            "li" => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("▸").color(ORANGE).size(14.0));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&t)
                                .color(CREAM)
                                .size(15.0)
                                .family(theme::ui_font()),
                        )
                        .wrap(),
                    );
                });
            }
            _ => {
                ui.add(
                    egui::Label::new(
                        RichText::new(&t)
                            .color(CREAM)
                            .size(15.0)
                            .family(theme::ui_font()),
                    )
                    .wrap(),
                );
            }
        }
        buf.clear();
        ui.add_space(6.0);
    };
    while i < clean.len() {
        if clean.as_bytes()[i] == b'<' {
            flush(ui, &mut buf, &kind);
            let (name, closing, next) = tag_name(&clean, i);
            if name == "br" {
                ui.add_space(8.0);
            } else if name == "hr" {
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width().max(40.0), 2.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(r, 0.0, ORANGE);
                ui.add_space(6.0);
            } else if !closing {
                match name.as_str() {
                    "h1" | "h2" | "h3" | "li" | "p" => kind = name,
                    _ => {}
                }
            } else {
                kind = "p".into();
            }
            i = next;
            continue;
        }
        if clean[i..].starts_with("&amp;") {
            buf.push('&');
            i += 5;
            continue;
        }
        if clean[i..].starts_with("&lt;") {
            buf.push('<');
            i += 4;
            continue;
        }
        if clean[i..].starts_with("&gt;") {
            buf.push('>');
            i += 4;
            continue;
        }
        if clean[i..].starts_with("&nbsp;") {
            buf.push(' ');
            i += 6;
            continue;
        }
        let ch = clean[i..].chars().next().unwrap_or(' ');
        buf.push(ch);
        i += ch.len_utf8();
    }
    flush(ui, &mut buf, &kind);
}

pub fn chrome_frame(ui: &mut egui::Ui, title: &str, owner: &str, inner: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(PANEL)
        .stroke(egui::Stroke::new(2.0, ORANGE))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let (net, _) = ui.allocate_exact_size(Vec2::new(40.0, 20.0), egui::Sense::hover());
                theme::fill_chamfer(ui, net, 4.0, ORANGE, egui::Stroke::NONE);
                ui.painter().text(
                    net.center(),
                    egui::Align2::CENTER_CENTER,
                    "NH",
                    FontId::new(11.0, theme::mono()),
                    Color32::from_rgb(17, 17, 17),
                );
                ui.label(
                    RichText::new(title)
                        .family(theme::display())
                        .size(20.0)
                        .color(ORANGE),
                );
                ui.label(
                    RichText::new(format!("nethook://{owner}"))
                        .family(theme::mono())
                        .size(11.0)
                        .color(CYAN),
                );
            });
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), 2.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(r, 0.0, ORANGE);
            ui.add_space(8.0);
            inner(ui);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_kills_script() {
        let s = strip_danger("<p>hi</p><script>alert(1)</script><p>ok</p>");
        assert!(!s.to_lowercase().contains("script"));
        assert!(s.contains("hi"));
    }

    #[test]
    fn merge_keeps_newer() {
        let mut v = vec![Nethook {
            id: "a".into(),
            title: "old".into(),
            owner_id: "1".into(),
            owner_name: "Ada".into(),
            html: "x".into(),
            updated: 1,
        }];
        merge(
            &mut v,
            Nethook {
                id: "a".into(),
                title: "new".into(),
                owner_id: "1".into(),
                owner_name: "Ada".into(),
                html: "y".into(),
                updated: 2,
            },
        );
        assert_eq!(v[0].title, "new");
        merge(
            &mut v,
            Nethook {
                id: "a".into(),
                title: "stale".into(),
                owner_id: "1".into(),
                owner_name: "Ada".into(),
                html: "z".into(),
                updated: 0,
            },
        );
        assert_eq!(v[0].title, "new");
        merge(
            &mut v,
            Nethook {
                id: "a".into(),
                title: "stolen".into(),
                owner_id: "2".into(),
                owner_name: "Mallory".into(),
                html: "no".into(),
                updated: 99,
            },
        );
        assert_eq!(v[0].title, "new");
        assert_eq!(v[0].owner_id, "1");
    }
}
