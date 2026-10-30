//! Pinch the flower and it does not fold: it lets go of itself. The petals go to grain,
//! the grain to a cloud, and the cloud comes down as dust over the floor in drifts.
//! Drag a finger through the dust and it is swept along like a broom pushes it; drifts
//! pushed together become one. Gather nearly all of it into one place and it heaps itself
//! up: a mound standing on the ground, which settles and firms and then breathes, slowly,
//! where it stands. Tap the mound and it answers low. Press and hold it and it comes back
//! up out of itself as the liquid, floating again, and the mound goes into it.
//! Left alone long enough, the dust drifts together on its own. Blown over, it drifts away
//! from you along the floor (gust.rs). Once the mound has
//! settled something takes root in it and grows as far as it is tended (growth.rs).
//!
//! Like the other forms, `Phase` only decides which 0..1 values move; the shader mixes
//! them: the body crumbling (`crumble_amount`), the cloud (`cloud`), the dust landing (`settle`),
//! and the mound (`heap`, `firm`).

use super::growth::Growth;
use super::Event;
use crate::math::*;

pub const CLUMPS: usize = 8;
/// Crumbling, the cloud, and the dust coming down.
const SCATTER_SECS: f32 = 3.0;
/// The last of it drawn in, and the heap firming.
const HEAP_SECS: f32 = 3.2;
/// Coming back up out of the mound.
const RISE_SECS: f32 = 2.6;
/// This much of it in one drift and it heaps itself.
const HEAP_AT: f32 = 0.9;
/// Pressing the mound this long brings it back up.
pub const RISE_HOLD_SECS: f32 = 1.2;
/// Once let go, the light the pressed mound holds goes out over this long.
const PRESS_FADE_SECS: f32 = 0.25;
/// Untouched this long, the dust starts to drift together by itself.
const GATHER_SELF_AFTER: f32 = 40.0;
/// How wide a finger sweeps, in world units.
const BRUSH: f32 = 0.6;
/// The dust is kept within this far of the middle of the room, where it can be seen.
const REACH: f32 = 4.2;
const FLOOR_Y: f32 = crate::floor::fusion::FLOOR_Y;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Clump {
    pub x: f32,
    pub z: f32,
    /// Its share of all the dust, 0..1; 0 is no drift.
    pub amount: f32,
}

impl Clump {
    /// How wide the drift lies; more dust lies wider.
    pub fn radius(&self) -> f32 {
        0.45 + 1.0 * self.amount.sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Scattering { t: f32 },
    Dust,
    Heaping { t: f32 },
    Mound,
    Rising { t: f32 },
}

pub struct Dust {
    pub phase: Phase,
    pub clumps: [Clump; CLUMPS],
    /// Seconds since a finger last touched the dust.
    idle: f32,
    swept: bool,
    /// The mound answering a tap, 0..1.
    pub thud: f32,
    /// How long the mound has been pressed, 0..1 of what it takes to come back up, as it is lit:
    /// it follows a finger at once, and once let go fades over `PRESS_FADE_SECS`.
    pub press: f32,
    /// How long the finger on the mound has held it, 0..1; 0 when none does (`hold`).
    pressing: f32,
    /// What grows out of the mound once it has settled.
    pub growth: Growth,
    /// The mound as it stands, following the fullest drift (`heap`).
    mound: Mound,
    pub(super) events: Vec<Event>,
}

/// Which drift the mound stands on, where, and how far it has grown, 0..1. When another drift
/// becomes the fullest, the mound sinks where it is before it rises there, so it never jumps.
#[derive(Clone, Copy, Debug, Default)]
struct Mound {
    on: usize,
    x: f32,
    z: f32,
    grown: f32,
}

/// How fast the mound can grow or sink, of its full height a second.
const MOUND_RATE: f32 = 2.0;
/// How fast a smaller drift lying over a larger is drawn onto it, in world units a second.
const MERGE_PULL: f32 = 1.5;
/// How much dust goes across from the smaller to the larger, of all of it a second, once they lie as one.
const MERGE_RATE: f32 = 0.6;

impl Default for Dust {
    fn default() -> Self {
        Dust {
            phase: Phase::Whole,
            clumps: [Clump::default(); CLUMPS],
            idle: 0.0,
            swept: false,
            thud: 0.0,
            press: 0.0,
            pressing: 0.0,
            growth: Growth::default(),
            mound: Mound::default(),
            events: Vec::new(),
        }
    }
}

impl Dust {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// Pinched while it is a flower: it comes apart over the floor under (x, z).
    /// `turn` sets where the drifts fall, so no two times are the same.
    pub fn crumble(&mut self, x: f32, z: f32, turn: f32) -> bool {
        if !self.whole() {
            return false;
        }
        let mut total = 0.0;
        for (i, c) in self.clumps.iter_mut().enumerate() {
            // a golden-angle spiral, so the drifts lie apart without a grid
            let a = turn + i as f32 * 2.399_963;
            let r = 0.9 + 2.6 * ((i as f32 + 0.5) / CLUMPS as f32).sqrt();
            c.x = x + a.cos() * r;
            c.z = z + a.sin() * r;
            c.amount = 0.7 + 0.6 * (i as f32 * 12.9898).sin().abs();
            total += c.amount;
        }
        for c in &mut self.clumps {
            c.amount /= total;
            clamp_reach(c);
        }
        self.phase = Phase::Scattering { t: 0.0 };
        self.idle = 0.0;
        self.swept = false;
        self.events.push(Event::Crumbled);
        true
    }

