use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 8766;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Idle,
    Host,
    Guest,
    Presence,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Wire {
    #[serde(rename = "hello")]
    Hello {
        id: String,
        name: String,
        role: String,
    },
    #[serde(rename = "welcome")]
    Welcome {
        id: String,
        role: String,
        peers: Vec<PeerInfo>,
    },
    #[serde(rename = "peers")]
    Peers { peers: Vec<PeerInfo> },
    #[serde(rename = "chat")]
    Chat {
        from: String,
        name: String,
        text: String,
        #[serde(default)]
        whisper: bool,
        #[serde(default)]
        to: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "voice")]
    Voice {
        from: String,
        name: String,
        action: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        crew: Option<String>,
    },
    #[serde(rename = "voice-pcm")]
    VoicePcm { from: String, pcm: String },
    #[serde(rename = "mix")]
    Mix {
        layers: HashMap<String, f32>,
        blight: bool,
        place: String,
        time: String,
        inside: bool,
    },
    #[serde(rename = "bye")]
    Bye,
    #[serde(rename = "image")]
    Image {
        from: String,
        name: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        crew: Option<String>,
        mime: String,
        data: String,
    },
    #[serde(rename = "video")]
    Video {
        from: String,
        name: String,
        action: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        crew: Option<String>,
    },
    #[serde(rename = "video-frame")]
    VideoFrame {
        from: String,
        name: String,
        kind: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        crew: Option<String>,
        data: String,
    },
    #[serde(rename = "nethook-put")]
    NethookPut { hook: crate::nethook::Nethook },
    #[serde(rename = "nethook-del")]
    NethookDel { id: String, owner_id: String },
    #[serde(rename = "nethook-ask")]
    NethookAsk,
    #[serde(rename = "file-start")]
    FileStart {
        from: String,
        name: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        crew: Option<String>,
        mime: String,
        filename: String,
        id: String,
        size: u64,
    },
    #[serde(rename = "file-chunk")]
    FileChunk {
        id: String,
        #[serde(default)]
        to: Option<String>,
        data: String,
    },
    #[serde(rename = "file-done")]
    FileDone {
        id: String,
        #[serde(default)]
        to: Option<String>,
    },
    #[serde(rename = "map-mark")]
    MapMark { mark: crate::maps::Mark },
    #[serde(rename = "map-mark-del")]
    MapMarkDel { id: String },
    #[serde(rename = "map-marks")]
    MapMarks { marks: Vec<crate::maps::Mark> },
    #[serde(rename = "map-marks-clear")]
    MapMarksClear,
    #[serde(rename = "map-marks-ask")]
    MapMarksAsk,
}

pub enum NetEvent {
    Status(String),
    Hosting {
        port: u16,
        addrs: Vec<String>,
        internet: bool,
    },
    Relay { url: String },
    Joined { addr: String },
    Left,
    Peers(Vec<PeerInfo>),
    Chat {
        from: String,
        name: String,
        text: String,
        whisper: bool,
    },
    Voice {
        from: String,
        name: String,
        action: String,
        to: Option<String>,
        crew: Option<String>,
    },
    VoicePcm { from: String, samples: Vec<f32> },
    Mix {
        layers: HashMap<String, f32>,
        blight: bool,
        place: String,
        time: String,
        inside: bool,
    },
    Error(String),
    Online { port: u16, addrs: Vec<String> },
    PeerSeen { id: String, name: String, addr: String },
    Image {
        from: String,
        name: String,
        to: Option<String>,
        crew: Option<String>,
        mime: String,
        data: Vec<u8>,
    },
    Video {
        from: String,
        name: String,
        action: String,
        to: Option<String>,
        crew: Option<String>,
    },
    VideoFrame {
        from: String,
        name: String,
        kind: String,
        to: Option<String>,
        crew: Option<String>,
        data: Vec<u8>,
    },
    NethookPut { hook: crate::nethook::Nethook },
    NethookDel { id: String, owner_id: String },
    NethookAsk,
    FileStart {
        from: String,
        name: String,
        to: Option<String>,
        crew: Option<String>,
        mime: String,
        filename: String,
        id: String,
        size: u64,
    },
    FileChunk { id: String, data: Vec<u8> },
    FileDone { id: String },
    MapMark { mark: crate::maps::Mark },
    MapMarkDel { id: String },
    MapMarks { marks: Vec<crate::maps::Mark> },
    MapMarksClear,
    MapMarksAsk,
}

enum Cmd {
    Host { internet: bool, root: std::path::PathBuf },
    Join(String),
    Leave,
    Send(Wire),
    Online,
    Dial(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub addr: String,
}

pub struct NetHub {
    pub role: Role,
    pub self_id: String,
    pub handle: String,
    pub peers: Vec<PeerInfo>,
    pub addrs: Vec<String>,
    pub public_url: String,
    pub port: u16,
    pub join_addr: String,
    pub internet: bool,
    pub presence: bool,
    pub seen: HashMap<String, String>,
    tx: Sender<Cmd>,
    rx: Receiver<NetEvent>,
    alive: Arc<AtomicBool>,
}

impl NetHub {
    pub fn new(handle: String, root: &std::path::Path) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (ev_tx, ev_rx) = mpsc::channel::<NetEvent>();
        let alive = Arc::new(AtomicBool::new(true));
        let self_id = load_peer_id(root);
        let hub_id = self_id.clone();
        let hub_name = handle.clone();
        let flag = alive.clone();
        thread::Builder::new()
            .name("blightnet-hub".into())
            .spawn(move || run_hub(hub_id, hub_name, cmd_rx, ev_tx, flag))
            .ok();
        Self {
            role: Role::Idle,
            self_id,
            handle,
            peers: vec![],
            addrs: vec![],
            public_url: String::new(),
            port: DEFAULT_PORT,
            join_addr: String::new(),
            internet: false,
            presence: false,
            seen: HashMap::new(),
            tx: cmd_tx,
            rx: ev_rx,
            alive,
        }
    }

    pub fn set_handle(&mut self, name: String) {
        self.handle = name;
    }

    pub fn host(&self, internet: bool, root: std::path::PathBuf) {
        let _ = self.tx.send(Cmd::Host { internet, root });
    }

    pub fn join(&self, addr: &str) {
        let _ = self.tx.send(Cmd::Join(addr.to_string()));
    }

    pub fn leave(&self) {
        let _ = self.tx.send(Cmd::Leave);
    }

    pub fn go_online(&self) {
        let _ = self.tx.send(Cmd::Online);
    }

    pub fn dial(&self, addr: &str) {
        if addr.is_empty() {
            return;
        }
        let _ = self.tx.send(Cmd::Dial(addr.to_string()));
    }

    pub fn send_nethook_put(&self, hook: crate::nethook::Nethook) {
        let _ = self.tx.send(Cmd::Send(Wire::NethookPut { hook }));
    }

    pub fn send_nethook_del(&self, id: &str, owner_id: &str) {
        let _ = self.tx.send(Cmd::Send(Wire::NethookDel {
            id: id.into(),
            owner_id: owner_id.into(),
        }));
    }

