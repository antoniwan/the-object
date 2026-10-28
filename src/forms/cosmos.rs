//! Stir it: pull a tendril round and round the body. The liquid turns with
//! you, and stirred long enough it stops being liquid and holds a universe: a
//! dark glass sphere with a spiral galaxy turning inside, the way you stirred.
//! Poke it and a star ignites at the core. Stir the other way and it settles.

use super::Event;
use crate::math::smoothstep;

/// Turns of stirring (in one direction, without stopping long) that make the universe.
pub const STIR_TURNS: f32 = 2.5;
/// Stop stirring for more than `PAUSE_SECS` and the stir fades, over about `FORGET_SECS`.
const PAUSE_SECS: f32 = 0.6;
const FORGET_SECS: f32 = 3.0;
const FORM_SECS: f32 = 3.5;
const SETTLE_SECS: f32 = 2.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Forming { t: f32 },
    Cosmos,
    Settling { t: f32 },
}

pub struct Cosmos {
    pub phase: Phase,
    /// +1 or -1: which way it was stirred, which is the way its galaxy turns.
    pub sense: f32,
    /// How far the galaxy has turned, in radians.
    pub spin: f32,
    spin_vel: f32,
    /// Turns stirred so far, signed.
    stirred: f32,
    /// Seconds since the last stir.
    quiet: f32,
    told: bool,
    /// A star igniting at the core, 0..1.
    pub flare: f32,
    pub(super) events: Vec<Event>,
}

impl Default for Cosmos {
    fn default() -> Self {
        Cosmos { phase: Phase::Whole, sense: 1.0, spin: 0.0, spin_vel: 0.0, stirred: 0.0, quiet: 0.0, told: false, flare: 0.0, events: Vec::new() }
    }
}

impl Cosmos {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// 0 = liquid, 1 = a universe.
    pub fn amount(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Forming { t } => smoothstep(0.0, FORM_SECS, t),
            Phase::Cosmos => 1.0,
            Phase::Settling { t } => 1.0 - smoothstep(0.0, SETTLE_SECS, t),
        }
    }

    /// Feed it `turns` of stirring (signed: counterclockwise on screen is positive).
    pub fn stir(&mut self, turns: f32) {
        self.stirred += turns;
        self.quiet = 0.0;
        self.spin_vel += turns * 6.0;
        match self.phase {
            Phase::Whole => {
                if !self.told && self.stirred.abs() >= 1.0 {
                    self.told = true;
                    self.events.push(Event::Stirring);
                }
                if self.stirred.abs() >= STIR_TURNS {
                    self.sense = self.stirred.signum();
                    self.stirred = 0.0;
                    self.phase = Phase::Forming { t: 0.0 };
                }
            }
            // Only the opposite turn undoes it; more of the same is not saved up.
            Phase::Forming { .. } | Phase::Cosmos if self.stirred * self.sense > 0.0 => self.stirred = 0.0,
            Phase::Cosmos if self.stirred * self.sense <= -STIR_TURNS => {
                self.stirred = 0.0;
                self.phase = Phase::Settling { t: 0.0 };
            }
            _ => {}
        }
    }

    /// A poke lights a star.
    pub fn ignite(&mut self) {
        if !self.whole() {
            self.flare = 1.0;
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.quiet += dt;
        if self.quiet > PAUSE_SECS {
            self.stirred *= (-dt / FORGET_SECS).exp();
        }
        if self.stirred.abs() < 0.2 {
            self.told = false;
        }
        // It keeps turning on its own, slowly, the way it was stirred.
        let idle = 0.25 * self.sense * self.amount();
        self.spin_vel += (idle - self.spin_vel) * (1.0 - (-0.8 * dt).exp());
        self.spin += self.spin_vel * dt;
        self.flare *= (-1.6 * dt).exp();
        self.phase = match self.phase {
            Phase::Forming { t } if t + dt >= FORM_SECS => {
                self.events.push(Event::Cosmos);
                Phase::Cosmos
            }
            Phase::Forming { t } => Phase::Forming { t: t + dt },
            Phase::Settling { t } if t + dt >= SETTLE_SECS => {
                self.events.push(Event::Settled);
                Phase::Whole
            }
            Phase::Settling { t } => Phase::Settling { t: t + dt },
            p => p,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stir_for(c: &mut Cosmos, turns: f32) {
        for _ in 0..(turns.abs() * 20.0) as usize {
            c.stir(0.05_f32.copysign(turns));
            c.step(1.0 / 60.0);
        }
    }

    #[test]
    fn stirring_makes_a_universe_and_stirring_back_settles_it() {
        let mut c = Cosmos::default();
        stir_for(&mut c, -3.0);
        assert!(c.events.contains(&Event::Stirring));
        assert!(!c.whole());
        assert_eq!(c.sense, -1.0);
        for _ in 0..300 {
            c.step(1.0 / 60.0);
        }
        assert_eq!(c.phase, Phase::Cosmos);
        // more of the same way changes nothing
        stir_for(&mut c, -3.0);
        assert_eq!(c.phase, Phase::Cosmos);
        stir_for(&mut c, 3.0);
        for _ in 0..300 {
            c.step(1.0 / 60.0);
        }
        assert!(c.whole());
        assert!(c.events.contains(&Event::Settled));
    }

    #[test]
    fn a_stir_left_unfinished_is_forgotten() {
        let mut c = Cosmos::default();
        stir_for(&mut c, 1.5);
        for _ in 0..(60 * 10) {
            c.step(1.0 / 60.0);
        }
        stir_for(&mut c, 1.5);
        assert!(c.whole());
    }
}
