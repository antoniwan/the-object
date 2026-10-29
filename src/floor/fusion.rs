//! Slam it and it stays down. Thrown at the floor hard enough, the object
//! doesn't bounce: it melts into the floor, and the floor becomes liquid,
//! spreading out from the impact until the whole ground is a mirror. Tap the
//! lake and it ripples. Pull up from it and the object rises back out.
//! What else the lake can become while the object is in it lives in mere.rs.
//!
//! Like meiosis.rs, `Phase` only decides which continuous values are moving;
//! the shader blends those, so the melt and the spread never jump.

use crate::math::*;
use crate::floor::mere::Mere;

/// Height of the floor plane. The shader uses the same value.
pub const FLOOR_Y: f32 = -1.45;
/// Landing faster than this (world units per second) fuses instead of bouncing.
pub const SLAM_SPEED: f32 = 6.0;
/// How far the liquid spreads, in world units, once it has fully spread.
pub const LAKE_RADIUS: f32 = 40.0;
pub const MAX_LAKE_RIPPLES: usize = 6;
const SPREAD_SECS: f32 = 2.6;
const SINK_SECS: f32 = 1.5;
const RISE_SECS: f32 = 2.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Fusing { t: f32 },
    Fused,
    Rising { t: f32 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Blend {
    /// How far the liquid has spread across the floor, 0..1.
    pub lake: f32,
    /// How far the object has sunk into it, 0..1.
    pub sink: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LakeRipple {
    pub x: f32,
    pub z: f32,
    pub age: f32,
    pub amp: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Hit the floor hard enough to fuse with it.
    Slammed,
    /// Hit the floor, but softly: it splatted and bounced.
    Bounced { speed: f32 },
    Rose,
}

pub struct Fusion {
    pub phase: Phase,
    /// Seconds since the impact, for the rings that spread from it.
    pub since_impact: f32,
    /// Where it landed, on the floor plane.
    pub impact: (f32, f32),
    pub ripples: Vec<LakeRipple>,
    /// Mirror, ice, deep, fountain: what the lake turns into (see mere.rs).
    pub mere: Mere,
    events: Vec<Event>,
}

impl Default for Fusion {
    fn default() -> Self {
        Fusion { phase: Phase::Whole, since_impact: 0.0, impact: (0.0, 0.0), ripples: Vec::new(), mere: Mere::default(), events: Vec::new() }
    }
}

impl Fusion {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    pub fn blend(&self) -> Blend {
        match self.phase {
            Phase::Whole => Blend::default(),
            Phase::Fusing { t } => Blend { lake: smoothstep(0.0, SPREAD_SECS, t), sink: smoothstep(0.05, SINK_SECS, t) },
            Phase::Fused => Blend { lake: 1.0, sink: 1.0 },
            Phase::Rising { t } => Blend {
                lake: 1.0 - smoothstep(0.5, RISE_SECS, t),
                sink: 1.0 - smoothstep(0.0, RISE_SECS * 0.6, t),
            },
        }
    }

    /// Where the body's spring pulls it: the middle of the room, or under the floor once
    /// fused, and further under once the lake has gone deep.
    pub fn home(&self, r: f32) -> V3 {
        let s = self.blend().sink;
        v3(self.impact.0 * s, (FLOOR_Y - (1.4 + 2.5 * self.mere.deep) * r) * s, self.impact.1 * s)
    }

    /// Floor contact for a body at `pos` moving at `vel`, with radius `r` and vertical scale
    /// `sy`. Returns the corrected position and velocity, and a squash impulse.
    pub fn collide(&mut self, pos: V3, vel: V3, r: f32, sy: f32) -> (V3, V3, f32) {
        let bottom = pos.y - r * sy;
        if !self.whole() || bottom >= FLOOR_Y || vel.y >= 0.0 {
            return (pos, vel, 0.0);
        }
        let speed = -vel.y;
        if speed > SLAM_SPEED {
            self.phase = Phase::Fusing { t: 0.0 };
            self.since_impact = 0.0;
            self.impact = (pos.x, pos.z);
            self.ripples.clear();
            self.mere.reset();
            self.events.push(Event::Slammed);
            // It keeps going down, into the floor, but loses most of its speed.
            return (pos, vel * 0.15, -speed * 0.9);
        }
        self.events.push(Event::Bounced { speed });
        let pos = v3(pos.x, FLOOR_Y + r * sy, pos.z);
        (pos, v3(vel.x * 0.7, speed * 0.35, vel.z * 0.7), -speed * 0.8)
    }

    pub fn step(&mut self, dt: f32) {
        self.mere.step(dt, self.phase == Phase::Fused);
        // Frozen, the rings stop where they are.
        if self.mere.frozen() {
            return;
        }
        self.since_impact += dt;
        self.phase = match self.phase {
            Phase::Fusing { t } if t + dt >= SPREAD_SECS => Phase::Fused,
            Phase::Fusing { t } => Phase::Fusing { t: t + dt },
            Phase::Rising { t } if t + dt >= RISE_SECS => {
                self.events.push(Event::Rose);
                Phase::Whole
            }
            Phase::Rising { t } => Phase::Rising { t: t + dt },
            p => p,
        };
        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 5.0);
    }

    /// Where a ray meets the liquid floor, if the floor is liquid there.
    pub fn lake_hit(&self, ro: V3, rd: V3) -> Option<(f32, f32)> {
        if self.whole() || rd.y >= -1e-4 {
            return None;
        }
        let t = (FLOOR_Y - ro.y) / rd.y;
        let p = ro + rd * t;
        let (dx, dz) = (p.x - self.impact.0, p.z - self.impact.1);
        let inside = (dx * dx + dz * dz).sqrt() < self.blend().lake * LAKE_RADIUS;
        inside.then_some((p.x, p.z))
    }

    pub fn touch_lake(&mut self, x: f32, z: f32, amp: f32) {
        self.mere.disturb();
        if self.ripples.len() >= MAX_LAKE_RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push(LakeRipple { x, z, age: 0.0, amp });
    }

    /// Pull it back up out of the floor. Not through ice.
    pub fn rise(&mut self) -> bool {
        if self.phase != Phase::Fused || self.mere.frozen() {
            return false;
        }
        self.mere.rise();
        self.phase = Phase::Rising { t: 0.0 };
        true
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_soft_landing_bounces() {
        let mut f = Fusion::default();
        let (p, v, squash) = f.collide(v3(0.0, -0.5, 0.0), v3(0.0, -3.0, 0.0), 1.0, 1.0);
        assert!(f.whole());
        assert!((p.y - (FLOOR_Y + 1.0)).abs() < 1e-5);
        assert!(v.y > 0.0 && squash < 0.0);
        assert_eq!(f.drain_events(), vec![Event::Bounced { speed: 3.0 }]);
    }

    #[test]
    fn a_hard_landing_fuses_and_the_floor_turns_liquid() {
        let mut f = Fusion::default();
        f.collide(v3(0.3, -0.5, 0.0), v3(0.0, -9.0, 0.0), 1.0, 1.0);
        assert_eq!(f.drain_events(), vec![Event::Slammed]);
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        assert_eq!(f.phase, Phase::Fused);
        assert_eq!(f.blend(), Blend { lake: 1.0, sink: 1.0 });
        assert!(f.home(1.0).y < FLOOR_Y - 1.0, "it rests under the surface");
        // looking down at the floor from above, the ray meets liquid
        assert!(f.lake_hit(v3(0.0, 3.0, 4.0), v3(0.0, -1.0, -0.6).norm()).is_some());
    }

    #[test]
    fn it_rises_back_out() {
        let mut f = Fusion::default();
        f.collide(v3(0.0, -0.5, 0.0), v3(0.0, -9.0, 0.0), 1.0, 1.0);
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        assert!(f.rise());
        for _ in 0..400 {
            f.step(1.0 / 120.0);
        }
        assert!(f.whole());
        assert!(f.drain_events().contains(&Event::Rose));
        assert_eq!(f.home(1.0), V3::ZERO);
    }
}
