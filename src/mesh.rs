//! Daemon-to-daemon internet path: UDP hole punch + encrypted frames.
//! No Cloudflare, no extra binary. The host node listens; the guest node punches.

use crate::crypt::Cipher;
use crate::net::{self, NetEvent, PeerInfo, Wire};
use rand::RngCore;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MAGIC: &[u8; 4] = b"BNU1";
const HELLO: u8 = 0;
const DATA: u8 = 1;
const ACK: u8 = 2;
const CHUNK: usize = 1000;

type ClientMap = net::ClientMap;
type Roster = Arc<Mutex<Vec<PeerInfo>>>;

fn pack(kind: u8, seq: u32, frag: u8, frags: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(10 + payload.len());
    v.extend_from_slice(MAGIC);
    v.push(kind);
    v.extend_from_slice(&seq.to_be_bytes());
    v.push(frag);
    v.push(frags);
    v.extend_from_slice(payload);
    v
}

fn unpack(buf: &[u8]) -> Option<(u8, u32, u8, u8, &[u8])> {
    if buf.len() < 11 || &buf[..4] != MAGIC {
        return None;
    }
    let kind = buf[4];
    let seq = u32::from_be_bytes([buf[5], buf[6], buf[7], buf[8]]);
    Some((kind, seq, buf[9], buf[10], &buf[11..]))
}

pub fn is_udp_slot(addr: &str) -> bool {
    addr.starts_with("udp:") || addr.starts_with("UDP:")
}

pub fn tcp_slots(addrs: &[String]) -> Vec<String> {
    addrs
        .iter()
        .filter(|a| !is_udp_slot(a))
        .cloned()
        .collect()
}

pub fn udp_targets(addrs: &[String]) -> Vec<SocketAddr> {
    let mut out = Vec::new();
    for a in addrs {
        let raw = a.strip_prefix("udp:").or_else(|| a.strip_prefix("UDP:")).unwrap_or(a);
        if let Ok(iter) = raw.to_socket_addrs() {
            for sa in iter {
                if !out.contains(&sa) {
                    out.push(sa);
                }
            }
        }
    }
    out
}

pub fn stun_mapped(sock: &UdpSocket) -> Option<(String, u16)> {
    for host in ["stun.l.google.com:19302", "stun.cloudflare.com:3478"] {
        if let Some(m) = stun_on(sock, host) {
            return Some(m);
        }
    }
    None
}

fn stun_on(sock: &UdpSocket, host: &str) -> Option<(String, u16)> {
    let prev = sock.read_timeout().ok().flatten();
    let _ = sock.set_read_timeout(Some(Duration::from_millis(900)));
    let mut req = [0u8; 20];
    req[0] = 0x00;
    req[1] = 0x01;
    req[4] = 0x21;
    req[5] = 0x12;
    req[6] = 0xa4;
    req[7] = 0x42;
    rand::thread_rng().fill_bytes(&mut req[8..20]);
    sock.send_to(&req, host).ok()?;
    let mut buf = [0u8; 256];
    let n = sock.recv(&mut buf).ok()?;
    let _ = sock.set_read_timeout(prev);
    parse_mapped(&buf[..n])
}

pub fn stun_mapped_ip_from_buf(buf: &[u8]) -> Option<String> {
    parse_mapped(buf).map(|(ip, _)| ip)
}

fn parse_mapped(buf: &[u8]) -> Option<(String, u16)> {
    if buf.len() < 20 {
        return None;
    }
    let mut i = 20usize;
    while i + 4 <= buf.len() {
        let typ = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let start = i + 4;
        let end = start.saturating_add(len);
        if end > buf.len() {
            break;
        }
        let body = &buf[start..end];
        if (typ == 0x0020 || typ == 0x0001) && body.len() >= 8 && body[1] == 0x01 {
            let xor = typ == 0x0020;
            let mut port = u16::from_be_bytes([body[2], body[3]]);
            let mut ip = [body[4], body[5], body[6], body[7]];
            if xor {
                port ^= 0x2112;
                ip[0] ^= 0x21;
                ip[1] ^= 0x12;
                ip[2] ^= 0xa4;
                ip[3] ^= 0x42;
            }
            return Some((format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]), port));
        }
        i = end + (4 - len % 4) % 4;
    }
    None
}

