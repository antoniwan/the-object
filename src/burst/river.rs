//! What the twelve become if they are brought together anywhere but the middle.
//! Carried home one by one they mend; crowded close somewhere else they forget
//! they were twelve, melt into the ground, and run out of it as a river.
//!
//! - touch the river and a column stands up out of it, solid or burning, holds
//!   for a moment, and falls back in; drag along it and a row stands up
//! - raise enough of them at once and they stop falling. The river stops running
//!   and beats, in the rhythm the columns were raised in, and every column beats
//!   with it. Then they all go down, the river drains, and the body gathers
//!   itself out of it
//!
//! The river is a winding band through the point where they met. The columns stand
//! on a lattice that follows its winding: `ROWS` along it, `LANES` across. The
//! shader rebuilds the same band and lattice from `at`, `angle` and `reach`, so
//! the two must agree (see riverLocal and pillars in shader.frag).

use crate::math::*;

/// The color of it, and of the body that comes back out of it.
pub const BLOOD: V3 = v3(0.42, 0.02, 0.05);

pub const ROWS: usize = 24;
pub const LANES: usize = 3;
pub const SLOTS: usize = ROWS * LANES;
/// Distance between columns, along the river and across it.
pub const GAP: f32 = 1.6;
/// All twelve within this distance of where they crowd together, and they run.
const GROUP_RADIUS: f32 = 7.0;
/// Columns standing at once that fill the view.
const FILL: usize = 30;
/// How far the river runs either way from where it rose.
const LENGTH: f32 = 36.0;
const SPREAD_SECS: f32 = 5.0;
const MELT_SECS: f32 = 2.2;
const DRAIN_SECS: f32 = 2.4;
const RISE_SECS: f32 = 0.45;
const STAND_SECS: f32 = 5.5;
const FALL_SECS: f32 = 0.6;
const BEATS: u32 = 7;
/// How much taller every column stands on a beat.
const SWELL: f32 = 0.16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// They melted together into the river.
    Bled,
    /// A column stood up for the first time.
    Raised,
    /// Enough of them stood at once; it starts to beat.
    Filled,
    Beat,
    /// The last beat: the columns go down and the river drains.
    Pulsed,
}

#[derive(Clone, Copy, Debug)]
struct Column {
    /// Seconds since it was raised; below zero it is down.
    t: f32,
}

impl Default for Column {
    fn default() -> Self {
        Column { t: -1.0 }
    }
}

impl Column {
    /// Up, or on its way up: not down and not yet falling.
    fn standing(self) -> bool {
        self.t >= 0.0 && self.t < RISE_SECS + STAND_SECS
    }
}

pub struct River {
    pub active: bool,
    /// Where it rose (x, z), and which way it runs.
    pub at: (f32, f32),
    pub angle: f32,
    /// 0..1 how much river there is: the pieces melting in, or it draining away.
    pub amount: f32,
    draining: bool,
    age: f32,
    /// How far it has flowed, for the shader to move the surface by.
    pub flow: f32,
    cols: [Column; SLOTS],
    /// Where a column fell back in (x, z), and how long ago.
    pub splashes: Vec<(f32, f32, f32)>,
    /// Beating: seconds per beat and seconds since it started.
    pulse: Option<(f32, f32)>,
    taps: Vec<f32>,
    raised_once: bool,
    clock: f32,
    events: Vec<Event>,
}

impl Default for River {
    fn default() -> Self {
        River {
            active: false,
            at: (0.0, 0.0),
            angle: 0.0,
            amount: 0.0,
            draining: false,
            age: 0.0,
            flow: 0.0,
            cols: [Column::default(); SLOTS],
            splashes: Vec::new(),
            pulse: None,
            taps: Vec::new(),
            raised_once: false,
            clock: 0.0,
            events: Vec::new(),
        }
    }
}

/// How far the river's middle winds off a straight line, `s` along it.
fn meander(s: f32) -> f32 {
    2.2 * (0.21 * s).sin() + 1.1 * (0.47 * s + 1.3).sin()
}