    /// How far the camera has stepped back to look down on the floor, 0..1.
    pub fn view(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Scattering { t } => smoothstep(0.0, SCATTER_SECS, t),
            Phase::Rising { t } => 1.0 - smoothstep(RISE_SECS * 0.3, RISE_SECS, t),
            _ => 1.0,
        }
    }

    /// How far the body has come apart: 0 whole, 1 nothing left of it.
    pub fn crumble_amount(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Scattering { t } => smoothstep(0.0, SCATTER_SECS * 0.45, t),
            Phase::Rising { t } => 1.0 - smoothstep(RISE_SECS * 0.15, RISE_SECS, t),
            _ => 1.0,
        }
    }

    /// Whether there is no body left to see or touch.
    pub fn body_gone(&self) -> bool {
        self.crumble_amount() >= 1.0
    }

    /// The cloud between the body and the floor: how much of it is in the air, and how far it has fallen.
    pub fn cloud(&self) -> (f32, f32) {
        match self.phase {
            Phase::Scattering { t } => {
                let k = t / SCATTER_SECS;
                (smoothstep(0.0, 0.3, k) * (1.0 - smoothstep(0.55, 1.0, k)), smoothstep(0.15, 0.95, k))
            }
            _ => (0.0, 0.0),
        }
    }

    /// How much of the dust is down on the floor, 0..1.
    pub fn settle(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Scattering { t } => smoothstep(SCATTER_SECS * 0.35, SCATTER_SECS, t),
            _ => 1.0,
        }
    }

    /// The fullest drift.
    pub fn biggest(&self) -> usize {
        let mut best = 0;
        for (i, c) in self.clumps.iter().enumerate() {
            if c.amount > self.clumps[best].amount {
                best = i;
            }
        }
        best
    }

    /// The mound: where it stands (x, z), how wide, how high. It rises out of the
    /// fullest drift as the dust is swept into it, so it never appears from nowhere.
    pub fn heap(&self) -> (f32, f32, f32, f32) {
        let m = self.mound;
        (m.x, m.z, 0.6 + 1.0 * m.grown.sqrt(), 1.0 * m.grown)
    }

    /// How far the mound would have grown on drift `k`, by now.
    fn grown(&self, k: usize) -> f32 {
        match self.phase {
            Phase::Whole | Phase::Scattering { .. } => 0.0,
            Phase::Dust => 0.7 * smoothstep(0.2, HEAP_AT, self.clumps[k].amount),
            Phase::Heaping { t } => 0.7 + 0.3 * smoothstep(0.0, HEAP_SECS, t),
            Phase::Mound => 1.0,
            Phase::Rising { t } => 1.0 - smoothstep(0.0, RISE_SECS * 0.85, t),
        }
    }

    /// The mound follows the fullest drift; another drift fuller, it sinks first, then moves.
    fn step_mound(&mut self, dt: f32) {
        let k = self.biggest();
        // Not standing at all, it is wherever the fullest drift is.
        if self.mound.grown <= 0.0 {
            self.mound = Mound { on: k, x: self.clumps[k].x, z: self.clumps[k].z, grown: 0.0 };
        }
        let goal = if self.mound.on == k { self.grown(k) } else { 0.0 };
        let c = self.clumps[self.mound.on];
        let m = &mut self.mound;
        m.grown += (goal - m.grown).clamp(-MOUND_RATE * dt, MOUND_RATE * dt);
        m.x = ease(m.x, c.x, -12.0 * dt);
        m.z = ease(m.z, c.z, -12.0 * dt);
    }

    /// How settled the mound is: grains at 0, one firm thing at 1.
    pub fn firm(&self) -> f32 {
        match self.phase {
            Phase::Heaping { t } => smoothstep(HEAP_SECS * 0.3, HEAP_SECS, t),
            Phase::Mound | Phase::Rising { .. } => 1.0,
            _ => 0.0,
        }
    }

    /// Whether (x, z) on the floor is on the mound.
    pub fn on_mound(&self, x: f32, z: f32) -> bool {
        let (hx, hz, r, h) = self.heap();
        h > 0.3 && (x - hx).hypot(z - hz) < r * 1.1
    }

    /// Where the body should be while there is no body: in the mound, and on the way up out of it.
    /// None once it is itself again.
    pub fn body_at(&self, home: V3) -> Option<V3> {
        let (x, z, _, h) = self.heap();
        let low = v3(x, FLOOR_Y + h * 0.5, z);
        match self.phase {
            Phase::Whole | Phase::Scattering { .. } => None,
            Phase::Rising { t } => Some(low.lerp(home, smoothstep(0.0, RISE_SECS, t))),
            _ => Some(low),
        }
    }

    /// A finger dragged over the floor from `a` to `b` (x, z): every drift it passes through
    /// goes along with it, and drifts pushed into each other become one. True if any moved.
    pub fn sweep(&mut self, a: (f32, f32), b: (f32, f32)) -> bool {
        if self.phase != Phase::Dust {
            return false;
        }
        self.idle = 0.0;
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dz * dz;
        if len2 < 1e-8 {
            return false;
        }
        let mut moved = false;
        for c in self.clumps.iter_mut().filter(|c| c.amount > 0.0) {
            // nearest point of the stroke to the drift
            let s = (((c.x - a.0) * dx + (c.z - a.1) * dz) / len2).clamp(0.0, 1.0);
            let (px, pz) = (a.0 + dx * s, a.1 + dz * s);
            let d = (c.x - px).hypot(c.z - pz);
            let reach = BRUSH + 0.5 * c.radius();
            if d < reach {
                // pushed the rest of the way the stroke went; heavier drifts lag a little
                let push = (1.0 - s) * (1.0 - 0.35 * c.amount) * smoothstep(reach, reach * 0.4, d).max(0.35);
                c.x += dx * push;
                c.z += dz * push;
                clamp_reach(c);
                moved = true;
            }
        }
        if moved && !self.swept {
            self.swept = true;
            self.events.push(Event::Swept);
        }
        moved
    }

    /// Breath over the floor moves every drift by (dx, dz), the lighter ones a little further,
    /// none past where it can be seen; ones blown into each other run together as always.
    /// True if any moved.
    pub fn blow(&mut self, dx: f32, dz: f32) -> bool {
        if self.phase != Phase::Dust || dx.hypot(dz) < 1e-7 {
            return false;
        }
        self.idle = 0.0;
        for c in self.clumps.iter_mut().filter(|c| c.amount > 0.0) {
            let k = 1.0 - 0.35 * c.amount;
            c.x += dx * k;
            c.z += dz * k;
            clamp_reach(c);
        }
        true
    }

    /// Drifts lying over each other run together over time: the smaller is drawn onto the
    /// larger and its dust goes across, faster the closer they lie, so neither jumps.
    fn merge(&mut self, dt: f32) {
        for i in 0..CLUMPS {
            for j in 0..CLUMPS {
                let (a, b) = (self.clumps[i], self.clumps[j]);
                if i == j
                    || a.amount <= 0.0
                    || b.amount <= 0.0
                    || a.amount < b.amount
                    || (a.amount == b.amount && i > j)
                {
                    continue;
                }
                let (ox, oz) = (a.x - b.x, a.z - b.z);
                let d = ox.hypot(oz);
                let near = 0.45 * (a.radius() + b.radius());
                if d >= near {
                    continue;
                }
                let pull = (MERGE_PULL * dt / d.max(1e-4)).min(1.0);
                let mut part = (MERGE_RATE * dt * (1.0 - d / near)).min(b.amount);
                // the last crumb goes across with the rest; a drift at 0 is gone
                if b.amount - part < 1e-3 {
                    part = b.amount;
                }
                let c = &mut self.clumps[j];
                c.x += ox * pull;
                c.z += oz * pull;
                c.amount -= part;
                self.clumps[i].amount += part;
            }
        }
    }

    /// A tap on the mound, or on what grows from it: it answers, and once it stands, it is
    /// tended. Returns how far it has grown, 0..1, for the pitch it answers in.
    pub fn tap(&mut self) -> Option<f32> {
        match self.phase {
            Phase::Heaping { .. } => {
                self.thud = 1.0;
                Some(0.0)
            }
            Phase::Mound => {
                self.thud = 1.0;
                Some(self.growth.tend())
            }
            _ => None,
        }
    }

    /// Whether the ray from `ro` along `rd` passes through what grows from the mound.
    pub fn touches_growth(&self, ro: V3, rd: V3) -> bool {
        let (x, z, _, h) = self.heap();
        if self.growth.shown < 0.3 || h < 0.3 {
            return false;
        }
        // the nearest the ray comes to the upright through the middle of the mound
        let a = v3(x, FLOOR_Y + h, z);
        let up = V3::UP;
        let w = ro - a;
        let (b, d, e) = (rd.dot(up), rd.dot(w), up.dot(w));
        let den = 1.0 - b * b;
        if den < 1e-4 {
            return false;
        }
        let s = ((b * d - e) / den).clamp(0.0, self.growth.reach());
        let t = (rd.dot(a + up * s - ro)).max(0.0);
        (ro + rd * t - (a + up * s)).len() < 0.2 + 0.8 * self.growth.spread()
    }

    /// Held long enough: it comes back up out of the mound.
    pub fn rise(&mut self) -> bool {
        if self.phase != Phase::Mound {
            return false;
        }
        self.phase = Phase::Rising { t: 0.0 };
        true
    }

    /// A finger on the mound for `held` of what it takes to come back up, or 0 once none is.
    pub fn hold(&mut self, held: f32) {
        self.pressing = held;
        self.press = self.press.max(held);
    }

    pub fn step(&mut self, dt: f32) {
        self.thud = fade(self.thud, -5.0 * dt);
        self.press = (self.press - dt / PRESS_FADE_SECS).max(self.pressing);
        self.idle += dt;
        self.phase = match self.phase {
            Phase::Whole => Phase::Whole,
            Phase::Scattering { t } if t + dt >= SCATTER_SECS => Phase::Dust,
            Phase::Scattering { t } => Phase::Scattering { t: t + dt },
            Phase::Dust => {
                if self.idle > GATHER_SELF_AFTER {
                    // left alone, it drifts toward its fullest part, a little faster the longer it waits
                    let pull = (0.15 + 0.01 * (self.idle - GATHER_SELF_AFTER)).min(0.6) * dt;
                    let k = self.biggest();
                    let (hx, hz) = (self.clumps[k].x, self.clumps[k].z);
                    for c in self.clumps.iter_mut() {
                        let (ox, oz) = (hx - c.x, hz - c.z);
                        let d = ox.hypot(oz).max(1e-4);
                        let step = (pull / d).min(1.0);
                        c.x += ox * step;
                        c.z += oz * step;
                    }
                }
                self.merge(dt);
                if self.clumps[self.biggest()].amount >= HEAP_AT {
                    self.events.push(Event::Heaped);
                    Phase::Heaping { t: 0.0 }
                } else {
                    Phase::Dust
                }
            }
            Phase::Heaping { t } => {
                // the last of it is drawn in, along the floor
                let k = self.biggest();
                let (hx, hz) = (self.clumps[k].x, self.clumps[k].z);
                let (was, now) = (smoothstep(0.0, HEAP_SECS * 0.6, t), smoothstep(0.0, HEAP_SECS * 0.6, t + dt));
                let share = if now >= 1.0 { 1.0 } else { ((now - was) / (1.0 - was).max(1e-4)).min(1.0) };
                let mut taken = 0.0;
                for (i, c) in self.clumps.iter_mut().enumerate() {
                    if i != k && c.amount > 0.0 {
                        let part = c.amount * share;
                        c.amount -= part;
                        taken += part;
                        c.x += (hx - c.x) * share;
                        c.z += (hz - c.z) * share;
                    }
                }
                self.clumps[k].amount += taken;
                if t + dt >= HEAP_SECS {
                    self.events.push(Event::Grounded);
                    Phase::Mound
                } else {
                    Phase::Heaping { t: t + dt }
                }
            }
            Phase::Mound => {
                self.growth.step(dt, &mut self.events);
                Phase::Mound
            }
            Phase::Rising { t } if t + dt >= RISE_SECS => {
                self.clumps = [Clump::default(); CLUMPS];
                self.growth = Growth::default();
                self.events.push(Event::Unearthed);
                Phase::Whole
            }
            Phase::Rising { t } => {
                self.growth.recede(dt);
                Phase::Rising { t: t + dt }
            }
        };
        self.step_mound(dt);
    }
}

