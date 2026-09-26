//! Rebels of the Net — a fixer on this computer. No cloud, no API key.

use crate::theme::{self, CREAM, CYAN, DIM, MUTED};
use eframe::egui::{self, RichText};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub path: String,
    pub bytes: u64,
}

#[derive(Default)]
pub struct Rotn {
    pub models: Vec<Model>,
    pub pick: usize,
    pub input: String,
    pub log: Vec<(bool, String)>,
    pub memory: String,
    pub loaded: bool,
    pub engine_server: bool,
    pub server: String,
    pub server_models: Vec<String>,
    pub server_pick: usize,
    pub pending: Option<Receiver<String>>,
    pub post: Option<(String, String, String)>,
    pub salt: u32,
}

pub struct Desk {
    pub world: String,
    pub place: String,
    pub inside: bool,
    pub period: String,
    pub date: String,
    pub clock: String,
    pub scene: String,
    pub seats: Vec<String>,
    pub sheets: Vec<String>,
}

impl Rotn {
    pub fn load(root: &Path) -> Self {
        let mut s = Self::default();
        s.models = read_models(root);
        s.memory = std::fs::read_to_string(root.join("data/rotn-memory.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("note").and_then(|n| n.as_str()).map(|n| n.to_string()))
            .unwrap_or_default();
        s.loaded = runner_path().is_some();
        if s.server.is_empty() {
            s.server = "127.0.0.1:11434".into();
        }
        if s.log.is_empty() {
            s.log.push((
                false,
                "Rebels of the Net. Drop a GGUF into Models, or ask for the date, the seats, or the sheets. Nothing leaves this deck.".into(),
            ));
        }
        s
    }
}

fn models_dir(root: &Path) -> PathBuf {
    root.join("data/models")
}

fn read_models(root: &Path) -> Vec<Model> {
    let dir = models_dir(root);
    let _ = std::fs::create_dir_all(&dir);
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if ext != "gguf" && ext != "bin" && ext != "onnx" {
                continue;
            }
            let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "model".into());
            out.push(Model {
                name,
                path: p.to_string_lossy().into_owned(),
                bytes,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn runner_path() -> Option<PathBuf> {
    for name in ["llama-cli", "llama-cli.exe", "main", "main.exe"] {
        if let Some(p) = crate::sys::which(name) {
            return Some(p);
        }
        let beside = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.join(name)))
            .filter(|p| p.is_file());
        if beside.is_some() {
            return beside;
        }
    }
    None
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Soul {
    pub name: String,
    pub vice: String,
    pub tell: String,
    pub voice: String,
}

pub fn soul_of(root: &Path, deck: &str) -> Soul {
    let path = root.join("data/rotn-soul.json");
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(soul) = serde_json::from_str(&s) {
            return soul;
        }
    }
    let soul = mint_soul(deck);
    let _ = std::fs::create_dir_all(root.join("data"));
    if let Ok(s) = serde_json::to_string_pretty(&soul) {
        let _ = std::fs::write(path, s);
    }
    soul
}

pub fn reroll_soul(root: &Path, deck: &str) -> Soul {
    let _ = std::fs::remove_file(root.join("data/rotn-soul.json"));
    let mut salt = deck.to_string();
    salt.push_str(&rand::random::<u32>().to_string());
    let soul = mint_soul(&salt);
    if let Ok(s) = serde_json::to_string_pretty(&soul) {
        let _ = std::fs::write(root.join("data/rotn-soul.json"), s);
    }
    soul
}

fn mint_soul(deck: &str) -> Soul {
    let mut h = 0u32;
    for b in deck.bytes() {
        h = h.wrapping_mul(16777619) ^ b as u32;
    }
    let names = ["Ash", "Vesper", "Kite", "Nox", "Rook", "Sable", "Iota", "Wren"];
    let vices = [
        "counts exits before faces",
        "keeps a paper map of a city that is gone",
        "will not say a real name twice",
        "trusts locks more than people",
    ];
    let tells = [
        "taps the desk once before a lie",
        "looks at the door when the mix drops",
        "speaks softer when the truth is close",
        "never sits with their back to a window",
    ];
    let voices = [
        "short sentences. no comfort.",
        "dry. precise. a little tired.",
        "street-quiet. never shouts.",
        "old radio. warm and brief.",
    ];
    Soul {
        name: names[(h as usize) % names.len()].into(),
        vice: vices[((h >> 3) as usize) % vices.len()].into(),
        tell: tells[((h >> 6) as usize) % tells.len()].into(),
        voice: voices[((h >> 9) as usize) % voices.len()].into(),
    }
}

pub fn answer(root: &Path, ask: &str, memory: &str, desk: &Desk) -> String {
    let soul = soul_of(root, "deck");
    let q = ask.to_lowercase();
    let mut bits = Vec::new();
    if q.contains("date") || q.contains("day") || q.contains("calendar") || q.contains("hour") {
        bits.push(format!(
            "Campaign date {}. Clock {}. It is {}.",
            desk.date, desk.clock, desk.period
        ));
    }
    if q.contains("time") || q.contains("clock") {
        bits.push(format!("Table clock {}.", desk.clock));
    }
    if q.contains("seat") || q.contains("who") || q.contains("player") || q.contains("table") {
        if desk.seats.is_empty() {
            bits.push("No one else is at the table.".into());
        } else {
            bits.push(format!("Seated: {}.", desk.seats.join(", ")));
        }
    }
    if q.contains("sheet") || q.contains("character") || q.contains("table") {
        if desk.sheets.is_empty() {
            bits.push("No sheets on this deck.".into());
        } else {
            bits.push(format!("Sheets: {}.", desk.sheets.join(", ")));
        }
    }
    if q.contains("place") || q.contains("where") || q.contains("scene") || q.contains("music") || q.contains("mix") {
        let door = if desk.inside { "inside" } else { "outside" };
        bits.push(format!(
            "{} at {}, {}. Scene {}.",
            desk.world, desk.place, door, desk.scene
        ));
    }
    if q.contains("remember") {
        bits.push("Write that in the memory box. It stays on this deck.".into());
    }
    if !memory.trim().is_empty() && (q.contains("memory") || q.contains("note")) {
        bits.push(format!("Memory: {memory}"));
    }
    let lead = format!("{} — {}. ", soul.name, soul.voice);
    if bits.is_empty() {
        return format!(
            "{lead}Press Hour, Table, Place, Rumor, Job, or NPC. Or ask in your own words. Nothing here is sent away."
        );
    }
    format!("{lead}{}", bits.join(" "))
}

pub fn job_line(root: &Path, desk: &Desk, job: &str, salt: u32) -> String {
    let soul = soul_of(root, "deck");
    let lead = format!("{} — {}. ", soul.name, soul.voice);
    let door = if desk.inside { "inside" } else { "outside" };
    let n = salt as usize;
    let body = match job {
        "hour" => format!(
            "Campaign date {}. Clock {}. It is {}.",
            desk.date, desk.clock, desk.period
        ),
        "table" => {
            let who = if desk.seats.is_empty() {
                "No one else is at the table.".into()
            } else {
                format!("Seated: {}.", desk.seats.join(", "))
            };
            let sheets = if desk.sheets.is_empty() {
                "No sheets on this deck.".into()
            } else {
                format!("Sheets: {}.", desk.sheets.join(", "))
            };
            format!("{who} {sheets}")
        }
        "place" => format!(
            "{} at {}, {}. Scene {}.",
            desk.world, desk.place, door, desk.scene
        ),
        "rumor" => {
            let lines = [
                format!("At {}, someone is selling a name for less than it is worth.", desk.place),
                format!("The {} watch at {} is two minutes fast, on purpose.", desk.period, desk.place),
                format!("A door at {} locks from the wrong side.", desk.place),
                format!("They say the pay at {} is real and the exit is not.", desk.place),
            ];
            lines[n % lines.len()].clone()
        }
        "job" => {
            let lines = [
                format!("A clerk at {} wants a package walked {} before {}. Pay is quiet coin.", desk.place, door, desk.period),
                format!("Someone at {} lost a key. Bring it back before {}. They pay in favors.", desk.place, desk.clock),
                format!("Stand watch {} at {} through {}. The pay is a name you can use once.", door, desk.place, desk.period),
            ];
            lines[n % lines.len()].clone()
        }
        "npc" => {
            let other = mint_soul(&format!("{}-{}", desk.place, salt));
            format!(
                "{} — {}. Tell: {}.",
                other.name, other.vice, other.tell
            )
        }
        _ => "Press a job.".into(),
    };
    format!("{lead}{body}")
}

fn prompt_of(root: &Path, ask: &str, memory: &str, desk: &Desk) -> String {
    let soul = soul_of(root, "deck");
    let door = if desk.inside { "inside" } else { "outside" };
    format!(
        "You are {name}, a fixer at the table. Voice: {voice}. Vice: {vice}. Stay in that voice. Two or three sentences. World {world}. Place {place}, {door}. Time {period}. Date {date}. Clock {clock}. Scene {scene}. Seated: {seats}. Sheets: {sheets}. Memory: {memory}\nUser: {ask}\n{name}:",
        name = soul.name,
        voice = soul.voice,
        vice = soul.vice,
        world = desk.world,
        place = desk.place,
        door = door,
        period = desk.period,
        date = desk.date,
        clock = desk.clock,
        scene = desk.scene,
        seats = desk.seats.join(", "),
        sheets = desk.sheets.join(", "),
        memory = memory,
    )
}

fn run_model(root: &Path, model: &Path, ask: &str, memory: &str, desk: &Desk) -> String {
    let Some(bin) = runner_path() else {
        return "No llama-cli on this computer. The Hour, Table, Place, Rumor, Job, and NPC buttons still work.".into();
    };
    let prompt = prompt_of(root, ask, memory, desk);
    let mut cmd = std::process::Command::new(bin);
    cmd.args(["-m", &model.to_string_lossy(), "-p", &prompt, "-n", "120", "--no-display-prompt"]);
    match cmd.output() {
        Ok(o) if o.status.success() => {
            let t = String::from_utf8_lossy(&o.stdout);
            let line = t.lines().last().unwrap_or("").trim();
            if line.is_empty() {
                "The model returned nothing.".into()
            } else {
                line.to_string()
            }
        }
        Ok(o) => format!("The model failed. {}", String::from_utf8_lossy(&o.stderr).trim()),
        Err(e) => format!("The model failed. {e}"),
    }
}

pub fn loopback_ok(addr: &str) -> bool {
    let addr = addr.trim();
    let host = if let Some(rest) = addr.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else if addr.matches(':').count() > 1 {
        addr
    } else {
        addr.split(':').next().unwrap_or("")
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn http(addr: &str, method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    if !loopback_ok(addr) {
        return Err("That address is not on this computer. Nothing was sent.".into());
    }
    let sock = addr
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "Bad address.".to_string())?;
    let wait = if body.is_some() { 45 } else { 2 };
    let mut stream = TcpStream::connect_timeout(&sock, Duration::from_secs(wait)).map_err(|e| {
        format!("Nothing is listening at {addr}. Start Ollama or llama.cpp on this computer. {e}")
    })?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(wait)));
    let payload = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    );
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let json = text
        .find('{')
        .map(|i| text[i..].trim().to_string())
        .unwrap_or_default();
    if json.is_empty() {
        return Err("The local model sent nothing back.".into());
    }
    Ok(json)
}

