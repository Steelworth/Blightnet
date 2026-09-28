//! Camera and screen capture via FFmpeg MJPEG (and GStreamer PipeWire on Wayland).
//!
//! Screen backends (preference order on Linux):
//! 1. **PipeWire + xdg-desktop-portal** (Wayland) — portal Screencast, then
//!    `ffmpeg -f pipewire` if built with it, else `gst-launch-1.0 pipewiresrc`.
//! 2. **x11grab** when `DISPLAY` is set (X11 / XWayland best-effort).
//! 3. **gdigrab** on Windows.
//!
//! Frames stay JPEG-over-Wire (`VideoFrame`). WAN screenshare is best-effort.
//! Capture death must never tear down voice — callers check [`Capture::alive`].

use crate::sys;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError, TrySendError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Modest caps: still JPEG-over-Wire; keep under ~160 KiB after encode when possible.
const CAM_SCALE: &str = "scale=320:-2";
const CAM_FPS: u32 = 8;
const SCREEN_SCALE: &str = "scale=720:-2";
const SCREEN_FPS: u32 = 8;
/// FFmpeg MJPEG qscale (2–31, lower = better). Was 8.
const MJPEG_Q: &str = "5";

#[derive(Clone)]
pub struct Camera {
    pub path: String,
    pub name: String,
}

/// Which capture tool produced this stream (for UI / README wording).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenBackend {
    /// `ffmpeg -f pipewire` after a portal Screencast grant.
    PipeWireFfmpeg,
    /// `gst-launch-1.0 pipewiresrc` after a portal Screencast grant.
    PipeWireGstreamer,
    /// `ffmpeg -f x11grab` (X11 / XWayland).
    X11Grab,
    /// `ffmpeg -f gdigrab` (Windows).
    GdiGrab,
}

impl ScreenBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::PipeWireFfmpeg => "PipeWire (ffmpeg + portal)",
            Self::PipeWireGstreamer => "PipeWire (GStreamer + portal)",
            Self::X11Grab => "X11 grab",
            Self::GdiGrab => "GDI grab",
        }
    }

    #[allow(dead_code)]
    pub fn is_pipewire(self) -> bool {
        matches!(self, Self::PipeWireFfmpeg | Self::PipeWireGstreamer)
    }
}

pub struct Capture {
    child: Child,
    rx: Receiver<Vec<u8>>,
    /// Cleared when the JPEG reader exits (ffmpeg/gst stdout closed).
    alive: Arc<AtomicBool>,
    /// Keeps the portal Screencast session open until capture ends (Linux PipeWire).
    _portal_hold: Option<PortalHold>,
    #[allow(dead_code)]
    pub backend: Option<ScreenBackend>,
}

/// Dropping this signals the portal hold thread to close the Screencast session.
struct PortalHold {
    stop: Option<mpsc::Sender<()>>,
}