/// Half the river's width, a distance `s` along it.
fn half_width(s: f32) -> f32 {
    3.2 + 0.6 * (0.33 * s + 0.4).sin()
}

/// How tall column `slot` stands, in world units.
pub fn tall(slot: usize) -> f32 {
    let h = hash(slot as f32 * 12.9898).abs();
    2.6 + 4.2 * h
}

/// One heartbeat: a strong beat and a weaker one just after, `t` seconds into it.
fn heartbeat(t: f32) -> f32 {
    let thump = |t: f32| if t < 0.0 { 0.0 } else { (t * 40.0).min(1.0) * (-t * 9.0).exp() };
    thump(t) + 0.6 * thump(t - 0.2)
}

impl River {
    /// Do the pieces at `homes` stand close enough together to run? Their centroid if so.
    pub fn crowded(homes: &[V3]) -> Option<V3> {
        let n = homes.len() as f32;
        let c = homes.iter().fold(V3::ZERO, |a, &h| a + h) * (1.0 / n);
        homes.iter().all(|h| (h.x - c.x).hypot(h.z - c.z) < GROUP_RADIUS).then_some(c)
    }

    /// The pieces have crowded together at `at`; it runs across the view from there.
    pub fn begin(&mut self, at: V3) {
        let out = if at.x.hypot(at.z) > 0.5 { at.z.atan2(at.x) } else { 0.0 };
        *self = River { active: true, at: (at.x, at.z), angle: out + std::f32::consts::FRAC_PI_2, ..River::default() };
        self.events.push(Event::Bled);
    }

    /// How far it reaches either way.
    pub fn reach(&self) -> f32 {
        LENGTH * smooth01(self.age / SPREAD_SECS).max(0.08)
    }

    fn dir(&self) -> (f32, f32) {
        (self.angle.cos(), self.angle.sin())
    }

    /// A point on the floor as (along the river, across from its winding middle).
    pub fn local(&self, x: f32, z: f32) -> (f32, f32) {
        let (dx, dz) = (x - self.at.0, z - self.at.1);
        let (cx, cz) = self.dir();
        let s = dx * cx + dz * cz;
        (s, -dx * cz + dz * cx - meander(s))
    }

    /// The other way round: where on the floor (x, z) is `s` along and `u` across.
    pub fn world(&self, s: f32, u: f32) -> (f32, f32) {
        let (cx, cz) = self.dir();
        let u = u + meander(s);
        (self.at.0 + cx * s - cz * u, self.at.1 + cz * s + cx * u)
    }

    /// Whether (x, z) on the floor is in the river and it can be touched.
    pub fn contains(&self, x: f32, z: f32) -> bool {
        if !self.active || self.draining || self.amount < 0.5 {
            return false;
        }
        let (s, u) = self.local(x, z);
        s.abs() < self.reach() && u.abs() < half_width(s)
    }

    /// Which column stands at (x, z), if the river is there.
    pub fn slot_at(&self, x: f32, z: f32) -> Option<usize> {
        if !self.contains(x, z) {
            return None;
        }
        let (s, u) = self.local(x, z);
        let k = (s / GAP).floor() + (ROWS / 2) as f32;
        if !(0.0..ROWS as f32).contains(&k) {
            return None;
        }
        let l = (u / GAP).round().clamp(-1.0, 1.0);
        Some(k as usize * LANES + (l + 1.0) as usize)
    }

    /// Where column `slot` stands (x, z).
    pub fn slot_pos(&self, slot: usize) -> (f32, f32) {
        let (k, l) = (slot / LANES, slot % LANES);
        self.world((k as f32 - ROWS as f32 / 2.0 + 0.5) * GAP, (l as f32 - 1.0) * GAP)
    }

    pub fn standing_count(&self) -> usize {
        self.cols.iter().filter(|c| c.standing()).count()
    }

