//! Acoustic echo cancellation on the capture path (Phase A #3).
//!
//! Mic PCM → AEC (far-end playout as reference) → Opus encode.
//! Prefer system **libspeexdsp** when `build.rs` finds it (`blightnet_has_aec`).
//! Without the lib, [`Aec`] is a no-op passthrough so voice still works.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::media::{FRAME_SAMPLES, SAMPLE_RATE};

/// ~200 ms filter at 16 kHz (Speex recommends 100–500 ms).
const FILTER_LEN: i32 = (SAMPLE_RATE as i32 * 200) / 1000;

static WARNED_STUB: AtomicBool = AtomicBool::new(false);

fn f32_to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * 32767.0) as i16
}

fn i16_to_f32(s: i16) -> f32 {
    s as f32 / 32767.0
}

/// True when linked against libspeexdsp (build-time).
pub fn is_live() -> bool {
    cfg!(blightnet_has_aec)
}

/// Log once if running without Speex AEC (stub path).
pub fn warn_if_stub() {
    if is_live() {
        return;
    }
    if !WARNED_STUB.swap(true, Ordering::Relaxed) {
        eprintln!("blightnet: AEC unavailable (libspeexdsp not linked); voice continues without echo cancel");
    }
}

#[cfg(blightnet_has_aec)]
mod speex {
    use super::*;
    use std::os::raw::c_int;

    #[repr(C)]
    struct SpeexEchoState {
        _private: [u8; 0],
    }

    const SPEEX_ECHO_SET_SAMPLING_RATE: c_int = 24;

    #[link(name = "speexdsp")]
    extern "C" {
        fn speex_echo_state_init(frame_size: c_int, filter_length: c_int) -> *mut SpeexEchoState;
        fn speex_echo_state_destroy(st: *mut SpeexEchoState);
        fn speex_echo_playback(st: *mut SpeexEchoState, play: *const i16);
        fn speex_echo_capture(st: *mut SpeexEchoState, rec: *const i16, out: *mut i16);
        fn speex_echo_ctl(st: *mut SpeexEchoState, request: c_int, ptr: *mut std::ffi::c_void) -> c_int;
        fn speex_echo_state_reset(st: *mut SpeexEchoState);
    }

    pub struct Inner {
        ptr: *mut SpeexEchoState,
        play_acc: Vec<i16>,
    }

    unsafe impl Send for Inner {}

    impl Inner {
        pub fn new() -> Option<Self> {
            let ptr = unsafe { speex_echo_state_init(FRAME_SAMPLES as c_int, FILTER_LEN) };
            if ptr.is_null() {
                return None;
            }
            let mut rate: c_int = SAMPLE_RATE as c_int;
            unsafe {
                speex_echo_ctl(
                    ptr,
                    SPEEX_ECHO_SET_SAMPLING_RATE,
                    &mut rate as *mut c_int as *mut std::ffi::c_void,
                );
            }
            Some(Self {
                ptr,
                play_acc: Vec::with_capacity(FRAME_SAMPLES * 2),
            })
        }

        pub fn feed_playback(&mut self, pcm: &[f32]) {
            for &s in pcm {
                self.play_acc.push(f32_to_i16(s));
                while self.play_acc.len() >= FRAME_SAMPLES {
                    let frame: Vec<i16> = self.play_acc.drain(..FRAME_SAMPLES).collect();
                    unsafe { speex_echo_playback(self.ptr, frame.as_ptr()) };
                }
            }
        }

        /// Cancel echo on one Opus-sized float frame (in-place).
        pub fn process_frame(&mut self, frame: &mut [f32]) {
            if frame.len() != FRAME_SAMPLES {
                return;
            }
            let mut rec = [0i16; FRAME_SAMPLES];
            let mut out = [0i16; FRAME_SAMPLES];
            for (i, s) in frame.iter().enumerate() {
                rec[i] = f32_to_i16(*s);
            }
            unsafe { speex_echo_capture(self.ptr, rec.as_ptr(), out.as_mut_ptr()) };
            for (i, s) in frame.iter_mut().enumerate() {
                *s = i16_to_f32(out[i]);
            }
        }


        pub fn reset(&mut self) {
            unsafe { speex_echo_state_reset(self.ptr) };
            self.play_acc.clear();
        }
    }

    impl Drop for Inner {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe { speex_echo_state_destroy(self.ptr) };
                self.ptr = std::ptr::null_mut();
            }
        }
    }
}

/// Acoustic echo canceller. Live Speex when built with speexdsp; else passthrough.
pub struct Aec {
    #[cfg(blightnet_has_aec)]
    inner: Option<speex::Inner>,
    #[cfg(not(blightnet_has_aec))]
    _stub: (),
}

impl Default for Aec {
    fn default() -> Self {
        Self::new()
    }
}

impl Aec {
    pub fn new() -> Self {
        warn_if_stub();
        #[cfg(blightnet_has_aec)]
        {
            Self {
                inner: speex::Inner::new(),
            }
        }
        #[cfg(not(blightnet_has_aec))]
        {
            Self { _stub: () }
        }
    }

    pub fn live(&self) -> bool {
        #[cfg(blightnet_has_aec)]
        {
            self.inner.is_some()
        }
        #[cfg(not(blightnet_has_aec))]
        {
            false
        }
    }

    /// Far-end reference = PCM about to play via the voice sink (16 kHz mono float).
    pub fn feed_playback(&mut self, pcm: &[f32]) {
        if pcm.is_empty() {
            return;
        }
        #[cfg(blightnet_has_aec)]
        if let Some(inner) = self.inner.as_mut() {
            inner.feed_playback(pcm);
        }
    }

    /// Run AEC on one 20 ms frame before Opus encode.
    pub fn process_frame(&mut self, frame: &mut [f32]) {
        #[cfg(blightnet_has_aec)]
        if let Some(inner) = self.inner.as_mut() {
            inner.process_frame(frame);
        }
    }

    pub fn reset(&mut self) {
        #[cfg(blightnet_has_aec)]
        if let Some(inner) = self.inner.as_mut() {
            inner.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aec_silence_identity_ish_no_panic() {
        let mut aec = Aec::new();
        let mut frame = vec![0f32; FRAME_SAMPLES];
        aec.process_frame(&mut frame);
        let e: f32 = frame.iter().map(|s| s * s).sum::<f32>();
        assert!(e < 1e-4, "silence should stay near zero, energy={e}");
        // Far-end tone + near silence: still must not panic.
        let play: Vec<f32> = (0..FRAME_SAMPLES)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.3
            })
            .collect();
        aec.feed_playback(&play);
        aec.feed_playback(&play);
        let mut near = play.clone();
        aec.process_frame(&mut near);
        assert_eq!(near.len(), FRAME_SAMPLES);
    }

    #[test]
    fn aec_new_reports_live_when_speex_linked() {
        let aec = Aec::new();
        // On Hellcat with speexdsp, live must be true; elsewhere stub is ok.
        if cfg!(blightnet_has_aec) {
            assert!(aec.live(), "speexdsp linked but Aec::new failed");
        } else {
            assert!(!aec.live());
        }
        assert_eq!(is_live(), cfg!(blightnet_has_aec));
    }
}