impl Drop for PortalHold {
    fn drop(&mut self) {
        self.stop.take();
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Capture {
    pub fn latest(&self) -> Option<Vec<u8>> {
        let mut last = None;
        loop {
            match self.rx.try_recv() {
                Ok(f) => last = Some(f),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.alive.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
        last
    }

    /// False when the capture process exited or the frame reader disconnected.
    /// Callers must clear cam/screen flags and leave voice up.
    pub fn alive(&mut self) -> bool {
        if !self.alive.load(Ordering::SeqCst) {
            return false;
        }
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => {
                self.alive.store(false, Ordering::SeqCst);
                false
            }
            Err(_) => {
                self.alive.store(false, Ordering::SeqCst);
                false
            }
        }
    }
}

pub fn list_cameras() -> Vec<Camera> {
    #[cfg(windows)]
    {
        return list_dshow();
    }
    #[cfg(not(windows))]
    {
        list_v4l2()
    }
}

#[cfg(not(windows))]
fn list_v4l2() -> Vec<Camera> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/video4linux") else {
        return out;
    };
    for e in dir.flatten() {
        let dev = e.file_name().to_string_lossy().into_owned();
        if !dev.starts_with("video") {
            continue;
        }
        let path = format!("/dev/{dev}");
        let name = std::fs::read_to_string(e.path().join("name")).unwrap_or_else(|_| dev.clone());
        out.push(Camera {
            path,
            name: name.trim().to_string(),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

#[cfg(windows)]
fn list_dshow() -> Vec<Camera> {
    let mut out = Vec::new();
    let Some(bin) = sys::ffmpeg_bin() else {
        return out;
    };
    let mut cmd = Command::new(bin);
    sys::hide(&mut cmd);
    let Ok(ret) = cmd
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
    else {
        return out;
    };
    let text = String::from_utf8_lossy(&ret.stderr);
    let mut video = false;
    for line in text.lines() {
        let low = line.to_lowercase();
        if low.contains("directshow video") {
            video = true;
            continue;
        }
        if low.contains("directshow audio") {
            video = false;
            continue;
        }
        if !video {
            continue;
        }
        if let Some(name) = dshow_quoted(line) {
            if name.starts_with('@') || name.is_empty() {
                continue;
            }
            if out.iter().any(|c: &Camera| c.path == name) {
                continue;
            }
            out.push(Camera {
                path: name.clone(),
                name,
            });
        }
    }
    out
}

#[cfg(windows)]
fn dshow_quoted(line: &str) -> Option<String> {
    let a = line.find('"')?;
    let b = line.rfind('"')?;
    if b <= a {
        return None;
    }
    Some(line[a + 1..b].trim().to_string())
}

pub fn default_camera() -> Option<Camera> {
    list_cameras().into_iter().next()
}

fn ffmpeg_mjpeg(args: &[&str], fps: u32) -> Option<Capture> {
    let bin = sys::ffmpeg_bin()?;
    let mut cmd = Command::new(bin);
    sys::hide(&mut cmd);
    cmd.args(["-hide_banner", "-loglevel", "error"])
        .args(args)
        .args([
            "-an", "-threads", "1", "-q:v", MJPEG_Q, "-f", "mjpeg", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let alive = Arc::new(AtomicBool::new(true));
    Some(Capture {
        child,
        rx: spawn_jpeg_reader(stdout, fps, Arc::clone(&alive)),
        alive,
        _portal_hold: None,
        backend: None,
    })
}

pub fn start_camera(path: &str) -> Option<Capture> {
    if path.is_empty() {
        return None;
    }
    let fps = CAM_FPS.to_string();
    #[cfg(windows)]
    {
        let src = if path.starts_with("video=") {
            path.to_string()
        } else {
            format!("video={path}")
        };
        return ffmpeg_mjpeg(
            &["-f", "dshow", "-i", &src, "-vf", CAM_SCALE, "-r", &fps],
            CAM_FPS,
        );
    }
    #[cfg(not(windows))]
    {
        ffmpeg_mjpeg(
            &["-f", "v4l2", "-i", path, "-vf", CAM_SCALE, "-r", &fps],
            CAM_FPS,
        )
    }
}

/// Result of starting screen capture (backend label for chat/status).
pub struct ScreenStart {
    pub capture: Capture,
    #[allow(dead_code)]
    pub backend: ScreenBackend,
    /// Short note for the chat/status line (WAN best-effort, portal click, etc.).
    pub note: String,
}

/// Start screen share. Prefer PipeWire portal on Wayland; soft-fails with a clear error.
pub fn start_screen() -> Result<ScreenStart, String> {
    #[cfg(windows)]
    {
        let fps = SCREEN_FPS.to_string();
        let mut cap = ffmpeg_mjpeg(
            &[
                "-f",
                "gdigrab",
                "-framerate",
                &fps,
                "-i",
                "desktop",
                "-vf",
                SCREEN_SCALE,
                "-r",
                &fps,
            ],
            SCREEN_FPS,
        )
        .ok_or_else(|| {
            format!(
                "Screen share needs FFmpeg (gdigrab). {}",
                sys::ffmpeg_install_tip()
            )
        })?;
        cap.backend = Some(ScreenBackend::GdiGrab);
        return Ok(ScreenStart {
            capture: cap,
            backend: ScreenBackend::GdiGrab,
            note: format!(
                "Screen share on ({}) — LAN OK; WAN is best-effort JPEG.",
                ScreenBackend::GdiGrab.label()
            ),
        });
    }
    #[cfg(not(windows))]
    {
        start_screen_linux()
    }
}

#[cfg(not(windows))]
fn start_screen_linux() -> Result<ScreenStart, String> {
    let caps = probe_linux_screen_caps();
    let backend = select_screen_backend(&caps).ok_or_else(|| missing_screen_hint(&caps))?;

    match backend {
        ScreenBackend::PipeWireFfmpeg | ScreenBackend::PipeWireGstreamer => {
            start_screen_pipewire(backend, &caps)
        }
        ScreenBackend::X11Grab => start_screen_x11grab(),
        ScreenBackend::GdiGrab => unreachable!("gdigrab is Windows-only"),
    }
}

/// Runtime capability bits used by [`select_screen_backend`] (also unit-tested).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxScreenCaps {
    pub wayland: bool,
    pub display_set: bool,
    pub ffmpeg_ok: bool,
    pub ffmpeg_pipewire: bool,
    pub gst_pipewire: bool,
    pub portal: bool,
}

pub fn probe_linux_screen_caps() -> LinuxScreenCaps {
    let session = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_lowercase();
    let wayland = session == "wayland" || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let display_set = std::env::var_os("DISPLAY").is_some();
    let ffmpeg_ok = sys::ffmpeg_bin().is_some();
    LinuxScreenCaps {
        wayland,
        display_set,
        ffmpeg_ok,
        ffmpeg_pipewire: ffmpeg_ok && ffmpeg_has_pipewire(),
        gst_pipewire: gstreamer_pipewire_ok(),
        portal: portal_desktop_available(),
    }
}

/// Pure backend picker — Wayland prefers PipeWire portal; X11 uses x11grab.
pub fn select_screen_backend(caps: &LinuxScreenCaps) -> Option<ScreenBackend> {
    if caps.wayland {
        if caps.portal {
            if caps.ffmpeg_pipewire {
                return Some(ScreenBackend::PipeWireFfmpeg);
            }
            if caps.gst_pipewire {
                return Some(ScreenBackend::PipeWireGstreamer);
            }
        }
        // XWayland best-effort only when PipeWire path is unavailable.
        if caps.display_set && caps.ffmpeg_ok {
            return Some(ScreenBackend::X11Grab);
        }
        return None;
    }
    if caps.display_set && caps.ffmpeg_ok {
        return Some(ScreenBackend::X11Grab);
    }
    None
}

fn missing_screen_hint(caps: &LinuxScreenCaps) -> String {
    if caps.wayland {
        let mut parts: Vec<String> =
            vec!["Wayland screen share needs xdg-desktop-portal + PipeWire capture.".to_string()];
        if !caps.portal {
            parts.push(
                "Portal service not found (install xdg-desktop-portal and a backend: kde/gnome/wlr)."
                    .to_string(),
            );
        }
        if !caps.ffmpeg_pipewire && !caps.gst_pipewire {
            parts.push(
                "Install gstreamer pipewiresrc (gst-plugin-pipewire) or an FFmpeg build with -f pipewire."
                    .to_string(),
            );
        }
        if caps.display_set && caps.ffmpeg_ok {
            parts.push("X11 grab may work only for XWayland windows.".to_string());
        }
        parts.join(" ")
    } else if !caps.ffmpeg_ok {
        format!("Screen share needs FFmpeg. {}", sys::ffmpeg_install_tip())
    } else if !caps.display_set {
        "Screen share needs DISPLAY (X11) or a Wayland portal path.".to_string()
    } else {
        "Could not share the screen.".to_string()
    }
}

fn ffmpeg_has_pipewire() -> bool {
    let Some(bin) = sys::ffmpeg_bin() else {
        return false;
    };
    let mut cmd = Command::new(bin);
    sys::hide(&mut cmd);
    let Ok(out) = cmd.args(["-hide_banner", "-devices"]).output() else {
        return false;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    text.to_lowercase()
        .lines()
        .any(|l| l.contains("pipewire") && !l.contains("unknown"))
}

fn gstreamer_pipewire_ok() -> bool {
    let Some(inspect) = which_bin("gst-inspect-1.0") else {
        return which_bin("gst-launch-1.0").is_some();
    };
    let mut icmd = Command::new(inspect);
    sys::hide(&mut icmd);
    matches!(icmd.args(["pipewiresrc"]).output(), Ok(out) if out.status.success())
}

fn portal_desktop_available() -> bool {
    std::path::Path::new("/usr/lib/xdg-desktop-portal").exists()
        || std::path::Path::new("/usr/libexec/xdg-desktop-portal").exists()
        || which_bin("xdg-desktop-portal").is_some()
}

fn which_bin(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}

#[cfg(not(windows))]
fn start_screen_x11grab() -> Result<ScreenStart, String> {
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
    let fps = SCREEN_FPS.to_string();
    let mut cap = ffmpeg_mjpeg(
        &[
            "-f",
            "x11grab",
            "-framerate",
            &fps,
            "-i",
            &display,
            "-vf",
            SCREEN_SCALE,
            "-r",
            &fps,
        ],
        SCREEN_FPS,
    )
    .ok_or_else(|| {
        "Could not start x11grab. On Wayland this is often blank — use the portal/PipeWire path."
            .to_string()
    })?;
    cap.backend = Some(ScreenBackend::X11Grab);
    let wayland = std::env::var("XDG_SESSION_TYPE")
        .map(|s| s.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false);
    let note = if wayland {
        format!(
            "Screen share on ({}) — Wayland fallback; may miss native windows. WAN best-effort.",
            ScreenBackend::X11Grab.label()
        )
    } else {
        format!(
            "Screen share on ({}) — LAN OK; WAN is best-effort JPEG.",
            ScreenBackend::X11Grab.label()
        )
    };
    Ok(ScreenStart {
        capture: cap,
        backend: ScreenBackend::X11Grab,
        note,
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn start_screen_pipewire(
    prefer: ScreenBackend,
    caps: &LinuxScreenCaps,
) -> Result<ScreenStart, String> {
    let grant = portal_screencast_grant().map_err(|e| {
        format!(
            "Screen portal failed ({e}). Click Share/Allow in the system dialog when it appears."
        )
    })?;

    let try_order: &[ScreenBackend] = if prefer == ScreenBackend::PipeWireFfmpeg {
        &[
            ScreenBackend::PipeWireFfmpeg,
            ScreenBackend::PipeWireGstreamer,
        ]
    } else {
        &[
            ScreenBackend::PipeWireGstreamer,
            ScreenBackend::PipeWireFfmpeg,
        ]
    };

    let mut last_err = String::new();
    let PortalGrant { node_id, fd, hold } = grant;

    for backend in try_order {
        if *backend == ScreenBackend::PipeWireFfmpeg && !caps.ffmpeg_pipewire {
            continue;
        }
        if *backend == ScreenBackend::PipeWireGstreamer && !caps.gst_pipewire {
            continue;
        }
        match spawn_pipewire_capture(*backend, node_id, &fd) {
            Ok(mut cap) => {
                cap.backend = Some(*backend);
                cap._portal_hold = Some(hold);
                let note = format!(
                    "Screen share on ({}) — approve the portal if asked. One GM share; LAN-good / WAN best-effort JPEG.",
                    backend.label()
                );
                return Ok(ScreenStart {
                    capture: cap,
                    backend: *backend,
                    note,
                });
            }
            Err(e) => last_err = e,
        }
    }
    drop(hold);
    drop(fd);
    Err(if last_err.is_empty() {
        missing_screen_hint(caps)
    } else {
        format!("{last_err} Voice stays up if video fails.")
    })
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
#[cfg(not(windows))]
fn start_screen_pipewire(
    _prefer: ScreenBackend,
    caps: &LinuxScreenCaps,
) -> Result<ScreenStart, String> {
    Err(missing_screen_hint(caps))
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
struct PortalGrant {
    node_id: u32,
    fd: std::os::fd::OwnedFd,
    hold: PortalHold,
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn portal_screencast_grant() -> Result<PortalGrant, String> {
    use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
    use ashpd::desktop::PersistMode;
    use std::os::fd::{FromRawFd, IntoRawFd};

    let (ready_tx, ready_rx) = mpsc::channel::<Result<(u32, i32), String>>();
    let (hold_tx, hold_rx) = mpsc::channel::<()>();

    thread::Builder::new()
        .name("blightnet-portal".into())
        .spawn(move || {
            // Keep Screencast + Session alive inside this block until Capture drops PortalHold.
            let err = pollster::block_on(async {
                let proxy = Screencast::new()
                    .await
                    .map_err(|e| format!("portal connect: {e}"))?;
                let session = proxy
                    .create_session()
                    .await
                    .map_err(|e| format!("CreateSession: {e}"))?;
                // One SelectSources per session. Prefer Embedded cursor when the portal allows it.
                let cursor = match proxy.available_cursor_modes().await {
                    Ok(m) if m.contains(CursorMode::Embedded) => CursorMode::Embedded,
                    Ok(m) if m.contains(CursorMode::Metadata) => CursorMode::Metadata,
                    _ => CursorMode::Hidden,
                };
                let sources = SourceType::Monitor | SourceType::Window;
                proxy
                    .select_sources(&session, cursor, sources, false, None, PersistMode::DoNot)
                    .await
                    .map_err(|e| format!("SelectSources: {e}"))?;
                let response = proxy
                    .start(&session, None)
                    .await
                    .map_err(|e| format!("Start: {e}"))?
                    .response()
                    .map_err(|e| format!("Start response: {e}"))?;
                let stream = response
                    .streams()
                    .first()
                    .ok_or_else(|| "No screen selected in the portal dialog.".to_string())?;
                let node_id = stream.pipe_wire_node_id();
                let owned = proxy
                    .open_pipe_wire_remote(&session)
                    .await
                    .map_err(|e| format!("OpenPipeWireRemote: {e}"))?;
                let raw_fd = owned.into_raw_fd();
                let dup = unsafe { libc::dup(raw_fd) };
                unsafe {
                    libc::close(raw_fd);
                }
                if dup < 0 {
                    return Err("dup(pipewire fd) failed".into());
                }
                let _ = ready_tx.send(Ok((node_id, dup)));
                // Block until PortalHold drops (sync recv is fine: this thread is dedicated).
                let _ = hold_rx.recv();
                drop(session);
                drop(proxy);
                Ok::<(), String>(())
            });
            if let Err(e) = err {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| format!("portal thread: {e}"))?;

    let (node_id, dup) = ready_rx
        .recv()
        .map_err(|_| "Portal thread ended before grant.".to_string())??;

    unsafe {
        let flags = libc::fcntl(dup, libc::F_GETFD);
        if flags >= 0 {
            libc::fcntl(dup, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        }
    }
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(dup) };

    Ok(PortalGrant {
        node_id,
        fd,
        hold: PortalHold {
            stop: Some(hold_tx),
        },
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn spawn_pipewire_capture(
    backend: ScreenBackend,
    node_id: u32,
    fd: &std::os::fd::OwnedFd,
) -> Result<Capture, String> {
    use std::os::fd::AsRawFd;
    let raw = fd.as_raw_fd();
    let child_fd = unsafe { libc::dup(raw) };
    if child_fd < 0 {
        return Err("dup for capture failed".into());
    }
    unsafe {
        let flags = libc::fcntl(child_fd, libc::F_GETFD);
        if flags >= 0 {
            libc::fcntl(child_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        }
    }

    let spawned = match backend {
        ScreenBackend::PipeWireFfmpeg => spawn_ffmpeg_pipewire(node_id, child_fd),
        ScreenBackend::PipeWireGstreamer => spawn_gst_pipewire(node_id, child_fd),
        _ => {
            unsafe {
                libc::close(child_fd);
            }
            return Err("not a PipeWire backend".into());
        }
    };

    // Child inherited child_fd; close our extra copy.
    unsafe {
        libc::close(child_fd);
    }

    let mut child = spawned?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "capture process has no stdout".to_string())?;
    let alive = Arc::new(AtomicBool::new(true));
    Ok(Capture {
        child,
        rx: spawn_jpeg_reader(stdout, SCREEN_FPS, Arc::clone(&alive)),
        alive,
        _portal_hold: None,
        backend: Some(backend),
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn spawn_ffmpeg_pipewire(node_id: u32, pw_fd: i32) -> Result<Child, String> {
    let bin = sys::ffmpeg_bin().ok_or_else(|| "FFmpeg missing".to_string())?;
    let fps = SCREEN_FPS.to_string();
    let node = node_id.to_string();
    let mut cmd = Command::new(bin);
    sys::hide(&mut cmd);
    // fd option is supported by ffmpeg pipewire demuxer when built with libpipewire.
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "pipewire",
        "-framerate",
        &fps,
    ])
    .arg("-fd")
    .arg(pw_fd.to_string())
    .args(["-i", &node, "-vf", SCREEN_SCALE, "-r", &fps])
    .args([
        "-an", "-threads", "1", "-q:v", MJPEG_Q, "-f", "mjpeg", "pipe:1",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .stdin(Stdio::null());
    cmd.spawn()
        .map_err(|e| format!("ffmpeg pipewire spawn: {e}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GstJpegEncoder {
    JpegEnc,
    AvencMjpeg,
}

impl GstJpegEncoder {
    fn element(self) -> &'static str {
        match self {
            Self::JpegEnc => "jpegenc",
            Self::AvencMjpeg => "avenc_mjpeg",
        }
    }
}

/// Prefer gst-plugins-good's encoder, but support the gst-libav alternative.
fn select_gst_jpeg_encoder(
    jpegenc_available: bool,
    avenc_mjpeg_available: bool,
) -> Option<GstJpegEncoder> {
    if jpegenc_available {
        Some(GstJpegEncoder::JpegEnc)
    } else if avenc_mjpeg_available {
        Some(GstJpegEncoder::AvencMjpeg)
    } else {
        None
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn gst_element_available(inspect: &str, element: &str) -> bool {
    let mut cmd = Command::new(inspect);
    sys::hide(&mut cmd);
    matches!(cmd.arg(element).output(), Ok(out) if out.status.success())
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn gst_jpeg_encoder() -> Result<GstJpegEncoder, String> {
    // Keep the existing gst-launch-only fallback: without gst-inspect there is
    // no reliable way to query plugin availability before starting the pipeline.
    let Some(inspect) = which_bin("gst-inspect-1.0") else {
        return Ok(GstJpegEncoder::JpegEnc);
    };
    select_gst_jpeg_encoder(
        gst_element_available(&inspect, "jpegenc"),
        gst_element_available(&inspect, "avenc_mjpeg"),
    )
    .ok_or_else(|| {
        "GStreamer PipeWire capture needs jpegenc (gst-plugins-good) or avenc_mjpeg (gst-libav); install either encoder package.".to_string()
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn spawn_gst_pipewire(node_id: u32, pw_fd: i32) -> Result<Child, String> {
    let gst = which_bin("gst-launch-1.0").ok_or_else(|| "gst-launch-1.0 missing".to_string())?;
    let encoder = gst_jpeg_encoder()?;
    let mut cmd = Command::new(gst);
    sys::hide(&mut cmd);
    // pipewiresrc fd= + path=node connects to the portal PipeWire remote.
    let fd_prop = format!("fd={pw_fd}");
    let path_prop = format!("path={node_id}");
    let caps = format!("video/x-raw,framerate={SCREEN_FPS}/1");
    cmd.args([
        "-q",
        "pipewiresrc",
        &fd_prop,
        &path_prop,
        "do-timestamp=true",
        "!",
        "videoconvert",
        "!",
        "videoscale",
        "!",
        "video/x-raw,width=720",
        "!",
        "videorate",
        "!",
        &caps,
        "!",
    ])
    .arg(encoder.element());
    if encoder == GstJpegEncoder::JpegEnc {
        cmd.arg("quality=65");
    }
    cmd.args(["!", "fdsink", "fd=1", "sync=false"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    cmd.spawn()
        .map_err(|e| format!("gstreamer pipewire spawn: {e}"))
}

fn spawn_jpeg_reader(
    mut r: impl Read + Send + 'static,
    fps: u32,
    alive: Arc<AtomicBool>,
) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::sync_channel(1);
    let min = Duration::from_millis((1000 / fps.max(1)) as u64);
    thread::Builder::new()
        .name("blightnet-vid".into())
        .spawn(move || {
            let mut buf = Vec::with_capacity(64 * 1024);
            let mut tmp = [0u8; 8192];
            let mut last = Instant::now() - min;
            loop {
                let n = match r.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                buf.extend_from_slice(&tmp[..n]);
                let mut start = 0;
                while start + 3 < buf.len() {
                    let Some(soi) = find_marker(&buf[start..], 0xD8) else {
                        buf.clear();
                        break;
                    };
                    start += soi;
                    let Some(rel) = find_marker(&buf[start + 2..], 0xD9) else {
                        if start > 0 {
                            buf.drain(..start);
                        }
                        break;
                    };
                    let end = start + 2 + rel + 2;
                    if end > buf.len() {
                        if start > 0 {
                            buf.drain(..start);
                        }
                        break;
                    }
                    if last.elapsed() >= min {
                        let frame = buf[start..end].to_vec();
                        match tx.try_send(frame) {
                            Ok(()) => last = Instant::now(),
                            Err(TrySendError::Full(_)) => last = Instant::now(),
                            Err(TrySendError::Disconnected(_)) => {
                                alive.store(false, Ordering::SeqCst);
                                return;
                            }
                        }
                    }
                    start = end;
                }
                if start > 0 && start <= buf.len() {
                    buf.drain(..start);
                }
                if buf.len() > 400_000 {
                    buf.clear();
                }
            }
            alive.store(false, Ordering::SeqCst);
        })
        .ok();
    rx
}

fn find_marker(buf: &[u8], second: u8) -> Option<usize> {
    buf.windows(2).position(|w| w[0] == 0xFF && w[1] == second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_markers() {
        let buf = [0, 1, 0xFF, 0xD8, 2, 3, 0xFF, 0xD9];
        assert_eq!(find_marker(&buf, 0xD8), Some(2));
        assert_eq!(find_marker(&buf, 0xD9), Some(6));
        assert!(find_marker(&[1, 2, 3], 0xD8).is_none());
    }

    #[test]
    fn select_gst_jpeg_encoder_prefers_good_then_libav() {
        assert_eq!(
            select_gst_jpeg_encoder(true, true),
            Some(GstJpegEncoder::JpegEnc)
        );
        assert_eq!(
            select_gst_jpeg_encoder(false, true),
            Some(GstJpegEncoder::AvencMjpeg)
        );
        assert_eq!(select_gst_jpeg_encoder(false, false), None);
    }

    #[test]
    fn list_cameras_does_not_panic() {
        let _ = list_cameras();
        let _ = default_camera();
    }

    #[test]
    fn select_backend_wayland_prefers_pipewire_gst() {
        let caps = LinuxScreenCaps {
            wayland: true,
            display_set: true,
            ffmpeg_ok: true,
            ffmpeg_pipewire: false,
            gst_pipewire: true,
            portal: true,
        };
        assert_eq!(
            select_screen_backend(&caps),
            Some(ScreenBackend::PipeWireGstreamer)
        );
    }

    #[test]
    fn select_backend_wayland_prefers_ffmpeg_pipewire() {
        let caps = LinuxScreenCaps {
            wayland: true,
            display_set: true,
            ffmpeg_ok: true,
            ffmpeg_pipewire: true,
            gst_pipewire: true,
            portal: true,
        };
        assert_eq!(
            select_screen_backend(&caps),
            Some(ScreenBackend::PipeWireFfmpeg)
        );
    }

    #[test]
    fn select_backend_wayland_falls_back_x11_without_portal() {
        let caps = LinuxScreenCaps {
            wayland: true,
            display_set: true,
            ffmpeg_ok: true,
            ffmpeg_pipewire: true,
            gst_pipewire: true,
            portal: false,
        };
        assert_eq!(select_screen_backend(&caps), Some(ScreenBackend::X11Grab));
    }

    #[test]
    fn select_backend_x11_session_uses_x11grab() {
        let caps = LinuxScreenCaps {
            wayland: false,
            display_set: true,
            ffmpeg_ok: true,
            ffmpeg_pipewire: false,
            gst_pipewire: false,
            portal: false,
        };
        assert_eq!(select_screen_backend(&caps), Some(ScreenBackend::X11Grab));
    }

    #[test]
    fn select_backend_wayland_none_without_tools() {
        let caps = LinuxScreenCaps {
            wayland: true,
            display_set: false,
            ffmpeg_ok: false,
            ffmpeg_pipewire: false,
            gst_pipewire: false,
            portal: true,
        };
        assert_eq!(select_screen_backend(&caps), None);
    }

    #[test]
    fn screen_backend_labels_are_nonempty() {
        for b in [
            ScreenBackend::PipeWireFfmpeg,
            ScreenBackend::PipeWireGstreamer,
            ScreenBackend::X11Grab,
            ScreenBackend::GdiGrab,
        ] {
            assert!(!b.label().is_empty());
        }
        assert!(ScreenBackend::PipeWireGstreamer.is_pipewire());
        assert!(!ScreenBackend::X11Grab.is_pipewire());
    }
}
