use crate::images::TexCache;
use crate::theme::{self, CYAN, DIM, KILL, MUTED, ORANGE};
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FOG: Color32 = Color32::from_rgb(10, 10, 6);

#[derive(Clone, Default)]
pub struct TokenSpec {
    pub name: String,
    pub image: String,
    pub sheet: String,
    pub cat: String,
    pub src: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapTok {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub image: String,
    pub sheet: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawTool {
    #[default]
    Off,
    Ink,
    Circle,
    Square,
    Erase,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Mark {
    pub id: String,
    pub kind: MarkKind,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum MarkKind {
    Stroke { pts: Vec<[f32; 2]>, w: f32 },
    Circle { x: f32, y: f32, r: f32 },
    Square { x: f32, y: f32, s: f32 },
}

pub enum MapOp {
    Add(Mark),
    Del(String),
    Clear,
    Image,
}

pub struct MapBoard {
    pub image: Option<PathBuf>,
    pub tokens: Vec<MapTok>,
    pub marks: Vec<Mark>,
    pub tool: DrawTool,
    pub zoom: f32,
    pub pan: Vec2,
    pub grid: bool,
    pub grid_size: f32,
    drag: Option<(usize, Vec2)>,
    resize: Option<usize>,
    ink: Vec<[f32; 2]>,
    draft: Option<MarkKind>,
}

impl Default for MapBoard {
    fn default() -> Self {
        Self {
            image: None,
            tokens: vec![],
            marks: vec![],
            tool: DrawTool::Off,
            zoom: 1.0,
            pan: Vec2::ZERO,
            grid: true,
            grid_size: 48.0,
            drag: None,
            resize: None,
            ink: vec![],
            draft: None,
        }
    }
}

impl MapBoard {
    pub fn drop_token(&mut self, spec: TokenSpec, nx: f32, ny: f32) {
        self.tokens.push(MapTok {
            id: format!("t{}", rand::random::<u32>()),
            name: spec.name,
            x: nx.clamp(0.02, 0.98),
            y: ny.clamp(0.02, 0.98),
            size: 0.08,
            image: spec.image,
            sheet: spec.sheet,
        });
    }
}

pub fn ui(
    ui: &mut egui::Ui,
    board: &mut MapBoard,
    tex: &mut TexCache,
    root: &Path,
    target: &mut Option<String>,
    is_gm: bool,
) -> (Vec<TokenSpec>, Vec<MapOp>) {
    let mut dropped = Vec::new();
    let mut ops = Vec::new();
    if !is_gm {
        board.tool = DrawTool::Off;
        board.ink.clear();
        board.draft = None;
    }
    ui.horizontal_wrapped(|ui| {
        theme::section_head(ui, "07", "MAPS");
        if theme::neon_btn(ui, "Import picture").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["jpg", "jpeg", "png", "webp"])
                .set_title("Import a play map")
                .pick_file()
            {
                board.image = Some(path);
                board.tokens.clear();
                board.marks.clear();
                board.zoom = 1.0;
                board.pan = Vec2::ZERO;
                ops.push(MapOp::Clear);
                ops.push(MapOp::Image);
            }
        }
        if theme::neon_btn_color(ui, "Grid", ORANGE, board.grid).clicked() {
            board.grid = !board.grid;
        }
        ui.label(RichText::new("cell").color(DIM).small());
        ui.add(egui::DragValue::new(&mut board.grid_size).range(16.0..=120.0));
        if theme::neon_btn(ui, "Fit").clicked() {
            board.zoom = 1.0;
            board.pan = Vec2::ZERO;
        }
        if is_gm {
            ui.label(RichText::new("DRAW").color(CYAN).family(theme::mono()).small());
            if theme::neon_btn_color(ui, "Ink", ORANGE, board.tool == DrawTool::Ink).clicked() {
                board.tool = if board.tool == DrawTool::Ink {
                    DrawTool::Off
                } else {
                    DrawTool::Ink
                };
            }
            if theme::neon_btn_color(ui, "Circle", ORANGE, board.tool == DrawTool::Circle).clicked()
            {
                board.tool = if board.tool == DrawTool::Circle {
                    DrawTool::Off
                } else {
                    DrawTool::Circle
                };
            }
            if theme::neon_btn_color(ui, "Square", ORANGE, board.tool == DrawTool::Square).clicked()
            {
                board.tool = if board.tool == DrawTool::Square {
                    DrawTool::Off
                } else {
                    DrawTool::Square
                };
            }
            if theme::neon_btn_color(ui, "Erase", CYAN, board.tool == DrawTool::Erase).clicked() {
                board.tool = if board.tool == DrawTool::Erase {
                    DrawTool::Off
                } else {
                    DrawTool::Erase
                };
            }
            if !board.marks.is_empty()
                && theme::neon_btn_color(ui, "Clear drawings", KILL, false).clicked()
            {
                board.marks.clear();
                board.draft = None;
                board.ink.clear();
                ops.push(MapOp::Clear);
            }
        }
        wrap_hint(ui, is_gm);
    });
    let dropped_files: Vec<PathBuf> = ui.ctx().input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    for path in dropped_files {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
            if board.image.is_none() {
                board.image = Some(path);
                ops.push(MapOp::Image);
            } else {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Token")
                    .to_string();
                let image = crate::images::stash_file(root, "data/map-cache", &path)
                    .unwrap_or_else(|| crate::images::portable_rel(root, &path.to_string_lossy()));
                board.drop_token(
                    TokenSpec {
                        name,
                        image,
                        sheet: String::new(),
                        cat: String::new(),
                        src: String::new(),
                    },
                    0.5,
                    0.5,
                );
            }
        }
    }

    let avail = ui.available_size();
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(avail.x.max(80.0), (avail.y - 8.0).max(80.0)),
        Sense::click_and_drag(),
    );

