use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 8766;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    #[serde(rename = "map-tokens")]
    MapTokens { tokens: Vec<crate::maps::MapTok> },
    #[serde(rename = "map-tokens-ask")]
    MapTokensAsk,
    #[serde(rename = "sheet")]
    Sheet {
        from: String,
        chars: Vec<crate::chars::Character>,
    },
    #[serde(rename = "sheet-ask")]
    SheetAsk,
    #[serde(rename = "map-image-ask")]
    MapImageAsk,
}

pub enum NetEvent {
    Status(String),
    Hosting {
        port: u16,
        addrs: Vec<String>,
        internet: bool,
        key: Vec<u8>,
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
    MapTokens { tokens: Vec<crate::maps::MapTok> },
    MapTokensAsk,
    Sheet {
        from: String,
        chars: Vec<crate::chars::Character>,
    },
    SheetAsk,
    MapImageAsk,
}

pub(crate) enum Cmd {
    Host { internet: bool, root: std::path::PathBuf },
    Join(String),
    Leave,
    Send(Wire),
    Online,
    Dial(String),
    SetHandle(String),
    RefreshInvite,
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
    pub table_key: Vec<u8>,
    pub daemon: bool,
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
            table_key: vec![],
            daemon: false,
            tx: cmd_tx,
            rx: ev_rx,
            alive,
        }
    }

    pub fn attach(handle: String, root: &std::path::Path) -> Self {
        match crate::daemon::connect_or_spawn(root, &handle) {
            Ok(stream) => Self::from_ipc(handle, root, stream),
            Err(_) => Self::new(handle, root),
        }
    }

    fn from_ipc(handle: String, root: &std::path::Path, stream: TcpStream) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (ev_tx, ev_rx) = mpsc::channel::<NetEvent>();
        let alive = Arc::new(AtomicBool::new(true));
        let self_id = load_peer_id(root);
        let flag = alive.clone();
        thread::Builder::new()
            .name("blightnet-ipc".into())
            .spawn(move || crate::daemon::run_client(stream, cmd_rx, ev_tx, flag))
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
            table_key: vec![],
            daemon: true,
            tx: cmd_tx,
            rx: ev_rx,
            alive,
        }
    }

    pub fn set_handle(&mut self, name: String) {
        if self.handle != name {
            let _ = self.tx.send(Cmd::SetHandle(name.clone()));
        }
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

    pub fn send_map_tokens(&self, tokens: Vec<crate::maps::MapTok>) {
        let _ = self.tx.send(Cmd::Send(Wire::MapTokens { tokens }));
    }

    pub fn send_map_tokens_ask(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::MapTokensAsk));
    }

    pub fn send_sheet(&self, chars: Vec<crate::chars::Character>) {
        let _ = self.tx.send(Cmd::Send(Wire::Sheet {
            from: self.self_id.clone(),
            chars,
        }));
    }

    pub fn send_sheet_ask(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::SheetAsk));
    }

    pub fn send_map_image_ask(&self) {
        let _ = self.tx.send(Cmd::Send(Wire::MapImageAsk));
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
                            key,
                        } => {
                            self.role = Role::Host;
                            self.presence = true;
                            self.port = *port;
                            self.addrs = addrs.clone();
                            self.internet = *internet;
                            self.table_key = key.clone();
                            if !internet {
                                self.public_url = crate::crypt::encode_invite(key, addrs);
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
                            self.table_key.clear();
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
        crate::crypt::encode_invite(&self.table_key, &self.invite_addrs())
    }

    pub fn lan_link(&self) -> String {
        crate::crypt::encode_invite(&self.table_key, &self.invite_addrs())
    }

    fn invite_addrs(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .addrs
            .iter()
            .filter(|a| !a.starts_with("127.") && !a.starts_with("[::"))
            .cloned()
            .collect();
        if v.is_empty() {
            v.push(format!("127.0.0.1:{}", self.port));
        }
        v
    }
}