pub fn list_server(addr: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    if let Ok(body) = http(addr, "GET", "/api/tags", None) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(arr) = v.get("models").and_then(|m| m.as_array()) {
                for m in arr {
                    if let Some(n) = m.get("name").and_then(|x| x.as_str()) {
                        names.push(n.to_string());
                    }
                }
            }
        }
    }
    if names.is_empty() {
        if let Ok(body) = http(addr, "GET", "/v1/models", None) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(arr) = v.get("data").and_then(|m| m.as_array()) {
                    for m in arr {
                        if let Some(n) = m.get("id").and_then(|x| x.as_str()) {
                            names.push(n.to_string());
                        }
                    }
                }
            }
        }
    }
    if names.is_empty() {
        return Err("No models listed. Pull DeepSeek or Kimi in Ollama, then press Rescan.".into());
    }
    names.sort();
    names.dedup();
    Ok(names)
}

fn ask_server(addr: &str, model: &str, root: &Path, ask: &str, memory: &str, desk: &Desk) -> String {
    let prompt = prompt_of(root, ask, memory, desk);
    let ollama = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            {"role": "user", "content": prompt}
        ]
    });
    if let Ok(body) = http(addr, "POST", "/api/chat", Some(&ollama.to_string())) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(t) = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                let t = t.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    let openai = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [{"role": "user", "content": prompt}]
    });
    match http(addr, "POST", "/v1/chat/completions", Some(&openai.to_string())) {
        Ok(body) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(t) = v
                    .pointer("/choices/0/message/content")
                    .and_then(|c| c.as_str())
                {
                    let t = t.trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
            "The local model sent nothing useful.".into()
        }
        Err(e) => e,
    }
}

