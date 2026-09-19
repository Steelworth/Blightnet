use rand::Rng;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Luck {
    Norm,
    Adv,
    Dis,
}

pub struct Roll {
    pub pct: i32,
    pub pct_a: i32,
    pub pct_b: Option<i32>,
    pub luck: Luck,
    pub grade: &'static str,
    pub damage: Option<i32>,
    pub taken: bool,
    pub heal: bool,
    pub label: String,
    pub target: String,
    pub start: Instant,
    pub applied: bool,
}

impl Roll {
    pub fn display_pct(&self) -> i32 {
        let t = self.start.elapsed().as_secs_f32();
        if t >= 0.72 {
            self.pct
        } else {
            (((t * 63.0 + 1.7).sin().abs()) * 100.0) as i32
        }
    }

    pub fn done(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= 0.85
    }
}

pub fn roll_pct() -> i32 {
    rand::thread_rng().gen_range(0..=100)
}

pub fn with_luck(luck: Luck) -> (i32, Option<i32>, i32) {
    let a = roll_pct();
    match luck {
        Luck::Norm => (a, None, a),
        Luck::Adv => {
            let b = roll_pct();
            (a, Some(b), a.max(b))
        }
        Luck::Dis => {
            let b = roll_pct();
            (a, Some(b), a.min(b))
        }
    }
}

pub fn grade(pct: i32) -> &'static str {
    if pct <= 0 {
        "critical failure"
    } else if pct >= 100 {
        "critical success"
    } else if pct >= 40 {
        "success"
    } else {
        "failure"
    }
}

pub fn roll_formula(spec: &str) -> i32 {
    let s = spec.trim().to_lowercase().replace(' ', "");
    if s.is_empty() {
        return 0;
    }
    let mut total = 0i32;
    let mut rest = s.as_str();
    let mut sign = 1i32;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('+') {
            sign = 1;
            rest = r;
            continue;
        }
        if let Some(r) = rest.strip_prefix('-') {
            sign = -1;
            rest = r;
            continue;
        }
        if let Some(x) = rest.find('d') {
            let n: i32 = rest[..x].parse().unwrap_or(1).clamp(1, 40);
            rest = &rest[x + 1..];
            let end = rest.find(['+', '-']).unwrap_or(rest.len());
            let faces: i32 = rest[..end].parse().unwrap_or(6).clamp(2, 100);
            rest = &rest[end..];
            let mut sum = 0;
            for _ in 0..n {
                sum += rand::thread_rng().gen_range(1..=faces);
            }
            total += sign * sum;
            sign = 1;
        } else {
            let end = rest.find(['+', '-']).unwrap_or(rest.len());
            let n: i32 = rest[..end].parse().unwrap_or(0);
            rest = &rest[end..];
            total += sign * n;
            sign = 1;
        }
    }
    total
}

pub fn outcome(formula: &str, pct: i32, heal: bool) -> (Option<i32>, bool) {
    let g = grade(pct);
    if formula.trim().is_empty() {
        return (None, false);
    }
    match g {
        "critical success" => (Some((roll_formula(formula) * 2).max(1)), false),
        "success" => {
            let rolled = roll_formula(formula).max(1);
            let scaled = ((rolled as f32) * (pct as f32 / 100.0)).round() as i32;
            (Some(scaled.max(1)), false)
        }
        "critical failure" if !heal => (Some(roll_formula(formula).max(1)), true),
        _ => (None, false),
    }
}

pub fn fire(label: &str, formula: &str, heal: bool, luck: Luck, target: &str) -> Roll {
    let (a, b, pct) = with_luck(luck);
    let g = grade(pct);
    let (damage, taken) = outcome(formula, pct, heal);
    Roll {
        pct,
        pct_a: a,
        pct_b: b,
        luck,
        grade: g,
        damage,
        taken,
        heal,
        label: label.into(),
        target: target.into(),
        start: Instant::now(),
        applied: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_cover_the_full_range() {
        assert_eq!(grade(0), "critical failure");
        assert_eq!(grade(1), "failure");
        assert_eq!(grade(39), "failure");
        assert_eq!(grade(40), "success");
        assert_eq!(grade(99), "success");
        assert_eq!(grade(100), "critical success");
    }

    #[test]
    fn formula_parses_dice_and_modifiers() {
        for _ in 0..40 {
            let n = roll_formula("1d4");
            assert!((1..=4).contains(&n), "{n}");
            let n = roll_formula("2d6+3");
            assert!((5..=15).contains(&n), "{n}");
            let n = roll_formula("1d8 slashing");
            assert!((1..=8).contains(&n), "{n}");
        }
        assert_eq!(roll_formula(""), 0);
        assert_eq!(roll_formula("5"), 5);
        assert_eq!(roll_formula("-2"), -2);
    }

    #[test]
    fn outcome_crits_and_heals() {
        for _ in 0..20 {
            let (d, taken) = outcome("1d6", 100, false);
            assert!(!taken);
            assert!(d.unwrap() >= 2);
            let (d, taken) = outcome("1d6", 0, false);
            assert!(taken);
            assert!(d.unwrap() >= 1);
            let (d, taken) = outcome("1d6", 20, false);
            assert!(d.is_none());
            assert!(!taken);
            let (d, taken) = outcome("1d8", 0, true);
            assert!(d.is_none());
            assert!(!taken);
            let (d, _) = outcome("1d6", 80, false);
            assert!(d.unwrap() >= 1);
        }
        assert_eq!(outcome("", 100, false), (None, false));
    }

    #[test]
    fn luck_adv_keeps_high_dis_keeps_low() {
        for _ in 0..30 {
            let (a, b, p) = with_luck(Luck::Adv);
            let b = b.unwrap();
            assert_eq!(p, a.max(b));
            let (a, b, p) = with_luck(Luck::Dis);
            let b = b.unwrap();
            assert_eq!(p, a.min(b));
            let (_, b, _) = with_luck(Luck::Norm);
            assert!(b.is_none());
        }
    }

    #[test]
    fn fire_fills_roll() {
        for _ in 0..15 {
            let r = fire("Punch", "1d4", false, Luck::Norm, "goblin");
            assert_eq!(r.label, "Punch");
            assert_eq!(r.target, "goblin");
            assert!(!r.applied);
            assert!((0..=100).contains(&r.pct));
        }
    }
}
