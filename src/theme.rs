use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};
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

pub fn plate(ui: &Ui, rect: Rect) {
    fill_chamfer(ui, rect, 10.0, PANEL, Stroke::new(1.0, ORANGE));
    brackets(ui, rect.shrink(8.0), CYAN, 12.0);
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
    let h = if sub.is_empty() { 34.0 } else { 50.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let hover = resp.hovered();
    let fill = if on {
        ORANGE
    } else if hover {
        Color32::from_rgba_unmultiplied(255, 106, 18, 36)
    } else {
        Color32::from_rgba_unmultiplied(255, 106, 18, 10)
    };
    fill_chamfer(ui, rect, 8.0, fill, Stroke::new(1.0, ORANGE));
    let fg = if on || hover { INK } else { ORANGE };
    let muted = if on || hover {
        Color32::from_rgb(40, 32, 16)
    } else {
        MUTED
    };
    let clip = rect.shrink2(Vec2::new(10.0, 4.0));
    let p = ui.painter().with_clip_rect(clip);
    p.text(
        clip.left_top() + Vec2::new(2.0, if sub.is_empty() { 4.0 } else { 2.0 }),
        egui::Align2::LEFT_TOP,
        label,
        FontId::new(15.0, ui_font()),
        fg,
    );
    if !sub.is_empty() {
        p.text(
            clip.left_bottom() + Vec2::new(2.0, -4.0),
            egui::Align2::LEFT_BOTTOM,
            sub,
            FontId::new(11.0, mono()),
            muted,
        );
    }
    resp
}

pub fn section_head(ui: &mut Ui, id: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(id)
                .family(display())
                .size(18.0)
                .color(ORANGE),
        );
        ui.label(
            egui::RichText::new(title)
                .family(display())
                .size(18.0)
                .color(ORANGE),
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
    p.circle_stroke(c, r, Stroke::new(1.6, pal.gold));
    p.circle_filled(c, r - 1.0, pal.panel);
    for i in 0..4 {
        let a = i as f32 * std::f32::consts::FRAC_PI_2;
        let inner = Vec2::new(a.sin() * (r - 5.0), -a.cos() * (r - 5.0));
        p.circle_filled(c + inner, 1.5, pal.gold);
    }
    let mins = (minutes % 1440) as f32;
    let h = mins / 60.0;
    let m = mins % 60.0;
    let hour_a = ((h % 12.0) + m / 60.0) / 12.0 * std::f32::consts::TAU;
    let min_a = m / 60.0 * std::f32::consts::TAU;
    p.line_segment(
        [
            c,
            c + Vec2::new(hour_a.sin() * 10.0, -hour_a.cos() * 10.0),
        ],
        Stroke::new(2.2, pal.gold),
    );
    p.line_segment(
        [c, c + Vec2::new(min_a.sin() * 14.0, -min_a.cos() * 14.0)],
        Stroke::new(1.4, pal.gold_soft),
    );
    p.circle_filled(c, 2.2, pal.gold);
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
    v.widgets.inactive.bg_fill = PANEL;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(40, 28, 8);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.active.bg_fill = ORANGE;
    v.widgets.active.fg_stroke = Stroke::new(1.0, INK);
    v.selection.bg_fill = ORANGE;
    v.selection.stroke = Stroke::new(1.0, INK);
    v.hyperlink_color = CYAN;
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
            Stroke::new(6.0, color),
        );
        x += 12.0;
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
    ui.label(
        egui::RichText::new(text)
            .color(ORANGE)
            .family(mono())
            .size(11.0),
    );
}

pub fn neon_btn(ui: &mut Ui, label: &str) -> egui::Response {
    neon_btn_color(ui, label, ORANGE, false)
}

pub fn neon_btn_color(ui: &mut Ui, label: &str, color: Color32, solid: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_uppercase(),
        FontId::new(13.0, ui_font()),
        if solid { INK } else { color },
    );
    let size = Vec2::new((galley.size().x + 20.0).max(34.0), 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let fill = if solid || hover { color } else { Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 12) };
    let fg = if solid || hover { INK } else { color };
    let thick = if solid { 2.0 } else { 1.0 };
    fill_chamfer(ui, rect, 6.0, fill, Stroke::new(thick, color));
    if hover {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            0.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90)),
            StrokeKind::Outside,
        );
    }
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        fg,
    );
    let _ = fg;
    resp
}