    if let Some(spec) = resp.dnd_release_payload::<TokenSpec>() {
        if let Some(pos) = resp.hover_pos() {
            let nx = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let ny = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
            let mut spec = (*spec).clone();
            spec.image = crate::images::stash_file(root, "data/map-cache", Path::new(&spec.image))
                .unwrap_or_else(|| crate::images::portable_rel(root, &spec.image));
            board.drop_token(spec.clone(), nx, ny);
            dropped.push(spec);
        }
    }

    if let Some(path) = board.image.clone() {
        let inner = world_rect(rect, board);
        crate::images::paint_cover(ui, tex, &path, inner);
        if board.grid {
            draw_grid(ui, inner, board);
        }
        paint_marks(ui, inner, &board.marks, board.draft.as_ref());
        let mut hit: Option<usize> = None;
        for (i, tok) in board.tokens.iter().enumerate() {
            let c = inner.lerp_inside(Vec2::new(tok.x, tok.y));
            let s = tok.size * inner.width().min(inner.height());
            let tr = Rect::from_center_size(c, Vec2::splat(s.max(18.0)));
            if !tok.image.is_empty() {
                let p = crate::images::resolve_rel(root, &tok.image);
                crate::images::paint_cover(ui, tex, &p, tr);
            } else {
                ui.painter().circle_filled(c, s * 0.45, ORANGE);
            }
            let aimed = target.as_deref() == Some(tok.sheet.as_str())
                || target.as_deref() == Some(tok.id.as_str());
            ui.painter().rect_stroke(
                tr,
                2.0,
                Stroke::new(if aimed { 2.4 } else { 1.0 }, if aimed { CYAN } else { ORANGE }),
                egui::StrokeKind::Outside,
            );
            ui.painter().text(
                tr.center_bottom() + Vec2::new(0.0, 2.0),
                egui::Align2::CENTER_TOP,
                &tok.name,
                egui::FontId::new(11.0, theme::mono()),
                if aimed { CYAN } else { ORANGE },
            );
            let handle = Rect::from_min_size(tr.right_bottom() - Vec2::splat(10.0), Vec2::splat(10.0));
            ui.painter().rect_filled(handle, 0.0, ORANGE);
            if let Some(pos) = resp.hover_pos() {
                if tr.contains(pos) {
                    hit = Some(i);
                }
            }
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let before = board.zoom;
                board.zoom = (board.zoom * (1.0 + scroll * 0.002)).clamp(0.4, 4.0);
                let _ = before;
            }
        }
        let drawing = is_gm
            && matches!(
                board.tool,
                DrawTool::Ink | DrawTool::Circle | DrawTool::Square
            );
        let erasing = is_gm && board.tool == DrawTool::Erase;
        if resp.dragged_by(PointerButton::Middle)
            || (resp.dragged_by(PointerButton::Primary) && ui.input(|i| i.modifiers.alt))
        {
            board.pan += resp.drag_delta();
            board.drag = None;
            board.ink.clear();
            board.draft = None;
        } else if drawing && resp.drag_started_by(PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                let n = to_norm(inner, pos);
                match board.tool {
                    DrawTool::Ink => {
                        board.ink.clear();
                        board.ink.push(n);
                        board.draft = Some(MarkKind::Stroke {
                            pts: board.ink.clone(),
                            w: 0.012,
                        });
                    }
                    DrawTool::Circle => {
                        board.draft = Some(MarkKind::Circle {
                            x: n[0],
                            y: n[1],
                            r: 0.01,
                        });
                    }
                    DrawTool::Square => {
                        board.draft = Some(MarkKind::Square {
                            x: n[0],
                            y: n[1],
                            s: 0.01,
                        });
                    }
                    _ => {}
                }
            }
        } else if drawing && resp.dragged_by(PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                let n = to_norm(inner, pos);
                match board.tool {
                    DrawTool::Ink => {
                        if board
                            .ink
                            .last()
                            .map(|p| (p[0] - n[0]).hypot(p[1] - n[1]) > 0.004)
                            .unwrap_or(true)
                        {
                            board.ink.push(n);
                        }
                        board.draft = Some(MarkKind::Stroke {
                            pts: board.ink.clone(),
                            w: 0.012,
                        });
                    }
                    DrawTool::Circle => {
                        if let Some(MarkKind::Circle { x, y, r }) = board.draft.as_mut() {
                            *r = (n[0] - *x).hypot(n[1] - *y).max(0.01);
                        }
                    }
                    DrawTool::Square => {
                        if let Some(MarkKind::Square { x, y, s }) = board.draft.as_mut() {
                            *s = (n[0] - *x).abs().max((n[1] - *y).abs()).max(0.01);
                        }
                    }
                    _ => {}
                }
            }
        } else if drawing && resp.drag_stopped() {
            if let Some(kind) = board.draft.take() {
                let ok = match &kind {
                    MarkKind::Stroke { pts, .. } => pts.len() >= 2,
                    MarkKind::Circle { r, .. } => *r >= 0.012,
                    MarkKind::Square { s, .. } => *s >= 0.012,
                };
                if ok {
                    let mark = Mark {
                        id: format!("m{:08x}", rand::random::<u32>()),
                        kind,
                    };
                    board.marks.push(mark.clone());
                    ops.push(MapOp::Add(mark));
                }
            }
            board.ink.clear();
        } else if resp.drag_started_by(PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                let mut start_drag = None;
                let mut start_resize = None;
                for (i, tok) in board.tokens.iter().enumerate() {
                    let c = inner.lerp_inside(Vec2::new(tok.x, tok.y));
                    let s = tok.size * inner.width().min(inner.height());
                    let tr = Rect::from_center_size(c, Vec2::splat(s.max(18.0)));
                    let handle =
                        Rect::from_min_size(tr.right_bottom() - Vec2::splat(12.0), Vec2::splat(12.0));
                    if handle.contains(pos) {
                        start_resize = Some(i);
                        break;
                    }
                    if tr.contains(pos) {
                        start_drag = Some((i, pos - c));
                        break;
                    }
                }
                board.resize = start_resize;
                board.drag = start_drag;
                if board.drag.is_none() && board.resize.is_none() {
                    board.pan += Vec2::ZERO;
                }
            }
        }
        if !drawing && resp.dragged_by(PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some(i) = board.resize {
                    let tok_c = {
                        let tok = &board.tokens[i];
                        inner.lerp_inside(Vec2::new(tok.x, tok.y))
                    };
                    let d = (pos - tok_c).length();
                    if let Some(tok) = board.tokens.get_mut(i) {
                        tok.size = (d * 2.0 / inner.width().min(inner.height())).clamp(0.03, 0.4);
                    }
                } else if let Some((i, off)) = board.drag {
                    let p = pos - off;
                    let nx = ((p.x - inner.left()) / inner.width()).clamp(0.02, 0.98);
                    let ny = ((p.y - inner.top()) / inner.height()).clamp(0.02, 0.98);
                    if let Some(tok) = board.tokens.get_mut(i) {
                        tok.x = nx;
                        tok.y = ny;
                    }
                } else {
                    board.pan += resp.drag_delta();
                }
            }
        }
        if resp.drag_stopped() {
            board.drag = None;
            board.resize = None;
        }
        if resp.clicked() {
            if erasing {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let n = to_norm(inner, pos);
                    if let Some(i) = hit_mark(&board.marks, n) {
                        let id = board.marks[i].id.clone();
                        board.marks.remove(i);
                        ops.push(MapOp::Del(id));
                    }
                }
            } else if let Some(i) = hit {
                let tok = &board.tokens[i];
                let id = if tok.sheet.is_empty() {
                    tok.id.clone()
                } else {
                    tok.sheet.clone()
                };
                *target = Some(id);
            }
        }
        if resp.clicked_by(PointerButton::Secondary) {
            if let Some(i) = hit {
                board.tokens.remove(i);
            }
        }
    } else {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(10, 10, 6));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Import a battle map, or drop a picture here.\nDrag a character or catalog portrait onto the map to make a token.",
            egui::FontId::new(13.0, theme::ui_font()),
            MUTED,
        );
    }
    (dropped, ops)
}

