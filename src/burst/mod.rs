//! Poke it to death and it bursts. Every poke leaves a wound that heals slowly;
//! twelve open at once and it can't hold any more. It swells, pops, and its
//! liquid flies out as twelve pieces that land far apart and harden into
//! twelve triangles, each its own shape, color and consistency, standing in
//! a white landscape. Poke a triangle and it answers in its own material.
//! Hold still on the empty ground and the pieces come home.
//! What else the landscape does is in landscape.rs; set close together they
//! keep together, in as many groups as they are set down in (kin.rs); all
//! crowded into one anywhere but the middle, they run as a river (river.rs).
//!
//! Like meiosis.rs and fusion.rs, `Phase` only decides which continuous values
//! move; the shader blends them, so the burst and the flight never jump.

pub mod kin;
pub mod landscape;
pub mod river;

use kin::Kin;
use landscape::Land;
use crate::math::*;
use river::River;

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
/// Each liquid bead's radius, as a fraction of the body's; the shader's blob matches it.
pub const BEAD: f32 = 0.437;
/// Holding still on empty ground this long calls the pieces home.
pub const GATHER_HOLD_SECS: f32 = 1.4;
/// Camera distance from which the whole landscape can be seen.
pub const LANDSCAPE_DIST: f32 = 21.0;
const FLOOR_Y: f32 = crate::floor::fusion::FLOOR_Y;

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
pub const SIZES: [f32; SHARDS] = [3.6, 4.8, 3.1, 2.5, 5.9, 2.8, 4.1, 2.2, 5.3, 2.7, 6.4, 3.4];

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
    Land(landscape::Event),
    River(river::Event),
    Kin(kin::Event),
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
    pub land: Land,
    pub river: River,
    pub kin: Kin,
    events: Vec<Event>,
}

fn scatter() -> [Shard; SHARDS] {
    let mut shards = [Shard::default(); SHARDS];
    for (i, s) in shards.iter_mut().enumerate() {
        // A golden-angle spiral spreads them evenly without a grid.
        let a = i as f32 * 2.399_963 + 0.6;
        let r = 6.0 + i as f32 * 1.05;
        s.home = v3(a.cos() * r, FLOOR_Y, a.sin() * r);
        s.yaw = a * 1.7 + i as f32;
        s.size = SIZES[i];
    }
    shards
}

impl Default for Burst {
    fn default() -> Self {
        Burst { phase: Phase::Whole, wounds: 0.0, since_wound: 0.0, shards: scatter(), land: Land::default(), river: River::default(), kin: Kin::default(), events: Vec::new() }
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
        let (home, melt) = self.land.place(i, self.shards[i].home);
        if self.land.returned(i) {
            // it melts on its way to its bead, and stays there until the body takes its place
            return (home, 1.0 - melt);
        }
        // Out along an arc: rise, then come down where it will stand.
        // kept together with others, it breathes with them
        let flat = V3::ZERO.lerp(home + self.kin.lean(i), s);
        let arc = (s * std::f32::consts::PI).sin() * (3.0 + self.shards[i].home.len() * 0.35);
        // planted, it stands partly underground
        let (depth, _) = self.land.depth_and_scale(i);
        let sunk = depth * self.shard_size(i) * s;
        // melted into the river, they are under the ground, and come up out of it as liquid
        let bled = smoothstep(0.0, 1.0, self.river.amount);
        let under = bled * (self.shard_size(i) * 0.9 + 1.5);
        let pos = v3(flat.x, flat.y + arc - sunk - under, flat.z);
        (pos, smoothstep(0.55, 1.0, s) * (1.0 - bled))
    }

    /// Height of piece `i`; planted ones grow.
    pub fn shard_size(&self, i: usize) -> f32 {
        let (_, scale) = self.land.depth_and_scale(i);
        self.shards[i].size * (1.0 + (scale - 1.0) * self.spread())
    }

    /// Slide piece `i` over the ground toward `to`, shoving the others aside.
    pub fn carry(&mut self, i: usize, to: V3) -> bool {
        if self.phase != Phase::Landscape || self.river.active || !self.land.can_carry(i) {
            return false;
        }
        let mut homes = self.shards.map(|s| s.home);
        // the rest of its group come after it (kin.rs)
        self.kin.carry(i, &homes);
        let sizes = std::array::from_fn(|j| self.shard_size(j));
        let at = self.land.shove(i, v3(to.x, FLOOR_Y, to.z), &mut homes, &sizes);
        homes[i] = at;
        for (s, h) in self.shards.iter_mut().zip(homes) {
            s.home = h;
        }
        true
    }

