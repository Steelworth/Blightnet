use crate::theme::{self, BG, CREAM, CYAN, DIM, MUTED, ORANGE, PANEL, TITLE};
use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Key, PointerButton, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Vec2,
};
use std::sync::Arc;
use std::time::Instant;

const MAP: i32 = 128;
const LOT: i32 = 8;
const FOV: f32 = 0.90;

const SIGNS: &[&str] = &[
    "BLIGHTNET",
    "NIGHT CITY",
    "RIOT FM",
    "GLOOM WIRE",
    "DUSK CH",
    "WAREHOUSE",
    "AFTERLIFE",
    "NETWATCH",
    "ICE.GATE",
    "CHROME ROW",
    "DATAVAULT",
    "WATSON",
    "PACIFICA",
    "HEYWOOD",
    "SOMA",
    "INDEX",
    "KANG",
    "OMNI",
    "HEXNODE",
    "VOID.SYS",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Avenue,
    Street,
    Alley,
    Sidewalk,
    Plaza,
    Park,
    Market,
    Trench,
    Solid,
}

#[derive(Clone, Copy)]
struct Cell {
    kind: Kind,
    h: f32,
    ice: bool,
    facade: u8,
    sign: u8,
    var: u8,
}

#[derive(Clone, Copy)]
struct Sprite {
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    kind: u8,
}

pub struct Netspace {
    map: Vec<Cell>,
    pub x: f32,
    pub z: f32,
    yaw: f32,
    pitch: f32,
    bob: f32,
    sprites: Vec<Sprite>,
    last: Instant,
    look: &'static str,
    atlas: Option<Atlas>,
    zbuf: Vec<f32>,
}

impl Default for Netspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Netspace {
    pub fn new() -> Self {
        let map = build_city();
        let (x, z) = spawn(&map);
        let sprites = scatter(&map);
        Self {
            map,
            x,
            z,
            yaw: 0.42,
            pitch: 0.06,
            bob: 0.0,
            sprites,
            last: Instant::now(),
            look: "CORP PLAZA",
            atlas: None,
            zbuf: Vec::new(),
        }
    }

    fn at(&self, x: i32, z: i32) -> Cell {
        let x = x.rem_euclid(MAP);
        let z = z.rem_euclid(MAP);
        self.map[(z * MAP + x) as usize]
    }

    fn walkable(&self, x: f32, z: f32) -> bool {
        let r = 0.20;
        for ox in [-r, 0.0, r] {
            for oz in [-r, 0.0, r] {
                if self.at((x + ox).floor() as i32, (z + oz).floor() as i32).h > 0.2 {
                    return false;
                }
            }
        }
        true
    }

    fn try_move(&mut self, dx: f32, dz: f32) {
        let nx = (self.x + dx).rem_euclid(MAP as f32);
        let nz = (self.z + dz).rem_euclid(MAP as f32);
        if self.walkable(nx, self.z) {
            self.x = nx;
        }
        if self.walkable(self.x, nz) {
            self.z = nz;
        }
    }

    pub fn district(&self) -> &'static str {
        district(self.x, self.z)
    }
}

fn idx(x: i32, z: i32) -> usize {
    (z.rem_euclid(MAP) * MAP + x.rem_euclid(MAP)) as usize
}

