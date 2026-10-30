//! Turns raw pointer input into verbs on the world. Everything here is in canvas
//! pixels and has no browser types, so new gestures can be added and tested natively.
//!
//! Gestures today:
//! - tap the liquid: poke (a quick tap the finger slid on a little is still one)
//! - press and drag the liquid: stretch (pull far enough and it tears)
//! - drag a drop: carry it; let go fast to fling it
//! - drag empty space: look around
//! - tap empty space: call it; it reaches toward the place
//! - a drop that touches the body runs back into it, mixing its color in; six drops free
//!   at once and the world turns to night, until they have all come home
//! - two-finger pinch, on the screen or a trackpad, or mouse wheel: squeeze (see pad.rs)
//! - swipe fast clean through it (starting off it): slice; it divides (see meiosis.rs)
//! - on two cells, hold a squeeze: they rejoin
//! - on two or more cells, swipe through again: every cell divides (see morula.rs);
//!   poke one cell three times and it dies; hold a squeeze and it turns to stone;
//!   press and hold the stone and it thaws
//! - press and hold still: warm it (it changes color)
//! - leave it alone: it starts to watch the pointer
//! - stretch it and let go with a fast flick: throw it; thrown hard at the floor it fuses (see fusion.rs)
//! - on the liquid floor: tap to ripple it, drag up to pull the object back out
//!   (see mere.rs for the rest): leave it alone and it stills into a mirror; hold
//!   still on it and it freezes, then tap the ice until it breaks; drag down and it
//!   goes deep; tap three times in one place and it throws up a fountain. Pulled out
//!   of a mirror it comes up quicksilver, out of the deep it comes up ink
//! - poke it twelve times without letting it heal: it bursts (see burst.rs);
//!   then tap a piece to feel its material, or hold still on the ground to gather them
//! - pinch it (a short tug let go of at once): a bud; five buds and it flowers, and a tap folds
//!   the flower back (see forms/bloom.rs); let go a little too late, a bud nearly rises and sinks
//!   back; a quick short tug is a pinch, never a throw; two fingers on it, pinched together and let
//!   go of at once, pinch it too, between them (and the flower)
//! - pinch the flower: it comes apart into dust on the floor (too slowly, its petals only flinch); drag through the dust to sweep it,
//!   sweep it together into a mound, tap the mound, hold it to bring it back up (see forms/dust.rs);
//!   it takes root in the mound, and every tap on it grows it further (see forms/growth.rs)
//! - spin the flower (drag round and round it): it grows dizzy and sways; left past dizzy its petals
//!   wind into ropes that fly out and come undone, right past dizzy it whirls up and comes back down;
//!   nearly dizzy again and again, over several visits, it learns your turn (see forms/spin.rs)
//! - stretch it and hold it taut without tearing: it becomes a thread; tap to pluck, hold to wind it back
//! - stir it (drag a tendril round and round it): it holds a universe; poke it and a star
//!   ignites; stir back to settle it
//! - stir the universe on, the same way: the room folds and it is everywhere; poke to add a
//!   mirror, hold still to run its time backward, stir back to fold the room up (see forms/fold.rs)
//! - draw a square around it: it becomes a box; knock three times (see forms/pandora.rs)
//! - in the landscape: drag a piece to carry it, hold one still to plant it,
//!   carry them all to the middle to mend it (see landscape.rs); crowd them all together
//!   anywhere else and they run as a river: tap it and a column stands up, drag along it
//!   for a row, and fill the view with them (see river.rs); set some close together and they
//!   keep together, as many groups as you like: carry one and its group follows, yank it
//!   and it comes loose, cut fast across the ground between them and they part (see kin.rs)
//!
//! - and from the keyboard: type, hold a letter, breathe with space, tip the room with
//!   the arrows (see keys.rs)
//! - once it asks and is let (see senses.rs): speak, hum, whisper or blow at it; move, move
//!   fast, or cover the camera
//! - and, once it hears you, in words: say a verb and it is done; greet it (see words.rs)
//! - and with a phone in the hand (see motion.rs): tip it to tip the room; shake it
//!
//! - late, once it has reached back (see other.rs), something else is in the room: tap it, carry
//!   it close, throw it, press the two together and squeeze, hurt the first while it is there,
//!   or leave them both alone
//!
//! And all along, it is learning the player back (see contact.rs): the more it has shown,
//! the more it follows, flinches, echoes, takes turns, and, once, replies.
//! It also remembers how it was treated (see remembrance.rs): come back, and the first time
//! a hand comes near it leans toward it, or away. Hurt it enough and it scars; hurt a scar and
//! it opens, and what falls out can close it again (see wound.rs).
//!
//! Stop finding things and let go, and now and then a line of what it is thinking
//! surfaces under it (see thoughts.rs).