struct Frag {
    parts: Vec<Option<Vec<u8>>>,
}

struct Sess {
    cipher: Cipher,
    wrx_alive: Arc<AtomicBool>,
    inbox: HashMap<u32, Frag>,
    cid: String,
}

fn send_msg(sock: &UdpSocket, dest: SocketAddr, cipher: &Cipher, seq: u32, msg: &Wire) -> bool {
    let Some(line) = cipher.seal_line(msg) else {
        return false;
    };
    let bytes = line.as_bytes();
    let n = (bytes.len() + CHUNK - 1) / CHUNK;
    let n = n.max(1) as u8;
    for i in 0..n {
        let start = i as usize * CHUNK;
        let end = (start + CHUNK).min(bytes.len());
        let pkt = pack(DATA, seq, i, n, &bytes[start..end]);
        if sock.send_to(&pkt, dest).is_err() {
            return false;
        }
    }
    true
}

fn send_hello(sock: &UdpSocket, dest: SocketAddr, text: &str) -> bool {
    sock.send_to(&pack(HELLO, 0, 0, 1, text.as_bytes()), dest)
        .is_ok()
}

fn send_ack(sock: &UdpSocket, dest: SocketAddr, seq: u32) {
    let _ = sock.send_to(&pack(ACK, seq, 0, 1, &[]), dest);
}

fn take_complete(inbox: &mut HashMap<u32, Frag>, seq: u32, frag: u8, frags: u8, payload: &[u8]) -> Option<String> {
    let e = inbox.entry(seq).or_insert_with(|| Frag {
        parts: vec![None; frags.max(1) as usize],
    });
    let i = frag as usize;
    if i >= e.parts.len() {
        return None;
    }
    e.parts[i] = Some(payload.to_vec());
    if e.parts.iter().all(|p| p.is_some()) {
        let mut all = Vec::new();
        for p in e.parts.drain(..) {
            all.extend(p.unwrap_or_default());
        }
        inbox.remove(&seq);
        String::from_utf8(all).ok()
    } else {
        None
    }
}

pub fn bind_mesh(port: u16) -> Option<UdpSocket> {
    UdpSocket::bind(("0.0.0.0", port))
        .or_else(|_| UdpSocket::bind("0.0.0.0:0"))
        .ok()
}

pub fn spawn_host(
    sock: UdpSocket,
    clients: ClientMap,
    roster: Roster,
    ev_tx: std::sync::mpsc::Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
    table_key: Arc<Vec<u8>>,
) {
    let _ = sock.set_read_timeout(Some(Duration::from_millis(40)));
    thread::Builder::new()
        .name("blightnet-mesh-host".into())
        .spawn(move || {
            host_loop(sock, clients, roster, ev_tx, host_id, host_name, stop, table_key);
        })
        .ok();
}

