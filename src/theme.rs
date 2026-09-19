use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};
use std::borrow::Cow;
use std::sync::Arc;

pub const ORANGE: Color32 = Color32::from_rgb(255, 106, 18);
pub const BG: Color32 = Color32::from_rgb(12, 12, 8);
pub const PANEL: Color32 = Color32::from_rgb(10, 10, 6);
pub const TITLE: Color32 = Color32::from_rgb(17, 17, 8);
pub const MUTED: Color32 = Color32::from_rgb(154, 147, 96);
pub const DIM: Color32 = Color32::from_rgb(109, 104, 64);
pub const CREAM: Color32 = Color32::from_rgb(207, 200, 147);
pub const CYAN: Color32 = Color32::from_rgb(77, 232, 255);
pub const KILL: Color32 = Color32::from_rgb(255, 0, 60);
pub const INK: Color32 = Color32::from_rgb(17, 17, 17);
pub const RAIL: Color32 = Color32::from_rgb(10, 10, 6);

pub fn mono() -> FontFamily {
    FontFamily::Name("share".into())
}
pub fn display() -> FontFamily {
    FontFamily::Name("oxanium".into())
}
pub fn serif() -> FontFamily {
    FontFamily::Name("cinzel".into())
}
pub fn ui_font() -> FontFamily {
    FontFamily::Name("rajdhani".into())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "rajdhani".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Rajdhani-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "rajdhani_bold".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Rajdhani-Bold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "share".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/ShareTechMono-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "oxanium".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Oxanium-Bold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "cinzel".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Cinzel-SemiBold.ttf"
        ))),
    );
    fonts
        .families
        .insert(ui_font(), vec!["rajdhani".into(), "rajdhani_bold".into()]);
    fonts.families.insert(display(), vec!["oxanium".into()]);
    fonts.families.insert(serif(), vec!["cinzel".into(), "oxanium".into()]);
    fonts.families.insert(mono(), vec!["share".into()]);
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "rajdhani".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "share".into());
    ctx.set_fonts(fonts);
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub gold: Color32,
    pub gold_soft: Color32,
    pub ember: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub dim: Color32,
    pub panel: Color32,
    pub bg: Color32,
    pub cut: f32,
}

impl Palette {
    pub fn line(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.gold.r(), self.gold.g(), self.gold.b(), 90)
    }
    pub fn fill(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.gold.r(), self.gold.g(), self.gold.b(), 22)
    }
    pub fn rounding(self) -> f32 {
        if self.cut > 6.0 {
            0.0
        } else {
            12.0
        }
    }
    pub fn head_family(self) -> FontFamily {
        if self.cut > 6.0 {
            display()
        } else {
            serif()
        }
    }
}

pub fn index_palette() -> Palette {
    Palette {
        gold: ORANGE,
        gold_soft: Color32::from_rgb(255, 176, 96),
        ember: CYAN,
        ink: CREAM,
        muted: MUTED,
        dim: DIM,
        panel: PANEL,
        bg: BG,
        cut: 8.0,
    }
}

pub fn fade(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn pane() -> egui::Frame {
    egui::Frame::NONE
        .fill(PANEL)
        .stroke(Stroke::new(1.0, ORANGE))
        .inner_margin(egui::Margin::symmetric(12, 10))
}

pub fn plate(ui: &Ui, rect: Rect) {
    fill_chamfer(ui, rect, 10.0, PANEL, Stroke::new(1.5, ORANGE));
    ui.painter().rect_stroke(
        rect.shrink(3.0),
        0.0,
        Stroke::new(1.0, fade(CYAN, 55)),
        StrokeKind::Inside,
    );
    hairline_top(ui, rect.shrink(1.0), fade(ORANGE, 90));
    brackets(ui, rect.shrink(8.0), fade(CYAN, 180), 12.0);
}

pub fn hairline_top(ui: &Ui, rect: Rect, color: Color32) {
    ui.painter().hline(
        rect.x_range(),
        rect.top() + 1.0,
        Stroke::new(1.0, color),
    );
}

#[allow(dead_code)]
pub fn hatch_bar(ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, TITLE);
    hatch(ui, rect, Color32::from_rgba_unmultiplied(255, 106, 18, 16));
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(2.0, ORANGE));
}