fn world_rect(view: Rect, board: &MapBoard) -> Rect {
    let c = view.center() + board.pan;
    let size = view.size() * board.zoom;
    Rect::from_center_size(c, size)
}

fn draw_grid(ui: &egui::Ui, inner: Rect, board: &MapBoard) {
    let step = board.grid_size * board.zoom;
    if step < 8.0 {
        return;
    }
    let p = ui.painter().with_clip_rect(inner);
    let mut x = inner.left();
    while x <= inner.right() {
        p.vline(
            x,
            inner.y_range(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 50)),
        );
        x += step;
    }
    let mut y = inner.top();
    while y <= inner.bottom() {
        p.hline(
            inner.x_range(),
            y,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 106, 18, 50)),
        );
        y += step;
    }
}

fn to_norm(inner: Rect, pos: Pos2) -> [f32; 2] {
    [
        ((pos.x - inner.left()) / inner.width().max(1.0)).clamp(0.0, 1.0),
        ((pos.y - inner.top()) / inner.height().max(1.0)).clamp(0.0, 1.0),
    ]
}

fn from_norm(inner: Rect, n: [f32; 2]) -> Pos2 {
    Pos2::new(
        inner.left() + n[0] * inner.width(),
        inner.top() + n[1] * inner.height(),
    )
}