    pub fn send_nethook_ask(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::NethookAsk));
    }

    pub fn send_file(
        &self,
        to: Option<String>,
        crew: Option<String>,
        mime: &str,
        filename: &str,
        bytes: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        let id = format!("f{:08x}", rand::random::<u32>());
        let _ = self.tx.send(Cmd::Send(Wire::FileStart {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            to: to.clone(),
            crew,
            mime: mime.to_string(),
            filename: filename.to_string(),
            id: id.clone(),
            size: bytes.len() as u64,
        }));
        const CHUNK: usize = 12 * 1024;
        for part in bytes.chunks(CHUNK) {
            let _ = self.tx.send(Cmd::Send(Wire::FileChunk {
                id: id.clone(),
                to: to.clone(),
                data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, part),
            }));
        }
        let _ = self.tx.send(Cmd::Send(Wire::FileDone {
            id,
            to,
        }));
    }

    pub fn send_map_mark(&self, mark: crate::maps::Mark) {
        let _ = self.tx.send(Cmd::Send(Wire::MapMark { mark }));
    }

    pub fn send_map_mark_del(&self, id: &str) {
        let _ = self.tx.send(Cmd::Send(Wire::MapMarkDel { id: id.into() }));
    }

    pub fn send_map_marks(&self, marks: Vec<crate::maps::Mark>) {
        let _ = self.tx.send(Cmd::Send(Wire::MapMarks { marks }));
    }

    pub fn send_map_marks_clear(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::MapMarksClear));
    }

    pub fn send_map_marks_ask(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::MapMarksAsk));
    }

    pub fn send_image(&self, to: Option<String>, crew: Option<String>, mime: &str, bytes: &[u8]) {
        let _ = self.tx.send(Cmd::Send(Wire::Image {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            to,
            crew,
            mime: mime.to_string(),
            data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
        }));
    }

    pub fn is_seen(&self, id: &str) -> bool {
        id == self.self_id
            || self.peers.iter().any(|p| p.id == id)
            || self.seen.contains_key(id)
    }

    pub fn send_chat(&self, text: &str, to: Option<String>, whisper: bool) {
        let _ = self.tx.send(Cmd::Send(Wire::Chat {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            text: text.to_string(),
            whisper,
            to,
        }));
    }

    pub fn send_voice(&self, action: &str, to: Option<String>) {
        self.send_voice_ex(action, to, None);
    }

    pub fn send_voice_ex(&self, action: &str, to: Option<String>, crew: Option<String>) {
        let _ = self.tx.send(Cmd::Send(Wire::Voice {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            action: action.to_string(),
            to,
            crew,
        }));
    }

    pub fn send_video(&self, action: &str, to: Option<String>, crew: Option<String>) {
        let _ = self.tx.send(Cmd::Send(Wire::Video {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            action: action.to_string(),
            to,
            crew,
        }));
    }

    pub fn send_video_frame(&self, kind: &str, to: Option<String>, crew: Option<String>, jpeg: &[u8]) {
        if jpeg.is_empty() || jpeg.len() > 80_000 {
            return;
        }
        let _ = self.tx.send(Cmd::Send(Wire::VideoFrame {
            from: self.self_id.clone(),
            name: self.handle.clone(),
            kind: kind.to_string(),
            to,
            crew,
            data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, jpeg),
        }));
    }

    pub fn send_pcm(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let _ = self.tx.send(Cmd::Send(Wire::VoicePcm {
            from: self.self_id.clone(),
            pcm: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
        }));
    }

    pub fn send_mix(
        &self,
        layers: HashMap<String, f32>,
        blight: bool,
        place: String,
        time: String,
        inside: bool,
    ) {
        if self.role != Role::Host {
            return;
        }
        let _ = self.tx.send(Cmd::Send(Wire::Mix {
            layers,
            blight,
            place,
            time,
            inside,
        }));
    }

    pub fn poll(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(ev) => {
                    match &ev {
                        NetEvent::Hosting {
                            port,
                            addrs,
                            internet,
                        } => {
                            self.role = Role::Host;
                            self.presence = true;
                            self.port = *port;
                            self.addrs = addrs.clone();
                            self.internet = *internet;
                            if !internet {
                                self.public_url.clear();
                            }
                        }
                        NetEvent::Relay { url } => {
                            self.public_url = url.clone();
                            self.internet = true;
                        }
                        NetEvent::Joined { addr } => {
                            self.role = Role::Guest;
                            self.presence = true;
                            self.join_addr = addr.clone();
                        }
                        NetEvent::Left => {
                            self.role = Role::Idle;
                            self.presence = false;
                            self.peers.clear();
                            self.addrs.clear();
                            self.public_url.clear();
                            self.internet = false;
                            self.seen.clear();
                        }
                        NetEvent::Online { port, addrs } => {
                            self.presence = true;
                            if self.role == Role::Idle {
                                self.role = Role::Presence;
                            }
                            self.port = *port;
                            self.addrs = addrs.clone();
                        }
                        NetEvent::PeerSeen { id, addr, .. } => {
                            let fresh = !self.seen.contains_key(id);
                            self.seen.insert(id.clone(), addr.clone());
                            if fresh && !addr.is_empty() {
                                let _ = self.tx.send(Cmd::Dial(addr.clone()));
                            }
                        }
                        NetEvent::Peers(p) => {
                            for peer in p {
                                self.seen.insert(peer.id.clone(), String::new());
                            }
                            self.peers = p.clone();
                        }
                        _ => {}
                    }
                    out.push(ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn paste_link(&self) -> String {
        if !self.public_url.is_empty() {
            return self.public_url.clone();
        }
        let host = self
            .addrs
            .iter()
            .find(|a| !a.starts_with("127.") && !a.starts_with("[::"))
            .or_else(|| self.addrs.first())
            .cloned()
            .unwrap_or_else(|| format!("127.0.0.1:{}", self.port));
        format!("blightnet://{host}")
    }

    pub fn lan_link(&self) -> String {
        let host = self
            .addrs
            .iter()
            .find(|a| !a.starts_with("127.") && !a.starts_with("[::"))
            .or_else(|| self.addrs.first())
            .cloned()
            .unwrap_or_else(|| format!("127.0.0.1:{}", self.port));
        format!("blightnet://{host}")
    }
}

impl Drop for NetHub {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        let _ = self.tx.send(Cmd::Leave);
    }
}

fn advertised_addrs(port: u16) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        let _ = s.set_read_timeout(Some(Duration::from_millis(80)));
        if s.connect("1.1.1.1:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                let ip = addr.ip();
                if !ip.is_loopback() && !ip.is_unspecified() {
                    out.push(format!("{ip}:{port}"));
                }
            }
        }
    }
    out.push(format!("127.0.0.1:{port}"));
    out
}

pub fn looks_web(raw: &str) -> bool {
    let s = raw.trim().to_lowercase();
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("ws://")
        || s.starts_with("wss://")
        || s.contains("trycloudflare.com")
}

pub fn parse_addr(raw: &str) -> Option<String> {
    let s = raw.trim().trim_end_matches('/');
    if s.is_empty() {
        return None;
    }
    if looks_web(s) {
        return Some(s.to_string());
    }
    let s = s
        .trim_start_matches("blightnet://")
        .trim_start_matches("BLIGHTNET://");
    if s.is_empty() {
        return None;
    }
    if s.contains(':') {
        Some(s.to_string())
    } else {
        Some(format!("{s}:{DEFAULT_PORT}"))
    }
}

fn to_ws_url(raw: &str) -> String {
    let s = raw.trim().trim_end_matches('/');
    if let Some(rest) = s.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = s.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if s.starts_with("wss://") || s.starts_with("ws://") {
        s.to_string()
    } else if s.contains("trycloudflare.com") {
        format!("wss://{s}")
    } else {
        format!("ws://{s}")
    }
}

fn write_line(stream: &mut TcpStream, msg: &Wire) -> bool {
    let Ok(mut s) = serde_json::to_string(msg) else {
        return false;
    };
    s.push('\n');
    stream.write_all(s.as_bytes()).is_ok() && stream.flush().is_ok()
}

fn decode_pcm(b64: &str) -> Vec<f32> {
    let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) else {
        return vec![];
    };
    bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32767.0)
        .collect()
}

type ClientMap = Arc<Mutex<HashMap<String, SyncSender<Wire>>>>;

const WIRE_CAP: usize = 16;

fn wire_chan() -> (SyncSender<Wire>, Receiver<Wire>) {
    mpsc::sync_channel(WIRE_CAP)
}

fn push_wire(tx: &SyncSender<Wire>, msg: Wire) {
    let _ = tx.try_send(msg);
}

fn push_wire_hold(tx: &SyncSender<Wire>, msg: Wire) {
    let _ = tx.send(msg);
}

fn wire_hold(msg: &Wire) -> bool {
    matches!(
        msg,
        Wire::FileStart { .. }
            | Wire::FileChunk { .. }
            | Wire::FileDone { .. }
            | Wire::MapMark { .. }
            | Wire::MapMarkDel { .. }
            | Wire::MapMarks { .. }
            | Wire::MapMarksClear
            | Wire::MapMarksAsk
    )
}

