use crate::theme::{self, CREAM, CYAN, DIM, PANEL};
use eframe::egui::{self, Color32, FontId, Rect, RichText, Vec2};
use std::collections::HashMap;
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
    /// handout, rumor, job, or lesson. Older saves read as handout.
    #[serde(default = "kind_handout")]
    pub kind: String,
    /// The one page shown on the table.
    #[serde(default)]
    pub posted: bool,
    /// Attached pictures, sound, and video. Bytes live on disk, not in this JSON.
    #[serde(default)]
    pub files: Vec<HookFile>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct HookFile {
    pub name: String,
    pub kind: String,
}

fn kind_handout() -> String {
    "handout".into()
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
            kind: "handout".into(),
            posted: false,
            files: Vec::new(),
        }
    }

    pub fn starter(kind: &str, owner_id: &str, owner_name: &str, date: &str, place: &str, scene: &str) -> Self {
        let (title, html) = match kind {
            "rumor" => (
                "Rumor",
                "<style>h1{color:#d6ff3f;font-size:26px}p{color:#d6e2e8;font-size:16px}</style>\n<h1>Rumor</h1>\n<p>Someone is talking. Write what they said.</p>\n".into(),
            ),
            "job" => (
                "Job",
                "<style>h1{color:#d6ff3f;font-size:26px}p{color:#d6e2e8;font-size:16px}li{color:#4de8ff}</style>\n<h1>Job</h1>\n<p>Who pays, and for what.</p>\n<ul><li>The work</li><li>The pay</li></ul>\n".into(),
            ),
            "recap" => (
                "Session",
                format!(
                    "<style>h1{{color:#d6ff3f;font-size:26px}}p{{color:#d6e2e8;font-size:16px}}</style>\n<h1>Session</h1>\n<p>{date}</p>\n<p>{place}</p>\n<p>{scene}</p>\n"
                ),
            ),
            _ => (
                "Handout",
                "<style>h1{color:#d6ff3f;font-size:26px}p{color:#d6e2e8;font-size:16px}</style>\n<h1>Handout</h1>\n<p>What everyone at the table should read.</p>\n".into(),
            ),
        };
        Self {
            id: format!("nh-{:08x}", rand::random::<u32>()),
            title: title.into(),
            owner_id: owner_id.into(),
            owner_name: owner_name.into(),
            html,
            updated: now_unix(),
            kind: if kind == "recap" { "handout".into() } else { kind.into() },
            posted: false,
            files: Vec::new(),
        }
    }

    pub fn from_text(kind: &str, title: &str, body: &str, owner_id: &str, owner_name: &str) -> Self {
        let mut html = String::from("<style>h1{color:#d6ff3f;font-size:26px}p{color:#d6e2e8;font-size:16px}</style>\n");
        html.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            html.push_str(&format!("<p>{}</p>\n", escape_html(line)));
        }
        Self {
            id: format!("nh-{:08x}", rand::random::<u32>()),
            title: title.into(),
            owner_id: owner_id.into(),
            owner_name: owner_name.into(),
            html,
            updated: now_unix(),
            kind: kind.into(),
            posted: false,
            files: Vec::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated = now_unix();
    }

    pub fn pinned(&self) -> bool {
        matches!(
            self.id.as_str(),
            MANIFESTO_ID | HTML_DOC_ID | CSS_DOC_ID
        )
    }
}

pub const MANIFESTO_ID: &str = "nh-manifesto";
pub const HTML_DOC_ID: &str = "nh-html-doc";
pub const CSS_DOC_ID: &str = "nh-css-doc";

