//! Stir its universe on, the same way, long after it has become one, and what it holds gets
//! out. The floor goes, the white room comes apart into a sky cut by mirrors, and it is
//! repeated in every direction, each copy the mirror of the next, each with the galaxy turning
//! inside. Poke it and the room takes another mirror. Hold still on it and its time runs the
//! other way until you let go. Stir the other way and the room folds back up around one of
//! it, still holding its universe.
//!
//! It starts only from the universe (cosmos.rs), which starts only from the ordinary liquid.
//! Like the other forms, the phase only decides which 0..1 values move; the shader's fold block
//! does the rest by changing coordinates, so there is nothing new for it to march.

use super::Event;
use crate::math::{ease, fade, smoothstep};

/// Turns past the universe, the way it was stirred, that open the fold.
pub const FOLD_TURNS: f32 = 3.0;
/// Turns the other way that close it again.
pub const UNFOLD_TURNS: f32 = 2.0;
const OPEN_SECS: f32 = 4.2;
const CLOSE_SECS: f32 = 3.0;
/// Stop stirring for more than `PAUSE_SECS` and the stir fades, over about `FORGET_SECS`.
const PAUSE_SECS: f32 = 0.6;
const FORGET_SECS: f32 = 3.0;
/// Held this long, its time turns round.
pub const BACKWARD_SECS: f32 = 0.6;
/// How many mirrors the room folds into first, and the most it takes before it starts again.
const FIRST_MIRRORS: u32 = 4;
const MOST_MIRRORS: u32 = 12;
const MIRROR_SECS: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Opening { t: f32 },
    Folded,
    Closing { t: f32 },
}

pub struct Fold {
    pub phase: Phase,
    /// Turns stirred past the universe, signed by the way it was first stirred.
    stirred: f32,
    quiet: f32,
    /// Its own clock: runs forward, and backward while it is held.
    pub clock: f32,
    /// How fast the clock runs, -1..1.
    pace: f32,
    /// Seconds it has been held still.
    held: f32,
    told_back: bool,
    /// The mirror count it had, the one it has now, and how far it has gone from one to the other.
    pub mirrors_was: u32,
    pub mirrors: u32,
    pub mirror_t: f32,
    /// The flash of a new mirror, 0..1.
    pub flash: f32,
    pub(super) events: Vec<Event>,
}

impl Default for Fold {
    fn default() -> Self {
        Fold {
            phase: Phase::Whole,
            stirred: 0.0,
            quiet: 0.0,
            clock: 0.0,
            pace: 1.0,
            held: 0.0,
            told_back: false,
            mirrors_was: FIRST_MIRRORS,
            mirrors: FIRST_MIRRORS,
            mirror_t: 1.0,
            flash: 0.0,
            events: Vec::new(),
        }
    }
}

impl Fold {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    /// 0 = the room, 1 = folded.
    pub fn amount(&self) -> f32 {
        match self.phase {
            Phase::Whole => 0.0,
            Phase::Opening { t } => smoothstep(0.0, OPEN_SECS, t),
            Phase::Folded => 1.0,
            Phase::Closing { t } => 1.0 - smoothstep(0.0, CLOSE_SECS, t),
        }
    }

    /// How fast its time runs: 1 forward, -1 backward.
    pub fn pace(&self) -> f32 {
        self.pace
    }

    /// 0..1, how far its time has turned round.
    pub fn backward(&self) -> f32 {
        smoothstep(1.0, -1.0, self.pace)
    }

    /// Feed it `turns` of stirring while it holds a universe that turns `sense` (+1 or -1).
    /// Returns true if the fold took the stir, so the universe should not.
    pub fn stir(&mut self, turns: f32, sense: f32, universe: bool) -> bool {
        let along = turns * sense;
        match self.phase {
            Phase::Whole if universe => {
                // Only the same way counts toward it; the other way belongs to the universe.
                self.quiet = 0.0;
                self.stirred = (self.stirred + along).max(0.0);
                if self.stirred >= FOLD_TURNS {
                    self.stirred = 0.0;
                    self.mirrors_was = FIRST_MIRRORS;
                    self.mirrors = FIRST_MIRRORS;
                    self.mirror_t = 1.0;
                    self.phase = Phase::Opening { t: 0.0 };
                }
                false
            }
            Phase::Whole => false,
            Phase::Opening { .. } | Phase::Folded => {
                self.quiet = 0.0;
                self.stirred = (self.stirred + along).min(0.0);
                if self.phase == Phase::Folded && self.stirred <= -UNFOLD_TURNS {
                    self.stirred = 0.0;
                    self.phase = Phase::Closing { t: 0.0 };
                }
                true
            }
            Phase::Closing { .. } => true,
        }
    }

