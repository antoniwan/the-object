//! Pinch it and a bud stays. A pinch is a short, small pull let go of at once,
//! too small to tear and too quick to be a stretch. A pull of the right length let go
//! of a little too late leaves only the shadow of a bud, which sinks back at once.
//! Two fingers on it, pinched together and let go of at once, are a pinch too, between them.
//! Five buds and it opens:
//! the body becomes a flower facing up, its petals unfolding from where the
//! buds were. Tap the flower and it folds back into the body. Pinch the flower
//! and it comes apart instead (dust.rs); pinch it a little too slowly and its petals
//! only flinch in, and open again.

use super::Event;
use crate::math::*;

pub const PETALS: usize = 5;
/// A pull let go of within this many seconds, reaching this far (in body radii), is a pinch.
pub const PINCH_MAX_SECS: f32 = 0.4;
pub const PINCH_MIN: f32 = 0.12;
pub const PINCH_MAX: f32 = 0.75;
/// The same pull let go of later than a pinch, but sooner than this, nearly leaves a bud.
pub const NEAR_MAX_SECS: f32 = 1.1;
/// How long the shadow of a bud lasts, and how big it gets beside a real one.
const NEAR_SECS: f32 = 1.3;
const NEAR_SIZE: f32 = 0.7;
/// Two fingers that both began on it, closed to this much of how far apart they began and
/// let go of within `FINGERS_MAX_SECS`, pinched it.
pub const FINGERS_CLOSE: f32 = 0.75;
pub const FINGERS_MAX_SECS: f32 = 0.7;
/// On the flower, which does not stretch, a pull this short (canvas heights) let go of at once is a pinch.
pub const TUG_MAX: f32 = 0.12;
/// The flower pinched a little too slowly: its petals draw in this much, and open again over `FLINCH_SECS`.
const FLINCH: f32 = 0.18;
const FLINCH_SECS: f32 = 1.4;
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
    /// A pinch let go of too late: a bud that rises a little and sinks back.
    pub near: Option<Bud>,
    /// Seconds since the flower last flinched, if it is still opening again.
    flinched: Option<f32>,
    /// Where around the stem the first petal points.
    pub angle: f32,
    pub(super) events: Vec<Event>,
}

impl Default for Bloom {
    fn default() -> Self {
        Bloom { phase: Phase::Buds, buds: Vec::new(), near: None, flinched: None, angle: 0.0, events: Vec::new() }
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
            Phase::Flower => 1.0 - self.flinched.map_or(0.0, |t| FLINCH * smoothstep(0.0, 0.12, t) * (1.0 - smoothstep(0.25, FLINCH_SECS, t))),
            Phase::Folding { t } => 1.0 - smoothstep(0.0, FOLD_SECS, t),
        }
    }

    /// How far each bud has grown, 0..1. They give themselves up to the petals as it opens.
    pub fn bud_size(&self, b: &Bud) -> f32 {
        let grown = smoothstep(0.0, GROW_SECS, b.age) * (1.0 - smoothstep(BUD_LIFE_SECS - 3.0, BUD_LIFE_SECS, b.age));
        grown * (1.0 - smoothstep(0.0, 0.3, self.open()))
    }

    /// How far the shadow of a bud has risen, 0..`NEAR_SIZE`.
    pub fn near_size(&self) -> f32 {
        self.near.map_or(0.0, |b| NEAR_SIZE * smoothstep(0.0, 0.15, b.age) * (1.0 - smoothstep(0.3, NEAR_SECS, b.age)))
    }

    /// A pinch at `dir` let go of a little too late. Returns true if a bud nearly rose there.
    pub fn almost(&mut self, dir: V3) -> bool {
        let dir = dir.norm();
        if self.phase != Phase::Buds || self.buds.len() >= PETALS || self.buds.iter().any(|b| b.dir.dot(dir) > 0.97) {
            return false;
        }
        self.near = Some(Bud { dir, age: 0.0 });
        true
    }

    /// Two fingers let go of after `secs`, having closed to `least` of how far apart they began:
    /// whether that was a pinch, not a squeeze.
    pub fn fingers_pinched(secs: f32, least: f32) -> bool {
        secs < FINGERS_MAX_SECS && least <= FINGERS_CLOSE
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
        self.near = None;
        self.events.push(Event::Budded);
        if self.buds.len() == PETALS {
            self.phase = Phase::Opening { t: 0.0 };
        }
        true
    }

    /// The flower pinched a little too slowly: it flinches. Returns true if it did.
    pub fn flinch(&mut self) -> bool {
        if self.phase != Phase::Flower {
            return false;
        }
        self.flinched = Some(0.0);
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

    /// Gone to dust (dust.rs): there is no flower left to fold, and no buds.
    pub fn scatter(&mut self) {
        self.buds.clear();
        self.near = None;
        self.phase = Phase::Buds;
    }

    pub fn step(&mut self, dt: f32) {
        for b in self.buds.iter_mut().chain(&mut self.near) {
            b.age += dt;
        }
        self.near = self.near.filter(|b| b.age < NEAR_SECS && self.phase == Phase::Buds);
        self.flinched = self.flinched.map(|t| t + dt).filter(|t| *t < FLINCH_SECS && self.phase == Phase::Flower);
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
        let q = (p - pos).cdiv(rad);
        let k0 = q.len();
        let k1 = (p - pos).cdiv(v3(rad.x * rad.x, rad.y * rad.y, rad.z * rad.z)).len();
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
    fn a_pinch_let_go_too_late_rises_a_little_and_sinks_back() {
        let mut b = Bloom::default();
        assert!(b.almost(v3(0.0, 0.0, 1.0)));
        b.step(0.3);
        let risen = b.near_size();
        assert!(risen > 0.5 && risen <= NEAR_SIZE);
        assert!(b.buds.is_empty() && b.events.is_empty(), "it is not a bud");
        b.step(1.2);
        assert!(b.near.is_none() && b.near_size() == 0.0);
        // where a bud already is, nothing nearly rises
        b.pinch(V3::UP);
        assert!(!b.almost(V3::UP));
    }

    #[test]
    fn the_flower_pinched_too_slowly_flinches_and_opens_again() {
        let mut b = Bloom { phase: Phase::Flower, ..Bloom::default() };
        assert!(b.flinch());
        b.step(0.3);
        assert!(b.open() < 1.0 - FLINCH * 0.9 && b.open() >= 1.0 - FLINCH);
        b.step(1.2);
        assert_eq!(b.open(), 1.0);
        assert_eq!(b.phase, Phase::Flower);
        assert!(!Bloom::default().flinch(), "only a flower flinches");
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
