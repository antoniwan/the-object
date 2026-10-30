//! Spin the flower: drag round and round it and it turns with your hand, and keeps turning a
//! little after you stop. Turned fast for long enough it grows dizzy and sways on itself, more
//! the longer it goes; let it slow and it rights itself. Turned past that, it goes one of two
//! ways, by which way it was turned:
//!
//! - to the left (counterclockwise on screen): its petals twist into ropes and wind tighter and
//!   tighter, then fly out all at once, unwinding as they go; they lie down over the floor, thin
//!   and are taken back in, until it is the liquid again.
//! - to the right: it lays its petals out flat and whirls, and the floor lets go of it. It hangs
//!   in the air turning, then comes down slowly in a wide spiral, the way a seed comes down from a
//!   tree, and cups its petals again where it lands.
//!
//! Brought to the edge of dizzy and let go of, again and again (three times in a visit), it
//! learns which way you turn it. If it is glad of you (remembrance.rs), it turns on with you by
//! itself once your hand stops; if not, it turns back against you, and is harder to make dizzy.
//!
//! Like the other forms, `Phase` only decides which 0..1 values move; the shader blends them.

use super::Event;
use crate::math::*;

/// How fast its speed (radians a second) dies away once the hand stops feeding it.
const DRAG: f32 = 1.0;
/// Dizziness gathers as speed times `GAIN` and drains as itself times `LEAK`, both a second:
/// a brisk turn a second makes it dizzy in about three; half that only brings it near.
const GAIN: f32 = 0.08;
const LEAK: f32 = 0.3;
/// The hand counts as turning it for this long after its last move.
const HAND_SECS: f32 = 0.5;
/// Dizzier than `EDGE` and back under `CLEAR` without tipping over: nearly.
pub const EDGE: f32 = 0.6;
const CLEAR: f32 = 0.25;
/// Nearly this many times in a visit and it knows which way you turn it.
pub const LEARN_AFTER: u32 = 3;
/// Once it braces against you, dizziness gathers this much slower, and your turn counts this much
/// less while it is against its own.
const BRACED_GAIN: f32 = 0.6;
const BRACED_HAND: f32 = 0.55;
/// How fast it turns on by itself (with you), or back (against you), and for how long it turns back.
const WITH_SPEED: f32 = 1.4;
const BACK_SPEED: f32 = 1.1;
const BACK_SECS: f32 = 3.0;
/// The most it leans while dizzy, in radians.
const SWAY: f32 = 0.38;

/// To the left: wound into ropes, flung out, lying down over the floor, taken back in.
pub const WIND_SECS: f32 = 2.6;
const FLING_SECS: f32 = 0.7;
const FALL_SECS: f32 = 3.0;
pub const GATHER_SECS: f32 = 2.4;
const ROPE_SECS: f32 = WIND_SECS + FLING_SECS + FALL_SECS + GATHER_SECS;
/// When the ropes have lain down and it starts to come back.
pub const LAID_DOWN: f32 = WIND_SECS + FLING_SECS + FALL_SECS;

/// To the right: laid flat, lifted, hanging, coming down, landing.
const FLAT_SECS: f32 = 1.2;
const RISE_SECS: f32 = 2.6;
const HANG_SECS: f32 = 2.0;
const DESCEND_SECS: f32 = 6.0;
const LAND_SECS: f32 = 1.4;
const WHIRL_SECS: f32 = FLAT_SECS + RISE_SECS + HANG_SECS + DESCEND_SECS + LAND_SECS;
/// How high it rises, in body radii, and how wide its spiral down is at the most.
pub const LIFT: f32 = 1.05;
const SPIRAL: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Calm,
    /// Turned left past dizzy.
    Ropes { t: f32 },
    /// Turned right past dizzy.
    Whirl { t: f32 },
}

