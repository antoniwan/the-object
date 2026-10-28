//! Four verbs that each turn the object into something it was not.
//!
//! - pinch it (a short tug, let go at once): a bud stays where you pinched.
//!   Five buds and it opens into a flower. Tap the flower and it folds back up.
//! - stretch it and hold it taut without letting it tear: it stops resisting
//!   and draws out into one thread, longer than the room. Tap the thread to
//!   pluck it; where you pluck decides the harmonic. Hold it still and it winds back.
//! - stir it (drag a tendril round and round it): the liquid turns and turns
//!   until it holds a small universe. Stir the other way and it settles.
//! - draw a square around it: it takes the corners. Knock on the box and
//!   something knocks back; knock three times and the lid opens and everything
//!   inside gets out. When it has all come home, it forgets its corners.
//!
//! Like meiosis.rs, each form's `Phase` only decides which 0..1 values move;
//! the shader blends those, so no form ever jumps into the next.

pub mod bloom;
pub mod cosmos;
pub mod filament;
pub mod pandora;

use crate::math::*;
use bloom::Bloom;
use cosmos::Cosmos;
use filament::Filament;
use pandora::Pandora;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A pinch left a bud.
    Budded,
    /// The buds opened into a flower.
    Bloomed,
    /// The flower closed back into the body.
    Folded,
    /// Held taut, it became a thread.
    Threaded,
    /// The thread was plucked; `harmonic` is which overtone of the string rings.
    Plucked { harmonic: u32 },
    /// The thread wound back into a sphere.
    Wound,
    /// It has been stirred a full turn.
    Stirring,
    /// Stirred long enough, it holds a universe.
    Cosmos,
    /// Stirred back, the universe settled into liquid again.
    Settled,
    /// It took the corners of the square drawn around it.
    Cornered,
    /// A knock on the box.
    Knocked,
    /// The lid opened.
    Lid,
    /// It forgot its corners and is round again.
    Rounded,
}

#[derive(Default)]
pub struct Forms {
    pub bloom: Bloom,
    pub filament: Filament,
    pub cosmos: Cosmos,
    pub pandora: Pandora,
}

impl Forms {
    /// True while it is still the liquid it started as (buds alone don't change what it is).
    pub fn whole(&self) -> bool {
        self.bloom.whole() && self.filament.whole() && self.cosmos.whole() && self.pandora.whole()
    }

    pub fn step(&mut self, dt: f32) {
        self.bloom.step(dt);
        self.filament.step(dt);
        self.cosmos.step(dt);
        self.pandora.step(dt);
    }

    /// The body's distance `d`, reshaped by whichever form it is taking.
    pub fn shape(&self, p: V3, pos: V3, r: f32, d: f32) -> f32 {
        let mut d = d;
        let open = self.bloom.open();
        if open > 0.0 {
            d = d + (self.bloom.sdf(p, pos, r) - d) * smoothstep(0.0, 0.35, open);
        }
        let m = self.filament.morph();
        if m > 0.0 {
            d = d + (self.filament.sdf(p, pos, r) - d) * smoothstep(0.0, 0.15, m);
        }
        let b = self.pandora.boxness();
        if b > 0.0 {
            d = d + (self.pandora.sdf(p, pos, r) - d) * b;
        }
        d
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        let mut out = std::mem::take(&mut self.bloom.events);
        out.append(&mut self.filament.events);
        out.append(&mut self.cosmos.events);
        out.append(&mut self.pandora.events);
        out
    }
}