fn hash2(x: i32, z: i32) -> u32 {
    let mut n = x.wrapping_mul(374761393).wrapping_add(z.wrapping_mul(668265263)) as u32;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

fn hash3(x: i32, z: i32, w: i32) -> u32 {
    hash2(x, z.wrapping_add(w.wrapping_mul(197)))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

fn fogged(c: Color32, dist: f32) -> Color32 {
    mix(c, BG, (dist * 0.032).clamp(0.0, 0.82))
}

fn build_city() -> Vec<Cell> {
    let empty = Cell {
        kind: Kind::Solid,
        h: 4.0,
        ice: false,
        facade: 1,
        sign: 0,
        var: 0,
    };
    let mut map = vec![empty; (MAP * MAP) as usize];
    for z in 0..MAP {
        for x in 0..MAP {
            let i = idx(x, z);
            let bx = x.rem_euclid(LOT);
            let bz = z.rem_euclid(LOT);
            let lx = x.div_euclid(LOT);
            let lz = z.div_euclid(LOT);
            let lot = hash2(lx, lz);
            let dc = {
                let dx = (x - MAP / 2) as f32;
                let dz = (z - MAP / 2) as f32;
                (dx * dx + dz * dz).sqrt()
            };
            let neon = lz.rem_euclid(12) == 5 || lz.rem_euclid(12) == 6;
            let downtown = dc < 18.0;

            let on_ave = bx == 0 || bz == 0;
            let on_side = bx == 4 || bz == 4;
            let on_walk = bx == 1 || bx == 7 || bz == 1 || bz == 7;

            if on_ave {
                map[i] = Cell {
                    kind: Kind::Avenue,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: (lot % 8) as u8,
                };
                continue;
            }
            if on_side {
                map[i] = Cell {
                    kind: Kind::Street,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: (lot % 5) as u8,
                };
                continue;
            }
            if on_walk {
                map[i] = Cell {
                    kind: Kind::Sidewalk,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: (hash2(x, z) % 6) as u8,
                };
                continue;
            }

            let lot_kind = lot % 13;
            let courtyard = lot_kind == 0 && bx >= 3 && bx <= 5 && bz >= 3 && bz <= 5;
            let park_lot = lot_kind == 1;
            let market_lot = lot_kind == 2;
            let alley = lot_kind == 3 && bx == 3;
            let l_cut = lot_kind == 4 && bx >= 5 && bz >= 5;
            let trench = lot_kind == 8 && (bx == 3 || bz == 3);
            let mast = (bx == 2 && bz == 2) && lot % 3 == 0;

            if courtyard {
                map[i] = Cell {
                    kind: Kind::Plaza,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: 1,
                };
                continue;
            }
            if park_lot {
                let tree = hash2(x, z) % 6 == 0;
                map[i] = Cell {
                    kind: if tree { Kind::Solid } else { Kind::Park },
                    h: if tree { 2.2 + (hash2(x, z) % 3) as f32 * 0.4 } else { 0.0 },
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: (hash2(x, z) % 7) as u8,
                };
                continue;
            }
            if market_lot {
                let stall = hash2(x, z) % 4 == 0;
                map[i] = Cell {
                    kind: if stall { Kind::Solid } else { Kind::Market },
                    h: if stall { 1.35 } else { 0.0 },
                    ice: false,
                    facade: 0,
                    sign: if stall { 1 + (hash2(x, z) % SIGNS.len() as u32) as u8 } else { 0 },
                    var: 2,
                };
                continue;
            }
            if trench {
                map[i] = Cell {
                    kind: Kind::Trench,
                    h: 0.0,
                    ice: true,
                    facade: 3,
                    sign: 0,
                    var: 8,
                };
                continue;
            }
            if alley {
                map[i] = Cell {
                    kind: Kind::Alley,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: 3,
                };
                continue;
            }
            if l_cut {
                map[i] = Cell {
                    kind: Kind::Alley,
                    h: 0.0,
                    ice: false,
                    facade: 0,
                    sign: 0,
                    var: 4,
                };
                continue;
            }

            let facade = if lot % 29 == 0 {
                7
            } else if lot % 23 == 0 {
                8
            } else if lot % 19 == 0 {
                9
            } else if lot_kind == 5 {
                3
            } else if lot_kind == 6 {
                2
            } else if neon || lot_kind == 10 {
                4
            } else if lot_kind == 7 {
                0
            } else if lot_kind == 11 {
                5
            } else if lot_kind == 12 {
                6
            } else {
                1
            };
            let mut h = match facade {
                0 | 9 => 2.4 + (lot % 4) as f32 * 0.45,
                2 => 10.0 + (lot % 14) as f32 * 0.85,
                3 | 7 => 8.0 + (lot % 10) as f32 * 0.7,
                4 => 6.5 + (lot % 8) as f32 * 0.7,
                5 => 1.8 + (lot % 3) as f32 * 0.3,
                6 => 2.0 + (lot % 4) as f32 * 0.4,
                8 => 5.0 + (lot % 4) as f32 * 0.5,
                _ => 3.8 + (lot % 6) as f32 * 0.55,
            };
            if lot_kind == 9 {
                h += 8.0;
            }
            if downtown {
                h += 2.8;
            }
            if neon {
                h += 3.4;
            }
            if mast {
                h += 2.6;
            }
            // slight per-cell roof jitter so lots aren't perfect boxes
            h += (hash2(x, z) % 5) as f32 * 0.08;
            let ice = facade == 3;
            let sign = if lot % 3 == 0 {
                1 + (lot % SIGNS.len() as u32) as u8
            } else if hash2(x, z) % 11 == 0 {
                1 + (hash2(x, z) % SIGNS.len() as u32) as u8
            } else {
                0
            };
            map[i] = Cell {
                kind: Kind::Solid,
                h,
                ice,
                facade,
                sign,
                var: (lot % 16) as u8,
            };
        }
    }
    // Corp plaza
    let c0 = MAP / 2 - 5;
    let c1 = MAP / 2 + 5;
    for z in c0..c1 {
        for x in c0..c1 {
            let i = idx(x, z);
            map[i].kind = Kind::Plaza;
            map[i].h = 0.0;
            map[i].ice = false;
            map[i].sign = 0;
        }
    }
    let f0 = MAP / 2 - 1;
    let f1 = MAP / 2 + 1;
    for z in f0..f1 {
        for x in f0..f1 {
            let i = idx(x, z);
            map[i] = Cell {
                kind: Kind::Solid,
                h: 1.55,
                ice: true,
                facade: 3,
                sign: 8,
                var: 9,
            };
        }
    }
    map
}

fn spawn(map: &[Cell]) -> (f32, f32) {
    let c = MAP / 2;
    for z in (c - 4)..(c + 4) {
        for x in (c - 4)..(c + 4) {
            if map[idx(x, z)].h < 0.2 {
                return (x as f32 + 0.5, z as f32 + 0.5);
            }
        }
    }
    (c as f32 + 0.5, c as f32 - 2.5)
}

fn scatter(map: &[Cell]) -> Vec<Sprite> {
    let mut out = Vec::new();
    for z in 0..MAP {
        for x in 0..MAP {
            let c = map[idx(x, z)];
            let n = hash2(x, z);
            if matches!(c.kind, Kind::Avenue | Kind::Street) && n % 23 == 0 && out.len() < 48 {
                let along_x = x.rem_euclid(LOT) == 0 || x.rem_euclid(LOT) == 4;
                let spd = 1.8 + (n % 6) as f32 * 0.4;
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: if along_x { 0.0 } else { spd },
                    vz: if along_x { spd } else { 0.0 },
                    kind: 0,
                });
            }
            if c.kind == Kind::Sidewalk && n % 17 == 0 && out.len() < 90 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 2,
                });
            }
            if matches!(c.kind, Kind::Plaza | Kind::Sidewalk | Kind::Market | Kind::Park)
                && n % 27 == 0
                && out.len() < 120
            {
                out.push(Sprite {
                    x: x as f32 + 0.35,
                    z: z as f32 + 0.55,
                    vx: ((n % 5) as f32 - 2.0) * 0.28,
                    vz: ((n % 7) as f32 - 3.0) * 0.22,
                    kind: 1,
                });
            }
            if matches!(c.kind, Kind::Avenue | Kind::Plaza) && n % 43 == 0 && out.len() < 110 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: ((n % 3) as f32 - 1.0) * 1.4,
                    vz: ((n % 5) as f32 - 2.0) * 1.1,
                    kind: 3,
                });
            }
            if c.kind == Kind::Market && n % 11 == 0 && out.len() < 130 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 4,
                });
            }
            if c.kind == Kind::Alley && n % 19 == 0 && out.len() < 150 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 5,
                });
            }
            if matches!(c.kind, Kind::Plaza | Kind::Avenue) && n % 47 == 0 && out.len() < 170 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 6,
                });
            }
            if c.kind == Kind::Avenue && x.rem_euclid(LOT) == 0 && n % 61 == 0 && out.len() < 185 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 6.2,
                    kind: 7,
                });
            }
            if c.kind == Kind::Avenue
                && x.rem_euclid(LOT) == 0
                && z.rem_euclid(LOT) == 0
                && out.len() < 200
            {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 8,
                });
            }
        }
    }
    out
}

