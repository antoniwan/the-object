//! Poke it to death and it bursts. Every poke leaves a wound that heals slowly;
//! twelve open at once and it can't hold any more. It swells, pops, and its
//! liquid flies out as twelve pieces that land far apart and harden into
//! twelve triangles, each its own shape, color and consistency, standing in
//! a white landscape. Poke a triangle and it answers in its own material.
//! Hold still on the empty ground and the pieces come home.
//!
//! Like meiosis.rs and fusion.rs, `Phase` only decides which continuous values
//! move; the shader blends them, so the burst and the flight never jump.

use crate::math::*;

pub const SHARDS: usize = 12;
/// Wounds open at once that kill it.
pub const FATAL_WOUNDS: f32 = 12.0;
/// Seconds for one wound to heal, once it has been left alone for `HEAL_AFTER_SECS`.
const HEAL_SECS: f32 = 5.0;
const HEAL_AFTER_SECS: f32 = 2.5;
/// The swell before the pop, then the flight.
const SWELL_SECS: f32 = 0.35;
const FLIGHT_SECS: f32 = 2.4;
const GATHER_SECS: f32 = 2.8;
/// Holding still on empty ground this long calls the pieces home.
pub const GATHER_HOLD_SECS: f32 = 1.4;
/// Camera distance from which the whole landscape can be seen.
pub const LANDSCAPE_DIST: f32 = 21.0;
const FLOOR_Y: f32 = crate::fusion::FLOOR_Y;

/// How each piece answers a poke. Index-matched with the materials in the shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Consistency {
    Milk, Chrome, Glass, Jelly, Stone, Honey, Velvet, Wax, Ice, Rubber, Obsidian, Moss,
}

pub const CONSISTENCIES: [Consistency; SHARDS] = [
    Consistency::Milk, Consistency::Chrome, Consistency::Glass, Consistency::Jelly,
    Consistency::Stone, Consistency::Honey, Consistency::Velvet, Consistency::Wax,
    Consistency::Ice, Consistency::Rubber, Consistency::Obsidian, Consistency::Moss,
];

impl Consistency {
    /// How far a poke makes it wobble, and how fast the wobble dies.
    pub fn give(self) -> (f32, f32) {
        match self {
            Consistency::Jelly => (1.0, 1.2),
            Consistency::Milk | Consistency::Honey => (0.7, 1.8),
            Consistency::Rubber => (0.8, 3.0),
            Consistency::Wax | Consistency::Velvet | Consistency::Moss => (0.25, 4.0),
            Consistency::Chrome | Consistency::Glass | Consistency::Ice => (0.12, 7.0),
            Consistency::Stone | Consistency::Obsidian => (0.04, 10.0),
        }
    }
}

