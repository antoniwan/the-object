//! What the floor becomes once the object is in it (see fusion.rs).
//!
//! The lake is not one thing. Left untouched, it stills into a mirror, and in
//! the mirror the object is still standing. Held down, it freezes; tapped, the
//! ice cracks until it breaks. Pushed down, it goes deep and dark. Struck three
//! times in one place, it throws something back up. And whatever the floor was
//! when the object is pulled back out, the object comes out made of it:
//! quicksilver from the mirror, ink from the deep.
//!
//! Like fusion.rs, nothing here jumps: every state is a value gliding 0..1
//! that the shader blends.

use crate::math::smoothstep;

/// Untouched this long (seconds), the lake begins to still.
const STILL_AFTER: f32 = 6.0;
const STILL_SECS: f32 = 2.5;
/// Holding a press still on the lake this long (seconds) freezes it.
pub const FREEZE_HOLD: f32 = 0.9;
/// How fast the ice spreads from the touch, in world units per second.
const FREEZE_SPEED: f32 = 7.0;
pub const MAX_CRACKS: usize = 6;
/// The ice breaks on this crack.
pub const CRACKS_TO_BREAK: usize = 5;
const THAW_SECS: f32 = 0.8;
const DEEP_SECS: f32 = 2.2;
const FOUNTAIN_TAPS: usize = 3;
/// Taps this close in time (seconds) and space (world units) count as one place.
const FOUNTAIN_WINDOW: f32 = 1.4;
const FOUNTAIN_REACH: f32 = 0.8;
/// The column climbs and falls back in this long; the drop it throws flies longer.
const COLUMN_SECS: f32 = 0.9;
const COLUMN_HEIGHT: f32 = 1.5;
const PINCH_AT: f32 = 0.4;
const DROP_UP: f32 = 3.5;
const GRAVITY: f32 = 9.0;
/// How long the object keeps the material it rose as, fading all the way.
const COAT_SECS: f32 = 75.0;

