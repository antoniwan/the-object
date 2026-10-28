//! Draw a square around it and it takes the corners: the sphere becomes a
//! box holding exactly as much as the sphere did. Knock on it and something
//! inside knocks back. Knock three times and the lid opens, light comes out,
//! and so does everything it was keeping, flying out as colored drops until the
//! sky goes dark. When every drop has come home, it forgets its corners.

use super::Event;
use crate::math::*;

/// Half the side of a cube with the volume of a unit sphere: (π/6)^(1/3).
pub const HALF_SIDE: f32 = 0.806;
const ROUNDING: f32 = 0.06;
const FORM_SECS: f32 = 2.0;
const OPEN_SECS: f32 = 1.8;
const ROUND_SECS: f32 = 2.6;
pub const KNOCKS: u32 = 3;
/// How long the lid stays open, at least, before it can round off again.
const OPEN_AT_LEAST_SECS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Forming { t: f32 },
    Box,
    Opening { t: f32 },
    Open { t: f32 },
    Rounding { t: f32 },
}

pub struct Pandora {
    pub phase: Phase,
    /// Which way the box faces, about the vertical.
    pub yaw: f32,
    pub knocks: u32,
    /// The shiver of the last knock, 0..1.
    pub rattle: f32,
    spill: bool,
    pub(super) events: Vec<Event>,
}

impl Default for Pandora {
    fn default() -> Self {
        Pandora { phase: Phase::Whole, yaw: 0.0, knocks: 0, rattle: 0.0, spill: false, events: Vec::new() }
    }
}

impl Pandora {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// 0 = round, 1 = a box.
    pub fn boxness(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Forming { t } => smoothstep(0.0, FORM_SECS, t),
            Phase::Box | Phase::Opening { .. } | Phase::Open { .. } => 1.0,
            Phase::Rounding { t } => 1.0 - smoothstep(0.6, ROUND_SECS, t),
        }
    }

    /// 0 = shut, 1 = the lid thrown back.
    pub fn lid(&self) -> f32 {
        match self.phase {
            Phase::Opening { t } => smoothstep(0.0, OPEN_SECS, t),
            Phase::Open { .. } => 1.0,
            Phase::Rounding { t } => 1.0 - smoothstep(0.0, 1.2, t),
            _ => 0.0,
        }
    }

    /// Take the corners. `yaw` is which way the box should face.
    pub fn corner(&mut self, yaw: f32) -> bool {
        if !self.whole() {
            return false;
        }
        self.yaw = yaw;
        self.knocks = 0;
        self.phase = Phase::Forming { t: 0.0 };
        self.events.push(Event::Cornered);
        true
    }

    pub fn knock(&mut self) -> bool {
        if self.phase != Phase::Box {
            return false;
        }
        self.knocks += 1;
        self.rattle = 1.0;
        self.events.push(Event::Knocked);
        if self.knocks >= KNOCKS {
            self.phase = Phase::Opening { t: 0.0 };
            self.events.push(Event::Lid);
        }
        true
    }

    /// True once, when the lid is open far enough for what is inside to get out.
    pub fn spill(&mut self) -> bool {
        std::mem::take(&mut self.spill)
    }

    /// Once everything has come home it can forget its corners.
    pub fn round(&mut self) -> bool {
        match self.phase {
            Phase::Open { t } if t >= OPEN_AT_LEAST_SECS => {
                self.phase = Phase::Rounding { t: 0.0 };
                true
            }
            _ => false,
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.rattle *= (-5.0 * dt).exp();
        self.phase = match self.phase {
            Phase::Forming { t } if t + dt >= FORM_SECS => Phase::Box,
            Phase::Forming { t } => Phase::Forming { t: t + dt },
            Phase::Opening { t } if t + dt >= OPEN_SECS => Phase::Open { t: 0.0 },
            Phase::Opening { t } => {
                if t < OPEN_SECS * 0.35 && t + dt >= OPEN_SECS * 0.35 {
                    self.spill = true;
                }
                Phase::Opening { t: t + dt }
            }
            Phase::Open { t } => Phase::Open { t: t + dt },
            Phase::Rounding { t } if t + dt >= ROUND_SECS => {
                self.events.push(Event::Rounded);
                Phase::Whole
            }
            Phase::Rounding { t } => Phase::Rounding { t: t + dt },
            p => p,
        };
    }

    /// The box without its lid's hinge (the shader has that), for touching it.
    pub fn sdf(&self, p: V3, pos: V3, r: f32) -> f32 {
        let (c, s) = (self.yaw.cos(), self.yaw.sin());
        let q = p - pos;
        let q = v3(c * q.x - s * q.z, q.y, s * q.x + c * q.z);
        let half = HALF_SIDE * r;
        let k = ROUNDING * half;
        let d = v3(q.x.abs() - half + k, q.y.abs() - half + k, q.z.abs() - half + k);
        let outside = v3(d.x.max(0.0), d.y.max(0.0), d.z.max(0.0)).len();
        outside + d.x.max(d.y).max(d.z).min(0.0) - k
    }
}

