//! The twelve need not stay twelve, or one. Set pieces down close together and
//! they keep together, as many groups as there are places to put them:
//!
//! - set one down beside another and the two hold on; set it beside a group
//!   and it joins; set two groups side by side and they are one
//! - carry one of a group and the rest come after it; pull away faster than
//!   they can follow and it comes loose, alone
//! - cut fast across the ground between them and the group parts where the cut
//!   crossed, each side stepping back from the other
//! - touch one of a group and the touch runs through the rest, one after another
//! - a group breathes, all of it at once, slower the more there are, and
//!   slowly turns its faces toward its own middle
//!
//! Only all twelve, crowded into one, run as a river (river.rs); smaller
//! groups keep to themselves. Carried to the middle, a group goes home together.
//!
//! `Kin` only keeps which pieces hold on to which; burst.rs owns where they
//! stand and folds the breathing into each piece's pose. Nothing new reaches
//! the shader: a piece's place and its turn are all it needs.

use crate::burst::{Shard, SHARDS};
use crate::landscape::Land;
use crate::math::*;

/// Set down this much further apart than they shove each other to, two pieces hold on.
const JOIN_GAP: f32 = 2.2;
/// Pulled this much further apart than that, they let go.
const PART_GAP: f32 = 3.2;
/// How fast a group comes after the piece being carried (per second, eased).
const FOLLOW: f32 = 5.0;
/// Seconds for a touch to reach the next piece along.
const PASS_SECS: f32 = 0.12;
/// How far a group leans in and out as it breathes, and its slowest breath.
const BREATH: f32 = 0.35;
const BREATH_SECS: f32 = 3.0;
/// How fast a piece takes up its group's breath, or lets it go (per second, eased).
const LEAN: f32 = 2.0;
/// How fast a group turns toward its middle (per second, eased).
const TURN: f32 = 0.5;
/// How far each side steps back from a cut, and how fast that dies away.
const STEP_BACK: f32 = 2.2;
const STEP_DECAY: f32 = 3.0;
/// A drag over the ground faster than this, in canvas heights a second, cuts.
pub const CUT_SPEED: f32 = 1.5;
/// Groups at once that count as many.
pub const MANY: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Two pieces, or two groups, hold on to each other.
    Joined,
    /// One was pulled away from its group faster than the group could follow.
    Loosed,
    /// A touch began to run through a group.
    Passed,
    /// The touch reached piece `i`.
    Felt { i: usize },
    /// There are `MANY` groups at once.
    Many,
}

/// A set of pieces, one bit each.
pub type Group = u16;

#[derive(Default)]
pub struct Kin {
    bonds: [Group; SHARDS],
    /// The piece being carried, the group it had when picked up, and where each of
    /// them stood from it then.
    held: Option<(usize, Group)>,
    offsets: [V3; SHARDS],
    /// Each piece stepping back from a cut, fading.
    step: [V3; SHARDS],
    /// How far each piece leans out with its group's breath now, eased so joining
    /// and parting never jump.
    lean: [V3; SHARDS],
    /// A touch on its way through a group: which piece, and seconds until it gets there.
    passing: Vec<(usize, f32)>,
    clock: f32,
    told_many: bool,
    events: Vec<Event>,
}

/// How close two pieces of these heights are shoved to, at most.
fn spacing(a: f32, b: f32) -> f32 {
    (a + b) * 0.32
}

fn flat(v: V3) -> V3 {
    v3(v.x, 0.0, v.z)
}

impl Kin {
    pub fn bonded(&self, a: usize, b: usize) -> bool {
        self.bonds[a] & (1 << b) != 0
    }

    fn bond(&mut self, a: usize, b: usize) {
        self.bonds[a] |= 1 << b;
        self.bonds[b] |= 1 << a;
    }

    fn unbond(&mut self, a: usize, b: usize) {
        self.bonds[a] &= !(1 << b);
        self.bonds[b] &= !(1 << a);
    }

    /// Everything piece `i` holds on to, through each other, itself included.
    pub fn group(&self, i: usize) -> Group {
        let mut g: Group = 1 << i;
        loop {
            let next = (0..SHARDS).filter(|&j| g & (1 << j) != 0).fold(g, |acc, j| acc | self.bonds[j]);
            if next == g {
                return g;
            }
            g = next;
        }
    }