/// What the object came back out as.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coat {
    Pearl,
    Quicksilver,
    Ink,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crack {
    pub x: f32,
    pub z: f32,
    pub age: f32,
    /// Turns the spokes so no two cracks look alike.
    pub seed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fountain {
    pub x: f32,
    pub z: f32,
    pub t: f32,
    splashed: bool,
}

impl Fountain {
    /// Height of the column above the floor, and of the drop it threw (negative: gone).
    pub fn pose(&self) -> (f32, f32) {
        let t = self.t;
        let column = if t < COLUMN_SECS { COLUMN_HEIGHT * (std::f32::consts::PI * t / COLUMN_SECS).sin() } else { 0.0 };
        let drop = if t < PINCH_AT {
            column
        } else {
            let s = t - PINCH_AT;
            let top = COLUMN_HEIGHT * (std::f32::consts::PI * PINCH_AT / COLUMN_SECS).sin();
            top + DROP_UP * s - 0.5 * GRAVITY * s * s
        };
        (column.max(0.0), drop)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The lake went still enough to be a mirror.
    Stilled,
    Froze,
    Cracked,
    /// The ice broke. Its cracks are still in `cracks` until it has thawed.
    Shattered,
    Sank,
    Fountain,
    /// The fountain's drop fell back into the lake here.
    Splash { x: f32, z: f32 },
}

pub struct Mere {
    /// Seconds since the lake was last touched.
    quiet: f32,
    /// 0..1, how much of a mirror it is.
    pub still: f32,
    /// 0..1, how deep and dark it has gone.
    pub deep: f32,
    sinking: bool,
    /// 0..1, how much ice there is, and where it started and how far it reaches.
    pub ice: f32,
    pub ice_at: (f32, f32),
    pub ice_reach: f32,
    thawing: bool,
    pub cracks: Vec<Crack>,
    pub fountain: Option<Fountain>,
    taps: Vec<(f32, f32, f32)>,
    clock: f32,
    /// What it last rose as, and how much of that is left on it, 0..1.
    pub coat: Coat,
    coat_left: f32,
    events: Vec<Event>,
}

impl Default for Mere {
    fn default() -> Self {
        Mere {
            quiet: 0.0,
            still: 0.0,
            deep: 0.0,
            sinking: false,
            ice: 0.0,
            ice_at: (0.0, 0.0),
            ice_reach: 0.0,
            thawing: false,
            cracks: Vec::new(),
            fountain: None,
            taps: Vec::new(),
            clock: 0.0,
            coat: Coat::Pearl,
            coat_left: 0.0,
            events: Vec::new(),
        }
    }
}

impl Mere {
    /// A fresh lake, for a fresh slam. What the object is made of stays.
    pub fn reset(&mut self) {
        let (coat, coat_left) = (self.coat, self.coat_left);
        *self = Mere { coat, coat_left, ..Mere::default() };
    }

    /// Frozen hard enough that nothing moves in it and nothing comes up through it.
    pub fn frozen(&self) -> bool {
        self.ice > 0.5 && !self.thawing
    }

    /// Any touch breaks the stillness.
    pub fn disturb(&mut self) {
        self.quiet = 0.0;
    }

    /// `lake` is true while the lake is whole and resting (not spreading or draining).
    pub fn step(&mut self, dt: f32, lake: bool) {
        self.clock += dt;
        self.quiet += dt;
        self.coat_left = (self.coat_left - dt / COAT_SECS).max(0.0);

        let calm = lake && self.quiet > STILL_AFTER && !self.frozen() && self.fountain.is_none();
        if calm {
            let was = self.still;
            self.still = (self.still + dt / STILL_SECS).min(1.0);
            if was < 1.0 && self.still >= 1.0 {
                self.events.push(Event::Stilled);
            }
        } else {
            // a touch shatters a mirror much faster than it formed
            self.still = (self.still - dt / 0.35).max(0.0);
        }

        if self.sinking && lake {
            self.deep = (self.deep + dt / DEEP_SECS).min(1.0);
        } else if !lake {
            self.deep = (self.deep - dt / 1.5).max(0.0);
        }

        if self.ice > 0.0 {
            self.ice_reach += FREEZE_SPEED * dt;
            if self.thawing || !lake {
                self.ice = (self.ice - dt / THAW_SECS).max(0.0);
                if self.ice <= 0.0 {
                    self.thawing = false;
                    self.cracks.clear();
                    self.ice_reach = 0.0;
                }
            }
        }
        for c in &mut self.cracks {
            c.age += dt;
        }

        if let Some(f) = &mut self.fountain {
            f.t += dt;
            let (_, drop) = f.pose();
            if !f.splashed && f.t > PINCH_AT && drop < 0.0 {
                f.splashed = true;
                self.events.push(Event::Splash { x: f.x, z: f.z });
            }
            if f.splashed && f.t > COLUMN_SECS {
                self.fountain = None;
            }
        }
        self.taps.retain(|t| self.clock - t.2 < FOUNTAIN_WINDOW);
    }

    /// A tap on the liquid. Three in one place and it throws one back.
    pub fn tap(&mut self, x: f32, z: f32) {
        self.disturb();
        if self.frozen() || self.fountain.is_some() {
            return;
        }
        self.taps.push((x, z, self.clock));
        if self.taps.len() < FOUNTAIN_TAPS {
            return;
        }
        let n = self.taps.len() as f32;
        let (cx, cz) = self.taps.iter().fold((0.0, 0.0), |a, t| (a.0 + t.0 / n, a.1 + t.1 / n));
        let near = self.taps.iter().all(|t| ((t.0 - cx).powi(2) + (t.1 - cz).powi(2)).sqrt() < FOUNTAIN_REACH);
        if near {
            self.taps.clear();
            self.fountain = Some(Fountain { x: cx, z: cz, t: 0.0, splashed: false });
            self.events.push(Event::Fountain);
        } else {
            self.taps.remove(0);
        }
    }

    /// A finger held still on the lake: it freezes outward from there.
    pub fn freeze(&mut self, x: f32, z: f32) -> bool {
        if self.ice > 0.0 {
            return false;
        }
        self.disturb();
        self.ice = 1.0;
        self.ice_at = (x, z);
        self.ice_reach = 0.0;
        self.cracks.clear();
        self.events.push(Event::Froze);
        true
    }

    /// A blow to the ice. Enough of them and it breaks back into liquid.
    pub fn crack(&mut self, x: f32, z: f32) {
        if !self.frozen() {
            return;
        }
        if self.cracks.len() >= MAX_CRACKS {
            self.cracks.remove(0);
        }
        let seed = (self.clock * 7.31 + x * 3.7 + z * 1.3).fract();
        self.cracks.push(Crack { x, z, age: 0.0, seed });
        self.events.push(Event::Cracked);
        if self.cracks.len() >= CRACKS_TO_BREAK {
            self.thawing = true;
            self.events.push(Event::Shattered);
        }
    }

    /// Pressed down: it goes deep.
    pub fn sink(&mut self) -> bool {
        if self.sinking || self.frozen() {
            return false;
        }
        self.disturb();
        self.sinking = true;
        self.events.push(Event::Sank);
        true
    }

    /// Pulled back out: it takes on what the floor was.
    pub fn rise(&mut self) -> Coat {
        self.coat = if self.deep > 0.5 {
            Coat::Ink
        } else if self.still > 0.6 {
            Coat::Quicksilver
        } else {
            Coat::Pearl
        };
        self.coat_left = if self.coat == Coat::Pearl { 0.0 } else { 1.0 };
        self.sinking = false;
        self.coat
    }

    /// How much quicksilver and how much ink is on the object, for the shader.
    pub fn coat_mix(&self) -> (f32, f32) {
        // all of it from the moment it rises, fading only over the last quarter of its time
        let c = smoothstep(0.0, 0.25, self.coat_left);
        match self.coat {
            Coat::Pearl => (0.0, 0.0),
            Coat::Quicksilver => (c, 0.0),
            Coat::Ink => (0.0, c),
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(m: &mut Mere, secs: f32, lake: bool) {
        for _ in 0..(secs * 120.0) as usize {
            m.step(1.0 / 120.0, lake);
        }
    }

    #[test]
    fn left_alone_it_stills_into_a_mirror_and_a_touch_breaks_it() {
        let mut m = Mere::default();
        run(&mut m, STILL_AFTER + STILL_SECS + 0.5, true);
        assert_eq!(m.still, 1.0);
        assert!(m.drain_events().contains(&Event::Stilled));
        m.tap(0.0, 0.0);
        run(&mut m, 0.5, true);
        assert_eq!(m.still, 0.0);
    }

    #[test]
    fn it_does_not_still_while_it_is_still_spreading() {
        let mut m = Mere::default();
        run(&mut m, 12.0, false);
        assert_eq!(m.still, 0.0);
    }

    #[test]
    fn frozen_it_cracks_and_then_breaks_and_thaws() {
        let mut m = Mere::default();
        assert!(m.freeze(1.0, 0.0));
        assert!(m.frozen());
        run(&mut m, 1.0, true);
        assert!(m.ice_reach > 5.0, "it spread from the touch");
        assert!(!m.sink(), "nothing goes down through ice");
        for i in 0..CRACKS_TO_BREAK {
            assert!(m.frozen(), "still holding at crack {i}");
            m.crack(i as f32 * 0.3, 0.0);
        }
        let ev = m.drain_events();
        assert!(ev.contains(&Event::Shattered));
        assert_eq!(ev.iter().filter(|e| **e == Event::Cracked).count(), CRACKS_TO_BREAK);
        assert!(!m.frozen());
        run(&mut m, 1.0, true);
        assert_eq!(m.ice, 0.0);
        assert!(m.cracks.is_empty());
    }

    #[test]
    fn three_taps_in_one_place_throw_up_a_fountain_whose_drop_falls_back() {
        let mut m = Mere::default();
        m.tap(2.0, 1.0);
        run(&mut m, 0.2, true);
        m.tap(2.2, 1.1);
        run(&mut m, 0.2, true);
        m.tap(1.9, 0.9);
        assert!(m.drain_events().contains(&Event::Fountain));
        run(&mut m, 0.3, true);
        let (column, drop) = m.fountain.unwrap().pose();
        assert!(column > 1.0 && drop > 1.0);
        run(&mut m, 2.0, true);
        assert!(m.fountain.is_none());
        assert!(m.drain_events().iter().any(|e| matches!(e, Event::Splash { .. })));
    }

    #[test]
    fn taps_far_apart_are_only_ripples() {
        let mut m = Mere::default();
        for i in 0..5 {
            m.tap(i as f32 * 2.0, 0.0);
            run(&mut m, 0.1, true);
        }
        assert!(m.fountain.is_none());
    }

    #[test]
    fn it_rises_as_what_the_floor_was() {
        let mut m = Mere::default();
        assert_eq!(m.rise(), Coat::Pearl);
        assert_eq!(m.coat_mix(), (0.0, 0.0));

        run(&mut m, 10.0, true);
        assert_eq!(m.rise(), Coat::Quicksilver);
        assert_eq!(m.coat_mix(), (1.0, 0.0));

        m.reset();
        assert!(m.sink());
        run(&mut m, 3.0, true);
        assert_eq!(m.rise(), Coat::Ink);
        // and it wears off, slowly
        run(&mut m, 30.0, false);
        assert!(m.coat_mix().1 > 0.9);
        run(&mut m, COAT_SECS, false);
        assert_eq!(m.coat_mix(), (0.0, 0.0));
    }
}
