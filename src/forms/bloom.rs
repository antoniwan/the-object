//! Pinch it and a bud stays. A pinch is a short, small pull let go of at once,
//! too small to tear and too quick to be a stretch. Five buds and it opens:
//! the body becomes a flower facing up, its petals unfolding from where the
//! buds were. Tap the flower and it folds back into the body.

use super::{smoothstep, Event};
use crate::math::*;

pub const PETALS: usize = 5;
/// A pull let go of within this many seconds, reaching this far (in body radii), is a pinch.
pub const PINCH_MAX_SECS: f32 = 0.4;
pub const PINCH_MIN: f32 = 0.12;
pub const PINCH_MAX: f32 = 0.75;
const GROW_SECS: f32 = 0.5;
/// Buds left alone this long sink back in.
const BUD_LIFE_SECS: f32 = 45.0;
const OPEN_SECS: f32 = 3.2;
const FOLD_SECS: f32 = 2.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bud {
    /// Direction from the body center.
    pub dir: V3,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Buds,
    Opening { t: f32 },
    Flower,
    Folding { t: f32 },
}

pub struct Bloom {
    pub phase: Phase,
    pub buds: Vec<Bud>,
    /// Where around the stem the first petal points.
    pub angle: f32,
    pub(super) events: Vec<Event>,
}

impl Default for Bloom {
    fn default() -> Self {
        Bloom { phase: Phase::Buds, buds: Vec::new(), angle: 0.0, events: Vec::new() }
    }
}

impl Bloom {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Buds
    }

    /// 0 = a body, 1 = a flower wide open.
    pub fn open(&self) -> f32 {
        match self.phase {
            Phase::Buds => 0.0,
            Phase::Opening { t } => smoothstep(0.0, OPEN_SECS, t),
            Phase::Flower => 1.0,
            Phase::Folding { t } => 1.0 - smoothstep(0.0, FOLD_SECS, t),
        }
    }

    /// How far each bud has grown, 0..1. They give themselves up to the petals as it opens.
    pub fn bud_size(&self, b: &Bud) -> f32 {
        let grown = smoothstep(0.0, GROW_SECS, b.age) * (1.0 - smoothstep(BUD_LIFE_SECS - 3.0, BUD_LIFE_SECS, b.age));
        grown * (1.0 - smoothstep(0.0, 0.3, self.open()))
    }

    /// A pinch at `dir`. Returns true if it left a bud.
    pub fn pinch(&mut self, dir: V3) -> bool {
        if self.phase != Phase::Buds || self.buds.len() >= PETALS {
            return false;
        }
        let dir = dir.norm();
        // A pinch right next to an older bud only feeds it.
        if let Some(b) = self.buds.iter_mut().find(|b| b.dir.dot(dir) > 0.97) {
            b.age = b.age.min(GROW_SECS);
            return false;
        }
        if self.buds.is_empty() {
            self.angle = dir.z.atan2(dir.x);
        }
        self.buds.push(Bud { dir, age: 0.0 });
        self.events.push(Event::Budded);
        if self.buds.len() == PETALS {
            self.phase = Phase::Opening { t: 0.0 };
        }
        true
    }

    /// Tap the flower and it closes.
    pub fn fold(&mut self) -> bool {
        if self.phase != Phase::Flower {
            return false;
        }
        self.phase = Phase::Folding { t: 0.0 };
        true
    }

    pub fn step(&mut self, dt: f32) {
        for b in &mut self.buds {
            b.age += dt;
        }
        self.phase = match self.phase {
            Phase::Buds => {
                self.buds.retain(|b| b.age < BUD_LIFE_SECS);
                Phase::Buds
            }
            Phase::Opening { t } if t + dt >= OPEN_SECS => {
                self.events.push(Event::Bloomed);
                Phase::Flower
            }
            Phase::Opening { t } => Phase::Opening { t: t + dt },
            Phase::Flower => Phase::Flower,
            Phase::Folding { t } if t + dt >= FOLD_SECS => {
                self.buds.clear();
                self.events.push(Event::Folded);
                Phase::Buds
            }
            Phase::Folding { t } => Phase::Folding { t: t + dt },
        };
    }

    /// Roughly the flower, for touching it: a wide, low disc of petals. The shader has the real petals.
    pub fn sdf(&self, p: V3, pos: V3, r: f32) -> f32 {
        let o = self.open();
        let reach = r * (0.9 + 0.55 * o);
        let height = r * (1.3 - 0.95 * o);
        let rad = v3(reach, height, reach);
        let q = (p - pos).div(rad);
        let k0 = q.len();
        let k1 = (p - pos).div(v3(rad.x * rad.x, rad.y * rad.y, rad.z * rad.z)).len();
        k0 * (k0 - 1.0) / k1.max(1e-6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_pinches_open_into_a_flower_and_a_tap_folds_it() {
        let mut b = Bloom::default();
        for i in 0..PETALS {
            let a = i as f32 * 1.3;
            assert!(b.pinch(v3(a.cos(), 0.3, a.sin())));
        }
        assert!(!b.whole());
        for _ in 0..500 {
            b.step(1.0 / 120.0);
        }
        assert_eq!(b.phase, Phase::Flower);
        assert_eq!(b.open(), 1.0);
        assert!(b.events.contains(&Event::Bloomed));
        // the flower is wider than it is tall
        assert!(b.sdf(v3(1.3, 0.0, 0.0), V3::ZERO, 1.0) < 0.0);
        assert!(b.sdf(v3(0.0, 0.6, 0.0), V3::ZERO, 1.0) > 0.0);
        assert!(b.fold());
        for _ in 0..400 {
            b.step(1.0 / 120.0);
        }
        assert!(b.whole() && b.buds.is_empty());
    }

    #[test]
    fn pinching_the_same_place_twice_makes_one_bud() {
        let mut b = Bloom::default();
        assert!(b.pinch(v3(1.0, 0.0, 0.0)));
        assert!(!b.pinch(v3(1.0, 0.05, 0.0)));
        assert_eq!(b.buds.len(), 1);
    }

    #[test]
    fn buds_left_alone_sink_back() {
        let mut b = Bloom::default();
        b.pinch(V3::UP);
        for _ in 0..(50 * 30) {
            b.step(1.0 / 30.0);
        }
        assert!(b.buds.is_empty());
    }
}
