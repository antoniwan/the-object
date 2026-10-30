//! What being cut does to it, besides the cut. Right after a slice the cells are
//! afraid: cold, trembling, their lines fine and restless. Nothing else happens to
//! them, and slowly they find out they are still there. The trembling stops, the
//! lines slow and thicken, and they light from inside. Every new cut frightens
//! them again, from wherever they had got to.
//!
//! Two eased values, never set: the shader draws the cells from them (`uNerve`), so
//! a cut can turn fear on quickly but never makes anything jump.

use crate::math::{ease, smoothstep};

/// Seconds after a cut that it stays afraid before it starts to calm.
const AFRAID_SECS: f32 = 2.5;
/// From this many seconds after a cut it begins to feel its strength...
const NERVE_FROM: f32 = 4.0;
/// ...and has all of it by this many.
const NERVE_FULL: f32 = 10.0;
/// How fast fear arrives, and how slowly it leaves (per second).
const FEAR_IN: f32 = 7.0;
const FEAR_OUT: f32 = 0.7;
/// How fast strength follows where it should be, and how fast a cut takes it away.
const NERVE_IN: f32 = 0.9;
const NERVE_OUT: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nerve {
    since: f32,
    /// 0..1 how afraid.
    pub fear: f32,
    /// 0..1 how sure it is that it is still alive.
    pub power: f32,
}

impl Default for Nerve {
    fn default() -> Self {
        Nerve { since: 1e3, fear: 0.0, power: 0.0 }
    }
}

impl Nerve {
    /// It has just been cut.
    pub fn cut(&mut self) {
        self.since = 0.0;
    }

    pub fn step(&mut self, dt: f32) {
        self.since += dt;
        let afraid = if self.since < AFRAID_SECS { 1.0 } else { 0.0 };
        let fear_rate = if afraid > self.fear { FEAR_IN } else { FEAR_OUT };
        self.fear = ease(self.fear, afraid, -fear_rate * dt);
        let sure = smoothstep(NERVE_FROM, NERVE_FULL, self.since);
        let power_rate = if sure < self.power { NERVE_OUT } else { NERVE_IN };
        self.power = ease(self.power, sure, -power_rate * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(n: &mut Nerve, secs: f32) -> f32 {
        let mut biggest_step = 0.0f32;
        let dt = 1.0 / 60.0;
        for _ in 0..(secs / dt) as usize {
            let (f, p) = (n.fear, n.power);
            n.step(dt);
            biggest_step = biggest_step.max((n.fear - f).abs()).max((n.power - p).abs());
        }
        biggest_step
    }

    #[test]
    fn afraid_at_first_then_sure_of_itself() {
        let mut n = Nerve::default();
        n.cut();
        run(&mut n, 1.5);
        assert!(n.fear > 0.9 && n.power < 0.05, "{n:?}");
        run(&mut n, 13.0);
        assert!(n.fear < 0.05 && n.power > 0.9, "{n:?}");
    }

    #[test]
    fn a_new_cut_frightens_it_again_without_a_jump() {
        let mut n = Nerve::default();
        n.cut();
        run(&mut n, 15.0);
        n.cut();
        let step = run(&mut n, 2.0);
        assert!(n.fear > 0.9 && n.power < 0.05, "{n:?}");
        // nothing moves more than a sliver in one frame
        assert!(step < 0.12, "{step}");
    }
}