pub struct Spin {
    pub phase: Phase,
    /// How far the petals have turned, radians, counterclockwise from above positive.
    pub angle: f32,
    vel: f32,
    /// 0 = steady, 1 = tipped over.
    pub dizzy: f32,
    /// The dizziest it has been since it was last steady.
    peak: f32,
    /// Seconds since the hand last turned it.
    hand: f32,
    /// Which way the hand last turned it, +1 or -1.
    sense: f32,
    /// How many times it was brought near dizzy and let go of, this visit.
    pub nearly: u32,
    /// Once learned: which way you turn it, and whether it turns with you (true) or against you.
    pub learned: Option<(f32, bool)>,
    /// Set by the game each frame: whether it is glad of you (remembrance.rs).
    pub glad: bool,
    swayed: bool,
    /// Where its lean points, turning round as it sways.
    lean: f32,
    /// How far round its spiral down it has come.
    round: f32,
    pub(super) events: Vec<Event>,
}

impl Default for Spin {
    fn default() -> Self {
        Spin {
            phase: Phase::Calm,
            angle: 0.0,
            vel: 0.0,
            dizzy: 0.0,
            peak: 0.0,
            hand: 1e3,
            sense: 1.0,
            nearly: 0,
            learned: None,
            glad: false,
            swayed: false,
            lean: 0.0,
            round: 0.0,
            events: Vec::new(),
        }
    }
}