pub fn sys_tile(ui: &mut Ui, id: &str, title: &str, sub: &str, go: &str, kill: bool) -> egui::Response {
    let size = Vec2::new(ui.available_width().max(120.0), 82.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let color = if kill { KILL } else { ORANGE };
    let hover = resp.hovered();
    let fill = if hover { color } else { Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 10) };
    fill_chamfer(ui, rect, 10.0, fill, Stroke::new(1.0, color));
    let fg = if hover { INK } else { color };
    let muted = if hover { Color32::from_rgb(40, 40, 40) } else { MUTED };
    let clip = rect.shrink2(Vec2::new(10.0, 6.0));
    let p = ui.painter().with_clip_rect(clip);
    p.text(clip.left_top(), egui::Align2::LEFT_TOP, id, FontId::new(16.0, ui_font()), fg);
    p.text(clip.left_top() + Vec2::new(0.0, 20.0), egui::Align2::LEFT_TOP, title, FontId::new(15.0, ui_font()), fg);
    p.text(clip.left_top() + Vec2::new(0.0, 40.0), egui::Align2::LEFT_TOP, sub, FontId::new(11.0, mono()), muted);
    p.text(clip.right_bottom(), egui::Align2::RIGHT_BOTTOM, go, FontId::new(11.0, ui_font()), fg);
    resp
}

pub fn jack_tile(ui: &mut Ui, t: f32) -> egui::Response {
    let size = Vec2::new(ui.available_width().max(180.0), 210.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = resp.hovered();
    let fill = if hover { ORANGE } else { PANEL };
    fill_chamfer(ui, rect, 22.0, fill, Stroke::new(2.0, ORANGE));
    if !hover {
        hatch(ui, rect.shrink(4.0), Color32::from_rgba_unmultiplied(255, 106, 18, 12));
    }
    brackets(ui, rect.shrink(10.0), if hover { INK } else { CYAN }, 16.0);
    let c = rect.center();
    let p = ui.painter().with_clip_rect(rect.shrink(4.0));
    for i in 0..3 {
        let r = 48.0 + i as f32 * 18.0 + (t * 12.0 + i as f32).sin() * 3.0;
        p.rect_stroke(
            Rect::from_center_size(c, Vec2::splat(r * 2.0)),
            CornerRadius::ZERO,
            Stroke::new(1.0, if hover { Color32::from_black_alpha(80) } else { Color32::from_rgba_unmultiplied(77, 232, 255, 70) }),
            StrokeKind::Inside,
        );
    }
    let scan_y = rect.top() + (t * 80.0 % rect.height());
    p.hline(rect.x_range(), scan_y, Stroke::new(18.0, Color32::from_rgba_unmultiplied(255, 106, 18, 18)));
    let id_c = if hover { Color32::from_rgb(40, 40, 40) } else { CYAN };
    let fg = if hover { INK } else { ORANGE };
    p.text(c + Vec2::new(0.0, -48.0), egui::Align2::CENTER_CENTER, "01", FontId::new(18.0, mono()), id_c);
    p.text(c + Vec2::new(0.0, -8.0), egui::Align2::CENTER_CENTER, "BLIGHTNEXUS", FontId::new(22.0, display()), fg);
    p.text(c + Vec2::new(0.0, 24.0), egui::Align2::CENTER_CENTER, "AMBIENCE // MIXER", FontId::new(11.0, mono()), if hover { Color32::from_rgb(50, 50, 50) } else { MUTED });
    let go = Rect::from_center_size(c + Vec2::new(0.0, 58.0), Vec2::new(110.0, 26.0));
    fill_chamfer(ui, go, 4.0, if hover { INK } else { ORANGE }, Stroke::new(1.0, if hover { INK } else { ORANGE }));
    p.text(go.center(), egui::Align2::CENTER_CENTER, "JACK IN", FontId::new(13.0, ui_font()), if hover { ORANGE } else { INK });
    resp
}
