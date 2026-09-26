use eframe::egui::{
    self, Color32, ColorImage, FontId, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2,
};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

const CAP: usize = 64;
const MAX_SIDE: u32 = 2048;

#[derive(Default)]
pub struct TexCache {
    map: HashMap<String, TextureHandle>,
    order: VecDeque<String>,
    sig: HashMap<String, u64>,
}

fn cheap_sig(bytes: &[u8]) -> u64 {
    let mut h = bytes.len() as u64;
    if bytes.len() >= 8 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&bytes[..8]);
        h ^= u64::from_le_bytes(b);
        b.copy_from_slice(&bytes[bytes.len() - 8..]);
        h = h.wrapping_mul(1099511628211) ^ u64::from_le_bytes(b);
        if bytes.len() > 48 {
            let m = bytes.len() / 2;
            b.copy_from_slice(&bytes[m..m + 8]);
            h ^= u64::from_le_bytes(b);
        }
    }
    h
}

impl TexCache {
    pub fn from_bytes(&mut self, ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<TextureHandle> {
        if let Some(tex) = self.map.get(key) {
            return Some(tex.clone());
        }
        self.put_bytes(ctx, key, bytes)
    }

    pub fn put_bytes(&mut self, ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<TextureHandle> {
        if bytes.is_empty() {
            return None;
        }
        let s = cheap_sig(bytes);
        if self.sig.get(key) == Some(&s) {
            return self.map.get(key).cloned();
        }
        let color = decode_rgba(bytes)?;
        self.sig.insert(key.to_string(), s);
        if let Some(tex) = self.map.get_mut(key) {
            tex.set(color, TextureOptions::LINEAR);
            return Some(tex.clone());
        }
        self.evict();
        let tex = ctx.load_texture(key.to_string(), color, TextureOptions::NEAREST);
        self.order.push_back(key.to_string());
        self.map.insert(key.to_string(), tex.clone());
        Some(tex)
    }

    pub fn forget(&mut self, key: &str) {
        self.map.remove(key);
        self.sig.remove(key);
        self.order.retain(|k| k != key);
    }

    pub fn get_key(&self, key: &str) -> Option<TextureHandle> {
        self.map.get(key).cloned()
    }

    fn evict(&mut self) {
        if self.order.len() < CAP {
            return;
        }
        let n = self.order.len();
        for _ in 0..n {
            if self.order.len() < CAP {
                return;
            }
            let Some(old) = self.order.pop_front() else {
                return;
            };
            if old.starts_with("local-") || old.starts_with("cam-") || old.starts_with("scr-") {
                self.order.push_back(old);
                continue;
            }
            self.map.remove(&old);
            self.sig.remove(&old);
            return;
        }
        // Live video keys can fill the cache; drop the oldest anyway.
        if let Some(old) = self.order.pop_front() {
            self.map.remove(&old);
            self.sig.remove(&old);
        }
    }

    pub fn get(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        let key = path.to_string_lossy().to_string();
        if let Some(tex) = self.map.get(&key) {
            return Some(tex.clone());
        }
        let tex = load(ctx, path)?;
        self.evict();
        self.order.push_back(key.clone());
        self.map.insert(key, tex.clone());
        Some(tex)
    }
}

fn decode_rgba(bytes: &[u8]) -> Option<ColorImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let img = img.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

fn load(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let color = decode_rgba(&bytes)?;
    Some(ctx.load_texture(
        path.to_string_lossy().to_string(),
        color,
        TextureOptions::LINEAR,
    ))
}

pub fn probe(path: &Path) -> Result<(u32, u32), String> {
    image::image_dimensions(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Fill `rect` with the picture, cropped to cover (no letterbox).
pub fn paint_cover(ui: &Ui, cache: &mut TexCache, path: &Path, rect: Rect) -> bool {
    let Some(tex) = cache.get(ui.ctx(), path) else {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(18, 14, 10));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No picture",
            FontId::proportional(14.0),
            Color32::from_rgb(160, 140, 100),
        );
        return false;
    };
    let sz = tex.size_vec2();
    if sz.x <= 0.0 || sz.y <= 0.0 {
        return false;
    }
    let scale = (rect.width() / sz.x).max(rect.height() / sz.y);
    let dest = Rect::from_center_size(rect.center(), sz * scale);
    ui.painter().with_clip_rect(rect).image(
        tex.id(),
        dest,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    true
}

pub fn show_bytes(
    ui: &mut Ui,
    cache: &mut TexCache,
    key: &str,
    bytes: &[u8],
    max: Vec2,
) -> egui::Response {
    if bytes.is_empty() {
        let (rect, resp) = ui.allocate_exact_size(max.min(Vec2::new(220.0, 140.0)), Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_rgb(20, 16, 12));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No feed",
            FontId::proportional(12.0),
            Color32::from_rgb(160, 140, 100),
        );
        return resp;
    }
    if let Some(tex) = cache.put_bytes(ui.ctx(), key, bytes) {
        let sz = tex.size_vec2();
        let scale = (max.x / sz.x).min(max.y / sz.y).min(1.0);
        let size = Vec2::new((sz.x * scale).max(1.0), (sz.y * scale).max(1.0));
        ui.add(
            egui::Image::from_texture(egui::load::SizedTexture::from(&tex))
                .fit_to_exact_size(size)
                .sense(Sense::click()),
        )
    } else {
        let (rect, resp) = ui.allocate_exact_size(max.min(Vec2::new(220.0, 140.0)), Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_rgb(20, 16, 12));
        resp
    }
}

pub fn show_fit(ui: &mut Ui, cache: &mut TexCache, path: &Path, max: Vec2) -> egui::Response {
    if let Some(tex) = cache.get(ui.ctx(), path) {
        let sz = tex.size_vec2();
        let scale = (max.x / sz.x).min(max.y / sz.y).min(1.0);
        let size = Vec2::new((sz.x * scale).max(1.0), (sz.y * scale).max(1.0));
        ui.add(
            egui::Image::from_texture(egui::load::SizedTexture::from(&tex))
                .fit_to_exact_size(size)
                .sense(Sense::click()),
        )
    } else {
        let (rect, resp) = ui.allocate_exact_size(max.min(Vec2::new(160.0, 90.0)), Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_rgb(20, 16, 12));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No picture",
            FontId::proportional(12.0),
            Color32::GRAY,
        );
        resp
    }
}

pub fn show_cover_click(
    ui: &mut Ui,
    cache: &mut TexCache,
    path: &Path,
    size: Vec2,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    paint_cover(ui, cache, path, rect);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hearth_and_icon_decode() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        probe(&root.join("assets/hearth.jpg")).expect("hearth.jpg");
        probe(&root.join("assets/icon.png")).expect("icon.png");
        probe(&root.join("assets/places/castle-day.jpg")).expect("castle-day");
        probe(&root.join("assets/places-blight/nightcity-day.jpg")).expect("nightcity-day");
        probe(&root.join("assets/bestiary/goblin.jpg")).expect("goblin");
        probe(&root.join("assets/datashard/assassin.jpg")).expect("datashard");
    }