fn district(x: f32, z: f32) -> &'static str {
    let x = x.rem_euclid(MAP as f32);
    let z = z.rem_euclid(MAP as f32);
    let c = MAP as f32 / 2.0;
    if (x - c).abs() < 7.0 && (z - c).abs() < 7.0 {
        "CITY CENTER"
    } else {
        let lx = (x as i32).div_euclid(LOT);
        let lz = (z as i32).div_euclid(LOT);
        match hash2(lx, lz) % 8 {
            0 => "CHROME ROW",
            1 => "WATSON",
            2 => "PACIFICA",
            3 => "HEYWOOD",
            4 => "SOMA GRID",
            5 => "COMBAT ZONE",
            6 => "DATA VAULTS",
            _ => "WESTBROOK",
        }
    }
}

struct Atlas {
    g: [Option<Arc<Galley>>; 128],
}

impl Atlas {
    fn new(painter: &egui::Painter, font: FontId) -> Self {
        const SET: &str =
            " .`'*+|:;/\\-=#_[](){}<>^~Iil!@H0O8█▓▒░,#\"$%&ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut g: [Option<Arc<Galley>>; 128] = std::array::from_fn(|_| None);
        for ch in SET.chars() {
            let u = ch as usize;
            if u < 128 {
                g[u] = Some(painter.layout_no_wrap(
                    ch.to_string(),
                    font.clone(),
                    Color32::PLACEHOLDER,
                ));
            }
        }
        Self { g }
    }

    fn get(&self, ch: char) -> Option<Arc<Galley>> {
        let u = ch as usize;
        if u < 128 {
            self.g[u].clone()
        } else {
            None
        }
    }
}

struct Hit {
    dist: f32,
    h: f32,
    ice: bool,
    facade: u8,
    sign: u8,
    side: u8,
    u: f32,
    var: u8,
    mx: i32,
    mz: i32,
}

fn march(ns: &Netspace, rdx: f32, rdz: f32) -> Hit {
    let mut mx = ns.x.floor() as i32;
    let mut mz = ns.z.floor() as i32;
    let dx = if rdx.abs() < 1e-5 { 1e30 } else { 1.0 / rdx.abs() };
    let dz = if rdz.abs() < 1e-5 { 1e30 } else { 1.0 / rdz.abs() };
    let step_x = if rdx < 0.0 { -1 } else { 1 };
    let step_z = if rdz < 0.0 { -1 } else { 1 };
    let mut sx = if rdx < 0.0 {
        (ns.x - mx as f32) * dx
    } else {
        (mx as f32 + 1.0 - ns.x) * dx
    };
    let mut sz = if rdz < 0.0 {
        (ns.z - mz as f32) * dz
    } else {
        (mz as f32 + 1.0 - ns.z) * dz
    };
    let mut dist;
    let mut side;
    let mut hit = Hit {
        dist: 48.0,
        h: 0.0,
        ice: false,
        facade: 0,
        sign: 0,
        side: 0,
        u: 0.0,
        var: 0,
        mx: 0,
        mz: 0,
    };
    for _ in 0..88 {
        if sx < sz {
            mx += step_x;
            dist = sx;
            sx += dx;
            side = 0;
        } else {
            mz += step_z;
            dist = sz;
            sz += dz;
            side = 1;
        }
        let c = ns.at(mx, mz);
        if c.h > 0.15 {
            let u = if side == 0 {
                ns.z + rdz * dist
            } else {
                ns.x + rdx * dist
            };
            hit = Hit {
                dist: dist.max(0.16),
                h: c.h,
                ice: c.ice,
                facade: c.facade,
                sign: c.sign,
                side,
                u: u.abs().fract(),
                var: c.var,
                mx,
                mz,
            };
            break;
        }
    }
    hit
}