impl Spin {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Calm
    }

    /// Feed it `turns` of the hand round the flower (signed: counterclockwise on screen is positive).
    pub fn turn(&mut self, turns: f32) {
        if self.phase != Phase::Calm || turns == 0.0 {
            return;
        }
        let against = matches!(self.learned, Some((s, false)) if turns * s > 0.0);
        self.vel += turns * std::f32::consts::TAU * DRAG * if against { BRACED_HAND } else { 1.0 };
        self.hand = 0.0;
        self.sense = turns.signum();
    }

    /// `flower`: whether the flower is open, standing, for it to turn.
    pub fn step(&mut self, dt: f32, flower: bool) {
        self.hand += dt;
        self.phase = match self.phase {
            Phase::Calm => self.calm(dt, flower),
            Phase::Ropes { t } => self.ropes(t, dt),
            Phase::Whirl { t } => self.whirl(t, dt),
        };
        self.angle += self.vel * dt;
        // Dizzy, it leans, and the lean goes round.
        self.lean += dt * (2.2 + 1.5 * self.dizzy) * self.sense;
        self.round += dt * 1.6;
    }

    fn calm(&mut self, dt: f32, flower: bool) -> Phase {
        if !flower {
            self.vel = fade(self.vel, -3.0 * dt);
            self.dizzy = fade(self.dizzy, -LEAK * 3.0 * dt);
            self.peak = 0.0;
            return Phase::Calm;
        }
        let turning = self.hand < HAND_SECS;
        match self.learned {
            Some((s, true)) if !turning => self.vel = ease(self.vel, WITH_SPEED * s, -1.2 * dt),
            Some((s, false)) if !turning && self.hand < BACK_SECS => self.vel = ease(self.vel, -BACK_SPEED * s, -1.5 * dt),
            _ => self.vel = fade(self.vel, -DRAG * dt),
        }
        let gain = if matches!(self.learned, Some((_, false))) { GAIN * BRACED_GAIN } else { GAIN };
        let fed = if turning { gain * self.vel.abs() } else { 0.0 };
        self.dizzy = (self.dizzy + dt * (fed - LEAK * self.dizzy)).max(0.0);
        self.peak = self.peak.max(self.dizzy);
        if !self.swayed && self.dizzy > 0.5 {
            self.swayed = true;
            self.events.push(Event::Swaying);
        }
        if self.dizzy >= 1.0 {
            self.swayed = false;
            self.peak = 0.0;
            if self.sense > 0.0 {
                self.events.push(Event::Winding);
                return Phase::Ropes { t: 0.0 };
            }
            self.events.push(Event::Whirling);
            return Phase::Whirl { t: 0.0 };
        }
        if self.dizzy < CLEAR {
            self.swayed = false;
            if self.peak >= EDGE {
                self.nearly += 1;
                self.events.push(Event::Nearly);
                if self.nearly >= LEARN_AFTER && self.learned.is_none() {
                    self.learned = Some((self.sense, self.glad));
                    self.events.push(Event::Learned { with: self.glad });
                }
            }
            self.peak = 0.0;
        }
        Phase::Calm
    }

    fn ropes(&mut self, t: f32, dt: f32) -> Phase {
        let t2 = t + dt;
        // winding faster and faster, then the other way as the ropes come undone
        let goal = if t2 < WIND_SECS { 4.0 + 12.0 * smoothstep(0.0, WIND_SECS, t2) } else if t2 < LAID_DOWN { -5.0 } else { 0.0 };
        self.vel = ease(self.vel, goal, -1.6 * dt);
        self.dizzy = fade(self.dizzy, -1.2 * dt);
        if t < WIND_SECS && t2 >= WIND_SECS {
            self.events.push(Event::Flung);
        }
        if t < LAID_DOWN && t2 >= LAID_DOWN {
            self.events.push(Event::LaidDown);
        }
        if t2 >= ROPE_SECS {
            self.vel = 0.0;
            self.angle = 0.0;
            self.events.push(Event::Unwound);
            return Phase::Calm;
        }
        Phase::Ropes { t: t2 }
    }

    fn whirl(&mut self, t: f32, dt: f32) -> Phase {
        let t2 = t + dt;
        let down = FLAT_SECS + RISE_SECS + HANG_SECS;
        let goal = if t2 < down { -11.0 } else if t2 < down + DESCEND_SECS { -11.0 + 7.5 * smoothstep(down, down + DESCEND_SECS, t2) } else { 0.0 };
        self.vel = ease(self.vel, goal, -1.1 * dt);
        self.dizzy = fade(self.dizzy, -1.2 * dt);
        let off = FLAT_SECS + 0.3 * RISE_SECS;
        if t < off && t2 >= off {
            self.events.push(Event::Aloft);
        }
        if t2 >= WHIRL_SECS {
            self.events.push(Event::Landed);
            return Phase::Calm;
        }
        Phase::Whirl { t: t2 }
    }

    /// The ropes: how far wound (0..1), how far flung out, how far lain down, how much of them is left.
    pub fn rope(&self) -> (f32, f32, f32, f32) {
        let Phase::Ropes { t } = self.phase else { return (0.0, 0.0, 0.0, 1.0) };
        let wound = smoothstep(0.0, WIND_SECS, t) * (1.0 - 0.85 * smoothstep(WIND_SECS, LAID_DOWN, t));
        let flung = smoothstep(WIND_SECS, WIND_SECS + FLING_SECS, t);
        let down = smoothstep(WIND_SECS + 0.3 * FLING_SECS, WIND_SECS + FLING_SECS + 0.8 * FALL_SECS, t);
        let left = 1.0 - smoothstep(LAID_DOWN - 0.6, ROPE_SECS - 0.4, t);
        (wound, flung, down, left)
    }

    /// Laid out flat to whirl, 0..1.
    pub fn flat(&self) -> f32 {
        let Phase::Whirl { t } = self.phase else { return 0.0 };
        smoothstep(0.0, FLAT_SECS, t) * (1.0 - smoothstep(WHIRL_SECS - LAND_SECS, WHIRL_SECS, t))
    }

    /// Where it is above where it stands, in body radii: up, and round its spiral down.
    pub fn offset(&self) -> V3 {
        let Phase::Whirl { t } = self.phase else { return V3::ZERO };
        let up = FLAT_SECS + RISE_SECS + HANG_SECS;
        let lift = LIFT * smoothstep(FLAT_SECS, FLAT_SECS + RISE_SECS, t) * (1.0 - smoothstep(up, up + DESCEND_SECS, t));
        let wide = SPIRAL * smoothstep(up - 0.5, up + 1.5, t) * (1.0 - smoothstep(up + 0.5 * DESCEND_SECS, up + DESCEND_SECS, t));
        v3(wide * self.round.cos(), lift, wide * self.round.sin())
    }

    /// Its lean while dizzy: small turns about x and z, radians.
    pub fn sway(&self) -> (f32, f32) {
        let a = SWAY * smoothstep(0.12, 1.0, self.dizzy);
        (a * self.lean.cos(), a * self.lean.sin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turn it `turns_per_sec` for `secs`, at 60 frames a second.
    fn turn_for(s: &mut Spin, turns_per_sec: f32, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            s.turn(turns_per_sec / 60.0);
            s.step(1.0 / 60.0, true);
        }
    }

    fn rest(s: &mut Spin, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            s.step(1.0 / 60.0, true);
        }
    }

    #[test]
    fn turned_left_past_dizzy_it_winds_into_ropes_and_comes_undone() {
        let mut s = Spin::default();
        turn_for(&mut s, 1.0, 1.0);
        assert!(s.whole() && s.dizzy > 0.1 && s.sway().0.abs() + s.sway().1.abs() > 0.0, "it sways");
        turn_for(&mut s, 1.0, 4.0);
        assert!(matches!(s.phase, Phase::Ropes { .. }));
        assert!(s.events.contains(&Event::Swaying) && s.events.contains(&Event::Winding));
        rest(&mut s, WIND_SECS + 0.8);
        assert!(s.events.contains(&Event::Flung) && s.rope().1 > 0.99);
        rest(&mut s, 10.0);
        assert!(s.whole() && s.events.contains(&Event::Unwound));
        assert_eq!(s.rope(), (0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn turned_right_past_dizzy_it_rises_and_comes_back_down() {
        let mut s = Spin::default();
        turn_for(&mut s, -1.0, 5.0);
        assert!(matches!(s.phase, Phase::Whirl { .. }));
        rest(&mut s, 5.0);
        assert!(s.events.contains(&Event::Aloft));
        assert!(s.offset().y > LIFT * 0.9 && s.flat() > 0.99);
        rest(&mut s, 10.0);
        assert!(s.whole() && s.events.contains(&Event::Landed));
        assert_eq!(s.offset(), V3::ZERO);
    }

    #[test]
    fn a_slow_turn_never_tips_it() {
        let mut s = Spin::default();
        turn_for(&mut s, 0.4, 20.0);
        assert!(s.whole() && s.dizzy < 1.0);
    }

    #[test]
    fn brought_near_dizzy_three_times_it_learns_your_turn() {
        let mut s = Spin { glad: true, ..Spin::default() };
        for _ in 0..LEARN_AFTER {
            turn_for(&mut s, 1.0, 2.7);
            assert!(s.whole() && s.dizzy > EDGE);
            rest(&mut s, 6.0);
        }
        assert_eq!(s.nearly, LEARN_AFTER);
        assert_eq!(s.learned, Some((1.0, true)));
        assert!(s.events.contains(&Event::Learned { with: true }));
        // with you: it keeps turning your way by itself, and not dizzily
        turn_for(&mut s, 0.5, 0.5);
        rest(&mut s, 6.0);
        assert!(s.vel > 1.0 && s.dizzy < 0.3);
    }

    #[test]
    fn not_glad_of_you_it_turns_back_and_is_harder_to_tip() {
        let mut s = Spin { learned: Some((1.0, false)), ..Spin::default() };
        turn_for(&mut s, 1.0, 6.0);
        assert!(s.whole(), "what tipped it before does not now");
        rest(&mut s, 1.5);
        assert!(s.vel < 0.0, "it turns back against you");
    }
}
