//! One board. Acid and cyan squares. No engine.

use crate::theme::{self, CREAM, CYAN, DIM};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Vec2};

#[derive(Clone)]
pub struct Game {
    pub sq: [u8; 64],
    pub white_turn: bool,
    pub sel: Option<u8>,
    pub msg: String,
    pub white: String,
    pub black: String,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let mut sq = [0u8; 64];
        let back = [4u8, 2, 3, 5, 6, 3, 2, 4];
        for f in 0..8 {
            sq[f] = back[f];
            sq[8 + f] = 1;
            sq[48 + f] = 1 + 8;
            sq[56 + f] = back[f] + 8;
        }
        Self {
            sq,
            white_turn: true,
            sel: None,
            msg: "White moves.".into(),
            white: "White".into(),
            black: "Black".into(),
        }
    }

    pub fn pack(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.white_turn as u8,
            self.white,
            self.black,
            self.sq.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
        )
    }

    pub fn unpack(s: &str) -> Option<Self> {
        let mut p = s.split('|');
        let turn = p.next()? == "1";
        let white = p.next()?.to_string();
        let black = p.next()?.to_string();
        let mut sq = [0u8; 64];
        for (i, n) in p.next()?.split(',').enumerate().take(64) {
            sq[i] = n.parse().unwrap_or(0);
        }
        Some(Self {
            sq,
            white_turn: turn,
            sel: None,
            msg: if turn { "White moves.".into() } else { "Black moves.".into() },
            white,
            black,
        })
    }

    pub fn click(&mut self, i: u8) {
        let piece = self.sq[i as usize];
        let white = piece > 0 && piece < 8;
        if let Some(from) = self.sel {
            if legal(self, from, i) {
                self.sq[i as usize] = self.sq[from as usize];
                self.sq[from as usize] = 0;
                self.white_turn = !self.white_turn;
                self.msg = if self.white_turn {
                    "White moves.".into()
                } else {
                    "Black moves.".into()
                };
            } else if (white && self.white_turn) || (!white && piece >= 8 && !self.white_turn) {
                self.sel = Some(i);
                return;
            }
            self.sel = None;
        } else if piece != 0 && white == self.white_turn {
            self.sel = Some(i);
        }
    }
}

fn legal(g: &Game, from: u8, to: u8) -> bool {
    if from == to {
        return false;
    }
    let p = g.sq[from as usize];
    if p == 0 {
        return false;
    }
    let mine_white = p < 8;
    if mine_white != g.white_turn {
        return false;
    }
    let dest = g.sq[to as usize];
    if dest != 0 && (dest < 8) == mine_white {
        return false;
    }
    let (fr, ff) = (from / 8, from % 8);
    let (tr, tf) = (to / 8, to % 8);
    let dr = tr as i32 - fr as i32;
    let df = tf as i32 - ff as i32;
    let kind = if p < 8 { p } else { p - 8 };
    match kind {
        1 => {
            let dir = if mine_white { 1 } else { -1 };
            let start = if mine_white { 1 } else { 6 };
            if df == 0 && dest == 0 && (dr == dir || (fr == start && dr == dir * 2)) {
                return path_clear(g, from, to);
            }
            df.abs() == 1 && dr == dir && dest != 0
        }
        2 => (dr.abs(), df.abs()) == (2, 1) || (dr.abs(), df.abs()) == (1, 2),
        3 => dr.abs() == df.abs() && path_clear(g, from, to),
        4 => (dr == 0 || df == 0) && path_clear(g, from, to),
        5 => ((dr == 0 || df == 0) || dr.abs() == df.abs()) && path_clear(g, from, to),
        6 => dr.abs() <= 1 && df.abs() <= 1,
        _ => false,
    }
}

fn path_clear(g: &Game, from: u8, to: u8) -> bool {
    let (fr, ff) = (from as i32 / 8, from as i32 % 8);
    let (tr, tf) = (to as i32 / 8, to as i32 % 8);
    let sr = (tr - fr).signum();
    let sf = (tf - ff).signum();
    let mut r = fr + sr;
    let mut f = ff + sf;
    while r != tr || f != tf {
        if g.sq[(r * 8 + f) as usize] != 0 {
            return false;
        }
        r += sr;
        f += sf;
    }
    true
}

fn glyph(p: u8) -> &'static str {
    match p {
        1 => "P",
        2 => "N",
        3 => "B",
        4 => "R",
        5 => "Q",
        6 => "K",
        9 => "p",
        10 => "n",
        11 => "b",
        12 => "r",
        13 => "q",
        14 => "k",
        _ => "",
    }
}

pub fn paint(ui: &mut egui::Ui, game: &mut Game, can_move: bool) -> bool {
    let mut changed = false;
    ui.label(
        RichText::new("CHESS")
            .family(theme::display())
            .size(22.0)
            .color(theme::ACID),
    );
    ui.label(
        RichText::new(format!("{}  vs  {}", game.white, game.black))
            .family(theme::mono())
            .size(12.0)
            .color(CYAN),
    );
    ui.label(RichText::new(&game.msg).color(CREAM).size(13.0));
    let side = ui.available_width().min(420.0);
    let origin = ui.cursor().min;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    let cell = side / 8.0;
    for r in 0..8 {
        for f in 0..8 {
            let i = ((7 - r) * 8 + f) as u8;
            let x = rect.left() + f as f32 * cell;
            let y = rect.top() + r as f32 * cell;
            let sq = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(cell - 1.0));
            let dark = (r + f) % 2 == 1;
            let fill = if game.sel == Some(i) {
                theme::ACID
            } else if dark {
                Color32::from_rgb(12, 28, 36)
            } else {
                Color32::from_rgb(18, 18, 16)
            };
            ui.painter().rect_filled(sq, 2.0, fill);
            let p = game.sq[i as usize];
            if p != 0 {
                let col = if p < 8 { theme::ACID } else { CYAN };
                ui.painter().text(
                    sq.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph(p),
                    egui::FontId::new(cell * 0.45, theme::display()),
                    if game.sel == Some(i) { Color32::from_rgb(17, 17, 17) } else { col },
                );
            }
            if can_move && ui.allocate_rect(sq, Sense::click()).clicked() {
                let before = game.pack();
                game.click(i);
                if game.pack() != before {
                    changed = true;
                }
            }
        }
    }
    let _ = origin;
    ui.horizontal(|ui| {
        if theme::neon_btn(ui, "New game").clicked() {
            let w = game.white.clone();
            let b = game.black.clone();
            *game = Game::new();
            game.white = w;
            game.black = b;
            changed = true;
        }
        if theme::neon_btn_color(ui, "Resign", theme::NEON_RED, false).clicked() {
            game.msg = if game.white_turn {
                "Black wins.".into()
            } else {
                "White wins.".into()
            };
            game.white_turn = !game.white_turn;
            changed = true;
        }
    });
    ui.label(RichText::new("Click a piece, then the square.").color(DIM).size(11.0));
    changed
}