fn relay_like_image(clients: &ClientMap, host_id: &str, to: &Option<String>, skip: Option<&str>, msg: &Wire) {
    if let Some(tid) = to {
        if tid != host_id {
            if let Ok(map) = clients.lock() {
                if let Some(tx) = map.get(tid) {
                    if wire_hold(msg) {
                        push_wire_hold(tx, msg.clone());
                    } else {
                        push_wire(tx, msg.clone());
                    }
                }
            }
        }
    } else if wire_hold(msg) {
        broadcast_hold(clients, msg, skip);
    } else {
        broadcast(clients, msg, skip);
    }
}

fn take_wires(wrx: &Receiver<Wire>) -> Vec<Wire> {
    let mut batch = Vec::new();
    while let Ok(m) = wrx.try_recv() {
        batch.push(m);
        if batch.len() >= 48 {
            break;
        }
    }
    coalesce_video(batch)
}

fn recv_batch(wrx: &Receiver<Wire>) -> Option<Vec<Wire>> {
    let first = wrx.recv().ok()?;
    let mut batch = vec![first];
    while let Ok(m) = wrx.try_recv() {
        batch.push(m);
        if batch.len() >= 48 {
            break;
        }
    }
    Some(coalesce_video(batch))
}

fn coalesce_video(batch: Vec<Wire>) -> Vec<Wire> {
    if batch.len() < 2 {
        return batch;
    }
    let mut last: HashMap<(String, String), usize> = HashMap::new();
    let mut keep = vec![true; batch.len()];
    for (i, m) in batch.iter().enumerate() {
        if let Wire::VideoFrame { from, kind, .. } = m {
            if let Some(prev) = last.insert((from.clone(), kind.clone()), i) {
                keep[prev] = false;
            }
        }
    }
    batch
        .into_iter()
        .zip(keep)
        .filter_map(|(m, k)| k.then_some(m))
        .collect()
}

fn run_hub(
    self_id: String,
    handle: String,
    cmd_rx: Receiver<Cmd>,
    ev_tx: Sender<NetEvent>,
    alive: Arc<AtomicBool>,
) {
    let mut stop = Arc::new(AtomicBool::new(false));
    let mut clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let mut guest_tx: Option<SyncSender<Wire>> = None;
    let mut names: HashMap<String, String> = HashMap::new();
    names.insert(self_id.clone(), handle.clone());
    let mut relay_child: Option<std::process::Child> = None;
    let mut listening = false;
    let mut listen_port = DEFAULT_PORT;

    while alive.load(Ordering::SeqCst) {
        let cmd = match cmd_rx.recv_timeout(Duration::from_millis(40)) {
            Ok(c) => c,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        match cmd {
            Cmd::Host { internet, root } => {
                stop.store(true, Ordering::SeqCst);
                stop = Arc::new(AtomicBool::new(false));
                clients = Arc::new(Mutex::new(HashMap::new()));
                guest_tx = None;
                if let Some(mut c) = relay_child.take() {
                    let _ = c.kill();
                }
                match TcpListener::bind(("0.0.0.0", DEFAULT_PORT))
                    .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))
                {
                    Ok(listener) => {
                        let _ = listener.set_nonblocking(true);
                        let port = listener.local_addr().map(|a| a.port()).unwrap_or(DEFAULT_PORT);
                        listen_port = port;
                        listening = true;
                        let addrs = advertised_addrs(port);
                        spawn_beacon(
                            self_id.clone(),
                            handle.clone(),
                            port,
                            stop.clone(),
                            ev_tx.clone(),
                        );
                        let _ = ev_tx.send(NetEvent::Hosting {
                            port,
                            addrs: addrs.clone(),
                            internet,
                        });
                        let _ = ev_tx.send(NetEvent::Status(if internet {
                            "Hosting · opening internet link…".into()
                        } else {
                            "Hosting · local network".into()
                        }));
                        spawn_accept(
                            listener,
                            clients.clone(),
                            ev_tx.clone(),
                            self_id.clone(),
                            handle.clone(),
                            stop.clone(),
                            names.clone(),
                        );
                        if internet {
                            relay_child = spawn_cloudflare(port, &root, ev_tx.clone(), stop.clone());
                        }
                    }
                    Err(e) => {
                        let _ = ev_tx.send(NetEvent::Error(format!("host: {e}")));
                    }
                }
            }
            Cmd::Join(addr) => {
                stop.store(true, Ordering::SeqCst);
                stop = Arc::new(AtomicBool::new(false));
                clients = Arc::new(Mutex::new(HashMap::new()));
                if let Some(mut c) = relay_child.take() {
                    let _ = c.kill();
                }
                let Some(target) = parse_addr(&addr) else {
                    let _ = ev_tx.send(NetEvent::Error(
                        "Paste a blightnet:// address or an https invite link.".into(),
                    ));
                    continue;
                };
                let (wtx, wrx) = wire_chan();
                guest_tx = Some(wtx.clone());
                let hello = Wire::Hello {
                    id: self_id.clone(),
                    name: handle.clone(),
                    role: "guest".into(),
                };
                if looks_web(&target) {
                    let ws_url = to_ws_url(&target);
                    match tungstenite::connect(&ws_url) {
                        Ok((mut ws, _)) => {
                            set_ws_timeout(&mut ws);
                            if ws_send(&mut ws, &hello) {
                                spawn_guest_ws(ws, wrx, ev_tx.clone(), stop.clone(), self_id.clone());
                                let _ = ev_tx.send(NetEvent::Joined { addr: target });
                                let _ = ev_tx.send(NetEvent::Status("Joined".into()));
                            } else {
                                let _ = ev_tx.send(NetEvent::Error(
                                    "Connected but could not say hello.".into(),
                                ));
                            }
                        }
                        Err(e) => {
                            let _ = ev_tx.send(NetEvent::Error(format!(
                                "Could not reach the table. {e}"
                            )));
                        }
                    }
                    continue;
                }
                let sock = target
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut it| it.next());
                let Some(sock) = sock else {
                    let _ = ev_tx.send(NetEvent::Error(format!("Bad address: {target}")));
                    continue;
                };
                match TcpStream::connect_timeout(&sock, Duration::from_secs(4)) {
                    Ok(stream) => {
                        let _ = stream.set_nodelay(true);
                        let mut s = stream.try_clone().ok();
                        if let Some(ref mut w) = s {
                            let _ = write_line(w, &hello);
                        }
                        spawn_guest(
                            stream,
                            wrx,
                            ev_tx.clone(),
                            stop.clone(),
                            self_id.clone(),
                        );
                        let _ = ev_tx.send(NetEvent::Joined { addr: target });
                        let _ = ev_tx.send(NetEvent::Status("Joined".into()));
                    }
                    Err(e) => {
                        let _ = ev_tx.send(NetEvent::Error(format!(
                            "Could not reach the table. {e}"
                        )));
                    }
                }
            }
            Cmd::Online => {
                if !listening {
                    match TcpListener::bind(("0.0.0.0", DEFAULT_PORT))
                        .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))
                    {
                        Ok(listener) => {
                            stop = Arc::new(AtomicBool::new(false));
                            let _ = listener.set_nonblocking(true);
                            let port =
                                listener.local_addr().map(|a| a.port()).unwrap_or(DEFAULT_PORT);
                            listen_port = port;
                            listening = true;
                            let addrs = advertised_addrs(port);
                            spawn_accept(
                                listener,
                                clients.clone(),
                                ev_tx.clone(),
                                self_id.clone(),
                                handle.clone(),
                                stop.clone(),
                                names.clone(),
                            );
                            spawn_beacon(
                                self_id.clone(),
                                handle.clone(),
                                port,
                                stop.clone(),
                                ev_tx.clone(),
                            );
                            let _ = ev_tx.send(NetEvent::Online { port, addrs });
                            let _ = ev_tx.send(NetEvent::Status("Online".into()));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(NetEvent::Error(format!("online: {e}")));
                        }
                    }
                } else {
                    spawn_beacon(
                        self_id.clone(),
                        handle.clone(),
                        listen_port,
                        stop.clone(),
                        ev_tx.clone(),
                    );
                    let _ = ev_tx.send(NetEvent::Online {
                        port: listen_port,
                        addrs: advertised_addrs(listen_port),
                    });
                    let _ = ev_tx.send(NetEvent::Status("Online".into()));
                }
            }
            Cmd::Dial(addr) => {
                let stop_c = stop.clone();
                let ev = ev_tx.clone();
                let sid = self_id.clone();
                let hname = handle.clone();
                let clients_c = clients.clone();
                thread::spawn(move || {
                    dial_peer(&addr, sid, hname, clients_c, ev, stop_c);
                });
            }
            Cmd::Leave => {
                stop.store(true, Ordering::SeqCst);
                listening = false;
                if let Some(mut c) = relay_child.take() {
                    let _ = c.kill();
                }
                if let Ok(mut map) = clients.lock() {
                    map.clear();
                }
                guest_tx = None;
                let _ = ev_tx.send(NetEvent::Left);
                let _ = ev_tx.send(NetEvent::Status("Offline".into()));
            }
            Cmd::Send(msg) => {
                match &msg {
                    Wire::Chat {
                        from,
                        name,
                        text,
                        whisper,
                        to,
                    } => {
                        let _ = ev_tx.send(NetEvent::Chat {
                            from: from.clone(),
                            name: name.clone(),
                            text: text.clone(),
                            whisper: *whisper,
                        });
                        let _ = (to, whisper);
                    }
                    _ => {}
                }
                let hold = wire_hold(&msg);
                if let Some(tx) = &guest_tx {
                    if hold {
                        push_wire_hold(tx, msg);
                    } else {
                        push_wire(tx, msg);
                    }
                } else if hold {
                    broadcast_hold(&clients, &msg, None);
                } else {
                    broadcast(&clients, &msg, None);
                }
            }
        }
    }
}