    /// Every group of two or more, once each.
    pub fn groups(&self) -> Vec<Group> {
        let mut seen: Group = 0;
        let mut out = Vec::new();
        for i in 0..SHARDS {
            if seen & (1 << i) != 0 {
                continue;
            }
            let g = self.group(i);
            seen |= g;
            if g.count_ones() > 1 {
                out.push(g);
            }
        }
        out
    }

    /// A piece gone home holds on to nothing.
    pub fn forget(&mut self, i: usize) {
        for j in 0..SHARDS {
            self.unbond(i, j);
        }
    }

    /// Piece `i` is being carried; its group comes after it, keeping their places.
    pub fn carry(&mut self, i: usize, homes: &[V3; SHARDS]) {
        let g = self.group(i);
        if self.held != Some((i, g)) {
            self.held = Some((i, g));
            for j in 0..SHARDS {
                self.offsets[j] = homes[j] - homes[i];
            }
        }
    }

    pub fn let_go(&mut self) {
        self.held = None;
    }

    /// Piece `i` has been set down: it and its group hold on to every loose piece
    /// close enough. Returns true if anything new held on.
    pub fn settle(&mut self, i: usize, homes: &[V3; SHARDS], sizes: &[f32; SHARDS], away: impl Fn(usize) -> bool) -> bool {
        self.let_go();
        if away(i) {
            return false;
        }
        let g = self.group(i);
        let mut joined = false;
        for a in (0..SHARDS).filter(|&a| g & (1 << a) != 0) {
            for b in 0..SHARDS {
                if a == b || self.bonded(a, b) || away(b) {
                    continue;
                }
                if flat(homes[a] - homes[b]).len() < spacing(sizes[a], sizes[b]) + JOIN_GAP {
                    self.bond(a, b);
                    joined = true;
                }
            }
        }
        if joined {
            self.events.push(Event::Joined);
            if !self.told_many && self.groups().len() >= MANY {
                self.told_many = true;
                self.events.push(Event::Many);
            }
        }
        joined
    }

    /// Cut between `a` and `b`; each side steps back from the other.
    pub fn cut(&mut self, a: usize, b: usize, homes: &[V3; SHARDS]) {
        if !self.bonded(a, b) {
            return;
        }
        self.unbond(a, b);
        let ga = self.group(a);
        if ga & (1 << b) != 0 {
            // still held on to some other way
            return;
        }
        let gb = self.group(b);
        let apart = flat(homes[a] - homes[b]).norm();
        for j in 0..SHARDS {
            if ga & (1 << j) != 0 {
                self.step[j] += apart * STEP_BACK;
            } else if gb & (1 << j) != 0 {
                self.step[j] -= apart * STEP_BACK;
            }
        }
    }

    /// A touch on piece `i` runs through its group, nearest first.
    pub fn touch(&mut self, i: usize) {
        let g = self.group(i);
        if g.count_ones() < 2 {
            return;
        }
        self.passing.clear();
        let mut reached: Group = 1 << i;
        let mut front: Group = 1 << i;
        let mut hop = 0.0;
        while front != 0 {
            hop += 1.0;
            let next = (0..SHARDS).filter(|&j| front & (1 << j) != 0).fold(0, |acc, j| acc | self.bonds[j]) & !reached;
            for j in (0..SHARDS).filter(|&j| next & (1 << j) != 0) {
                self.passing.push((j, hop * PASS_SECS));
            }
            reached |= next;
            front = next;
        }
        self.events.push(Event::Passed);
    }

    /// How far piece `i` leans out from its group's middle now.
    pub fn lean(&self, i: usize) -> V3 {
        self.lean[i]
    }

    /// How far piece `i` would lean out from its group's middle as the group breathes.
    pub fn breath(&self, i: usize, homes: &[V3; SHARDS]) -> V3 {
        let g = self.group(i);
        let n = g.count_ones();
        if n < 2 {
            return V3::ZERO;
        }
        let members = (0..SHARDS).filter(|&j| g & (1 << j) != 0);
        let mid = members.clone().fold(V3::ZERO, |a, j| a + homes[j]) * (1.0 / n as f32);
        let out = flat(homes[i] - mid);
        if out.len() < 1e-3 {
            return V3::ZERO;
        }
        // each group in its own time: the lowest of them sets where in the breath it is
        let first = g.trailing_zeros() as f32;
        let secs = BREATH_SECS + 0.35 * n as f32;
        let phase = self.clock * std::f32::consts::TAU / secs + first * 1.3;
        out.norm() * (BREATH * phase.sin())
    }

