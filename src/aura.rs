//! What the words on screen know about the object: its color, how upset it is, and where it sits.
//! The page reads these as CSS variables; web/glyph.js decodes every line through them.

use crate::burst::FATAL_WOUNDS;
use crate::camera::Camera;
use crate::math::*;
use crate::sim::World;

const FLUSH: V3 = v3(0.78, 0.30, 0.42);

pub struct Aura {
    /// The body's color, pushed away from white so it can be read on the backdrop. 0..1 per channel.
    pub tint: V3,
    /// 0..1: agitation, or the wounds that lead to the burst, whichever is louder.
    pub unrest: f32,
    /// Where the body's center lands on screen, as fractions of the canvas (0,0 top-left).
    pub x: f32,
    pub y: f32,
}

pub fn aura(world: &World, cam: &Camera, w: f32, h: f32) -> Aura {
    let c = world.body.color;
    let mean = (c.x + c.y + c.z) / 3.0;
    // Pastels become ink: deepen by day, lighten by night (and in the dark of the fold).
    let base = 0.42 + 0.36 * world.night.max(world.forms.fold.amount());
    let ink = |v: f32| (base + (v - mean) * 2.6).clamp(0.0, 1.0);
    let unrest = world.agitation.max(world.burst.wounds / FATAL_WOUNDS).clamp(0.0, 1.0);
    // Upset, it flushes; so do its words.
    let tint = v3(ink(c.x), ink(c.y), ink(c.z)).lerp(FLUSH, unrest * 0.75);

    let (ux, uy) = cam.project(world.body.pos);
    let x = 0.5 + ux * h / (2.0 * w.max(1.0));
    let y = 0.5 - uy * 0.5;

    Aura { tint, unrest, x: x.clamp(-0.5, 1.5), y: y.clamp(-0.5, 1.5) }
}

/// How many things it has shown you, counted the old way: 7 is "vii".
pub fn roman(mut n: usize) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (v, s) in NUMERALS {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_old_way() {
        assert_eq!(roman(7), "vii");
        assert_eq!(roman(25), "xxv");
        assert_eq!(roman(49), "xlix");
    }

    #[test]
    fn a_calm_pearl_at_rest_sits_near_the_middle_in_grey_ink() {
        let world = World::default();
        let a = aura(&world, &Camera::new(), 1600.0, 900.0);
        assert!(a.x > 0.3 && a.x < 0.7 && a.y > 0.2 && a.y < 0.8, "{} {}", a.x, a.y);
        assert!(a.tint.x < 0.6 && a.unrest < 0.05);
    }
}