fn spawn_accept(
    listener: TcpListener,
    clients: ClientMap,
    ev_tx: Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
    _names: HashMap<String, String>,
) {
    thread::spawn(move || {
        let roster = Arc::new(Mutex::new(vec![PeerInfo {
            id: host_id.clone(),
            name: host_name.clone(),
        }]));
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nodelay(true);
                    let (wtx, wrx) = wire_chan();
                    let ev = ev_tx.clone();
                    let clients_c = clients.clone();
                    let roster_c = roster.clone();
                    let host_id_c = host_id.clone();
                    let host_name_c = host_name.clone();
                    let stop_c = stop.clone();
                    thread::spawn(move || {
                        if peek_http(&stream) {
                            handle_ws_client(
                                stream, wtx, wrx, clients_c, roster_c, ev, host_id_c, host_name_c,
                                stop_c,
                            );
                        } else {
                            handle_client(
                                stream, wtx, wrx, clients_c, roster_c, ev, host_id_c, host_name_c,
                                stop_c,
                            );
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(40));
                }
                Err(_) => thread::sleep(Duration::from_millis(80)),
            }
        }
    });
}

fn handle_client(
    stream: TcpStream,
    wtx: SyncSender<Wire>,
    wrx: Receiver<Wire>,
    clients: ClientMap,
    roster: Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
) {
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let hello: Wire = match serde_json::from_str(line.trim()) {
        Ok(w) => w,
        Err(_) => return,
    };
    let (cid, cname) = match hello {
        Wire::Hello { id, name, .. } => (id, name),
        _ => return,
    };
    {
        let mut r = roster.lock().unwrap();
        if !r.iter().any(|p| p.id == cid) {
            r.push(PeerInfo {
                id: cid.clone(),
                name: cname.clone(),
            });
        }
    }
    {
        let mut map = clients.lock().unwrap();
        map.insert(cid.clone(), wtx);
    }
    let peers = roster.lock().unwrap().clone();
    let _ = write_line(
        &mut writer,
        &Wire::Welcome {
            id: cid.clone(),
            role: "guest".into(),
            peers: peers.clone(),
        },
    );
    let _ = ev_tx.send(NetEvent::Peers(peers.clone()));
    let _ = ev_tx.send(NetEvent::Status(format!("{cname} jacked in")));
    broadcast(
        &clients,
        &Wire::Peers {
            peers: peers.clone(),
        },
        None,
    );

    let mut writer2 = writer.try_clone().ok();
    thread::spawn(move || {
        while let Some(batch) = recv_batch(&wrx) {
            let mut dead = false;
            if let Some(ref mut w) = writer2 {
                for msg in &batch {
                    if !write_line(w, msg) {
                        dead = true;
                        break;
                    }
                }
            } else {
                break;
            }
            if dead {
                break;
            }
        }
    });

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let Ok(msg) = serde_json::from_str::<Wire>(line.trim()) else {
                    continue;
                };
                match &msg {
                    Wire::Chat {
                        from,
                        name,
                        text,
                        whisper,
                        to,
                    } => {
                        let _ = ev_tx.send(NetEvent::Chat {
                            from: from.clone(),
                            name: name.clone(),
                            text: text.clone(),
                            whisper: *whisper,
                        });
                        if *whisper {
                            if let Some(tid) = to {
                                if tid == &host_id {
                                    // already sent to host via event
                                } else if let Ok(map) = clients.lock() {
                                    if let Some(tx) = map.get(tid) {
                                        push_wire(tx, msg.clone());
                                    }
                                }
                            }
                        } else {
                            broadcast(&clients, &msg, Some(&cid));
                        }
                    }
                    Wire::Voice {
                        from,
                        name,
                        action,
                        to,
                        crew,
                    } => {
                        let _ = ev_tx.send(NetEvent::Voice {
                            from: from.clone(),
                            name: name.clone(),
                            action: action.clone(),
                            to: to.clone(),
                            crew: crew.clone(),
                        });
                        relay_to(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::VoicePcm { from, pcm } => {
                        let _ = ev_tx.send(NetEvent::VoicePcm {
                            from: from.clone(),
                            samples: decode_pcm(pcm),
                        });
                        broadcast(&clients, &msg, Some(&cid));
                    }
                    Wire::Mix { .. } => broadcast(&clients, &msg, Some(&cid)),
                    Wire::Ping => {
                        let _ = write_line(&mut writer, &Wire::Pong);
                    }
                    Wire::Image {
                        from,
                        name,
                        to,
                        crew,
                        mime,
                        data,
                    } => {
                        let bytes = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            data,
                        )
                        .unwrap_or_default();
                        let _ = ev_tx.send(NetEvent::Image {
                            from: from.clone(),
                            name: name.clone(),
                            to: to.clone(),
                            crew: crew.clone(),
                            mime: mime.clone(),
                            data: bytes,
                        });
                        if let Some(tid) = to {
                            if tid != &host_id {
                                if let Ok(map) = clients.lock() {
                                    if let Some(tx) = map.get(tid) {
                                        push_wire(tx, msg.clone());
                                    }
                                }
                            }
                        } else {
                            broadcast(&clients, &msg, Some(&cid));
                        }
                    }
                    Wire::Video {
                        from,
                        name,
                        action,
                        to,
                        crew,
                    } => {
                        let _ = ev_tx.send(NetEvent::Video {
                            from: from.clone(),
                            name: name.clone(),
                            action: action.clone(),
                            to: to.clone(),
                            crew: crew.clone(),
                        });
                        relay_to(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::VideoFrame {
                        from,
                        name,
                        kind,
                        to,
                        crew,
                        data,
                    } => {
                        let bytes = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            data,
                        )
                        .unwrap_or_default();
                        let _ = ev_tx.send(NetEvent::VideoFrame {
                            from: from.clone(),
                            name: name.clone(),
                            kind: kind.clone(),
                            to: to.clone(),
                            crew: crew.clone(),
                            data: bytes,
                        });
                        relay_to(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::NethookPut { hook } => {
                        let _ = ev_tx.send(NetEvent::NethookPut { hook: hook.clone() });
                        broadcast(&clients, &msg, Some(&cid));
                    }
                    Wire::NethookDel { id, owner_id } => {
                        let _ = ev_tx.send(NetEvent::NethookDel {
                            id: id.clone(),
                            owner_id: owner_id.clone(),
                        });
                        broadcast(&clients, &msg, Some(&cid));
                    }
                    Wire::NethookAsk => {
                        let _ = ev_tx.send(NetEvent::NethookAsk);
                        broadcast(&clients, &msg, Some(&cid));
                    }
                    Wire::FileStart {
                        from,
                        name,
                        to,
                        crew,
                        mime,
                        filename,
                        id,
                        size,
                    } => {
                        let _ = ev_tx.send(NetEvent::FileStart {
                            from: from.clone(),
                            name: name.clone(),
                            to: to.clone(),
                            crew: crew.clone(),
                            mime: mime.clone(),
                            filename: filename.clone(),
                            id: id.clone(),
                            size: *size,
                        });
                        relay_like_image(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::FileChunk { id, to, data } => {
                        let bytes = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            data,
                        )
                        .unwrap_or_default();
                        let _ = ev_tx.send(NetEvent::FileChunk {
                            id: id.clone(),
                            data: bytes,
                        });
                        relay_like_image(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::FileDone { id, to } => {
                        let _ = ev_tx.send(NetEvent::FileDone { id: id.clone() });
                        relay_like_image(&clients, &host_id, to, Some(&cid), &msg);
                    }
                    Wire::MapMark { mark } => {
                        let _ = ev_tx.send(NetEvent::MapMark { mark: mark.clone() });
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapMarkDel { id } => {
                        let _ = ev_tx.send(NetEvent::MapMarkDel { id: id.clone() });
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapMarks { marks } => {
                        let _ = ev_tx.send(NetEvent::MapMarks {
                            marks: marks.clone(),
                        });
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapMarksClear => {
                        let _ = ev_tx.send(NetEvent::MapMarksClear);
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapMarksAsk => {
                        let _ = ev_tx.send(NetEvent::MapMarksAsk);
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    _ => {}
                }
                let _ = (host_name.clone(),);
            }
            Err(_) => break,
        }
    }
    {
        let mut map = clients.lock().unwrap();
        map.remove(&cid);
    }
    {
        let mut r = roster.lock().unwrap();
        r.retain(|p| p.id != cid);
        let peers = r.clone();
        drop(r);
        let _ = ev_tx.send(NetEvent::Peers(peers.clone()));
        broadcast(&clients, &Wire::Peers { peers }, None);
    }
}

fn spawn_guest(
    stream: TcpStream,
    wrx: Receiver<Wire>,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
    self_id: String,
) {
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    thread::spawn(move || {
        while let Some(batch) = recv_batch(&wrx) {
            let mut dead = false;
            for msg in &batch {
                if !write_line(&mut writer, msg) {
                    dead = true;
                    break;
                }
            }
            if dead {
                break;
            }
        }
    });
    thread::spawn(move || {
        let mut reader = BufReader::new(reader_stream);
        let mut line = String::new();
        while !stop.load(Ordering::SeqCst) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let Ok(msg) = serde_json::from_str::<Wire>(line.trim()) else {
                        continue;
                    };
                    guest_incoming(msg, &self_id, &ev_tx);
                }
                Err(_) => break,
            }
        }
        let _ = ev_tx.send(NetEvent::Left);
        let _ = ev_tx.send(NetEvent::Status("Offline".into()));
    });
}

fn broadcast(clients: &ClientMap, msg: &Wire, skip: Option<&str>) {
    if let Ok(map) = clients.lock() {
        for (id, tx) in map.iter() {
            if skip.map(|s| s == id).unwrap_or(false) {
                continue;
            }
            push_wire(tx, msg.clone());
        }
    }
}

fn broadcast_hold(clients: &ClientMap, msg: &Wire, skip: Option<&str>) {
    if let Ok(map) = clients.lock() {
        for (id, tx) in map.iter() {
            if skip.map(|s| s == id).unwrap_or(false) {
                continue;
            }
            push_wire_hold(tx, msg.clone());
        }
    }
}

pub fn load_contacts(root: &std::path::Path) -> Vec<Contact> {
    std::fs::read_to_string(root.join("data/contacts.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_contacts(root: &std::path::Path, rows: &[Contact]) {
    if let Ok(s) = serde_json::to_string_pretty(rows) {
        let _ = std::fs::write(root.join("data/contacts.json"), s);
    }
}

fn load_peer_id(root: &std::path::Path) -> String {
    let path = root.join("data/peer-id.txt");
    if let Ok(s) = std::fs::read_to_string(&path) {
        let t = s.trim().to_string();
        if t.starts_with("p-") && t.len() > 5 {
            return t;
        }
    }
    let id = format!(
        "p-{}{:04}",
        rand::random::<u32>(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 10000)
            .unwrap_or(1)
    );
    let _ = std::fs::create_dir_all(root.join("data"));
    let _ = std::fs::write(path, &id);
    id
}

const UDP_PORT: u16 = 8768;

fn spawn_beacon(
    self_id: String,
    handle: String,
    tcp_port: u16,
    stop: Arc<AtomicBool>,
    ev_tx: Sender<NetEvent>,
) {
    thread::spawn(move || {
        let sock = UdpSocket::bind(("0.0.0.0", UDP_PORT))
            .or_else(|_| UdpSocket::bind("0.0.0.0:0"));
        let Ok(sock) = sock else {
            return;
        };
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(400)));
        let pkt = format!("BN|{self_id}|{handle}|{tcp_port}");
        let mut buf = [0u8; 512];
        while !stop.load(Ordering::SeqCst) {
            let _ = sock.send_to(pkt.as_bytes(), ("255.255.255.255", UDP_PORT));
            if let Ok((n, from)) = sock.recv_from(&mut buf) {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    if let Some((id, name, port)) = parse_beacon(text) {
                        if id != self_id {
                            let addr = format!("{}:{port}", from.ip());
                            let _ = ev_tx.send(NetEvent::PeerSeen { id, name, addr });
                        }
                    }
                }
            }
        }
    });
}

fn parse_beacon(text: &str) -> Option<(String, String, u16)> {
    let mut parts = text.trim().split('|');
    if parts.next()? != "BN" {
        return None;
    }
    let id = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    let port = parts.next()?.parse().ok()?;
    Some((id, name, port))
}

fn dial_peer(
    addr: &str,
    self_id: String,
    handle: String,
    clients: ClientMap,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
) {
    let Some(target) = parse_addr(addr) else {
        return;
    };
    let hello = Wire::Hello {
        id: self_id.clone(),
        name: handle,
        role: "presence".into(),
    };
    if looks_web(&target) {
        if let Ok((mut ws, _)) = tungstenite::connect(to_ws_url(&target)) {
            set_ws_timeout(&mut ws);
            if !ws_send(&mut ws, &hello) {
                return;
            }
            let (wtx, wrx) = wire_chan();
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                for msg in take_wires(&wrx) {
                    let _ = ws_send(&mut ws, &msg);
                }
                match ws_recv(&mut ws) {
                    Ok(Some(Wire::Welcome { id, .. })) => {
                        if let Ok(mut map) = clients.lock() {
                            map.insert(id.clone(), wtx.clone());
                        }
                        let _ = ev_tx.send(NetEvent::PeerSeen {
                            id,
                            name: String::new(),
                            addr: target.clone(),
                        });
                    }
                    Ok(Some(msg)) => guest_incoming(msg, &self_id, &ev_tx),
                    Ok(None) => {}
                    Err(()) => break,
                }
            }
        }
        return;
    }
    let Some(sock) = target.to_socket_addrs().ok().and_then(|mut it| it.next()) else {
        return;
    };
    let Ok(stream) = TcpStream::connect_timeout(&sock, Duration::from_secs(3)) else {
        return;
    };
    let _ = stream.set_nodelay(true);
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    if !write_line(&mut writer, &hello) {
        return;
    };
    let (wtx, wrx) = wire_chan();
    let mut w2 = writer.try_clone().ok();
    thread::spawn(move || {
        while let Some(batch) = recv_batch(&wrx) {
            let mut dead = false;
            if let Some(ref mut w) = w2 {
                for msg in &batch {
                    if !write_line(w, msg) {
                        dead = true;
                        break;
                    }
                }
            } else {
                break;
            }
            if dead {
                break;
            }
        }
    });
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while !stop.load(Ordering::SeqCst) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let Ok(msg) = serde_json::from_str::<Wire>(line.trim()) else {
                    continue;
                };
                if let Wire::Welcome { id, .. } = &msg {
                    if let Ok(mut map) = clients.lock() {
                        map.insert(id.clone(), wtx.clone());
                    }
                    let _ = ev_tx.send(NetEvent::PeerSeen {
                        id: id.clone(),
                        name: String::new(),
                        addr: target.clone(),
                    });
                }
                guest_incoming(msg, &self_id, &ev_tx);
            }
            Err(_) => break,
        }
    }
}

fn set_ws_timeout(
    ws: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) {
    match ws.get_mut() {
        tungstenite::stream::MaybeTlsStream::NativeTls(t) => {
            let _ = t
                .get_mut()
                .set_read_timeout(Some(Duration::from_millis(80)));
        }
        tungstenite::stream::MaybeTlsStream::Plain(t) => {
            let _ = t.set_read_timeout(Some(Duration::from_millis(80)));
        }
        _ => {}
    }
}

fn peek_http(stream: &TcpStream) -> bool {
    let mut buf = [0u8; 4];
    matches!(stream.peek(&mut buf), Ok(n) if n >= 3 && buf.starts_with(b"GET"))
}

fn ws_send<S: std::io::Read + std::io::Write>(
    ws: &mut tungstenite::WebSocket<S>,
    msg: &Wire,
) -> bool {
    let Ok(s) = serde_json::to_string(msg) else {
        return false;
    };
    ws.send(tungstenite::Message::Text(s.into())).is_ok()
}

fn ws_recv<S: std::io::Read + std::io::Write>(
    ws: &mut tungstenite::WebSocket<S>,
) -> Result<Option<Wire>, ()> {
    match ws.read() {
        Ok(tungstenite::Message::Text(t)) => Ok(serde_json::from_str(t.as_str()).ok()),
        Ok(tungstenite::Message::Ping(p)) => {
            let _ = ws.send(tungstenite::Message::Pong(p));
            Ok(None)
        }
        Ok(tungstenite::Message::Close(_)) => Err(()),
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(e))
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            Ok(None)
        }
        Err(_) => Err(()),
    }
}

