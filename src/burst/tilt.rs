//! The room tips while the twelve lie on the ground: tip it with the arrows, or a phone
//! in the hand (keys.rs, motion.rs), and the pieces feel it too.
//!
//! - a group goes downhill together, slowly, all of it at the same pace, and stops when
//!   the room is level again; the ground holds it back, so a slight tip moves nothing
//! - a piece on its own goes a little faster and a little to one side, so loose pieces
//!   spread out as they go
//! - two groups that come together on the way down hold on to each other and are one
//!   (kin.rs's own rule for setting them down close)
//! - a planted piece stands its ground, and so does the group it is in; nothing goes past
//!   the edge of the ground, each piece slowing to a stop as it nears it
//!
//! The rest of the senses act only on the ordinary liquid; this is one of the two places
//! they reach into a form (the other is forms/gust.rs). The pieces only change where they
//! stand, so nothing new reaches the shader. Sliding never makes them run as a river by
//! itself: that is still decided when a piece is let go (burst/mod.rs).

use super::kin::Group;
use super::{Burst, Phase, SHARDS};
use crate::game::Game;
use crate::math::*;
use crate::progress::Discovery;

/// How far the room must tip (0..1) before anything slides: below it, the ground holds them.
const GRIP: f32 = 0.3;
/// Fastest a group slides, and a piece on its own, in units a second, the room tipped all the way.
const GROUP_SPEED: f32 = 1.0;
const LOOSE_SPEED: f32 = 1.6;
/// How fast a slide takes up its pace, or loses it (per second, eased).
const TAKE_UP: f32 = 1.2;
/// The most a piece on its own turns aside from straight downhill, in radians.
const ASIDE: f32 = 0.45;
/// Pieces slow to a stop over this last stretch before this far out.
const EDGE: f32 = 20.0;
const SOFT: f32 = 4.0;

/// How each piece is sliding now, eased, so it never starts or stops in one frame.
#[derive(Default)]
pub struct Tilt {
    vel: [V3; SHARDS],
}

fn flat(v: V3) -> V3 {
    v3(v.x, 0.0, v.z)
}

fn members(g: Group) -> impl Iterator<Item = usize> {
    (0..SHARDS).filter(move |&j| g & (1 << j) != 0)
}

impl Burst {
    /// The room tipped downhill along `slope` (flat, 0..1 long). Returns true if two groups
    /// came together on the way down.
    pub fn lean(&mut self, slope: V3, dt: f32) -> bool {
        if self.phase != Phase::Landscape || self.river.active || self.kin.holding() {
            self.tilt = Tilt::default();
            return false;
        }
        let slope = flat(slope);
        let push = ((slope.len() - GRIP) / (1.0 - GRIP)).clamp(0.0, 1.0);
        let down = if slope.len() > 1e-4 { slope * (1.0 / slope.len()) } else { V3::ZERO };
        let take = 1.0 - (-TAKE_UP * dt).exp();
        let sizes: [f32; SHARDS] = std::array::from_fn(|j| self.shard_size(j));
        let land = &self.land;
        let fixed = |j: usize| land.returned(j) || !land.can_carry(j);
        let mut homes = self.shards.map(|s| s.home);
        let mut moved: Group = 0;
        let mut seen: Group = 0;
        for i in 0..SHARDS {
            if seen & (1 << i) != 0 {
                continue;
            }
            let g = self.kin.group(i);
            seen |= g;
            let n = g.count_ones();
            let goal = if members(g).any(fixed) {
                V3::ZERO
            } else if n > 1 {
                down * (GROUP_SPEED * push)
            } else {
                // each its own way aside, so they spread as they go
                let (s, c) = ((i as f32 * 2.399_963).sin() * ASIDE).sin_cos();
                let side = v3(-down.z, 0.0, down.x);
                (down * c + side * s) * (LOOSE_SPEED * push)
            };
            // all of a group at one pace
            let mut v = members(g).fold(V3::ZERO, |a, j| a + self.tilt.vel[j]) * (1.0 / n as f32);
            v += (goal - v) * take;
            // coming to rest, the last of it goes at once: under a hair's breadth a second
            if goal == V3::ZERO && v.len() < 0.01 {
                v = V3::ZERO;
            }
            // near the edge it slows to a stop, all of it as one
            let mut step = v * dt;
            for j in members(g) {
                let at = flat(homes[j]);
                let r = at.len();
                if r < 1e-3 {
                    continue;
                }
                let out = at * (1.0 / r);
                let o = step.dot(out);
                if o > 0.0 {
                    let k = ((EDGE - r) / SOFT).clamp(0.0, 1.0);
                    step -= out * (o * (1.0 - k));
                }
            }
            let v = if dt > 0.0 { step * (1.0 / dt) } else { V3::ZERO };
            for j in members(g) {
                self.tilt.vel[j] = v;
            }
            if step.len() < 1e-6 {
                continue;
            }
            for j in members(g) {
                homes[j] += step;
            }
            // it pushes strangers aside, not its own
            for j in members(g) {
                let before = homes;
                let at = land.shove(j, homes[j], &mut homes, &sizes);
                for m in members(g) {
                    homes[m] = before[m];
                }
                homes[j] = at;
            }
            moved |= g;
        }
        if moved == 0 {
            return false;
        }
        for (s, h) in self.shards.iter_mut().zip(homes) {
            s.home = h;
        }
        // where they have come to, they hold on to whatever is close, as if set down there
        let was = self.kin.groups().len();
        for i in members(moved) {
            self.kin.settle(i, &homes, &sizes, |j| land.returned(j));
        }
        // someone is doing something: the weather waits
        self.land.touch();
        self.kin.groups().len() < was
    }
}

