//! Distance to the liquid's surface, its normal, and a ray cast against it, for the pointer.
//! `src/shader.frag` mirrors `sdf()`; change one and change the other.

use super::{Hit, Target, World, K_DROP, K_HANDLE};
use crate::math::*;

impl World {
    /// Distance to the liquid surface, and which object is nearest. Ripples are
    /// left out: they are cosmetic and only exist in the shader.
    pub fn sdf(&self, p: V3) -> (f32, Target) {
        let b = &self.body;
        let r = b.radius();
        let s = b.scale();
        // Once it has burst, or gone to dust, there is no body to touch.
        let mut d = if self.burst.body_gone() || self.forms.dust.body_gone() { 1e3 } else { self.forms.shape(p, b.pos, r, self.division.body_sdf(p, b.pos, r, s)) };
        for h in &self.handles {
            d = smin(d, h.sdf(b, p), K_HANDLE * r);
        }
        let mut target = Target::Body;
        let mut nearest = d;
        for drop in &self.drops {
            let dd = (p - drop.pos).len() - drop.r;
            if dd < nearest {
                nearest = dd;
                target = Target::Drop(drop.id);
            }
            d = smin(d, dd, K_DROP * r);
        }
        (d, target)
    }

    fn normal(&self, p: V3) -> V3 {
        let e = 0.002;
        let f = |o: V3| self.sdf(p + o).0;
        v3(
            f(v3(e, 0.0, 0.0)) - f(v3(-e, 0.0, 0.0)),
            f(v3(0.0, e, 0.0)) - f(v3(0.0, -e, 0.0)),
            f(v3(0.0, 0.0, e)) - f(v3(0.0, 0.0, -e)),
        )
        .norm()
    }

    pub fn raycast(&self, ro: V3, rd: V3) -> Option<Hit> {
        let mut t = 0.0;
        for _ in 0..160 {
            let p = ro + rd * t;
            let (d, target) = self.sdf(p);
            if d < 0.001 {
                return Some(Hit { pos: p, normal: self.normal(p), target });
            }
            t += d * 0.9;
            if t > 20.0 {
                break;
            }
        }
        None
    }
}