use crate::burst;
use crate::camera::{Basis, Camera, FOCAL};
use crate::division::meiosis::{self, Swipe};
use crate::division::morula;
use crate::floor::fusion;
use crate::floor::mere;
use crate::forms::{self, bloom, filament, pandora};
use crate::input::senses::{self, Senses};
use crate::math::*;
use crate::mind::contact::{self, Contact, Sense};
use crate::mind::thoughts::{Mind, Thoughts};
use crate::progress::{Discovery, Progress};
use crate::sim::{Event, Hit, Target, World};

mod gesture;
mod learn;
mod news;
mod sound;
#[cfg(test)]
mod tests;
mod view;

use gesture::Gesture;
pub use sound::Sound;

/// How far (in canvas pixels, scaled by canvas height) a press can move and still count as a tap.
const TAP_SLOP: f32 = 0.012;
/// A press let go of this soon (seconds), having pulled less than a pinch, was a tap the finger slid on.
const SLID_TAP_SECS: f32 = 0.3;
/// Holding a press this long (seconds) without moving turns it into warming.
const HOLD: f32 = 0.55;
/// Nobody touching it for this long (seconds) and it starts watching.
const IDLE_WATCH: f32 = 12.0;

/// A flick faster than this, in canvas heights per second, throws the object.
const THROW_SPEED: f32 = 2.2;
/// Camera distance from which the whole slab can be seen.
const SLAB_DIST: f32 = 9.0;

/// Distance between two points on the canvas.
fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

pub struct Game {
    pub world: World,
    pub cam: Camera,
    pub progress: Progress,
    /// Canvas size in pixels.
    pub w: f32,
    pub h: f32,
    gesture: Gesture,
    pointers: Vec<(i32, f32, f32)>,
    wheel_squeeze: f32,
    orbited: f32,
    swipe: Swipe,
    /// Real time, for timing swipes even while the world is frozen.
    clock: f32,
    /// Real seconds left of the freeze right after a cut.
    hit_stop: f32,
    snapped: bool,
    /// Smoothed pointer velocity, in canvas pixels per second.
    pvel: (f32, f32),
    /// New discoveries for the HUD to announce.
    pub announcements: Vec<Discovery>,
    pub sounds: Vec<Sound>,
    /// Its question after contact, asked and answered, for the page to show and send on.
    pub question: Vec<contact::Event>,
    idle: f32,
    /// The path of a drag on empty space, for noticing a square drawn around it. In canvas
    /// heights, so it holds its shape if the canvas is resized halfway through.
    trace: Vec<(f32, f32)>,
    /// Where the camera was when the drawing started, and whether to ease back there.
    look_from: (f32, f32),
    look_back: bool,
    /// What it has learned about the player, and what it does about it.
    pub contact: Contact,
    /// Seconds since the pointer last did anything at all, pressed or not.
    still: f32,
    /// Where the pointer last pointed, on the plane through the body.
    pub(crate) pointer_at: Option<V3>,
    /// Whether it may hear and see the player, and what it does with it (senses.rs).
    pub senses: Senses,
    /// After the river: the beat it kept. Seconds per beat, seconds into this one, beats left.
    heart: (f32, f32, u32),
    /// What it is thinking, for the page to show now and then.
    pub thoughts: Thoughts,
    /// What the keyboard is doing to it (see keys.rs).
    pub keys: crate::input::keys::Keys,
    /// What a phone in the hand is doing to the room (see motion.rs).
    pub motion: crate::input::motion::Motion,
    /// Two fingers pinched on a trackpad (see pad.rs).
    pub pad: crate::input::pad::Pad,
    /// What it remembers of what has been done to it, and how that leaves it (see remembrance.rs).
    pub remembrance: crate::mind::remembrance::Remembrance,
    /// Words it hears, and how it has been treated (see words.rs).
    pub words: crate::input::words::Words,
}