    #[test]
    fn icon_is_square_and_kit_art_finds_pack_or_item() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (w, h) = probe(&root.join("assets/icon.png")).unwrap();
        assert_eq!(w, h);
        assert!(w >= 256);
        let _ = kit_art(&root, "longsword", "Longsword");
    }

    #[test]
    fn portable_paths_survive_windows_separators() {
        let root = PathBuf::from("/home/runner/blightnet");
        assert_eq!(
            portable_rel(&root, "assets/bestiary/goblin.jpg"),
            "assets/bestiary/goblin.jpg"
        );
        assert_eq!(
            portable_rel(&root, r"C:\Users\Ada\blightnet\assets\bestiary\goblin.jpg"),
            "assets/bestiary/goblin.jpg"
        );
        assert_eq!(
            portable_rel(&root, "/home/runner/blightnet/data/chars/x-portrait.jpg"),
            "data/chars/x-portrait.jpg"
        );
        let p = resolve_rel(&root, r"C:\game\assets\places\tavern-day.jpg");
        assert!(p.ends_with("assets/places/tavern-day.jpg") || p.ends_with("assets\\places\\tavern-day.jpg"));
    }
}

/// Forward-slash relative path so Windows and Linux seats resolve the same asset.
pub fn portable_rel(root: &Path, raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let n = raw.replace('\\', "/");
    let p = Path::new(&n);
    if let Ok(rel) = p.strip_prefix(root) {
        return rel.to_string_lossy().replace('\\', "/");
    }
    let root_s = root.to_string_lossy().replace('\\', "/");
    if let Some(rest) = n.strip_prefix(&root_s) {
        return rest.trim_start_matches('/').to_string();
    }
    for needle in ["assets/", "data/"] {
        if let Some(i) = n.find(needle) {
            return n[i..].to_string();
        }
    }
    n
}

pub fn resolve_rel(root: &Path, raw: &str) -> PathBuf {
    if raw.is_empty() {
        return PathBuf::new();
    }
    let n = portable_rel(root, raw);
    let p = Path::new(&n);
    if p.is_absolute() {
        if p.is_file() {
            return p.to_path_buf();
        }
        for needle in ["assets/", "data/"] {
            if let Some(i) = n.find(needle) {
                let cand = root.join(&n[i..]);
                if cand.is_file() {
                    return cand;
                }
            }
        }
        return p.to_path_buf();
    }
    root.join(n.trim_start_matches('/'))
}

pub fn stash_bytes(root: &Path, dir: &str, bytes: &[u8], ext: &str) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let mut h = bytes.len() as u64;
    let take = bytes.len().min(4096);
    for (i, b) in bytes[..take].iter().enumerate() {
        h = h.wrapping_mul(16777619) ^ (*b as u64).wrapping_add(i as u64);
    }
    if bytes.len() > 64 {
        let tail = &bytes[bytes.len() - 64..];
        for b in tail {
            h = h.wrapping_mul(16777619) ^ (*b as u64);
        }
    }
    let ext = match ext.to_lowercase().as_str() {
        "jpeg" => "jpg",
        "png" | "webp" | "jpg" | "gif" => ext,
        _ => "jpg",
    };
    let rel = format!("{dir}/{h:016x}.{ext}");
    let dest = root.join(&rel);
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !dest.is_file() {
        std::fs::write(&dest, bytes).ok()?;
    }
    Some(rel)
}

pub fn stash_file(root: &Path, dir: &str, src: &Path) -> Option<String> {
    if !src.is_file() {
        return None;
    }
    let rel = portable_rel(root, &src.to_string_lossy());
    let cand = root.join(&rel);
    if !Path::new(&rel).is_absolute() && cand.is_file() {
        return Some(rel);
    }
    let bytes = std::fs::read(src).ok()?;
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");
    stash_bytes(root, dir, &bytes, ext)
}

pub fn kit_art(root: &Path, id: &str, name: &str) -> Option<std::path::PathBuf> {
    let dir = root.join("assets/kit/look");
    let slug = name.to_lowercase().replace(' ', "-");
    for stem in [id, slug.as_str()] {
        let p = dir.join(format!("{stem}.jpg"));
        if p.is_file() {
            return Some(p);
        }
    }
    let pack = dir.join("pack.jpg");
    pack.is_file().then_some(pack)
}
