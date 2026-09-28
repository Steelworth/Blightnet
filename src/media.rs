//! Opus voice frames + binary media packet layout (Phase A #2).
//!
//! On-wire AEAD plaintext (ver 1), same on mesh UDP MEDIA and LAN TCP BNM1:
//! ```text
//! u8  ver=1
//! u8  flags   bit0=has_to  bit1=has_crew
//! u8  from_len + from UTF-8
//! [u8 to_len + to] if has_to
//! [u8 crew_len + crew] if has_crew
//! remaining = Opus packet
//! ```
//!
//! Mesh (BNU1 kind=MEDIA): `BNU1 | MEDIA | seq_be32 | frag | frags | nonce(12)+ct+tag`
//! LAN TCP: length-prefixed `BNM1 | len_be32 | nonce(12)+ct+tag` interleaved with BN1 wire lines.
//! Session ChaCha20-Poly1305 (compact binary AEAD — not JSON seal_line).
//! Decode at the net edge → `NetEvent::VoicePcm` so the desk still plays PCM.

use std::os::raw::c_int;

pub const SAMPLE_RATE: u32 = 16_000;
pub const FRAME_MS: u32 = 20;
pub const FRAME_SAMPLES: usize = (SAMPLE_RATE as usize * FRAME_MS as usize) / 1000; // 320
pub const MEDIA_VER: u8 = 1;
const FLAG_TO: u8 = 1;
const FLAG_CREW: u8 = 2;
const MAX_ID: usize = 64;
const MAX_OPUS: usize = 400;
const OPUS_APPLICATION_VOIP: c_int = 2048;
const OPUS_OK: c_int = 0;

#[repr(C)]
struct OpusEncoder {
    _private: [u8; 0],
}
#[repr(C)]
struct OpusDecoder {
    _private: [u8; 0],
}

#[link(name = "opus")]
extern "C" {
    fn opus_encoder_create(
        fs: c_int,
        channels: c_int,
        application: c_int,
        error: *mut c_int,
    ) -> *mut OpusEncoder;
    fn opus_encoder_destroy(st: *mut OpusEncoder);
    fn opus_encode_float(
        st: *mut OpusEncoder,
        pcm: *const f32,
        frame_size: c_int,
        data: *mut u8,
        max_data_bytes: c_int,
    ) -> c_int;
    fn opus_decoder_create(fs: c_int, channels: c_int, error: *mut c_int) -> *mut OpusDecoder;
    fn opus_decoder_destroy(st: *mut OpusDecoder);
    fn opus_decode_float(
        st: *mut OpusDecoder,
        data: *const u8,
        len: c_int,
        pcm: *mut f32,
        frame_size: c_int,
        decode_fec: c_int,
    ) -> c_int;
}

pub struct Encoder {
    ptr: *mut OpusEncoder,
}

unsafe impl Send for Encoder {}

impl Encoder {
    pub fn new() -> Option<Self> {
        let mut err: c_int = 0;
        let ptr = unsafe {
            opus_encoder_create(SAMPLE_RATE as c_int, 1, OPUS_APPLICATION_VOIP, &mut err)
        };
        if ptr.is_null() || err != OPUS_OK {
            return None;
        }
        Some(Self { ptr })
    }

    /// Encode one 20 ms mono float frame → Opus packet.
    pub fn encode(&mut self, pcm: &[f32]) -> Option<Vec<u8>> {
        if pcm.len() != FRAME_SAMPLES {
            return None;
        }
        let mut out = vec![0u8; MAX_OPUS];
        let n = unsafe {
            opus_encode_float(
                self.ptr,
                pcm.as_ptr(),
                FRAME_SAMPLES as c_int,
                out.as_mut_ptr(),
                out.len() as c_int,
            )
        };
        if n < 0 {
            return None;
        }
        out.truncate(n as usize);
        Some(out)
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { opus_encoder_destroy(self.ptr) };
            self.ptr = std::ptr::null_mut();
        }
    }
}

pub struct Decoder {
    ptr: *mut OpusDecoder,
}

unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new() -> Option<Self> {
        let mut err: c_int = 0;
        let ptr = unsafe { opus_decoder_create(SAMPLE_RATE as c_int, 1, &mut err) };
        if ptr.is_null() || err != OPUS_OK {
            return None;
        }
        Some(Self { ptr })
    }

    pub fn decode(&mut self, opus: &[u8]) -> Option<Vec<f32>> {
        if opus.is_empty() || opus.len() > MAX_OPUS {
            return None;
        }
        let mut pcm = vec![0f32; FRAME_SAMPLES];
        let n = unsafe {
            opus_decode_float(
                self.ptr,
                opus.as_ptr(),
                opus.len() as c_int,
                pcm.as_mut_ptr(),
                FRAME_SAMPLES as c_int,
                0,
            )
        };
        if n < 0 {
            return None;
        }
        pcm.truncate(n as usize);
        Some(pcm)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { opus_decoder_destroy(self.ptr) };
            self.ptr = std::ptr::null_mut();
        }
    }
}

/// One call-scoped Opus frame (pre-AEAD).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaFrame {
    pub from: String,
    pub to: Option<String>,
    pub crew: Option<String>,
    pub opus: Vec<u8>,
}