fn spawn_ask(rotn: &mut Rotn, root: &Path, ask: String, desk: Desk) {
    if rotn.pending.is_some() {
        rotn.log.push((false, "Still thinking.".into()));
        return;
    }
    let (tx, rx) = mpsc::channel();
    rotn.pending = Some(rx);
    rotn.log.push((false, "…thinking".into()));
    let memory = rotn.memory.clone();
    let root = root.to_path_buf();
    if rotn.engine_server {
        let addr = rotn.server.clone();
        let model = rotn
            .server_models
            .get(rotn.server_pick)
            .cloned()
            .unwrap_or_default();
        thread::spawn(move || {
            let reply = if model.is_empty() {
                "Press Rescan and pick a model. DeepSeek and Kimi show up here after you start them on this computer.".into()
            } else {
                ask_server(&addr, &model, &root, &ask, &memory, &desk)
            };
            let _ = tx.send(reply);
        });
    } else {
        let model = rotn
            .models
            .get(rotn.pick)
            .map(|m| PathBuf::from(&m.path));
        thread::spawn(move || {
            let reply = if let Some(path) = model {
                run_model(&root, &path, &ask, &memory, &desk)
            } else {
                "No model file yet. The buttons on the left still answer.".into()
            };
            let _ = tx.send(reply);
        });
    }
}

