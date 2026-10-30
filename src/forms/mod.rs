//! Four verbs that each turn the object into something it was not.
//!
//! - pinch it (a short tug, let go at once): a bud stays where you pinched.
//!   Five buds and it opens into a flower. Tap the flower and it folds back up.
//!   Spin the flower and it grows dizzy; past that, turned left it winds into ropes that fly
//!   out and come undone, turned right it whirls up off the floor and comes back down (spin.rs).
//!   Pinch the flower and it comes apart into dust on the floor; sweep the dust
//!   together and it heaps into a mound; something takes root in it and grows as far as it
//!   is tended (growth.rs); hold the mound and it rises (see dust.rs).
//! - stretch it and hold it taut without letting it tear: it stops resisting
//!   and draws out into one thread, longer than the room. Tap the thread to
//!   pluck it; where you pluck decides the harmonic. Hold it still and it winds back.
//! - stir it (drag a tendril round and round it): the liquid turns and turns
//!   until it holds a small universe. Stir the other way and it settles.
//!   Stir the universe on, the same way, and the room folds: it is everywhere at once,
//!   in mirrors (see fold.rs). Stir the other way and the room folds back up.
//! - draw a square around it: it takes the corners. Knock on the box and
//!   something knocks back; knock three times and the lid opens and everything
//!   inside gets out. When it has all come home, it forgets its corners.
//!
//! After contact, on its own turn, it may begin the flower, the box or the universe by itself and
//! stop partway, waiting for the verb to take it on (lean.rs).
//!
//! Like meiosis.rs, each form's `Phase` only decides which 0..1 values move;
//! the shader blends those, so no form ever jumps into the next.

pub mod bloom;
pub mod cosmos;
pub mod dust;
pub mod filament;
pub mod fold;
pub mod growth;
pub mod lean;
pub mod pandora;
pub mod spin;

use crate::math::*;
use bloom::Bloom;
use cosmos::Cosmos;
use dust::Dust;
use filament::Filament;
use fold::Fold;
use pandora::Pandora;
use spin::Spin;

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
    /// Stirred on past its universe, the room folded and it is everywhere.
    Everywhere,
    /// The folded room took another mirror.
    Facet,
    /// Held, its time turned round.
    Backward,
    /// The room folded back up around one of it.
    Crease,
    /// Pinched as a flower, it came apart into dust.
    Crumbled,
    /// The dust was swept along.
    Swept,
    /// Nearly all of it in one place: it heaps itself up.
    Heaped,
    /// The heap has settled into a mound.
    Grounded,
    /// It came back up out of the mound.
    Unearthed,
    /// Something took root in the mound.
    Rooted,
    /// Tended all the way: in leaf and blossom.
    Canopy,
    /// Left alone, what grew drew back down.
    Withered,
    /// The flower, turned fast, has started to sway.
    Swaying,
    /// Brought near dizzy and let go of.
    Nearly,
    /// Brought near dizzy often enough: it knows which way you turn it, and turns `with` you or back.
    Learned { with: bool },
    /// Turned left past dizzy: its petals wind into ropes.
    Winding,
    /// The ropes flew out.
    Flung,
    /// The ropes lay down over the floor; it starts to come back.
    LaidDown,
    /// The ropes were taken back in, and it is the liquid again.
    Unwound,
    /// Turned right past dizzy: it lays its petals flat and whirls.
    Whirling,
    /// The floor let go of it.
    Aloft,
    /// It came back down, a flower again.
    Landed,
}

#[derive(Default)]
pub struct Forms {
    pub bloom: Bloom,
    pub filament: Filament,
    pub cosmos: Cosmos,
    pub pandora: Pandora,
    pub fold: Fold,
    pub dust: Dust,
    pub spin: Spin,
}

impl Forms {
    /// True while it is still the liquid it started as (buds alone don't change what it is).
    pub fn whole(&self) -> bool {
        self.bloom.whole()
            && self.filament.whole()
            && self.cosmos.whole()
            && self.pandora.whole()
            && self.fold.whole()
            && self.dust.whole()
            && self.spin.whole()
    }

    pub fn step(&mut self, dt: f32) {
        self.bloom.step(dt);
        self.spin.step(dt, self.bloom.phase == bloom::Phase::Flower);
        if self.spin.events.contains(&Event::LaidDown) {
            self.bloom.let_go();
        }
        if self.bloom.whole() {
            self.spin.angle = 0.0;
        }
        self.filament.step(dt);
        self.fold.step(dt);
        // in the fold, the universe keeps the fold's time, backward too
        self.cosmos.pace = if self.fold.whole() { 1.0 } else { self.fold.pace() };
        self.cosmos.step(dt);
        self.pandora.step(dt);
        self.dust.step(dt);
        // once nothing is left of it, the flower it was is gone too
        if self.dust.body_gone() && !self.bloom.whole() {
            self.bloom.scatter();
        }
    }

    /// Whether it is holding a form partway on its own turn (lean.rs).
    pub fn leaning(&self) -> bool {
        self.bloom.lean.leaning() || self.pandora.lean.leaning() || self.cosmos.lean.leaning()
    }

    /// Something else has started: whatever it began by itself sinks back, from where it is.
    pub fn sink_leans(&mut self) {
        self.bloom.lean.sink();
        self.pandora.lean.sink();
        self.cosmos.lean.sink();
    }

    /// A pinch on the flower: it comes apart over the floor under `pos`.
    pub fn pinch_flower(&mut self, pos: V3) -> bool {
        self.bloom.phase == bloom::Phase::Flower
            && self.spin.whole()
            && self.dust.crumble(pos.x, pos.z, self.bloom.angle)
    }

    /// A stir (in turns, counterclockwise on screen positive): the fold takes it while there
    /// is one or while the universe is being stirred on, and the universe takes the rest.
    pub fn stir(&mut self, turns: f32) {
        let universe = self.cosmos.phase == cosmos::Phase::Cosmos;
        if !self.fold.stir(turns, self.cosmos.sense, universe) {
            self.cosmos.stir(turns);
        }
    }

    /// The body's distance `d`, reshaped by whichever form it is taking.
    pub fn shape(&self, p: V3, pos: V3, r: f32, d: f32) -> f32 {
        let mut d = d;
        let open = self.bloom.open();
        if open > 0.0 {
            d = d + (self.bloom.sdf(p, pos + self.spin.offset() * r, r) - d) * smoothstep(0.0, 0.35, open);
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
        out.append(&mut self.fold.events);
        out.append(&mut self.dust.events);
        out.append(&mut self.spin.events);
        out
    }
}