impl Drop for NetHub {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        // The node keeps the table when the window closes. Only an in-process
        // hub (tests, fallback) should tear the link down here.
        if !self.daemon {
            let _ = self.tx.send(Cmd::Leave);
        }
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

pub fn parse_addr(raw: &str) -> Option<String> {
    let s = raw
        .trim()
        .trim_start_matches('\u{feff}')
        .trim_end_matches(|c: char| c == '/' || c == '\r' || c.is_whitespace());
    if s.is_empty() {
        return None;
    }
    let lower = s.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("ws://") {
        return None;
    }
    let s = s
        .trim_start_matches("blightnet://")
        .trim_start_matches("BLIGHTNET://")
        .trim_start_matches("blightnet:/")
        .trim_start_matches("blightnet:");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let s = if let Some((k, rest)) = s.split_once('@') {
        if crate::crypt::decode_key(k).is_some() {
            rest.split(',').next().unwrap_or(rest)
        } else {
            s
        }
    } else {
        s.split(',').next().unwrap_or(s)
    };
    let s = s.trim();
    if s.contains(':') {
        Some(s.to_string())
    } else {
        Some(format!("{s}:{DEFAULT_PORT}"))
    }
}

fn write_line(
    stream: &mut TcpStream,
    msg: &Wire,
    cipher: &crate::crypt::Cipher,
) -> bool {
    crate::crypt::write_sealed(stream, cipher, msg)
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

pub(crate) type ClientMap = Arc<Mutex<HashMap<String, SyncSender<Wire>>>>;

const WIRE_CAP: usize = 16;

pub(crate) fn wire_chan() -> (SyncSender<Wire>, Receiver<Wire>) {
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
            | Wire::MapTokens { .. }
            | Wire::MapTokensAsk
            | Wire::Sheet { .. }
            | Wire::SheetAsk
            | Wire::MapImageAsk
            | Wire::Mix { .. }
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

pub(crate) fn recv_batch(wrx: &Receiver<Wire>) -> Option<Vec<Wire>> {
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

pub(crate) fn run_hub(
    self_id: String,
    handle: String,
    cmd_rx: Receiver<Cmd>,
    ev_tx: Sender<NetEvent>,
    alive: Arc<AtomicBool>,
) {
    let mut handle = handle;
    let mut stop = Arc::new(AtomicBool::new(false));
    let mut clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let mut guest_tx: Option<SyncSender<Wire>> = None;
    let mut names: HashMap<String, String> = HashMap::new();
    names.insert(self_id.clone(), handle.clone());
    let mut relay_child: Option<std::process::Child> = None;
    let mut listening = false;
    let mut listen_port = DEFAULT_PORT;
    let mut table_key: std::sync::Arc<Vec<u8>> = std::sync::Arc::new(Vec::new());
    let mut internet_on = false;
    let mut mesh_slot: Option<String> = None;

    while alive.load(Ordering::SeqCst) {
        let cmd = match cmd_rx.recv_timeout(Duration::from_millis(40)) {
            Ok(c) => c,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        match cmd {
            Cmd::Host { internet, root: _ } => {
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
                        let key = crate::crypt::mint_key();
                        table_key = std::sync::Arc::new(key.clone());
                        let mut addrs = advertised_addrs(port);
                        let roster = Arc::new(Mutex::new(vec![PeerInfo {
                            id: self_id.clone(),
                            name: handle.clone(),
                        }]));
                        spawn_beacon(
                            self_id.clone(),
                            handle.clone(),
                            port,
                            stop.clone(),
                            ev_tx.clone(),
                            table_key.clone(),
                        );
                        mesh_slot = None;
                        if let Some(udp) = crate::mesh::bind_mesh(port) {
                            if let Some((ip, mapped)) = crate::mesh::stun_mapped(&udp) {
                                let slot = format!("udp:{ip}:{mapped}");
                                if !addrs.iter().any(|a| a == &slot) {
                                    addrs.push(slot.clone());
                                }
                                mesh_slot = Some(slot);
                            }
                            crate::mesh::spawn_host(
                                udp,
                                clients.clone(),
                                roster.clone(),
                                ev_tx.clone(),
                                self_id.clone(),
                                handle.clone(),
                                stop.clone(),
                                table_key.clone(),
                            );
                        }
                        let _ = ev_tx.send(NetEvent::Hosting {
                            port,
                            addrs: addrs.clone(),
                            internet,
                            key: key.clone(),
                        });
                        let _ = ev_tx.send(NetEvent::Status(if internet {
                            "Hosting · node opening internet path…".into()
                        } else {
                            "Hosting · local network".into()
                        }));
                        spawn_accept(
                            listener,
                            clients.clone(),
                            roster,
                            ev_tx.clone(),
                            self_id.clone(),
                            handle.clone(),
                            stop.clone(),
                            names.clone(),
                            table_key.clone(),
                        );
                        internet_on = internet;
                        if internet {
                            let ev = ev_tx.clone();
                            let halt = stop.clone();
                            let k = key.clone();
                            let extra = mesh_slot.clone();
                            thread::spawn(move || {
                                open_internet_invite(port, &k, addrs, extra, ev, halt);
                            });
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
                let Some(inv) = crate::crypt::parse_invite(&addr) else {
                    let _ = ev_tx.send(NetEvent::Error(
                        "Paste a blightnet:// invite from the host.".into(),
                    ));
                    continue;
                };
                table_key = std::sync::Arc::new(inv.key.clone());
                let (wtx, wrx) = wire_chan();
                guest_tx = Some(wtx.clone());
                let hello = Wire::Hello {
                    id: self_id.clone(),
                    name: handle.clone(),
                    role: "guest".into(),
                };
                match connect_invite(&crate::mesh::tcp_slots(&inv.addrs)) {
                    Some(stream) => {
                        let Ok(reader_stream) = stream.try_clone() else {
                            let _ = ev_tx.send(NetEvent::Error("Could not clone the socket.".into()));
                            continue;
                        };
                        let mut writer = stream.try_clone().ok();
                        let mut reader = BufReader::new(reader_stream);
                        let Some(ref mut w) = writer else {
                            let _ = ev_tx.send(NetEvent::Error("Could not write the socket.".into()));
                            continue;
                        };
                        let Some(cipher) =
                            crate::crypt::handshake_client(&mut reader, w, &inv.key)
                        else {
                            let _ = ev_tx.send(NetEvent::Error(
                                "That table did not complete a Blightnet handshake. Use a fresh invite from Host.".into(),
                            ));
                            continue;
                        };
                        if !write_line(w, &hello, &cipher) {
                            let _ = ev_tx.send(NetEvent::Error("Connected but could not say hello.".into()));
                            continue;
                        }
                        spawn_guest(
                            reader,
                            writer.take().unwrap_or_else(|| stream),
                            wrx,
                            ev_tx.clone(),
                            stop.clone(),
                            self_id.clone(),
                            cipher,
                        );
                        let shown = crate::crypt::encode_invite(&inv.key, &inv.addrs);
                        let _ = ev_tx.send(NetEvent::Joined { addr: shown });
                        let _ = ev_tx.send(NetEvent::Status("Joined".into()));
                    }
                    None => {
                        let targets = crate::mesh::udp_targets(&inv.addrs);
                        if crate::mesh::join_guest(
                            &targets,
                            &inv.key,
                            hello,
                            wrx,
                            ev_tx.clone(),
                            stop.clone(),
                            self_id.clone(),
                        ) {
                            let shown = crate::crypt::encode_invite(&inv.key, &inv.addrs);
                            let _ = ev_tx.send(NetEvent::Joined { addr: shown });
                            let _ = ev_tx.send(NetEvent::Status("Joined · node mesh".into()));
                        } else {
                            let _ = ev_tx.send(NetEvent::Error(
                                "Could not reach the host node. Both sides must run Blightnet. Try the same network, or wait for the host invite to list a udp: address.".into(),
                            ));
                        }
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
                            let roster = Arc::new(Mutex::new(vec![PeerInfo {
                                id: self_id.clone(),
                                name: handle.clone(),
                            }]));
                            spawn_accept(
                                listener,
                                clients.clone(),
                                roster,
                                ev_tx.clone(),
                                self_id.clone(),
                                handle.clone(),
                                stop.clone(),
                                names.clone(),
                                table_key.clone(),
                            );
                            spawn_beacon(
                                self_id.clone(),
                                handle.clone(),
                                port,
                                stop.clone(),
                                ev_tx.clone(),
                                table_key.clone(),
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
                        table_key.clone(),
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
                internet_on = false;
                mesh_slot = None;
                if let Some(mut c) = relay_child.take() {
                    let _ = c.kill();
                }
                if let Ok(mut map) = clients.lock() {
                    map.clear();
                }
                guest_tx = None;
                table_key = std::sync::Arc::new(Vec::new());
                let _ = ev_tx.send(NetEvent::Left);
                let _ = ev_tx.send(NetEvent::Status("Offline".into()));
            }
            Cmd::SetHandle(name) => {
                handle = name;
                names.insert(self_id.clone(), handle.clone());
            }
            Cmd::RefreshInvite => {
                if listening && internet_on {
                    let ev = ev_tx.clone();
                    let halt = stop.clone();
                    let k = (*table_key).clone();
                    let port = listen_port;
                    let mut addrs = advertised_addrs(port);
                    if let Some(s) = &mesh_slot {
                        if !addrs.iter().any(|a| a == s) {
                            addrs.push(s.clone());
                        }
                    }
                    let extra = mesh_slot.clone();
                    thread::spawn(move || {
                        open_internet_invite(port, &k, addrs, extra, ev, halt);
                    });
                }
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
    roster: Arc<Mutex<Vec<PeerInfo>>>,
    ev_tx: Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
    _names: HashMap<String, String>,
    table_key: Arc<Vec<u8>>,
) {
    thread::spawn(move || {
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
                    let key_c = table_key.clone();
                    thread::spawn(move || {
                        handle_client(
                            stream, wtx, wrx, clients_c, roster_c, ev, host_id_c, host_name_c,
                            stop_c, key_c,
                        );
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
    table_key: Arc<Vec<u8>>,
) {
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    let mut reader = BufReader::new(reader_stream);
    let Some(cipher) = crate::crypt::handshake_server(&mut reader, &mut writer, &table_key) else {
        return;
    };
    let cipher = Arc::new(cipher);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let hello: Wire = match cipher.open_line(line.trim()) {
        Some(w) => w,
        None => return,
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
        &cipher,
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
    let cipher_w = cipher.clone();
    thread::spawn(move || {
        while let Some(batch) = recv_batch(&wrx) {
            let mut dead = false;
            if let Some(ref mut w) = writer2 {
                for msg in &batch {
                    if !write_line(w, msg, &cipher_w) {
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
                let Some(msg) = cipher.open_line::<Wire>(line.trim()) else {
                    continue;
                };
                host_incoming(msg, &cid, &host_id, &clients, &ev_tx);
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
    mut reader: BufReader<TcpStream>,
    writer: TcpStream,
    wrx: Receiver<Wire>,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
    self_id: String,
    cipher: crate::crypt::Cipher,
) {
    let cipher = Arc::new(cipher);
    let mut writer_out = writer.try_clone().ok();
    let cipher_w = cipher.clone();
    thread::spawn(move || {
        while let Some(batch) = recv_batch(&wrx) {
            let mut dead = false;
            if let Some(ref mut w) = writer_out {
                for msg in &batch {
                    if !write_line(w, msg, &cipher_w) {
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
        let _ = writer;
    });
    thread::spawn(move || {
        let mut line = String::new();
        while !stop.load(Ordering::SeqCst) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let Some(msg) = cipher.open_line::<Wire>(line.trim()) else {
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

pub(crate) fn host_incoming(
    msg: Wire,
    cid: &str,
    host_id: &str,
    clients: &ClientMap,
    ev_tx: &Sender<NetEvent>,
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
                    Wire::Mix {
                        layers,
                        blight,
                        place,
                        time,
                        inside,
                    } => {
                        let _ = ev_tx.send(NetEvent::Mix {
                            layers: layers.clone(),
                            blight: *blight,
                            place: place.clone(),
                            time: time.clone(),
                            inside: *inside,
                        });
                        broadcast(&clients, &msg, Some(&cid));
                    }
                    Wire::Ping => {
                        if let Ok(map) = clients.lock() {
                            if let Some(tx) = map.get(cid) {
                                push_wire(tx, Wire::Pong);
                            }
                        }
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
                    Wire::MapTokens { tokens } => {
                        let _ = ev_tx.send(NetEvent::MapTokens {
                            tokens: tokens.clone(),
                        });
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapTokensAsk => {
                        let _ = ev_tx.send(NetEvent::MapTokensAsk);
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::Sheet { from, chars } => {
                        let _ = ev_tx.send(NetEvent::Sheet {
                            from: from.clone(),
                            chars: chars.clone(),
                        });
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::SheetAsk => {
                        let _ = ev_tx.send(NetEvent::SheetAsk);
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    Wire::MapImageAsk => {
                        let _ = ev_tx.send(NetEvent::MapImageAsk);
                        broadcast_hold(&clients, &msg, Some(&cid));
                    }
                    _ => {}
                }
}

pub(crate) fn finish_join(
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

pub(crate) fn load_peer_id(root: &std::path::Path) -> String {
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
    table_key: Arc<Vec<u8>>,
) {
    thread::spawn(move || {
        let sock = UdpSocket::bind(("0.0.0.0", UDP_PORT))
            .or_else(|_| UdpSocket::bind("0.0.0.0:0"));
        let Ok(sock) = sock else {
            return;
        };
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(400)));
        let pkt = if table_key.len() == crate::crypt::KEY_LEN {
            format!(
                "BN|{self_id}|{handle}|{tcp_port}|{}",
                crate::crypt::encode_key(&table_key)
            )
        } else {
            format!("BN|{self_id}|{handle}|{tcp_port}")
        };
        let mut buf = [0u8; 512];
        while !stop.load(Ordering::SeqCst) {
            let _ = sock.send_to(pkt.as_bytes(), ("255.255.255.255", UDP_PORT));
            if let Ok((n, from)) = sock.recv_from(&mut buf) {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    if let Some((id, name, port, key)) = parse_beacon(text) {
                        if id != self_id {
                            let host = format!("{}:{port}", from.ip());
                            let addr = if key.len() == crate::crypt::KEY_LEN {
                                crate::crypt::encode_invite(&key, &[host])
                            } else {
                                host
                            };
                            let _ = ev_tx.send(NetEvent::PeerSeen { id, name, addr });
                        }
                    }
                }
            }
        }
    });
}

fn parse_beacon(text: &str) -> Option<(String, String, u16, Vec<u8>)> {
    let mut parts = text.trim().split('|');
    if parts.next()? != "BN" {
        return None;
    }
    let id = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    let port = parts.next()?.parse().ok()?;
    let key = parts
        .next()
        .and_then(crate::crypt::decode_key)
        .unwrap_or_default();
    Some((id, name, port, key))
}

fn dial_peer(
    addr: &str,
    self_id: String,
    handle: String,
    clients: ClientMap,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
) {
    let Some(inv) = crate::crypt::parse_invite(addr).or_else(|| {
        parse_addr(addr).map(|a| crate::crypt::Invite {
            key: vec![],
            addrs: vec![a],
            web: None,
        })
    }) else {
        return;
    };
    let hello = Wire::Hello {
        id: self_id.clone(),
        name: handle,
        role: "presence".into(),
    };
    if let Some(stream) = connect_invite(&crate::mesh::tcp_slots(&inv.addrs)) {
        let Ok(reader_stream) = stream.try_clone() else {
            return;
        };
        let Ok(mut writer) = stream.try_clone() else {
            return;
        };
        let mut reader = BufReader::new(reader_stream);
        let Some(cipher) = crate::crypt::handshake_client(&mut reader, &mut writer, &inv.key) else {
            return;
        };
        let cipher = Arc::new(cipher);
        if !write_line(&mut writer, &hello, &cipher) {
            return;
        }
        let shown = crate::crypt::encode_invite(&inv.key, &inv.addrs);
        let (wtx, wrx) = wire_chan();
        let mut w2 = writer.try_clone().ok();
        let cipher_w = cipher.clone();
        thread::spawn(move || {
            while let Some(batch) = recv_batch(&wrx) {
                let mut dead = false;
                if let Some(ref mut w) = w2 {
                    for msg in &batch {
                        if !write_line(w, msg, &cipher_w) {
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
        let mut line = String::new();
        while !stop.load(Ordering::SeqCst) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let Some(msg) = cipher.open_line::<Wire>(line.trim()) else {
                        continue;
                    };
                    if let Wire::Welcome { id, .. } = &msg {
                        if let Ok(mut map) = clients.lock() {
                            map.insert(id.clone(), wtx.clone());
                        }
                        let _ = ev_tx.send(NetEvent::PeerSeen {
                            id: id.clone(),
                            name: String::new(),
                            addr: shown.clone(),
                        });
                    }
                    guest_incoming(msg, &self_id, &ev_tx);
                }
                Err(_) => break,
            }
        }
        return;
    }
    let (wtx, wrx) = wire_chan();
    let _ = wtx;
    let _ = crate::mesh::join_guest(
        &crate::mesh::udp_targets(&inv.addrs),
        &inv.key,
        hello,
        wrx,
        ev_tx,
        stop,
        self_id,
    );
}

pub(crate) fn guest_incoming(msg: Wire, self_id: &str, ev_tx: &Sender<NetEvent>) {
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
        Wire::MapTokens { tokens } => {
            let _ = ev_tx.send(NetEvent::MapTokens { tokens });
        }
        Wire::MapTokensAsk => {
            let _ = ev_tx.send(NetEvent::MapTokensAsk);
        }
        Wire::Sheet { from, chars } => {
            if from != self_id {
                let _ = ev_tx.send(NetEvent::Sheet { from, chars });
            }
        }
        Wire::SheetAsk => {
            let _ = ev_tx.send(NetEvent::SheetAsk);
        }
        Wire::MapImageAsk => {
            let _ = ev_tx.send(NetEvent::MapImageAsk);
        }
        _ => {}
    }
}

fn is_lan_host(addr: &str) -> bool {
    let host = addr
        .strip_prefix("udp:")
        .unwrap_or(addr)
        .split(':')
        .next()
        .unwrap_or(addr);
    host == "localhost"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.2")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
}

fn connect_invite(addrs: &[String]) -> Option<TcpStream> {
    let mut lan = Vec::new();
    let mut wan = Vec::new();
    for a in addrs {
        if crate::mesh::is_udp_slot(a) {
            continue;
        }
        if is_lan_host(a) {
            lan.push(a.clone());
        } else {
            wan.push(a.clone());
        }
    }
    if let Some(s) = connect_any(&lan, Duration::from_millis(800)) {
        return Some(s);
    }
    connect_any(&wan, Duration::from_secs(4))
}

fn connect_any(addrs: &[String], timeout: Duration) -> Option<TcpStream> {
    if addrs.is_empty() {
        return None;
    }
    if addrs.len() == 1 {
        let sock = addrs[0].to_socket_addrs().ok().and_then(|mut it| it.next())?;
        let s = TcpStream::connect_timeout(&sock, timeout).ok()?;
        let _ = s.set_nodelay(true);
        return Some(s);
    }
    let (tx, rx) = mpsc::channel();
    for a in addrs {
        let tx = tx.clone();
        let a = a.clone();
        thread::spawn(move || {
            if let Some(sock) = a.to_socket_addrs().ok().and_then(|mut it| it.next()) {
                if let Ok(s) = TcpStream::connect_timeout(&sock, timeout) {
                    let _ = s.set_nodelay(true);
                    let _ = tx.send(s);
                }
            }
        });
    }
    drop(tx);
    rx.recv_timeout(timeout + Duration::from_millis(250)).ok()
}

fn open_internet_invite(
    port: u16,
    key: &[u8],
    mut addrs: Vec<String>,
    mesh: Option<String>,
    ev_tx: Sender<NetEvent>,
    stop: Arc<AtomicBool>,
) {
    if stop.load(Ordering::SeqCst) {
        return;
    }
    let lan = addrs
        .iter()
        .find(|a| !a.starts_with("127.") && !a.starts_with("[::") && !a.starts_with("udp:"))
        .cloned();
    let mapped = if let Some(ref local) = lan {
        let tcp = upnp_map(port, local, "TCP");
        let udp = upnp_map(port, local, "UDP");
        tcp || udp
    } else {
        false
    };
    if let Some(ip) = stun_public_ip().or_else(http_public_ip) {
        let wan = format!("{ip}:{port}");
        if !addrs.iter().any(|a| a == &wan) {
            addrs.insert(0, wan);
        }
        let udp = format!("udp:{ip}:{port}");
        if !addrs.iter().any(|a| a == &udp) {
            addrs.push(udp);
        }
    }
    if let Some(slot) = mesh {
        if !addrs.iter().any(|a| a == &slot) {
            addrs.push(slot);
        }
    }
    let invite = crate::crypt::encode_invite(key, &addrs);
    let _ = ev_tx.send(NetEvent::Relay { url: invite });
    let _ = ev_tx.send(NetEvent::Status(if mapped {
        "Hosting · node internet path ready".into()
    } else {
        "Hosting · node invite ready. Daemons punch UDP; TCP maps if the router allows it.".into()
    }));
}

fn stun_public_ip() -> Option<String> {
    for host in ["stun.l.google.com:19302", "stun.cloudflare.com:3478"] {
        if let Some(ip) = stun_query(host) {
            return Some(ip);
        }
    }
    None
}

fn stun_query(host: &str) -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(900))).ok()?;
    let mut req = [0u8; 20];
    req[0] = 0x00;
    req[1] = 0x01;
    req[4] = 0x21;
    req[5] = 0x12;
    req[6] = 0xa4;
    req[7] = 0x42;
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut req[8..20]);
    sock.send_to(&req, host).ok()?;
    let mut buf = [0u8; 256];
    let n = sock.recv(&mut buf).ok()?;
    parse_stun_mapped(&buf[..n])
}

fn parse_stun_mapped(buf: &[u8]) -> Option<String> {
    crate::mesh::stun_mapped_ip_from_buf(buf)
}

fn http_public_ip() -> Option<String> {
    let sock = ("api.ipify.org", 443)
        .to_socket_addrs()
        .ok()?
        .find(|a| a.is_ipv4())?;
    let stream = TcpStream::connect_timeout(&sock, Duration::from_secs(3)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let cx = native_tls::TlsConnector::new().ok()?;
    let mut tls = cx.connect("api.ipify.org", stream).ok()?;
    tls.write_all(b"GET / HTTP/1.0\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut s = String::new();
    let _ = tls.read_to_string(&mut s);
    let body = s.split("\r\n\r\n").nth(1)?.trim();
    if body.split('.').count() == 4 && body.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some(body.to_string())
    } else {
        None
    }
}

fn upnp_map(port: u16, local: &str, proto: &str) -> bool {
    let ip = local.split(':').next().unwrap_or(local);
    let sock = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = sock.set_broadcast(true);
    let search = concat!(
        "M-SEARCH * HTTP/1.1\r\n",
        "HOST: 239.255.255.250:1900\r\n",
        "MAN: \"ssdp:discover\"\r\n",
        "MX: 1\r\n",
        "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n",
        "\r\n"
    );
    if sock.send_to(search.as_bytes(), "239.255.255.250:1900").is_err() {
        return false;
    }
    let mut buf = [0u8; 2048];
    let Ok((n, _)) = sock.recv_from(&mut buf) else {
        return false;
    };
    let text = String::from_utf8_lossy(&buf[..n]);
    let loc = text.lines().find_map(|l| {
        let l = l.trim();
        if l.to_ascii_lowercase().starts_with("location:") {
            Some(l.splitn(2, ':').nth(1)?.trim().to_string())
        } else {
            None
        }
    });
    let Some(loc) = loc else {
        return false;
    };
    upnp_add_mapping(&loc, ip, port, proto)
}

fn upnp_add_mapping(location: &str, internal_ip: &str, port: u16, proto: &str) -> bool {
    let Some((host, path, port_http)) = split_http_url(location) else {
        return false;
    };
    let xml = match http_get(&host, port_http, &path) {
        Some(s) => s,
        None => return false,
    };
    let ctrl = upnp_control_url(&xml).unwrap_or_else(|| path.clone());
    let ctrl_path = if ctrl.starts_with("http") {
        split_http_url(&ctrl).map(|(_, p, _)| p).unwrap_or(ctrl)
    } else if ctrl.starts_with('/') {
        ctrl
    } else {
        format!("/{ctrl}")
    };
    let body = format!(
        concat!(
            "<?xml version=\"1.0\"?>",
            "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
            "s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">",
            "<s:Body>",
            "<u:AddPortMapping xmlns:u=\"urn:schemas-upnp-org:service:WANIPConnection:1\">",
            "<NewRemoteHost></NewRemoteHost>",
            "<NewExternalPort>{port}</NewExternalPort>",
            "<NewProtocol>{proto}</NewProtocol>",
            "<NewInternalPort>{port}</NewInternalPort>",
            "<NewInternalClient>{ip}</NewInternalClient>",
            "<NewEnabled>1</NewEnabled>",
            "<NewPortMappingDescription>Blightnet</NewPortMappingDescription>",
            "<NewLeaseDuration>0</NewLeaseDuration>",
            "</u:AddPortMapping>",
            "</s:Body></s:Envelope>"
        ),
        port = port,
        ip = internal_ip,
        proto = proto
    );
    http_post_soap(&host, port_http, &ctrl_path, &body)
}

fn split_http_url(url: &str) -> Option<(String, String, u16)> {
    let rest = url.strip_prefix("http://")?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let path = format!("/{path}");
    if let Some((h, p)) = hostport.split_once(':') {
        Some((h.to_string(), path, p.parse().ok()?))
    } else {
        Some((hostport.to_string(), path, 80))
    }
}

fn http_get(host: &str, port: u16, path: &str) -> Option<String> {
    let addr = format!("{host}:{port}");
    let sock = addr.to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&sock, Duration::from_secs(2)).ok()?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes()).ok()?;
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    Some(out)
}

fn http_post_soap(host: &str, port: u16, path: &str, body: &str) -> bool {
    let addr = format!("{host}:{port}");
    let Some(sock) = addr.to_socket_addrs().ok().and_then(|mut it| it.next()) else {
        return false;
    };
    let Ok(mut s) = TcpStream::connect_timeout(&sock, Duration::from_secs(2)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let req = format!(
        "POST {path} HTTP/1.0\r\nHost: {host}\r\nContent-Type: text/xml; charset=\"utf-8\"\r\nSOAPAction: \"urn:schemas-upnp-org:service:WANIPConnection:1#AddPortMapping\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    if s.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    out.contains("200") && !out.to_ascii_lowercase().contains("fault")
}

fn upnp_control_url(xml: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let idx = lower.find("wanipconnection")?;
    let slice = &xml[idx..];
    let c = slice.to_ascii_lowercase().find("<controlurl>")?;
    let rest = &slice[c + 12..];
    let end = rest.to_ascii_lowercase().find("</controlurl>")?;
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_accepts_lan_invite() {
        assert_eq!(
            parse_addr("blightnet://10.0.0.4:8766").as_deref(),
            Some("10.0.0.4:8766")
        );
        assert_eq!(parse_addr("192.168.1.9").as_deref(), Some("192.168.1.9:8766"));
        assert!(parse_addr("https://foo.trycloudflare.com").is_none());
        assert!(parse_addr("").is_none());
        assert_eq!(
            parse_addr("blightnet://10.0.0.4:8766\r").as_deref(),
            Some("10.0.0.4:8766")
        );
        assert_eq!(
            parse_addr("BLIGHTNET://192.168.0.12").as_deref(),
            Some("192.168.0.12:8766")
        );
        let key = crate::crypt::mint_key();
        let inv = crate::crypt::encode_invite(&key, &["10.0.0.4:8766".into(), "9.9.9.9:8766".into()]);
        assert_eq!(parse_addr(&inv).as_deref(), Some("10.0.0.4:8766"));
        let parsed = crate::crypt::parse_invite(&inv).unwrap();
        assert_eq!(parsed.key, key);
        assert_eq!(parsed.addrs.len(), 2);
    }

    #[test]
    fn stun_xor_mapped_ipv4() {
        let mut buf = vec![0u8; 32];
        buf[0] = 0x01;
        buf[1] = 0x01;
        buf[20] = 0x00;
        buf[21] = 0x20;
        buf[22] = 0x00;
        buf[23] = 0x08;
        buf[25] = 0x01;
        buf[26] = 0x21;
        buf[27] = 0x12;
        buf[28] = 0x21 ^ 10;
        buf[29] = 0x12 ^ 0;
        buf[30] = 0xa4 ^ 0;
        buf[31] = 0x42 ^ 1;
        assert_eq!(parse_stun_mapped(&buf).as_deref(), Some("10.0.0.1"));
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
            Wire::Mix {
                layers: HashMap::from([("rain".into(), 0.4)]),
                blight: true,
                place: "nightcity".into(),
                time: "night".into(),
                inside: false,
            },
            Wire::SheetAsk,
            Wire::MapImageAsk,
            Wire::MapTokensAsk,
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
        let mut invite = String::new();
        for _ in 0..80 {
            for ev in host.poll() {
                if let NetEvent::Hosting { port: p, key, .. } = ev {
                    port = p;
                    invite = crate::crypt::encode_invite(&key, &[format!("127.0.0.1:{p}")]);
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
        guest.join(&invite);
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

    #[test]
    fn host_guest_sync_mix_sheet_tokens() {
        let a = std::env::temp_dir().join(format!("bn-sync-h-{}", rand::random::<u32>()));
        let b = std::env::temp_dir().join(format!("bn-sync-g-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let mut host = NetHub::new("Host".into(), &a);
        host.host(false, a.clone());
        let mut port = 0u16;
        let mut invite = String::new();
        for _ in 0..80 {
            for ev in host.poll() {
                if let NetEvent::Hosting { port: p, key, .. } = ev {
                    port = p;
                    invite = crate::crypt::encode_invite(&key, &[format!("127.0.0.1:{p}")]);
                }
            }
            if host.role == Role::Host && port != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let mut guest = NetHub::new("Guest".into(), &b);
        guest.join(&invite);
        for _ in 0..80 {
            let _ = host.poll();
            let _ = guest.poll();
            if guest.role == Role::Guest {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert_eq!(guest.role, Role::Guest);
        guest.send_mix(
            HashMap::from([("rain".into(), 0.5)]),
            true,
            "nightcity".into(),
            "night".into(),
            false,
        );
        let mut saw_mix = false;
        for _ in 0..80 {
            for ev in host.poll() {
                if let NetEvent::Mix { place, .. } = ev {
                    if place == "nightcity" {
                        saw_mix = true;
                    }
                }
            }
            if saw_mix {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        host.send_sheet(vec![]);
        host.send_map_tokens(vec![]);
        host.send_map_image_ask();
        let mut saw_sheet = false;
        let mut saw_tok = false;
        let mut saw_ask = false;
        for _ in 0..80 {
            for ev in guest.poll() {
                match ev {
                    NetEvent::Sheet { .. } => saw_sheet = true,
                    NetEvent::MapTokens { .. } => saw_tok = true,
                    NetEvent::MapImageAsk => saw_ask = true,
                    _ => {}
                }
            }
            if saw_sheet && saw_tok && saw_ask {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        drop(guest);
        drop(host);
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
        assert!(saw_mix, "host did not receive guest mix");
        assert!(saw_sheet, "guest did not receive sheets");
        assert!(saw_tok, "guest did not receive tokens");
        assert!(saw_ask, "guest did not receive map-image-ask");
    }
}