pub fn wide_btn(ui: &mut Ui, label: &str, sub: &str, on: bool) -> egui::Response {
    let w = ui.available_width().max(40.0);
    let h = if sub.is_empty() { 36.0 } else { 52.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let hover = resp.hovered();
    let fill = if on {
        ORANGE
    } else if hover {
        fade(ORANGE, 48)
    } else {
        fade(ORANGE, 14)
    };
    fill_chamfer(ui, rect, 8.0, fill, Stroke::new(if on { 1.6 } else { 1.0 }, ORANGE));
    if hover && !on {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            0.0,
            Stroke::new(1.0, fade(ORANGE, 80)),
            StrokeKind::Outside,
        );
    }
    hairline_top(ui, rect.shrink(2.0), fade(CREAM, if on || hover { 50 } else { 22 }));
    if on {
        let tick = Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 6.0), Vec2::new(3.0, rect.height() - 12.0));
        ui.painter().rect_filled(tick, 0.0, CYAN);
    }
    let fg = if on || hover { INK } else { ORANGE };
    let muted = if on || hover {
        Color32::from_rgb(40, 32, 16)
    } else {
        MUTED
    };
    let clip = rect.shrink2(Vec2::new(12.0, 4.0));
    let p = ui.painter().with_clip_rect(clip);
    p.text(
        clip.left_top() + Vec2::new(2.0, if sub.is_empty() { 6.0 } else { 3.0 }),
        egui::Align2::LEFT_TOP,
        label,
        FontId::new(16.0, ui_font()),
        fg,
    );
    if !sub.is_empty() {
        p.text(
            clip.left_bottom() + Vec2::new(2.0, -5.0),
            egui::Align2::LEFT_BOTTOM,
            sub,
            FontId::new(11.0, mono()),
            muted,
        );
    }
    let tip = if sub.is_empty() {
        hint_for(label).into_owned()
    } else {
        format!("{label} — {sub}")
    };
    attach_tip(resp, tip)
}

pub fn section_head(ui: &mut Ui, id: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(id)
                .family(mono())
                .size(13.0)
                .color(CYAN),
        );
        ui.label(
            egui::RichText::new(title)
                .family(display())
                .size(18.0)
                .color(ORANGE),
        );
        let (r, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(8.0), 2.0),
            Sense::hover(),
        );
        ui.painter().rect_filled(r, 0.0, fade(ORANGE, 140));
        ui.painter().rect_filled(
            Rect::from_min_size(r.left_top(), Vec2::new((r.width() * 0.22).max(8.0), 2.0)),
            0.0,
            CYAN,
        );
    });
}