pub fn paint(ui: &mut egui::Ui, root: &Path, rotn: &mut Rotn, desk: &Desk) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("REBELS OF THE NET")
                .family(theme::display())
                .size(26.0)
                .color(theme::ACID),
        );
        ui.label(
            RichText::new(if rotn.loaded { "RUNNER READY" } else { "TOOLS ONLY" })
                .family(theme::mono())
                .size(11.0)
                .color(if rotn.loaded { CYAN } else { DIM }),
        );
    });
    theme::kicker(ui, "ROTN://LOCAL  ·  NO WIRE");
    wrap_text_local(
        ui,
        "How this works. The fixer sits on your computer and knows this table. Press Hour, Table, or Place for the facts. Press Rumor, Job, or NPC and they make something up for where you are. You can type a question too. None of that is sent to anyone.",
    );
    wrap_text_local(
        ui,
        "You do not need a model. If you want longer answers, start DeepSeek or Kimi on this computer with Ollama, press Rescan, and pick the name. An address on the internet is refused.",
    );
    ui.add_space(6.0);
    if let Some(rx) = rotn.pending.take() {
        match rx.try_recv() {
            Ok(line) => {
                if rotn.log.last().map(|(u, t)| !u && t == "…thinking").unwrap_or(false) {
                    rotn.log.pop();
                }
                rotn.log.push((false, line));
            }
            Err(mpsc::TryRecvError::Empty) => {
                rotn.pending = Some(rx);
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                if rotn.log.last().map(|(u, t)| !u && t == "…thinking").unwrap_or(false) {
                    rotn.log.pop();
                }
                rotn.log.push((false, "The model stopped.".into()));
            }
        }
    }
    let rest = ui.available_rect_before_wrap();
    ui.allocate_rect(rest, egui::Sense::hover());
    let side_w = (rest.width() * 0.34).clamp(200.0, 340.0).min((rest.width() - 180.0).max(120.0));
    let (left, right) = rest.split_left_right_at_x(rest.left() + side_w);
    let left = left.shrink2(egui::Vec2::new(4.0, 2.0));
    let right = right.shrink2(egui::Vec2::new(6.0, 2.0));
    let mut side_host = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    side_host.set_clip_rect(left);
    egui::ScrollArea::vertical()
        .id_salt("rotn-side")
        .max_height(left.height().max(40.0))
        .auto_shrink([false, false])
        .show(&mut side_host, |side| {
            side.set_width((left.width() - 14.0).max(40.0));
            rotn_side(side, root, rotn, desk);
        });
    let mut talk = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    talk.set_clip_rect(right);
    let ask_h = 78.0;
    let log_h = (right.height() - ask_h).max(48.0);
    egui::ScrollArea::vertical()
        .id_salt("rotn-log")
        .max_height(log_h)
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut talk, |ui| {
            ui.set_width((right.width() - 16.0).max(40.0));
            for (user, line) in &rotn.log {
                ui.label(
                    RichText::new(if *user { "YOU" } else { "ROTN" })
                        .family(theme::mono())
                        .size(10.0)
                        .color(if *user { theme::ACID } else { CYAN }),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(line)
                            .color(CREAM)
                            .size(14.0)
                            .family(theme::ui_font()),
                    )
                    .wrap(),
                );
                ui.add_space(6.0);
            }
        });
    let field_w = (right.width() * 0.5).clamp(80.0, 480.0);
    talk.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut rotn.input)
                .desired_width(field_w.min(ui.available_width().max(40.0)))
                .hint_text("Ask the fixer…"),
        );
        if theme::neon_btn(ui, "Send").clicked() && !rotn.input.trim().is_empty() {
            let ask = rotn.input.trim().to_string();
            rotn.input.clear();
            rotn.log.push((true, ask.clone()));
            let use_model = if rotn.engine_server {
                !rotn.server_models.is_empty()
            } else {
                runner_path().is_some() && !rotn.models.is_empty()
            };
            if use_model {
                spawn_ask(rotn, root, ask, desk_owned(desk));
            } else {
                rotn.log.push((false, answer(root, &ask, &rotn.memory, desk)));
            }
        }
        if theme::neon_btn(ui, "Post rumor").clicked() {
            if let Some(line) = last_fixer(rotn) {
                rotn.post = Some(("rumor".into(), "Rumor".into(), line));
            }
        }
        if theme::neon_btn(ui, "Post job").clicked() {
            if let Some(line) = last_fixer(rotn) {
                rotn.post = Some(("job".into(), "Job".into(), line));
            }
        }
    });
}