fn handle_ws_client(
    stream: TcpStream,
    wtx: SyncSender<Wire>,
    wrx: Receiver<Wire>,
    clients: ClientMap,
    roster: Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(80)));
    let Ok(mut ws) = tungstenite::accept(stream) else {
        return;
    };
    let hello = loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match ws_recv(&mut ws) {
            Ok(Some(Wire::Hello { id, name, .. })) => break (id, name),
            Ok(_) => continue,
            Err(()) => return,
        }
    };
    let (cid, cname) = hello;
    let cid_keep = cid.clone();
    finish_join(
        cid,
        cname,
        wtx,
        &clients,
        &roster,
        &ev_tx,
        &host_id,
        &host_name,
    );
    pump_ws_host(
        &mut ws,
        wrx,
        &clients,
        &roster,
        &ev_tx,
        &host_id,
        &cid_keep,
        &stop,
    );
}

fn finish_join(
    cid: String,
    cname: String,
    wtx: SyncSender<Wire>,
    clients: &ClientMap,
    roster: &Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: &Sender<NetEvent>,
    _host_id: &str,
    _host_name: &str,
) {
    {
        let mut r = roster.lock().unwrap();
        if !r.iter().any(|p| p.id == cid) {
            r.push(PeerInfo {
                id: cid.clone(),
                name: cname.clone(),
            });
        }
    }
    {
        let mut map = clients.lock().unwrap();
        map.insert(cid.clone(), wtx);
    }
    let peers = roster.lock().unwrap().clone();
    let _ = ev_tx.send(NetEvent::Peers(peers.clone()));
    let _ = ev_tx.send(NetEvent::Status(format!("{cname} jacked in")));
    broadcast(clients, &Wire::Peers { peers: peers.clone() }, Some(&cid));
    if let Ok(map) = clients.lock() {
        if let Some(tx) = map.get(&cid) {
            push_wire(
                tx,
                Wire::Welcome {
                    id: cid,
                    role: "guest".into(),
                    peers,
                },
            );
        }
    }
}

