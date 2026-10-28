//! Tiny vector math, so the simulation has no dependencies and runs natively in tests.

use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub const UP: V3 = v3(0.0, 1.0, 0.0);

    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self * (1.0 / l)
        } else {
            V3::UP
        }
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
    /// Component by component.
    pub fn cdiv(self, o: V3) -> V3 {
        v3(self.x / o.x, self.y / o.y, self.z / o.z)
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for V3 {
    fn add_assign(&mut self, o: V3) {
        *self = *self + o;
    }
}
impl SubAssign for V3 {
    fn sub_assign(&mut self, o: V3) {
        *self = *self - o;
    }
}
impl MulAssign<f32> for V3 {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

/// Hermite ease from 0 at `a` to 1 at `b`, as in GLSL.
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Polynomial smooth minimum: the "liquid" join between two shapes.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Distance to a segment whose radius tapers from `ra` at `a` to `rb` at `b`.
pub fn sd_taper(p: V3, a: V3, b: V3, ra: f32, rb: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = (pa.dot(ba) / ba.dot(ba).max(1e-6)).clamp(0.0, 1.0);
    (pa - ba * h).len() - (ra + (rb - ra) * h)
}

/// `x * e^by`, for `by <= 0`: something dying away. Bit for bit the same as writing it
/// out, but a value that is already zero skips the exponential.
#[inline]
pub fn fade(x: f32, by: f32) -> f32 {
    if x == 0.0 {
        x
    } else {
        x * by.exp()
    }
}

/// `x + (goal - x) * (1 - e^by)`, for `by <= 0`: easing toward `goal`. Bit for bit the same
/// as writing it out (a zero gap times a factor in 0..1 is the same zero), but a value that
/// has arrived skips the exponential.
#[inline]
pub fn ease(x: f32, goal: f32, by: f32) -> f32 {
    let d = goal - x;
    if d == 0.0 {
        x + d
    } else {
        x + d * (1.0 - by.exp())
    }
}