/// Every frame, from `Game::update`, after keys.rs has decided how far the room is tipped.
pub fn step(g: &mut Game, dt: f32) {
    let (side, away) = g.keys.lean();
    let b = g.cam.basis();
    let level = flat(b.fwd);
    let level = if level.len() > 1e-4 { level * (1.0 / level.len()) } else { V3::ZERO };
    let slope = flat(b.right) * side + level * away;
    if g.world.burst.lean(slope, dt) {
        g.discover(Discovery::Downhill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::burst::kin::Kin;
    use crate::burst::landscape::spacing;
    use crate::burst::river::River;

    const FLOOR_Y: f32 = crate::floor::fusion::FLOOR_Y;

    /// How far piece `i` stands from the nearest other piece, beyond how close they shove to.
    fn gap(b: &Burst, i: usize) -> f32 {
        let sizes: [f32; SHARDS] = std::array::from_fn(|j| b.shard_size(j));
        (0..SHARDS)
            .filter(|&j| j != i)
            .map(|j| flat(b.shards[i].home - b.shards[j].home).len() - spacing(sizes[i], sizes[j]))
            .fold(f32::MAX, f32::min)
    }

    fn landed() -> Burst {
        let mut b = Burst::default();
        for _ in 0..12 {
            b.wound();
        }
        for _ in 0..360 {
            b.step(1.0 / 120.0);
        }
        assert_eq!(b.phase, Phase::Landscape);
        b
    }

    fn run(b: &mut Burst, slope: V3, secs: f32) -> bool {
        let mut joined = false;
        for _ in 0..(secs * 60.0) as usize {
            joined |= b.lean(slope, 1.0 / 60.0);
            b.step(1.0 / 60.0);
        }
        joined
    }

    /// Everyone else stands far apart around the edge; these stand where they are put, and
    /// hold on to whatever they were set down close to.
    fn put(b: &mut Burst, at: &[(usize, f32, f32)]) {
        b.kin = Kin::default();
        for i in 0..SHARDS {
            let a = i as f32 * std::f32::consts::TAU / SHARDS as f32;
            b.shards[i].home = v3(a.cos() * 19.0, FLOOR_Y, a.sin() * 19.0);
        }
        for &(i, x, z) in at {
            b.shards[i].home = v3(x, FLOOR_Y, z);
        }
        let homes = b.shards.map(|s| s.home);
        let sizes: [f32; SHARDS] = std::array::from_fn(|j| b.shard_size(j));
        for &(i, _, _) in at {
            b.kin.settle(i, &homes, &sizes, |_| false);
        }
    }

    #[test]
    fn level_nothing_moves() {
        let mut b = landed();
        let was = b.shards.map(|s| s.home);
        run(&mut b, V3::ZERO, 10.0);
        run(&mut b, v3(GRIP * 0.9, 0.0, 0.0), 10.0);
        for (i, (s, w)) in b.shards.iter().zip(was).enumerate() {
            assert_eq!(s.home, w, "piece {i} moved");
        }
    }

    #[test]
    fn a_group_slides_downhill_together_and_stops_when_level() {
        let mut b = landed();
        // two set down close, out of everyone's way
        put(&mut b, &[(0, -8.0, 8.0), (1, -8.0, 11.0)]);
        assert!(b.kin.bonded(0, 1));
        let (a0, a1) = (b.shards[0].home, b.shards[1].home);
        run(&mut b, v3(1.0, 0.0, 0.0), 3.0);
        let (d0, d1) = (b.shards[0].home - a0, b.shards[1].home - a1);
        assert!(d0.x > 1.0, "it went downhill: {d0:?}");
        assert!((d0 - d1).len() < 1e-3, "as one: {d0:?} {d1:?}");
        assert!(d0.x < 3.0 * GROUP_SPEED, "slowly: {d0:?}");
        // level again, it comes to rest without a jump
        let mut last = b.shards[0].home;
        let mut fastest = 0.0f32;
        for _ in 0..240 {
            b.lean(V3::ZERO, 1.0 / 60.0);
            b.step(1.0 / 60.0);
            fastest = fastest.max((b.shards[0].home - last).len());
            last = b.shards[0].home;
        }
        assert!(fastest < GROUP_SPEED / 60.0 + 1e-4, "{fastest}");
        let rest = b.shards[0].home;
        run(&mut b, V3::ZERO, 1.0);
        assert!((b.shards[0].home - rest).len() < 1e-3, "it stopped");
    }

    #[test]
    fn a_loose_piece_goes_further_than_a_group() {
        let mut b = landed();
        put(&mut b, &[(0, -8.0, 8.0), (1, -8.0, 11.0), (2, -8.0, -8.0)]);
        assert_eq!(b.kin.group(2), 1 << 2);
        let (g, l) = (b.shards[0].home, b.shards[2].home);
        run(&mut b, v3(1.0, 0.0, 0.0), 3.0);
        assert!(b.shards[2].home.x - l.x > b.shards[0].home.x - g.x);
    }

    #[test]
    fn two_groups_that_slide_together_are_one() {
        let mut b = landed();
        put(&mut b, &[(0, -12.0, 4.0), (1, -12.0, 7.0), (2, 0.0, 4.0), (3, 0.0, 7.0)]);
        let (ga, gb) = (b.kin.group(0), b.kin.group(2));
        assert!(ga != gb && ga.count_ones() == 2 && gb.count_ones() == 2);
        // the second group is held where it is by a planted piece
        b.land.toggle_root(2);
        let joined = run(&mut b, v3(1.0, 0.0, 0.0), 15.0);
        assert!(joined, "they met");
        assert_eq!(b.kin.group(0) & 0b1111, 0b1111);
    }

    #[test]
    fn played_through_game_two_groups_meeting_downhill_is_found() {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.world.burst = landed();
        // the second group stands downhill of the first, as the player sees it
        let right = flat(g.cam.basis().right).norm();
        let across = v3(-right.z, 0.0, right.x);
        let mid = v3(-4.0, 0.0, 4.0);
        let at: Vec<(usize, f32, f32)> = [(-6.0f32, 0.0f32), (-6.0, 3.0), (6.0, 0.0), (6.0, 3.0)]
            .into_iter()
            .enumerate()
            .map(|(i, (s, a))| {
                let p = mid + right * s + across * a;
                (i, p.x, p.z)
            })
            .collect();
        put(&mut g.world.burst, &at);
        g.world.burst.land.toggle_root(2);
        crate::input::keys::down(&mut g, "ArrowRight");
        for _ in 0..(20.0 * 60.0) as usize {
            g.update(1.0 / 60.0);
            if g.progress.has(Discovery::Downhill) {
                break;
            }
        }
        assert!(g.progress.has(Discovery::Downhill));
    }

    #[test]
    fn nothing_slides_past_the_edge_and_tipping_alone_does_not_make_them_run() {
        let mut b = landed();
        for dir in [v3(1.0, 0.0, 0.0), v3(0.0, 0.0, -1.0), v3(-0.7, 0.0, 0.7)] {
            run(&mut b, dir, 40.0);
            for s in &b.shards {
                assert!(flat(s.home).len() <= EDGE + 0.5, "{:?}", s.home);
            }
            assert!(!b.river.active);
            // piled against the edge, they still lie along it, too far apart to run
            assert!(River::crowded(&b.shards.map(|s| s.home)).is_none(), "{dir:?}");
        }
        // a gentle tip, for a while: they are not left crowded enough to run
        let mut b = landed();
        run(&mut b, v3(0.5, 0.0, 0.0), 20.0);
        assert!(River::crowded(&b.shards.map(|s| s.home)).is_none());
        assert!((0..SHARDS).all(|i| gap(&b, i) > -1e-3), "they keep their distance");
    }

    #[test]
    fn a_planted_piece_holds_its_group() {
        let mut b = landed();
        put(&mut b, &[(0, -8.0, 8.0), (1, -8.0, 11.0)]);
        // everyone planted but the one held by its planted neighbour
        for j in (0..SHARDS).filter(|&j| j != 1) {
            b.land.toggle_root(j);
        }
        let was = b.shards[1].home;
        run(&mut b, v3(1.0, 0.0, 0.0), 3.0);
        assert!((b.shards[1].home - was).len() < 1e-3, "{:?} -> {:?}", was, b.shards[1].home);
    }
}
