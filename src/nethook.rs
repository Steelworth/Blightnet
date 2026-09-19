use crate::theme::{self, CREAM, CYAN, ORANGE, PANEL};
use eframe::egui::{self, Color32, FontId, Rect, RichText, Vec2};
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

    pub fn pinned(&self) -> bool {
        self.id == MANIFESTO_ID
    }
}

pub const MANIFESTO_ID: &str = "nh-manifesto";

pub fn manifesto() -> Nethook {
    Nethook {
        id: MANIFESTO_ID.into(),
        title: "A Cypherpunk's Manifesto".into(),
        owner_id: "grid".into(),
        owner_name: "Eric Hughes".into(),
        html: MANIFESTO_HTML.into(),
        updated: u64::MAX,
    }
}

pub fn ensure_pinned(list: &mut Vec<Nethook>) {
    list.retain(|h| h.id != MANIFESTO_ID);
    list.insert(0, manifesto());
}

const MANIFESTO_HTML: &str = r#"<h1>A Cypherpunk's Manifesto</h1>
<p>Eric Hughes · 9 March 1993</p>
<hr>
<p>Privacy is necessary for an open society in the electronic age. Privacy is not secrecy. A private matter is something one doesn't want the whole world to know, but a secret matter is something one doesn't want anybody to know. Privacy is the power to selectively reveal oneself to the world.</p>
<p>If two parties have some sort of dealings, then each has a memory of their interaction. Each party can speak about their own memory of this; how could anyone prevent it? One could pass laws against it, but the freedom of speech, even more than privacy, is fundamental to an open society; we seek not to restrict any speech at all. If many parties speak together in the same forum, each can speak to all the others and aggregate together knowledge about individuals and other parties. The power of electronic communications has enabled such group speech, and it will not go away merely because we might want it to.</p>
<p>Since we desire privacy, we must ensure that each party to a transaction have knowledge only of that which is directly necessary for that transaction. Since any information can be spoken of, we must ensure that we reveal as little as possible. In most cases personal identity is not salient. When I purchase a magazine at a store and hand cash to the clerk, there is no need to know who I am. When I ask my electronic mail provider to send and receive messages, my provider need not know to whom I am speaking or what I am saying or what others are saying to me; my provider only need know how to get the message there and how much I owe them in fees. When my identity is revealed by the underlying mechanism of the transaction, I have no privacy. I cannot here selectively reveal myself; I must always reveal myself.</p>
<p>Therefore, privacy in an open society requires anonymous transaction systems. Until now, cash has been the primary such system. An anonymous transaction system is not a secret transaction system. An anonymous system empowers individuals to reveal their identity when desired and only when desired; this is the essence of privacy.</p>
<p>Privacy in an open society also requires cryptography. If I say something, I want it heard only by those for whom I intend it. If the content of my speech is available to the world, I have no privacy. To encrypt is to indicate the desire for privacy, and to encrypt with weak cryptography is to indicate not too much desire for privacy. Furthermore, to reveal one's identity with assurance when the default is anonymity requires the cryptographic signature.</p>
<p>We cannot expect governments, corporations, or other large, faceless organizations to grant us privacy out of their beneficence. It is to their advantage to speak of us, and we should expect that they will speak. To try to prevent their speech is to fight against the realities of information. Information does not just want to be free, it longs to be free. Information expands to fill the available storage space. Information is Rumor's younger, stronger cousin; Information is fleeter of foot, has more eyes, knows more, and understands less than Rumor.</p>
<p>We must defend our own privacy if we expect to have any. We must come together and create systems which allow anonymous transactions to take place. People have been defending their own privacy for centuries with whispers, darkness, envelopes, closed doors, secret handshakes, and couriers. The technologies of the past did not allow for strong privacy, but electronic technologies do.</p>
<p>We the Cypherpunks are dedicated to building anonymous systems. We are defending our privacy with cryptography, with anonymous mail forwarding systems, with digital signatures, and with electronic money.</p>
<p>Cypherpunks write code. We know that someone has to write software to defend privacy, and since we can't get privacy unless we all do, we're going to write it. We publish our code so that our fellow Cypherpunks may practice and play with it. Our code is free for all to use, worldwide. We don't much care if you don't approve of the software we write. We know that software can't be destroyed and that a widely dispersed system can't be shut down.</p>
<p>Cypherpunks deplore regulations on cryptography, for encryption is fundamentally a private act. The act of encryption, in fact, removes information from the public realm. Even laws against cryptography reach only so far as a nation's border and the arm of its violence. Cryptography will ineluctably spread over the whole globe, and with it the anonymous transactions systems that it makes possible.</p>
<p>For privacy to be widespread it must be part of a social contract. People must come and together deploy these systems for the common good. Privacy only extends so far as the cooperation of one's fellows in society. We the Cypherpunks seek your questions and your concerns and hope we may engage you so that we do not deceive ourselves. We will not, however, be moved out of our course because some may disagree with our goals.</p>
<p>The Cypherpunks are actively engaged in making the networks safer for privacy. Let us proceed together apace.</p>
<p>Onward.</p>
<p>Eric Hughes</p>
<p>9 March 1993</p>
"#;

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
    if let Ok(rd) = std::fs::read_dir(dir(root)) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(h) = serde_json::from_str::<Nethook>(&s) {
                    if h.id != MANIFESTO_ID {
                        out.push(h);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated.cmp(&a.updated));
    ensure_pinned(&mut out);
    out
}

pub fn save_one(root: &Path, h: &Nethook) {
    if h.pinned() {
        return;
    }
    let d = dir(root);
    let _ = std::fs::create_dir_all(&d);
    if let Ok(s) = serde_json::to_string_pretty(h) {
        let _ = std::fs::write(d.join(format!("{}.json", h.id)), s);
    }
}

pub fn delete_one(root: &Path, id: &str) {
    if id == MANIFESTO_ID {
        return;
    }
    let _ = std::fs::remove_file(dir(root).join(format!("{id}.json")));
}

pub fn merge(list: &mut Vec<Nethook>, incoming: Nethook) {
    if incoming.id == MANIFESTO_ID {
        return;
    }
    if let Some(i) = list.iter().position(|x| x.id == incoming.id) {
        if list[i].pinned() || list[i].owner_id != incoming.owner_id {
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
    let shown = egui::Frame::NONE
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
            ui.painter().rect_filled(
                Rect::from_min_size(r.left_top(), Vec2::new((r.width() * 0.2).max(12.0), 2.0)),
                0.0,
                CYAN,
            );
            ui.add_space(8.0);
            inner(ui);
        });
    theme::brackets(ui, shown.response.rect.shrink(4.0), CYAN, 10.0);
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

    #[test]
    fn manifesto_is_first_and_pinned() {
        let m = manifesto();
        assert_eq!(m.id, MANIFESTO_ID);
        assert!(m.pinned());
        assert!(m.html.to_lowercase().contains("cypherpunks write code"));
        let mut v = vec![];
        ensure_pinned(&mut v);
        ensure_pinned(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "A Cypherpunk's Manifesto");
        merge(
            &mut v,
            Nethook {
                id: MANIFESTO_ID.into(),
                title: "stolen".into(),
                owner_id: "x".into(),
                owner_name: "x".into(),
                html: "no".into(),
                updated: 99,
            },
        );
        assert_eq!(v[0].title, "A Cypherpunk's Manifesto");
        delete_one(Path::new("/tmp"), MANIFESTO_ID);
        ensure_pinned(&mut v);
        assert_eq!(v[0].id, MANIFESTO_ID);
    }
}
