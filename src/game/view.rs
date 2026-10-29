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
        self.screen(p)
    }

    /// Where a point in the world lands on the canvas, in pixels.
    pub(super) fn screen(&self, p: V3) -> (f32, f32) {
        let b = self.cam.basis();
        let d = p - b.pos;
        let z = d.dot(b.fwd).max(0.1);
        let (ux, uy) = (d.dot(b.right) * FOCAL / z, d.dot(b.up) * FOCAL / z);
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

    /// Distance at which both cells, side by side, just fit across the screen.
    pub(super) fn cells_dist(&self) -> f32 {
        let aspect = self.w / self.h.max(1.0);
        let half = (meiosis::SEP + meiosis::CELL_SCALE) * self.world.body.radius();
        (FOCAL * half / (0.82 * aspect)).clamp(2.8, 7.5)
    }
}
