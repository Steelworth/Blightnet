//! Local node: connectivity, keep-alive, reconnect, invite refresh.
//! The window talks to this process over 127.0.0.1 so the table can outlive the UI.

use crate::net::{self, Cmd, NetEvent, PeerInfo, Role, Wire};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const PING_EVERY: Duration = Duration::from_secs(15);
const RECONNECT_WAIT: Duration = Duration::from_secs(3);
const INVITE_EVERY: Duration = Duration::from_secs(5 * 60);
const CONNECT_TRIES: u32 = 24;
/// Local node port. Table traffic stays on 8766. This is UI ↔ node only.
pub const IPC_PORT: u16 = 18766;

#[derive(Clone, Serialize, Deserialize)]
pub struct Cfg {
    #[serde(default = "yes")]
    pub reconnect: bool,
    #[serde(default = "yes")]
    pub keep_table: bool,
    #[serde(default)]
    pub auto_online: bool,
    #[serde(default = "yes")]
    pub ping: bool,
    #[serde(default = "yes")]
    pub refresh_invite: bool,
}

fn yes() -> bool {
    true
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            reconnect: true,
            keep_table: true,
            auto_online: false,
            ping: true,
            refresh_invite: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Lock {
    port: u16,
    token: String,
    pid: u32,
}

#[derive(Clone, Serialize, Deserialize)]
struct Persist {
    #[serde(default)]
    invite: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    internet: bool,
    #[serde(default)]
    handle: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Ipc {
    Hello {
        token: String,
        handle: String,
    },
    Welcome {
        ok: bool,
        err: String,
    },
    Cmd {
        cmd: IpcCmd,
    },
    Event {
        event: IpcEvent,
    },
    Ping,
    Pong,
    Bye,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
enum IpcCmd {
    Host { internet: bool },
    Join { invite: String },
    Leave,
    Online,
    Dial { addr: String },
    Send { wire: Wire },
    SetHandle { handle: String },
    RefreshInvite,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
enum IpcEvent {
    Status { text: String },
    Hosting {
        port: u16,
        addrs: Vec<String>,
        internet: bool,
        key: String,
    },
    Relay { url: String },
    Joined { addr: String },
    Left,
    Peers { peers: Vec<PeerInfo> },
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
    VoicePcm { from: String, pcm: String },
    Mix {
        layers: HashMap<String, f32>,
        blight: bool,
        place: String,
        time: String,
        inside: bool,
    },
    Error { text: String },
    Online { port: u16, addrs: Vec<String> },
    PeerSeen { id: String, name: String, addr: String },
    Image {
        from: String,
        name: String,
        to: Option<String>,
        crew: Option<String>,
        mime: String,
        data: String,
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
        data: String,
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
    FileChunk { id: String, data: String },
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
    Pit { from: String, game: String, body: String },
    Probe { from: String, n: u64 },
    ProbeBack { from: String, n: u64 },
    Share {
        from: String,
        name: String,
        kind: String,
        body: String,
    },
    NetPos {
        from: String,
        name: String,
        x: f32,
        z: f32,
        yaw: f32,
    },
}

struct Inner {
    cfg: Cfg,
    root: PathBuf,
    handle: String,
    role: Role,
    invite: String,
    internet: bool,
    port: u16,
    addrs: Vec<String>,
    table_key: Vec<u8>,
    peers: Vec<PeerInfo>,
    last_left: Option<Instant>,
    last_ping: Instant,
    last_invite: Instant,
    gui: u32,
    suppress_reconnect: bool,
    clients: Vec<Sender<Ipc>>,
}

pub fn lock_path(root: &Path) -> PathBuf {
    root.join("data/daemon.lock")
}

fn cfg_path(root: &Path) -> PathBuf {
    root.join("data/daemon.json")
}

fn persist_path(root: &Path) -> PathBuf {
    root.join("data/daemon-state.json")
}

fn log_path(root: &Path) -> PathBuf {
    root.join("data/daemon.log")
}

pub fn load_cfg(root: &Path) -> Cfg {
    std::fs::read_to_string(cfg_path(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_cfg(root: &Path, cfg: &Cfg) {
    let _ = std::fs::create_dir_all(root.join("data"));
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(cfg_path(root), s);
    }
}

fn load_persist(root: &Path) -> Persist {
    std::fs::read_to_string(persist_path(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Persist {
            invite: String::new(),
            role: String::new(),
            internet: false,
            handle: String::new(),
        })
}

fn save_persist(root: &Path, p: &Persist) {
    let _ = std::fs::create_dir_all(root.join("data"));
    if let Ok(s) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(persist_path(root), s);
    }
}

fn read_lock(root: &Path) -> Option<Lock> {
    let s = std::fs::read_to_string(lock_path(root)).ok()?;
    serde_json::from_str(&s).ok()
}

fn write_lock(root: &Path, lock: &Lock) {
    let _ = std::fs::create_dir_all(root.join("data"));
    if let Ok(s) = serde_json::to_string(lock) {
        let _ = std::fs::write(lock_path(root), s);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        windows_pid_alive(pid)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        true
    }
}

#[cfg(windows)]
fn windows_pid_alive(pid: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(h: *mut std::ffi::c_void) -> i32;
        fn GetExitCodeProcess(h: *mut std::ffi::c_void, code: *mut u32) -> i32;
    }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code);
        CloseHandle(h);
        ok != 0 && code == STILL_ACTIVE
    }
}

fn ipc_addr(port: u16) -> std::net::SocketAddr {
    std::net::SocketAddr::from(([127, 0, 0, 1], port))
}

fn try_tcp(port: u16) -> Option<TcpStream> {
    TcpStream::connect_timeout(&ipc_addr(port), Duration::from_millis(400)).ok()
}

fn b64(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}

fn unb64(s: &str) -> Vec<u8> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).unwrap_or_default()
}

fn pcm_b64(samples: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    b64(&bytes)
}

fn pcm_from_b64(s: &str) -> Vec<f32> {
    unb64(s)
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32767.0)
        .collect()
}

fn event_out(ev: &NetEvent) -> IpcEvent {
    match ev {
        NetEvent::Status(t) => IpcEvent::Status { text: t.clone() },
        NetEvent::Hosting {
            port,
            addrs,
            internet,
            key,
        } => IpcEvent::Hosting {
            port: *port,
            addrs: addrs.clone(),
            internet: *internet,
            key: b64(key),
        },
        NetEvent::Relay { url } => IpcEvent::Relay { url: url.clone() },
        NetEvent::Joined { addr } => IpcEvent::Joined { addr: addr.clone() },
        NetEvent::Left => IpcEvent::Left,
        NetEvent::Peers(p) => IpcEvent::Peers { peers: p.clone() },
        NetEvent::Chat {
            from,
            name,
            text,
            whisper,
        } => IpcEvent::Chat {
            from: from.clone(),
            name: name.clone(),
            text: text.clone(),
            whisper: *whisper,
        },
        NetEvent::Voice {
            from,
            name,
            action,
            to,
            crew,
        } => IpcEvent::Voice {
            from: from.clone(),
            name: name.clone(),
            action: action.clone(),
            to: to.clone(),
            crew: crew.clone(),
        },
        NetEvent::VoicePcm { from, samples } => IpcEvent::VoicePcm {
            from: from.clone(),
            pcm: pcm_b64(samples),
        },
        NetEvent::Mix {
            layers,
            blight,
            place,
            time,
            inside,
        } => IpcEvent::Mix {
            layers: layers.clone(),
            blight: *blight,
            place: place.clone(),
            time: time.clone(),
            inside: *inside,
        },
        NetEvent::Error(t) => IpcEvent::Error { text: t.clone() },
        NetEvent::Online { port, addrs } => IpcEvent::Online {
            port: *port,
            addrs: addrs.clone(),
        },
        NetEvent::PeerSeen { id, name, addr } => IpcEvent::PeerSeen {
            id: id.clone(),
            name: name.clone(),
            addr: addr.clone(),
        },
        NetEvent::Image {
            from,
            name,
            to,
            crew,
            mime,
            data,
        } => IpcEvent::Image {
            from: from.clone(),
            name: name.clone(),
            to: to.clone(),
            crew: crew.clone(),
            mime: mime.clone(),
            data: b64(data),
        },
        NetEvent::Video {
            from,
            name,
            action,
            to,
            crew,
        } => IpcEvent::Video {
            from: from.clone(),
            name: name.clone(),
            action: action.clone(),
            to: to.clone(),
            crew: crew.clone(),
        },
        NetEvent::VideoFrame {
            from,
            name,
            kind,
            to,
            crew,
            data,
        } => IpcEvent::VideoFrame {
            from: from.clone(),
            name: name.clone(),
            kind: kind.clone(),
            to: to.clone(),
            crew: crew.clone(),
            data: b64(data),
        },
        NetEvent::NethookPut { hook } => IpcEvent::NethookPut { hook: hook.clone() },
        NetEvent::NethookDel { id, owner_id } => IpcEvent::NethookDel {
            id: id.clone(),
            owner_id: owner_id.clone(),
        },
        NetEvent::NethookAsk => IpcEvent::NethookAsk,
        NetEvent::FileStart {
            from,
            name,
            to,
            crew,
            mime,
            filename,
            id,
            size,
        } => IpcEvent::FileStart {
            from: from.clone(),
            name: name.clone(),
            to: to.clone(),
            crew: crew.clone(),
            mime: mime.clone(),
            filename: filename.clone(),
            id: id.clone(),
            size: *size,
        },
        NetEvent::FileChunk { id, data } => IpcEvent::FileChunk {
            id: id.clone(),
            data: b64(data),
        },
        NetEvent::FileDone { id } => IpcEvent::FileDone { id: id.clone() },
        NetEvent::MapMark { mark } => IpcEvent::MapMark { mark: mark.clone() },
        NetEvent::MapMarkDel { id } => IpcEvent::MapMarkDel { id: id.clone() },
        NetEvent::MapMarks { marks } => IpcEvent::MapMarks {
            marks: marks.clone(),
        },
        NetEvent::MapMarksClear => IpcEvent::MapMarksClear,
        NetEvent::MapMarksAsk => IpcEvent::MapMarksAsk,
        NetEvent::MapTokens { tokens } => IpcEvent::MapTokens {
            tokens: tokens.clone(),
        },
        NetEvent::MapTokensAsk => IpcEvent::MapTokensAsk,
        NetEvent::Sheet { from, chars } => IpcEvent::Sheet {
            from: from.clone(),
            chars: chars.clone(),
        },
        NetEvent::SheetAsk => IpcEvent::SheetAsk,
        NetEvent::MapImageAsk => IpcEvent::MapImageAsk,
        NetEvent::Pit { from, game, body } => IpcEvent::Pit {
            from: from.clone(),
            game: game.clone(),
            body: body.clone(),
        },
        NetEvent::Probe { from, n } => IpcEvent::Probe {
            from: from.clone(),
            n: *n,
        },
        NetEvent::ProbeBack { from, n } => IpcEvent::ProbeBack {
            from: from.clone(),
            n: *n,
        },
        NetEvent::Share {
            from,
            name,
            kind,
            body,
        } => IpcEvent::Share {
            from: from.clone(),
            name: name.clone(),
            kind: kind.clone(),
            body: body.clone(),
        },
        NetEvent::NetPos { from, name, x, z, yaw } => IpcEvent::NetPos {
            from: from.clone(),
            name: name.clone(),
            x: *x,
            z: *z,
            yaw: *yaw,
        },
    }
}

fn event_in(ev: IpcEvent) -> NetEvent {
    match ev {
        IpcEvent::Status { text } => NetEvent::Status(text),
        IpcEvent::Hosting {
            port,
            addrs,
            internet,
            key,
        } => NetEvent::Hosting {
            port,
            addrs,
            internet,
            key: unb64(&key),
        },
        IpcEvent::Relay { url } => NetEvent::Relay { url },
        IpcEvent::Joined { addr } => NetEvent::Joined { addr },
        IpcEvent::Left => NetEvent::Left,
        IpcEvent::Peers { peers } => NetEvent::Peers(peers),
        IpcEvent::Chat {
            from,
            name,
            text,
            whisper,
        } => NetEvent::Chat {
            from,
            name,
            text,
            whisper,
        },
        IpcEvent::Voice {
            from,
            name,
            action,
            to,
            crew,
        } => NetEvent::Voice {
            from,
            name,
            action,
            to,
            crew,
        },
        IpcEvent::VoicePcm { from, pcm } => NetEvent::VoicePcm {
            from,
            samples: pcm_from_b64(&pcm),
        },
        IpcEvent::Mix {
            layers,
            blight,
            place,
            time,
            inside,
        } => NetEvent::Mix {
            layers,
            blight,
            place,
            time,
            inside,
        },
        IpcEvent::Error { text } => NetEvent::Error(text),
        IpcEvent::Online { port, addrs } => NetEvent::Online { port, addrs },
        IpcEvent::PeerSeen { id, name, addr } => NetEvent::PeerSeen { id, name, addr },
        IpcEvent::Image {
            from,
            name,
            to,
            crew,
            mime,
            data,
        } => NetEvent::Image {
            from,
            name,
            to,
            crew,
            mime,
            data: unb64(&data),
        },
        IpcEvent::Video {
            from,
            name,
            action,
            to,
            crew,
        } => NetEvent::Video {
            from,
            name,
            action,
            to,
            crew,
        },
        IpcEvent::VideoFrame {
            from,
            name,
            kind,
            to,
            crew,
            data,
        } => NetEvent::VideoFrame {
            from,
            name,
            kind,
            to,
            crew,
            data: unb64(&data),
        },
        IpcEvent::NethookPut { hook } => NetEvent::NethookPut { hook },
        IpcEvent::NethookDel { id, owner_id } => NetEvent::NethookDel { id, owner_id },
        IpcEvent::NethookAsk => NetEvent::NethookAsk,
        IpcEvent::FileStart {
            from,
            name,
            to,
            crew,
            mime,
            filename,
            id,
            size,
        } => NetEvent::FileStart {
            from,
            name,
            to,
            crew,
            mime,
            filename,
            id,
            size,
        },
        IpcEvent::FileChunk { id, data } => NetEvent::FileChunk {
            id,
            data: unb64(&data),
        },
        IpcEvent::FileDone { id } => NetEvent::FileDone { id },
        IpcEvent::MapMark { mark } => NetEvent::MapMark { mark },
        IpcEvent::MapMarkDel { id } => NetEvent::MapMarkDel { id },
        IpcEvent::MapMarks { marks } => NetEvent::MapMarks { marks },
        IpcEvent::MapMarksClear => NetEvent::MapMarksClear,
        IpcEvent::MapMarksAsk => NetEvent::MapMarksAsk,
        IpcEvent::MapTokens { tokens } => NetEvent::MapTokens { tokens },
        IpcEvent::MapTokensAsk => NetEvent::MapTokensAsk,
        IpcEvent::Sheet { from, chars } => NetEvent::Sheet { from, chars },
        IpcEvent::SheetAsk => NetEvent::SheetAsk,
        IpcEvent::MapImageAsk => NetEvent::MapImageAsk,
        IpcEvent::NetPos { from, name, x, z, yaw } => NetEvent::NetPos { from, name, x, z, yaw },
        IpcEvent::Pit { from, game, body } => NetEvent::Pit { from, game, body },
        IpcEvent::Probe { from, n } => NetEvent::Probe { from, n },
        IpcEvent::ProbeBack { from, n } => NetEvent::ProbeBack { from, n },
        IpcEvent::Share {
            from,
            name,
            kind,
            body,
        } => NetEvent::Share {
            from,
            name,
            kind,
            body,
        },
    }
}

fn write_ipc(stream: &mut TcpStream, msg: &Ipc) -> bool {
    let Ok(mut s) = serde_json::to_string(msg) else {
        return false;
    };
    s.push('\n');
    stream.write_all(s.as_bytes()).is_ok() && stream.flush().is_ok()
}

fn cmd_from_ipc(c: IpcCmd, root: &Path) -> Cmd {
    match c {
        IpcCmd::Host { internet } => Cmd::Host {
            internet,
            root: root.to_path_buf(),
        },
        IpcCmd::Join { invite } => Cmd::Join(invite),
        IpcCmd::Leave => Cmd::Leave,
        IpcCmd::Online => Cmd::Online,
        IpcCmd::Dial { addr } => Cmd::Dial(addr),
        IpcCmd::Send { wire } => Cmd::Send(wire),
        IpcCmd::SetHandle { handle } => Cmd::SetHandle(handle),
        IpcCmd::RefreshInvite => Cmd::RefreshInvite,
    }
}

fn ipc_from_cmd(c: &Cmd) -> Option<IpcCmd> {
    Some(match c {
        Cmd::Host { internet, .. } => IpcCmd::Host {
            internet: *internet,
        },
        Cmd::Join(s) => IpcCmd::Join { invite: s.clone() },
        Cmd::Leave => IpcCmd::Leave,
        Cmd::Online => IpcCmd::Online,
        Cmd::Dial(a) => IpcCmd::Dial { addr: a.clone() },
        Cmd::Send(w) => IpcCmd::Send { wire: w.clone() },
        Cmd::SetHandle(h) => IpcCmd::SetHandle { handle: h.clone() },
        Cmd::RefreshInvite => IpcCmd::RefreshInvite,
    })
}

fn try_connect(root: &Path, handle: &str) -> Option<TcpStream> {
    let lock = read_lock(root)?;
    let mut ports = vec![lock.port, IPC_PORT];
    ports.dedup();
    for port in ports {
        let Some(mut stream) = try_tcp(port) else {
            continue;
        };
        let _ = stream.set_nodelay(true);
        if !write_ipc(
            &mut stream,
            &Ipc::Hello {
                token: lock.token.clone(),
                handle: handle.into(),
            },
        ) {
            continue;
        }
        if stream
            .set_read_timeout(Some(Duration::from_millis(800)))
            .is_err()
        {
            continue;
        }
        let Ok(clone) = stream.try_clone() else {
            continue;
        };
        let mut r = BufReader::new(clone);
        let mut line = String::new();
        if r.read_line(&mut line).is_err() {
            continue;
        }
        if let Ok(Ipc::Welcome { ok: true, .. }) = serde_json::from_str::<Ipc>(line.trim()) {
            let _ = stream.set_read_timeout(None);
            return Some(stream);
        }
    }
    None
}

fn log_line(root: &Path, msg: &str) {
    let _ = std::fs::create_dir_all(root.join("data"));
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(root))
    {
        let _ = writeln!(f, "{msg}");
    }
}

