//! Table-key + ECDH wire crypto. Invite holders can talk. Packet sniffers cannot.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::io::{BufRead, Write};
use x25519_dalek::{PublicKey, StaticSecret};

const PEPPER: &[u8] = b"BLIGHTNET.INDEX.WIRE.v1";
const INFO: &[u8] = b"blightnet-wire-v1";
pub const KEY_LEN: usize = 16;
const NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct Cipher {
    send: ChaCha20Poly1305,
    recv: ChaCha20Poly1305,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite {
    pub key: Vec<u8>,
    pub addrs: Vec<String>,
    pub web: Option<String>,
}

pub fn mint_key() -> Vec<u8> {
    let mut k = vec![0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut k);
    k
}

pub fn encode_key(key: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(key)
}

pub fn decode_key(s: &str) -> Option<Vec<u8>> {
    let b = URL_SAFE_NO_PAD.decode(s.trim()).ok()?;
    if b.len() == KEY_LEN {
        Some(b)
    } else {
        None
    }
}

pub fn encode_invite(key: &[u8], addrs: &[String]) -> String {
    let mut unique = Vec::new();
    for a in addrs {
        if !a.is_empty() && !unique.iter().any(|x: &String| x == a) {
            unique.push(a.clone());
        }
    }
    if unique.is_empty() {
        unique.push("127.0.0.1:8766".into());
    }
    if key.len() == KEY_LEN {
        format!("blightnet://{}@{}", encode_key(key), unique.join(","))
    } else {
        format!("blightnet://{}", unique.join(","))
    }
}

pub fn parse_invite(raw: &str) -> Option<Invite> {
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
    let (key, rest) = if let Some((k, rest)) = s.split_once('@') {
        (decode_key(k).unwrap_or_default(), rest)
    } else {
        (vec![], s)
    };
    let mut addrs = Vec::new();
    for part in rest.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if p.contains(':') {
            addrs.push(p.to_string());
        } else {
            addrs.push(format!("{p}:{}", crate::net::DEFAULT_PORT));
        }
    }
    if addrs.is_empty() {
        return None;
    }
    Some(Invite {
        key,
        addrs,
        web: None,
    })
}

impl Cipher {
    pub fn seal_line(&self, msg: &impl serde::Serialize) -> Option<String> {
        let pt = serde_json::to_vec(msg).ok()?;
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ct = self.send.encrypt(Nonce::from_slice(&nonce), pt.as_slice()).ok()?;
        let mut blob = Vec::with_capacity(NONCE_LEN + ct.len());
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        Some(format!("BN1.{}", URL_SAFE_NO_PAD.encode(blob)))
    }

    pub fn open_line<T: serde::de::DeserializeOwned>(&self, line: &str) -> Option<T> {
        let rest = line.trim().strip_prefix("BN1.")?;
        let blob = URL_SAFE_NO_PAD.decode(rest).ok()?;
        if blob.len() < NONCE_LEN + 16 {
            return None;
        }
        let pt = self
            .recv
            .decrypt(Nonce::from_slice(&blob[..NONCE_LEN]), &blob[NONCE_LEN..])
            .ok()?;
        serde_json::from_slice(&pt).ok()
    }
}

pub fn write_sealed<W: Write>(w: &mut W, cipher: &Cipher, msg: &impl serde::Serialize) -> bool {
    let Some(mut line) = cipher.seal_line(msg) else {
        return false;
    };
    line.push('\n');
    w.write_all(line.as_bytes()).is_ok() && w.flush().is_ok()
}

fn parse_hello_line(line: &str) -> Option<([u8; 32], [u8; 16])> {
    let t = line.trim();
    let mut it = t.split_whitespace();
    if it.next()? != "BN1" {
        return None;
    }
    if it.next()? != "HELLO" {
        return None;
    }
    let pk = URL_SAFE_NO_PAD.decode(it.next()?).ok()?;
    let n = URL_SAFE_NO_PAD.decode(it.next()?).ok()?;
    if pk.len() != 32 || n.len() != 16 {
        return None;
    }
    let mut pkb = [0u8; 32];
    pkb.copy_from_slice(&pk);
    let mut nb = [0u8; 16];
    nb.copy_from_slice(&n);
    Some((pkb, nb))
}

fn hello_line(pk: &PublicKey, nonce: &[u8; 16]) -> String {
    format!(
        "BN1 HELLO {} {}\n",
        URL_SAFE_NO_PAD.encode(pk.as_bytes()),
        URL_SAFE_NO_PAD.encode(nonce)
    )
}

fn new_secret() -> (StaticSecret, PublicKey, [u8; 16]) {
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let secret = StaticSecret::from(seed);
    let public = PublicKey::from(&secret);
    let mut n = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut n);
    (secret, public, n)
}