#[allow(dead_code)]
pub fn world_palette(blight: bool, time: &str) -> Palette {
    if blight {
        match time {
            "morning" => Palette {
                gold: Color32::from_rgb(255, 229, 106),
                gold_soft: Color32::from_rgb(255, 229, 106),
                ember: Color32::from_rgb(122, 240, 200),
                ink: Color32::from_rgb(244, 247, 232),
                muted: Color32::from_rgb(154, 168, 106),
                dim: Color32::from_rgb(109, 122, 82),
                panel: Color32::from_rgba_unmultiplied(8, 12, 14, 220),
                bg: Color32::from_rgb(5, 6, 8),
                cut: 10.0,
            },
            "evening" => Palette {
                gold: Color32::from_rgb(255, 90, 31),
                gold_soft: Color32::from_rgb(255, 176, 138),
                ember: Color32::from_rgb(255, 0, 60),
                ink: Color32::from_rgb(255, 232, 220),
                muted: Color32::from_rgb(154, 168, 106),
                dim: Color32::from_rgb(109, 122, 82),
                panel: Color32::from_rgba_unmultiplied(8, 12, 14, 220),
                bg: Color32::from_rgb(5, 6, 8),
                cut: 10.0,
            },
            "night" => Palette {
                gold: Color32::from_rgb(0, 240, 255),
                gold_soft: Color32::from_rgb(184, 255, 255),
                ember: Color32::from_rgb(255, 45, 106),
                ink: Color32::from_rgb(232, 251, 255),
                muted: Color32::from_rgb(122, 168, 184),
                dim: Color32::from_rgb(78, 115, 128),
                panel: Color32::from_rgba_unmultiplied(8, 12, 14, 230),
                bg: Color32::from_rgb(5, 6, 8),
                cut: 10.0,
            },
            _ => Palette {
                gold: Color32::from_rgb(255, 45, 106),
                gold_soft: Color32::from_rgb(255, 245, 106),
                ember: Color32::from_rgb(0, 240, 255),
                ink: Color32::from_rgb(244, 247, 232),
                muted: Color32::from_rgb(154, 168, 106),
                dim: Color32::from_rgb(109, 122, 82),
                panel: Color32::from_rgba_unmultiplied(8, 12, 14, 220),
                bg: Color32::from_rgb(5, 6, 8),
                cut: 10.0,
            },
        }
    } else {
        match time {
            "morning" => Palette {
                gold: Color32::from_rgb(232, 192, 112),
                gold_soft: Color32::from_rgb(244, 220, 156),
                ember: Color32::from_rgb(212, 104, 56),
                ink: Color32::from_rgb(248, 238, 220),
                muted: Color32::from_rgb(212, 176, 122),
                dim: Color32::from_rgb(154, 116, 80),
                panel: Color32::from_rgba_unmultiplied(42, 24, 12, 220),
                bg: Color32::from_rgb(26, 20, 12),
                cut: 4.0,
            },
            "evening" => Palette {
                gold: Color32::from_rgb(224, 112, 56),
                gold_soft: Color32::from_rgb(240, 160, 112),
                ember: Color32::from_rgb(160, 40, 24),
                ink: Color32::from_rgb(248, 228, 212),
                muted: Color32::from_rgb(208, 128, 80),
                dim: Color32::from_rgb(138, 72, 48),
                panel: Color32::from_rgba_unmultiplied(40, 14, 8, 224),
                bg: Color32::from_rgb(24, 10, 6),
                cut: 4.0,
            },
            "night" => Palette {
                gold: Color32::from_rgb(200, 184, 120),
                gold_soft: Color32::from_rgb(221, 208, 160),
                ember: Color32::from_rgb(106, 74, 136),
                ink: Color32::from_rgb(232, 228, 240),
                muted: Color32::from_rgb(168, 152, 120),
                dim: Color32::from_rgb(106, 92, 112),
                panel: Color32::from_rgba_unmultiplied(16, 14, 28, 224),
                bg: Color32::from_rgb(12, 10, 20),
                cut: 4.0,
            },
            _ => Palette {
                gold: Color32::from_rgb(226, 179, 74),
                gold_soft: Color32::from_rgb(240, 208, 138),
                ember: Color32::from_rgb(194, 74, 34),
                ink: Color32::from_rgb(246, 234, 216),
                muted: Color32::from_rgb(201, 160, 106),
                dim: Color32::from_rgb(138, 104, 68),
                panel: Color32::from_rgba_unmultiplied(36, 18, 10, 220),
                bg: Color32::from_rgb(22, 14, 8),
                cut: 4.0,
            },
        }
    }
}

#[allow(dead_code)]
pub fn world_visuals(pal: Palette) -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(pal.ink);
    v.panel_fill = pal.bg;
    v.window_fill = pal.panel;
    v.extreme_bg_color = pal.bg;
    v.widgets.inactive.bg_fill = pal.panel;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.gold);
    v.widgets.hovered.bg_fill = pal.fill();
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, pal.gold_soft);
    v.widgets.active.bg_fill = pal.gold;
    v.widgets.active.fg_stroke = Stroke::new(1.0, INK);
    v.selection.bg_fill = pal.gold;
    v.selection.stroke = Stroke::new(1.0, INK);
    v.hyperlink_color = pal.ember;
    v.window_stroke = Stroke::new(1.0, pal.gold);
    v
}

pub fn fill_world_panel(ui: &Ui, rect: Rect, pal: Palette) {
    if pal.cut > 6.0 {
        fill_chamfer(ui, rect, pal.cut, pal.panel, Stroke::new(1.0, pal.gold));
    } else {
        ui.painter()
            .rect_filled(rect, pal.rounding(), pal.panel);
        ui.painter().rect_stroke(
            rect,
            pal.rounding(),
            Stroke::new(1.0, pal.line()),
            StrokeKind::Inside,
        );
    }
}

pub fn ghost_btn(ui: &mut Ui, label: &str, pal: Palette, on: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::new(13.0, ui_font()),
        if on { INK } else { pal.gold },
    );
    let size = Vec2::new((galley.size().x + 16.0).max(36.0), 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let fill = if on {
        pal.gold
    } else if hover {
        pal.fill()
    } else {
        Color32::TRANSPARENT
    };
    let stroke = Stroke::new(1.0, if on || hover { pal.gold } else { pal.line() });
    if pal.cut > 6.0 {
        fill_chamfer(ui, rect, pal.cut.min(8.0), fill, stroke);
    } else {
        ui.painter().rect_filled(rect, 8.0, fill);
        ui.painter()
            .rect_stroke(rect, 8.0, stroke, StrokeKind::Inside);
    }
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        if on { INK } else if hover { pal.ink } else { pal.gold },
    );
    resp
}