fn host_loop(
    sock: UdpSocket,
    clients: ClientMap,
    roster: Roster,
    ev_tx: std::sync::mpsc::Sender<NetEvent>,
    host_id: String,
    host_name: String,
    stop: Arc<AtomicBool>,
    table_key: Arc<Vec<u8>>,
) {
    let mut sessions: HashMap<SocketAddr, Sess> = HashMap::new();
    let mut buf = [0u8; 1400];
    while !stop.load(Ordering::SeqCst) {
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => {
                let Some((kind, seq, frag, frags, payload)) = unpack(&buf[..n]) else {
                    continue;
                };
                if kind == HELLO {
                    if let Some(sess) = sessions.get(&from) {
                        if sess.cid.is_empty() {
                            continue;
                        }
                    }
                    if let Some(old) = sessions.remove(&from) {
                        old.wrx_alive.store(false, Ordering::SeqCst);
                    }
                    let text = String::from_utf8_lossy(payload);
                    if let Some((cipher, reply)) = crate::crypt::finish_server(text.as_ref(), &table_key)
                    {
                        let _ = send_hello(&sock, from, &reply);
                        sessions.insert(
                            from,
                            Sess {
                                cipher,
                                wrx_alive: Arc::new(AtomicBool::new(true)),
                                inbox: HashMap::new(),
                                cid: String::new(),
                            },
                        );
                    }
                    continue;
                }
                let Some(sess) = sessions.get_mut(&from) else {
                    continue;
                };
                if kind == ACK {
                    continue;
                }
                if kind != DATA {
                    continue;
                }
                send_ack(&sock, from, seq);
                let Some(line) = take_complete(&mut sess.inbox, seq, frag, frags, payload) else {
                    continue;
                };
                let Some(msg) = sess.cipher.open_line::<Wire>(&line) else {
                    continue;
                };
                if sess.cid.is_empty() {
                    if let Wire::Hello { id, name, .. } = &msg {
                        let (wtx, wrx) = net::wire_chan();
                        sess.cid = id.clone();
                        sess.wrx_alive = Arc::new(AtomicBool::new(true));
                        net::finish_join(
                            id.clone(),
                            name.clone(),
                            wtx,
                            &clients,
                            &roster,
                            &ev_tx,
                            &host_id,
                            &host_name,
                        );
                        let sock_c = sock.try_clone().ok();
                        let dest = from;
                        let cipher = sess.cipher.clone();
                        let live = sess.wrx_alive.clone();
                        if let Some(sock_c) = sock_c {
                            thread::spawn(move || {
                                pump_out(sock_c, dest, cipher, wrx, live);
                            });
                        }
                    }
                    continue;
                }
                let cid = sess.cid.clone();
                net::host_incoming(msg, &cid, &host_id, &clients, &ev_tx);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => thread::sleep(Duration::from_millis(40)),
        }
    }
}

fn pump_out(
    sock: UdpSocket,
    dest: SocketAddr,
    cipher: Cipher,
    wrx: Receiver<Wire>,
    live: Arc<AtomicBool>,
) {
    let mut seq = 1u32;
    while live.load(Ordering::SeqCst) {
        let Some(batch) = net::recv_batch(&wrx) else {
            break;
        };
        for msg in batch {
            let s = seq;
            seq = seq.wrapping_add(1);
            let _ = send_msg(&sock, dest, &cipher, s, &msg);
            thread::sleep(Duration::from_millis(4));
            let _ = send_msg(&sock, dest, &cipher, s, &msg);
        }
    }
}

