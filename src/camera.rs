//! The orbit camera: where the player looks from, how it drifts, how far the lens leans in, and the screen shake.

use crate::math::*;

/// Focal length at rest. The picker and the shader both use the camera's own `focal`, which
/// only leaves this while the lens leans in.
pub const FOCAL: f32 = 1.9;
/// How far the lens leans in, at most: the focal length it eases toward while it is held still
/// with nobody's hand on it. A fifth of the way with less motion asked for.
pub const LEAN: f32 = 2.3;
const LEAN_STILL: f32 = 0.2;
/// It leans in this slowly (rate a second: most of the way in about five seconds), and back
/// a little faster, so that a finger set down on it stops it soon but never in one frame.
const LEAN_IN: f32 = 0.6;
const LEAN_OUT: f32 = 1.2;
/// The shake dies away at this rate a second, toward a little under nothing (`SHAKE_UNDER`, world
/// units) so that it ends: the hardest (a burst, 0.12) in under a second, a slam in about 0.9 s.
const SHAKE_RATE: f32 = 4.0;
const SHAKE_UNDER: f32 = 0.003;
/// A shock fades at this rate a second, reaching nothing at `SHOCK_SECS`.
const SHOCK_RATE: f32 = 3.0;
const SHOCK_SECS: f32 = 1.5;

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub yaw_vel: f32,
    pub pitch_vel: f32,
    /// Screen shake amplitude, in world units; decays on its own.
    pub shake: f32,
    shake_t: f32,
    /// The last shock, for the shader's flash and ring: how hard (0..1) and seconds since.
    shock: (f32, f32),
    last_shake: f32,
    /// The player asked for less motion: the room shakes a fifth as far, and the flash and
    /// ring of a shock come at 40%.
    pub still: bool,
    /// Focal length now: `FOCAL`, or on the way to `LEAN` and back.
    pub focal: f32,
}