pub fn manifesto() -> Nethook {
    Nethook {
        id: MANIFESTO_ID.into(),
        title: "A Cypherpunk's Manifesto".into(),
        owner_id: "grid".into(),
        owner_name: "Eric Hughes".into(),
        html: MANIFESTO_HTML.into(),
        updated: u64::MAX,
        kind: "lesson".into(),
        posted: false,
            files: Vec::new(),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn ensure_pinned(list: &mut Vec<Nethook>) {
    list.retain(|h| !h.pinned());
    list.insert(0, css_doc());
    list.insert(0, html_doc());
    list.insert(0, manifesto());
}

fn doc(id: &str, title: &str, html: &str) -> Nethook {
    Nethook {
        id: id.into(),
        title: title.into(),
        owner_id: "grid".into(),
        owner_name: "INDEX".into(),
        html: html.into(),
        updated: u64::MAX - 1,
        kind: "lesson".into(),
        posted: false,
            files: Vec::new(),
    }
}

pub fn html_doc() -> Nethook {
    doc(HTML_DOC_ID, "HTML on the grid", HTML_DOC)
}

pub fn css_doc() -> Nethook {
    doc(CSS_DOC_ID, "CSS on the grid", CSS_DOC)
}

const HTML_DOC: &str = r#"<h1>HTML on the grid</h1>
<p>A nethook is a page this table can paint. You write tags. The preview shows them at once. Nothing on the page can run a program, and nothing on the page can fetch a site.</p>
<h2>How a page is built</h2>
<p>A tag is a name in angle brackets. Most tags come in a pair. The first opens. The second closes, and it has a slash.</p>
<pre>&lt;p&gt;One sentence.&lt;/p&gt;</pre>
<p>What you put between the pair is what people read. A page is just these pairs, one after another, from top to bottom.</p>
<h2>Lesson 1 — the title</h2>
<p>h1 is the name of the page. Use one. It is the line someone sees first.</p>
<pre>&lt;h1&gt;Night stall&lt;/h1&gt;</pre>
<h2>Lesson 2 — smaller titles</h2>
<p>h2 is a section. h3 is a section inside that. Read them as a list of contents: the h1, then each h2, then the h3 lines under it.</p>
<pre>&lt;h2&gt;Prices&lt;/h2&gt;
&lt;h3&gt;Maps&lt;/h3&gt;</pre>
<h2>Lesson 3 — a paragraph</h2>
<p>p is one idea. When the idea changes, start another p. Do not put the whole handout in one paragraph.</p>
<pre>&lt;p&gt;Open after dusk. Cash only.&lt;/p&gt;
&lt;p&gt;The back room is for the job.&lt;/p&gt;</pre>
<h2>Lesson 4 — a list</h2>
<p>ul is the list. Each line is an li. Use a list when the order is not the point: gear, names, rumors.</p>
<pre>&lt;ul&gt;
&lt;li&gt;Maps&lt;/li&gt;
&lt;li&gt;Rumors&lt;/li&gt;
&lt;li&gt;A key&lt;/li&gt;
&lt;/ul&gt;</pre>
<h2>Lesson 5 — loud and lean</h2>
<p>strong is the word you would hit with a finger. em is a lean, a title, a foreign word. They sit inside a paragraph.</p>
<pre>&lt;p&gt;Bring &lt;strong&gt;the key&lt;/strong&gt;, not &lt;em&gt;a&lt;/em&gt; key.&lt;/p&gt;</pre>
<h2>Lesson 6 — words as typed</h2>
<p>code is a short mark: a price, a die, a handle. pre is a block that keeps your line breaks. Use pre for a sample of tags, or a note copied as written.</p>
<pre>&lt;p&gt;The code is &lt;code&gt;dusk-4&lt;/code&gt;.&lt;/p&gt;</pre>
<h2>Lesson 7 — someone else's line</h2>
<p>blockquote is a line you are repeating, not one you are claiming. The painter draws it in the warning color so it does not look like the rest of the page.</p>
<pre>&lt;blockquote&gt;She said the door is already open.&lt;/blockquote&gt;</pre>
<h2>Lesson 8 — a small table</h2>
<p>table holds rows. tr is one row. td is one cell. Two cells in a row sit side by side. Keep tables short. This is a handout, not a ledger.</p>
<pre>&lt;table&gt;
&lt;tr&gt;&lt;td&gt;Map&lt;/td&gt;&lt;td&gt;40 eb&lt;/td&gt;&lt;/tr&gt;
&lt;tr&gt;&lt;td&gt;Rumor&lt;/td&gt;&lt;td&gt;10 eb&lt;/td&gt;&lt;/tr&gt;
&lt;/table&gt;</pre>
<h2>Lesson 9 — a rule and a break</h2>
<p>hr draws a line. Use it between two parts of a handout. br is a single break when you do not want a whole new paragraph.</p>
<pre>&lt;p&gt;Prices&lt;/p&gt;
&lt;hr&gt;
&lt;p&gt;The job&lt;/p&gt;</pre>
<h2>Lesson 10 — a picture, a sound, a video</h2>
<p>Do not paste a web address. Attach the file on this page. The tag points at that file with nh-file.</p>
<pre>&lt;img src="nh-file://stall.png"&gt;
&lt;audio src="nh-file://rain.ogg"&gt;&lt;/audio&gt;
&lt;video src="nh-file://door.mp4"&gt;&lt;/video&gt;</pre>
<p>A picture is drawn in the page. A sound has play and stop, and it does not change the table mix. A video plays here when ffmpeg is on the computer, or you can open it outside.</p>
<h2>Lesson 11 — what is cut</h2>
<p>script does not run. iframe, object, and embed are removed. A javascript link is cut. A style rule cannot load a file. If a tag is not in these lessons, the painter skips it and keeps the words.</p>
<h2>A whole page</h2>
<pre>&lt;h1&gt;Night stall&lt;/h1&gt;
&lt;p&gt;Open after dusk.&lt;/p&gt;
&lt;ul&gt;
&lt;li&gt;Maps&lt;/li&gt;
&lt;li&gt;Rumors&lt;/li&gt;
&lt;/ul&gt;
&lt;blockquote&gt;Ask for Ada.&lt;/blockquote&gt;</pre>
<h2>If you do not want to type tags</h2>
<p>Open your page, press Build, and add a heading, a paragraph, a list, a picture. The preview is the same painter. Press Code when you want the tags.</p>
"#;

const CSS_DOC: &str = r#"<h1>CSS on the grid</h1>
<p>CSS is the paint for the tags. One style block at the top of the page names a tag and sets how it looks. The preview updates as you type. Nothing is loaded from the network.</p>
<h2>Where it goes</h2>
<p>The style block is first. After it, the page. A rule inside the block is a tag name, then braces, then the settings.</p>
<pre>&lt;style&gt;
h1 { color: #d6ff3f; font-size: 28px; }
&lt;/style&gt;
&lt;h1&gt;Stall&lt;/h1&gt;</pre>
<h2>Lesson 1 — color</h2>
<p>color is the ink. It is six hex digits after a hash. d6ff3f is the acid title. 4de8ff is the cyan line. f4efe4 is cream body text. ff4fd8 is hot pink. ff1744 is the warning red.</p>
<pre>&lt;style&gt;
p { color: #f4efe4; }
strong { color: #d6ff3f; }
&lt;/style&gt;</pre>
<p>The color applies to every tag of that name on the page. One rule for p paints every paragraph.</p>
<h2>Lesson 2 — background</h2>
<p>background is the slab behind the words. Use it on a title or a single loud line. A full-page background fights the desk, so keep it on one tag.</p>
<pre>&lt;style&gt;
h1 { color: #10140a; background: #d6ff3f; }
&lt;/style&gt;</pre>
<h2>Lesson 3 — size</h2>
<p>font-size is a number and the letters px. h1 is already large. Use size when a section should sit between a title and a sentence. 28 for a title, 20 for a section, 16 for reading, 13 for a note.</p>
<pre>&lt;style&gt;
h2 { font-size: 20px; color: #4de8ff; }
p { font-size: 16px; }
&lt;/style&gt;</pre>
<h2>Lesson 4 — one rule, several settings</h2>
<p>Settings are separated by semicolons. You can set color and size on the same tag.</p>
<pre>&lt;style&gt;
h1 { color: #d6ff3f; font-size: 28px; }
p { color: #f4efe4; font-size: 16px; }
li { color: #4de8ff; font-size: 15px; }
blockquote { color: #ff1744; font-size: 16px; }
&lt;/style&gt;
&lt;h1&gt;Stall&lt;/h1&gt;
&lt;p&gt;Quiet type. No flicker.&lt;/p&gt;
&lt;ul&gt;&lt;li&gt;Maps&lt;/li&gt;&lt;/ul&gt;</pre>
<h2>Lesson 5 — what the names mean</h2>
<ul>
<li>h1 h2 h3 are the titles</li>
<li>p is a paragraph</li>
<li>li is a list line</li>
<li>strong and em are words inside a line</li>
<li>code and pre are words as typed</li>
<li>blockquote is a quoted line</li>
<li>td is a table cell</li>
</ul>
<p>If you style a name that is not on the page, nothing changes. If you misspell the name, nothing changes. Look at the preview.</p>
<h2>What is refused</h2>
<ul>
<li>url( ) would fetch a picture or a font. It is cut.</li>
<li>@import would fetch another style sheet. It is cut.</li>
<li>A color that is not six hex digits is ignored.</li>
<li>There is no animation and no blinking. The page stays still.</li>
</ul>
<p>Pictures, sound, and video are attached files, not style rules. See the HTML lesson.</p>
"#;

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
    if h.pinned() || h.id == HTML_DOC_ID || h.id == CSS_DOC_ID {
        return;
    }
    let d = dir(root);
    let _ = std::fs::create_dir_all(&d);
    if let Ok(s) = serde_json::to_string_pretty(h) {
        let _ = std::fs::write(d.join(format!("{}.json", h.id)), s);
    }
}

pub fn delete_one(root: &Path, id: &str) {
    if id == MANIFESTO_ID || id == HTML_DOC_ID || id == CSS_DOC_ID {
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
    let low = s.to_lowercase();
    if low.contains("@import") || low.contains("url(") || low.contains("javascript:") {
        s = s
            .replace("@import", "")
            .replace("@IMPORT", "")
            .replace("url(", "")
            .replace("URL(", "")
            .replace("javascript:", "");
    }
    for tag in ["script", "iframe", "object", "embed"] {
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

fn hex_color(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgb(
        (n >> 16) as u8,
        (n >> 8) as u8,
        n as u8,
    ))
}

fn parse_styles(html: &str) -> HashMap<String, (Option<Color32>, Option<Color32>, Option<f32>)> {
    let mut map = HashMap::new();
    let low = html.to_lowercase();
    let Some(a) = low.find("<style") else {
        return map;
    };
    let Some(start) = html[a..].find('>') else {
        return map;
    };
    let body = &html[a + start + 1..];
    let end = body.to_lowercase().find("</style>").unwrap_or(body.len());
    let css = &body[..end];
    for block in css.split('}') {
        let Some((sel, props)) = block.split_once('{') else {
            continue;
        };
        let sel = sel.split_whitespace().last().unwrap_or("").trim().to_lowercase();
        if sel.is_empty() {
            continue;
        }
        let mut color = None;
        let mut bg = None;
        let mut size = None;
        for decl in props.split(';') {
            let Some((k, v)) = decl.split_once(':') else {
                continue;
            };
            let k = k.trim().to_lowercase();
            let v = v.trim();
            if k == "color" {
                color = hex_color(v);
            } else if k == "background" || k == "background-color" {
                bg = hex_color(v);
            } else if k == "font-size" {
                size = v.trim_end_matches("px").trim().parse().ok();
            }
        }
        map.insert(sel, (color, bg, size));
    }
    map
}

fn drop_styles(html: &str) -> String {
    let mut s = html.to_string();
    loop {
        let low = s.to_lowercase();
        let Some(a) = low.find("<style") else {
            break;
        };
        let rest = &low[a..];
        let end = rest
            .find("</style>")
            .map(|i| a + i + "</style>".len())
            .unwrap_or(s.len());
        s.replace_range(a..end.min(s.len()), "");
    }
    s
}

pub struct HookFace<'a> {
    pub root: &'a Path,
    pub hook_id: &'a str,
    pub tex: &'a mut crate::images::TexCache,
    pub playing: &'a str,
}

pub enum HookAct {
    Audio(PathBuf),
    Open(PathBuf),
    Video(PathBuf),
}

pub fn paint(ui: &mut egui::Ui, html: &str, mut face: Option<&mut HookFace<'_>>) -> Vec<HookAct> {
    let mut acts = Vec::new();
    let styles = parse_styles(html);
    let clean = drop_styles(&strip_danger(html));
    let mut i = 0;
    let mut buf = String::new();
    let mut kind = String::from("p");
    let mut row: Vec<String> = Vec::new();
    let mut in_row = false;
    let flush = |ui: &mut egui::Ui, buf: &mut String, kind: &str, in_row: bool, row: &mut Vec<String>| {
        let t = buf.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.is_empty() {
            buf.clear();
            return;
        }
        let (col, bg, sz) = styles
            .get(kind)
            .copied()
            .unwrap_or((None, None, None));
        let size = sz.unwrap_or(match kind {
            "h1" => 26.0,
            "h2" => 20.0,
            "h3" => 14.0,
            _ => 15.0,
        });
        let color = col.unwrap_or(match kind {
            "h1" | "h3" => CYAN,
            "h2" | "li" => CYAN,
            "strong" | "td" => crate::theme::ACID,
            "em" | "blockquote" => crate::theme::NEON_RED,
            "code" | "pre" => CYAN,
            _ => CREAM,
        });
        let mut text = RichText::new(if kind == "h1" { t.to_uppercase() } else { t.clone() })
            .size(size)
            .color(color);
        if let Some(b) = bg {
            text = text.background_color(b);
        }
        text = if matches!(kind, "h1" | "h2") {
            text.family(theme::display())
        } else if kind == "h3" {
            text.family(theme::mono())
        } else {
            text.family(theme::ui_font())
        };
        if kind == "td" && in_row {
            row.push(t);
            buf.clear();
            return;
        }
        match kind {
            "li" => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("▸").color(CYAN).size(14.0));
                    ui.add(egui::Label::new(text).wrap());
                });
            }
            _ => {
                ui.add(egui::Label::new(text).wrap());
            }
        }
        buf.clear();
        ui.add_space(6.0);
    };
    let paint_row = |ui: &mut egui::Ui, row: &mut Vec<String>| {
        if row.is_empty() {
            return;
        }
        if row.len() == 1 {
            ui.add(egui::Label::new(RichText::new(row[0].clone()).color(CREAM).size(15.0)).wrap());
        } else {
            let n = row.len().min(4);
            ui.columns(n, |cols| {
                for (i, cell) in row.iter().take(n).enumerate() {
                    cols[i].add(
                        egui::Label::new(RichText::new(cell).color(CREAM).size(15.0)).wrap(),
                    );
                }
            });
        }
        row.clear();
        ui.add_space(6.0);
    };
    while i < clean.len() {
        if clean.as_bytes()[i] == b'<' {
            flush(ui, &mut buf, &kind, in_row, &mut row);
            let (name, closing, next) = tag_name(&clean, i);
            let src = attr_value(&clean[i..next.min(clean.len())], "src");
            if !closing && matches!(name.as_str(), "img" | "audio" | "video") {
                paint_media(ui, &name, src.as_deref(), &mut face, &mut acts);
                i = next;
                continue;
            }
            if name == "tr" && closing {
                flush(ui, &mut buf, &kind, in_row, &mut row);
                paint_row(ui, &mut row);
                in_row = false;
                i = next;
                continue;
            }
            if name == "tr" && !closing {
                in_row = true;
                row.clear();
                i = next;
                continue;
            }
            if name == "br" {
                ui.add_space(8.0);
            } else if name == "hr" {
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width().max(40.0), 2.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(r, 0.0, CYAN);
                ui.add_space(6.0);
            } else if !closing {
                match name.as_str() {
                    "h1" | "h2" | "h3" | "li" | "p" | "strong" | "em" | "code" | "pre"
                    | "blockquote" | "td" => kind = name,
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
    flush(ui, &mut buf, &kind, in_row, &mut row);
    if in_row {
        paint_row(ui, &mut row);
    }
    acts
}

fn attr_value(tag: &str, key: &str) -> Option<String> {
    let low = tag.to_lowercase();
    let needle = format!("{key}=\"");
    let i = low.find(&needle)?;
    let rest = &tag[i + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn paint_media(
    ui: &mut egui::Ui,
    name: &str,
    src: Option<&str>,
    face: &mut Option<&mut HookFace<'_>>,
    acts: &mut Vec<HookAct>,
) {
    let Some(src) = src.and_then(|s| safe_file_name(s)) else {
        ui.label(RichText::new("That file is not attached to this page.").color(DIM).size(12.0));
        return;
    };
    let Some(face) = face.as_deref_mut() else {
        ui.label(RichText::new(format!("{name} · {src}")).color(CYAN).size(13.0));
        return;
    };
    let Some(path) = hook_file(face.root, face.hook_id, &src) else {
        ui.label(RichText::new(format!("Missing {src}")).color(DIM).size(12.0));
        return;
    };
    match name {
        "img" => {
            let w = ui.available_width().min(480.0).max(40.0);
            let _ = crate::images::show_fit(ui, face.tex, &path, Vec2::new(w, 240.0));
        }
        "audio" => {
            let on = face.playing == src;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&src).color(CREAM).size(13.0));
                if theme::neon_btn(ui, if on { "Stop sound" } else { "Play sound" }).clicked() {
                    acts.push(HookAct::Audio(path.clone()));
                }
            });
        }
        "video" => {
            let frame = video_frame(&path);
            if frame.is_file() {
                let w = ui.available_width().min(480.0).max(40.0);
                let _ = crate::images::show_fit(ui, face.tex, &frame, Vec2::new(w, 240.0));
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&src).color(CREAM).size(13.0));
                if theme::neon_btn(ui, "Play video").clicked() {
                    acts.push(HookAct::Video(path.clone()));
                }
                if theme::neon_btn(ui, "Open outside").clicked() {
                    acts.push(HookAct::Open(path.clone()));
                }
            });
        }
        _ => {}
    }
    ui.add_space(6.0);
}

pub fn safe_file_name(src: &str) -> Option<String> {
    let name = src.trim().strip_prefix("nh-file://").unwrap_or(src.trim());
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains(':')
    {
        return None;
    }
    Some(name.to_string())
}

pub fn file_dir(root: &Path, hook_id: &str) -> PathBuf {
    let id: String = hook_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    dir(root).join("files").join(id)
}

pub fn hook_file(root: &Path, hook_id: &str, src: &str) -> Option<PathBuf> {
    let name = safe_file_name(src)?;
    let path = file_dir(root, hook_id).join(&name);
    path.is_file().then_some(path)
}

pub fn video_frame(video: &Path) -> PathBuf {
    let stem = video
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "clip".into());
    video.with_file_name(format!("{stem}-frame.png"))
}

pub fn store_attachment(root: &Path, hook_id: &str, src_path: &Path) -> Result<HookFile, String> {
    let raw = src_path
        .file_name()
        .ok_or_else(|| "That file has no name.".to_string())?
        .to_string_lossy()
        .to_string();
    let name = safe_file_name(&raw).ok_or_else(|| "Use a simple file name.".to_string())?;
    let kind = match name.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" => "image",
        "ogg" | "mp3" | "wav" | "flac" | "opus" | "m4a" | "aac" => "audio",
        "mp4" | "webm" | "mkv" => "video",
        _ => return Err("Use a picture, a sound, or a video.".into()),
    };
    let dest_dir = file_dir(root, hook_id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    std::fs::copy(src_path, dest_dir.join(&name)).map_err(|e| e.to_string())?;
    Ok(HookFile {
        name,
        kind: kind.into(),
    })
}

pub fn tag_for(file: &HookFile) -> String {
    let src = format!("nh-file://{}", file.name);
    match file.kind.as_str() {
        "audio" => format!("<audio src=\"{src}\"></audio>\n"),
        "video" => format!("<video src=\"{src}\"></video>\n"),
        _ => format!("<img src=\"{src}\">\n"),
    }
}

pub fn chrome_frame(ui: &mut egui::Ui, title: &str, owner: &str, inner: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, theme::fade(theme::HOT, 160)))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let (net, _) = ui.allocate_exact_size(Vec2::new(40.0, 20.0), egui::Sense::hover());
                theme::fill_chamfer(
                    ui,
                    net,
                    4.0,
                    PANEL,
                    egui::Stroke::new(1.0, theme::ACID),
                );
                ui.painter().text(
                    net.center(),
                    egui::Align2::CENTER_CENTER,
                    "NH",
                    FontId::new(11.0, theme::mono()),
                    theme::ACID,
                );
                ui.label(
                    RichText::new(title)
                        .family(theme::display())
                        .size(20.0)
                        .color(theme::ACID),
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
                Vec2::new(ui.available_width().max(8.0), 2.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(r, 0.0, theme::fade(theme::HOT, 90));
            ui.painter().rect_filled(
                Rect::from_min_size(r.left_top(), Vec2::new(36.0, 2.0)),
                0.0,
                CYAN,
            );
            ui.add_space(8.0);
            inner(ui);
        });
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    List(String),
    Quote(String),
    Rule,
    Picture(String),
    Sound(String),
    Film(String),
    Columns(String, String),
}

pub fn blocks_to_html(blocks: &[Block]) -> String {
    let mut html = String::from(
        "<style>h1{color:#d6ff3f;font-size:26px}h2{color:#4de8ff;font-size:20px}p,li{color:#f4efe4;font-size:16px}</style>\n",
    );
    for b in blocks {
        match b {
            Block::Heading { level, text } => {
                let n = match *level {
                    2 => 2,
                    3.. => 3,
                    _ => 1,
                };
                html.push_str(&format!("<h{n}>{}</h{n}>\n", escape_html(text)));
            }
            Block::Paragraph(t) => html.push_str(&format!("<p>{}</p>\n", escape_html(t))),
            Block::List(t) => {
                html.push_str("<ul>\n");
                for line in t.lines().map(str::trim).filter(|s| !s.is_empty()) {
                    html.push_str(&format!("<li>{}</li>\n", escape_html(line)));
                }
                html.push_str("</ul>\n");
            }
            Block::Quote(t) => {
                html.push_str(&format!("<blockquote>{}</blockquote>\n", escape_html(t)))
            }
            Block::Rule => html.push_str("<hr>\n"),
            Block::Picture(n) => html.push_str(&format!("<img src=\"nh-file://{}\">\n", escape_html(n))),
            Block::Sound(n) => {
                html.push_str(&format!("<audio src=\"nh-file://{}\"></audio>\n", escape_html(n)))
            }
            Block::Film(n) => {
                html.push_str(&format!("<video src=\"nh-file://{}\"></video>\n", escape_html(n)))
            }
            Block::Columns(a, b) => html.push_str(&format!(
                "<table><tr><td>{}</td><td>{}</td></tr></table>\n",
                escape_html(a),
                escape_html(b)
            )),
        }
    }
    html
}

pub fn html_to_blocks(html: &str) -> Vec<Block> {
    let clean = drop_styles(&strip_danger(html));
    let mut blocks = Vec::new();
    let mut i = 0;
    let mut buf = String::new();
    let mut kind = String::new();
    let mut items: Vec<String> = Vec::new();
    let mut cells: Vec<String> = Vec::new();
    while i < clean.len() {
        if clean.as_bytes()[i] == b'<' {
            let (name, closing, next) = tag_name(&clean, i);
            let src = attr_value(&clean[i..next.min(clean.len())], "src");
            if !closing && matches!(name.as_str(), "img" | "audio" | "video") {
                if let Some(file) = src.as_deref().and_then(safe_file_name) {
                    blocks.push(match name.as_str() {
                        "audio" => Block::Sound(file),
                        "video" => Block::Film(file),
                        _ => Block::Picture(file),
                    });
                }
                i = next;
                continue;
            }
            if closing {
                let text = buf.split_whitespace().collect::<Vec<_>>().join(" ");
                buf.clear();
                match name.as_str() {
                    "h1" => blocks.push(Block::Heading { level: 1, text }),
                    "h2" => blocks.push(Block::Heading { level: 2, text }),
                    "h3" => blocks.push(Block::Heading { level: 3, text }),
                    "p" => {
                        if !text.is_empty() {
                            blocks.push(Block::Paragraph(text));
                        }
                    }
                    "blockquote" => {
                        if !text.is_empty() {
                            blocks.push(Block::Quote(text));
                        }
                    }
                    "li" => {
                        if !text.is_empty() {
                            items.push(text);
                        }
                    }
                    "ul" => {
                        if !items.is_empty() {
                            blocks.push(Block::List(items.join("\n")));
                            items.clear();
                        }
                    }
                    "td" => {
                        if !text.is_empty() {
                            cells.push(text);
                        }
                    }
                    "tr" => {
                        if cells.len() >= 2 {
                            blocks.push(Block::Columns(cells[0].clone(), cells[1].clone()));
                        } else if let Some(c) = cells.first() {
                            blocks.push(Block::Paragraph(c.clone()));
                        }
                        cells.clear();
                    }
                    _ => {}
                }
                kind.clear();
            } else if name == "hr" {
                blocks.push(Block::Rule);
            } else {
                kind = name;
            }
            i = next;
            continue;
        }
        let ch = clean[i..].chars().next().unwrap_or(' ');
        if !kind.is_empty() {
            buf.push(ch);
        }
        i += ch.len_utf8();
    }
    if blocks.is_empty() {
        blocks.push(Block::Paragraph(String::new()));
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_save_is_a_handout() {
        let h: Nethook = serde_json::from_str(
            r#"{"id":"a","title":"t","owner_id":"1","owner_name":"A","html":"<p>x</p>","updated":1}"#,
        )
        .unwrap();
        assert_eq!(h.kind, "handout");
        assert!(!h.posted);
    }

    #[test]
    fn strip_kills_script() {
        let s = strip_danger("<p>hi</p><script>alert(1)</script><p>ok</p>");
        assert!(!s.to_lowercase().contains("script"));
        assert!(s.contains("hi"));
        let css = strip_danger("<style>h1{color:#ff6a12}</style><h1>Stall</h1>");
        assert!(css.contains("style") || css.contains("Stall"));
        assert!(parse_styles("<style>h1 { color: #ff6a12; font-size: 28px; }</style>").contains_key("h1"));
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
            kind: String::new(),
            posted: false,
            files: Vec::new(),
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
                kind: String::new(),
                posted: false,
            files: Vec::new(),
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
                kind: String::new(),
                posted: false,
            files: Vec::new(),
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
                kind: String::new(),
                posted: false,
            files: Vec::new(),
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
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].title, "A Cypherpunk's Manifesto");
        assert!(v.iter().any(|h| h.id == HTML_DOC_ID));
        assert!(v.iter().any(|h| h.id == CSS_DOC_ID));
        merge(
            &mut v,
            Nethook {
                id: MANIFESTO_ID.into(),
                title: "stolen".into(),
                owner_id: "x".into(),
                owner_name: "x".into(),
                html: "no".into(),
                updated: 99,
                kind: String::new(),
                posted: false,
            files: Vec::new(),
            },
        );
        assert_eq!(v[0].title, "A Cypherpunk's Manifesto");
        delete_one(Path::new("/tmp"), MANIFESTO_ID);
        ensure_pinned(&mut v);
        assert_eq!(v[0].id, MANIFESTO_ID);
    }

    #[test]
    fn blocks_roundtrip_a_heading_and_keep_remote_pictures_out() {
        let html = blocks_to_html(&[
            Block::Heading {
                level: 1,
                text: "Stall".into(),
            },
            Block::Paragraph("Open after dusk.".into()),
        ]);
        let back = html_to_blocks(&html);
        assert!(matches!(back[0], Block::Heading { level: 1, .. }));
        assert!(html.contains("Stall"));
        assert_eq!(safe_file_name("nh-file://stall.png").as_deref(), Some("stall.png"));
        assert!(safe_file_name("https://example.com/a.png").is_none());
        assert!(safe_file_name("../secret").is_none());
    }
}