    /// `homes` and `shards` are burst.rs's; `fixed(j)` is true for a piece that cannot move
    /// (planted, or gone home).
    pub fn step(&mut self, dt: f32, shards: &mut [Shard; SHARDS], sizes: &[f32; SHARDS], land: &Land, fixed: impl Fn(usize) -> bool) {
        self.clock += dt;
        let mut homes = shards.map(|s| s.home);

        // the carried one's group comes after it
        if let Some((i, g)) = self.held {
            let k = 1.0 - (-FOLLOW * dt).exp();
            for j in (0..SHARDS).filter(|&j| j != i && g & (1 << j) != 0 && !fixed(j)) {
                let to = homes[i] + self.offsets[j];
                let at = homes[j] + (to - homes[j]) * k;
                let before = homes;
                let at = land.shove(j, at, &mut homes, sizes);
                // it pushes strangers aside, not its own
                for m in (0..SHARDS).filter(|&m| g & (1 << m) != 0) {
                    homes[m] = before[m];
                }
                homes[j] = at;
            }
        }

        // stepping back from a cut
        for j in 0..SHARDS {
            if self.step[j].len() < 1e-4 || fixed(j) {
                self.step[j] = V3::ZERO;
                continue;
            }
            homes[j] += self.step[j] * dt;
            self.step[j] = self.step[j] * (-STEP_DECAY * dt).exp();
        }

        // stretched too far, they let go
        for a in 0..SHARDS {
            for b in a + 1..SHARDS {
                if self.bonded(a, b) && flat(homes[a] - homes[b]).len() > spacing(sizes[a], sizes[b]) + PART_GAP {
                    self.unbond(a, b);
                    if self.group(a) & (1 << b) == 0 {
                        self.events.push(Event::Loosed);
                    }
                }
            }
        }

        // a group slowly turns its faces toward its own middle
        let turn = 1.0 - (-TURN * dt).exp();
        for g in self.groups() {
            let n = g.count_ones() as f32;
            let mid = (0..SHARDS).filter(|&j| g & (1 << j) != 0).fold(V3::ZERO, |a, j| a + homes[j]) * (1.0 / n);
            for j in (0..SHARDS).filter(|&j| g & (1 << j) != 0) {
                let d = flat(mid - homes[j]);
                if d.len() < 0.5 {
                    continue;
                }
                // its face looks along (sin yaw, cos yaw), and either side will do
                let want = d.x.atan2(d.z);
                let pi = std::f32::consts::PI;
                let off = (want - shards[j].yaw + pi * 0.5).rem_euclid(pi) - pi * 0.5;
                shards[j].yaw += off * turn;
            }
        }

        for (s, h) in shards.iter_mut().zip(homes) {
            s.home = h;
        }
        let ease = 1.0 - (-LEAN * dt).exp();
        for j in 0..SHARDS {
            let to = self.breath(j, &homes);
            self.lean[j] += (to - self.lean[j]) * ease;
        }

        // the touch arriving, one piece after another
        let mut felt = Vec::new();
        self.passing.retain_mut(|(j, t)| {
            *t -= dt;
            if *t <= 0.0 {
                felt.push(*j);
                false
            } else {
                true
            }
        });
        self.events.extend(felt.into_iter().map(|i| Event::Felt { i }));
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

/// Whether the screen segments a–b and c–d cross.
pub fn crosses(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    let side = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::burst::SIZES;

    fn row(xs: &[f32]) -> [V3; SHARDS] {
        // the rest stand far away, each alone
        std::array::from_fn(|i| if i < xs.len() { v3(xs[i], 0.0, 0.0) } else { v3(i as f32 * 40.0, 0.0, 300.0) })
    }

    /// Set the first `n` down, one after another.
    fn settle(k: &mut Kin, homes: &[V3; SHARDS], n: usize) {
        for i in 0..n {
            k.settle(i, homes, &SIZES, |_| false);
        }
    }

    fn shards(homes: [V3; SHARDS]) -> [Shard; SHARDS] {
        std::array::from_fn(|i| Shard { home: homes[i], yaw: 0.0, size: SIZES[i], wobble: 0.0 })
    }

    #[test]
    fn set_down_close_they_hold_on_and_apart_they_do_not() {
        let mut k = Kin::default();
        let homes = row(&[0.0, 3.0, 30.0]);
        assert!(k.settle(1, &homes, &SIZES, |_| false));
        assert!(k.bonded(0, 1));
        assert!(!k.settle(2, &homes, &SIZES, |_| false), "too far to hold on");
        assert_eq!(k.groups(), vec![0b11]);
        assert_eq!(k.drain_events(), vec![Event::Joined]);
    }

    #[test]
    fn as_many_groups_as_there_are_places() {
        let mut k = Kin::default();
        let homes = row(&[0.0, 3.0, 40.0, 43.0, 80.0, 83.0, 120.0, 123.0]);
        for i in [1, 3, 5, 7] {
            k.settle(i, &homes, &SIZES, |_| false);
        }
        assert_eq!(k.groups().len(), 4);
        assert!(k.drain_events().contains(&Event::Many));
    }

    #[test]
    fn carried_slowly_the_group_comes_along_and_pulled_away_fast_it_comes_loose() {
        let land = Land::default();
        let mut k = Kin::default();
        let mut s = shards(row(&[0.0, 3.0, 6.0]));
        let homes = s.map(|s| s.home);
        settle(&mut k, &homes, 3);
        assert_eq!(k.group(0), 0b111);
        // a slow walk: two units a second for five seconds
        for _ in 0..300 {
            s[0].home.z += 2.0 / 60.0;
            k.carry(0, &s.map(|s| s.home));
            k.step(1.0 / 60.0, &mut s, &SIZES, &land, |_| false);
        }
        assert_eq!(k.group(0), 0b111, "they kept up");
        assert!(s[2].home.z > 8.5, "and came along: {}", s[2].home.z);
        // then a yank
        for _ in 0..20 {
            s[0].home.z += 40.0 / 60.0;
            k.carry(0, &s.map(|s| s.home));
            k.step(1.0 / 60.0, &mut s, &SIZES, &land, |_| false);
        }
        assert_eq!(k.group(0), 0b1, "it came away alone");
        assert_eq!(k.group(1), 0b110, "the others still hold on");
        assert!(k.drain_events().contains(&Event::Loosed));
    }

    #[test]
    fn a_cut_parts_a_group_and_each_side_steps_back() {
        let land = Land::default();
        let mut k = Kin::default();
        let mut s = shards(row(&[0.0, 3.0, 6.0, 9.0]));
        let homes = s.map(|s| s.home);
        settle(&mut k, &homes, 4);
        assert_eq!(k.group(0), 0b1111);
        k.cut(1, 2, &homes);
        assert_eq!(k.groups(), vec![0b0011, 0b1100]);
        let gap = s[2].home.x - s[1].home.x;
        for _ in 0..120 {
            k.step(1.0 / 60.0, &mut s, &SIZES, &land, |_| false);
        }
        assert!(s[2].home.x - s[1].home.x > gap + 0.5, "they stepped apart");
        assert_eq!(k.groups().len(), 2);
    }

    #[test]
    fn a_touch_runs_through_the_group_nearest_first() {
        let land = Land::default();
        let mut k = Kin::default();
        let mut s = shards(row(&[0.0, 3.0, 6.0]));
        let homes = s.map(|s| s.home);
        settle(&mut k, &homes, 3);
        k.drain_events();
        k.touch(0);
        let mut felt = Vec::new();
        for _ in 0..60 {
            k.step(1.0 / 60.0, &mut s, &SIZES, &land, |_| false);
            for e in k.drain_events() {
                if let Event::Felt { i } = e {
                    felt.push(i);
                }
            }
        }
        assert_eq!(felt, vec![1, 2]);
    }

    #[test]
    fn a_group_breathes_together_and_turns_inward() {
        let land = Land::default();
        let mut k = Kin::default();
        let mut s = shards(row(&[0.0, 3.0]));
        let homes = s.map(|s| s.home);
        k.settle(1, &homes, &SIZES, |_| false);
        for _ in 0..600 {
            k.step(1.0 / 60.0, &mut s, &SIZES, &land, |_| false);
        }
        let (a, b) = (k.breath(0, &homes), k.breath(1, &homes));
        assert!((a + b).len() < 1e-4, "they lean in and out together");
        assert_eq!(k.breath(5, &homes), V3::ZERO, "a piece alone does not");
        // facing along x, toward each other: yaw a quarter turn either way
        let pi = std::f32::consts::PI;
        assert!(((s[0].yaw.rem_euclid(pi)) - pi * 0.5).abs() < 0.2, "{}", s[0].yaw);
    }

    #[test]
    fn segments_cross() {
        assert!(crosses((0.0, 0.0), (2.0, 2.0), (0.0, 2.0), (2.0, 0.0)));
        assert!(!crosses((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)));
    }
}
