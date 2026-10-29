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

/// A tendril of liquid pulled out of the body.
pub struct Handle {
    pub id: u32,
    /// Direction from the body center the tendril grows from.
    pub dir: V3,
    pub tip: V3,
    pub vel: V3,
    /// Where the player's pointer wants the tip to be, while held.
    pub target: Option<V3>,
}

impl Handle {
    pub fn anchor(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.45)
    }
    pub fn rest(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.8)
    }
    pub fn stretch(&self, b: &Body) -> f32 {
        (self.tip - self.rest(b)).len() / b.radius()
    }
    /// (base radius, neck radius, bulb radius). The neck thins as it stretches.
    pub fn radii(&self, b: &Body) -> (f32, f32, f32) {
        let r = b.radius();
        let s = self.stretch(b).min(TEAR_RATIO);
        let neck = r * (0.24 - 0.13 * s).max(0.05);
        (r * 0.42, neck, r * 0.27)
    }
    pub fn sdf(&self, b: &Body, p: V3) -> f32 {
        let (base, neck, bulb) = self.radii(b);
        let stem = sd_taper(p, self.anchor(b), self.tip, base, neck);
        smin(stem, (p - self.tip).len() - bulb, 0.12 * b.radius())
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