pub fn analog_watch(ui: &mut Ui, minutes: u32, pal: Palette) -> egui::Response {
    let size = Vec2::splat(48.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let c = rect.center();
    let r = 21.0;
    let p = ui.painter();
    p.circle_filled(c, r, pal.panel);
    p.circle_stroke(c, r, Stroke::new(1.8, pal.gold));
    p.circle_stroke(c, r - 3.5, Stroke::new(1.0, fade(CYAN, 120)));
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.0;
        let outer = Vec2::new(a.sin() * (r - 4.5), -a.cos() * (r - 4.5));
        let inner = Vec2::new(a.sin() * (r - 7.5), -a.cos() * (r - 7.5));
        p.line_segment(
            [c + inner, c + outer],
            Stroke::new(if i % 3 == 0 { 1.6 } else { 0.8 }, pal.gold),
        );
    }
    let mins = (minutes % 1440) as f32;
    let h = mins / 60.0;
    let m = mins % 60.0;
    let hour_a = ((h % 12.0) + m / 60.0) / 12.0 * std::f32::consts::TAU;
    let min_a = m / 60.0 * std::f32::consts::TAU;
    p.line_segment(
        [
            c,
            c + Vec2::new(hour_a.sin() * 9.0, -hour_a.cos() * 9.0),
        ],
        Stroke::new(2.4, pal.gold),
    );
    p.line_segment(
        [c, c + Vec2::new(min_a.sin() * 13.5, -min_a.cos() * 13.5)],
        Stroke::new(1.3, CYAN),
    );
    p.circle_filled(c, 2.4, pal.gold);
    p.circle_filled(c, 1.1, INK);
    resp
}

pub fn place_btn(ui: &mut Ui, label: &str, on: bool, pal: Palette) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::new(13.0, ui_font()),
        if on { INK } else { pal.gold },
    );
    let size = Vec2::new(galley.size().x + 18.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if on { pal.gold } else { pal.panel };
    fill_chamfer(ui, rect, pal.cut, fill, Stroke::new(1.0, pal.gold));
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        if on { INK } else { pal.gold },
    );
    resp
}

pub fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(ORANGE);
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(5, 5, 3);
    v.faint_bg_color = Color32::from_rgb(16, 14, 8);
    v.code_bg_color = Color32::from_rgb(8, 8, 4);
    v.hyperlink_color = CYAN;
    v.window_stroke = Stroke::new(1.0, ORANGE);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.45 };
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.inactive.bg_fill = PANEL;
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(14, 12, 8);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, fade(ORANGE, 110));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(42, 28, 10);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(42, 28, 10);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(255, 176, 96));
    v.widgets.active.bg_fill = ORANGE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, CYAN);
    v.widgets.active.fg_stroke = Stroke::new(1.0, INK);
    v.widgets.open.bg_fill = Color32::from_rgb(22, 16, 8);
    v.widgets.open.bg_stroke = Stroke::new(1.0, ORANGE);
    v.selection.bg_fill = fade(ORANGE, 170);
    v.selection.stroke = Stroke::new(1.0, CYAN);
    v.text_cursor.stroke = Stroke::new(1.6, CYAN);
    v.popup_shadow = egui::Shadow {
        offset: [2, 3],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(180),
    };
    v
}

pub fn chamfer(rect: Rect, cut: f32) -> Vec<Pos2> {
    let c = cut.min(rect.width() * 0.25).min(rect.height() * 0.25).max(0.0);
    vec![
        Pos2::new(rect.left() + c, rect.top()),
        Pos2::new(rect.right(), rect.top()),
        Pos2::new(rect.right(), rect.bottom() - c),
        Pos2::new(rect.right() - c, rect.bottom()),
        Pos2::new(rect.left(), rect.bottom()),
        Pos2::new(rect.left(), rect.top() + c),
    ]
}

pub fn fill_chamfer(ui: &Ui, rect: Rect, cut: f32, fill: Color32, stroke: Stroke) {
    ui.painter()
        .add(Shape::convex_polygon(chamfer(rect, cut), fill, stroke));
}