fn derive(
    shared: &[u8],
    table_key: &[u8],
    client_n: &[u8],
    server_n: &[u8],
    we_are_server: bool,
) -> Option<Cipher> {
    let mut ikm = Vec::with_capacity(shared.len() + table_key.len() + PEPPER.len());
    ikm.extend_from_slice(shared);
    ikm.extend_from_slice(table_key);
    ikm.extend_from_slice(PEPPER);
    let mut salt = Vec::with_capacity(32);
    salt.extend_from_slice(client_n);
    salt.extend_from_slice(server_n);
    let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
    let mut okm = [0u8; 64];
    hk.expand(INFO, &mut okm).ok()?;
    let (c2s, s2c) = okm.split_at(32);
    let (send, recv) = if we_are_server {
        (s2c, c2s)
    } else {
        (c2s, s2c)
    };
    Some(Cipher {
        send: ChaCha20Poly1305::new_from_slice(send).ok()?,
        recv: ChaCha20Poly1305::new_from_slice(recv).ok()?,
    })
}

pub fn handshake_server(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    table_key: &[u8],
) -> Option<Cipher> {
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let (peer_pk, client_n) = parse_hello_line(&line)?;
    let (secret, public, server_n) = new_secret();
    writer.write_all(hello_line(&public, &server_n).as_bytes()).ok()?;
    writer.flush().ok()?;
    let shared = secret.diffie_hellman(&PublicKey::from(peer_pk));
    derive(shared.as_bytes(), table_key, &client_n, &server_n, true)
}

pub fn handshake_client(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    table_key: &[u8],
) -> Option<Cipher> {
    let (secret, public, client_n) = new_secret();
    writer.write_all(hello_line(&public, &client_n).as_bytes()).ok()?;
    writer.flush().ok()?;
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let (peer_pk, server_n) = parse_hello_line(&line)?;
    let shared = secret.diffie_hellman(&PublicKey::from(peer_pk));
    derive(shared.as_bytes(), table_key, &client_n, &server_n, false)
}

pub fn ws_hello_text(pk: &PublicKey, nonce: &[u8; 16]) -> String {
    hello_line(pk, nonce).trim_end().to_string()
}

pub fn start_client_hello() -> (StaticSecret, PublicKey, [u8; 16]) {
    new_secret()
}

pub fn finish_client(
    secret: StaticSecret,
    client_n: &[u8; 16],
    line: &str,
    table_key: &[u8],
) -> Option<Cipher> {
    let (peer_pk, server_n) = parse_hello_line(line)?;
    let shared = secret.diffie_hellman(&PublicKey::from(peer_pk));
    derive(shared.as_bytes(), table_key, client_n, &server_n, false)
}

pub fn finish_server(
    line: &str,
    table_key: &[u8],
) -> Option<(Cipher, String)> {
    let (peer_pk, client_n) = parse_hello_line(line)?;
    let (secret, public, server_n) = new_secret();
    let shared = secret.diffie_hellman(&PublicKey::from(peer_pk));
    let cipher = derive(shared.as_bytes(), table_key, &client_n, &server_n, true)?;
    Some((cipher, ws_hello_text(&public, &server_n)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn invite_roundtrip_and_legacy() {
        let key = mint_key();
        let s = encode_invite(&key, &["10.0.0.4:8766".into(), "203.0.113.9:8766".into()]);
        let inv = parse_invite(&s).unwrap();
        assert_eq!(inv.key, key);
        assert_eq!(inv.addrs.len(), 2);
        let legacy = parse_invite("blightnet://192.168.1.9:8766").unwrap();
        assert!(legacy.key.is_empty());
        assert_eq!(legacy.addrs, vec!["192.168.1.9:8766"]);
        assert!(parse_invite("https://foo.trycloudflare.com").is_none());
        let u = encode_invite(&key, &["10.0.0.1:8766".into(), "udp:9.9.9.9:40111".into()]);
        let p = parse_invite(&u).unwrap();
        assert!(p.addrs.iter().any(|a| a.starts_with("udp:")));
    }

    #[test]
    fn handshake_and_seal_over_buffers() {
        let key = mint_key();
        let (sec, pubk, cn) = new_secret();
        let client_hello = hello_line(&pubk, &cn);
        let mut server_out = Vec::new();
        let mut server_in = Cursor::new(client_hello.into_bytes());
        let server = handshake_server(&mut server_in, &mut server_out, &key).unwrap();
        let reply = String::from_utf8(server_out).unwrap();
        let client = finish_client(sec, &cn, &reply, &key).unwrap();
        let line = client.seal_line(&serde_json::json!({"type":"ping"})).unwrap();
        let back: serde_json::Value = server.open_line(&line).unwrap();
        assert_eq!(back["type"], "ping");
        let line = server.seal_line(&serde_json::json!({"type":"pong"})).unwrap();
        let back: serde_json::Value = client.open_line(&line).unwrap();
        assert_eq!(back["type"], "pong");
        assert!(server.open_line::<serde_json::Value>("{\"type\":\"ping\"}").is_none());
    }

    #[test]
    fn wrong_table_key_cannot_open() {
        let a = mint_key();
        let b = mint_key();
        let (sec, pubk, cn) = new_secret();
        let hello = hello_line(&pubk, &cn);
        let (server, reply) = finish_server(&hello, &a).unwrap();
        let client = finish_client(sec, &cn, &reply, &b).unwrap();
        let line = client.seal_line(&serde_json::json!({"type":"chat","text":"secret"})).unwrap();
        assert!(server.open_line::<serde_json::Value>(&line).is_none());
    }
}
