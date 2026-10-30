//! Pinch it and a bud stays. A pinch is a short, small pull let go of at once,
//! too small to tear and too quick to be a stretch. A pull of the right length let go
//! of a little too late leaves only the shadow of a bud, which sinks back at once.
//! Two fingers on it, pinched together and let go of at once, are a pinch too, between them.
//! Five buds and it opens:
//! the body becomes a flower facing up, its petals unfolding from where the
//! buds were. Tap the flower and it folds back into the body. Pinch the flower
//! and it comes apart instead (dust.rs); pinch it a little too slowly and its petals
//! only flinch in, and open again.
//! On its own turn, after contact (mind/contact.rs), it may raise one bud by itself and wait
//! (lean.rs): a pinch anywhere on it keeps that bud as the first of five; left alone, it sinks back.

use super::lean::Lean;
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
const FINGERS_CLOSE: f32 = 0.75;
const FINGERS_MAX_SECS: f32 = 0.7;
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
    /// Raised by itself, on its turn: its size is the lean's until a pinch keeps it.
    pub offered: bool,
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
    /// Folding because it was let go of (spin.rs), not tapped: it says nothing when it is done.
    quiet: bool,
    /// The bud it raised by itself, rising, waiting or sinking back.
    pub lean: Lean,
    pub(super) events: Vec<Event>,
}

impl Default for Bloom {
    fn default() -> Self {
        Bloom {
            phase: Phase::Buds,
            buds: Vec::new(),
            near: None,
            flinched: None,
            angle: 0.0,
            quiet: false,
            lean: Lean::default(),
            events: Vec::new(),
        }
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
            Phase::Flower => 1.0 - self.flinched.map_or(0.0, |t| self.drawn_in(t)),
            Phase::Folding { t } => 1.0 - smoothstep(0.0, FOLD_SECS, t),
        }
    }

    /// How far the petals have drawn in, `t` seconds into a flinch.
    fn drawn_in(&self, t: f32) -> f32 {
        FLINCH * smoothstep(0.0, 0.12, t) * (1.0 - smoothstep(0.25, FLINCH_SECS, t))
    }

    /// Where to start folding so the petals close from as open as they are now.
    fn fold_from(&self) -> f32 {
        unsmoothstep(0.0, FOLD_SECS, 1.0 - self.open())
    }

    /// How far each bud has grown, 0..1. They give themselves up to the petals as it opens.
    pub fn bud_size(&self, b: &Bud) -> f32 {
        if b.offered {
            return self.lean.value() * (1.0 - smoothstep(0.0, 0.3, self.open()));
        }
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
        self.near = Some(Bud { dir, age: 0.0, offered: false });
        true
    }

    /// Raise one bud at `dir` by itself, to `to` of a bud, and wait (lean.rs). Only on the bare
    /// liquid: no buds yet, no shadow of one.
    pub fn offer(&mut self, dir: V3, to: f32) -> bool {
        if self.phase != Phase::Buds || !self.buds.is_empty() || self.near.is_some() || self.lean.leaning() {
            return false;
        }
        self.lean.begin(to);
        self.angle = dir.z.atan2(dir.x);
        self.buds.push(Bud { dir: dir.norm(), age: 0.0, offered: true });
        true
    }

    /// The bud it raised is taken on as an ordinary bud, grown from exactly as far as it was.
    /// Returns true if there was one.
    fn keep_offered(&mut self) -> bool {
        let Some(i) = self.buds.iter().position(|b| b.offered) else { return false };
        let size = self.lean.take();
        let b = &mut self.buds[i];
        b.offered = false;
        b.age = unsmoothstep(0.0, GROW_SECS, size);
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
        // A bud it raised by itself is kept, from as far as it had risen, as one of the five.
        let kept = self.keep_offered();
        // A pinch right next to an older bud only feeds it.
        if let Some(b) = self.buds.iter_mut().find(|b| b.dir.dot(dir) > 0.97) {
            b.age = b.age.min(GROW_SECS);
            return kept;
        }
        if self.buds.is_empty() {
            self.angle = dir.z.atan2(dir.x);
        }
        self.buds.push(Bud { dir, age: 0.0, offered: false });
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
        // Flinching again, it draws in further from where it is, never back open first.
        let t = self.flinched.map_or(0.0, |t| unsmoothstep(0.0, 0.12, self.drawn_in(t) / FLINCH));
        self.flinched = Some(t);
        true
    }

    /// Tap the flower and it closes.
    pub fn fold(&mut self) -> bool {
        if self.phase != Phase::Flower {
            return false;
        }
        self.phase = Phase::Folding { t: self.fold_from() };
        self.quiet = false;
        true
    }

    /// Its petals are gone (spin.rs): what is left of the flower closes back into the body, quietly.
    pub fn let_go(&mut self) {
        if self.phase == Phase::Flower {
            self.phase = Phase::Folding { t: self.fold_from() };
            self.quiet = true;
        }
    }

    /// Gone to dust (dust.rs): there is no flower left to fold, and no buds.
    pub fn scatter(&mut self) {
        self.lean = Lean::default();
        self.buds.clear();
        self.near = None;
        self.phase = Phase::Buds;
    }

    pub fn step(&mut self, dt: f32) {
        for b in self.buds.iter_mut().chain(&mut self.near) {
            b.age += dt;
        }
        // The bud it raised, left alone, sinks back and is gone.
        self.lean.step(dt);
        if !self.lean.leaning() {
            self.buds.retain(|b| !b.offered);
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
                if !std::mem::take(&mut self.quiet) {
                    self.events.push(Event::Folded);
                }
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
    fn tapped_or_pinched_again_while_it_flinches_it_does_not_jump_open() {
        let mut b = Bloom { phase: Phase::Flower, ..Bloom::default() };
        b.flinch();
        b.step(0.3);
        let was = b.open();
        assert!(b.fold());
        b.step(1.0 / 60.0);
        assert!((b.open() - was).abs() < 0.03, "folding: {was} -> {}", b.open());
        for late in [0.05, 0.3, 0.9] {
            let mut c = Bloom { phase: Phase::Flower, ..Bloom::default() };
            c.flinch();
            c.step(late);
            let was = c.open();
            assert!(c.flinch());
            c.step(1.0 / 60.0);
            // no faster than a flinch draws in by itself
            assert!((c.open() - was).abs() < 0.04, "flinching again at {late}: {was} -> {}", c.open());
        }
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