fn clear_stale_lock(root: &Path) {
    let Some(lock) = read_lock(root) else {
        return;
    };
    if try_tcp(lock.port).is_some() {
        return;
    }
    let _ = std::fs::remove_file(lock_path(root));
}

fn spawn_proc(root: &Path) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        log_line(root, "node spawn: no current exe");
        return false;
    };
    let log = log_path(root);
    let _ = std::fs::create_dir_all(root.join("data"));
    log_line(
        root,
        &format!("node spawn: {exe:?} daemon --root {root:?}"),
    );
    let Ok(file) = OpenOptions::new().create(true).append(true).open(&log) else {
        return false;
    };
    let Ok(err) = file.try_clone() else {
        return false;
    };
    let mut cmd = Command::new(&exe);
    cmd.arg("daemon").arg("--root").arg(root);
    cmd.current_dir(root);
    cmd.env("BLIGHTNET_NODE", "1");
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(file);
    cmd.stderr(err);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW only. DETACHED_PROCESS drops stdio and the child exits.
        crate::sys::hide(&mut cmd);
        cmd.creation_flags(0x08000000);
    }
    match cmd.spawn() {
        Ok(child) => {
            log_line(root, &format!("node spawn: pid {}", child.id()));
            true
        }
        Err(e) => {
            log_line(root, &format!("node spawn failed: {e}"));
            false
        }
    }
}