impl Game {
    pub fn new() -> Game {
        Game {
            world: World::new(),
            cam: Camera::new(),
            progress: Progress::default(),
            w: 1.0,
            h: 1.0,
            gesture: Gesture::Idle,
            pointers: Vec::new(),
            wheel_squeeze: 0.0,
            orbited: 0.0,
            swipe: Swipe::default(),
            clock: 0.0,
            hit_stop: 0.0,
            snapped: false,
            pvel: (0.0, 0.0),
            announcements: Vec::new(),
            sounds: Vec::new(),
            question: Vec::new(),
            idle: 0.0,
            trace: Vec::new(),
            look_from: (0.0, 0.0),
            look_back: false,
            contact: Contact::default(),
            senses: Senses::default(),
            still: 0.0,
            heart: (0.0, 0.0, 0),
            pointer_at: None,
            thoughts: Thoughts::default(),
            keys: Default::default(),
            motion: Default::default(),
            pad: Default::default(),
            remembrance: Default::default(),
            words: Default::default(),
        }
    }

    /// How far along it is in learning the player.
    pub fn stage(&self) -> contact::Stage {
        contact::stage(self.progress.count())
    }

    pub fn discover(&mut self, d: Discovery) {
        if self.progress.discover(d) {
            self.announcements.push(d);
            self.sounds.push(Sound::Discover);
        }
    }

    /// A finger or a button is down somewhere.
    pub fn held(&self) -> bool {
        !self.pointers.is_empty()
    }

    /// The drop in the player's hand, if one is.
    pub fn carried_drop(&self) -> Option<u32> {
        match self.gesture {
            Gesture::Dragging { drop, .. } => Some(drop),
            _ => None,
        }
    }

    pub fn hovering_liquid(&self, x: f32, y: f32) -> bool {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        self.pointers.is_empty() && (self.pick(x, y).is_some() || self.world.other.pick(ro, rd).is_some())
    }