pub fn hatch(ui: &Ui, rect: Rect, color: Color32) {
    let painter = ui.painter().with_clip_rect(rect);
    let mut x = rect.left() - rect.height();
    while x < rect.right() {
        painter.line_segment(
            [
                Pos2::new(x, rect.top()),
                Pos2::new(x + rect.height(), rect.bottom()),
            ],
            Stroke::new(1.5, color),
        );
        x += 11.0;
    }
}

pub fn scanlines(ui: &Ui, rect: Rect) {
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_black_alpha(16));
}

pub fn brackets(ui: &Ui, rect: Rect, color: Color32, size: f32) {
    let s = Stroke::new(2.0, color);
    let p = ui.painter();
    p.line_segment([rect.left_top(), rect.left_top() + Vec2::new(size, 0.0)], s);
    p.line_segment([rect.left_top(), rect.left_top() + Vec2::new(0.0, size)], s);
    p.line_segment([rect.right_bottom(), rect.right_bottom() - Vec2::new(size, 0.0)], s);
    p.line_segment([rect.right_bottom(), rect.right_bottom() - Vec2::new(0.0, size)], s);
}

pub fn kicker(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("▸")
                .color(CYAN)
                .family(mono())
                .size(11.0),
        );
        ui.label(
            egui::RichText::new(text)
                .color(ORANGE)
                .family(mono())
                .size(11.0),
        );
    });
}

pub fn neon_btn(ui: &mut Ui, label: &str) -> egui::Response {
    neon_btn_color(ui, label, ORANGE, false)
}

pub fn neon_btn_color(ui: &mut Ui, label: &str, color: Color32, solid: bool) -> egui::Response {
    attach_tip(neon_btn_paint(ui, label, color, solid), hint_for(label))
}

fn neon_btn_paint(ui: &mut Ui, label: &str, color: Color32, solid: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_uppercase(),
        FontId::new(13.0, ui_font()),
        if solid { INK } else { color },
    );
    let size = Vec2::new((galley.size().x + 22.0).max(36.0), 30.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let fill = if solid || hover {
        color
    } else {
        fade(color, 16)
    };
    let fg = if solid || hover { INK } else { color };
    let thick = if solid { 1.8 } else { 1.0 };
    fill_chamfer(ui, rect, 6.0, fill, Stroke::new(thick, color));
    hairline_top(ui, rect.shrink(1.5), fade(CREAM, if solid || hover { 55 } else { 28 }));
    if solid {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 1.5,
            Stroke::new(1.5, fade(CYAN, 180)),
        );
    }
    if hover {
        ui.painter().rect_stroke(
            rect.expand(1.5),
            0.0,
            Stroke::new(1.0, fade(color, 100)),
            StrokeKind::Outside,
        );
        brackets(ui, rect.shrink(3.0), fade(CYAN, 160), 5.0);
    }
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        fg,
    );
    resp
}

/// Same chrome as `neon_btn_color`, with an exact hover tip (seat names, tracks).
pub fn neon_btn_tip(
    ui: &mut Ui,
    label: &str,
    color: Color32,
    solid: bool,
    tip: &str,
) -> egui::Response {
    attach_tip(neon_btn_paint(ui, label, color, solid), tip.to_string())
}

fn attach_tip(resp: egui::Response, hint: impl Into<String>) -> egui::Response {
    let hint = hint.into();
    if hint.is_empty() {
        return resp;
    }
    resp.on_hover_ui_at_pointer(move |ui| {
        ui.set_max_width(280.0);
        ui.label(
            egui::RichText::new(&hint)
                .size(13.0)
                .color(CREAM)
                .family(ui_font()),
        );
    })
}