impl MediaFrame {
    pub fn pack(&self) -> Option<Vec<u8>> {
        if self.from.is_empty() || self.from.len() > MAX_ID || self.opus.is_empty() || self.opus.len() > MAX_OPUS
        {
            return None;
        }
        if self.to.as_ref().map(|s| s.len() > MAX_ID || s.is_empty()).unwrap_or(false) {
            return None;
        }
        if self.crew.as_ref().map(|s| s.len() > MAX_ID || s.is_empty()).unwrap_or(false) {
            return None;
        }
        let mut flags = 0u8;
        if self.to.is_some() {
            flags |= FLAG_TO;
        }
        if self.crew.is_some() {
            flags |= FLAG_CREW;
        }
        let mut v = Vec::with_capacity(4 + self.from.len() + self.opus.len() + 64);
        v.push(MEDIA_VER);
        v.push(flags);
        v.push(self.from.len() as u8);
        v.extend_from_slice(self.from.as_bytes());
        if let Some(t) = &self.to {
            v.push(t.len() as u8);
            v.extend_from_slice(t.as_bytes());
        }
        if let Some(c) = &self.crew {
            v.push(c.len() as u8);
            v.extend_from_slice(c.as_bytes());
        }
        v.extend_from_slice(&self.opus);
        Some(v)
    }

    pub fn unpack(buf: &[u8]) -> Option<Self> {
        if buf.len() < 4 || buf[0] != MEDIA_VER {
            return None;
        }
        let flags = buf[1];
        let fl = buf[2] as usize;
        let mut i = 3usize;
        if fl == 0 || fl > MAX_ID || i + fl > buf.len() {
            return None;
        }
        let from = std::str::from_utf8(&buf[i..i + fl]).ok()?.to_string();
        i += fl;
        let to = if flags & FLAG_TO != 0 {
            if i >= buf.len() {
                return None;
            }
            let n = buf[i] as usize;
            i += 1;
            if n == 0 || n > MAX_ID || i + n > buf.len() {
                return None;
            }
            let s = std::str::from_utf8(&buf[i..i + n]).ok()?.to_string();
            i += n;
            Some(s)
        } else {
            None
        };
        let crew = if flags & FLAG_CREW != 0 {
            if i >= buf.len() {
                return None;
            }
            let n = buf[i] as usize;
            i += 1;
            if n == 0 || n > MAX_ID || i + n > buf.len() {
                return None;
            }
            let s = std::str::from_utf8(&buf[i..i + n]).ok()?.to_string();
            i += n;
            Some(s)
        } else {
            None
        };
        if i >= buf.len() || buf.len() - i > MAX_OPUS {
            return None;
        }
        Some(Self {
            from,
            to,
            crew,
            opus: buf[i..].to_vec(),
        })
    }
}

pub fn decode_opus_pcm(opus: &[u8]) -> Vec<f32> {
    let Some(mut dec) = Decoder::new() else {
        return vec![];
    };
    dec.decode(opus).unwrap_or_default()
}

/// Silence / tone helpers for tests.
pub fn sine_frame(hz: f32, amp: f32) -> Vec<f32> {
    (0..FRAME_SAMPLES)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            (2.0 * std::f32::consts::PI * hz * t).sin() * amp
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::mint_key;

    #[test]
    fn opus_encode_decode_roundtrip() {
        let mut enc = Encoder::new().expect("opus encoder (libopus)");
        let mut dec = Decoder::new().expect("opus decoder");
        let pcm = sine_frame(440.0, 0.4);
        let pkt = enc.encode(&pcm).expect("encode");
        assert!(pkt.len() > 10 && pkt.len() < MAX_OPUS);
        let back = dec.decode(&pkt).expect("decode");
        assert_eq!(back.len(), FRAME_SAMPLES);
        // Energy should survive lossy encode.
        let e: f32 = back.iter().map(|s| s * s).sum::<f32>() / back.len() as f32;
        assert!(e > 0.01, "decoded energy too low: {e}");
    }

    #[test]
    fn media_frame_pack_unpack() {
        let f = MediaFrame {
            from: "p-abc".into(),
            to: Some("p-xyz".into()),
            crew: Some("crew-1".into()),
            opus: vec![1, 2, 3, 4, 5],
        };
        let pt = f.pack().unwrap();
        let back = MediaFrame::unpack(&pt).unwrap();
        assert_eq!(back, f);
        let open = MediaFrame {
            from: "host".into(),
            to: None,
            crew: None,
            opus: vec![9, 8, 7],
        };
        assert_eq!(MediaFrame::unpack(&open.pack().unwrap()).unwrap(), open);
        assert!(MediaFrame::unpack(b"xx").is_none());
    }

    #[test]
    fn media_aead_roundtrip_with_session_cipher() {
        let key = mint_key();
        let (sec, pubk, cn) = crate::crypt::start_client_hello();
        let hello = crate::crypt::ws_hello_text(&pubk, &cn);
        let (server, reply) = crate::crypt::finish_server(&hello, &key).unwrap();
        let client = crate::crypt::finish_client(sec, &cn, &reply, &key).unwrap();
        let frame = MediaFrame {
            from: "a".into(),
            to: Some("b".into()),
            crew: None,
            opus: vec![42; 40],
        };
        let pt = frame.pack().unwrap();
        let sealed = client.seal_bin(&pt).unwrap();
        assert_ne!(sealed, pt);
        let opened = server.open_bin(&sealed).unwrap();
        assert_eq!(MediaFrame::unpack(&opened).unwrap(), frame);
        // Wrong direction / garbage fails closed.
        assert!(server.open_bin(&pt).is_none());
        let _ = client;
    }
}