fn pump_ws_host<S: std::io::Read + std::io::Write>(
    ws: &mut tungstenite::WebSocket<S>,
    wrx: Receiver<Wire>,
    clients: &ClientMap,
    roster: &Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: &Sender<NetEvent>,
    host_id: &str,
    cid: &str,
    stop: &AtomicBool,
) {
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        for msg in take_wires(&wrx) {
            if !ws_send(ws, &msg) {
                drop_peer(cid, clients, roster, ev_tx);
                return;
            }
        }
        match ws_recv(ws) {
            Ok(Some(msg)) => host_incoming(msg, cid, host_id, clients, ev_tx, ws),
            Ok(None) => {}
            Err(()) => break,
        }
    }
    drop_peer(cid, clients, roster, ev_tx);
}

fn host_incoming<S: std::io::Read + std::io::Write>(
    msg: Wire,
    cid: &str,
    host_id: &str,
    clients: &ClientMap,
    ev_tx: &Sender<NetEvent>,
    ws: &mut tungstenite::WebSocket<S>,
) {
    match &msg {
        Wire::Chat {
            from,
            name,
            text,
            whisper,
            to,
        } => {
            let _ = ev_tx.send(NetEvent::Chat {
                from: from.clone(),
                name: name.clone(),
                text: text.clone(),
                whisper: *whisper,
            });
            if *whisper {
                if let Some(tid) = to {
                    if tid != host_id {
                        if let Ok(map) = clients.lock() {
                            if let Some(tx) = map.get(tid) {
                                push_wire(tx, msg.clone());
                            }
                        }
                    }
                }
            } else {
                broadcast(clients, &msg, Some(cid));
            }
        }
        Wire::Voice {
            from,
            name,
            action,
            to,
            crew,
        } => {
            let _ = ev_tx.send(NetEvent::Voice {
                from: from.clone(),
                name: name.clone(),
                action: action.clone(),
                to: to.clone(),
                crew: crew.clone(),
            });
            relay_to(clients, host_id, to, Some(cid), &msg);
        }
        Wire::VoicePcm { from, pcm } => {
            let _ = ev_tx.send(NetEvent::VoicePcm {
                from: from.clone(),
                samples: decode_pcm(pcm),
            });
            broadcast(clients, &msg, Some(cid));
        }
        Wire::Mix { .. } => broadcast(clients, &msg, Some(cid)),
        Wire::Ping => {
            let _ = ws_send(ws, &Wire::Pong);
        }
        Wire::Image {
            from,
            name,
            to,
            crew,
            mime,
            data,
        } => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                .unwrap_or_default();
            let _ = ev_tx.send(NetEvent::Image {
                from: from.clone(),
                name: name.clone(),
                to: to.clone(),
                crew: crew.clone(),
                mime: mime.clone(),
                data: bytes,
            });
            if let Some(tid) = to {
                if tid != host_id {
                    if let Ok(map) = clients.lock() {
                        if let Some(tx) = map.get(tid) {
                            push_wire(tx, msg.clone());
                        }
                    }
                }
            } else {
                broadcast(clients, &msg, Some(cid));
            }
        }
        Wire::Video {
            from,
            name,
            action,
            to,
            crew,
        } => {
            let _ = ev_tx.send(NetEvent::Video {
                from: from.clone(),
                name: name.clone(),
                action: action.clone(),
                to: to.clone(),
                crew: crew.clone(),
            });
            relay_to(clients, host_id, to, Some(cid), &msg);
        }
        Wire::VideoFrame {
            from,
            name,
            kind,
            to,
            crew,
            data,
        } => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                .unwrap_or_default();
            let _ = ev_tx.send(NetEvent::VideoFrame {
                from: from.clone(),
                name: name.clone(),
                kind: kind.clone(),
                to: to.clone(),
                crew: crew.clone(),
                data: bytes,
            });
            relay_to(clients, host_id, to, Some(cid), &msg);
        }
        Wire::NethookPut { hook } => {
            let _ = ev_tx.send(NetEvent::NethookPut { hook: hook.clone() });
            broadcast(clients, &msg, Some(cid));
        }
        Wire::NethookDel { id, owner_id } => {
            let _ = ev_tx.send(NetEvent::NethookDel {
                id: id.clone(),
                owner_id: owner_id.clone(),
            });
            broadcast(clients, &msg, Some(cid));
        }
        Wire::NethookAsk => {
            let _ = ev_tx.send(NetEvent::NethookAsk);
            broadcast(clients, &msg, Some(cid));
        }
        Wire::FileStart {
            from,
            name,
            to,
            crew,
            mime,
            filename,
            id,
            size,
        } => {
            let _ = ev_tx.send(NetEvent::FileStart {
                from: from.clone(),
                name: name.clone(),
                to: to.clone(),
                crew: crew.clone(),
                mime: mime.clone(),
                filename: filename.clone(),
                id: id.clone(),
                size: *size,
            });
            relay_like_image(clients, host_id, to, Some(cid), &msg);
        }
        Wire::FileChunk { id, to, data } => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                .unwrap_or_default();
            let _ = ev_tx.send(NetEvent::FileChunk {
                id: id.clone(),
                data: bytes,
            });
            relay_like_image(clients, host_id, to, Some(cid), &msg);
        }
        Wire::FileDone { id, to } => {
            let _ = ev_tx.send(NetEvent::FileDone { id: id.clone() });
            relay_like_image(clients, host_id, to, Some(cid), &msg);
        }
        Wire::MapMark { mark } => {
            let _ = ev_tx.send(NetEvent::MapMark { mark: mark.clone() });
            broadcast_hold(clients, &msg, Some(cid));
        }
        Wire::MapMarkDel { id } => {
            let _ = ev_tx.send(NetEvent::MapMarkDel { id: id.clone() });
            broadcast_hold(clients, &msg, Some(cid));
        }
        Wire::MapMarks { marks } => {
            let _ = ev_tx.send(NetEvent::MapMarks {
                marks: marks.clone(),
            });
            broadcast_hold(clients, &msg, Some(cid));
        }
        Wire::MapMarksClear => {
            let _ = ev_tx.send(NetEvent::MapMarksClear);
            broadcast_hold(clients, &msg, Some(cid));
        }
        Wire::MapMarksAsk => {
            let _ = ev_tx.send(NetEvent::MapMarksAsk);
            broadcast_hold(clients, &msg, Some(cid));
        }
        _ => {}
    }
}