fn hint_for(label: &str) -> Cow<'static, str> {
    let t = label.trim();
    let known = match t {
        "Host" => "Open a table. Friends Join with the address you copy.",
        "Join" => "Paste a blightnet:// invite from the host. This program connects and decrypts the table.",
        "Leave" | "Leave table" => "Leave this table. Mix and map stay on the host.",
        "Copy address" | "Copy LAN address" => "Copy the blightnet:// join link for this table.",
        "Copy invite link" => "Copy the encrypted blightnet:// invite. Friends paste it into Join. No extra program.",
        "Go online" | "Online" => {
            "Start or stop the node. Online = node active (host, join, calls). Press again to take it offline."
        }
        "Go offline" => "Stop the node. Hosting, joins, and presence go down.",
        "Chat" => "Open or close table talk, DMs, and file send.",
        "Contacts" => "Open or close people you have saved. Dial, DM, or call.",
        "Voice" => "Open or close mic, mute, and voice calls.",
        "Video" => "Open video chat, or send a video file from this chat.",
        "Update" => "Rescan mics, speakers, and cameras on this deck.",
        "Play" => "Play or resume your local music player. Does not change the table mix.",
        "Pause" => "Pause the local player. Does not change the table mix.",
        "Prev" => "Previous track in your local library.",
        "Next" => "Next track in your local library.",
        "Library" => "Add files or folders to the local player.",
        "Record" => "Record a voice note from this mic, then Stop to send.",
        "Image" => "Send a picture to this chat.",
        "Audio" => "Send an audio file to this chat.",
        "File" => "Send any file (up to about 96 MB).",
        "Send" => "Send the text in the box.",
        "Wipe chat" => "Erase this deck's saved chat. Others keep theirs.",
        "Characters" => "Open or close your character sheet on the right. Press again to close.",
        "Log" => "Open or close the shared combat log on the right. Press again to close.",
        "Notes" => "Open or close private notes on the right. Only you see them. Press again to close.",
        "Armory" => "Open or close GM gear. Drag items onto a sheet. Press again to close.",
        "Vendors" | "Night Market" => {
            "Open or close the player shop. Stock is limited by level. Press again to close."
        }
        "Maps" => "Open or close the play map. Press again to close.",
        "Jack-in" => "Open the NETSPACE city tab.",
        "21" => "Open or close House 21. Press again to close.",
        "Add sound" => "Add an audio file to this table's mix. Other seats hear it.",
        "Blight" | "Hearthsong" => "Switch the table world. Mix and catalogs change for everyone.",
        "Inside" => "Indoor mix presence. Synced to the table.",
        "Outside" => "Outdoor mix presence. Synced to the table.",
        "Player" => "You are a player at this table. Local role only.",
        "GM" => "You are the gamemaster. Armory and map fog are yours. Local role only.",
        "Datashard" | "Faces" | "Corps" | "Gangs" | "Lore" | "Bestiary" | "NPCs" | "Gods" => {
            "Open or close this catalog on the table. Press again to close."
        }
        "Fit" => "Reset map pan and zoom.",
        "Grid" => "Toggle the map grid.",
        "Import picture" => "Load a map image. It is sent to every seat at the table.",
        "Ink" => "Freehand fog. Drag on the map. Press again to put the pen down.",
        "Circle" => "Filled circle fog. Drag from center. Press again to put the pen down.",
        "Square" => "Filled square fog. Drag from center. Press again to put the pen down.",
        "Erase" => "Click a drawing to delete it. Press again to stop erasing.",
        "Clear drawings" => "Remove every fog mark on this map for the whole table.",
        "+ New" => "Create a blank character on this deck.",
        "Save" => "Write this sheet to disk and push it to the table.",
        "Delete" => "Remove this character from your deck.",
        "Short rest" | "Long rest" => "Recover hit points and resources. Logged for the table.",
        "Death save" => "Roll a death saving throw. Logged for the table.",
        "d%" => "Roll percentile against the current target.",
        "Norm" => "Straight d% rolls.",
        "Adv" => "Roll twice, keep the higher.",
        "Dis" => "Roll twice, keep the lower.",
        "Aim" => "Target this character for rolls.",
        "Target this sheet" => "Aim table rolls at this character.",
        "Upload close-up" => "Choose a portrait image stored on this deck.",
        "Upload full body" => "Choose a full-body image stored on this deck.",
        "AUTO WALK" | "AUTO ON" => "Cruise the city along generated streets. C also toggles.",
        "← INDEX" => "Back to INDEX.",
        "TABLE" => "Open the mix table.",
        "+ New nethook" => "Create a site. Only you can edit it.",
        "Edit" => "Edit this nethook. Only the author can.",
        "Host table" | "Host on local network" => {
            "Start hosting on this LAN. Friends Join with blightnet://."
        }
        "Host on the internet" => "Your node punches UDP to friends' nodes and builds an encrypted invite. No Cloudflare.",
        "Connect" => "Join the address in the box.",
        "Fade out" => "Silence the table mix. Fade in brings the last mix back.",
        "Fade in" => "Restore the mix you faded. Everyone at the table hears it.",
        "Silence" => "Stop every mix layer at this table.",
        "Day" => "Set table time to day. Mix presence and the painting follow.",
        "Dusk" => "Set table time to dusk. Mix presence and the painting follow.",
        "Evening" => "Set table time to evening. Mix presence and the painting follow.",
        "Night" => "Set table time to night. Mix presence and the painting follow.",
        "−" => "Step the table clock back 15 minutes.",
        "+" => "Step the table clock forward 15 minutes.",
        "Shuffle" => "Pick a random mix scene for this world.",
        "Shuffle stall" => "Reroll vendor stock for this buyer.",
        "Deal" => "Deal a new House 21 hand.",
        "Hit" => "Take another card.",
        "Stand" => "Keep this hand. Dealer plays.",
        "Buy" => "Buy this item with this character's money.",
        "Give" => "Give this item to the open character.",
        "Level up" => "Open the level-up spend for this sheet.",
        "Unequip" => "Take this item off.",
        "Equip" => "Put this item on.",
        "Drop" => "Remove this item from the sheet.",
        "Cancel" => "Abort this action.",
        "Confirm spend" => "Spend improvement points on the queued rank.",
        "Confirm level" => "Lock in this level-up.",
        "+ Attack" => "Add a blank attack row.",
        "Shuffle first" => "Roll a new first name.",
        "Shuffle last" => "Roll a new last name.",
        "Shuffle both" => "Roll a new full name.",
        "Female" => "Set this character female and reshuffle names.",
        "Male" => "Set this character male and reshuffle names.",
        "Hang up" => "End the current call.",
        "Accept" => "Pick up this incoming call.",
        "Decline" => "Refuse this incoming call.",
        "Mute" => "Stop sending your microphone.",
        "Muted" => "Unmute your microphone.",
        "Camera" => "Toggle sending this camera.",
        "Share screen" => "Toggle sending this screen.",
        "Rescan cameras" => "Look for cameras again.",
        "Audio page" => "Open the AUDIO systems page.",
        "Open Voice" => "Open the voice rail.",
        "Message" => "Open a DM with this person.",
        "Call" => "Start a voice call.",
        "Join table" => "Join the table address saved on this contact.",
        "Remove" => "Delete this contact from this deck.",
        "Create crew" => "Make a named crew from selected contacts.",
        "Open chat" => "Talk to this crew.",
        "Crew video" => "Start a video call with this crew.",
        "Disband" => "Delete this crew from this deck.",
        "Add" => "Save this person to contacts.",
        "Add files" => "Add audio files to the local player library.",
        "Add folder" => "Add a folder of audio to the local player library.",
        "Clear" => "Empty this list.",
        "Download" => "Keep a copy of this file on this deck.",
        "Open video" => "Play this video in the system player.",
        "Table" => "Send chat to everyone at this table.",
        "YOU" => "Open your character sheet. Press again to close.",
        "Off air" => "Turn the Night City radio off.",
        "×" => "Close Blightnet.",
        "□" => "Maximize this window.",
        "❐" => "Restore this window.",
        _ => "",
    };
    if !known.is_empty() {
        return Cow::Borrowed(known);
    }
    if t.starts_with("Stop ") {
        return Cow::Borrowed("Stop recording and send the voice note.");
    }
    if t.starts_with("crew:") {
        return Cow::Borrowed("Send chat to this crew.");
    }
    if t.starts_with("★ ") {
        return Cow::Borrowed("Load this saved mix onto the table.");
    }
    Cow::Owned(format!(
        "{t} — press to use. If this is a panel, press again to close."
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn common_buttons_have_tooltips() {
        assert!(!super::hint_for("Host").is_empty());
        assert!(!super::hint_for("Characters").is_empty());
        assert!(!super::hint_for("Log").is_empty());
        assert!(!super::hint_for("Notes").is_empty());
        assert!(!super::hint_for("×").is_empty());
        assert!(super::hint_for("Ada").contains("Ada"));
        assert!(super::hint_for("Stop 3s").contains("voice note"));
    }
}

pub fn sys_tile(ui: &mut Ui, id: &str, title: &str, sub: &str, go: &str, kill: bool) -> egui::Response {
    let size = Vec2::new(ui.available_width().max(120.0), 88.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let color = if kill { KILL } else { ORANGE };
    let hover = resp.hovered();
    let fill = if hover { color } else { fade(color, 12) };
    fill_chamfer(ui, rect, 10.0, fill, Stroke::new(1.4, color));
    hairline_top(ui, rect.shrink(2.0), fade(CREAM, if hover { 40 } else { 22 }));
    if hover {
        brackets(ui, rect.shrink(6.0), fade(CYAN, 200), 8.0);
    } else {
        brackets(ui, rect.shrink(8.0), fade(CYAN, 70), 7.0);
    }
    let fg = if hover { INK } else { color };
    let muted = if hover { Color32::from_rgb(40, 40, 40) } else { MUTED };
    let id_c = if hover { INK } else { CYAN };
    let clip = rect.shrink2(Vec2::new(12.0, 8.0));
    let p = ui.painter().with_clip_rect(clip);
    p.text(clip.left_top(), egui::Align2::LEFT_TOP, id, FontId::new(13.0, mono()), id_c);
    p.text(clip.left_top() + Vec2::new(0.0, 18.0), egui::Align2::LEFT_TOP, title, FontId::new(16.0, display()), fg);
    p.text(clip.left_top() + Vec2::new(0.0, 42.0), egui::Align2::LEFT_TOP, sub, FontId::new(11.0, mono()), muted);
    let go_r = Rect::from_min_size(
        clip.right_bottom() - Vec2::new(56.0, 18.0),
        Vec2::new(56.0, 18.0),
    );
    fill_chamfer(
        ui,
        go_r,
        4.0,
        if hover { INK } else { fade(color, 40) },
        Stroke::new(1.0, if hover { INK } else { color }),
    );
    ui.painter().text(
        go_r.center(),
        egui::Align2::CENTER_CENTER,
        go,
        FontId::new(11.0, ui_font()),
        if hover { color } else { fg },
    );
    attach_tip(resp, format!("{title} — {sub}"))
}

pub fn jack_tile(ui: &mut Ui, t: f32) -> egui::Response {
    let size = Vec2::new(ui.available_width().max(180.0), 216.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let fill = if hover { ORANGE } else { PANEL };
    fill_chamfer(ui, rect, 22.0, fill, Stroke::new(2.0, ORANGE));
    ui.painter().rect_stroke(
        rect.shrink(4.0),
        0.0,
        Stroke::new(1.0, fade(CYAN, if hover { 40 } else { 80 })),
        StrokeKind::Inside,
    );
    if !hover {
        hatch(ui, rect.shrink(6.0), fade(ORANGE, 14));
    }
    brackets(ui, rect.shrink(10.0), if hover { INK } else { CYAN }, 16.0);
    let c = rect.center();
    let p = ui.painter().with_clip_rect(rect.shrink(4.0));
    for i in 0..3 {
        let r = 48.0 + i as f32 * 18.0 + (t * 12.0 + i as f32).sin() * 3.0;
        p.rect_stroke(
            Rect::from_center_size(c, Vec2::splat(r * 2.0)),
            CornerRadius::ZERO,
            Stroke::new(
                1.0,
                if hover {
                    Color32::from_black_alpha(80)
                } else {
                    fade(CYAN, 70)
                },
            ),
            StrokeKind::Inside,
        );
    }
    let scan_y = rect.top() + (t * 80.0 % rect.height());
    p.hline(rect.x_range(), scan_y, Stroke::new(14.0, fade(ORANGE, 18)));
    let id_c = if hover { Color32::from_rgb(40, 40, 40) } else { CYAN };
    let fg = if hover { INK } else { ORANGE };
    p.text(c + Vec2::new(0.0, -52.0), egui::Align2::CENTER_CENTER, "01", FontId::new(16.0, mono()), id_c);
    p.text(c + Vec2::new(0.0, -10.0), egui::Align2::CENTER_CENTER, "BLIGHTNEXUS", FontId::new(24.0, display()), fg);
    p.text(
        c + Vec2::new(0.0, 22.0),
        egui::Align2::CENTER_CENTER,
        "AMBIENCE // MIXER",
        FontId::new(11.0, mono()),
        if hover { Color32::from_rgb(50, 50, 50) } else { MUTED },
    );
    let go = Rect::from_center_size(c + Vec2::new(0.0, 60.0), Vec2::new(118.0, 28.0));
    fill_chamfer(ui, go, 5.0, if hover { INK } else { ORANGE }, Stroke::new(1.0, if hover { INK } else { ORANGE }));
    p.text(go.center(), egui::Align2::CENTER_CENTER, "JACK IN", FontId::new(13.0, ui_font()), if hover { ORANGE } else { INK });
    attach_tip(
        resp,
        "Open the TABLE mix. Scenes, catalogs, map, and sheets.",
    )
}
