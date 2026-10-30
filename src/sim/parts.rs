//! The pieces of the liquid: the body, the tendrils pulled out of it, ripples, free drops,
//! and what a ray hits and what a frame reports.

use super::TEAR_RATIO;
use crate::math::*;

pub struct Body {
    pub pos: V3,
    pub vel: V3,
    /// Volume in units of radius cubed, so radius = cbrt(volume).
    pub volume: f32,
    /// Vertical scale; horizontal scale is 1/sqrt(squash), so volume is preserved.
    pub squash: f32,
    pub squash_vel: f32,
    /// Squeeze input in -1..1, set by the player every frame while squeezing.
    pub squeeze: f32,
    pub color: V3,
}

impl Body {
    pub fn radius(&self) -> f32 {
        self.volume.cbrt()
    }
    pub fn scale(&self) -> V3 {
        let s = 1.0 / self.squash.sqrt();
        v3(s, self.squash, s)
    }
}

/// How much taller its squash spring wants it while it moves fast up or down: a little for
/// each unit of vertical speed, never past a quarter, and less the more it is held squeezed.
pub(super) fn flight_stretch(vel_y: f32, squeeze: f32) -> f32 {
    (0.04 * vel_y.abs()).min(0.25) * (1.0 - squeeze.abs()).max(0.0)
}

/// A tendril of liquid pulled out of the body.
pub struct Handle {
    pub id: u32,
    /// Direction from the body center the tendril grows from.
    pub dir: V3,
    pub tip: V3,
    pub vel: V3,
    /// Where the player's pointer wants the tip to be, while held.
    pub target: Option<V3>,
    /// 0..1, how much of it has come out of the body: at 0 the surface is round, as if it
    /// were not there. It grows in once pulled and sinks back before it goes (`step_handles`).
    pub weight: f32,
    /// How fast it is sinking back, a second; 0 while it stays.
    pub fade: f32,
}

impl Handle {
    pub fn new(id: u32, dir: V3, at: V3) -> Handle {
        Handle { id, dir, tip: at, vel: V3::ZERO, target: Some(at), weight: 0.0, fade: 0.0 }
    }
    /// Where its tip shows: drawn in toward the anchor, deep inside, while only part of it
    /// is out. At weight 0 it is a point too far under the surface to raise it.
    pub fn shown_tip(&self, b: &Body) -> V3 {
        let a = self.anchor(b);
        a + (self.tip - a) * self.weight.clamp(0.0, 1.0)
    }
    pub fn anchor(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.45)
    }
    pub fn rest(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.8)
    }
    pub fn stretch(&self, b: &Body) -> f32 {
        (self.tip - self.rest(b)).len() / b.radius()
    }
    /// (base radius, neck radius, bulb radius). The neck thins as it stretches; all of them
    /// shrink to nothing as it sinks back.
    pub fn radii(&self, b: &Body) -> (f32, f32, f32) {
        let r = b.radius() * self.weight.clamp(0.0, 1.0);
        let s = self.stretch(b).min(TEAR_RATIO);
        let neck = r * (0.24 - 0.13 * s).max(0.05);
        (r * 0.42, neck, r * 0.27)
    }
    pub fn sdf(&self, b: &Body, p: V3) -> f32 {
        let (base, neck, bulb) = self.radii(b);
        let tip = self.shown_tip(b);
        let stem = sd_taper(p, self.anchor(b), tip, base, neck);
        smin(stem, (p - tip).len() - bulb, 0.12 * b.radius())
    }
}

pub struct Ripple {
    pub dir: V3,
    pub amp: f32,
    pub age: f32,
}

pub struct Drop {
    pub id: u32,
    pub pos: V3,
    pub vel: V3,
    pub r: f32,
    pub target: Option<V3>,
    pub age: f32,
    pub color: V3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Body,
    Drop(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub pos: V3,
    pub normal: V3,
    pub target: Target,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A tendril pinched off into a new drop.
    Tore { handle: u32, drop: u32 },
    /// A drop fell back into the body.
    MergedIntoBody { drop: u32 },
    /// Two drops became one; `absorbed` no longer exists, `into` does.
    MergedDrops { absorbed: u32, into: u32 },
    /// Two different colors became one.
    Mixed,
    /// Poked too often, too fast.
    Provoked,
    /// Enough of it is free that the world turns to night.
    Opened,
    /// Everything came back; day returns.
    Closed,
}
