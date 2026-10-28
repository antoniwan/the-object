//! What the twelve do once they have landed. The landscape is not a pause
//! between bursting and gathering; it has its own slow life:
//!
//! - poke a piece and it sounds its own note; three different ones close
//!   together and all twelve ring at once
//! - drag a piece and it slides over the ground; pieces shove each other aside
//! - press and hold one still and it sinks into the ground, roots, and slowly
//!   rises again larger than it was; hold it again and it lets go
//! - carry a piece back to where the body was and it melts into a bead there;
//!   bring all twelve and they are one body again, colored by the last to arrive
//! - leave it all alone and the weather rounds every edge, until the pebbles
//!   roll home on their own
//!
//! `Land` only keeps the numbers; burst.rs folds them into each piece's pose,
//! and the shader reads them as height, size and rounding.

use crate::math::*;

use crate::burst::SHARDS;

/// Holding a piece still this long plants it, or pulls it back out.
pub const PLANT_HOLD_SECS: f32 = 0.9;
/// Seconds to sink, and then to grow back up out of the ground.
const SINK_SECS: f32 = 1.4;
const GROW_SECS: f32 = 24.0;
/// How deep a planted piece goes, as a fraction of its height, and how much larger it comes back.
const SINK_DEPTH: f32 = 0.72;
pub const GROWTH: f32 = 0.6;
/// Untouched this long, the weather starts; this long more and it is finished.
const CALM_SECS: f32 = 3.0;
const WEATHER_SECS: f32 = 50.0;
/// The most a corner rounds, in units of the piece's height.
const MAX_ROUND: f32 = 0.22;
/// Three different pieces within this many seconds ring together.
const CHORD_SECS: f32 = 1.6;
/// Dropped this close to the middle, a piece goes back.
pub const MEND_RADIUS: f32 = 2.4;
const MELT_SECS: f32 = 0.9;
/// Pieces can be carried at most this far out.
const REACH: f32 = 26.0;