fn wall_tex(hit: &Hit, v: f32, t: f32) -> (char, Color32) {
    let d = hit.dist * if hit.side == 1 { 1.15 } else { 1.0 };
    let floors = (hit.h * 1.25).clamp(2.0, 30.0) as i32;
    let floor_i = ((1.0 - v) * floors as f32).clamp(0.0, (floors - 1) as f32) as i32;
    let cols = 8 + (hit.var % 7) as i32;
    let col_i = (hit.u * cols as f32).floor() as i32;
    let n = hash3(hit.mx, hit.mz, floor_i.wrapping_mul(17).wrapping_add(col_i));
    let district = hash2(hit.mx.div_euclid(LOT), hit.mz.div_euclid(LOT)) % 8;
    let chrome = district == 0 || hit.facade == 4;
    let vault = district == 6 || hit.ice;
    let combat = district == 5;
    let orange = fogged(ORANGE, d);
    let cyan = fogged(CYAN, d);
    let cream = fogged(CREAM, d);
    let muted = fogged(MUTED, d);
    let dim = fogged(DIM, d);
    let body = if vault {
        cyan
    } else if chrome && n % 3 == 0 {
        cyan
    } else if combat {
        orange
    } else {
        orange
    };

    let door = v > 0.84 && (hit.u - 0.5).abs() < 0.12 && hit.facade != 2 && hit.h < 9.0;
    let awning = v > 0.74 && v < 0.84 && matches!(hit.facade, 0 | 4 | 9);
    let balcony = floor_i % 4 == 2 && v > 0.18 && v < 0.28 && hit.h > 5.0;
    let ledge = ((1.0 - v) * floors as f32).fract() < 0.08 && v > 0.1 && v < 0.9;
    let roof = v < 0.06;
    let mast = roof && (hit.var % 3 == 0) && col_i % 3 == 0;
    let dish = roof && hit.var % 4 == 1 && col_i % 5 == 2;
    let ticker = hit.sign > 0 && floor_i >= floors.saturating_sub(2) && v < 0.24 && v > 0.04;
    let neon_bar = (chrome || vault) && (floor_i == 1 || floor_i == 3 || floor_i == 5);
    let blink = ((t * (2.2 + (hit.var % 5) as f32)).floor() as i32 + hit.mx).rem_euclid(5) == 0;
    let fire_esc = hit.var % 3 == 0 && hit.u < 0.16 && v > 0.1 && v < 0.9;
    let pipe = hit.var % 5 == 2 && hit.u > 0.88;
    let ac = floor_i % 3 == 0 && col_i % 4 == 2 && v > 0.18 && v < 0.32;
    let graffiti = floor_i == 0 && n % 6 == 0 && v > 0.72;
    let floor_num = col_i == 0 && (v * 20.0) as i32 % 7 == 0 && v > 0.15 && v < 0.85;
    let win_pitch = 1 + (hit.var % 3) as i32;
    let window = !door
        && !balcony
        && v > 0.07
        && v < 0.86
        && col_i.rem_euclid(win_pitch + 1) == 1
        && floor_i.rem_euclid(2) == 1;

    if mast {
        return ('|', cyan);
    }
    if dish {
        return ('O', cream);
    }
    if roof {
        return (if n % 3 == 0 { '=' } else { '_' }, orange);
    }
    if door {
        let ch = if (hit.u - 0.5).abs() < 0.04 { '_' } else { '[' };
        return (ch, cream);
    }
    if awning {
        return ('~', if blink { cyan } else { orange });
    }
    if ticker {
        let s = SIGNS[(hit.sign as usize - 1) % SIGNS.len()];
        let bytes = s.as_bytes();
        let scroll = ((t * 4.0) as usize).wrapping_add((hit.u * 12.0) as usize);
        let k = scroll % bytes.len();
        let ch = bytes[k] as char;
        return (ch, if vault || blink { cyan } else { orange });
    }
    if neon_bar {
        return ('=', if blink { cream } else { cyan });
    }
    if balcony {
        return ('=', cream);
    }
    if ledge {
        return ('-', muted);
    }
    if fire_esc {
        return (if col_i == 0 { '#' } else { 'H' }, muted);
    }
    if pipe {
        return ('|', dim);
    }
    if ac {
        return ('o', muted);
    }
    if floor_num {
        let digit = b"0123456789"[(floor_i as usize) % 10] as char;
        return (digit, cyan);
    }
    if graffiti {
        let g = b"INDEX"[n as usize % 5] as char;
        return (g, mix(ORANGE, DIM, 0.45));
    }
    if window {
        let live = n % 8 > 1;
        let flicker = live && n % 13 == 0 && blink;
        let lit = live && !flicker;
        let inner = if n % 5 == 0 { '+' } else { '#' };
        if vault {
            return (if lit { inner } else { ':' }, if lit { cyan } else { dim });
        }
        if hit.facade == 2 {
            return (
                if lit { 'H' } else { '0' },
                if lit { cream } else { mix(ORANGE, BG, 0.55) },
            );
        }
        if chrome {
            return (if lit { '#' } else { '.' }, if lit { cyan } else { orange });
        }
        if hit.facade == 7 {
            return (if lit { '+' } else { ':' }, if lit { cyan } else { cream });
        }
        if hit.facade == 8 {
            return (if lit { '#' } else { '.' }, cream);
        }
        if hit.facade == 5 {
            return ('_', dim);
        }
        if hit.facade == 6 {
            return (if n % 2 == 0 { '.' } else { '`' }, muted);
        }
        return (
            if lit { inner } else { '.' },
            if lit { cream } else { mix(ORANGE, BG, 0.6) },
        );
    }
    let rain = hash3(hit.mx, col_i, (t * 9.0) as i32) % 11 == 0;
    if rain && v < 0.96 {
        return ('/', mix(CYAN, BG, 0.55));
    }
    let ch = if hit.dist < 2.2 {
        '█'
    } else if hit.dist < 4.6 {
        '▓'
    } else if hit.dist < 8.2 {
        '▒'
    } else if hit.dist < 13.5 {
        '░'
    } else if hit.dist < 22.0 {
        '·'
    } else {
        '`'
    };
    (ch, if n % 19 == 0 { muted } else { body })
}

fn lamp_lit(fx: f32, fz: f32, lamps: &[(f32, f32)]) -> bool {
    lamps.iter().any(|(lx, lz)| {
        let dx = fx - lx;
        let dz = fz - lz;
        dx * dx + dz * dz < 8.5
    })
}

