//! Text backup, media tags, and gif frames. Nothing here is sent on its own.

use std::path::{Path, PathBuf};

pub const TEXT_CAP: u64 = 1024 * 1024;
pub const GIF_CAP: usize = 48 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TagSet {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub comment: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct GifFrame {
    pub ms: u32,
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}

pub struct GifSet {
    pub frames: Vec<GifFrame>,
    pub note: String,
}

pub fn is_text_name(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "txt" | "md" | "log" | "csv" | "json" | "toml"
    )
}

pub fn bak_path(root: &Path, src: &Path) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    src.hash(&mut hasher);
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    root.join("data/file-bak")
        .join(format!("{:x}.{ext}", hasher.finish()))
}

pub fn backup_file(root: &Path, src: &Path) -> Result<(), String> {
    if !src.is_file() {
        return Ok(());
    }
    let dest = bak_path(root, src);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "The backup folder was not created.".to_string())?;
    }
    std::fs::copy(src, &dest).map(|_| ()).map_err(|_| "The backup was not written.".to_string())
}

pub fn restore_file(root: &Path, src: &Path) -> Result<(), String> {
    let dest = bak_path(root, src);
    if !dest.is_file() {
        return Err("There is no backup of this file yet.".into());
    }
    std::fs::copy(&dest, src).map(|_| ()).map_err(|_| "The file was not restored.".to_string())
}

pub fn read_text(path: &Path) -> Result<String, String> {
    let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if len > TEXT_CAP {
        return Err("This text file is too large to edit here.".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "The text file did not open.".to_string())?;
    String::from_utf8(bytes).map_err(|_| "This file is not plain text.".to_string())
}

pub fn write_text(root: &Path, path: &Path, body: &str) -> Result<(), String> {
    backup_file(root, path)?;
    std::fs::write(path, body.as_bytes()).map_err(|_| "The text was not saved.".to_string())
}

pub fn changed_stamp(path: &Path) -> String {
    let Ok(meta) = std::fs::metadata(path) else {
        return "unknown".into();
    };
    let Ok(modified) = meta.modified() else {
        return "unknown".into();
    };
    let Ok(ago) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return "unknown".into();
    };
    let (y, m, d, hh, mm) = ymd_hm(ago.as_secs());
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

fn ymd_hm(secs: u64) -> (i32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let clock = (secs % 86400) as u32;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, clock / 3600, (clock % 3600) / 60)
}

fn civil_from_days(mut z: i64) -> (i32, u32, u32) {
    z += 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn parse_ffprobe(text: &str) -> TagSet {
    let mut tags = TagSet::default();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return tags;
    };
    if let Some(map) = v.get("format").and_then(|f| f.get("tags")).and_then(|t| t.as_object()) {
        tags.title = tag_get(map, &["title", "TITLE"]);
        tags.artist = tag_get(map, &["artist", "ARTIST", "author"]);
        tags.album = tag_get(map, &["album", "ALBUM"]);
        tags.year = tag_get(map, &["date", "DATE", "year", "YEAR"]);
        tags.comment = tag_get(map, &["comment", "COMMENT", "description"]);
    }
    if let Some(streams) = v.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let w = stream.get("width").and_then(|n| n.as_u64()).unwrap_or(0) as u32;
            let h = stream.get("height").and_then(|n| n.as_u64()).unwrap_or(0) as u32;
            if w > 0 && h > 0 {
                tags.width = w;
                tags.height = h;
                break;
            }
        }
    }
    tags
}

fn tag_get(map: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = map.get(*key).and_then(|v| v.as_str()) {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    String::new()
}

pub fn tag_args(src: &Path, dest: &Path, tags: &TagSet, kind: &str, clear: bool) -> Vec<String> {
    let mut args = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-i".into(),
        src.display().to_string(),
        "-map_metadata".into(),
        "-1".into(),
    ];
    if kind != "image" {
        args.push("-c".into());
        args.push("copy".into());
    }
    if !clear {
        for (key, val) in [
            ("title", &tags.title),
            ("artist", &tags.artist),
            ("album", &tags.album),
            ("date", &tags.year),
            ("comment", &tags.comment),
        ] {
            let flat: String = val.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
            args.push("-metadata".into());
            args.push(format!("{key}={flat}"));
        }
    }
    args.push(dest.display().to_string());
    args
}