/// Does a drag path (canvas pixels) draw a closed four-cornered shape around `center`?
/// `h` is the canvas height, which sets the scale of everything.
pub fn is_square(path: &[(f32, f32)], center: (f32, f32), h: f32) -> bool {
    if path.len() < 6 {
        return false;
    }
    // Resample at even steps so a fast and a slow hand look the same.
    let step = 0.02 * h;
    let mut pts = vec![path[0]];
    let mut carry = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let mut at = step - carry;
        while at <= seg {
            let f = at / seg;
            pts.push((a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
            at += step;
        }
        carry = seg - (at - step);
    }
    let len = pts.len() as f32 * step;
    let (first, last) = (pts[0], *pts.last().unwrap());
    let gap = ((last.0 - first.0).powi(2) + (last.1 - first.1).powi(2)).sqrt();
    if len < 0.5 * h || gap > (0.2 * h).max(len * 0.15) || pts.len() < 12 {
        return false;
    }
    // Close the loop so the last corner can be found too.
    let n = pts.len();
    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    // It has to go around the object, once.
    let mut winding = 0.0;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let (aa, ba) = ((a.1 - center.1).atan2(a.0 - center.0), (b.1 - center.1).atan2(b.0 - center.0));
        winding += wrap(ba - aa);
    }
    if (winding.abs() / std::f32::consts::TAU) < 0.75 {
        return false;
    }

    let heading: Vec<f32> = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (b.1 - a.1).atan2(b.0 - a.0)
        })
        .collect();
    let turn: Vec<f32> = (0..n).map(|i| wrap(heading[(i + 1) % n] - heading[i])).collect();
    let total: f32 = turn.iter().map(|t| t.abs()).sum();
    // A corner is a short stretch where the path turns hard.
    let sharp: Vec<bool> = (0..n).map(|i| (turn[(i + n - 1) % n] + turn[i] + turn[(i + 1) % n]).abs() > 0.8).collect();
    let mut corners = 0;
    let mut in_corners = 0.0;
    for i in 0..n {
        if sharp[i] {
            in_corners += turn[i].abs();
            if !sharp[(i + n - 1) % n] {
                corners += 1;
            }
        }
    }
    (3..=5).contains(&corners) && in_corners > 0.55 * total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon(corners: &[(f32, f32)], per_side: usize) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            for k in 0..per_side {
                let f = k as f32 / per_side as f32;
                out.push((a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
            }
        }
        out.push(corners[0]);
        out
    }

    #[test]
    fn a_square_around_it_counts_and_a_circle_does_not() {
        let sq = polygon(&[(200.0, 100.0), (600.0, 100.0), (600.0, 500.0), (200.0, 500.0)], 20);
        assert!(is_square(&sq, (400.0, 300.0), 600.0));
        // a square drawn somewhere else does not hold it
        assert!(!is_square(&sq, (750.0, 300.0), 600.0));
        let circle: Vec<_> = (0..=80)
            .map(|i| {
                let a = i as f32 / 80.0 * std::f32::consts::TAU;
                (400.0 + 220.0 * a.cos(), 300.0 + 220.0 * a.sin())
            })
            .collect();
        assert!(!is_square(&circle, (400.0, 300.0), 600.0));
        // nor does a line there and back
        let line = polygon(&[(200.0, 300.0), (600.0, 300.0)], 30);
        assert!(!is_square(&line, (400.0, 300.0), 600.0));
    }

    #[test]
    fn three_knocks_open_it_and_it_rounds_again() {
        let mut p = Pandora::default();
        assert!(p.corner(0.3));
        for _ in 0..300 {
            p.step(1.0 / 120.0);
        }
        assert_eq!(p.phase, Phase::Box);
        // a box with the sphere's volume
        assert!(p.sdf(v3(0.0, HALF_SIDE, 0.0), V3::ZERO, 1.0).abs() < 1e-3);
        for _ in 0..KNOCKS {
            assert!(p.knock());
        }
        let mut spilled = false;
        for _ in 0..(120 * 6) {
            p.step(1.0 / 120.0);
            spilled |= p.spill();
        }
        assert!(spilled);
        assert!(p.events.contains(&Event::Lid));
        assert!(p.round());
        for _ in 0..400 {
            p.step(1.0 / 120.0);
        }
        assert!(p.whole());
    }
}