fn floor_tex(
    ns: &Netspace,
    fx: f32,
    fz: f32,
    d: f32,
    t: f32,
    lamps: &[(f32, f32)],
    reflect: Option<(char, Color32)>,
) -> (char, Color32) {
    let ix = fx.floor() as i32;
    let iz = fz.floor() as i32;
    let c = ns.at(ix, iz);
    let gx = fx.rem_euclid(1.0);
    let gz = fz.rem_euclid(1.0);
    let n = hash3(ix, iz, (t as i32).wrapping_mul(3));
    let bx = ix.rem_euclid(LOT);
    let bz = iz.rem_euclid(LOT);
    let cross = (bx == 0 && bz == 0) || (bx == 4 && bz == 4);
    let (mut glyph, mut col) = match c.kind {
        Kind::Avenue => {
            let lane = (gx - 0.5).abs() < 0.04 || (gz - 0.5).abs() < 0.04;
            if cross && ((gx * 8.0) as i32 + (gz * 8.0) as i32) % 2 == 0 {
                ('#', fogged(CREAM, d))
            } else if lane {
                ('=', fogged(CREAM, d))
            } else if n % 17 == 0 {
                ('*', fogged(ORANGE, d * 1.2))
            } else {
                ('.', fogged(DIM, d))
            }
        }
        Kind::Street => {
            let dash = ((ix + iz) % 2 == 0)
                && ((gx - 0.5).abs() < 0.05 || (gz - 0.5).abs() < 0.05);
            if dash {
                (':', fogged(MUTED, d))
            } else {
                ('.', fogged(DIM, d))
            }
        }
        Kind::Alley => {
            let wet = n % 3 == 0;
            (
                if wet { '~' } else { '.' },
                fogged(if wet { CYAN } else { DIM }, d),
            )
        }
        Kind::Trench => {
            let flow = ((fx * 4.0 + t * 3.0) as i32).rem_euclid(3);
            (
                if flow == 0 { '~' } else { '=' },
                fogged(CYAN, d * 0.7),
            )
        }
        Kind::Sidewalk => {
            if n % 18 == 0 {
                ('#', fogged(CYAN, d))
            } else if gx < 0.07 || gz < 0.07 {
                ('+', fogged(MUTED, d))
            } else if n % 9 == 0 {
                (',', fogged(ORANGE, d * 1.3))
            } else {
                (':', fogged(MUTED, d))
            }
        }
        Kind::Plaza => {
            let tile = (ix + iz) % 2 == 0;
            let rose = (ix - MAP / 2).abs() <= 1 && (iz - MAP / 2).abs() <= 1;
            if rose {
                ('*', fogged(CYAN, d * 0.5))
            } else {
                (
                    if tile { '+' } else { '#' },
                    fogged(if tile { CYAN } else { ORANGE }, d),
                )
            }
        }
        Kind::Park => {
            let g = n % 3;
            (
                if g == 0 {
                    ','
                } else if g == 1 {
                    '"'
                } else {
                    '`'
                },
                fogged(MUTED, d),
            )
        }
        Kind::Market => {
            let stall = n % 4 == 0;
            (
                if stall { '*' } else { '+' },
                fogged(if stall { ORANGE } else { CREAM }, d),
            )
        }
        Kind::Solid => ('.', fogged(ORANGE, d * 1.4)),
    };
    if d < 10.5 && lamp_lit(fx, fz, lamps) {
        col = mix(col, CYAN, 0.28);
        if glyph == '.' {
            glyph = ':';
        }
    }
    let wet = matches!(
        c.kind,
        Kind::Avenue | Kind::Street | Kind::Alley | Kind::Trench | Kind::Plaza
    );
    if wet {
        if let Some((rg, rc)) = reflect {
            if n % 4 == 0 {
                glyph = rg;
                col = mix(col, rc, 0.48);
            } else if gx < 0.18 || gz < 0.18 {
                col = mix(col, rc, 0.22);
            }
        }
    }
    if c.kind == Kind::Sidewalk && n % 13 == 0 {
        let s = SIGNS[(hash2(ix / 8, iz / 8) as usize) % SIGNS.len()];
        let k = (ix.abs() as usize) % s.len();
        glyph = s.as_bytes()[k] as char;
        col = fogged(ORANGE, d);
    }
    (glyph, col)
}

fn ceiling_tex(fx: f32, fz: f32, d: f32, t: f32) -> (char, Color32) {
    let ix = fx.floor() as i32;
    let iz = fz.floor() as i32;
    let n = hash3(ix, iz, (t * 2.0) as i32);
    let gx = fx.rem_euclid(1.0);
    if n % 11 == 0 {
        ('=', fogged(ORANGE, d))
    } else if n % 7 == 0 || gx < 0.06 {
        ('-', fogged(CYAN, d))
    } else if n % 17 == 0 {
        ('+', fogged(MUTED, d))
    } else {
        ('`', fogged(DIM, d * 0.8))
    }
}

fn sky_tex(col: i32, row: i32, t: f32, near_horizon: bool) -> (char, Color32) {
    let n = hash3(col, row, (t * 5.0) as i32);
    if n % 7 == 0 {
        return ('|', mix(CYAN, BG, 0.62));
    }
    if near_horizon {
        let ch = match n % 7 {
            0 => '|',
            1 => 'H',
            2 => '!',
            3 => '=',
            4 => '#',
            _ => '`',
        };
        return (ch, mix(ORANGE, BG, 0.55));
    }
    if n % 37 == 0 {
        ('*', CYAN)
    } else if n % 23 == 0 {
        ('.', ORANGE)
    } else if row < 2 {
        ('`', DIM)
    } else if n % 47 == 0 {
        ('-', mix(MUTED, BG, 0.4))
    } else {
        (' ', BG)
    }
}