fn paint_marks(ui: &egui::Ui, inner: Rect, marks: &[Mark], draft: Option<&MarkKind>) {
    let p = ui.painter().with_clip_rect(inner);
    let w = inner.width().max(1.0);
    for m in marks {
        paint_kind(&p, inner, w, &m.kind);
    }
    if let Some(kind) = draft {
        paint_kind(&p, inner, w, kind);
    }
}

fn paint_kind(p: &egui::Painter, inner: Rect, w: f32, kind: &MarkKind) {
    match kind {
        MarkKind::Stroke { pts, w: nw } => {
            if pts.len() < 2 {
                return;
            }
            let width = (*nw * w).clamp(2.0, 18.0);
            for pair in pts.windows(2) {
                p.line_segment(
                    [from_norm(inner, pair[0]), from_norm(inner, pair[1])],
                    Stroke::new(width + 4.0, FOG),
                );
                p.line_segment(
                    [from_norm(inner, pair[0]), from_norm(inner, pair[1])],
                    Stroke::new(width, ORANGE),
                );
            }
        }
        MarkKind::Circle { x, y, r } => {
            let c = from_norm(inner, [*x, *y]);
            let rad = (*r * w).max(4.0);
            p.circle_filled(c, rad, FOG);
            p.circle_stroke(c, rad, Stroke::new(2.0, ORANGE));
        }
        MarkKind::Square { x, y, s } => {
            let c = from_norm(inner, [*x, *y]);
            let half = (*s * w).max(4.0);
            let r = Rect::from_center_size(c, Vec2::splat(half * 2.0));
            p.rect_filled(r, 0.0, FOG);
            p.rect_stroke(r, 0.0, Stroke::new(2.0, ORANGE), egui::StrokeKind::Outside);
        }
    }
}