    /// Let go of a carried piece. Near the middle it melts into a bead where the body was.
    pub fn release(&mut self, i: usize, body_r: f32) -> bool {
        if self.phase != Phase::Landscape || self.river.active {
            return false;
        }
        let homes = self.shards.map(|s| s.home);
        if !self.land.release(i, homes[i]) {
            // not home, but maybe close to others: it holds on to them
            let sizes = std::array::from_fn(|j| self.shard_size(j));
            self.kin.settle(i, &homes, &sizes, |j| self.land.returned(j));
            // and all of them crowded into one run
            if (0..SHARDS).all(|j| !self.land.returned(j)) {
                if let Some(at) = River::crowded(&homes) {
                    self.river.begin(at);
                }
            }
            return false;
        }
        // its group goes home with it, all but the planted
        let g = self.kin.group(i);
        self.kin.let_go();
        let blob = BEAD * body_r;
        for j in (0..SHARDS).filter(|&j| g & (1 << j) != 0) {
            if j == i || (self.land.can_carry(j) && self.land.take_home(j, homes[j])) {
                self.shards[j].home = landscape::bead(j, blob);
            }
            self.kin.forget(j);
        }
        true
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
            // every burst is a new landscape
            self.shards = scatter();
            self.land = Land::default();
            self.river = River::default();
            self.kin = Kin::default();
            self.wounds = 0.0;
            self.events.push(Event::Burst);
            return true;
        }
        false
    }

    /// Returns true if this poke made all twelve ring together.
    pub fn poke_shard(&mut self, i: usize) -> bool {
        self.shards[i].wobble = 1.0;
        // and the rest of its group feel it after
        self.kin.touch(i);
        if self.land.struck(i) {
            for s in self.shards.iter_mut() {
                s.wobble = 1.0;
            }
            return true;
        }
        false
    }

    pub fn gather(&mut self) -> bool {
        if self.phase != Phase::Landscape {
            return false;
        }
        if self.river.active {
            self.leave_river();
        }
        self.phase = Phase::Gathering { t: 0.0 };
        true
    }

    /// The river drains and the pieces come up out of it along its middle, as liquid.
    fn leave_river(&mut self) {
        self.river.drain();
        for (i, s) in self.shards.iter_mut().enumerate() {
            let (x, z) = self.river.world((i as f32 - 5.5) * 1.4, 0.0);
            s.home = v3(x, FLOOR_Y, z);
        }
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
            s.wobble = fade(s.wobble, -decay * dt);
        }
        if self.phase == Phase::Landscape && !self.river.active {
            self.land.step(dt);
            let sizes = std::array::from_fn(|j| self.shard_size(j));
            let land = &self.land;
            self.kin.step(dt, &mut self.shards, &sizes, land, |j| land.returned(j) || !land.can_carry(j));
        } else {
            self.kin.let_go();
        }
        for e in self.kin.drain_events() {
            if let kin::Event::Felt { i } = e {
                self.shards[i].wobble = self.shards[i].wobble.max(0.8);
            }
            self.events.push(Event::Kin(e));
        }
        self.river.step(dt);
        for e in self.river.drain_events() {
            if e == river::Event::Pulsed {
                self.gather();
            }
            self.events.push(Event::River(e));
        }
        for e in self.land.drain_events() {
            match e {
                landscape::Event::Eroded => {
                    self.gather();
                }
                landscape::Event::Mended { .. } => {
                    // the beads are already home; the camera comes in, then they are one again
                    self.phase = Phase::Gathering { t: 0.0 };
                }
                _ => {}
            }
            self.events.push(Event::Land(e));
        }
    }

    /// Which landed piece a ray hits first, by a bounding sphere around each.
    pub fn pick(&self, ro: V3, rd: V3) -> Option<usize> {
        if self.phase != Phase::Landscape || self.river.active {
            return None;
        }
        let mut best: Option<(f32, usize)> = None;
        for i in 0..SHARDS {
            if self.land.returned(i) {
                continue;
            }
            let size = self.shard_size(i);
            let c = self.shard_pose(i).0 + V3::UP * (size * 0.45);
            let oc = ro - c;
            let b = oc.dot(rd);
            let disc = b * b - (oc.dot(oc) - (size * 0.5).powi(2));
            if disc < 0.0 {
                continue;
            }
            let t = -b - disc.sqrt();
            if t > 0.0 && best.is_none_or(|(bt, _)| t < bt) {
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

    /// Carry every piece to a spot near `c`, one by one, as a player would.
    pub fn crowd(b: &mut Burst, c: V3) {
        for i in 0..SHARDS {
            let a = i as f32 * 2.399_963;
            let r = 1.9 * (i as f32).sqrt();
            assert!(b.carry(i, c + v3(a.cos() * r, 0.0, a.sin() * r)));
            b.release(i, 1.0);
        }
    }

    #[test]
    fn crowded_together_away_from_the_middle_they_run_and_come_back_out_of_it() {
        let mut b = Burst::default();
        for _ in 0..12 {
            b.wound();
        }
        run(&mut b, 3.0);
        crowd(&mut b, v3(11.0, FLOOR_Y, -4.0));
        assert!(b.river.active, "they ran");
        assert!((0..SHARDS).all(|i| !b.land.returned(i)), "none of them went home");
        run(&mut b, 3.0);
        assert!(b.pick(v3(11.0, 12.0, -4.0), -V3::UP).is_none(), "they are under the river now");
        for i in 0..SHARDS {
            assert!(b.shard_pose(i).0.y < FLOOR_Y, "piece {i} is under the ground");
        }
        assert!(!b.carry(0, v3(0.0, FLOOR_Y, 0.0)));
        assert!(b.drain_events().contains(&Event::River(river::Event::Bled)));
        // holding still on the ground still calls them home, up out of the river
        assert!(b.gather());
        run(&mut b, 3.0);
        assert!(b.whole() && !b.river.active);
    }
}