pub fn apply_tags(root: &Path, src: &Path, tags: &TagSet, kind: &str, clear: bool) -> Result<(), String> {
    let Some(bin) = crate::sys::ffmpeg_bin() else {
        return Err("ffmpeg is not on this computer, so tags cannot be changed here.".into());
    };
    backup_file(root, src)?;
    let temp = temp_beside(src);
    let args = tag_args(src, &temp, tags, kind, clear);
    let out = std::process::Command::new(bin)
        .args(&args)
        .output()
        .map_err(|_| "ffmpeg did not start.".to_string())?;
    if !out.status.success() || !temp.is_file() {
        let _ = std::fs::remove_file(&temp);
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        if err.is_empty() {
            return Err("The tags were not written.".into());
        }
        let tail: String = err.chars().rev().take(160).collect::<String>().chars().rev().collect();
        return Err(format!("The tags were not written. {tail}"));
    }
    if std::fs::rename(&temp, src).is_err() {
        let _ = std::fs::remove_file(src);
        std::fs::rename(&temp, src).map_err(|_| "The new file did not replace the old one.".to_string())?;
    }
    Ok(())
}

fn temp_beside(src: &Path) -> PathBuf {
    let name = src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!(".{name}.blight-tmp"))
}

pub fn read_tags(path: &Path) -> Result<TagSet, String> {
    let mut tags = TagSet::default();
    if let Ok((w, h)) = image::image_dimensions(path) {
        tags.width = w;
        tags.height = h;
    }
    let Some(bin) = crate::sys::which("ffprobe").or_else(crate::sys::ffmpeg_bin) else {
        return Ok(tags);
    };
    let probe = crate::sys::which("ffprobe");
    let out = if let Some(ffprobe) = probe.as_ref() {
        std::process::Command::new(ffprobe)
            .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"])
            .arg(path)
            .output()
            .map_err(|_| "The tags did not open.".to_string())?
    } else {
        std::process::Command::new(bin)
            .args(["-hide_banner", "-i"])
            .arg(path)
            .output()
            .map_err(|_| "The tags did not open.".to_string())?
    };
    if probe.is_some() {
        let text = String::from_utf8_lossy(&out.stdout);
        let parsed = parse_ffprobe(&text);
        if tags.width == 0 {
            tags.width = parsed.width;
        }
        if tags.height == 0 {
            tags.height = parsed.height;
        }
        tags.title = parsed.title;
        tags.artist = parsed.artist;
        tags.album = parsed.album;
        tags.year = parsed.year;
        tags.comment = parsed.comment;
    }
    Ok(tags)
}

pub fn decode_gif(path: &Path) -> Result<GifSet, String> {
    let bytes = std::fs::read(path).map_err(|_| "The gif did not open.".to_string())?;
    decode_gif_bytes(&bytes)
}