    /// Stand column `slot` up. `tapped`: a fresh press, which counts toward the rhythm.
    /// Returns true if it rose (it was down, or falling).
    pub fn raise(&mut self, slot: usize, tapped: bool) -> bool {
        if self.pulse.is_some() || !self.active || self.draining {
            return false;
        }
        if tapped {
            self.taps.push(self.clock);
            if self.taps.len() > 6 {
                self.taps.remove(0);
            }
        }
        let c = &mut self.cols[slot];
        if c.standing() {
            // already up: it stands a little longer
            c.t = c.t.min(RISE_SECS);
            return false;
        }
        c.t = 0.0;
        if !self.raised_once {
            self.raised_once = true;
            self.events.push(Event::Raised);
        }
        if self.standing_count() >= FILL {
            self.pulse = Some((self.rhythm(), 0.0));
            for c in self.cols.iter_mut() {
                if c.standing() {
                    c.t = c.t.min(RISE_SECS);
                }
            }
            self.events.push(Event::Filled);
        }
        true
    }

    /// Seconds between the taps that raised the columns, as one beat.
    pub fn rhythm(&self) -> f32 {
        let gaps: Vec<f32> = self.taps.windows(2).map(|w| w[1] - w[0]).filter(|&g| g > 0.05 && g < 2.0).collect();
        if gaps.is_empty() {
            return 0.85;
        }
        (gaps.iter().sum::<f32>() / gaps.len() as f32).clamp(0.5, 1.2)
    }

    pub fn beating(&self) -> bool {
        self.pulse.is_some()
    }

    /// 0..1, the heartbeat while it beats.
    pub fn beat(&self) -> f32 {
        self.pulse.map_or(0.0, |(period, t)| heartbeat(t % period))
    }

    /// How tall column `slot` stands out of the river now.
    pub fn height(&self, slot: usize) -> f32 {
        let t = self.cols[slot].t;
        if t < 0.0 {
            return 0.0;
        }
        let h = tall(slot);
        let up = if t < RISE_SECS {
            // shoots up past where it stops, and settles back
            let x = t / RISE_SECS;
            1.0 + 0.12 * (x * std::f32::consts::PI).sin() - (1.0 - x).powi(3)
        } else if t < RISE_SECS + STAND_SECS {
            1.0
        } else {
            let x = ((t - RISE_SECS - STAND_SECS) / FALL_SECS).min(1.0);
            1.0 - x * x
        };
        h * up.max(0.0) * (1.0 + SWELL * self.beat())
    }

    /// How many rows from row `k` the nearest row with a column standing is, ahead and
    /// behind (31: none), as `ahead + 32 * behind`, so the shader can step over empty
    /// stretches of the river.
    pub fn rows_to_standing(&self, k: usize) -> f32 {
        let stands = |j: usize| (0..LANES).any(|l| self.height(j * LANES + l) > 0.0);
        let ahead = (k + 1..ROWS).find(|&j| stands(j)).map_or(31, |j| j - k);
        let behind = (0..k).rev().find(|&j| stands(j)).map_or(31, |j| k - j);
        (ahead + 32 * behind) as f32
    }

    /// The tallest column standing, so the shader can skip them all when there are none.
    pub fn tallest(&self) -> f32 {
        (0..SLOTS).map(|i| self.height(i)).fold(0.0, f32::max)
    }