fn clamp_reach(c: &mut Clump) {
    let d = c.x.hypot(c.z);
    if d > REACH {
        c.x *= REACH / d;
        c.z *= REACH / d;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(d: &mut Dust, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            d.step(1.0 / 60.0);
        }
    }

    fn total(d: &Dust) -> f32 {
        d.clumps.iter().map(|c| c.amount).sum()
    }

    #[test]
    fn it_crumbles_lands_and_sweeping_it_together_heaps_a_mound_that_rises_back() {
        let mut d = Dust::default();
        assert!(d.crumble(0.0, 0.0, 0.3));
        assert!(!d.crumble(0.0, 0.0, 0.3));
        run(&mut d, 3.5);
        assert_eq!(d.phase, Phase::Dust);
        assert!(d.body_gone());
        assert!((total(&d) - 1.0).abs() < 1e-4);
        // sweep every drift to one place, the way a broom would
        for _ in 0..40 {
            if d.phase != Phase::Dust {
                break;
            }
            for i in 0..CLUMPS {
                let c = d.clumps[i];
                if c.amount > 0.0 && c.x.hypot(c.z) > 0.2 {
                    let back = (c.x - c.x * 0.2, c.z - c.z * 0.2);
                    d.sweep((c.x * 1.2, c.z * 1.2), back);
                    d.sweep(back, (0.0, 0.0));
                }
            }
            d.step(1.0 / 60.0);
        }
        assert!(d.events.contains(&Event::Swept));
        assert!(matches!(d.phase, Phase::Heaping { .. }), "{:?} {:?}", d.phase, d.clumps);
        run(&mut d, 3.5);
        assert_eq!(d.phase, Phase::Mound);
        assert!((total(&d) - 1.0).abs() < 1e-3, "{}", total(&d));
        let (x, z, _, h) = d.heap();
        assert!(h > 0.9 && d.on_mound(x, z));
        assert!(d.tap().is_some() && d.thud > 0.9);
        assert!(d.rise());
        run(&mut d, 3.0);
        assert!(d.whole() && d.crumble_amount() == 0.0);
        assert!(d.events.contains(&Event::Unearthed));
    }

    /// The dust drawn at (x, z), summed the way the floor draws it.
    fn drawn(d: &Dust, x: f32, z: f32) -> f32 {
        d.clumps
            .iter()
            .filter(|c| c.amount > 0.0)
            .map(|c| {
                let (ox, oz) = (x - c.x, z - c.z);
                3.0 * c.amount * (-(ox * ox + oz * oz) / (c.radius() * c.radius())).exp()
            })
            .sum()
    }

    #[test]
    fn drifts_left_alone_run_together_without_a_jump() {
        let mut d = Dust { phase: Phase::Dust, idle: GATHER_SELF_AFTER + 1.0, ..Default::default() };
        d.clumps[0] = Clump { x: 0.0, z: 0.0, amount: 0.5 };
        d.clumps[1] = Clump { x: 2.5, z: 0.0, amount: 0.25 };
        d.clumps[2] = Clump { x: -2.5, z: 1.0, amount: 0.25 };
        let grid = |d: &Dust| {
            let mut v = Vec::new();
            for i in 0..60 {
                for j in 0..60 {
                    v.push(drawn(d, -4.0 + i as f32 * 0.14, -4.0 + j as f32 * 0.14));
                }
            }
            v
        };
        let mut was = grid(&d);
        let mut worst = 0.0f32;
        for _ in 0..600 {
            d.step(1.0 / 60.0);
            let now = grid(&d);
            worst = was.iter().zip(&now).map(|(a, b)| (a - b).abs()).fold(worst, f32::max);
            was = now;
        }
        assert!(d.phase != Phase::Dust, "they ran together and heaped: {:?}", d.clumps);
        assert!((total(&d) - 1.0).abs() < 1e-4);
        assert!(worst < 0.05, "the drawn dust changed by {worst} in a frame");
    }

    #[test]
    fn left_alone_the_dust_gathers_itself() {
        let mut d = Dust::default();
        d.crumble(0.5, -0.5, 1.0);
        run(&mut d, 3.5 + GATHER_SELF_AFTER + 40.0);
        assert!(matches!(d.phase, Phase::Heaping { .. } | Phase::Mound), "{:?}", d.clumps);
    }

    #[test]
    fn the_dust_stays_where_it_can_be_seen() {
        let mut d = Dust::default();
        d.crumble(0.0, 0.0, 0.0);
        run(&mut d, 3.5);
        for _ in 0..30 {
            d.sweep((0.0, 0.0), (6.0, 0.0));
        }
        assert!(d.clumps.iter().all(|c| c.x.hypot(c.z) <= REACH + 1e-3));
    }

    #[test]
    fn when_another_drift_becomes_the_fullest_the_mound_sinks_before_it_rises_there() {
        let mut d = Dust { phase: Phase::Dust, ..Default::default() };
        d.clumps[0] = Clump { x: -2.0, z: 0.0, amount: 0.45 };
        d.clumps[1] = Clump { x: 1.5, z: 0.0, amount: 0.30 };
        d.clumps[2] = Clump { x: 2.6, z: 0.0, amount: 0.25 };
        run(&mut d, 1.0);
        let mut was = d.heap();
        assert!(was.0 < -1.9 && was.3 > 0.2, "on the fullest: {was:?}");
        // the two smaller drifts swept together, they run into one, which becomes the fullest
        d.sweep((3.2, 0.0), (1.6, 0.0));
        for _ in 0..180 {
            d.step(1.0 / 60.0);
            let now = d.heap();
            assert!((now.3 - was.3).abs() < 0.05, "height jumped: {was:?} -> {now:?}");
            let moved = (now.0 - was.0).hypot(now.1 - was.1);
            assert!(moved < 0.05 || was.3 < 0.01, "moved while it stood: {was:?} -> {now:?}");
            was = now;
        }
        let k = d.biggest();
        assert_eq!(k, 1);
        assert!((was.0 - d.clumps[k].x).abs() < 1e-4 && was.3 > 0.3, "risen on the new one: {was:?}");
    }
}