fn start_embedded(root: PathBuf) {
    log_line(&root, "node: starting in-process");
    thread::Builder::new()
        .name("blightnet-node".into())
        .spawn(move || {
            let _ = run_node(root);
        })
        .ok();
}

pub fn is_up(root: &Path) -> bool {
    if let Some(lock) = read_lock(root) {
        if try_tcp(lock.port).is_some() {
            return true;
        }
    }
    try_tcp(IPC_PORT).is_some()
}

pub fn connect_or_spawn(root: &Path, handle: &str) -> Result<TcpStream, String> {
    clear_stale_lock(root);
    if let Some(s) = try_connect(root, handle) {
        return Ok(s);
    }
    let _ = spawn_proc(root);
    for _ in 0..CONNECT_TRIES {
        thread::sleep(Duration::from_millis(80));
        if let Some(s) = try_connect(root, handle) {
            return Ok(s);
        }
    }
    log_line(root, "node spawn did not accept; embedding in this process");
    start_embedded(root.to_path_buf());
    for _ in 0..CONNECT_TRIES {
        thread::sleep(Duration::from_millis(80));
        if let Some(s) = try_connect(root, handle) {
            return Ok(s);
        }
    }
    Err("could not start the Blightnet node".into())
}

pub fn run_client(
    stream: TcpStream,
    cmd_rx: Receiver<Cmd>,
    ev_tx: Sender<NetEvent>,
    alive: Arc<AtomicBool>,
) {
    let _ = stream.set_nodelay(true);
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    let stop = alive.clone();
    thread::spawn(move || {
        let mut r = BufReader::new(reader_stream);
        let mut line = String::new();
        while stop.load(Ordering::SeqCst) {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => match serde_json::from_str::<Ipc>(line.trim()) {
                    Ok(Ipc::Event { event }) => {
                        let _ = ev_tx.send(event_in(event));
                    }
                    Ok(Ipc::Pong) | Ok(Ipc::Ping) => {}
                    Ok(Ipc::Bye) => break,
                    _ => {}
                },
                Err(_) => break,
            }
        }
        let _ = ev_tx.send(NetEvent::Status("Node link dropped.".into()));
    });
    while alive.load(Ordering::SeqCst) {
        match cmd_rx.recv_timeout(Duration::from_millis(40)) {
            Ok(cmd) => {
                if let Some(c) = ipc_from_cmd(&cmd) {
                    if !write_ipc(&mut writer, &Ipc::Cmd { cmd: c }) {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
}

pub fn run(root: PathBuf) -> i32 {
    if std::env::var_os("BLIGHTNET_NODE").is_none() {
        #[cfg(unix)]
        unsafe {
            if libc::setsid() != -1 {
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
            }
        }
    }
    run_node(root)
}

fn run_node(root: PathBuf) -> i32 {
    let _ = std::fs::create_dir_all(root.join("data"));
    eprintln!("blightnet node starting in {}", root.display());
    let _ = std::io::stderr().flush();
    let listener = match TcpListener::bind(ipc_addr(IPC_PORT))
        .or_else(|_| TcpListener::bind(ipc_addr(0)))
    {
        Ok(l) => l,
        Err(e) => {
            eprintln!("blightnet node bind: {e}");
            let _ = std::io::stderr().flush();
            return 1;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(IPC_PORT);
    let mut token_bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut token_bytes);
    let token = b64(&token_bytes);
    let lock = Lock {
        port,
        token: token.clone(),
        pid: std::process::id(),
    };
    write_lock(&root, &lock);
    eprintln!("blightnet node listening on 127.0.0.1:{port}");
    let _ = std::io::stderr().flush();
    let cfg = load_cfg(&root);
    save_cfg(&root, &cfg);
    let persist = load_persist(&root);
    let handle = if persist.handle.is_empty() {
        "Traveller".into()
    } else {
        persist.handle.clone()
    };
    let self_id = net::load_peer_id(&root);
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let (ev_tx, ev_rx) = mpsc::channel::<NetEvent>();
    let alive = Arc::new(AtomicBool::new(true));
    let hub_alive = alive.clone();
    let hub_handle = handle.clone();
    thread::Builder::new()
        .name("blightnet-hub".into())
        .spawn(move || net::run_hub(self_id, hub_handle, cmd_rx, ev_tx, hub_alive))
        .ok();

    let inner = Arc::new(Mutex::new(Inner {
        cfg: cfg.clone(),
        root: root.clone(),
        handle: handle.clone(),
        role: Role::Idle,
        invite: persist.invite.clone(),
        internet: persist.internet,
        port: 0,
        addrs: vec![],
        table_key: vec![],
        peers: vec![],
        last_left: None,
        last_ping: Instant::now(),
        last_invite: Instant::now(),
        gui: 0,
        suppress_reconnect: false,
        clients: vec![],
    }));

    if cfg.auto_online {
        let _ = cmd_tx.send(Cmd::Online);
        for c in net::load_contacts(&root) {
            if !c.addr.is_empty() {
                let _ = cmd_tx.send(Cmd::Dial(c.addr));
            }
        }
    }
    if cfg.reconnect && persist.role == "guest" && !persist.invite.is_empty() {
        let _ = cmd_tx.send(Cmd::Join(persist.invite.clone()));
    }

    let jobs_inner = inner.clone();
    let jobs_tx = cmd_tx.clone();
    let jobs_alive = alive.clone();
    thread::spawn(move || job_loop(jobs_inner, jobs_tx, jobs_alive));

    let fan_inner = inner.clone();
    let fan_alive = alive.clone();
    thread::spawn(move || fan_events(ev_rx, fan_inner, fan_alive));

    let _ = listener.set_nonblocking(true);
    while alive.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nodelay(true);
                let tok = token.clone();
                let inn = inner.clone();
                let tx = cmd_tx.clone();
                let flag = alive.clone();
                thread::spawn(move || serve_gui(stream, tok, inn, tx, flag));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(40));
            }
            Err(_) => thread::sleep(Duration::from_millis(80)),
        }
        if let Some(lock_now) = read_lock(&root) {
            if lock_now.pid != std::process::id() {
                break;
            }
        }
    }
    alive.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_file(lock_path(&root));
    0
}

fn fan_events(ev_rx: Receiver<NetEvent>, inner: Arc<Mutex<Inner>>, alive: Arc<AtomicBool>) {
    while alive.load(Ordering::SeqCst) {
        match ev_rx.recv_timeout(Duration::from_millis(80)) {
            Ok(ev) => {
                {
                    let mut g = inner.lock().unwrap();
                    match &ev {
                        NetEvent::Hosting {
                            internet,
                            port,
                            addrs,
                            key,
                        } => {
                            g.role = Role::Host;
                            g.internet = *internet;
                            g.port = *port;
                            g.addrs = addrs.clone();
                            g.table_key = key.clone();
                            g.last_left = None;
                        }
                        NetEvent::Relay { url } => {
                            g.invite = url.clone();
                            g.internet = true;
                            g.last_invite = Instant::now();
                        }
                        NetEvent::Joined { addr } => {
                            g.role = Role::Guest;
                            g.invite = addr.clone();
                            g.last_left = None;
                        }
                        NetEvent::Left => {
                            g.role = Role::Idle;
                            g.peers.clear();
                            if g.suppress_reconnect {
                                g.last_left = None;
                                g.invite.clear();
                                g.suppress_reconnect = false;
                            } else {
                                g.last_left = Some(Instant::now());
                            }
                        }
                        NetEvent::Online { port, addrs } => {
                            if g.role == Role::Idle {
                                g.role = Role::Presence;
                            }
                            g.port = *port;
                            g.addrs = addrs.clone();
                        }
                        NetEvent::Peers(p) => {
                            g.peers = p.clone();
                        }
                        _ => {}
                    }
                    let p = Persist {
                        invite: g.invite.clone(),
                        role: match g.role {
                            Role::Host => "host",
                            Role::Guest => "guest",
                            Role::Presence => "presence",
                            Role::Idle => "idle",
                        }
                        .into(),
                        internet: g.internet,
                        handle: g.handle.clone(),
                    };
                    let root = g.root.clone();
                    drop(g);
                    save_persist(&root, &p);
                }
                let msg = Ipc::Event {
                    event: event_out(&ev),
                };
                let mut g = inner.lock().unwrap();
                g.clients.retain(|c| c.send(msg.clone()).is_ok());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
}

fn job_loop(inner: Arc<Mutex<Inner>>, cmd_tx: Sender<Cmd>, alive: Arc<AtomicBool>) {
    while alive.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));
        let mut g = inner.lock().unwrap();
        let cfg = g.cfg.clone();
        let now = Instant::now();
        if cfg.ping && g.role != Role::Idle && g.last_ping.elapsed() >= PING_EVERY {
            g.last_ping = now;
            let _ = cmd_tx.send(Cmd::Send(Wire::Ping));
        }
        if cfg.refresh_invite
            && g.role == Role::Host
            && g.internet
            && g.last_invite.elapsed() >= INVITE_EVERY
        {
            g.last_invite = now;
            let _ = cmd_tx.send(Cmd::RefreshInvite);
        }
        if cfg.reconnect && g.role == Role::Idle {
            if let Some(at) = g.last_left {
                if at.elapsed() >= RECONNECT_WAIT && !g.invite.is_empty() {
                    let inv = g.invite.clone();
                    g.last_left = None;
                    drop(g);
                    let _ = cmd_tx.send(Cmd::Join(inv));
                    continue;
                }
            }
        }
        if !cfg.keep_table && g.gui == 0 && g.role != Role::Idle {
            drop(g);
            let _ = cmd_tx.send(Cmd::Leave);
            continue;
        }
    }
}

fn snapshot_events(g: &Inner) -> Vec<Ipc> {
    let mut out = Vec::new();
    match g.role {
        Role::Host => {
            out.push(Ipc::Event {
                event: IpcEvent::Hosting {
                    port: g.port,
                    addrs: g.addrs.clone(),
                    internet: g.internet,
                    key: b64(&g.table_key),
                },
            });
            if !g.invite.is_empty() {
                out.push(Ipc::Event {
                    event: IpcEvent::Relay {
                        url: g.invite.clone(),
                    },
                });
            }
        }
        Role::Guest => {
            out.push(Ipc::Event {
                event: IpcEvent::Joined {
                    addr: g.invite.clone(),
                },
            });
        }
        Role::Presence => {
            out.push(Ipc::Event {
                event: IpcEvent::Online {
                    port: g.port,
                    addrs: g.addrs.clone(),
                },
            });
        }
        Role::Idle => {}
    }
    if !g.peers.is_empty() {
        out.push(Ipc::Event {
            event: IpcEvent::Peers {
                peers: g.peers.clone(),
            },
        });
    }
    out
}

fn token_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn serve_gui(
    mut stream: TcpStream,
    token: String,
    inner: Arc<Mutex<Inner>>,
    cmd_tx: Sender<Cmd>,
    alive: Arc<AtomicBool>,
) {
    let _ = stream.set_nodelay(true);
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let hello: Ipc = match serde_json::from_str(line.trim()) {
        Ok(m) => m,
        Err(_) => return,
    };
    match hello {
        Ipc::Hello { token: t, handle } if token_eq(&t, &token) => {
            if !write_ipc(
                &mut stream,
                &Ipc::Welcome {
                    ok: true,
                    err: String::new(),
                },
            ) {
                return;
            }
            let mut g = inner.lock().unwrap();
            g.gui += 1;
            if !handle.is_empty() {
                g.handle = handle.clone();
                let _ = cmd_tx.send(Cmd::SetHandle(handle));
            }
        }
        _ => {
            let _ = write_ipc(
                &mut stream,
                &Ipc::Welcome {
                    ok: false,
                    err: "bad token".into(),
                },
            );
            return;
        }
    }
    let (out_tx, out_rx) = mpsc::channel::<Ipc>();
    let replay = {
        let mut g = inner.lock().unwrap();
        g.clients.push(out_tx.clone());
        snapshot_events(&g)
    };
    let mut writer = stream;
    let flag = alive.clone();
    let write_thr = thread::spawn(move || {
        while flag.load(Ordering::SeqCst) {
            match out_rx.recv_timeout(Duration::from_millis(80)) {
                Ok(msg) => {
                    if !write_ipc(&mut writer, &msg) {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
    });
    for msg in replay {
        let _ = out_tx.send(msg);
    }
    loop {
        if !alive.load(Ordering::SeqCst) {
            break;
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => match serde_json::from_str::<Ipc>(line.trim()) {
                Ok(Ipc::Cmd { cmd }) => {
                    let root = inner.lock().unwrap().root.clone();
                    {
                        let mut g = inner.lock().unwrap();
                        match &cmd {
                            IpcCmd::Join { invite } => {
                                g.invite = invite.clone();
                                g.suppress_reconnect = false;
                                g.last_left = None;
                            }
                            IpcCmd::Host { .. } => {
                                g.last_left = None;
                                g.suppress_reconnect = false;
                            }
                            IpcCmd::Leave => {
                                g.suppress_reconnect = true;
                            }
                            IpcCmd::SetHandle { handle } => {
                                g.handle = handle.clone();
                            }
                            _ => {}
                        }
                    }
                    let _ = cmd_tx.send(cmd_from_ipc(cmd, &root));
                }
                Ok(Ipc::Ping) => {}
                Ok(Ipc::Bye) => break,
                _ => {}
            },
            Err(_) => break,
        }
    }
    {
        let mut g = inner.lock().unwrap();
        g.gui = g.gui.saturating_sub(1);
    }
    drop(write_thr);
}

pub fn print_status(root: &Path) -> i32 {
    match read_lock(root) {
        Some(lock) if try_tcp(lock.port).is_some() || try_tcp(IPC_PORT).is_some() => {
            println!("NODE  up");
            println!("PID   {}", lock.pid);
            println!("PORT  127.0.0.1:{}", lock.port);
            let p = load_persist(root);
            if !p.role.is_empty() {
                println!("ROLE  {}", p.role);
            }
            if !p.invite.is_empty() {
                println!("INVITE {}", p.invite);
            }
            0
        }
        _ => {
            println!("NODE  down");
            1
        }
    }
}

pub fn stop(root: &Path) -> i32 {
    if let Some(lock) = read_lock(root) {
        #[cfg(unix)]
        unsafe {
            libc::kill(lock.pid as i32, libc::SIGTERM);
        }
        #[cfg(windows)]
        {
            let mut c = Command::new("taskkill");
            crate::sys::hide(&mut c);
            let _ = c.args(["/PID", &lock.pid.to_string(), "/F"]).status();
        }
        thread::sleep(Duration::from_millis(200));
        let _ = std::fs::remove_file(lock_path(root));
        println!("NODE  stopped");
        0
    } else {
        println!("NODE  was not running");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_defaults_keep_table_and_reconnect() {
        let c = Cfg::default();
        assert!(c.reconnect && c.keep_table && c.ping && c.refresh_invite);
        assert!(!c.auto_online);
        let s = serde_json::to_string(&c).unwrap();
        let back: Cfg = serde_json::from_str(&s).unwrap();
        assert_eq!(back.reconnect, c.reconnect);
    }

    #[test]
    fn event_roundtrip_chat_and_hosting() {
        let ev = NetEvent::Chat {
            from: "a".into(),
            name: "Ada".into(),
            text: "hi".into(),
            whisper: true,
        };
        let back = event_in(event_out(&ev));
        match back {
            NetEvent::Chat { text, whisper, .. } => {
                assert_eq!(text, "hi");
                assert!(whisper);
            }
            _ => panic!("chat"),
        }
        let ev = NetEvent::Hosting {
            port: 8766,
            addrs: vec!["10.0.0.1:8766".into()],
            internet: true,
            key: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        };
        match event_in(event_out(&ev)) {
            NetEvent::Hosting { port, key, .. } => {
                assert_eq!(port, 8766);
                assert_eq!(key.len(), 16);
            }
            _ => panic!("hosting"),
        }
    }

    #[test]
    fn lock_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bn-daemon-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        write_lock(
            &dir,
            &Lock {
                port: 1234,
                token: "abc".into(),
                pid: 9,
            },
        );
        let l = read_lock(&dir).unwrap();
        assert_eq!(l.port, 1234);
        assert_eq!(l.token, "abc");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_zero_is_dead_and_ipc_is_loopback() {
        assert!(!pid_alive(0));
        let a = ipc_addr(IPC_PORT);
        assert!(a.ip().is_loopback());
        assert_eq!(a.port(), 18766);
        let dir = std::env::temp_dir().join(format!("bn-noup-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        write_lock(
            &dir,
            &Lock {
                port: 1,
                token: "x".into(),
                pid: 0,
            },
        );
        assert!(!try_tcp(1).is_some() && !pid_alive(0));
        clear_stale_lock(&dir);
        assert!(read_lock(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn node_ipc_hello_from_a_client() {
        let dir = std::env::temp_dir().join(format!("bn-ipc-{}", rand::random::<u32>()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let token = b64(b"0123456789abcdef");
        write_lock(
            &dir,
            &Lock {
                port,
                token: token.clone(),
                pid: std::process::id(),
            },
        );
        let stop = Arc::new(AtomicBool::new(true));
        let inner = Arc::new(Mutex::new(Inner {
            cfg: Cfg::default(),
            root: dir.clone(),
            handle: "Host".into(),
            role: Role::Idle,
            invite: String::new(),
            internet: false,
            port: 0,
            addrs: vec![],
            table_key: vec![],
            peers: vec![],
            last_left: None,
            last_ping: Instant::now(),
            last_invite: Instant::now(),
            gui: 0,
            suppress_reconnect: false,
            clients: vec![],
        }));
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let flag = stop.clone();
        thread::spawn(move || {
            let _ = listener.set_nonblocking(true);
            for _ in 0..80 {
                if let Ok((stream, _)) = listener.accept() {
                    serve_gui(stream, token.clone(), inner.clone(), cmd_tx.clone(), flag.clone());
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        let s = try_connect(&dir, "Ada");
        assert!(s.is_some(), "UI should attach to the node on loopback");
        drop(s);
        drop(cmd_rx);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