fn relay_to(clients: &ClientMap, host_id: &str, to: &Option<String>, skip: Option<&str>, msg: &Wire) {
    if let Some(tid) = to {
        if tid != host_id {
            if let Ok(map) = clients.lock() {
                if let Some(tx) = map.get(tid) {
                    push_wire(tx, msg.clone());
                }
            }
        }
    } else {
        broadcast(clients, msg, skip);
    }
}

fn drop_peer(
    cid: &str,
    clients: &ClientMap,
    roster: &Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: &Sender<NetEvent>,
) {
    if let Ok(mut map) = clients.lock() {
        map.remove(cid);
    }
    if let Ok(mut r) = roster.lock() {
        r.retain(|p| p.id != cid);
        let peers = r.clone();
        drop(r);
        let _ = ev_tx.send(NetEvent::Peers(peers.clone()));
        broadcast(clients, &Wire::Peers { peers }, None);
    }
}

fn spawn_guest_ws<S>(
    mut ws: tungstenite::WebSocket<S>,
    wrx: Receiver<Wire>,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
    self_id: String,
) where
    S: std::io::Read + std::io::Write + Send + 'static,
{
    thread::spawn(move || {
        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            for msg in take_wires(&wrx) {
                if !ws_send(&mut ws, &msg) {
                    let _ = ev_tx.send(NetEvent::Left);
                    return;
                }
            }
            match ws_recv(&mut ws) {
                Ok(Some(msg)) => guest_incoming(msg, &self_id, &ev_tx),
                Ok(None) => {}
                Err(()) => break,
            }
        }
        let _ = ev_tx.send(NetEvent::Left);
        let _ = ev_tx.send(NetEvent::Status("Offline".into()));
    });
}

fn guest_incoming(msg: Wire, self_id: &str, ev_tx: &Sender<NetEvent>) {
    match msg {
        Wire::Welcome { peers, .. } | Wire::Peers { peers } => {
            let _ = ev_tx.send(NetEvent::Peers(peers));
        }
        Wire::Chat {
            from,
            name,
            text,
            whisper,
            ..
        } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::Chat {
                    from,
                    name,
                    text,
                    whisper,
                });
            }
        }
        Wire::Voice {
            from,
            name,
            action,
            to,
            crew,
        } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::Voice {
                    from,
                    name,
                    action,
                    to,
                    crew,
                });
            }
        }
        Wire::VoicePcm { from, pcm } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::VoicePcm {
                    from,
                    samples: decode_pcm(&pcm),
                });
            }
        }
        Wire::Mix {
            layers,
            blight,
            place,
            time,
            inside,
        } => {
            let _ = ev_tx.send(NetEvent::Mix {
                layers,
                blight,
                place,
                time,
                inside,
            });
        }
        Wire::Image {
            from,
            name,
            to,
            crew,
            mime,
            data,
        } => {
            if from != self_id {
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                    .unwrap_or_default();
                let _ = ev_tx.send(NetEvent::Image {
                    from,
                    name,
                    to,
                    crew,
                    mime,
                    data: bytes,
                });
            }
        }
        Wire::Video {
            from,
            name,
            action,
            to,
            crew,
        } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::Video {
                    from,
                    name,
                    action,
                    to,
                    crew,
                });
            }
        }
        Wire::VideoFrame {
            from,
            name,
            kind,
            to,
            crew,
            data,
        } => {
            if from != self_id {
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                    .unwrap_or_default();
                let _ = ev_tx.send(NetEvent::VideoFrame {
                    from,
                    name,
                    kind,
                    to,
                    crew,
                    data: bytes,
                });
            }
        }
        Wire::NethookPut { hook } => {
            if hook.owner_id != self_id {
                let _ = ev_tx.send(NetEvent::NethookPut { hook });
            }
        }
        Wire::NethookDel { id, owner_id } => {
            let _ = ev_tx.send(NetEvent::NethookDel { id, owner_id });
        }
        Wire::NethookAsk => {
            let _ = ev_tx.send(NetEvent::NethookAsk);
        }
        Wire::FileStart {
            from,
            name,
            to,
            crew,
            mime,
            filename,
            id,
            size,
        } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::FileStart {
                    from,
                    name,
                    to,
                    crew,
                    mime,
                    filename,
                    id,
                    size,
                });
            }
        }
        Wire::FileChunk { id, data, .. } => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                .unwrap_or_default();
            let _ = ev_tx.send(NetEvent::FileChunk { id, data: bytes });
        }
        Wire::FileDone { id, .. } => {
            let _ = ev_tx.send(NetEvent::FileDone { id });
        }
        Wire::MapMark { mark } => {
            let _ = ev_tx.send(NetEvent::MapMark { mark });
        }
        Wire::MapMarkDel { id } => {
            let _ = ev_tx.send(NetEvent::MapMarkDel { id });
        }
        Wire::MapMarks { marks } => {
            let _ = ev_tx.send(NetEvent::MapMarks { marks });
        }
        Wire::MapMarksClear => {
            let _ = ev_tx.send(NetEvent::MapMarksClear);
        }
        Wire::MapMarksAsk => {
            let _ = ev_tx.send(NetEvent::MapMarksAsk);
        }
        _ => {}
    }
}

