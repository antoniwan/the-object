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
}

pub struct Basis {
    pub pos: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
}

impl Camera {
    pub fn new() -> Camera {
        Camera { yaw: 0.35, pitch: 0.22, dist: 5.2, yaw_vel: 0.0, pitch_vel: 0.0, shake: 0.0, shake_t: 0.0 }
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
        self.shake = (self.shake - dt * 0.12).max(0.0);
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
