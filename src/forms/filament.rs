//! Stretch it and hold it taut, just short of tearing, and it stops resisting:
//! the whole body draws out along the pull into one thread, longer than the
//! room, holding exactly as much liquid as the sphere did. Tap the thread and
//! it is plucked; the farther from the middle you pluck it, the higher the
//! harmonic that rings. Hold it still and it winds back into a sphere. Drawn out by a hand
//! it is wary of (mind/temper.rs), it does not wait to be held: left alone a while, it winds
//! itself back.

use super::Event;
use crate::math::*;

/// Half the thread's length once fully drawn, in world units: past both edges of the screen.
const HALF_LEN: f32 = 11.0;
/// A tendril held at least this stretched (in body radii) for `TAUT_SECS` becomes the thread.
pub const TAUT: f32 = 0.95;
pub const TAUT_SECS: f32 = 0.9;
const DRAW_SECS: f32 = 2.6;
const WIND_SECS: f32 = 2.2;
/// How far a pluck swings it, and how fast the ringing dies.
const PLUCK_AMP: f32 = 0.32;
const PLUCK_DECAY: f32 = 0.9;
const MAX_HARMONIC: u32 = 9;
/// Plucked again while it rings, the swing it had and the shape of its harmonic give way to
/// the new ones over this long.
const REPLUCK_SECS: f32 = 0.12;
/// Drawn out wary, left alone this long (seconds) over how wary, it winds itself back; less
/// wary than `RESTLESS_FROM`, it stays.
const UNREST_SECS: f32 = 8.0;
const RESTLESS_FROM: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Drawing { t: f32 },
    Thread,
    Winding { t: f32 },
}

pub struct Filament {
    pub phase: Phase,
    /// Which way the thread runs, through the body's center.
    pub axis: V3,
    /// Which way it swings when plucked; perpendicular to `axis`.
    pub swing: V3,
    pub harmonic: u32,
    /// Seconds since the last pluck.
    since_pluck: f32,
    /// The swing, and the harmonic's shape, it had when it was last plucked.
    from_swing: f32,
    from_harmonic: f32,
    /// How wary it was of the hand that drew it, 0..1 (set as it is drawn), and seconds since it
    /// was last plucked or drawn: wary enough, left alone, it winds itself back.
    pub restless: f32,
    alone: f32,
    /// It wound itself back, not the hand.
    pub by_itself: bool,
    pub(super) events: Vec<Event>,
}

impl Default for Filament {
    fn default() -> Self {
        Filament {
            phase: Phase::Whole,
            axis: v3(1.0, 0.0, 0.0),
            swing: V3::UP,
            harmonic: 1,
            since_pluck: 1e3,
            from_swing: 0.0,
            from_harmonic: 1.0,
            restless: 0.0,
            alone: 0.0,
            by_itself: false,
            events: Vec::new(),
        }
    }
}

