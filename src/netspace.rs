use crate::theme::{self, BG, CREAM, CYAN, DIM, INK, MUTED, ORANGE, PANEL, TITLE};
use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Key, Pos2, Rect, RichText, Sense,
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
    "ARASAKA",
    "MILITECH",
    "KANG TAO",
    "ZETATECH",
    "BIOTECHNICA",
    "TRAUMA TEAM",
    "NCPD",
    "AFTERLIFE",
    "LIZZIE'S",
    "TOTENTANZ",
    "CLINIC",
    "BAR",
    "RAIL",
    "SHRINE",
    "VAULT",
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
    Canal,
    Garden,
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
    pub cruise: bool,
    stuck: f32,
    cruise_tx: f32,
    cruise_tz: f32,
    cruise_dir: i32,
    people: Vec<(String, f32, f32, f32)>,
    indoors: bool,
    floor: i32,
    ret_x: f32,
    ret_z: f32,
    ret_yaw: f32,
    look_on: bool,
    aim_yaw: f32,
    aim_pitch: f32,
    map_on: bool,
    relay: bool,
    clock: f32,
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
            cruise: false,
            stuck: 0.0,
            cruise_tx: x,
            cruise_tz: z,
            cruise_dir: 2,
            people: Vec::new(),
            indoors: false,
            floor: 0,
            ret_x: x,
            ret_z: z,
            ret_yaw: 0.42,
            look_on: false,
            aim_yaw: 0.42,
            aim_pitch: 0.06,
            map_on: true,
            relay: false,
            clock: 0.0,
        }
    }

    pub fn street(&self) -> &'static str {
        street_name(self.x, self.z)
    }

    pub fn yaw_pub(&self) -> f32 {
        self.yaw
    }

    pub fn note_person(&mut self, name: String, x: f32, z: f32, yaw: f32) {
        if let Some(slot) = self.people.iter_mut().find(|p| p.0 == name) {
            *slot = (name, x, z, yaw);
        } else if self.people.len() < 24 {
            self.people.push((name, x, z, yaw));
        }
    }

    fn at(&self, x: i32, z: i32) -> Cell {
        if self.indoors {
            return room_cell(x, z, self.floor);
        }
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

const CRUISE_DIR: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

fn wrap_delta(a: f32, b: f32) -> f32 {
    let m = MAP as f32;
    let mut d = (a - b).rem_euclid(m);
    if d > m * 0.5 {
        d -= m;
    }
    d
}

fn ang_diff(want: f32, have: f32) -> f32 {
    let mut d = want - have;
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

fn room_cell(x: i32, z: i32, floor: i32) -> Cell {
    let wall = Cell {
        kind: Kind::Solid,
        h: 3.2,
        ice: false,
        facade: floor.clamp(0, 12) as u8,
        sign: 0,
        var: 1,
    };
    let open = Cell {
        kind: Kind::Plaza,
        h: 0.0,
        ice: false,
        facade: 0,
        sign: 0,
        var: 0,
    };
    if !(1..=14).contains(&x) || !(1..=14).contains(&z) {
        return wall;
    }
    if x == 1 || x == 14 || z == 1 || z == 14 {
        if z == 14 && (x == 7 || x == 8) {
            return open;
        }
        return wall;
    }
    open
}

fn dir_yaw(d: i32) -> f32 {
    match d.rem_euclid(4) {
        0 => std::f32::consts::PI,
        1 => std::f32::consts::FRAC_PI_2,
        2 => 0.0,
        _ => -std::f32::consts::FRAC_PI_2,
    }
}

fn cruise_ok(ns: &Netspace, x: i32, z: i32) -> bool {
    let c = ns.at(x, z);
    c.h < 0.2
        && matches!(
            c.kind,
            Kind::Avenue | Kind::Street | Kind::Plaza | Kind::Sidewalk | Kind::Alley | Kind::Canal
        )
}

fn cruise_ahead(ns: &Netspace, x: i32, z: i32, d: i32, steps: i32) -> bool {
    let (dx, dz) = CRUISE_DIR[d.rem_euclid(4) as usize];
    for s in 1..=steps {
        if !cruise_ok(ns, x + dx * s, z + dz * s) {
            return false;
        }
    }
    true
}

fn cruise_pick(ns: &mut Netspace, allow_back: bool) {
    let cx = ns.x.floor() as i32;
    let cz = ns.z.floor() as i32;
    let back = (ns.cruise_dir + 2).rem_euclid(4);
    let mut opts: Vec<i32> = (0..4)
        .filter(|&d| allow_back || d != back)
        .filter(|&d| cruise_ahead(ns, cx, cz, d, 2))
        .collect();
    if opts.is_empty() {
        opts = (0..4).filter(|&d| cruise_ahead(ns, cx, cz, d, 1)).collect();
    }
    if opts.is_empty() {
        ns.cruise_dir = (ns.cruise_dir + 1).rem_euclid(4);
        let (dx, dz) = CRUISE_DIR[ns.cruise_dir as usize];
        ns.cruise_tx = (cx + dx * 2) as f32 + 0.5;
        ns.cruise_tz = (cz + dz * 2) as f32 + 0.5;
        return;
    }
    let h = hash2(cx, cz.wrapping_add(ns.cruise_dir * 17));
    let choice = if opts.contains(&ns.cruise_dir) && h % 5 != 0 {
        ns.cruise_dir
    } else {
        opts[(h as usize) % opts.len()]
    };
    let steps = 4 + (h % 6) as i32;
    let (dx, dz) = CRUISE_DIR[choice as usize];
    ns.cruise_dir = choice;
    ns.cruise_tx = (cx + dx * steps) as f32 + 0.5;
    ns.cruise_tz = (cz + dz * steps) as f32 + 0.5;
}

fn cruise_step(ns: &mut Netspace, dt: f32) {
    let dx = wrap_delta(ns.cruise_tx, ns.x);
    let dz = wrap_delta(ns.cruise_tz, ns.z);
    let dist = dx.hypot(dz);
    if dist < 0.85 {
        cruise_pick(ns, false);
    }
    let want = dx.atan2(dz);
    let err = ang_diff(want, ns.yaw);
    ns.yaw += err.clamp(-1.85 * dt, 1.85 * dt);
    let sy = ns.yaw.sin();
    let cy = ns.yaw.cos();
    let ahead = ns.at((ns.x + dx.signum()).floor() as i32, (ns.z + dz.signum()).floor() as i32);
    let spd = if ahead.h > 16.0 {
        1.15
    } else if err.abs() < 0.45 {
        3.55
    } else {
        1.35
    };
    let before = (ns.x, ns.z);
    ns.try_move(sy * spd * dt, cy * spd * dt);
    let moved = wrap_delta(ns.x, before.0).abs() + wrap_delta(ns.z, before.1).abs();
    if moved < 0.004 {
        ns.stuck += dt;
    } else {
        ns.stuck *= 0.4;
    }
    if ns.stuck > 0.9 {
        cruise_pick(ns, true);
        ns.yaw = dir_yaw(ns.cruise_dir);
        ns.stuck = 0.0;
    }
    ns.bob += dt * 7.2;
    ns.aim_yaw = ns.yaw;
    ns.aim_pitch = ns.pitch;
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

fn coat_color(x: f32, z: f32) -> Color32 {
    const COATS: [Color32; 6] = [
        Color32::from_rgb(180, 40, 70),
        Color32::from_rgb(40, 90, 180),
        Color32::from_rgb(150, 90, 40),
        Color32::from_rgb(90, 40, 140),
        Color32::from_rgb(40, 130, 90),
        Color32::from_rgb(200, 170, 60),
    ];
    COATS[(hash2(x as i32, z as i32) as usize) % COATS.len()]
}

fn car_color(x: f32, z: f32) -> Color32 {
    const CARS: [Color32; 5] = [
        Color32::from_rgb(180, 30, 40),
        Color32::from_rgb(30, 70, 160),
        Color32::from_rgb(220, 220, 230),
        Color32::from_rgb(20, 20, 24),
        Color32::from_rgb(40, 140, 80),
    ];
    CARS[(hash2(x as i32, z as i32) as usize) % CARS.len()]
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

            let lot_kind = lot % 17;
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
            } else if lot_kind == 13 {
                10
            } else if lot_kind == 14 {
                11
            } else if lot_kind == 15 {
                12
            } else if lot % 17 == 0 {
                14
            } else if lot % 13 == 0 {
                15
            } else if lot % 11 == 0 {
                16
            } else if lot % 31 == 0 {
                17
            } else if lot % 37 == 0 {
                18
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
                10 => 1.6 + (lot % 3) as f32 * 0.25,
                11 => 3.2 + (lot % 5) as f32 * 0.4,
                12 => 2.6 + (lot % 4) as f32 * 0.35,
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
    // Canal with bridges
    for x in 0..MAP {
        let z = 24 + ((x as f32 * 0.22).sin() * 4.0).round() as i32;
        for dz in -1..=1 {
            let zz = z + dz;
            let i = idx(x, zz);
            if map[i].kind == Kind::Plaza {
                continue;
            }
            if x % 16 == 0 {
                map[i].kind = Kind::Street;
                map[i].h = 0.0;
                map[i].var = 12;
            } else {
                map[i].kind = Kind::Canal;
                map[i].h = 0.0;
                map[i].ice = false;
                map[i].facade = 0;
            }
        }
    }
    // North–south boulevard
    for z in 0..MAP {
        for dx in 0..2 {
            let i = idx(40 + dx, z);
            if map[i].kind != Kind::Plaza {
                map[i].kind = Kind::Avenue;
                map[i].h = 0.0;
            }
        }
    }
    // Gardens beside parks, extra planters on sidewalks
    for z in 0..MAP {
        for x in 0..MAP {
            let i = idx(x, z);
            if map[i].kind == Kind::Park && hash2(x, z) % 5 == 0 && map[i].h < 0.2 {
                map[i].kind = Kind::Garden;
            }
            if map[i].kind == Kind::Sidewalk {
                let n = map[idx(x + 1, z)].kind == Kind::Park
                    || map[idx(x - 1, z)].kind == Kind::Park
                    || map[idx(x, z + 1)].kind == Kind::Park
                    || map[idx(x, z - 1)].kind == Kind::Park;
                if n && hash2(x, z) % 3 == 0 {
                    map[i].kind = Kind::Garden;
                    map[i].h = 0.0;
                }
            }
            if map[i].kind == Kind::Park && hash2(x + 3, z) % 7 == 0 && map[i].h < 0.2 {
                map[i] = Cell {
                    kind: Kind::Solid,
                    h: 1.8 + (hash2(x, z) % 3) as f32 * 0.5,
                    ice: false,
                    facade: 13,
                    sign: 0,
                    var: 3,
                };
            }
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
    let mut booths = 0;
    let mut bikes = 0;
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
            if matches!(c.kind, Kind::Park | Kind::Garden) && n % 8 == 0 && out.len() < 280 {
                out.push(Sprite {
                    x: x as f32 + 0.4,
                    z: z as f32 + 0.6,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 9,
                });
            }
            if c.kind == Kind::Garden && n % 5 == 0 && out.len() < 320 {
                out.push(Sprite {
                    x: x as f32 + 0.55,
                    z: z as f32 + 0.4,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 10,
                });
            }
            if matches!(c.kind, Kind::Plaza | Kind::Park | Kind::Avenue) && n % 53 == 0 && out.len() < 340 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: ((n % 4) as f32 - 1.5) * 2.2,
                    vz: ((n % 6) as f32 - 2.5) * 1.6,
                    kind: 11,
                });
            }
            if c.kind == Kind::Sidewalk && n % 29 == 0 && out.len() < 360 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 12,
                });
            }
            if matches!(c.kind, Kind::Avenue | Kind::Street) && n % 37 == 0 && out.len() < 400 {
                out.push(Sprite {
                    x: x as f32 + 0.35,
                    z: z as f32 + 0.65,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 12,
                });
            }
            if c.kind == Kind::Sidewalk
                && x.rem_euclid(LOT) == 1
                && z.rem_euclid(LOT) == 1
                && booths < 8
                && n % 2 == 0
            {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: 0.0,
                    vz: 0.0,
                    kind: 14,
                });
                booths += 1;
            }
            if c.kind == Kind::Sidewalk && n % 19 == 0 && bikes < 18 {
                let spd = 1.1 + (n % 4) as f32 * 0.25;
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: if x % 2 == 0 { spd } else { 0.0 },
                    vz: if x % 2 == 0 { 0.0 } else { spd },
                    kind: 13,
                });
                bikes += 1;
            }
            if c.kind == Kind::Plaza && n % 9 == 0 && out.len() < 430 {
                out.push(Sprite {
                    x: x as f32 + 0.5,
                    z: z as f32 + 0.5,
                    vx: ((n % 3) as f32 - 1.0) * 0.4,
                    vz: ((n % 5) as f32 - 2.0) * 0.3,
                    kind: 1,
                });
            }
        }
    }
    out
}