fn rotn_side(side: &mut egui::Ui, root: &Path, rotn: &mut Rotn, desk: &Desk) {
    let soul = soul_of(root, "deck");
    side.label(RichText::new("SOUL").family(theme::mono()).size(11.0).color(crate::theme::ACID));
    side.label(RichText::new(&soul.name).color(CYAN).size(16.0).family(theme::display()));
    wrap_text_local(side, &format!("{} {}", soul.vice, soul.tell));
    if theme::neon_btn(side, "Reroll soul").clicked() {
        let _ = reroll_soul(root, "deck");
    }
    side.label(RichText::new("JOBS").family(theme::mono()).size(11.0).color(theme::ACID));
    side.horizontal_wrapped(|ui| {
        for job in ["Hour", "Table", "Place", "Rumor", "Job", "NPC"] {
            if theme::neon_btn(ui, job).clicked() {
                rotn.salt = rotn.salt.wrapping_add(1);
                let reply = job_line(root, desk, &job.to_lowercase(), rotn.salt);
                rotn.log.push((true, job.into()));
                rotn.log.push((false, reply));
            }
        }
    });
    side.label(RichText::new("ENGINE").family(theme::mono()).size(11.0).color(theme::ACID));
    side.horizontal_wrapped(|ui| {
        if theme::neon_btn_color(ui, "File", CYAN, !rotn.engine_server).clicked() {
            rotn.engine_server = false;
        }
        if theme::neon_btn_color(ui, "Server", CYAN, rotn.engine_server).clicked() {
            rotn.engine_server = true;
        }
    });
    if rotn.engine_server {
        side.horizontal_wrapped(|ui| {
            if theme::neon_btn(ui, "Ollama").clicked() {
                rotn.server = "127.0.0.1:11434".into();
            }
            if theme::neon_btn(ui, "llama.cpp").clicked() {
                rotn.server = "127.0.0.1:8080".into();
            }
        });
        side.add(
            egui::TextEdit::singleline(&mut rotn.server)
                .hint_text("127.0.0.1:11434")
                .desired_width(side.available_width()),
        );
        wrap_text_local(side, "Only this computer. DeepSeek and Kimi appear after you start them here.");
    }
    side.label(RichText::new("MODELS").family(theme::mono()).size(11.0).color(theme::ACID));
    wrap_text_local(side, "Add a .gguf, .bin, or .onnx. It is copied into data/models. The file stays on this machine.");
    if theme::neon_btn(side, "Add model").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Model", &["gguf", "bin", "onnx"])
            .pick_file()
        {
            let _ = std::fs::create_dir_all(models_dir(root));
            let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "model.gguf".into());
            let dest = models_dir(root).join(&name);
            if std::fs::copy(&path, &dest).is_ok() {
                rotn.models = read_models(root);
            }
        }
    }
    if theme::neon_btn(side, "Rescan").clicked() {
        rotn.loaded = runner_path().is_some();
        if rotn.engine_server {
            match list_server(&rotn.server) {
                Ok(names) => {
                    rotn.server_models = names;
                    rotn.server_pick = rotn.server_pick.min(rotn.server_models.len().saturating_sub(1));
                }
                Err(e) => rotn.log.push((false, e)),
            }
        } else {
            rotn.models = read_models(root);
        }
    }
    egui::ScrollArea::vertical().id_salt("rotn-models").max_height(160.0).show(side, |ui| {
        if rotn.engine_server {
            for (i, name) in rotn.server_models.iter().enumerate() {
                if theme::wide_btn(ui, name, "on this computer", rotn.server_pick == i).clicked() {
                    rotn.server_pick = i;
                }
            }
            if rotn.server_models.is_empty() {
                ui.label(RichText::new("Press Rescan. The buttons still work.").color(DIM).size(12.0));
            }
        } else {
            for (i, m) in rotn.models.iter().enumerate() {
                let mb = m.bytes as f32 / (1024.0 * 1024.0);
                let sub = format!("{mb:.1} MB");
                if theme::wide_btn(ui, &m.name, &sub, rotn.pick == i).clicked() {
                    rotn.pick = i;
                }
            }
            if rotn.models.is_empty() {
                ui.label(RichText::new("No model file yet. The buttons still work.").color(DIM).size(12.0));
            }
        }
    });
    side.label(RichText::new("MEMORY").family(theme::mono()).size(11.0).color(CYAN));
    let mem = side.add(
        egui::TextEdit::multiline(&mut rotn.memory)
            .desired_rows(4)
            .desired_width(side.available_width()),
    );
    if mem.changed() {
        let _ = std::fs::create_dir_all(root.join("data"));
        let _ = std::fs::write(
            root.join("data/rotn-memory.json"),
            format!("{{\"note\":{}}}", serde_json::to_string(&rotn.memory).unwrap_or_else(|_| "\"\"".into())),
        );
    }
}

fn desk_owned(desk: &Desk) -> Desk {
    Desk {
        world: desk.world.clone(),
        place: desk.place.clone(),
        inside: desk.inside,
        period: desk.period.clone(),
        date: desk.date.clone(),
        clock: desk.clock.clone(),
        scene: desk.scene.clone(),
        seats: desk.seats.clone(),
        sheets: desk.sheets.clone(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn refuses_the_internet() {
        assert!(super::loopback_ok("127.0.0.1:11434"));
        assert!(super::loopback_ok("localhost:8080"));
        assert!(super::loopback_ok("::1"));
        assert!(!super::loopback_ok("api.deepseek.com"));
        assert!(!super::loopback_ok("1.2.3.4:11434"));
    }
}

fn last_fixer(rotn: &Rotn) -> Option<String> {
    rotn.log.iter().rev().find(|(user, _)| !user).map(|(_, l)| l.clone())
}

fn wrap_text_local(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).color(MUTED).size(12.0).family(theme::ui_font())).wrap());
}
