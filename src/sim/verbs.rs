//! What a hand can do to the liquid: poke it, lean it toward something, pull a tendril
//! out of it, carry a drop and let it go.

use super::{Event, Handle, Hit, Target, World, MAX_HANDLES, PROVOKE_POKES, PROVOKE_WINDOW, STUMP_FADE, STUMP_SINK};
use crate::math::*;

/// How fast a let-go stump swings back and forth about its rest, radians a second (`step_handles`).
const STUMP_SWING: f32 = 10.5;

impl World {
    fn handle_mut(&mut self, id: u32) -> Option<&mut Handle> {
        self.handles.iter_mut().find(|h| h.id == id)
    }

    pub fn poke(&mut self, hit: Hit, strength: f32) {
        match hit.target {
            Target::Body => {
                let now = self.time;
                self.recent_pokes.retain(|t| now - *t < PROVOKE_WINDOW);
                self.recent_pokes.push(now);
                if self.recent_pokes.len() >= PROVOKE_POKES && self.agitation < 0.5 {
                    self.events.push(Event::Provoked);
                }
                if self.recent_pokes.len() >= PROVOKE_POKES {
                    self.agitation = 1.0;
                }
                // Touching it startles it out of watching.
                self.gaze *= 0.2;
                // Upset, it jumps further, but its shape stays its own.
                // On two cells, ripples spread from the touched cell's own center.
                let dir = self.division.ripple_dir(hit.pos, self.body.pos, self.body.radius());
                self.add_ripple(dir, strength);
                self.body.vel -= dir * (0.9 * strength * (1.0 + self.agitation));
                // Poking from above flattens it, from the side makes it bob up.
                self.body.squash_vel += if dir.y.abs() > 0.6 { -4.0 } else { 2.5 } * strength;
            }
            Target::Drop(id) => {
                if let Some(d) = self.drop_mut(id) {
                    d.vel -= hit.normal * (1.6 * strength);
                }
            }
        }
    }

    /// Lean briefly toward something.
    pub fn reach(&mut self, toward: V3) {
        self.gaze_dir = toward.norm();
        self.gaze = self.gaze.max(0.7);
    }

    pub fn begin_stretch(&mut self, at: V3) -> Option<u32> {
        // those already sinking back are on their way out
        if self.handles.iter().filter(|h| h.fade == 0.0).count() >= MAX_HANDLES {
            // Of the stumps let go of, if any, the one nearest settled sinks back to make
            // way, so nothing vanishes in one frame; the new one waits for it, unseen,
            // before it grows (`step_handles`).
            let body = &self.body;
            // how far it will still swing out, where it is and how fast it is going together
            let unsettled = |h: &Handle| (h.tip - h.rest(body)).len().hypot(h.vel.len() / STUMP_SWING);
            let h = self
                .handles
                .iter_mut()
                .filter(|h| h.target.is_none() && h.fade == 0.0)
                .min_by(|a, b| unsettled(a).total_cmp(&unsettled(b)))?;
            h.fade = STUMP_FADE;
        }
        let id = self.id();
        let dir = (at - self.body.pos).norm();
        self.handles.push(Handle::new(id, dir, at));
        Some(id)
    }

    pub fn pull_handle(&mut self, id: u32, to: V3) {
        if let Some(h) = self.handle_mut(id) {
            h.target = Some(to);
        }
    }

    pub fn release_handle(&mut self, id: u32) {
        if let Some(h) = self.handle_mut(id) {
            h.target = None;
        }
    }

    /// Has a stretch fade where it is, over a moment, instead of springing back: for when
    /// something else takes its place.
    pub fn sink_handle(&mut self, id: u32) {
        if let Some(h) = self.handle_mut(id) {
            h.target = Some(h.tip);
            h.fade = STUMP_SINK;
        }
    }

    pub fn grab_drop(&mut self, id: u32, to: V3) {
        if let Some(d) = self.drop_mut(id) {
            d.target = Some(to);
        }
    }

    pub fn release_drop(&mut self, id: u32) -> f32 {
        match self.drop_mut(id) {
            Some(d) => {
                d.target = None;
                d.vel.len()
            }
            None => 0.0,
        }
    }
}