    /// Let it go: every column falls and the river drains away.
    pub fn drain(&mut self) {
        if !self.active || self.draining {
            return;
        }
        self.draining = true;
        self.pulse = None;
        for c in self.cols.iter_mut() {
            if c.standing() {
                c.t = RISE_SECS + STAND_SECS;
            }
        }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.active {
            return;
        }
        self.clock += dt;
        self.age += dt;
        if self.draining {
            self.amount = (self.amount - dt / DRAIN_SECS).max(0.0);
        } else {
            self.amount = (self.amount + dt / MELT_SECS).min(1.0);
        }
        // it stops running while it beats
        self.flow += dt * if self.pulse.is_some() { 0.0 } else { 1.0 };

        let mut last_beat = false;
        if let Some((period, t)) = self.pulse.as_mut() {
            let before = (*t / *period).floor();
            *t += dt;
            let now = (*t / *period).floor();
            if now > before {
                last_beat = now as u32 >= BEATS;
                self.events.push(if last_beat { Event::Pulsed } else { Event::Beat });
            }
        }
        if last_beat {
            self.drain();
        }
        let beating = self.pulse.is_some();
        let mut fell = Vec::new();
        for (slot, c) in self.cols.iter_mut().enumerate() {
            if c.t < 0.0 {
                continue;
            }
            // while it beats they hold
            if beating && c.t >= RISE_SECS && c.t < RISE_SECS + STAND_SECS {
                continue;
            }
            c.t += dt;
            if c.t >= RISE_SECS + STAND_SECS + FALL_SECS {
                c.t = -1.0;
                fell.push(slot);
            }
        }
        for slot in fell {
            let (x, z) = self.slot_pos(slot);
            self.splashes.push((x, z, 0.0));
        }
        for s in self.splashes.iter_mut() {
            s.2 += dt;
        }
        self.splashes.retain(|s| s.2 < 3.0);
        if self.splashes.len() > 6 {
            let extra = self.splashes.len() - 6;
            self.splashes.drain(..extra);
        }
        if self.draining && self.amount <= 0.0 {
            let events = std::mem::take(&mut self.events);
            *self = River { events, ..River::default() };
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(r: &mut River, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            r.step(1.0 / 60.0);
        }
    }

    fn running() -> River {
        let mut r = River::default();
        r.begin(v3(8.0, 0.0, 3.0));
        run(&mut r, SPREAD_SECS);
        assert_eq!(r.drain_events(), vec![Event::Bled]);
        r
    }

    #[test]
    fn crowded_together_anywhere_they_run() {
        let close: Vec<V3> = (0..12).map(|i| v3(10.0 + (i % 4) as f32 * 2.5, 0.0, (i / 4) as f32 * 2.5)).collect();
        assert!(River::crowded(&close).is_some());
        let mut apart = close.clone();
        apart[3] = v3(-10.0, 0.0, 0.0);
        assert!(River::crowded(&apart).is_none());
    }

    #[test]
    fn the_lattice_and_the_floor_agree() {
        let r = running();
        for slot in [0, 1, 2, 30, 37, SLOTS - 1] {
            let (x, z) = r.slot_pos(slot);
            assert_eq!(r.slot_at(x, z), Some(slot));
            let (s, u) = r.local(x, z);
            let back = r.world(s, u);
            assert!((back.0 - x).abs() < 1e-3 && (back.1 - z).abs() < 1e-3);
        }
        let (x, z) = r.world(0.0, half_width(0.0) + 1.0);
        assert!(r.slot_at(x, z).is_none(), "the bank is not the river");
    }

    #[test]
    fn a_column_stands_then_falls_back_in() {
        let mut r = running();
        assert!(r.raise(37, true));
        assert_eq!(r.drain_events(), vec![Event::Raised]);
        run(&mut r, RISE_SECS + 0.1);
        assert!((r.height(37) - tall(37)).abs() < 0.3);
        run(&mut r, STAND_SECS + FALL_SECS);
        assert_eq!(r.height(37), 0.0);
        assert_eq!(r.splashes.len(), 1);
    }

    #[test]
    fn filled_it_beats_in_your_rhythm_then_drains() {
        let mut r = running();
        for slot in 0..FILL {
            r.raise(slot, true);
            run(&mut r, 0.1);
        }
        assert!(r.beating());
        assert!((r.rhythm() - 0.5).abs() < 0.05, "taps a tenth of a second apart beat as fast as it will go");
        let ev = r.drain_events();
        assert!(ev.contains(&Event::Filled));
        // the first one raised would be falling by now; while it beats, they hold
        run(&mut r, 3.3);
        assert!(r.height(0) > tall(0) * 0.95);
        let rest = BEATS as f32 * r.rhythm() - 3.3 + 0.1;
        run(&mut r, rest);
        let ev = r.drain_events();
        assert_eq!(ev.iter().filter(|e| **e == Event::Beat).count(), BEATS as usize - 1);
        assert!(ev.contains(&Event::Pulsed));
        run(&mut r, DRAIN_SECS + FALL_SECS);
        assert!(!r.active && r.tallest() == 0.0);
    }
}