fn tick(ns: &mut Netspace, ui: &egui::Ui, focused: bool, dt: f32) {
    let dt = dt.clamp(0.0, 0.05);
    let typing = ui.ctx().wants_keyboard_input() && !focused;
    let mut mx = 0.0f32;
    let mut mz = 0.0f32;
    let mut yaw_d = 0.0f32;
    if !typing {
        ui.input(|i| {
            if i.key_down(Key::W) || i.key_down(Key::ArrowUp) {
                mx += 1.0;
            }
            if i.key_down(Key::S) || i.key_down(Key::ArrowDown) {
                mx -= 1.0;
            }
            if i.key_down(Key::A) {
                mz -= 1.0;
            }
            if i.key_down(Key::D) {
                mz += 1.0;
            }
            if i.key_down(Key::Q) || i.key_down(Key::ArrowLeft) {
                yaw_d -= 1.0;
            }
            if i.key_down(Key::E) || i.key_down(Key::ArrowRight) {
                yaw_d += 1.0;
            }
        });
    }
    let sprint = ui.input(|i| i.modifiers.shift);
    let speed = if sprint { 7.4 } else { 3.9 };
    ns.yaw += yaw_d * 1.7 * dt;
    let cy = ns.yaw.cos();
    let sy = ns.yaw.sin();
    let moving = mx.abs() + mz.abs() > 0.0;
    if moving {
        ns.try_move((sy * mx + cy * mz) * speed * dt, (cy * mx - sy * mz) * speed * dt);
        ns.bob += dt * 10.0;
    } else {
        ns.bob *= 0.9;
    }
    let m = MAP as f32;
    for i in 0..ns.sprites.len() {
        let k = ns.sprites[i].kind;
        if k == 2 || k == 4 || k == 5 || k == 6 || k == 8 {
            continue;
        }
        let mut x = (ns.sprites[i].x + ns.sprites[i].vx * dt).rem_euclid(m);
        let mut z = (ns.sprites[i].z + ns.sprites[i].vz * dt).rem_euclid(m);
        if k != 3 {
            let blocked = ns.at(x.floor() as i32, z.floor() as i32).h > 0.2;
            if blocked {
                ns.sprites[i].vx = -ns.sprites[i].vx;
                ns.sprites[i].vz = -ns.sprites[i].vz;
                x = (ns.sprites[i].x + ns.sprites[i].vx * dt * 2.0).rem_euclid(m);
                z = (ns.sprites[i].z + ns.sprites[i].vz * dt * 2.0).rem_euclid(m);
            }
        }
        if k == 1 && hash2(x as i32, (ns.bob * 3.0) as i32) % 80 == 0 {
            let vx = ns.sprites[i].vx;
            ns.sprites[i].vx = -ns.sprites[i].vz * 0.8;
            ns.sprites[i].vz = vx * 0.5;
        }
        ns.sprites[i].x = x;
        ns.sprites[i].z = z;
    }
}

pub fn paint(ui: &mut egui::Ui, ns: &mut Netspace, t: f32) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    if resp.clicked() {
        resp.request_focus();
    }
    if resp.dragged_by(PointerButton::Primary) {
        let d = resp.drag_delta();
        ns.yaw += d.x * 0.007;
        ns.pitch = (ns.pitch - d.y * 0.005).clamp(-0.38, 0.48);
        resp.request_focus();
    }
    let now = Instant::now();
    let dt = now.saturating_duration_since(ns.last).as_secs_f32();
    ns.last = now;
    tick(ns, ui, resp.has_focus(), dt);

    ui.painter().rect_filled(rect, 0.0, BG);
    let pad = rect.shrink2(Vec2::new(4.0, 2.0));
    let radar_w = (pad.width() * 0.24).clamp(188.0, 280.0);
    let (view, radar) = pad.split_left_right_at_x(pad.right() - radar_w);
    let inner = view.shrink2(Vec2::new(2.0, 0.0));
    let cw = 6.0;
    let ch = 10.0;
    let cols = (inner.width() / cw).floor().clamp(36.0, 120.0) as i32;
    let rows = (inner.height() / ch).floor().clamp(18.0, 48.0) as i32;
    if cols < 10 || rows < 10 {
        draw_radar(ns, ui.painter(), radar.shrink(4.0));
        return;
    }
    let painter = ui.painter().with_clip_rect(inner);
    let font = FontId::new(10.0, theme::mono());
    if ns.atlas.is_none() {
        ns.atlas = Some(Atlas::new(&painter, font));
    }
    let atlas = ns.atlas.as_ref().unwrap();
    let cam_y = 1.44 + ns.bob.sin() * 0.05;
    let horizon = rows as f32 * (0.38 + ns.pitch * 0.42);
    let lamps: Vec<(f32, f32)> = ns
        .sprites
        .iter()
        .filter(|s| s.kind == 2)
        .map(|s| (s.x, s.z))
        .collect();
    ns.zbuf.clear();
    ns.zbuf.resize(cols.max(1) as usize, 48.0);
    let mut center_sign: u8 = 0;
    let mut center_kind = Kind::Avenue;
    let mut center_facade: u8 = 1;
    let mut center_wall = false;

    for col in 0..cols {
        let u = (col as f32 + 0.5) / cols as f32 - 0.5;
        let ang = ns.yaw + u * FOV;
        let rdx = ang.sin();
        let rdz = ang.cos();
        let mut hit = march(ns, rdx, rdz);
        hit.dist *= (u * FOV).cos().max(0.32);
        if col == cols / 2 {
            center_sign = hit.sign;
            center_facade = hit.facade;
            center_wall = hit.h > 0.15;
            center_kind = ns.at(ns.x.floor() as i32, ns.z.floor() as i32).kind;
        }
        ns.zbuf[col as usize] = hit.dist;
        let wall_h = (hit.h / hit.dist.max(0.28)) * rows as f32 * 0.62;
        let top = horizon - wall_h;
        let bot = horizon + (cam_y / hit.dist.max(0.28)) * rows as f32 * 0.22;
        let reflect = if hit.h > 0.15 {
            Some(wall_tex(&hit, 0.62, t))
        } else {
            None
        };
        for row in 0..rows {
            let rf = row as f32;
            let (glyph, color) = if hit.h > 0.15 && rf >= top && rf <= bot && hit.dist < 48.0 {
                let v = ((rf - top) / (bot - top).max(0.001)).clamp(0.0, 1.0);
                wall_tex(&hit, v, t)
            } else if rf > horizon {
                let p = ((rf - horizon) / (rows as f32 - horizon).max(1.0)).max(0.03);
                let d = cam_y / p;
                let fx = ns.x + rdx * d;
                let fz = ns.z + rdz * d;
                floor_tex(ns, fx, fz, d, t, &lamps, reflect)
            } else {
                let p = ((horizon - rf) / horizon.max(1.0)).max(0.04);
                let d = 2.4 / p;
                let fx = ns.x + rdx * d;
                let fz = ns.z + rdz * d;
                if d < 14.0 {
                    ceiling_tex(fx, fz, d, t)
                } else {
                    sky_tex(col, row, t, rf > horizon - 5.0 && hit.dist > 16.0)
                }
            };
            blit(&painter, &atlas, inner, col, row, cw, ch, glyph, color);
        }
    }

    draw_sprites(
        ns, &painter, &atlas, inner, cols, rows, horizon, cam_y, cw, ch, t,
    );

    ns.look = if center_sign > 0 {
        SIGNS[(center_sign as usize - 1) % SIGNS.len()]
    } else if center_wall {
        match center_facade {
            3 => "ICE TOWER",
            2 => "DATA SPIRE",
            4 => "NEON STACK",
            0 => "SHOPFRONT",
            5 => "PARK DECK",
            6 => "COMBAT RUIN",
            7 => "TRAUMA WARD",
            8 => "NCPD",
            9 => "NOMAD DINER",
            _ => "HAB BLOCK",
        }
    } else {
        match center_kind {
            Kind::Plaza => "CORP PLAZA",
            Kind::Park => "GRID PARK",
            Kind::Avenue => "AVENUE",
            Kind::Street => "SIDE STREET",
            Kind::Alley => "ALLEY",
            Kind::Sidewalk => "WALK",
            Kind::Market => "NIGHT MARKET",
            Kind::Trench => "DATA TRENCH",
            Kind::Solid => "ICE",
        }
    };

    draw_radar(ns, ui.painter(), radar.shrink(4.0));
}