impl Filament {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// 0 = a sphere, 1 = the thread fully drawn.
    pub fn morph(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Drawing { t } => smoothstep(0.0, DRAW_SECS, t),
            Phase::Thread => 1.0,
            Phase::Winding { t } => 1.0 - smoothstep(0.0, WIND_SECS, t),
        }
    }

    pub fn half_len(&self) -> f32 {
        let m = self.morph();
        HALF_LEN * m * m
    }

    /// Thickness of a thread `half_len` long holding the volume of a sphere of radius `r`:
    /// the cylinder plus its two round ends, 2h·a² + 4/3·a³ = 4/3·r³.
    pub fn radius(&self, r: f32) -> f32 {
        let h = self.half_len();
        let goal = 4.0 / 3.0 * r * r * r;
        let mut a = r;
        for _ in 0..8 {
            let f = 2.0 * h * a * a + 4.0 / 3.0 * a * a * a - goal;
            let df = 4.0 * h * a + 4.0 * a * a;
            a = (a - f / df.max(1e-6)).clamp(0.01, r);
        }
        a
    }

    /// The thread's sideways swing right now, before the shape of the harmonic.
    pub fn ringing(&self) -> f32 {
        let t = self.since_pluck;
        let w = 7.0 + 2.2 * self.harmonic as f32;
        let rung = PLUCK_AMP * (-PLUCK_DECAY * t).exp() * (w * t).cos() * smoothstep(0.0, 0.03, t)
            / (self.harmonic as f32).sqrt();
        rung + self.from_swing * (1.0 - smoothstep(0.0, REPLUCK_SECS, t))
    }

    /// The shape of the harmonic it rings in now, which may lie between two just after a pluck.
    pub fn shape(&self) -> f32 {
        let k = smoothstep(0.0, REPLUCK_SECS, self.since_pluck);
        self.from_harmonic + (self.harmonic as f32 - self.from_harmonic) * k
    }

    /// Start drawing out along `axis`, the direction it was pulled.
    pub fn draw(&mut self, axis: V3) -> bool {
        if !self.whole() {
            return false;
        }
        self.axis = axis.norm();
        let side = if self.axis.dot(V3::UP).abs() > 0.9 { v3(1.0, 0.0, 0.0) } else { V3::UP };
        self.swing = (side - self.axis * side.dot(self.axis)).norm();
        self.phase = Phase::Drawing { t: 0.0 };
        self.restless = 0.0;
        self.alone = 0.0;
        self.by_itself = false;
        true
    }

    /// Pluck it at `at`. The harmonic is set by how far from the middle it was touched.
    pub fn pluck(&mut self, at: V3, pos: V3) -> Option<u32> {
        if self.phase != Phase::Thread {
            return None;
        }
        let s = (at - pos).dot(self.axis).abs();
        // still ringing, the new pluck starts from where the thread is
        self.from_swing = self.ringing();
        self.from_harmonic = self.shape();
        self.harmonic = (1 + (s / 0.7) as u32).min(MAX_HARMONIC);
        self.since_pluck = 0.0;
        self.alone = 0.0;
        self.events.push(Event::Plucked { harmonic: self.harmonic });
        Some(self.harmonic)
    }

    /// Wind back into a sphere.
    pub fn wind(&mut self) -> bool {
        if self.phase != Phase::Thread {
            return false;
        }
        self.phase = Phase::Winding { t: 0.0 };
        true
    }

    pub fn step(&mut self, dt: f32) {
        self.since_pluck += dt;
        if self.phase == Phase::Thread {
            self.alone += dt;
            if self.restless >= RESTLESS_FROM && self.alone * self.restless > UNREST_SECS {
                self.by_itself = true;
                self.phase = Phase::Winding { t: 0.0 };
            }
        }
        self.phase = match self.phase {
            Phase::Drawing { t } if t + dt >= DRAW_SECS => {
                self.events.push(Event::Threaded);
                Phase::Thread
            }
            Phase::Drawing { t } => Phase::Drawing { t: t + dt },
            Phase::Winding { t } if t + dt >= WIND_SECS => {
                self.events.push(Event::Wound);
                Phase::Whole
            }
            Phase::Winding { t } => Phase::Winding { t: t + dt },
            p => p,
        };
    }

    /// The thread, without its ringing (that only exists in the shader).
    pub fn sdf(&self, p: V3, pos: V3, r: f32) -> f32 {
        let h = self.half_len();
        let v = p - pos;
        let s = v.dot(self.axis).clamp(-h, h);
        (v - self.axis * s).len() - self.radius(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thread_holds_the_same_liquid_as_the_sphere() {
        let mut f = Filament::default();
        assert!(f.draw(v3(1.0, 0.2, 0.0)));
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        assert_eq!(f.phase, Phase::Thread);
        let (h, a) = (f.half_len(), f.radius(1.0));
        let pi = std::f32::consts::PI;
        let volume = pi * a * a * 2.0 * h + 4.0 / 3.0 * pi * a * a * a;
        assert!((volume - 4.0 / 3.0 * pi).abs() < 1e-3, "{volume}");
        assert!(a < 0.3, "it is thin: {a}");
        assert!(f.swing.dot(f.axis).abs() < 1e-4);
        // it runs far past the body, and is thin across
        assert!(f.sdf(f.axis * 8.0, V3::ZERO, 1.0) < 0.0);
        assert!(f.sdf(f.swing * 0.5, V3::ZERO, 1.0) > 0.0);
    }

    #[test]
    fn plucking_further_out_rings_a_higher_harmonic_and_holding_winds_it_back() {
        let mut f = Filament::default();
        f.draw(v3(1.0, 0.0, 0.0));
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        let low = f.pluck(v3(0.1, 0.0, 0.0), V3::ZERO).unwrap();
        let high = f.pluck(v3(2.5, 0.0, 0.0), V3::ZERO).unwrap();
        assert!(high > low);
        assert!(f.wind());
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        assert!(f.whole());
        assert!(f.events.contains(&Event::Wound));
        assert!((f.radius(1.0) - 1.0).abs() < 1e-4, "a sphere again");
    }

    #[test]
    fn drawn_out_wary_and_left_alone_it_winds_itself_back() {
        let mut f = Filament::default();
        f.draw(v3(1.0, 0.0, 0.0));
        f.restless = 1.0;
        for _ in 0..(60 * 9) {
            f.step(1.0 / 60.0);
        }
        assert_eq!(f.phase, Phase::Thread, "not yet");
        for _ in 0..(60 * 6) {
            f.step(1.0 / 60.0);
        }
        assert!(f.whole() && f.by_itself);
        // drawn out by a hand it is not wary of, it stays
        let mut f = Filament::default();
        f.draw(v3(1.0, 0.0, 0.0));
        f.restless = 0.1;
        for _ in 0..(60 * 120) {
            f.step(1.0 / 60.0);
        }
        assert_eq!(f.phase, Phase::Thread);
    }

    #[test]
    fn plucked_again_while_it_rings_it_does_not_jump() {
        let mut f = Filament::default();
        f.draw(v3(1.0, 0.0, 0.0));
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        let wave = |f: &Filament| f.ringing() * (f.shape() * 0.6).sin();
        f.pluck(v3(0.1, 0.0, 0.0), V3::ZERO);
        for _ in 0..5 {
            f.step(1.0 / 60.0);
        }
        let was = wave(&f);
        assert!(was.abs() > 0.05, "it rings: {was}");
        f.pluck(v3(2.5, 0.0, 0.0), V3::ZERO);
        assert!((wave(&f) - was).abs() < 1e-4, "the swing snapped: {was} -> {}", wave(&f));
        f.step(1.0 / 60.0);
        assert!((wave(&f) - was).abs() < 0.08, "the swing jumped: {was} -> {}", wave(&f));
    }
}