    /// A poke: the room takes another mirror. Returns true if it did.
    pub fn facet(&mut self) -> bool {
        if self.phase != Phase::Folded {
            return false;
        }
        self.mirrors_was = self.mirrors;
        self.mirrors = if self.mirrors >= MOST_MIRRORS { 3 } else { self.mirrors + 1 };
        self.mirror_t = 0.0;
        self.flash = 1.0;
        self.events.push(Event::Facet);
        true
    }

    /// Whether it is being held still on, this frame.
    pub fn hold(&mut self, held: bool, dt: f32) {
        self.held = if held && self.phase == Phase::Folded { self.held + dt } else { 0.0 };
    }

    pub fn step(&mut self, dt: f32) {
        self.quiet += dt;
        if self.quiet > PAUSE_SECS {
            self.stirred = fade(self.stirred, -dt / FORGET_SECS);
        }
        // Held, its time slows, stops and turns round; let go, it turns forward again.
        let goal = if self.held > BACKWARD_SECS { -1.0 } else { 1.0 };
        self.pace = ease(self.pace, goal, -1.8 * dt);
        if self.pace < -0.5 && !self.told_back {
            self.told_back = true;
            self.events.push(Event::Backward);
        }
        if self.pace > 0.5 {
            self.told_back = false;
        }
        self.clock += self.pace * dt * self.amount();
        self.flash = fade(self.flash, -2.2 * dt);
        self.mirror_t = (self.mirror_t + dt / MIRROR_SECS).min(1.0);
        self.phase = match self.phase {
            Phase::Opening { t } if t + dt >= OPEN_SECS => {
                self.events.push(Event::Everywhere);
                Phase::Folded
            }
            Phase::Opening { t } => Phase::Opening { t: t + dt },
            Phase::Closing { t } if t + dt >= CLOSE_SECS => {
                self.held = 0.0;
                self.events.push(Event::Crease);
                Phase::Whole
            }
            Phase::Closing { t } => Phase::Closing { t: t + dt },
            p => p,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stir_for(f: &mut Fold, turns: f32, universe: bool) {
        for _ in 0..(turns.abs() * 20.0) as usize {
            f.stir(0.05_f32.copysign(turns), 1.0, universe);
            f.step(1.0 / 60.0);
        }
    }

    fn wait(f: &mut Fold, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            f.step(1.0 / 60.0);
        }
    }

    #[test]
    fn stirred_on_past_the_universe_it_folds_and_stirred_back_it_closes() {
        let mut f = Fold::default();
        stir_for(&mut f, 4.0, false);
        assert!(f.whole(), "no universe, no fold");
        stir_for(&mut f, 3.5, true);
        assert!(!f.whole());
        wait(&mut f, 5.0);
        assert_eq!(f.phase, Phase::Folded);
        assert!(f.events.contains(&Event::Everywhere));
        stir_for(&mut f, 3.0, true);
        assert_eq!(f.phase, Phase::Folded, "more of the same changes nothing");
        stir_for(&mut f, -2.5, true);
        wait(&mut f, 4.0);
        assert!(f.whole());
        assert!(f.events.contains(&Event::Crease));
    }

    #[test]
    fn the_other_way_is_left_to_the_universe() {
        let mut f = Fold::default();
        assert!(!f.stir(-0.5, 1.0, true));
        stir_for(&mut f, -6.0, true);
        assert!(f.whole());
    }

    #[test]
    fn a_poke_adds_a_mirror_and_they_come_round_again() {
        let mut f = Fold { phase: Phase::Folded, ..Fold::default() };
        assert!(f.facet());
        assert_eq!((f.mirrors_was, f.mirrors), (FIRST_MIRRORS, FIRST_MIRRORS + 1));
        for _ in 0..20 {
            f.facet();
        }
        assert!((3..=MOST_MIRRORS).contains(&f.mirrors));
        assert!(!Fold::default().facet());
    }

    #[test]
    fn held_its_time_runs_backward() {
        let mut f = Fold { phase: Phase::Folded, ..Fold::default() };
        wait(&mut f, 2.0);
        let before = f.clock;
        for _ in 0..(3 * 60) {
            f.hold(true, 1.0 / 60.0);
            f.step(1.0 / 60.0);
        }
        assert!(f.events.contains(&Event::Backward));
        assert!(f.clock < before + 0.5, "{} then {}", before, f.clock);
        let back = f.clock;
        for _ in 0..(3 * 60) {
            f.hold(false, 1.0 / 60.0);
            f.step(1.0 / 60.0);
        }
        assert!(f.pace() > 0.9);
        assert!(f.clock > back);
    }
}