pub fn decode_gif_bytes(data: &[u8]) -> Result<GifSet, String> {
    if data.len() < 13 || !(data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        return Err("The gif did not open.".into());
    }
    let mut p = 6usize;
    let sw = u16_at(data, &mut p)? as usize;
    let sh = u16_at(data, &mut p)? as usize;
    if sw == 0 || sh == 0 || sw.saturating_mul(sh).saturating_mul(3) > GIF_CAP {
        return Err("The gif did not open.".into());
    }
    let packed = byte_at(data, &mut p)?;
    let bg = byte_at(data, &mut p)? as usize;
    let _aspect = byte_at(data, &mut p)?;
    let gct = if packed & 0x80 != 0 {
        read_palette(data, &mut p, 1 << ((packed & 7) + 1))?
    } else {
        Vec::new()
    };
    let mut canvas = vec![0u8; sw * sh * 3];
    if !gct.is_empty() {
        let bg = gct.get(bg).copied().unwrap_or([0, 0, 0]);
        for px in canvas.chunks_exact_mut(3) {
            px.copy_from_slice(&bg);
        }
    }
    let mut frames = Vec::new();
    let mut used = 0usize;
    let mut note = String::new();
    let mut delay = 100u32;
    let mut trans: Option<u8> = None;
    let mut disposal = 0u8;
    while p < data.len() {
        match data[p] {
            0x3B => break,
            0x21 => {
                p += 1;
                let label = byte_at(data, &mut p)?;
                if label == 0xF9 {
                    let block = read_subblocks(data, &mut p)?;
                    if block.len() >= 4 {
                        let flags = block[0];
                        delay = u16::from_le_bytes([block[1], block[2]]) as u32 * 10;
                        if delay == 0 {
                            delay = 100;
                        }
                        trans = if flags & 1 != 0 { Some(block[3]) } else { None };
                        disposal = (flags >> 2) & 7;
                    }
                } else {
                    let _ = read_subblocks(data, &mut p)?;
                }
            }
            0x2C => {
                p += 1;
                let left = u16_at(data, &mut p)? as usize;
                let top = u16_at(data, &mut p)? as usize;
                let iw = u16_at(data, &mut p)? as usize;
                let ih = u16_at(data, &mut p)? as usize;
                let ipacked = byte_at(data, &mut p)?;
                let palette = if ipacked & 0x80 != 0 {
                    read_palette(data, &mut p, 1 << ((ipacked & 7) + 1))?
                } else {
                    gct.clone()
                };
                let min_code = byte_at(data, &mut p)?;
                let lzw = read_subblocks(data, &mut p)?;
                let index = lzw_decode(&lzw, min_code);
                let snapshot = if disposal == 3 { Some(canvas.clone()) } else { None };
                for y in 0..ih {
                    for x in 0..iw {
                        let n = y * iw + x;
                        let Some(color_i) = index.get(n).copied() else {
                            continue;
                        };
                        if trans == Some(color_i) {
                            continue;
                        }
                        let dx = left + x;
                        let dy = top + y;
                        if dx >= sw || dy >= sh {
                            continue;
                        }
                        if let Some(rgb) = palette.get(color_i as usize) {
                            let o = (dy * sw + dx) * 3;
                            canvas[o..o + 3].copy_from_slice(rgb);
                        }
                    }
                }
                let rgb = canvas.clone();
                if used + rgb.len() > GIF_CAP {
                    note = "This gif is long. The rest of the frames were left out.".into();
                    break;
                }
                used += rgb.len();
                frames.push(GifFrame {
                    ms: delay.max(10),
                    w: sw as u32,
                    h: sh as u32,
                    rgb,
                });
                if disposal == 2 {
                    let bg = gct.get(bg).copied().unwrap_or([0, 0, 0]);
                    for y in 0..ih {
                        for x in 0..iw {
                            let dx = left + x;
                            let dy = top + y;
                            if dx >= sw || dy >= sh {
                                continue;
                            }
                            let o = (dy * sw + dx) * 3;
                            canvas[o..o + 3].copy_from_slice(&bg);
                        }
                    }
                } else if disposal == 3 {
                    if let Some(prev) = snapshot {
                        canvas = prev;
                    }
                }
                delay = 100;
                trans = None;
                disposal = 0;
            }
            _ => {
                p += 1;
            }
        }
    }
    if frames.is_empty() {
        return Err("The gif did not open.".into());
    }
    Ok(GifSet { frames, note })
}

fn byte_at(data: &[u8], i: &mut usize) -> Result<u8, String> {
    let b = *data.get(*i).ok_or("The gif did not open.")?;
    *i += 1;
    Ok(b)
}

fn u16_at(data: &[u8], i: &mut usize) -> Result<u16, String> {
    let lo = byte_at(data, i)?;
    let hi = byte_at(data, i)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_palette(data: &[u8], i: &mut usize, n: usize) -> Result<Vec<[u8; 3]>, String> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let r = byte_at(data, i)?;
        let g = byte_at(data, i)?;
        let b = byte_at(data, i)?;
        out.push([r, g, b]);
    }
    Ok(out)
}

fn read_subblocks(data: &[u8], i: &mut usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let n = byte_at(data, i)? as usize;
        if n == 0 {
            break;
        }
        if *i + n > data.len() {
            return Err("The gif did not open.".into());
        }
        out.extend_from_slice(&data[*i..*i + n]);
        *i += n;
    }
    Ok(out)
}

fn lzw_decode(data: &[u8], min_code: u8) -> Vec<u8> {
    if !(2..=8).contains(&min_code) {
        return Vec::new();
    }
    let clear = 1usize << min_code;
    let eoi = clear + 1;
    let mut code_size = min_code as usize + 1;
    let mut next_code = eoi + 1;
    let mut dict: Vec<Vec<u8>> = (0..clear).map(|i| vec![i as u8]).collect();
    dict.push(Vec::new());
    dict.push(Vec::new());
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut pos = 0usize;
    let mut prev: Option<usize> = None;
    let mut out = Vec::new();
    let pull = |acc: &mut u32, bits: &mut u32, pos: &mut usize, width: usize| -> Option<usize> {
        while *bits < width as u32 {
            if *pos >= data.len() {
                return None;
            }
            *acc |= (data[*pos] as u32) << *bits;
            *bits += 8;
            *pos += 1;
        }
        let mask = (1u32 << width) - 1;
        let code = (*acc & mask) as usize;
        *acc >>= width;
        *bits -= width as u32;
        Some(code)
    };
    while let Some(code) = pull(&mut acc, &mut bits, &mut pos, code_size) {
        if code == clear {
            code_size = min_code as usize + 1;
            next_code = eoi + 1;
            dict.truncate(eoi + 1);
            prev = None;
            continue;
        }
        if code == eoi {
            break;
        }
        let entry = if code < dict.len() && !dict[code].is_empty() {
            dict[code].clone()
        } else if code == next_code {
            let Some(p) = prev else { break };
            let mut e = dict[p].clone();
            if e.is_empty() {
                break;
            }
            e.push(e[0]);
            e
        } else {
            break;
        };
        out.extend_from_slice(&entry);
        if let Some(p) = prev {
            if next_code < 4096 && !dict[p].is_empty() && !entry.is_empty() {
                let mut built = dict[p].clone();
                built.push(entry[0]);
                if next_code == dict.len() {
                    dict.push(built);
                } else if next_code < dict.len() {
                    dict[next_code] = built;
                }
                next_code += 1;
                if next_code == (1 << code_size) && code_size < 12 {
                    code_size += 1;
                }
            }
        }
        prev = if code < dict.len() { Some(code) } else { Some(dict.len().saturating_sub(1)) };
    }
    out
}