/// Height of each piece once landed, in world units. Shapes live in the shader.
const SIZES: [f32; SHARDS] = [3.6, 4.8, 3.1, 2.5, 5.9, 2.8, 4.1, 2.2, 5.3, 2.7, 6.4, 3.4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    /// `t` seconds since the last poke burst it.
    Bursting { t: f32 },
    Landscape,
    Gathering { t: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Burst,
    Gathered,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Shard {
    /// Where it stands once landed: a point on the floor.
    pub home: V3,
    pub yaw: f32,
    pub size: f32,
    /// Wobble left from the last poke, 0..1.
    pub wobble: f32,
}

pub struct Burst {
    pub phase: Phase,
    /// Open wounds; fractional while healing.
    pub wounds: f32,
    since_wound: f32,
    pub shards: [Shard; SHARDS],
    events: Vec<Event>,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Default for Burst {
    fn default() -> Self {
        let mut shards = [Shard::default(); SHARDS];
        for (i, s) in shards.iter_mut().enumerate() {
            // A golden-angle spiral spreads them evenly without a grid.
            let a = i as f32 * 2.399_963 + 0.6;
            let r = 6.0 + i as f32 * 1.05;
            s.home = v3(a.cos() * r, FLOOR_Y, a.sin() * r);
            s.yaw = a * 1.7 + i as f32;
            s.size = SIZES[i];
        }
        Burst { phase: Phase::Whole, wounds: 0.0, since_wound: 0.0, shards, events: Vec::new() }
    }
}

impl Burst {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// 0..1 progress of the pieces from the body out to the landscape.
    pub fn spread(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Bursting { t } => smoothstep(SWELL_SECS, SWELL_SECS + FLIGHT_SECS, t),
            Phase::Landscape => 1.0,
            Phase::Gathering { t } => 1.0 - smoothstep(0.0, GATHER_SECS, t),
        }
    }

    /// How much the body has swollen before it pops, 0..1; 0 again after.
    pub fn swell(&self) -> f32 {
        match self.phase {
            Phase::Whole => (self.wounds / FATAL_WOUNDS) * 0.25,
            Phase::Bursting { t } if t < SWELL_SECS => 0.25 + 0.75 * smoothstep(0.0, SWELL_SECS, t),
            _ => 0.0,
        }
    }

    /// Whether the body itself is gone (it has popped and not re-formed).
    pub fn body_gone(&self) -> bool {
        match self.phase {
            Phase::Whole => false,
            Phase::Bursting { t } => t >= SWELL_SECS,
            Phase::Landscape => true,
            Phase::Gathering { t } => t < GATHER_SECS * 0.92,
        }
    }

    /// The white flash of the pop.
    pub fn flash(&self) -> f32 {
        match self.phase {
            Phase::Bursting { t } if t >= SWELL_SECS => (-(t - SWELL_SECS) * 6.0).exp(),
            _ => 0.0,
        }
    }

    /// Where piece `i` is now, and how far it has turned from blob into triangle (0..1).
    pub fn shard_pose(&self, i: usize) -> (V3, f32) {
        let s = self.spread();
        let home = self.shards[i].home;
        // Out along an arc: rise, then come down where it will stand.
        let flat = V3::ZERO.lerp(home, s);
        let arc = (s * std::f32::consts::PI).sin() * (3.0 + home.len() * 0.35);
        let pos = v3(flat.x, flat.y + arc, flat.z);
        (pos, smoothstep(0.55, 1.0, s))
    }

    /// One poke on the body. Returns true if that one killed it.
    pub fn wound(&mut self) -> bool {
        if !self.whole() {
            return false;
        }
        self.wounds += 1.0;
        self.since_wound = 0.0;
        if self.wounds >= FATAL_WOUNDS {
            self.phase = Phase::Bursting { t: 0.0 };
            self.wounds = 0.0;
            self.events.push(Event::Burst);
            return true;
        }
        false
    }

    pub fn poke_shard(&mut self, i: usize) {
        self.shards[i].wobble = 1.0;
    }

    pub fn gather(&mut self) -> bool {
        if self.phase != Phase::Landscape {
            return false;
        }
        self.phase = Phase::Gathering { t: 0.0 };
        true
    }

    pub fn step(&mut self, dt: f32) {
        // Keep poking and nothing heals; leave it alone and it slowly does.
        self.since_wound += dt;
        if self.since_wound > HEAL_AFTER_SECS {
            self.wounds = (self.wounds - dt / HEAL_SECS).max(0.0);
        }
        self.phase = match self.phase {
            Phase::Bursting { t } if t + dt >= SWELL_SECS + FLIGHT_SECS => Phase::Landscape,
            Phase::Bursting { t } => Phase::Bursting { t: t + dt },
            Phase::Gathering { t } if t + dt >= GATHER_SECS => {
                self.events.push(Event::Gathered);
                Phase::Whole
            }
            Phase::Gathering { t } => Phase::Gathering { t: t + dt },
            p => p,
        };
        for (i, s) in self.shards.iter_mut().enumerate() {
            let (_, decay) = CONSISTENCIES[i].give();
            s.wobble *= (-decay * dt).exp();
        }
    }

    /// Which landed piece a ray hits first, by a bounding sphere around each.
    pub fn pick(&self, ro: V3, rd: V3) -> Option<usize> {
        if self.phase != Phase::Landscape {
            return None;
        }
        let mut best: Option<(f32, usize)> = None;
        for (i, s) in self.shards.iter().enumerate() {
            let c = s.home + V3::UP * (s.size * 0.45);
            let oc = ro - c;
            let b = oc.dot(rd);
            let disc = b * b - (oc.dot(oc) - (s.size * 0.5).powi(2));
            if disc < 0.0 {
                continue;
            }
            let t = -b - disc.sqrt();
            if t > 0.0 && best.map_or(true, |(bt, _)| t < bt) {
                best = Some((t, i));
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(b: &mut Burst, secs: f32) {
        for _ in 0..(secs * 120.0) as usize {
            b.step(1.0 / 120.0);
        }
    }

    #[test]
    fn eleven_quick_pokes_hurt_and_the_twelfth_kills() {
        let mut b = Burst::default();
        for _ in 0..11 {
            assert!(!b.wound());
        }
        assert!(b.swell() > 0.2);
        assert!(b.wound());
        assert_eq!(b.drain_events(), vec![Event::Burst]);
    }

    #[test]
    fn wounds_heal_if_you_stop() {
        let mut b = Burst::default();
        for _ in 0..11 {
            b.wound();
        }
        run(&mut b, 20.0);
        assert!(!b.wound(), "healed enough that one more poke doesn't kill it");
    }

    #[test]
    fn the_pieces_fly_out_land_as_triangles_and_come_home() {
        let mut b = Burst::default();
        for _ in 0..12 {
            b.wound();
        }
        run(&mut b, 3.0);
        assert_eq!(b.phase, Phase::Landscape);
        assert!(b.body_gone());
        for i in 0..SHARDS {
            let (pos, morph) = b.shard_pose(i);
            assert_eq!(morph, 1.0);
            assert!((pos - b.shards[i].home).len() < 1e-4);
        }
        // every piece can be picked out from above
        let i = b.pick(v3(0.0, 12.0, 0.0) + b.shards[3].home, -V3::UP).unwrap();
        assert_eq!(i, 3);
        assert!(b.gather());
        run(&mut b, 3.0);
        assert!(b.whole() && !b.body_gone());
        assert!(b.drain_events().contains(&Event::Gathered));
    }
}