fn blit(
    painter: &egui::Painter,
    atlas: &Atlas,
    inner: egui::Rect,
    col: i32,
    row: i32,
    cw: f32,
    ch: f32,
    glyph: char,
    color: Color32,
) {
    let Some(g) = atlas.get(glyph) else {
        return;
    };
    let pos = Pos2::new(inner.left() + col as f32 * cw, inner.top() + row as f32 * ch);
    painter.galley_with_override_text_color(pos, g, color);
}

fn draw_sprites(
    ns: &Netspace,
    painter: &egui::Painter,
    atlas: &Atlas,
    inner: egui::Rect,
    cols: i32,
    rows: i32,
    horizon: f32,
    cam_y: f32,
    cw: f32,
    ch: f32,
    t: f32,
) {
    let cy = ns.yaw.cos();
    let sy = ns.yaw.sin();
    let mut order: Vec<(f32, Sprite)> = ns
        .sprites
        .iter()
        .copied()
        .map(|s| {
            let dx = s.x - ns.x;
            let dz = s.z - ns.z;
            let depth = dx * sy + dz * cy;
            (depth, s)
        })
        .filter(|(d, _)| *d > 0.32 && *d < 26.0)
        .collect();
    order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (depth, s) in order {
        let dx = s.x - ns.x;
        let dz = s.z - ns.z;
        let rx = dx * cy - dz * sy;
        let sx = (rx / depth) / FOV;
        let col = ((0.5 + sx) * cols as f32).round() as i32;
        let body_h = match s.kind {
            0 => 1.05,
            3 => 0.7,
            4 => 1.3,
            _ => 1.55,
        };
        let spr_h = (body_h / depth) * rows as f32 * 0.5;
        let lift = if s.kind == 3 { 2.2 + (t * 2.0).sin() * 0.4 } else { 0.0 };
        let bot = horizon + ((cam_y - lift) / depth) * rows as f32 * 0.20;
        let top = bot - spr_h;
        let width = match s.kind {
            0 => (2.4 / depth).ceil() as i32,
            4 => (1.6 / depth).ceil() as i32,
            7 => (4.2 / depth).ceil() as i32,
            _ => 1,
        };
        for dc in -width..=width {
            let c = col + dc;
            if c < 0 || c >= cols {
                continue;
            }
            if (c as usize) < ns.zbuf.len() && depth >= ns.zbuf[c as usize] - 0.04 {
                continue;
            }
            let (glyph, color) = match s.kind {
                0 => {
                    let body = if dc == 0 { '=' } else { '#' };
                    (
                        if depth < 4.0 { body } else { 'H' },
                        fogged(ORANGE, depth),
                    )
                }
                1 => (
                    if depth < 5.0 { 'i' } else { '!' },
                    fogged(CREAM, depth),
                ),
                3 => ('*', fogged(CYAN, depth * 0.6)),
                4 => ('A', fogged(ORANGE, depth)),
                5 => ('#', fogged(DIM, depth)),
                6 => (
                    if ((t * 6.0) as i32) % 2 == 0 { '*' } else { '+' },
                    fogged(ORANGE, depth * 0.5),
                ),
                7 => ('=', fogged(CYAN, depth * 0.4)),
                8 => (
                    if ((t * 3.0) as i32) % 2 == 0 { 'O' } else { 'o' },
                    if ((t * 3.0) as i32) % 2 == 0 {
                        fogged(ORANGE, depth)
                    } else {
                        fogged(CYAN, depth)
                    },
                ),
                _ => ('I', fogged(CYAN, depth)),
            };
            let r0 = top.max(0.0) as i32;
            let r1 = bot.min(rows as f32 - 1.0) as i32;
            for row in r0..=r1 {
                let gch = if s.kind == 2 && row == r0 {
                    '*'
                } else if s.kind == 3 && row == r0 {
                    '+'
                } else {
                    glyph
                };
                blit(painter, atlas, inner, c, row, cw, ch, gch, color);
            }
        }
    }
}