pub fn safe_download_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let s: String = base
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => c,
            _ => '_',
        })
        .collect();
    if s.is_empty() || s.starts_with('.') {
        format!("file{s}")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_the_first_day() {
        assert_eq!(ymd_hm(0), (1970, 1, 1, 0, 0));
        assert_eq!(ymd_hm(86400 + 3660), (1970, 1, 2, 1, 1));
    }

    #[test]
    fn probe_json_reads_tags_and_size() {
        let raw = r#"{"format":{"tags":{"title":"Night","artist":"Ada","album":"Street","date":"2020","comment":"wet"}},"streams":[{"codec_type":"audio"},{"width":640,"height":360}]}"#;
        let tags = parse_ffprobe(raw);
        assert_eq!(tags.title, "Night");
        assert_eq!(tags.artist, "Ada");
        assert_eq!(tags.album, "Street");
        assert_eq!(tags.year, "2020");
        assert_eq!(tags.comment, "wet");
        assert_eq!((tags.width, tags.height), (640, 360));
    }

    #[test]
    fn tag_args_copy_audio_and_rewrite_a_picture() {
        let tags = TagSet {
            title: "Night".into(),
            ..TagSet::default()
        };
        let audio = tag_args(Path::new("a.mp3"), Path::new("b.mp3"), &tags, "audio", false);
        let line = audio.join(" ");
        assert!(line.contains("-c copy"));
        assert!(line.contains("-map_metadata -1"));
        assert!(line.contains("-metadata title=Night"));
        let clear = tag_args(Path::new("a.mp3"), Path::new("b.mp3"), &tags, "audio", true);
        assert!(!clear.iter().any(|s| s.starts_with("-metadata")));
        let picture = tag_args(Path::new("a.jpg"), Path::new("b.jpg"), &tags, "image", false);
        assert!(!picture.windows(2).any(|w| w[0] == "-c" && w[1] == "copy"));
    }

    #[test]
    fn text_backup_roundtrip_and_a_gif_frame() {
        let dir = std::env::temp_dir().join(format!("bn-desk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let note = dir.join("note.txt");
        std::fs::write(&note, "one").unwrap();
        write_text(&dir, &note, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&note).unwrap(), "two");
        restore_file(&dir, &note).unwrap();
        assert_eq!(std::fs::read_to_string(&note).unwrap(), "one");
        assert!(is_text_name(&note));

        let dot = [
            0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0xFF, 0xFF,
            0xFF, 0x00, 0x00, 0x00, 0x21, 0xF9, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3B,
        ];
        let set = decode_gif_bytes(&dot).unwrap();
        assert_eq!(set.frames.len(), 1);
        assert_eq!((set.frames[0].w, set.frames[0].h), (1, 1));
        assert_eq!(set.frames[0].ms, 100);
        assert_eq!(&set.frames[0].rgb, &[255, 255, 255]);
        assert_eq!(safe_download_name("a/b c.gif"), "b_c.gif");
        if let Some(bin) = crate::sys::ffmpeg_bin() {
            let gif = dir.join("spin.gif");
            let made = std::process::Command::new(bin)
                .args([
                    "-y",
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc=size=16x16:rate=5:duration=0.4",
                ])
                .arg(&gif)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if made {
                let set = decode_gif(&gif).unwrap();
                assert!(set.frames.len() >= 2, "frames {}", set.frames.len());
                assert_eq!((set.frames[0].w, set.frames[0].h), (16, 16));
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
