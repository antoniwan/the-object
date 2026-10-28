//! The orbit camera: where the player looks from, how it drifts, and the screen shake.

use crate::math::*;

/// Focal length used by both the picker and the shader.
pub const FOCAL: f32 = 1.9;

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
}

pub struct Basis {
    pub pos: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
}

impl Camera {
    pub fn new() -> Camera {
        Camera { yaw: 0.35, pitch: 0.22, dist: 5.2, yaw_vel: 0.0, pitch_vel: 0.0, shake: 0.0, shake_t: 0.0, shock: (0.0, 9.0), last_shake: 0.0 }
    }

    pub fn basis(&self) -> Basis {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let pos = v3(self.dist * cp * self.yaw.sin(), self.dist * sp, self.dist * cp * self.yaw.cos());
        let fwd = (-pos).norm();
        let right = fwd.cross(V3::UP).norm();
        let up = right.cross(fwd);
        let t = self.shake_t;
        let pos = pos + right * ((t * 71.0).sin() * self.shake) + up * ((t * 53.0).cos() * self.shake);
        Basis { pos, right, up, fwd }
    }

    /// The last shock: how hard (0..1), fading over a second and a half, and seconds since it.
    pub fn shock(&self) -> (f32, f32) {
        let (amount, age) = self.shock;
        (amount * (1.0 - age / 1.5).max(0.0), age)
    }

    /// Ray through a pixel, with (0,0) at the top-left of a `w` x `h` canvas.
    pub fn ray(&self, px: f32, py: f32, w: f32, h: f32) -> (V3, V3) {
        let b = self.basis();
        let ux = (2.0 * px - w) / h;
        let uy = (h - 2.0 * py) / h;
        (b.pos, (b.right * ux + b.up * uy + b.fwd * FOCAL).norm())
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx;
        self.pitch = (self.pitch + dy).clamp(-1.2, 1.3);
    }

    /// Coast after an orbit flick.
    pub fn step(&mut self, dt: f32, held: bool) {
        self.shake_t += dt;
        // Anything that shakes the room is a shock; the hardest so far (a burst, 0.12) is 1.
        if self.shake > self.last_shake + 1e-4 {
            self.shock = ((self.shake / 0.09).min(1.0), 0.0);
        } else {
            self.shock.1 += dt;
        }
        self.shake = (self.shake - dt * 0.12).max(0.0);
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
        for _ in 0..120 {
            c.step(0.016, false);
        }
        assert_eq!(c.shock().0, 0.0);
    }
}