    /// How many hands are on the canvas now.
    pub(crate) fn hands(&self) -> usize {
        self.pointers.len()
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt;
        if let Gesture::Pending { start, hit, since } = self.gesture {
            if self.world.time - since > HOLD {
                self.gesture = Gesture::Warming { start, hit, since: self.world.time };
                // Held still, a thread winds back into a sphere instead of warming; in the fold, time turns round.
                self.world.warming = !self.world.forms.filament.wind()
                    && self.world.forms.filament.whole()
                    && self.world.forms.fold.whole();
                if self.world.warming {
                    self.contact.held();
                }
            }
        }
        // Held taut, just short of tearing, it stops resisting and becomes a thread.
        if let Gesture::Stretching { handle, taut, .. } = &mut self.gesture {
            let handle = *handle;
            let stretched = self
                .world
                .handles
                .iter()
                .find(|h| h.id == handle)
                .map(|h| (h.stretch(&self.world.body), h.tip, h.vel.len()));
            // Held, not moving: a stir also keeps it taut, but never still.
            match stretched {
                Some((s, _, speed)) if s > filament::TAUT && speed < 0.5 && self.world.ordinary() => *taut += dt,
                _ => *taut = 0.0,
            }
            if *taut > filament::TAUT_SECS {
                let (_, tip, _) = stretched.unwrap();
                let fwd = self.cam.basis().fwd;
                let pull = tip - self.world.body.pos;
                if self.world.forms.filament.draw(pull - fwd * pull.dot(fwd)) {
                    self.world.sink_handle(handle);
                    self.gesture = Gesture::Idle;
                    self.sounds.push(Sound::Become);
                }
            }
        }
        if let Gesture::Lake { at, since, moved: false, .. } = self.gesture {
            if self.clock - since > mere::FREEZE_HOLD
                && self.world.fusion.phase == fusion::Phase::Fused
                && self.world.fusion.mere.freeze(at.0, at.1)
            {
                self.discover(Discovery::Freeze);
                self.gesture = Gesture::Idle;
            }
        }
        self.world.forms.fold.hold(matches!(self.gesture, Gesture::Warming { .. }), dt);
        if let Gesture::Warming { since, .. } = self.gesture {
            if self.world.time - since > 1.2 && self.world.warming {
                self.discover(Discovery::Warm);
            }
            // Held long enough, stone remembers it was liquid.
            if self.world.time - since > morula::THAW_HOLD_SECS {
                if let Some(m) = &mut self.world.division.morula {
                    if m.thaw() {
                        self.cam.shake = 0.03;
                    }
                }
            }
        }

        if self.pointers.is_empty() {
            self.idle += dt;
        }
        self.still += dt;
        // The further along it is, the sooner it starts watching you back.
        if self.idle > IDLE_WATCH - 2.0 * f32::from(self.stage() as u8) {
            if !self.world.watching {
                // Until the pointer moves, it reaches toward the camera.
                self.world.gaze_dir = -self.cam.basis().fwd;
            }
            self.world.watching = true;
        }
        if self.world.gaze > 0.7 {
            self.discover(Discovery::Patience);
        }

        let orbiting = matches!(self.gesture, Gesture::Orbiting { .. });
        self.cam.step(dt, orbiting);
        // Hit-stop: the world nearly freezes for a beat right after the cut.
        let dt = if self.hit_stop > 0.0 {
            self.hit_stop -= dt;
            dt * 0.12
        } else {
            dt
        };

        if !matches!(self.gesture, Gesture::Squeezing { .. }) {
            match self.pad.step(dt) {
                Some(s) => {
                    self.wheel_squeeze = s;
                    if s.abs() > 0.3 {
                        self.discover(Discovery::Squeeze);
                    }
                }
                None => self.wheel_squeeze = fade(self.wheel_squeeze, -2.5 * dt),
            }
            self.world.body.squeeze = self.wheel_squeeze;
        }
        crate::input::motion::step(self, dt);
        crate::input::keys::step(self, dt);
        crate::input::words::step(self, dt);

        let sub = 3;
        for _ in 0..sub {
            self.world.step(dt / sub as f32);
        }

        let blend = self.world.division.blend();
        let base = Camera::new().dist;
        let spread = self.world.burst.spread();
        self.cam.dist = base
            + (self.cells_dist() - base) * blend.zoom
            + (burst::LANDSCAPE_DIST - base) * spread
            + 1.8 * self.world.forms.filament.morph()
            + 1.4 * self.world.forms.fold.amount()
            + 3.6 * self.world.forms.dust.view();
        // Step back and level the eye to take in the slab.
        let slab = self.world.division.morula.as_ref().map_or(0.0, |m| {
            let l = m.look();
            l.slab * (1.0 - l.thaw)
        });
        if slab > 0.0 {
            self.cam.dist += (SLAB_DIST - self.cam.dist) * slab;
            if !orbiting {
                self.cam.pitch += (0.06 - self.cam.pitch) * (1.0 - (-1.2 * dt).exp()) * slab;
            }
        }
        self.update_forms(dt, orbiting);
        let b = self.cam.basis();
        let look = crate::other::Look {
            due: self.contact.contacted || self.progress.count() >= crate::other::ARRIVE_FOUND,
            busy: self.contact.acting(),
            visit: self.clock,
            still: self.still,
            right: b.right,
            up: b.up,
            fwd: b.fwd,
            aspect: self.w / self.h.max(1.0),
            view: self.cam.dist / FOCAL,
        };
        crate::other::step(&mut self.world, dt, &look);
        self.other_news();
        // Only the camera's shake can have moved since `b`, and it moves the eye, not where it faces.
        self.update_contact(dt, &b);
        self.update_senses(dt, &b);
        self.update_heart(dt);
        let mind = Mind {
            found: &self.progress,
            world: &self.world,
            contact: &self.contact,
            felt: self.motion.alive && self.senses.is_open(senses::Kind::Feel),
        };
        self.thoughts.step(dt, self.idle, &mind);
        // Lit while a finger holds the mound still; any other gesture lets the light fade.
        let held = match self.gesture {
            Gesture::Dust { since, mound: true, moved: false, .. } => {
                Some((self.clock - since) / forms::dust::RISE_HOLD_SECS)
            }
            _ => None,
        };
        self.world.forms.dust.hold(held.map_or(0.0, |h| h.min(1.0)));
        if let Some(held) = held {
            if held >= 1.0 && self.world.forms.dust.rise() {
                self.sounds.push(Sound::Become);
                self.gesture = Gesture::Idle;
            }
        }
        if let Gesture::Ground { since, .. } = self.gesture {
            if self.clock - since > burst::GATHER_HOLD_SECS && self.world.burst.gather() {
                self.discover(Discovery::Gather);
                self.gesture = Gesture::Idle;
            }
        }
        if let Gesture::Shard { i, since, carried: false, .. } = self.gesture {
            if self.clock - since > burst::landscape::PLANT_HOLD_SECS {
                if self.world.burst.land.toggle_root(i) {
                    self.discover(Discovery::Plant);
                }
                self.world.burst.shards[i].wobble = 0.6;
                self.sounds.push(Sound::Merge);
                self.gesture = Gesture::Idle;
            }
        }
        for e in self.world.burst.drain_events() {
            match e {
                burst::Event::Burst => {
                    self.discover(Discovery::Burst);
                    self.cam.shake = 0.12;
                    self.hit_stop = 0.08;
                    self.gesture = Gesture::Idle;
                }
                burst::Event::Gathered => {
                    self.world.ripple_both(0.9);
                }
                burst::Event::Land(e) => self.on_land(e),
                burst::Event::River(e) => self.on_river(e),
                burst::Event::Kin(e) => self.on_kin(e),
            }
        }
        // Pull back and up so the whole landscape can be seen.
        if spread > 0.0 && !orbiting {
            self.cam.pitch += (0.42 - self.cam.pitch) * (1.0 - (-1.5 * dt).exp()) * spread;
        }
        // The neck snaps: each new cell recoils where they were joined.
        if !self.snapped && blend.split > 0.82 && matches!(self.world.division.phase, meiosis::Phase::Dividing { .. }) {
            self.snapped = true;
            self.world.add_ripple(self.world.division.axis, 0.9);
            self.cam.shake = self.cam.shake.max(0.035);
        }
        for e in self.world.fusion.drain_events() {
            match e {
                fusion::Event::Slammed => {
                    self.discover(Discovery::Slam);
                    self.cam.shake = 0.09;
                    self.hit_stop = 0.1;
                    self.gesture = Gesture::Idle;
                }
                fusion::Event::Bounced { speed } => {
                    self.world.add_ripple(-V3::UP, (speed * 0.12).min(0.9));
                }
                fusion::Event::Rose => {
                    self.world.ripple_both(0.8);
                }
            }
        }
        for e in self.world.fusion.mere.drain_events() {
            match e {
                mere::Event::Stilled => self.discover(Discovery::Mirror),
                mere::Event::Cracked => self.sounds.push(Sound::Poke { pitch: 1.0, size: 0.5 }),
                mere::Event::Shattered => {
                    self.sounds.push(Sound::Tear);
                    self.discover(Discovery::Shatter);
                    self.cam.shake = self.cam.shake.max(0.05);
                    let cracks = self.world.fusion.mere.cracks.clone();
                    for c in cracks {
                        self.world.fusion.touch_lake(c.x, c.z, 1.4);
                    }
                }
                mere::Event::Fountain => self.discover(Discovery::Fountain),
                mere::Event::Splash { x, z } => {
                    self.world.fusion.touch_lake(x, z, 1.2);
                    self.sounds.push(Sound::Poke { pitch: 0.8, size: 0.4 });
                }
                mere::Event::Froze | mere::Event::Sank => {}
            }
        }
        // Once it's in the floor, the camera tilts down to look at what it became.
        let lake = self.world.fusion.blend().lake;
        if lake > 0.0 && !orbiting {
            self.cam.pitch += (0.62 - self.cam.pitch) * (1.0 - (-1.5 * dt).exp()) * lake;
        }
        for e in self.world.division.drain_events() {
            match e {
                meiosis::Event::Rejoined => {
                    self.discover(Discovery::Rejoin);
                    self.world.ripple_both(0.7);
                }
                meiosis::Event::Many(e) => self.morula_event(e),
                meiosis::Event::Divided => {}
            }
        }

        for e in self.world.drain_events() {
            match e {
                Event::Tore { handle, drop } => {
                    self.contact.took(drop);
                    self.sounds.push(Sound::Tear);
                    self.discover(Discovery::Tear);
                    // Keep holding what tore off.
                    if let Gesture::Stretching { handle: h, plane, .. } = self.gesture {
                        if h == handle {
                            self.gesture = Gesture::Dragging { drop, plane, offset: V3::ZERO };
                        }
                    }
                }
                Event::MergedIntoBody { drop } => {
                    self.sounds.push(Sound::Merge);
                    self.discover(Discovery::Merge);
                    if let Gesture::Dragging { drop: d, .. } = self.gesture {
                        if d == drop {
                            self.gesture = Gesture::Idle;
                        }
                    }
                }
                Event::MergedDrops { absorbed, into } => {
                    self.sounds.push(Sound::Merge);
                    self.discover(Discovery::Merge);
                    if let Gesture::Dragging { drop, plane, offset } = self.gesture {
                        if drop == absorbed {
                            self.gesture = Gesture::Dragging { drop: into, plane, offset };
                        }
                    }
                }
                Event::Mixed => self.discover(Discovery::Mix),
                Event::Provoked => {
                    self.sounds.push(Sound::Provoked);
                    self.discover(Discovery::Provoke);
                }
                Event::Opened => self.discover(Discovery::Open),
                Event::Closed => self.discover(Discovery::Close),
            }
        }
        crate::mind::remembrance::step(self, dt);
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
