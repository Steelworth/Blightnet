use crate::sys;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError, TrySendError};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Camera {
    pub path: String,
    pub name: String,
}

pub struct Capture {
    child: Child,
    rx: Receiver<Vec<u8>>,
}

impl Drop for Capture {
    fn drop(&mut self) {
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
                Err(TryRecvError::Disconnected) => break,
            }
        }
        last
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

fn ffmpeg_mjpeg(args: &[&str]) -> Option<Capture> {
    let bin = sys::ffmpeg_bin()?;
    let mut cmd = Command::new(bin);
    sys::hide(&mut cmd);
    cmd.args(["-hide_banner", "-loglevel", "error"])
        .args(args)
        .args([
            "-an",
            "-threads",
            "1",
            "-q:v",
            "8",
            "-f",
            "mjpeg",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take()?;
    Some(Capture {
        child,
        rx: spawn_jpeg_reader(stdout, 5),
    })
}

pub fn start_camera(path: &str) -> Option<Capture> {
    if path.is_empty() {
        return None;
    }
    #[cfg(windows)]
    {
        let src = if path.starts_with("video=") {
            path.to_string()
        } else {
            format!("video={path}")
        };
        return ffmpeg_mjpeg(&["-f", "dshow", "-i", &src, "-vf", "scale=240:-2", "-r", "5"]);
    }
    #[cfg(not(windows))]
    {
        ffmpeg_mjpeg(&["-f", "v4l2", "-i", path, "-vf", "scale=240:-2", "-r", "5"])
    }
}

pub fn start_screen() -> Option<Capture> {
    #[cfg(windows)]
    {
        return ffmpeg_mjpeg(&[
            "-f",
            "gdigrab",
            "-framerate",
            "3",
            "-i",
            "desktop",
            "-vf",
            "scale=480:-2",
            "-r",
            "3",
        ]);
    }
    #[cfg(not(windows))]
    {
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
        ffmpeg_mjpeg(&[
            "-f",
            "x11grab",
            "-framerate",
            "3",
            "-i",
            &display,
            "-vf",
            "scale=480:-2",
            "-r",
            "3",
        ])
    }
}

fn spawn_jpeg_reader(mut r: impl Read + Send + 'static, fps: u32) -> Receiver<Vec<u8>> {
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
                            Err(TrySendError::Disconnected(_)) => return,
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
        })
        .ok();
    rx
}

fn find_marker(buf: &[u8], second: u8) -> Option<usize> {
    buf.windows(2)
        .position(|w| w[0] == 0xFF && w[1] == second)
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
    fn list_cameras_does_not_panic() {
        let _ = list_cameras();
        let _ = default_camera();
    }
}