fn pick_relay_url(text: &str) -> Option<String> {
    for raw in text.split_whitespace() {
        let mut url = raw.trim_matches(|c: char| "<>.,;\"'()[]".contains(c)).to_string();
        if url.starts_with("http://") {
            url = format!("https://{}", &url[7..]);
        }
        if !url.starts_with("https://") {
            continue;
        }
        let host = url
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if host.is_empty()
            || host == "github.com"
            || host == "localhost"
            || host == "127.0.0.1"
            || host.ends_with(".github.com")
            || host.ends_with(".google.com")
        {
            continue;
        }
        return Some(url.trim_end_matches('/').to_string());
    }
    None
}

fn cloudflared_bin(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let names = if cfg!(windows) {
        ["cloudflared.exe", "cloudflared"]
    } else {
        ["cloudflared", "cloudflared.exe"]
    };
    if let Some(p) = crate::sys::which("cloudflared") {
        return Some(p);
    }
    for n in names {
        let p = root.join(n);
        if p.is_file() {
            return Some(p);
        }
    }
    let local = if cfg!(windows) {
        root.join("cloudflared.exe")
    } else {
        root.join("cloudflared")
    };
    let url = if cfg!(windows) {
        "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-amd64.tgz"
    } else {
        "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64"
    };
    if !crate::sys::download(url, &local) {
        let _ = std::fs::remove_file(&local);
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&local) {
            let mut p = meta.permissions();
            p.set_mode(0o755);
            let _ = std::fs::set_permissions(&local, p);
        }
    }
    Some(local)
}

fn spawn_cloudflare(
    port: u16,
    root: &std::path::Path,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
) -> Option<std::process::Child> {
    let bin = match cloudflared_bin(root) {
        Some(b) => b,
        None => {
            let _ = ev_tx.send(NetEvent::Error(
                "No cloudflared. Internet Host needs Cloudflare's tunnel binary.".into(),
            ));
            return None;
        }
    };
    let local = format!("http://127.0.0.1:{port}");
    let mut cmd = std::process::Command::new(&bin);
    crate::sys::hide(&mut cmd);
    cmd.args(["tunnel", "--no-autoupdate", "--url", &local])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = ev_tx.send(NetEvent::Error(format!("cloudflared: {e}")));
            return None;
        }
    };
    if let Some(out) = child.stderr.take() {
        let ev = ev_tx.clone();
        let halt = stop.clone();
        thread::spawn(move || {
            use std::io::Read;
            let mut r = BufReader::new(out);
            let mut buf = String::new();
            let mut chunk = [0u8; 512];
            while !halt.load(Ordering::SeqCst) {
                match r.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                        if buf.len() > 8000 {
                            buf = buf[buf.len() - 4000..].to_string();
                        }
                        if let Some(url) = pick_relay_url(&buf) {
                            let _ = ev.send(NetEvent::Relay { url: url.clone() });
                            let _ = ev.send(NetEvent::Status("Hosting · internet link ready".into()));
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(out) = child.stdout.take() {
        let ev = ev_tx.clone();
        let halt = stop.clone();
        thread::spawn(move || {
            use std::io::Read;
            let mut r = BufReader::new(out);
            let mut buf = String::new();
            let mut chunk = [0u8; 512];
            while !halt.load(Ordering::SeqCst) {
                match r.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                        if let Some(url) = pick_relay_url(&buf) {
                            let _ = ev.send(NetEvent::Relay { url });
                            let _ = ev.send(NetEvent::Status("Hosting · internet link ready".into()));
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
    let _ = ev_tx.send(NetEvent::Status(
        "Hosting · waiting on Cloudflare invite…".into(),
    ));
    Some(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_accepts_lan_and_https() {
        assert_eq!(
            parse_addr("blightnet://10.0.0.4:8766").as_deref(),
            Some("10.0.0.4:8766")
        );
        assert_eq!(parse_addr("192.168.1.9").as_deref(), Some("192.168.1.9:8766"));
        assert!(looks_web("https://foo.trycloudflare.com"));
        assert_eq!(
            parse_addr("https://foo.trycloudflare.com").as_deref(),
            Some("https://foo.trycloudflare.com")
        );
        assert!(parse_addr("").is_none());
        assert_eq!(to_ws_url("https://foo.trycloudflare.com"), "wss://foo.trycloudflare.com");
        assert_eq!(to_ws_url("http://127.0.0.1:8766"), "ws://127.0.0.1:8766");
    }

    #[test]
    fn wire_roundtrip_chat_voice_video() {
        for msg in [
            Wire::Chat {
                from: "a".into(),
                name: "Ada".into(),
                text: "hi".into(),
                whisper: true,
                to: Some("b".into()),
            },
            Wire::Voice {
                from: "a".into(),
                name: "Ada".into(),
                action: "invite".into(),
                to: Some("b".into()),
                crew: None,
            },
            Wire::Video {
                from: "a".into(),
                name: "Ada".into(),
                action: "invite".into(),
                to: Some("b".into()),
                crew: Some("crew-1".into()),
            },
            Wire::VideoFrame {
                from: "a".into(),
                name: "Ada".into(),
                kind: "cam".into(),
                to: Some("b".into()),
                crew: None,
                data: "abcd".into(),
            },
        ] {
            let s = serde_json::to_string(&msg).unwrap();
            let back: Wire = serde_json::from_str(&s).unwrap();
            let s2 = serde_json::to_string(&back).unwrap();
            assert_eq!(s, s2);
        }
    }

    #[test]
    fn coalesce_keeps_latest_video_frame() {
        let batch = vec![
            Wire::Chat {
                from: "a".into(),
                name: "A".into(),
                text: "x".into(),
                whisper: false,
                to: None,
            },
            Wire::VideoFrame {
                from: "p".into(),
                name: "P".into(),
                kind: "cam".into(),
                to: None,
                crew: None,
                data: "old".into(),
            },
            Wire::VideoFrame {
                from: "p".into(),
                name: "P".into(),
                kind: "cam".into(),
                to: None,
                crew: None,
                data: "new".into(),
            },
        ];
        let out = coalesce_video(batch);
        assert_eq!(out.len(), 2);
        match &out[1] {
            Wire::VideoFrame { data, .. } => assert_eq!(data, "new"),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn advertised_addrs_includes_loopback() {
        let a = advertised_addrs(8766);
        assert!(a.iter().any(|x| x.contains("127.0.0.1:8766")));
    }

    #[test]
    fn host_join_and_chat_over_lan() {
        let a = std::env::temp_dir().join(format!("bn-host-{}", rand::random::<u32>()));
        let b = std::env::temp_dir().join(format!("bn-guest-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let mut host = NetHub::new("Host".into(), &a);
        host.host(false, a.clone());
        let mut port = 0u16;
        for _ in 0..80 {
            for ev in host.poll() {
                if let NetEvent::Hosting { port: p, .. } = ev {
                    port = p;
                }
            }
            if host.role == Role::Host && port != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(matches!(host.role, Role::Host));
        assert!(port != 0);
        let mut guest = NetHub::new("Guest".into(), &b);
        guest.join(&format!("127.0.0.1:{port}"));
        let mut joined = false;
        for _ in 0..80 {
            let _ = host.poll();
            for ev in guest.poll() {
                if matches!(ev, NetEvent::Joined { .. }) {
                    joined = true;
                }
            }
            if guest.role == Role::Guest {
                joined = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(joined, "guest did not join");
        guest.send_chat("ping from guest", None, false);
        let mut saw = false;
        for _ in 0..80 {
            for ev in host.poll() {
                if let NetEvent::Chat { text, .. } = ev {
                    if text.contains("ping from guest") {
                        saw = true;
                    }
                }
            }
            if saw {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        drop(guest);
        drop(host);
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
        assert!(saw, "host did not receive guest chat");
    }
}