pub fn join_guest(
    targets: &[SocketAddr],
    table_key: &[u8],
    hello: Wire,
    wrx: Receiver<Wire>,
    ev_tx: std::sync::mpsc::Sender<NetEvent>,
    stop: Arc<AtomicBool>,
    self_id: String,
) -> bool {
    if targets.is_empty() {
        return false;
    }
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
        return false;
    };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(80)));
    let _ = stun_mapped(&sock);
    let (secret, public, nonce) = crate::crypt::start_client_hello();
    let hello_txt = crate::crypt::ws_hello_text(&public, &nonce);
    let t0 = Instant::now();
    let mut reply: Option<(SocketAddr, String)> = None;
    let mut buf = [0u8; 1400];
    while t0.elapsed() < Duration::from_secs(5) && !stop.load(Ordering::SeqCst) {
        for dest in targets {
            let _ = send_hello(&sock, *dest, &hello_txt);
        }
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => {
                if let Some((HELLO, _, _, _, payload)) = unpack(&buf[..n]) {
                    reply = Some((from, String::from_utf8_lossy(payload).into_owned()));
                    break;
                }
            }
            Err(_) => {}
        }
    }
    let Some((dest, text)) = reply else {
        return false;
    };
    let Some(cipher) = crate::crypt::finish_client(secret, &nonce, &text, table_key) else {
        return false;
    };
    if !send_msg(&sock, dest, &cipher, 1, &hello) {
        return false;
    }
    let _ = send_msg(&sock, dest, &cipher, 1, &hello);
    let live = Arc::new(AtomicBool::new(true));
    let sock_out = sock.try_clone().ok();
    let cipher_out = cipher.clone();
    let live_out = live.clone();
    if let Some(sock_out) = sock_out {
        thread::spawn(move || {
            pump_out(sock_out, dest, cipher_out, wrx, live_out);
        });
    }
    let stop_c = stop.clone();
    let live_c = live.clone();
    thread::spawn(move || {
        let mut inbox: HashMap<u32, Frag> = HashMap::new();
        let mut buf = [0u8; 1400];
        while !stop_c.load(Ordering::SeqCst) && live_c.load(Ordering::SeqCst) {
            match sock.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if from != dest {
                        continue;
                    }
                    let Some((kind, seq, frag, frags, payload)) = unpack(&buf[..n]) else {
                        continue;
                    };
                    if kind == ACK {
                        continue;
                    }
                    if kind != DATA {
                        continue;
                    }
                    send_ack(&sock, dest, seq);
                    let Some(line) = take_complete(&mut inbox, seq, frag, frags, payload) else {
                        continue;
                    };
                    if let Some(msg) = cipher.open_line::<Wire>(&line) {
                        net::guest_incoming(msg, &self_id, &ev_tx);
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
        live_c.store(false, Ordering::SeqCst);
        let _ = ev_tx.send(NetEvent::Left);
        let _ = ev_tx.send(NetEvent::Status("Offline".into()));
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_roundtrip() {
        let p = pack(DATA, 9, 1, 3, b"abc");
        let (k, seq, f, n, body) = unpack(&p).unwrap();
        assert_eq!(k, DATA);
        assert_eq!(seq, 9);
        assert_eq!(f, 1);
        assert_eq!(n, 3);
        assert_eq!(body, b"abc");
        assert!(unpack(b"nope").is_none());
    }

    #[test]
    fn tcp_udp_slot_split() {
        let addrs = vec![
            "10.0.0.4:8766".into(),
            "udp:203.0.113.9:40111".into(),
            "1.2.3.4:8766".into(),
        ];
        let t = tcp_slots(&addrs);
        assert_eq!(t.len(), 2);
        assert!(is_udp_slot("udp:1.2.3.4:9"));
        assert!(!is_udp_slot("1.2.3.4:9"));
        let u = udp_targets(&addrs);
        assert!(u.iter().any(|a| a.port() == 40111));
        assert!(u.iter().any(|a| a.port() == 8766));
    }

    #[test]
    fn mesh_localhost_guest_jacks_in() {
        use crate::net::{PeerInfo, Wire};
        use std::collections::HashMap;
        use std::sync::mpsc;
        let key = crate::crypt::mint_key();
        let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = sock.local_addr().unwrap();
        let clients: crate::net::ClientMap = Arc::new(Mutex::new(HashMap::new()));
        let roster = Arc::new(Mutex::new(vec![PeerInfo {
            id: "host".into(),
            name: "Host".into(),
        }]));
        let (ev_tx, ev_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        spawn_host(
            sock,
            clients,
            roster,
            ev_tx,
            "host".into(),
            "Host".into(),
            stop.clone(),
            Arc::new(key.clone()),
        );
        let (_wtx, wrx) = crate::net::wire_chan();
        let (gev_tx, _gev_rx) = mpsc::channel();
        let hello = Wire::Hello {
            id: "guest".into(),
            name: "Guest".into(),
            role: "guest".into(),
        };
        assert!(
            join_guest(&[addr], &key, hello, wrx, gev_tx, stop.clone(), "guest".into()),
            "UDP mesh join failed on loopback"
        );
        let mut saw = false;
        for _ in 0..80 {
            while let Ok(ev) = ev_rx.try_recv() {
                match ev {
                    crate::net::NetEvent::Status(s) if s.contains("jacked in") => saw = true,
                    crate::net::NetEvent::Peers(p) if p.iter().any(|x| x.id == "guest") => {
                        saw = true;
                    }
                    _ => {}
                }
            }
            if saw {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        stop.store(true, Ordering::SeqCst);
        assert!(saw, "host node did not see the guest over UDP");
    }
}