/// Each piece's color, matching TCOL in the shader. The body takes on the last one home.
pub const TINTS: [V3; SHARDS] = [
    v3(0.95, 0.94, 0.97), v3(0.80, 0.82, 0.86), v3(0.86, 0.96, 0.98), v3(0.93, 0.22, 0.45),
    v3(0.62, 0.58, 0.53), v3(0.96, 0.60, 0.14), v3(0.16, 0.19, 0.56), v3(0.96, 0.90, 0.78),
    v3(0.80, 0.91, 1.00), v3(0.95, 0.36, 0.16), v3(0.05, 0.05, 0.07), v3(0.36, 0.55, 0.26),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Chord,
    Planted,
    /// A planted piece has come all the way back up.
    Grown,
    Weathered,
    /// The weather has finished; they roll home.
    Eroded,
    /// The last of the twelve was carried home; `last` decides the color.
    Mended { last: usize },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Root {
    pub rooted: bool,
    /// 0..1 how far into the ground, then 0..1 how far it has grown back out.
    pub sink: f32,
    pub grow: f32,
    told: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Return {
    /// 0 still out there; climbs to 1 as it melts into its bead.
    melt: f32,
    from: V3,
}

#[derive(Default)]
pub struct Land {
    pub roots: [Root; SHARDS],
    returns: [Return; SHARDS],
    /// 0..1 how worn the edges are.
    pub weather: f32,
    calm: f32,
    told_weather: bool,
    clock: f32,
    recent: Vec<(f32, usize)>,
    events: Vec<Event>,
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where each bead settles: the corners of an icosahedron, a small cluster
/// where the body was. `blob` is a bead's radius; positions are its base.
pub fn bead(i: usize, blob: f32) -> V3 {
    const P: f32 = 1.618_034;
    const ICO: [(f32, f32, f32); SHARDS] = [
        (0.0, 1.0, P), (0.0, 1.0, -P), (0.0, -1.0, P), (0.0, -1.0, -P),
        (1.0, P, 0.0), (-1.0, P, 0.0), (1.0, -P, 0.0), (-1.0, -P, 0.0),
        (P, 0.0, 1.0), (-P, 0.0, 1.0), (P, 0.0, -1.0), (-P, 0.0, -1.0),
    ];
    let (x, y, z) = ICO[i];
    v3(x, y, z).norm() * (blob * 1.25) - V3::UP * blob
}

impl Land {
    /// Any touch on the landscape calms the weather for a while.
    pub fn touch(&mut self) {
        self.calm = 0.0;
    }

    pub fn returned(&self, i: usize) -> bool {
        self.returns[i].melt > 0.0
    }

    pub fn can_carry(&self, i: usize) -> bool {
        !self.roots[i].rooted && self.roots[i].sink < 0.05 && !self.returned(i)
    }

    /// A poke landed on piece `i`. Returns true if it completed a chord.
    pub fn struck(&mut self, i: usize) -> bool {
        let now = self.clock;
        self.recent.retain(|&(t, j)| now - t < CHORD_SECS && j != i);
        self.recent.push((now, i));
        if self.recent.len() >= 3 {
            self.recent.clear();
            self.events.push(Event::Chord);
            return true;
        }
        false
    }

    /// Plant it, or pull a planted one back out. Returns true if it was planted.
    pub fn toggle_root(&mut self, i: usize) -> bool {
        if self.returned(i) {
            return false;
        }
        let r = &mut self.roots[i];
        r.rooted = !r.rooted;
        if r.rooted {
            r.told = false;
            self.events.push(Event::Planted);
        }
        r.rooted
    }

    /// Dropped piece `i`, standing at `at`: in the middle, it goes home. Returns true if it did.
    pub fn release(&mut self, i: usize, at: V3) -> bool {
        if self.returned(i) || (at.x * at.x + at.z * at.z).sqrt() > MEND_RADIUS {
            return false;
        }
        self.returns[i] = Return { melt: 1e-4, from: at };
        true
    }

    /// Where piece `i` stands while landed, and how far it has melted back (0..1).
    /// `home` is where burst.rs keeps it; `blob` is a bead's radius.
    pub fn place(&self, i: usize, home: V3) -> (V3, f32) {
        let r = self.returns[i];
        if r.melt <= 0.0 {
            return (home, 0.0);
        }
        // up over a small arc and down into its bead
        let e = ease(r.melt);
        let p = r.from.lerp(home, e);
        (p + V3::UP * ((e * std::f32::consts::PI).sin() * 1.2), e)
    }

    /// How far below the ground its base is, as a fraction of height, and how much larger it is.
    pub fn depth_and_scale(&self, i: usize) -> (f32, f32) {
        let r = self.roots[i];
        let g = ease(r.grow);
        (ease(r.sink) * SINK_DEPTH * (1.0 - g), 1.0 + GROWTH * g)
    }

    pub fn rounding(&self) -> f32 {
        ease(self.weather) * MAX_ROUND
    }

    pub fn step(&mut self, dt: f32) {
        self.clock += dt;
        for r in self.roots.iter_mut() {
            if r.rooted {
                r.sink = (r.sink + dt / SINK_SECS).min(1.0);
                if r.sink >= 1.0 {
                    r.grow = (r.grow + dt / GROW_SECS).min(1.0);
                }
                if r.grow >= 1.0 && !r.told {
                    r.told = true;
                    self.events.push(Event::Grown);
                }
            } else {
                // pulled out: what it grew, it keeps
                r.sink = (r.sink - dt / SINK_SECS).max(0.0);
            }
        }

        let mut last = None;
        for (i, r) in self.returns.iter_mut().enumerate() {
            if r.melt > 0.0 && r.melt < 1.0 {
                r.melt = (r.melt + dt / MELT_SECS).min(1.0);
                if r.melt >= 1.0 {
                    last = Some(i);
                }
            }
        }
        if let Some(last) = last {
            if self.returns.iter().all(|r| r.melt >= 1.0) {
                self.events.push(Event::Mended { last });
            }
        }

        self.calm += dt;
        if self.calm > CALM_SECS && self.weather < 1.0 {
            self.weather = (self.weather + dt / WEATHER_SECS).min(1.0);
            if self.weather > 0.5 && !self.told_weather {
                self.told_weather = true;
                self.events.push(Event::Weathered);
            }
            if self.weather >= 1.0 {
                self.events.push(Event::Eroded);
            }
        }
    }

    /// Nudge every other loose piece out of the way of piece `i` at `at`;
    /// a rooted one stands its ground and pushes `at` back instead.
    pub fn shove(&self, i: usize, mut at: V3, homes: &mut [V3; SHARDS], sizes: &[f32; SHARDS]) -> V3 {
        let flat = (at.x * at.x + at.z * at.z).sqrt();
        if flat > REACH {
            at = v3(at.x * REACH / flat, at.y, at.z * REACH / flat);
        }
        for j in 0..SHARDS {
            if j == i || self.returned(j) {
                continue;
            }
            let min = (sizes[i] + sizes[j]) * 0.32;
            let d = v3(homes[j].x - at.x, 0.0, homes[j].z - at.z);
            let l = d.len();
            if l >= min {
                continue;
            }
            let dir = if l > 1e-4 { d * (1.0 / l) } else { v3(1.0, 0.0, 0.0) };
            if self.roots[j].rooted || self.roots[j].sink > 0.0 {
                at = at - dir * (min - l);
            } else {
                homes[j] = homes[j] + dir * (min - l);
            }
        }
        at
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(l: &mut Land, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            l.step(1.0 / 60.0);
        }
    }

    #[test]
    fn three_different_pieces_in_a_breath_ring_together() {
        let mut l = Land::default();
        assert!(!l.struck(2));
        assert!(!l.struck(2), "the same one twice is not a chord");
        run(&mut l, 0.3);
        assert!(!l.struck(5));
        assert!(l.struck(9));
        assert_eq!(l.drain_events(), vec![Event::Chord]);
        // too slow and it is only three notes
        l.struck(1);
        run(&mut l, 2.0);
        l.struck(2);
        assert!(!l.struck(3));
    }

    #[test]
    fn a_planted_piece_sinks_then_rises_larger() {
        let mut l = Land::default();
        assert!(l.toggle_root(4));
        assert!(!l.can_carry(4));
        run(&mut l, SINK_SECS + 0.1);
        let (d, s) = l.depth_and_scale(4);
        assert!(d > 0.6 && s < 1.05);
        run(&mut l, GROW_SECS);
        let (d, s) = l.depth_and_scale(4);
        assert!(d < 1e-3 && (s - 1.0 - GROWTH).abs() < 1e-3);
        assert!(l.drain_events().contains(&Event::Grown));
        assert!(!l.toggle_root(4));
        run(&mut l, SINK_SECS + 0.1);
        assert!(l.can_carry(4));
    }

    #[test]
    fn left_alone_the_weather_wears_them_down_and_touching_pauses_it() {
        let mut l = Land::default();
        run(&mut l, CALM_SECS + WEATHER_SECS * 0.3);
        let w = l.weather;
        assert!(w > 0.2);
        for _ in 0..60 {
            l.touch();
            run(&mut l, 1.0);
        }
        assert_eq!(l.weather, w);
        run(&mut l, CALM_SECS + WEATHER_SECS);
        let ev = l.drain_events();
        assert!(ev.contains(&Event::Weathered) && ev.contains(&Event::Eroded));
        assert!(l.rounding() > MAX_ROUND * 0.99);
    }

    #[test]
    fn carried_home_one_by_one_they_mend() {
        let mut l = Land::default();
        assert!(!l.release(0, v3(9.0, 0.0, 0.0)), "too far out");
        for i in 0..SHARDS {
            assert!(l.release(i, v3(1.0, 0.0, 0.5)));
            run(&mut l, 0.2);
        }
        run(&mut l, MELT_SECS);
        let ev = l.drain_events();
        assert_eq!(ev, vec![Event::Mended { last: SHARDS - 1 }]);
    }

    #[test]
    fn a_rooted_piece_cannot_be_shoved() {
        let mut l = Land::default();
        let mut homes = [V3::ZERO; SHARDS];
        let sizes = [4.0; SHARDS];
        for (j, h) in homes.iter_mut().enumerate() {
            *h = v3(j as f32 * 20.0, 0.0, 0.0);
        }
        homes[1] = v3(5.0, 0.0, 0.0);
        let at = l.shove(0, v3(4.0, 0.0, 0.0), &mut homes, &sizes);
        assert!((at - v3(4.0, 0.0, 0.0)).len() < 1e-4);
        assert!(homes[1].x > 6.5, "the loose one moved aside");
        l.toggle_root(1);
        homes[1] = v3(5.0, 0.0, 0.0);
        let at = l.shove(0, v3(4.0, 0.0, 0.0), &mut homes, &sizes);
        assert!(at.x < 2.5 && homes[1].x == 5.0);
    }
}
