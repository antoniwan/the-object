//! Between the canvas and the world: what the pointer is over, and where a point lands on screen.

use super::*;

impl Game {
    pub(super) fn pick(&self, x: f32, y: f32) -> Option<Hit> {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        self.world.raycast(ro, rd)
    }

    /// Where the pointer is on the camera-facing plane through `through`.
    pub(super) fn plane_point(&self, x: f32, y: f32, through: V3) -> V3 {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        let n = self.cam.basis().fwd;
        let t = (through - ro).dot(n) / rd.dot(n);
        ro + rd * t
    }

    /// Where a point in the world lands on the canvas, in pixels.
    pub(crate) fn screen_of(&self, p: V3) -> (f32, f32) {
        let (ux, uy) = self.cam.project(p);
        ((ux * self.h + self.w) * 0.5, (self.h - uy * self.h) * 0.5)
    }

    /// Where the pointer meets the floor, if it does.
    pub(super) fn floor_point(&self, x: f32, y: f32) -> Option<V3> {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        if rd.y > -0.02 {
            return None;
        }
        Some(ro + rd * ((fusion::FLOOR_Y - ro.y) / rd.y))
    }

    /// How far (0..1) the lens should lean in this frame: while it reaches toward you (watching
    /// you, seeing you, in contact) or is held still changing color, with no finger down. Never
    /// for the swell under a held finger, never with a finger down, so it does not slide out
    /// from under one; only while it is its round, ordinary self, seen from where it rests
    /// (`rest`, the resting distance).
    pub(super) fn lean_goal(&self, rest: f32) -> f32 {
        if self.pointers.iter().any(|p| p.0 >= 0) || !self.world.ordinary() {
            return 0.0;
        }
        let reaching = if self.contact.in_contact() { 1.0 } else { smoothstep(0.3, 0.9, self.world.gaze) };
        let warming = if self.world.warming || self.keys.hum().is_some() { 1.0 } else { 0.0 };
        let framed = 1.0 - smoothstep(0.0, 0.4, (self.cam.dist - rest).abs());
        let round = 1.0 - smoothstep(0.05, 0.3, self.world.body.squeeze.abs());
        reaching.max(warming) * framed * round
    }

    /// Distance at which both cells, side by side, just fit across the screen.
    pub(super) fn cells_dist(&self) -> f32 {
        let aspect = self.w / self.h.max(1.0);
        let half = (meiosis::SEP + meiosis::CELL_SCALE) * self.world.body.radius();
        // Framed for the resting lens: the lens never leans while it is divided.
        (FOCAL * half / (0.82 * aspect)).clamp(2.8, 7.5)
    }
}