pub struct Basis {
    pub pos: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            yaw: 0.35,
            pitch: 0.22,
            dist: 5.2,
            yaw_vel: 0.0,
            pitch_vel: 0.0,
            shake: 0.0,
            shake_t: 0.0,
            shock: (0.0, 9.0),
            last_shake: 0.0,
            still: false,
            focal: FOCAL,
        }
    }

    /// Each frame: the lens leans in toward `goal` (0..1 of the way to `LEAN`), or back out,
    /// eased; with less motion asked for, only a fifth as far.
    pub fn lean(&mut self, goal: f32, dt: f32) {
        let reach = if self.still { LEAN_STILL } else { 1.0 };
        let want = FOCAL + (LEAN - FOCAL) * goal.clamp(0.0, 1.0) * reach;
        let rate = if want > self.focal { LEAN_IN } else { LEAN_OUT };
        self.focal += (want - self.focal) * (1.0 - (-rate * dt).exp());
    }

    pub fn basis(&self) -> Basis {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let pos = v3(self.dist * cp * self.yaw.sin(), self.dist * sp, self.dist * cp * self.yaw.cos());
        let fwd = (-pos).norm();
        let right = fwd.cross(V3::UP).norm();
        let up = right.cross(fwd);
        let t = self.shake_t;
        let shake = if self.still { self.shake * 0.2 } else { self.shake };
        let pos = pos + right * ((t * 71.0).sin() * shake) + up * ((t * 53.0).cos() * shake);
        Basis { pos, right, up, fwd }
    }

    /// The last shock: how hard (0..1), fading exponentially to nothing over a second and a
    /// half, and seconds since it.
    pub fn shock(&self) -> (f32, f32) {
        let (amount, age) = self.shock;
        let amount = if self.still { amount * 0.4 } else { amount };
        let end = (-SHOCK_RATE * SHOCK_SECS).exp();
        (amount * (((-SHOCK_RATE * age).exp() - end) / (1.0 - end)).max(0.0), age)
    }

    /// Ray through a pixel, with (0,0) at the top-left of a `w` x `h` canvas.
    pub fn ray(&self, px: f32, py: f32, w: f32, h: f32) -> (V3, V3) {
        let b = self.basis();
        let ux = (2.0 * px - w) / h;
        let uy = (h - 2.0 * py) / h;
        (b.pos, (b.right * ux + b.up * uy + b.fwd * self.focal).norm())
    }

    /// Where a point in the room lands on screen, in the units `ray` takes: from the middle of
    /// the canvas, in half its height, y up.
    pub fn project(&self, p: V3) -> (f32, f32) {
        let b = self.basis();
        let d = p - b.pos;
        let z = d.dot(b.fwd).max(0.1);
        (d.dot(b.right) * self.focal / z, d.dot(b.up) * self.focal / z)
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx;
        self.pitch = (self.pitch + dy).clamp(-1.2, 1.3);
    }

    /// Each frame: the shake dies down and is noted as a shock, and, unless held, the view
    /// coasts on after an orbit flick.
    pub fn step(&mut self, dt: f32, held: bool) {
        self.shake_t += dt;
        // Anything that shakes the room is a shock; a slam (0.09) or anything harder is 1.
        if self.shake > self.last_shake + 1e-4 {
            self.shock = ((self.shake / 0.09).min(1.0), 0.0);
        } else {
            self.shock.1 += dt;
        }
        if self.shake > 0.0 {
            self.shake = ((self.shake + SHAKE_UNDER) * (-SHAKE_RATE * dt).exp() - SHAKE_UNDER).max(0.0);
        }
        self.last_shake = self.shake;
        if !held {
            self.orbit(self.yaw_vel * dt, self.pitch_vel * dt);
            let damp = (-3.0 * dt).exp();
            self.yaw_vel *= damp;
            self.pitch_vel *= damp;
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shake_is_a_shock_that_fades() {
        let mut c = Camera::new();
        c.step(0.016, false);
        assert_eq!(c.shock().0, 0.0);
        c.shake = 0.09;
        c.step(0.016, false);
        let (amount, age) = c.shock();
        assert!(amount > 0.95 && age < 0.02);
        // it dies away as everything does: most of it at once, the rest slowly
        let mut seen = vec![c.shock().0];
        for _ in 0..120 {
            c.step(0.016, false);
            seen.push(c.shock().0);
        }
        assert_eq!(c.shock().0, 0.0);
        assert_eq!(c.shake, 0.0);
        let at = |t: f32| seen[(t / 0.016) as usize];
        assert!(at(0.375) < 0.4, "gone fastest at first: {}", at(0.375));
        assert!(at(0.75) > 0.0 && at(1.45) > 0.0 && at(1.45) < 0.01, "and slowly to nothing");
        assert!(seen.windows(2).all(|w| w[1] <= w[0]), "it never comes back");
    }

    #[test]
    fn the_shake_dies_away_rather_than_running_down() {
        let mut c = Camera::new();
        c.shake = 0.12;
        let mut t = 0.0;
        let mut half = None;
        while c.shake > 0.0 {
            c.step(0.016, false);
            t += 0.016;
            if half.is_none() && c.shake < 0.06 {
                half = Some(t);
            }
        }
        // a straight line would take half the time to lose half
        let half = half.unwrap();
        assert!(half < 0.3 * t, "half gone in {half} of {t} s");
        assert!((0.6..1.1).contains(&t), "about as long as it always took: {t} s");
    }

    #[test]
    fn asked_for_less_motion_it_shakes_and_flashes_less() {
        let (mut a, mut b) = (Camera::new(), Camera::new());
        b.still = true;
        for c in [&mut a, &mut b] {
            c.shake = 0.09;
            c.step(0.016, false);
        }
        let rest = Camera::new().basis().pos;
        let moved = |c: &Camera| (c.basis().pos - rest).len();
        assert!(moved(&b) < moved(&a) * 0.25 && moved(&b) > 0.0);
        assert!(b.shock().0 < a.shock().0 * 0.5 && b.shock().0 > 0.0);
    }

    #[test]
    fn the_leaned_lens_still_points_where_it_draws() {
        let mut c = Camera::new();
        c.focal = LEAN;
        let (w, h) = (800.0, 600.0);
        for p in [v3(0.0, 0.0, 0.0), v3(0.7, 0.3, -0.4), v3(-1.1, -0.5, 0.6)] {
            let (ux, uy) = c.project(p);
            let (ro, rd) = c.ray((ux * h + w) * 0.5, (h - uy * h) * 0.5, w, h);
            let off = (p - ro) - rd * (p - ro).dot(rd);
            assert!(off.len() < 1e-4, "{p:?} is {} off its own ray", off.len());
        }
    }

    #[test]
    fn the_lens_leans_slowly_and_comes_back() {
        let mut c = Camera::new();
        let mut last = c.focal;
        for _ in 0..600 {
            c.lean(1.0, 1.0 / 60.0);
            assert!(c.focal >= last && c.focal - last < 0.005, "no jump: {last} to {}", c.focal);
            last = c.focal;
        }
        assert!((c.focal - LEAN).abs() < 0.02, "most of the way in ten seconds: {}", c.focal);
        for _ in 0..600 {
            c.lean(0.0, 1.0 / 60.0);
            assert!(c.focal <= last && last - c.focal < 0.01);
            last = c.focal;
        }
        assert!((c.focal - FOCAL).abs() < 1e-3, "{}", c.focal);
        // with less motion asked for, a fifth of the way
        let mut c = Camera::new();
        c.still = true;
        for _ in 0..1200 {
            c.lean(1.0, 1.0 / 60.0);
        }
        assert!((c.focal - (FOCAL + (LEAN - FOCAL) * 0.2)).abs() < 1e-3, "{}", c.focal);
    }
}