fn draw_radar(ns: &Netspace, painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, TITLE);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(2.0, ORANGE),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(Vec2::new(8.0, 8.0));
    painter.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        "RADAR",
        FontId::new(13.0, theme::display()),
        ORANGE,
    );
    painter.text(
        inner.right_top(),
        Align2::RIGHT_TOP,
        ns.district(),
        FontId::new(11.0, theme::mono()),
        CYAN,
    );
    let grid = Rect::from_min_max(
        inner.left_top() + Vec2::new(0.0, 22.0),
        inner.right_bottom() - Vec2::new(0.0, 36.0),
    );
    painter.rect_filled(grid, 0.0, BG);
    painter.rect_stroke(grid, 0.0, Stroke::new(1.0, mix(ORANGE, BG, 0.45)), StrokeKind::Inside);
    let cells = 36i32;
    let cw = grid.width() / cells as f32;
    let ch = grid.height() / cells as f32;
    let px = ns.x.floor() as i32;
    let pz = ns.z.floor() as i32;
    for mz in 0..cells {
        for mx in 0..cells {
            let wx = px + mx - cells / 2;
            let wz = pz + mz - cells / 2;
            let c = ns.at(wx, wz);
            let col = if c.h > 0.2 {
                if c.ice {
                    mix(CYAN, BG, 0.15)
                } else if c.facade == 4 {
                    mix(ORANGE, CYAN, 0.25)
                } else {
                    ORANGE
                }
            } else {
                match c.kind {
                    Kind::Plaza => mix(CYAN, BG, 0.35),
                    Kind::Park => mix(MUTED, BG, 0.2),
                    Kind::Market => mix(ORANGE, BG, 0.35),
                    Kind::Trench => mix(CYAN, BG, 0.45),
                    Kind::Avenue => mix(DIM, BG, 0.15),
                    Kind::Street => mix(DIM, BG, 0.35),
                    Kind::Alley => mix(DIM, BG, 0.5),
                    _ => mix(PANEL, BG, 0.1),
                }
            };
            let p = Pos2::new(grid.left() + mx as f32 * cw, grid.top() + mz as f32 * ch);
            painter.rect_filled(Rect::from_min_size(p, Vec2::new(cw.max(1.0), ch.max(1.0))), 0.0, col);
        }
    }
    let cx = grid.center();
    let facing = Vec2::new(ns.yaw.sin(), -ns.yaw.cos());
    let tip = cx + facing * (cw.max(ch) * 4.5);
    painter.line_segment([cx, tip], Stroke::new(2.0, CYAN));
    painter.rect_filled(
        Rect::from_center_size(cx, Vec2::splat((cw.max(ch) * 1.6).max(4.0))),
        0.0,
        CYAN,
    );
    painter.text(
        inner.left_bottom() + Vec2::new(0.0, -18.0),
        Align2::LEFT_BOTTOM,
        format!("POS  {:05.1}  {:05.1}", ns.x, ns.z),
        FontId::new(11.0, theme::mono()),
        CREAM,
    );
    painter.text(
        inner.left_bottom(),
        Align2::LEFT_BOTTOM,
        format!("LOOK {}", ns.look),
        FontId::new(11.0, theme::mono()),
        CYAN,
    );
    painter.text(
        inner.right_bottom(),
        Align2::RIGHT_BOTTOM,
        "YOU",
        FontId::new(11.0, theme::mono()),
        ORANGE,
    );
}

pub fn hud(ui: &mut egui::Ui, ns: &Netspace, t: f32) {
    let ping = 8 + ((t * 1.4).sin().abs() * 10.0) as i32;
    ui.label(
        RichText::new(format!(
            "NETSPACE  ·  {}  ·  {:02.0},{:02.0}  ·  LOOK {}  ·  PING {ping}ms  ·  WASD  Q/E  drag  SHIFT",
            ns.district(),
            ns.x,
            ns.z,
            ns.look
        ))
        .family(theme::mono())
        .size(11.0)
        .color(CYAN),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_has_streets_buildings_and_a_spawn() {
        let ns = Netspace::new();
        let mut walk = 0;
        let mut walls = 0;
        let mut plaza = 0;
        let mut alley = 0;
        for z in 0..MAP {
            for x in 0..MAP {
                let c = ns.at(x, z);
                if c.h < 0.2 {
                    walk += 1;
                } else {
                    walls += 1;
                }
                if c.kind == Kind::Plaza {
                    plaza += 1;
                }
                if c.kind == Kind::Alley {
                    alley += 1;
                }
            }
        }
        assert!(walk > 800, "walkable {walk}");
        assert!(walls > 800, "walls {walls}");
        assert!(plaza >= 16, "plaza {plaza}");
        assert!(alley > 10, "alley {alley}");
        assert!(ns.at(ns.x.floor() as i32, ns.z.floor() as i32).h < 0.2);
    }

    #[test]
    fn collision_blocks_solid_cells() {
        let mut ns = Netspace::new();
        let c = MAP / 2;
        ns.x = c as f32 - 3.5;
        ns.z = c as f32 - 3.5;
        assert!(ns.walkable(ns.x, ns.z));
        let start_x = ns.x;
        for _ in 0..80 {
            ns.try_move(0.45, 0.0);
        }
        assert!(ns.walkable(ns.x, ns.z));
        assert!(ns.at(ns.x.floor() as i32, ns.z.floor() as i32).h < 0.2);
        assert!(ns.x != start_x);
    }

    #[test]
    fn wrap_and_procedural_variety() {
        let ns = Netspace::new();
        assert!(!ns.district().is_empty());
        let a = ns.at(0, 0);
        let b = ns.at(MAP, MAP);
        assert_eq!(a.h, b.h);
        let mut signs = 0;
        let mut facades = [0u32; 5];
        for z in 0..MAP {
            for x in 0..MAP {
                let c = ns.at(x, z);
                if c.sign > 0 {
                    signs += 1;
                }
                if c.h > 0.2 {
                    facades[c.facade.min(4) as usize] += 1;
                }
            }
        }
        assert!(signs > 40, "signs {signs}");
        assert!(facades.iter().filter(|n| **n > 0).count() >= 4);
        assert!(ns.sprites.len() > 20);
    }
}