fn street_name(x: f32, z: f32) -> &'static str {
    const EAST: &[&str] = &[
        "KANG", "WATSON", "SOMA", "HEYWOOD", "PACIFICA", "INDEX", "CHROME", "VOID",
    ];
    const NORTH: &[&str] = &[
        "ARASAKA", "MILITECH", "RAIL", "CLINIC", "AFTERLIFE", "VAULT", "NCPD", "BIO",
    ];
    let x = x.rem_euclid(MAP as f32) as i32;
    let z = z.rem_euclid(MAP as f32) as i32;
    if x.rem_euclid(LOT) <= 1 {
        NORTH[(z / LOT).rem_euclid(NORTH.len() as i32) as usize]
    } else {
        EAST[(x / LOT).rem_euclid(EAST.len() as i32) as usize]
    }
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
    px: f32,
}

impl Atlas {
    fn new(painter: &egui::Painter, font: FontId, px: f32) -> Self {
        const SET: &str =
            " .`'*+|:;/\\-=#_[](){}<>^~Iil!@H0O8█▓▒░,#\"$%&ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789Y^n";
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
        Self { g, px }
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
    march_from(ns, ns.x, ns.z, rdx, rdz, false)
}

fn march_from(ns: &Netspace, ox: f32, oz: f32, rdx: f32, rdz: f32, street: bool) -> Hit {
    let mut mx = ox.floor() as i32;
    let mut mz = oz.floor() as i32;
    let dx = if rdx.abs() < 1e-5 { 1e30 } else { 1.0 / rdx.abs() };
    let dz = if rdz.abs() < 1e-5 { 1e30 } else { 1.0 / rdz.abs() };
    let step_x = if rdx < 0.0 { -1 } else { 1 };
    let step_z = if rdz < 0.0 { -1 } else { 1 };
    let mut sx = if rdx < 0.0 {
        (ox - mx as f32) * dx
    } else {
        (mx as f32 + 1.0 - ox) * dx
    };
    let mut sz = if rdz < 0.0 {
        (oz - mz as f32) * dz
    } else {
        (mz as f32 + 1.0 - oz) * dz
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
        let c = if street || !ns.indoors {
            let x = mx.rem_euclid(MAP);
            let z = mz.rem_euclid(MAP);
            ns.map[(z * MAP + x) as usize]
        } else {
            room_cell(mx, mz, ns.floor)
        };
        if c.h > 0.15 {
            let u = if side == 0 {
                oz + rdz * dist
            } else {
                ox + rdx * dist
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

fn interior_lamp(floor: i32) -> Color32 {
    match floor.rem_euclid(6) {
        0 => Color32::from_rgb(255, 186, 120),
        1 => Color32::from_rgb(150, 186, 255),
        2 => Color32::from_rgb(255, 110, 168),
        3 => Color32::from_rgb(120, 210, 150),
        4 => Color32::from_rgb(255, 150, 64),
        _ => Color32::from_rgb(186, 140, 255),
    }
}

fn window_glow(n: u32) -> Color32 {
    match n % 6 {
        0 => Color32::from_rgb(255, 214, 140),
        1 => Color32::from_rgb(140, 196, 255),
        2 => Color32::from_rgb(255, 120, 170),
        3 => Color32::from_rgb(150, 235, 160),
        4 => Color32::from_rgb(255, 96, 72),
        _ => Color32::from_rgb(255, 236, 190),
    }
}

fn sign_ink(s: &str) -> Color32 {
    if s.contains("CLINIC") || s.contains("TRAUMA") {
        Color32::from_rgb(220, 40, 60)
    } else if s.contains("BAR") || s.contains("LIZZIE") || s.contains("SHRINE") || s.contains("TOTENTANZ") {
        Color32::from_rgb(220, 60, 180)
    } else if s.contains("VAULT") || s.contains("ICE") {
        CYAN
    } else if s.contains("PARK") || s.contains("BIO") {
        Color32::from_rgb(70, 180, 90)
    } else if s.contains("NCPD") {
        Color32::from_rgb(80, 140, 255)
    } else {
        Color32::from_rgb(255, 176, 60)
    }
}

fn district_body(district: u32) -> Color32 {
    match district % 8 {
        0 => Color32::from_rgb(58, 92, 128),
        1 => Color32::from_rgb(132, 72, 58),
        2 => Color32::from_rgb(78, 96, 64),
        3 => Color32::from_rgb(150, 98, 42),
        4 => Color32::from_rgb(92, 48, 112),
        5 => Color32::from_rgb(128, 42, 48),
        6 => Color32::from_rgb(36, 108, 118),
        _ => Color32::from_rgb(112, 108, 96),
    }
}

fn facade_body(district: u32, facade: u8) -> Color32 {
    match facade {
        0 => Color32::from_rgb(168, 122, 64),
        2 => Color32::from_rgb(70, 130, 168),
        3 => Color32::from_rgb(48, 140, 168),
        4 => Color32::from_rgb(150, 48, 120),
        5 => Color32::from_rgb(46, 120, 64),
        6 => Color32::from_rgb(110, 48, 44),
        7 => Color32::from_rgb(176, 40, 52),
        8 => Color32::from_rgb(48, 72, 140),
        9 => Color32::from_rgb(168, 96, 48),
        10 => Color32::from_rgb(196, 160, 48),
        11 => Color32::from_rgb(150, 64, 150),
        12 => Color32::from_rgb(72, 150, 110),
        13 => Color32::from_rgb(40, 110, 58),
        14 => Color32::from_rgb(190, 48, 48),
        15 => Color32::from_rgb(80, 160, 70),
        16 => Color32::from_rgb(160, 50, 40),
        17 => Color32::from_rgb(180, 150, 40),
        18 => Color32::from_rgb(200, 80, 40),
        _ => district_body(district),
    }
}

fn wall_tex(hit: &Hit, v: f32, t: f32) -> (char, Color32) {
    let _ = t;
    let d = hit.dist * if hit.side == 1 { 1.15 } else { 1.0 };
    let floors = (hit.h * 1.25).clamp(2.0, 30.0) as i32;
    let floor_i = ((1.0 - v) * floors as f32).clamp(0.0, (floors - 1) as f32) as i32;
    let cols = 8 + (hit.var % 7) as i32;
    let col_i = (hit.u * cols as f32).floor() as i32;
    let n = hash3(hit.mx, hit.mz, floor_i.wrapping_mul(17).wrapping_add(col_i));
    let district = hash2(hit.mx.div_euclid(LOT), hit.mz.div_euclid(LOT)) % 8;
    let chrome = district == 0 || hit.facade == 4;
    let vault = district == 6 || hit.ice;
    let paint_src = facade_body(district, hit.facade);
    let orange = fogged(paint_src, d);
    let cyan = fogged(CYAN, d);
    let red = fogged(crate::theme::NEON_RED, d);
    let acid = fogged(crate::theme::ACID, d);
    let cream = fogged(CREAM, d);
    let muted = fogged(MUTED, d);
    let dim = fogged(DIM, d);
    let body = if vault {
        cyan
    } else if chrome && n % 3 == 0 {
        fogged(mix(CYAN, paint_src, 0.45), d)
    } else {
        orange
    };
    if (hit.h - 3.2).abs() < 0.08 && hit.var == 1 && hit.facade <= 5 {
        let lamp = interior_lamp(hit.facade as i32);
        let plaster = fogged(mix(lamp, Color32::from_rgb(214, 204, 190), 0.7), d);
        let glow = fogged(lamp, d * 0.35);
        if v > 0.78 && (hit.u - 0.5).abs() < 0.16 {
            return ('[', glow);
        }
        if v > 0.22 && v < 0.58 && (hit.u * 6.0).fract() < 0.18 {
            return ('#', glow);
        }
        let ch = if hit.dist < 3.0 {
            '█'
        } else if hit.dist < 7.0 {
            '▓'
        } else {
            '░'
        };
        return (ch, plaster);
    }
    if hit.facade == 0 && hit.dist < 5.5 && v > 0.32 && v < 0.74 && (hit.u - 0.5).abs() < 0.3 {
        if v > 0.55 {
            return ('_', fogged(Color32::from_rgb(255, 176, 80), d * 0.45));
        }
        return ('|', fogged(Color32::from_rgb(48, 32, 22), d));
    }

    let door = v > 0.84 && (hit.u - 0.5).abs() < 0.12 && hit.facade != 2 && hit.h < 9.0;
    let awning = v > 0.74 && v < 0.84 && matches!(hit.facade, 0 | 4 | 9);
    let balcony = floor_i % 4 == 2 && v > 0.18 && v < 0.28 && hit.h > 5.0;
    let ledge = ((1.0 - v) * floors as f32).fract() < 0.08 && v > 0.1 && v < 0.9;
    let roof = v < 0.06;
    let mast = roof && (hit.var % 3 == 0) && col_i % 3 == 0;
    let dish = roof && hit.var % 4 == 1 && col_i % 5 == 2;
    let ticker = hit.sign > 0 && floor_i >= floors.saturating_sub(2) && v < 0.24 && v > 0.04;
    let neon_bar = (chrome || vault) && (floor_i == 1 || floor_i == 3 || floor_i == 5);

    let fire_esc = hit.var % 3 == 0 && hit.u < 0.16 && v > 0.1 && v < 0.9;
    let pipe = hit.var % 5 == 2 && hit.u > 0.88;
    let ac = floor_i % 3 == 0 && col_i % 4 == 2 && v > 0.18 && v < 0.32;
    let vent = floor_i % 5 == 4 && col_i % 3 == 1 && v > 0.42 && v < 0.52;
    let shutter = hit.facade == 0 && v > 0.62 && v < 0.72 && (hit.u - 0.5).abs() < 0.22;
    let cable = hit.facade != 2 && v > 0.08 && v < 0.14 && n % 4 == 0;
    let graffiti = floor_i == 0 && n % 6 == 0 && v > 0.72;
    let floor_num = col_i == 0 && (v * 20.0) as i32 % 7 == 0 && v > 0.15 && v < 0.85;
    let win_pitch = 1 + (hit.var % 3) as i32;
    let window = !door
        && !balcony
        && v > 0.07
        && v < 0.86
        && col_i.rem_euclid(win_pitch + 1) == 1
        && floor_i.rem_euclid(2) == 1;

    if hit.facade == 14 {
        return (if n % 3 == 0 { '+' } else { 'H' }, red);
    }
    if hit.facade == 15 {
        return (if v < 0.2 { '^' } else { '+' }, acid);
    }
    if hit.facade == 16 {
        return (if v > 0.8 { '[' } else { '=' }, red);
    }
    if hit.facade == 17 {
        return (if n % 2 == 0 { '=' } else { '-' }, acid);
    }
    if hit.facade == 18 {
        return ('X', mix(red, acid, 0.45));
    }
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
        return ('~', orange);
    }
    if ticker {
        let s = SIGNS[(hit.sign as usize - 1) % SIGNS.len()];
        let bytes = s.as_bytes();
        let k = ((hit.u * 14.0) as usize) % bytes.len();
        let ch = bytes[k] as char;
        return (ch, fogged(sign_ink(s), d));
    }
    if neon_bar {
        return ('=', cyan);
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
    if vent {
        return ('#', dim);
    }
    if shutter {
        return ('=', muted);
    }
    if cable {
        return ('~', dim);
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
        let lit = live;
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
            return (
                if lit { '#' } else { '.' },
                if lit { fogged(window_glow(n), d) } else { orange },
            );
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
        if hit.facade == 10 {
            return (if n % 2 == 0 { '_' } else { '.' }, cream);
        }
        if hit.facade == 11 {
            return (if lit { '*' } else { '+' }, if lit { orange } else { muted });
        }
        if hit.facade == 12 {
            return (if lit { '#' } else { ':' }, if lit { cyan } else { mix(MUTED, CYAN, 0.4) });
        }
        if hit.facade == 13 {
            return (if n % 2 == 0 { '"' } else { ',' }, muted);
        }
        return (
            if lit { inner } else { '.' },
            if lit {
                fogged(window_glow(n), d)
            } else {
                fogged(mix(paint_src, BG, 0.55), d)
            },
        );
    }
    let ivy = hit.facade == 13 && n % 3 == 0 && v > 0.2 && v < 0.9;
    if ivy {
        return (if n % 2 == 0 { '"' } else { ',' }, mix(MUTED, ORANGE, 0.35));
    }
    let banner = hit.var % 7 == 0 && v > 0.35 && v < 0.55 && (hit.u - 0.5).abs() < 0.28;
    if banner {
        return ('=', orange);
    }
    let rain = hash3(hit.mx, col_i, 0) % 14 == 0;
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
    let _ = t;
    let ix = fx.floor() as i32;
    let iz = fz.floor() as i32;
    let c = ns.at(ix, iz);
    if ns.indoors {
        let lamp = interior_lamp(ns.floor);
        let tile = (ix + iz) % 2 == 0;
        let shaft = (fx - 8.5).abs() < 0.8 && (fz - 8.5).abs() < 0.8;
        return if shaft {
            ('+', fogged(lamp, d * 0.4))
        } else {
            (
                if tile { '+' } else { '.' },
                fogged(mix(lamp, Color32::from_rgb(48, 42, 36), 0.62), d),
            )
        };
    }
    let gx = fx.rem_euclid(1.0);
    let gz = fz.rem_euclid(1.0);
    let n = hash3(ix, iz, 0);
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
            } else if n % 29 == 0 {
                ('H', fogged(ORANGE, d))
            } else if (gx - 0.25).abs() < 0.03 || (gx - 0.75).abs() < 0.03 {
                (':', fogged(MUTED, d))
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
            let flow = ((ix + iz) % 3).abs();
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
            } else if n % 14 == 0 {
                ('o', fogged(DIM, d))
            } else if n % 21 == 0 {
                ('~', fogged(CYAN, d * 1.1))
            } else if n % 11 == 0 {
                ('+', fogged(ORANGE, d * 1.4))
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
                fogged(Color32::from_rgb(46, 140, 72), d),
            )
        }
        Kind::Garden => {
            let g = n % 4;
            let ink = match g {
                0 => Color32::from_rgb(230, 80, 120),
                1 => Color32::from_rgb(46, 140, 72),
                2 => Color32::from_rgb(240, 200, 70),
                _ => Color32::from_rgb(230, 230, 220),
            };
            (
                if g == 0 {
                    '*'
                } else if g == 1 {
                    ','
                } else if g == 2 {
                    '"'
                } else {
                    '+'
                },
                fogged(ink, d),
            )
        }
        Kind::Canal => {
            let flow = ((ix * 2 + iz) % 3).abs();
            (
                if flow == 0 {
                    '~'
                } else if flow == 1 {
                    '='
                } else {
                    '-'
                },
                fogged(CYAN, d * 0.55),
            )
        }
        Kind::Market => {
            let stall = n % 4 == 0;
            let ink = match n % 5 {
                0 => Color32::from_rgb(190, 48, 60),
                1 => Color32::from_rgb(220, 170, 50),
                2 => Color32::from_rgb(40, 140, 130),
                3 => Color32::from_rgb(70, 110, 190),
                _ => Color32::from_rgb(210, 200, 180),
            };
            (
                if stall { '*' } else { '+' },
                fogged(ink, d),
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
        Kind::Avenue | Kind::Street | Kind::Alley | Kind::Trench | Kind::Plaza | Kind::Canal
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
        col = fogged(sign_ink(s), d);
    }
    (glyph, col)
}

fn ceiling_tex(fx: f32, fz: f32, d: f32, t: f32, lamp: Option<Color32>) -> (char, Color32) {
    let _ = t;
    if let Some(lamp) = lamp {
        let ix = fx.floor() as i32;
        let iz = fz.floor() as i32;
        let n = hash3(ix, iz, 3);
        let ch = if n % 5 == 0 { '=' } else { '-' };
        return (ch, fogged(mix(lamp, Color32::from_rgb(36, 32, 28), 0.55), d));
    }
    let ix = fx.floor() as i32;
    let iz = fz.floor() as i32;
    let n = hash3(ix, iz, 0);
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
    let _ = t;
    let n = hash3(col, row, 0);
    if n % 5 == 0 {
        return ('|', mix(CYAN, BG, 0.72));
    }
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
        return (ch, mix(Color32::from_rgb(40, 70, 120), Color32::from_rgb(6, 10, 28), 0.35));
    }
    if n % 37 == 0 {
        ('*', CYAN)
    } else if n % 23 == 0 {
        (
            '.',
            match n % 5 {
                0 => Color32::from_rgb(255, 220, 160),
                1 => Color32::from_rgb(160, 190, 255),
                2 => Color32::from_rgb(255, 160, 190),
                _ => Color32::from_rgb(190, 235, 200),
            },
        )
    } else if n % 61 == 0 {
        ('+', mix(CYAN, BG, 0.5))
    } else if row < 2 {
        ('`', DIM)
    } else if n % 47 == 0 {
        ('-', mix(MUTED, BG, 0.4))
    } else if n % 89 == 0 {
        ('o', mix(ORANGE, BG, 0.7))
    } else {
        (' ', Color32::from_rgb(6, 10, 28))
    }
}

fn near_booth(ns: &Netspace) -> bool {
    ns.sprites.iter().any(|s| {
        s.kind == 14 && wrap_delta(s.x, ns.x).hypot(wrap_delta(s.z, ns.z)) < 1.35
    })
}

fn skyline_tex(hit: &Hit, v: f32) -> (char, Color32) {
    let body = fogged(facade_body(hash2(hit.mx, hit.mz) % 8, hit.facade), hit.dist);
    let lit = v > 0.18 && v < 0.86 && (hit.u * 5.0).fract() < 0.16 && (v * 7.0).fract() < 0.22;
    if lit {
        ('#', fogged(window_glow((hit.mx as u32).wrapping_add(hit.mz as u32)), hit.dist))
    } else if hit.dist > 34.0 {
        ('.', body)
    } else {
        ('#', body)
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
            if i.key_pressed(Key::C) {
                ns.cruise = !ns.cruise;
                ns.stuck = 0.0;
                if ns.cruise {
                    cruise_pick(ns, true);
                    ns.yaw = dir_yaw(ns.cruise_dir);
                    ns.aim_yaw = ns.yaw;
                }
            }
            if i.key_pressed(Key::Escape) {
                ns.look_on = false;
                ns.relay = false;
            }
            if i.key_pressed(Key::M) {
                ns.map_on = !ns.map_on;
            }
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
            let at_booth = !ns.indoors && near_booth(ns);
            if i.key_pressed(Key::E) && at_booth {
                ns.relay = !ns.relay;
            } else if i.key_down(Key::E) || i.key_down(Key::ArrowRight) {
                yaw_d += 1.0;
            }
            if i.key_pressed(Key::F) {
                if ns.indoors {
                    if (ns.x - 8.5).abs() < 1.4 && (ns.z - 8.5).abs() < 1.4 {
                        ns.floor = (ns.floor + 1) % 6;
                    }
                } else {
                    let ax = ns.x + ns.yaw.sin();
                    let az = ns.z + ns.yaw.cos();
                    if ns.at(ax.floor() as i32, az.floor() as i32).h > 0.2 {
                        ns.ret_x = ns.x;
                        ns.ret_z = ns.z;
                        ns.ret_yaw = ns.yaw;
                        ns.indoors = true;
                        ns.floor = 0;
                        ns.x = 8.5;
                        ns.z = 12.2;
                        ns.yaw = std::f32::consts::PI;
                    }
                }
            }
        });
    }
    if ns.indoors && ns.z > 13.3 && (ns.x - 8.0).abs() < 1.6 {
        ns.x = ns.ret_x;
        ns.z = ns.ret_z;
        ns.indoors = false;
        ns.floor = 0;
    }
    if mx.abs() + mz.abs() + yaw_d.abs() > 0.0 {
        ns.cruise = false;
        ns.stuck = 0.0;
    }
    let sprint = ui.input(|i| i.modifiers.shift);
    let speed = if sprint { 7.4 } else { 3.9 };
    if yaw_d != 0.0 {
        ns.aim_yaw += yaw_d * 1.7 * dt;
        if !ns.look_on {
            ns.yaw = ns.aim_yaw;
        }
    }
    ns.clock += dt;
    let cy = ns.yaw.cos();
    let sy = ns.yaw.sin();
    let moving = mx.abs() + mz.abs() > 0.0;
    if moving {
        ns.try_move((sy * mx + cy * mz) * speed * dt, (cy * mx - sy * mz) * speed * dt);
        ns.bob += dt * 10.0;
    } else if ns.cruise {
        cruise_step(ns, dt);
    } else {
        ns.bob *= 0.9;
    }
    if ns.look_on {
        let k = 1.0 - (-12.0 * dt).exp();
        ns.yaw += ang_diff(ns.aim_yaw, ns.yaw) * k;
        ns.pitch += (ns.aim_pitch - ns.pitch) * k;
    } else {
        ns.aim_yaw = ns.yaw;
        ns.aim_pitch = ns.pitch;
    }
    let m = MAP as f32;
    for i in 0..ns.sprites.len() {
        let k = ns.sprites[i].kind;
        if k == 2 || k == 4 || k == 5 || k == 6 || k == 8 || k == 9 || k == 10 || k == 12 || k == 14 {
            continue;
        }
        let mut x = (ns.sprites[i].x + ns.sprites[i].vx * dt).rem_euclid(m);
        let mut z = (ns.sprites[i].z + ns.sprites[i].vz * dt).rem_euclid(m);
        if k == 0 || k == 13 {
            let near = wrap_delta(x, ns.x).hypot(wrap_delta(z, ns.z)) < if k == 0 { 2.2 } else { 1.4 };
            let ix = x.floor() as i32;
            let iz = z.floor() as i32;
            let crossing = ix.rem_euclid(LOT) <= 1 || iz.rem_euclid(LOT) <= 1;
            let phase = (ix.div_euclid(LOT) + iz.div_euclid(LOT)).rem_euclid(2);
            let red = ((ns.clock * 0.12) as i32 + phase) % 2 == 0;
            if near || (k == 0 && crossing && red) {
                x = ns.sprites[i].x;
                z = ns.sprites[i].z;
            }
        }
        if k == 1 {
            let dx = ns.x - x;
            let dz = ns.z - z;
            if dx.hypot(dz) < 1.3 {
                x -= dx.signum() * 0.08;
                z -= dz.signum() * 0.08;
            }
        }
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

pub fn paint(ui: &mut egui::Ui, ns: &mut Netspace, t: f32, full: bool) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    if resp.clicked() {
        ns.look_on = true;
        resp.request_focus();
    }
    if ns.look_on && resp.has_focus() {
        let d = ui.input(|i| i.pointer.delta());
        if d.length_sq() > 0.0 {
            ns.aim_yaw += d.x * 0.0045;
            ns.aim_pitch = (ns.aim_pitch - d.y * 0.0032).clamp(-0.45, 0.58);
        }
    }
    let now = Instant::now();
    let dt = now.saturating_duration_since(ns.last).as_secs_f32();
    ns.last = now;
    tick(ns, ui, resp.has_focus(), dt);

    ui.painter().rect_filled(rect, 0.0, BG);
    let pad = rect.shrink2(Vec2::new(if full { 0.0 } else { 4.0 }, if full { 0.0 } else { 2.0 }));
    let radar_w = if full {
        (pad.width() * 0.22).clamp(200.0, 280.0)
    } else {
        (pad.width() * 0.24).clamp(188.0, 280.0)
    };
    let (view, radar_split) = if full {
        (pad, Rect::from_min_size(Pos2::ZERO, Vec2::ZERO))
    } else {
        pad.split_left_right_at_x(pad.right() - radar_w)
    };
    let inner = if full {
        view
    } else {
        view.shrink2(Vec2::new(2.0, 0.0))
    };
    let mut cols = (inner.width() / if full { 8.0 } else { 10.0 }).floor() as i32;
    let mut rows = (inner.height() / if full { 11.0 } else { 14.0 }).floor() as i32;
    cols = cols.clamp(40, if full { 180 } else { 90 });
    rows = rows.clamp(20, if full { 80 } else { 36 });
    let cw = inner.width() / cols as f32;
    let ch = inner.height() / rows as f32;
    if cols < 10 || rows < 10 {
        let rr = if full {
            Rect::from_min_size(
                pad.right_bottom() - Vec2::new(radar_w + 10.0, radar_w * 1.15 + 10.0),
                Vec2::new(radar_w, radar_w * 1.15),
            )
        } else {
            radar_split.shrink(4.0)
        };
        draw_radar(ns, ui.painter(), rr, full);
        return;
    }
    let painter = ui.painter().with_clip_rect(inner);
    let font_px = ch.clamp(8.0, 14.0);
    let need = ns
        .atlas
        .as_ref()
        .map(|a| (a.px - font_px).abs() > 0.45)
        .unwrap_or(true);
    if need {
        ns.atlas = Some(Atlas::new(
            &painter,
            FontId::new(font_px, theme::mono()),
            font_px,
        ));
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
        if ns.indoors && hit.mz >= 14 && (hit.mx == 7 || hit.mx == 8 || hit.mz > 14) {
            let mut out = march_from(ns, ns.ret_x, ns.ret_z, rdx, rdz, true);
            out.dist += hit.dist.max(0.4);
            if out.h > 0.15 {
                hit = out;
            }
        }
        hit.dist *= (u * FOV).cos().max(0.72);
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
        let far = hit.dist > 22.0 && !ns.indoors;
        let reflect = if hit.h > 0.15 && !far {
            Some(wall_tex(&hit, 0.62, t))
        } else {
            None
        };
        for row in 0..rows {
            let rf = row as f32;
            let (glyph, color) = if hit.h > 0.15 && rf >= top && rf <= bot && hit.dist < 48.0 {
                let v = ((rf - top) / (bot - top).max(0.001)).clamp(0.0, 1.0);
                if far {
                    skyline_tex(&hit, v)
                } else {
                    wall_tex(&hit, v, t)
                }
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
                if ns.indoors {
                    ceiling_tex(fx, fz, d, t, Some(interior_lamp(ns.floor)))
                } else if d < 14.0 {
                    ceiling_tex(fx, fz, d, t, None)
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
            10 => "KIOSK",
            11 => "SHRINE",
            12 => "GLASSHOUSE",
            13 => "CANOPY",
            _ => "HAB BLOCK",
        }
    } else {
        match center_kind {
            Kind::Plaza => "CORP PLAZA",
            Kind::Park => "GRID PARK",
            Kind::Garden => "PLANTER",
            Kind::Avenue => "AVENUE",
            Kind::Street => "SIDE STREET",
            Kind::Alley => "ALLEY",
            Kind::Sidewalk => "WALK",
            Kind::Market => "NIGHT MARKET",
            Kind::Trench => "DATA TRENCH",
            Kind::Canal => "CANAL",
            Kind::Solid => "ICE",
        }
    };

    let radar_rect = if full {
        Rect::from_min_size(
            pad.right_bottom() - Vec2::new(radar_w + 12.0, radar_w * 1.18 + 14.0),
            Vec2::new(radar_w, radar_w * 1.18),
        )
    } else {
        radar_split.shrink(4.0)
    };
    if ns.map_on {
        draw_radar(ns, ui.painter(), radar_rect, full);
    }
    if full {
        draw_cruise_hud(ui, ns, pad);
        draw_relay(ui, ns, pad);
    }
}

fn draw_relay(ui: &mut egui::Ui, ns: &mut Netspace, pad: Rect) {
    if !ns.relay {
        return;
    }
    let booths: Vec<(f32, f32)> = ns
        .sprites
        .iter()
        .filter(|s| s.kind == 14)
        .map(|s| (s.x, s.z))
        .collect();
    let h = 28.0 + booths.len() as f32 * 26.0;
    let plate = Rect::from_min_size(
        pad.left_top() + Vec2::new(12.0, 42.0),
        Vec2::new(220.0, h.min(pad.height() - 56.0)),
    );
    ui.painter()
        .rect_filled(plate, 4.0, Color32::from_rgba_unmultiplied(8, 10, 14, 230));
    ui.painter()
        .rect_stroke(plate, 4.0, Stroke::new(1.0, CYAN), StrokeKind::Inside);
    ui.painter().text(
        plate.left_top() + Vec2::new(10.0, 8.0),
        Align2::LEFT_TOP,
        "TELEPHONE RELAY",
        FontId::new(12.0, theme::mono()),
        theme::ACID,
    );
    for (i, (x, z)) in booths.iter().enumerate() {
        let row = Rect::from_min_size(
            plate.left_top() + Vec2::new(8.0, 28.0 + i as f32 * 26.0),
            Vec2::new(plate.width() - 16.0, 24.0),
        );
        if row.bottom() > plate.bottom() - 4.0 {
            break;
        }
        let resp = ui.interact(row, egui::Id::new(("relay", i)), Sense::click());
        let col = if resp.hovered() { theme::ACID } else { CYAN };
        ui.painter().rect_filled(
            row,
            3.0,
            if resp.hovered() {
                Color32::from_rgba_unmultiplied(214, 255, 63, 28)
            } else {
                Color32::from_rgba_unmultiplied(77, 232, 255, 16)
            },
        );
        ui.painter().text(
            row.left_center() + Vec2::new(8.0, 0.0),
            Align2::LEFT_CENTER,
            format!("BOOTH {}  {}", i + 1, street_name(*x, *z)),
            FontId::new(12.0, theme::mono()),
            col,
        );
        if resp.clicked() {
            ns.x = *x;
            ns.z = *z + 0.9;
            ns.relay = false;
            ns.indoors = false;
            ns.look_on = false;
        }
    }
}

fn draw_cruise_hud(ui: &mut egui::Ui, ns: &mut Netspace, pad: Rect) {
    let bar = Rect::from_min_max(
        pad.left_top() + Vec2::new(10.0, 8.0),
        Pos2::new(pad.right() - 12.0, pad.top() + 34.0),
    );
    ui.painter()
        .rect_filled(bar, 0.0, Color32::from_rgba_unmultiplied(6, 10, 14, 210));
    ui.painter()
        .rect_stroke(bar, 0.0, Stroke::new(1.0, CYAN), StrokeKind::Inside);
    let auto = Rect::from_min_size(bar.right_center() + Vec2::new(-118.0, -12.0), Vec2::new(108.0, 24.0));
    let text_clip = Rect::from_min_max(
        bar.left_top() + Vec2::new(8.0, 2.0),
        Pos2::new((auto.left() - 8.0).max(bar.left() + 24.0), bar.bottom() - 2.0),
    );
    ui.painter().with_clip_rect(text_clip).text(
        text_clip.left_center(),
        Align2::LEFT_CENTER,
        format!(
            "NETSPACE  ·  {}  ·  {}  ·  {}  ·  CLICK LOOK  M MAP  E BOOTH  F DOOR  C AUTO",
            ns.district(),
            ns.street(),
            ns.look
        ),
        FontId::new(12.0, theme::mono()),
        CYAN,
    );
    let on = ns.cruise;
    theme::fill_chamfer(
        ui,
        auto,
        4.0,
        if on { CYAN } else { Color32::from_rgba_unmultiplied(77, 232, 255, 18) },
        Stroke::new(1.4, CYAN),
    );
    ui.painter().text(
        auto.center(),
        Align2::CENTER_CENTER,
        if on { "AUTO ON" } else { "AUTO WALK" },
        FontId::new(12.0, theme::ui_font()),
        if on { INK } else { CYAN },
    );
    let hit = ui.interact(auto, egui::Id::new("ns-cruise"), Sense::click());
    if hit.clicked() {
        ns.cruise = !ns.cruise;
        ns.stuck = 0.0;
        if ns.cruise {
            cruise_pick(ns, true);
            ns.yaw = dir_yaw(ns.cruise_dir);
        }
    }
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
    for (name, px, pz, _) in &ns.people {
        let s = Sprite {
            x: *px,
            z: *pz,
            vx: 0.0,
            vz: 0.0,
            kind: 20,
        };
        let dx = s.x - ns.x;
        let dz = s.z - ns.z;
        let depth = dx * sy + dz * cy;
        if depth > 0.32 && depth < 18.0 {
            order.push((depth, s));
            let _ = name;
        }
    }
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
        let lift = if s.kind == 3 {
            2.2 + (t * 2.0).sin() * 0.4
        } else if s.kind == 11 {
            3.4 + (t * 1.6).sin() * 0.8
        } else {
            0.0
        };
        let bot = horizon + ((cam_y - lift) / depth) * rows as f32 * 0.20;
        let top = bot - spr_h;
        let width = match s.kind {
            0 => (3.1 / depth).ceil() as i32,
            13 => (1.8 / depth).ceil() as i32,
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
                    let paint = car_color(s.x, s.z);
                    let lamp = depth < 6.0 && dc == 0;
                    (
                        if depth < 4.0 { body } else { 'H' },
                        fogged(if lamp { Color32::from_rgb(255, 244, 214) } else { paint }, depth),
                    )
                }
                1 => (
                    if depth < 5.0 { 'i' } else { '!' },
                    fogged(coat_color(s.x, s.z), depth),
                ),
                2 => (
                    'I',
                    fogged(Color32::from_rgb(255, 214, 150), depth),
                ),
                3 => ('*', fogged(CYAN, depth * 0.6)),
                4 => (
                    'A',
                    fogged(
                        match hash2(s.x as i32, s.z as i32) % 4 {
                            0 => Color32::from_rgb(190, 48, 60),
                            1 => Color32::from_rgb(220, 170, 50),
                            2 => Color32::from_rgb(40, 140, 130),
                            _ => Color32::from_rgb(70, 110, 190),
                        },
                        depth,
                    ),
                ),
                9 => (
                    if dc == 0 { 'Y' } else { '"' },
                    fogged(Color32::from_rgb(40, 130, 60), depth),
                ),
                10 => (
                    '*',
                    fogged(
                        match hash2(s.x as i32, s.z as i32) % 4 {
                            0 => Color32::from_rgb(230, 80, 120),
                            1 => Color32::from_rgb(240, 200, 60),
                            2 => Color32::from_rgb(240, 240, 245),
                            _ => Color32::from_rgb(120, 90, 200),
                        },
                        depth * 0.7,
                    ),
                ),
                11 => ('^', fogged(CYAN, depth * 0.5)),
                12 => ('n', fogged(DIM, depth)),
                13 => ('o', fogged(Color32::from_rgb(80, 200, 140), depth)),
                14 => ('T', fogged(Color32::from_rgb(255, 196, 80), depth)),
                5 => ('#', fogged(DIM, depth)),
                6 => ('*', fogged(Color32::from_rgb(255, 210, 140), depth * 0.5)),
                7 => ('=', fogged(CYAN, depth * 0.4)),
                8 => ('O', fogged(Color32::from_rgb(255, 236, 200), depth)),
                20 => ('@', fogged(crate::theme::ACID, depth * 0.5)),
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
            if s.kind == 20 {
                if let Some((name, _, _, _)) = ns.people.iter().find(|p| (p.1 - s.x).abs() < 0.2 && (p.2 - s.z).abs() < 0.2) {
                    let label = if name.chars().count() > 12 {
                        name.chars().take(12).collect::<String>()
                    } else {
                        name.clone()
                    };
                    painter.text(
                        Pos2::new(inner.left() + col as f32 * cw, inner.top() + top.max(0.0) * ch - ch),
                        Align2::CENTER_BOTTOM,
                        label,
                        FontId::new(ch.max(10.0), theme::mono()),
                        crate::theme::ACID,
                    );
                }
            }
        }
    }
}

fn draw_radar(ns: &Netspace, painter: &egui::Painter, rect: Rect, rich: bool) {
    painter.rect_filled(rect, 0.0, TITLE);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.5, CYAN), StrokeKind::Inside);
    painter.rect_stroke(
        rect.shrink(3.0),
        0.0,
        Stroke::new(1.0, mix(CYAN, BG, 0.4)),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(Vec2::new(8.0, 8.0));
    painter.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        if rich { "NETMAP" } else { "RADAR" },
        FontId::new(13.0, theme::display()),
        CYAN,
    );
    painter.text(
        inner.right_top(),
        Align2::RIGHT_TOP,
        ns.district(),
        FontId::new(11.0, theme::mono()),
        CYAN,
    );
    let grid = Rect::from_min_max(
        inner.left_top() + Vec2::new(0.0, 20.0),
        inner.right_bottom() - Vec2::new(0.0, if rich { 42.0 } else { 36.0 }),
    );
    painter.rect_filled(grid, 0.0, BG);
    painter.rect_stroke(grid, 0.0, Stroke::new(1.0, mix(ORANGE, BG, 0.45)), StrokeKind::Inside);
    let cells = if rich { 52i32 } else { 36i32 };
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
                let body = facade_body(
                    hash2(wx.div_euclid(LOT), wz.div_euclid(LOT)) % 8,
                    c.facade,
                );
                if c.ice {
                    mix(CYAN, BG, 0.15)
                } else {
                    mix(body, BG, 0.28)
                }
            } else {
                match c.kind {
                    Kind::Plaza => mix(CYAN, BG, 0.35),
                    Kind::Park => mix(Color32::from_rgb(46, 140, 72), BG, 0.35),
                    Kind::Garden => mix(Color32::from_rgb(80, 160, 70), BG, 0.4),
                    Kind::Market => mix(Color32::from_rgb(190, 120, 48), BG, 0.35),
                    Kind::Trench | Kind::Canal => mix(CYAN, BG, 0.4),
                    Kind::Avenue => mix(DIM, BG, 0.08),
                    Kind::Street => mix(DIM, BG, 0.28),
                    Kind::Alley => mix(DIM, BG, 0.5),
                    Kind::Sidewalk => mix(PANEL, MUTED, 0.35),
                    _ => mix(PANEL, BG, 0.1),
                }
            };
            let p = Pos2::new(grid.left() + mx as f32 * cw, grid.top() + mz as f32 * ch);
            painter.rect_filled(
                Rect::from_min_size(p, Vec2::new(cw.max(1.0), ch.max(1.0))),
                0.0,
                col,
            );
        }
    }
    let cx = grid.center();
    let facing = Vec2::new(ns.yaw.sin(), -ns.yaw.cos());
    let range = cw.max(ch);
    for i in 1..=3 {
        let r = range * (6.0 + i as f32 * 5.0);
        painter.circle_stroke(cx, r.min(grid.width() * 0.46), Stroke::new(1.0, mix(CYAN, BG, 0.72)));
    }
    let left = Vec2::new((-FOV * 0.5).sin(), -(-FOV * 0.5).cos());
    let right = Vec2::new((FOV * 0.5).sin(), -(FOV * 0.5).cos());
    let rot = |v: Vec2| {
        let c = ns.yaw.cos();
        let s = ns.yaw.sin();
        Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
    };
    let cone = range * 14.0;
    painter.line_segment([cx, cx + rot(left) * cone], Stroke::new(1.0, mix(CYAN, BG, 0.35)));
    painter.line_segment([cx, cx + rot(right) * cone], Stroke::new(1.0, mix(CYAN, BG, 0.35)));
    painter.line_segment([cx, cx + facing * (range * 7.0)], Stroke::new(2.0, CYAN));
    for s in &ns.sprites {
        let dx = ((s.x - ns.x).round() as i32).clamp(-cells / 2, cells / 2);
        let dz = ((s.z - ns.z).round() as i32).clamp(-cells / 2, cells / 2);
        if dx.abs() >= cells / 2 || dz.abs() >= cells / 2 {
            continue;
        }
        let p = Pos2::new(
            grid.center().x + dx as f32 * cw,
            grid.center().y + dz as f32 * ch,
        );
        let col = match s.kind {
            0 | 7 => ORANGE,
            1 => CREAM,
            2 | 8 => CYAN,
            3 | 11 => mix(CYAN, CREAM, 0.4),
            9 | 10 => MUTED,
            _ => DIM,
        };
        painter.rect_filled(Rect::from_center_size(p, Vec2::splat(2.2)), 0.0, col);
    }
    let tip = cx + facing * (range * 3.2);
    let left_w = cx + Vec2::new(-facing.y, facing.x) * (range * 1.4) - facing * range;
    let right_w = cx + Vec2::new(facing.y, -facing.x) * (range * 1.4) - facing * range;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, left_w, right_w],
        CYAN,
        Stroke::NONE,
    ));
    let n_pos = grid.center_top() + Vec2::new(0.0, 10.0);
    painter.text(n_pos, Align2::CENTER_CENTER, "N", FontId::new(11.0, theme::mono()), CYAN);
    painter.text(grid.center_bottom() + Vec2::new(0.0, -10.0), Align2::CENTER_CENTER, "S", FontId::new(10.0, theme::mono()), DIM);
    painter.text(grid.left_center() + Vec2::new(10.0, 0.0), Align2::CENTER_CENTER, "W", FontId::new(10.0, theme::mono()), DIM);
    painter.text(grid.right_center() + Vec2::new(-10.0, 0.0), Align2::CENTER_CENTER, "E", FontId::new(10.0, theme::mono()), DIM);
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
        if rich { "YOU · FOV" } else { "YOU" },
        FontId::new(11.0, theme::mono()),
        theme::ACID,
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
        let mut canal = 0;
        let mut garden = 0;
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
                if c.kind == Kind::Canal {
                    canal += 1;
                }
                if c.kind == Kind::Garden {
                    garden += 1;
                }
            }
        }
        assert!(walk > 800, "walkable {walk}");
        assert!(walls > 800, "walls {walls}");
        assert!(plaza >= 16, "plaza {plaza}");
        assert!(alley > 10, "alley {alley}");
        assert!(canal > 20, "canal {canal}");
        assert!(garden > 10, "garden {garden}");
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
    fn cruise_picks_street_waypoints() {
        let mut ns = Netspace::new();
        ns.cruise = true;
        cruise_pick(&mut ns, true);
        let d0 = wrap_delta(ns.cruise_tx, ns.x).hypot(wrap_delta(ns.cruise_tz, ns.z));
        assert!(d0 > 0.4, "waypoint too close {d0}");
        for _ in 0..40 {
            cruise_step(&mut ns, 0.05);
        }
        assert!(ns.walkable(ns.x, ns.z));
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
        assert!(ns.sprites.len() > 40);
    }
}