fn hit_mark(marks: &[Mark], n: [f32; 2]) -> Option<usize> {
    marks.iter().enumerate().rev().find_map(|(i, m)| {
        let hit = match &m.kind {
            MarkKind::Stroke { pts, w } => pts
                .iter()
                .any(|p| (p[0] - n[0]).hypot(p[1] - n[1]) < w.max(0.02)),
            MarkKind::Circle { x, y, r } => (x - n[0]).hypot(y - n[1]) <= *r,
            MarkKind::Square { x, y, s } => (n[0] - x).abs() <= *s && (n[1] - y).abs() <= *s,
        };
        hit.then_some(i)
    })
}

fn wrap_hint(ui: &mut egui::Ui, is_gm: bool) {
    ui.label(
        RichText::new(if is_gm {
            "Ink / filled Circle / Square to fog the map. Erase click. Clear drawings. Drag tokens. Alt-drag pans."
        } else {
            "Drag tokens. Gold corner resizes. Scroll zooms. Alt-drag pans. Right-click deletes. Click to target."
        })
            .color(DIM)
            .small(),
    );
}

pub fn drag_source(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    spec: impl FnOnce() -> TokenSpec,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    drag_click(ui, id, spec, add)
}

/// Drag after the pointer moves. A press and release without that move is a click.
/// The payload is built only while a drag is actually in progress.
pub fn drag_click<P>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    payload: impl FnOnce() -> P,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response
where
    P: std::any::Any + Send + Sync,
{
    let id = egui::Id::new(("tok", id));
    let ctx = ui.ctx().clone();
    if ctx.is_being_dragged(id) {
        egui::DragAndDrop::set_payload(&ctx, payload());
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let inner = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), add);
        if let Some(pointer) = ctx.pointer_interact_pos() {
            let delta = pointer - inner.response.rect.center();
            ctx.transform_layer_shapes(
                layer_id,
                egui::emath::TSTransform::from_translation(delta),
            );
        }
        inner.response
    } else {
        let inner = ui.scope(add);
        let drag = ui
            .interact(inner.response.rect, id, Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab);
        drag | inner.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_token_clamps_and_keeps_catalog_sheet() {
        let mut b = MapBoard::default();
        for (x, y) in [(0.0, 0.0), (1.5, -1.0), (0.4, 0.6)] {
            b.drop_token(
                TokenSpec {
                    name: "Goblin".into(),
                    image: "assets/bestiary/goblin.jpg".into(),
                    sheet: String::new(),
                    cat: "Bestiary".into(),
                    src: "goblin".into(),
                },
                x,
                y,
            );
        }
        assert_eq!(b.tokens.len(), 3);
        for t in &b.tokens {
            assert!(t.x >= 0.02 && t.x <= 0.98);
            assert!(t.y >= 0.02 && t.y <= 0.98);
                assert_eq!(t.name, "Goblin");
        }
    }

    #[test]
    fn marks_erase_and_clear() {
        let mut b = MapBoard::default();
        b.marks.push(Mark {
            id: "a".into(),
            kind: MarkKind::Circle {
                x: 0.5,
                y: 0.5,
                r: 0.2,
            },
        });
        b.marks.push(Mark {
            id: "b".into(),
            kind: MarkKind::Square {
                x: 0.1,
                y: 0.1,
                s: 0.05,
            },
        });
        let i = hit_mark(&b.marks, [0.5, 0.5]).unwrap();
        assert_eq!(b.marks[i].id, "a");
        b.marks.remove(i);
        assert_eq!(b.marks.len(), 1);
        b.marks.clear();
        assert!(b.marks.is_empty());
        assert!(hit_mark(&b.marks, [0.5, 0.5]).is_none());
    }
}
