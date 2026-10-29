//! Turns raw pointer input into verbs on the world. Everything here is in canvas
//! pixels and has no browser types, so new gestures can be added and tested natively.
//!
//! Gestures today:
//! - tap the liquid: poke
//! - press and drag the liquid: stretch (pull far enough and it tears)
//! - drag a drop: carry it; let go fast to fling it
//! - drag empty space: look around
//! - tap empty space: call it; it reaches toward the place
//! - a drop that touches the body runs back into it, mixing its color in; six drops free
//!   at once and the world turns to night, until they have all come home
//! - two-finger pinch or mouse wheel: squeeze
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
//! - pinch it (a short tug let go of at once): a bud; five buds and it flowers (see forms/bloom.rs)
//! - pinch the flower: it comes apart into dust on the floor; drag through the dust to sweep it,
//!   sweep it together into a mound, tap the mound, hold it to bring it back up (see forms/dust.rs)
//! - stretch it and hold it taut without tearing: it becomes a thread; tap to pluck, hold to wind it back
//! - stir it (drag a tendril round and round it): it holds a universe; stir back to settle it
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
//! a hand comes near it leans toward it, or away.
//!
//! Stop finding things and let go, and now and then a line of what it is thinking
//! surfaces under it (see thoughts.rs).

use crate::camera::{Basis, Camera, FOCAL};
use crate::math::*;
use crate::burst;
use crate::contact::{self, Contact, Sense};
use crate::forms::{self, bloom, filament, pandora};
use crate::fusion;
use crate::meiosis::{self, Swipe};
use crate::mere;
use crate::morula;
use crate::progress::{Discovery, Progress};
use crate::senses::{self, Senses};
use crate::sim::{Event, Hit, Target, World};
use crate::thoughts::{Mind, Thoughts};

/// How far (in canvas pixels, scaled by canvas height) a press can move and still count as a tap.
const TAP_SLOP: f32 = 0.012;
/// Holding a press this long (seconds) without moving turns it into warming.
const HOLD: f32 = 0.55;
/// Nobody touching it for this long (seconds) and it starts watching.
const IDLE_WATCH: f32 = 12.0;

/// Things worth hearing. The browser layer turns these into tones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    /// `pitch` is 0..1, higher where it was touched higher up.
    Poke { pitch: f32, size: f32 },
    Tear,
    Merge,
    Discover,
    Provoked,
    Call,
    Pinch,
    Bloom,
    /// The thread rings; `harmonic` is which overtone of its one string.
    Pluck { harmonic: u32 },
    Stir,
    /// A knock on the box, and the knock that answers it.
    Knock,
    Lid,
    /// It became something it wasn't.
    Become,
    /// One of the twelve, struck.
    Shard { i: usize },
    /// All twelve at once.
    Chord,
    /// A column standing up out of the river; each one its own pitch.
    Column { slot: usize },
    /// The river's heartbeat.
    Beat,
    /// The other one, struck; `pitch` 0..1.
    Ring { pitch: f32 },
}

enum Gesture {
    Idle,
    Pending { start: (f32, f32), hit: Hit, since: f32 },
    Warming { start: (f32, f32), hit: Hit, since: f32 },
    /// `since` when it started (real time), `reach` the furthest pull so far in body radii,
    /// `taut` seconds held nearly tearing, `angle` where it was last around the body, for stirring.
    Stretching { handle: u32, plane: V3, since: f32, reach: f32, taut: f32, angle: Option<f32> },
    Dragging { drop: u32, plane: V3, offset: V3 },
    Orbiting { start: (f32, f32), last: (f32, f32) },
    Squeezing { start_dist: f32 },
    /// Touching the liquid floor; `at` is where, on the floor. Held still, it freezes.
    Lake { start: (f32, f32), at: (f32, f32), since: f32, moved: bool },
    /// Pressing the empty ground of the landscape; held still long enough, it gathers.
    Ground { start: (f32, f32), since: f32 },
    /// Swept fast off the empty ground of the landscape: it cuts, and does not look around.
    Cutting { last: (f32, f32) },
    /// Pressing one of the twelve: a tap pokes it, a drag carries it, holding still plants it.
    Shard { i: usize, start: (f32, f32), since: f32, carried: bool },
    /// Touching the river: every column under the pointer stands up.
    River,
    /// Pressing the other one (other.rs): a tap strikes it, a drag carries it.
    Other { start: (f32, f32), plane: V3, offset: V3, carried: bool },
    /// A short pull on the flower, which would not stretch: let go at once, it is a pinch.
    Tug { start: (f32, f32), since: f32 },
    /// On the floor while it is dust (forms/dust.rs): dragging sweeps it from `last`;
    /// pressing the mound and holding still brings it back up.
    Dust { start: (f32, f32), last: (f32, f32), since: f32, mound: bool, moved: bool },
}

/// Distance between two points on the canvas.
fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// A flick faster than this, in canvas heights per second, throws the object.
const THROW_SPEED: f32 = 2.2;
/// Camera distance from which the whole slab can be seen.
const SLAB_DIST: f32 = 9.0;

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
    pub keys: crate::keys::Keys,
    /// What a phone in the hand is doing to the room (see motion.rs).
    pub motion: crate::motion::Motion,
    /// What it remembers of what has been done to it, and how that leaves it (see remembrance.rs).
    pub remembrance: crate::remembrance::Remembrance,
    /// Words it hears, and how it has been treated (see words.rs).
    pub words: crate::words::Words,
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

    fn pick(&self, x: f32, y: f32) -> Option<Hit> {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        self.world.raycast(ro, rd)
    }

    /// Where the pointer is on the camera-facing plane through `through`.
    fn plane_point(&self, x: f32, y: f32, through: V3) -> V3 {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        let n = self.cam.basis().fwd;
        let t = (through - ro).dot(n) / rd.dot(n);
        ro + rd * t
    }

    pub fn hovering_liquid(&self, x: f32, y: f32) -> bool {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        self.pointers.is_empty() && (self.pick(x, y).is_some() || self.world.other.pick(ro, rd).is_some())
    }

    fn end_gesture(&mut self) {
        match std::mem::replace(&mut self.gesture, Gesture::Idle) {
            Gesture::Pending { hit, .. } => {
                if hit.target == Target::Body && self.tap_form(hit) {
                    return;
                }
                self.world.poke(hit, 1.0);
                let pitch = ((hit.pos.y - self.world.body.pos.y) / self.world.body.radius() * 0.5 + 0.5).clamp(0.0, 1.0);
                self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() });
                self.contact.poked((hit.pos - self.world.body.pos).norm(), pitch);
                self.world.other.hurt();
                self.discover(Discovery::Poke);
                if self.world.division.phase == meiosis::Phase::Two {
                    self.discover(Discovery::Twin);
                }
                let r = self.world.body.radius();
                let at = (hit.pos - self.world.body.pos) * (1.0 / r);
                if let Some(m) = &mut self.world.division.morula {
                    m.wound(at);
                }
                if self.world.ordinary() {
                    self.world.burst.wound();
                }
                self.world.forms.cosmos.ignite();
                self.world.forms.fold.facet();
            }
            Gesture::Warming { .. } => self.world.warming = false,
            Gesture::Stretching { handle, since, reach, .. } => {
                let stage = self.stage();
                let dir = self.world.handles.iter().find(|h| h.id == handle).map(|h| h.dir);
                self.world.release_handle(handle);
                let thrown = self.throw();
                // A short tug let go of at once is a pinch.
                let quick = self.clock - since < bloom::PINCH_MAX_SECS && (bloom::PINCH_MIN..bloom::PINCH_MAX).contains(&reach);
                if let (Some(dir), true, false, true) = (dir, quick, thrown, self.world.ordinary()) {
                    if self.world.forms.bloom.pinch(dir) {
                        self.world.add_ripple(dir, 0.35);
                    }
                } else if let (Some(dir), false, false, true) = (dir, quick, thrown, self.world.ordinary()) {
                    self.contact.reached(dir, reach, stage);
                }
            }
            Gesture::Lake { at, moved, .. } => {
                if self.world.fusion.mere.frozen() {
                    self.world.fusion.mere.crack(at.0, at.1);
                } else if !moved {
                    self.world.fusion.touch_lake(at.0, at.1, 1.0);
                    self.world.fusion.mere.tap(at.0, at.1);
                    self.discover(Discovery::Lake);
                }
            }
            Gesture::Dragging { drop, .. } => {
                if self.world.release_drop(drop) > 4.0 {
                    self.discover(Discovery::Fling);
                }
            }
            Gesture::Squeezing { .. } => self.world.body.squeeze = 0.0,
            Gesture::Orbiting { start, last } => {
                // A tap on empty space: it leans toward where you tapped.
                // (A closed shape also ends where it started, so count the whole path.)
                let trace = std::mem::take(&mut self.trace);
                let path: f32 = trace.windows(2).map(|w| dist(w[0], w[1])).sum();
                let moved = (dist(start, last) / self.h).max(path);
                let (cx, cy) = self.screen(self.world.body.pos);
                if moved >= TAP_SLOP && self.world.ordinary() && pandora::is_square(&trace, (cx / self.h, cy / self.h), 1.0) {
                    // The camera went round with the drawing; it drifts back to where it was.
                    self.cam.yaw_vel = 0.0;
                    self.cam.pitch_vel = 0.0;
                    self.look_back = true;
                    self.world.forms.pandora.corner(self.look_from.0 - 0.35);
                } else if moved < TAP_SLOP {
                    let to = self.plane_point(start.0, start.1, self.world.body.pos);
                    self.world.reach(to - self.world.body.pos);
                    self.sounds.push(Sound::Call);
                    self.discover(Discovery::Call);
                    self.contact.called();
                }
            }
            Gesture::Shard { i, carried: false, .. } => {
                let chord = self.world.burst.poke_shard(i);
                self.sounds.push(Sound::Shard { i });
                self.discover(Discovery::Shards);
                if chord {
                    self.sounds.push(Sound::Chord);
                    self.discover(Discovery::Chord);
                }
            }
            Gesture::Shard { i, carried: true, .. } => {
                if self.world.burst.release(i, self.world.body.radius()) {
                    self.sounds.push(Sound::Merge);
                }
            }
            Gesture::Other { carried: false, .. } => crate::other::strike(&mut self.world),
            Gesture::Other { carried: true, .. } => {
                let b = self.cam.basis();
                let (vx, vy) = (self.pvel.0 / self.h, self.pvel.1 / self.h);
                self.world.other.release((b.right * vx - b.up * vy) * 3.0);
            }
            Gesture::Tug { since, .. } => {
                if self.clock - since < bloom::PINCH_MAX_SECS && self.world.forms.pinch_flower(self.world.body.pos) {
                    self.world.add_ripple(V3::UP, 0.5);
                }
            }
            Gesture::Dust { mound: true, moved: false, .. } => {
                if self.world.forms.dust.tap() {
                    self.sounds.push(Sound::Poke { pitch: 0.0, size: 1.6 });
                }
            }
            Gesture::Dust { .. } => {}
            Gesture::Ground { .. } | Gesture::Cutting { .. } | Gesture::River | Gesture::Idle => {}
        }
        self.world.forms.dust.press = 0.0;
    }

    /// A tap on a body that has become something else. Returns true if that form answered it.
    fn tap_form(&mut self, hit: Hit) -> bool {
        let pos = self.world.body.pos;
        let f = &mut self.world.forms;
        f.filament.pluck(hit.pos, pos).is_some() || f.bloom.fold() || f.pandora.knock() || !(f.filament.whole() && f.bloom.whole() && f.pandora.whole())
    }

    /// How many hands are on the canvas now.
    pub(crate) fn hands(&self) -> usize {
        self.pointers.len()
    }

    /// Where a point in the world lands on the canvas, in pixels.
    pub(crate) fn screen_of(&self, p: V3) -> (f32, f32) {
        self.screen(p)
    }

    /// Where a point in the world lands on the canvas, in pixels.
    fn screen(&self, p: V3) -> (f32, f32) {
        let b = self.cam.basis();
        let d = p - b.pos;
        let z = d.dot(b.fwd).max(0.1);
        let (ux, uy) = (d.dot(b.right) * FOCAL / z, d.dot(b.up) * FOCAL / z);
        ((ux * self.h + self.w) * 0.5, (self.h - uy * self.h) * 0.5)
    }

    /// Stand up the column of the river under the pointer. Returns true if the river is there.
    fn raise_column(&mut self, x: f32, y: f32, tapped: bool) -> bool {
        let Some(at) = self.floor_point(x, y) else { return false };
        let river = &mut self.world.burst.river;
        let Some(slot) = river.slot_at(at.x, at.z) else { return false };
        if river.raise(slot, tapped) {
            self.sounds.push(Sound::Column { slot });
        }
        true
    }

    /// Where the pointer meets the floor, if it does.
    fn floor_point(&self, x: f32, y: f32) -> Option<V3> {
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        if rd.y > -0.02 {
            return None;
        }
        Some(ro + rd * ((fusion::FLOOR_Y - ro.y) / rd.y))
    }

    fn pinch_dist(&self) -> f32 {
        let (a, b) = (self.pointers[0], self.pointers[1]);
        dist((a.1, a.2), (b.1, b.2)).max(1.0)
    }

    pub fn pointer_down(&mut self, id: i32, x: f32, y: f32) {
        self.pointers.retain(|p| p.0 != id);
        self.pointers.push((id, x, y));
        match self.pointers.len() {
            1 => {}
            2 => {
                // A second finger turns whatever was happening into a squeeze. What the first
                // finger began is dropped, not finished: no tap, no pinch, no throw, no call.
                match std::mem::replace(&mut self.gesture, Gesture::Idle) {
                    Gesture::Stretching { handle, .. } => self.world.release_handle(handle),
                    Gesture::Dragging { drop, .. } => {
                        self.world.release_drop(drop);
                    }
                    Gesture::Shard { i, carried: true, .. } => {
                        self.world.burst.release(i, self.world.body.radius());
                    }
                    Gesture::Other { .. } => self.world.other.release(V3::ZERO),
                    _ => {}
                }
                self.trace.clear();
                self.world.warming = false;
                self.gesture = Gesture::Squeezing { start_dist: self.pinch_dist() };
                return;
            }
            _ => return,
        }
        self.cam.yaw_vel = 0.0;
        self.cam.pitch_vel = 0.0;
        self.swipe = Swipe::default();
        self.trace = vec![(x / self.h, y / self.h)];
        if !self.look_back {
            self.look_from = (self.cam.yaw, self.cam.pitch);
        }
        self.idle = 0.0;
        self.still = 0.0;
        self.world.watching = false;
        let since = self.world.time;
        self.pvel = (0.0, 0.0);
        if !self.world.burst.whole() {
            self.world.burst.land.touch();
            if self.raise_column(x, y, true) {
                self.gesture = Gesture::River;
                return;
            }
            let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
            self.gesture = match self.world.burst.pick(ro, rd) {
                Some(i) => Gesture::Shard { i, start: (x, y), since: self.clock, carried: false },
                None if self.world.burst.phase == burst::Phase::Landscape => {
                    Gesture::Ground { start: (x, y), since: self.clock }
                }
                None => Gesture::Orbiting { start: (x, y), last: (x, y) },
            };
            return;
        }
        if !self.world.forms.dust.whole() {
            // It is dust on the floor now; the floor is what there is to touch.
            self.gesture = match self.floor_point(x, y) {
                Some(at) => Gesture::Dust {
                    start: (x, y),
                    last: (at.x, at.z),
                    since: self.clock,
                    mound: self.world.forms.dust.on_mound(at.x, at.z),
                    moved: false,
                },
                None => Gesture::Orbiting { start: (x, y), last: (x, y) },
            };
            return;
        }
        if !self.world.fusion.whole() {
            // It is in the floor now; the floor is what there is to touch.
            let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
            self.gesture = match self.world.fusion.lake_hit(ro, rd) {
                Some(at) => {
                    self.world.fusion.mere.disturb();
                    Gesture::Lake { start: (x, y), at, since: self.clock, moved: false }
                }
                None => Gesture::Orbiting { start: (x, y), last: (x, y) },
            };
            return;
        }
        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
        if let Some(t) = self.world.other.pick(ro, rd) {
            if self.pick(x, y).is_none_or(|h| (h.pos - ro).len() > t) {
                // It is carried at the soft one's depth, so it can be brought to it.
                let fwd = self.cam.basis().fwd;
                let plane = self.world.body.pos;
                let off = self.world.other.pos - self.plane_point(x, y, plane);
                let offset = off - fwd * off.dot(fwd);
                self.gesture = Gesture::Other { start: (x, y), plane, offset, carried: false };
                return;
            }
        }
        self.gesture = match self.pick(x, y) {
            Some(hit @ Hit { target: Target::Body, .. }) => {
                let stage = self.stage();
                self.contact.touched((hit.pos - self.world.body.pos).norm(), stage, &mut self.world);
                Gesture::Pending { start: (x, y), hit, since }
            }
            Some(Hit { target: Target::Drop(drop), pos, .. }) => {
                self.contact.took(drop);
                let center = self.world.drops.iter().find(|d| d.id == drop).map_or(pos, |d| d.pos);
                self.world.grab_drop(drop, center);
                Gesture::Dragging { drop, plane: pos, offset: center - pos }
            }
            None => Gesture::Orbiting { start: (x, y), last: (x, y) },
        };
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32, dt: f32) {
        self.still = 0.0;
        let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) else {
            self.hover(x, y, dt);
            return;
        };
        if dt > 0.0 {
            let v = ((x - p.1) / dt, (y - p.2) / dt);
            self.pvel = (self.pvel.0 * 0.4 + v.0 * 0.6, self.pvel.1 * 0.4 + v.1 * 0.6);
        }
        p.1 = x;
        p.2 = y;
        match self.gesture {
            Gesture::Pending { start, hit, since } | Gesture::Warming { start, hit, since } => {
                let moved = dist(start, (x, y)) / self.h;
                if moved > TAP_SLOP {
                    self.world.warming = false;
                    // Cells (and flowers, threads, boxes) don't stretch into tendrils; dragging them just looks around.
                    let f = &self.world.forms;
                    let stretchy = self.world.division.whole() && f.bloom.whole() && f.filament.whole() && f.pandora.whole();
                    let flower = f.bloom.phase == bloom::Phase::Flower;
                    let stretch = if stretchy { self.world.begin_stretch(hit.pos) } else { None };
                    self.gesture = match stretch {
                        Some(handle) => Gesture::Stretching { handle, plane: hit.pos, since: self.clock, reach: 0.0, taut: 0.0, angle: None },
                        None if flower => Gesture::Tug { start, since: self.clock - (self.world.time - since) },
                        None => Gesture::Orbiting { start, last: (x, y) },
                    };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Stretching { handle, plane, since, reach, taut, angle } => {
                let to = self.plane_point(x, y, plane);
                self.world.pull_handle(handle, to);
                let r = self.world.body.radius();
                if (to - plane).len() > 0.35 * r {
                    self.discover(Discovery::Stretch);
                }
                // Round and round the body: stirring.
                let b = self.cam.basis();
                let v = to - self.world.body.pos;
                let v = v - b.fwd * v.dot(b.fwd);
                let now = (v.dot(b.up)).atan2(v.dot(b.right));
                let stirrable = self.world.ordinary() || !self.world.forms.cosmos.whole();
                if let (Some(last), true, true) = (angle, v.len() > 0.5 * r, stirrable) {
                    let pi = std::f32::consts::PI;
                    let turn = (now - last + pi).rem_euclid(2.0 * pi) - pi;
                    self.world.forms.stir(turn / (2.0 * pi));
                }
                let reach = reach.max((to - plane).len() / r);
                self.gesture = Gesture::Stretching { handle, plane, since, reach, taut, angle: Some(now) };
            }
            Gesture::Dragging { drop, plane, offset } => {
                let to = self.plane_point(x, y, plane) + offset;
                self.world.grab_drop(drop, to);
            }
            Gesture::Other { start, plane, offset, carried } => {
                if carried || dist(start, (x, y)) / self.h > TAP_SLOP {
                    let to = self.plane_point(x, y, plane) + offset;
                    self.world.other.carry(to);
                    self.gesture = Gesture::Other { start, plane, offset, carried: true };
                }
            }
            Gesture::Orbiting { start, last } => {
                self.track_swipe(last, (x, y));
                if self.trace.len() < 2000 {
                    self.trace.push((x / self.h, y / self.h));
                }
                let (dx, dy) = ((x - last.0) / self.h * 3.0, (y - last.1) / self.h * 3.0);
                self.cam.orbit(dx, dy);
                self.look_back = false;
                if dt > 0.0 {
                    self.cam.yaw_vel = self.cam.yaw_vel * 0.5 + dx / dt * 0.5;
                    self.cam.pitch_vel = self.cam.pitch_vel * 0.5 + dy / dt * 0.5;
                }
                self.orbited += dx.abs() + dy.abs();
                if self.orbited > 1.5 {
                    self.discover(Discovery::LookAround);
                }
                self.gesture = Gesture::Orbiting { start, last: (x, y) };
            }
            Gesture::River => {
                self.raise_column(x, y, false);
            }
            Gesture::Tug { start, .. } => {
                // pulled further, it was never a pinch: look around
                if dist(start, (x, y)) / self.h > bloom::TUG_MAX {
                    self.gesture = Gesture::Orbiting { start, last: start };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Dust { start, last, since, mound, moved } => {
                let moved = moved || dist(start, (x, y)) / self.h > TAP_SLOP;
                let mut to = last;
                if moved {
                    if let Some(at) = self.floor_point(x, y) {
                        to = (at.x, at.z);
                        if self.world.forms.dust.sweep(last, to) {
                            self.discover(Discovery::Sweep);
                        }
                    }
                }
                self.gesture = Gesture::Dust { start, last: to, since, mound, moved };
            }
            Gesture::Ground { start, .. } => {
                let moved = dist(start, (x, y)) / self.h;
                if moved > TAP_SLOP {
                    // off fast where pieces hold on to each other, it cuts between them; otherwise it looks around
                    let fast = dt > 0.0 && moved / dt > crate::kin::CUT_SPEED && !self.world.burst.kin.groups().is_empty();
                    self.gesture = if fast { Gesture::Cutting { last: start } } else { Gesture::Orbiting { start, last: start } };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Cutting { last } => {
                self.cut_kin(last, (x, y));
                self.gesture = Gesture::Cutting { last: (x, y) };
            }
            Gesture::Shard { i, start, since, carried } => {
                let moved = dist(start, (x, y)) / self.h;
                if carried || moved > TAP_SLOP {
                    let to = self.floor_point(x, y);
                    if to.is_some_and(|to| self.world.burst.carry(i, to)) {
                        self.world.burst.land.touch();
                        self.gesture = Gesture::Shard { i, start, since, carried: true };
                        self.discover(Discovery::Carry);
                    } else if !carried {
                        // rooted, it won't move; look around instead
                        self.gesture = Gesture::Orbiting { start, last: start };
                        self.pointer_move(id, x, y, dt);
                    }
                }
            }
            Gesture::Lake { start, at, since, .. } => {
                let moved = dist(start, (x, y)) / self.h > TAP_SLOP;
                self.gesture = Gesture::Lake { start, at, since, moved };
                let up = (start.1 - y) / self.h;
                if up > 0.12 {
                    // Pull up from the floor and it comes back out, as whatever the floor was.
                    if self.world.fusion.mere.frozen() {
                        // ice won't let it: the pull only cracks it
                        self.world.fusion.mere.crack(at.0, at.1);
                        self.gesture = Gesture::Idle;
                    } else if self.world.fusion.rise() {
                        self.discover(Discovery::Rise);
                        match self.world.fusion.mere.coat {
                            mere::Coat::Quicksilver => self.discover(Discovery::Quicksilver),
                            mere::Coat::Ink => self.discover(Discovery::Ink),
                            mere::Coat::Pearl => {}
                        }
                        self.gesture = Gesture::Idle;
                    }
                } else if up < -0.12 && self.world.fusion.phase == fusion::Phase::Fused && self.world.fusion.mere.sink() {
                    // Push down and it goes deeper.
                    self.discover(Discovery::Deep);
                    self.gesture = Gesture::Idle;
                }
            }
            Gesture::Squeezing { start_dist } if self.pointers.len() >= 2 => {
                let ratio = self.pinch_dist() / start_dist;
                let s = ((ratio - 1.0) * 1.6).clamp(-1.0, 1.0);
                self.world.body.squeeze = s;
                if s.abs() > 0.3 {
                    self.discover(Discovery::Squeeze);
                }
            }
            _ => {}
        }
    }

    pub fn pointer_up(&mut self, id: i32) {
        let Some(i) = self.pointers.iter().position(|p| p.0 == id) else { return };
        self.pointers.remove(i);
        // A squeeze is the first two fingers; a third one coming and going changes nothing.
        if i >= 2 && matches!(self.gesture, Gesture::Squeezing { .. }) {
            return;
        }
        // Lifting one finger of a squeeze ends it; the other finger does nothing until lifted.
        self.end_gesture();
    }

    /// Sample the drag segment finely, so a fast flick can't jump over the body.
    fn track_swipe(&mut self, from: (f32, f32), to: (f32, f32)) {
        if !self.world.division.can_slice() || !self.world.fusion.whole() || !self.world.burst.whole() || !self.world.forms.whole() {
            return;
        }
        let len = dist(from, to);
        let steps = (len / (self.h * 0.02)).ceil().clamp(1.0, 40.0) as usize;
        for i in 1..=steps {
            let f = i as f32 / steps as f32;
            let (x, y) = (from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f);
            let over = matches!(self.pick(x, y), Some(Hit { target: Target::Body, .. }));
            if let Some((dx, dy)) = self.swipe.sample(self.clock, x, y, over, self.h * 0.05) {
                self.slice(dx, dy);
                return;
            }
        }
    }

    /// A fast drag over the landscape cuts between pieces that hold on to each other.
    fn cut_kin(&mut self, from: (f32, f32), to: (f32, f32)) {
        let bu = &self.world.burst;
        if bu.phase != burst::Phase::Landscape || bu.river.active {
            return;
        }
        let at: Vec<(f32, f32)> = (0..burst::SHARDS).map(|i| self.screen(bu.shard_pose(i).0 + V3::UP * (bu.shard_size(i) * 0.3))).collect();
        let cuts: Vec<(usize, usize)> = (0..burst::SHARDS)
            .flat_map(|a| (a + 1..burst::SHARDS).map(move |b| (a, b)))
            .filter(|&(a, b)| bu.kin.bonded(a, b) && crate::kin::crosses(from, to, at[a], at[b]))
            .collect();
        if cuts.is_empty() {
            return;
        }
        let homes = bu.shards.map(|s| s.home);
        for (a, b) in cuts {
            self.world.burst.kin.cut(a, b, &homes);
        }
        self.world.burst.land.touch();
        self.sounds.push(Sound::Tear);
        self.discover(Discovery::Part);
    }

    /// Cut across a screen-space swipe; the halves part perpendicular to it.
    fn slice(&mut self, dx: f32, dy: f32) {
        let b = self.cam.basis();
        let along = b.right * dx - b.up * dy;
        let first = self.world.division.whole();
        if self.world.division.slice(along.cross(b.fwd)) {
            self.hit_stop = meiosis::HIT_STOP_SECS;
            self.snapped = false;
            self.cam.shake = 0.05;
            self.cam.yaw_vel = 0.0;
            self.cam.pitch_vel = 0.0;
            self.world.body.squash_vel -= 2.5;
            self.discover(if first { Discovery::Divide } else { Discovery::Cleave });
        }
    }

    /// Let go of a stretch with a fast flick and the whole object flies that way.
    fn throw(&mut self) -> bool {
        let (vx, vy) = (self.pvel.0 / self.h, self.pvel.1 / self.h);
        let speed = (vx * vx + vy * vy).sqrt();
        if speed < THROW_SPEED || !self.world.ordinary() {
            return false;
        }
        let b = self.cam.basis();
        let dir = (b.right * vx - b.up * vy).norm();
        self.world.body.vel += dir * (speed * 4.5).min(16.0);
        self.discover(Discovery::Throw);
        true
    }

    /// What the four other forms ask of the game each frame: their news, the camera, and the box's contents.
    fn update_forms(&mut self, dt: f32, orbiting: bool) {
        use forms::Event as F;
        for e in self.world.forms.drain_events() {
            match e {
                F::Budded => {
                    self.sounds.push(Sound::Pinch);
                    self.discover(Discovery::Pinch);
                }
                F::Bloomed => {
                    self.sounds.push(Sound::Bloom);
                    self.discover(Discovery::Bloom);
                }
                F::Folded => {
                    self.world.add_ripple(V3::UP, 0.7);
                    self.discover(Discovery::Fold);
                }
                F::Threaded => self.discover(Discovery::Thread),
                F::Plucked { harmonic } => {
                    self.sounds.push(Sound::Pluck { harmonic });
                    self.discover(Discovery::Pluck);
                }
                F::Wound => {
                    self.world.add_ripple(self.world.forms.filament.axis, 0.8);
                    self.world.add_ripple(-self.world.forms.filament.axis, 0.8);
                    self.discover(Discovery::Wind);
                }
                F::Stirring => {
                    self.sounds.push(Sound::Stir);
                    self.discover(Discovery::Stir);
                }
                F::Cosmos => {
                    self.sounds.push(Sound::Become);
                    self.discover(Discovery::Cosmos);
                }
                F::Settled => {
                    self.world.add_ripple(V3::UP, 0.6);
                    self.discover(Discovery::Settle);
                }
                F::Cornered => {
                    self.sounds.push(Sound::Become);
                    self.discover(Discovery::Corners);
                }
                F::Knocked => {
                    self.sounds.push(Sound::Knock);
                    self.discover(Discovery::Knock);
                }
                F::Lid => {
                    self.sounds.push(Sound::Lid);
                    self.cam.shake = 0.04;
                    self.discover(Discovery::Lid);
                }
                F::Rounded => {
                    self.world.add_ripple(V3::UP, 0.8);
                    self.world.add_ripple(-V3::UP, 0.8);
                    self.discover(Discovery::Round);
                }
                F::Everywhere => {
                    self.sounds.push(Sound::Become);
                    self.cam.shake = 0.05;
                    self.discover(Discovery::Everywhere);
                }
                F::Facet => {
                    self.sounds.push(Sound::Chord);
                    self.discover(Discovery::Facet);
                }
                F::Backward => {
                    self.sounds.push(Sound::Stir);
                    self.discover(Discovery::Backward);
                }
                F::Crease => {
                    self.world.add_ripple(V3::UP, 0.7);
                    self.discover(Discovery::Crease);
                }
                F::Crumbled => {
                    self.sounds.push(Sound::Become);
                    self.cam.shake = 0.05;
                    self.discover(Discovery::Dust);
                }
                F::Swept => {}
                F::Heaped => self.sounds.push(Sound::Become),
                F::Grounded => {
                    self.cam.shake = 0.03;
                    self.discover(Discovery::Mound);
                }
                F::Unearthed => {
                    self.world.add_ripple(V3::UP, 0.8);
                    self.world.add_ripple(-V3::UP, 0.8);
                    self.discover(Discovery::Unearth);
                }
            }
        }
        // Everything it was keeping gets out at once.
        if self.world.forms.pandora.spill() {
            let r = self.world.body.radius();
            let top = self.world.body.pos + V3::UP * (pandora::HALF_SIDE * r);
            for i in 0..7 {
                let a = i as f32 * 2.399 + self.world.time;
                let out = v3(a.cos(), 0.0, a.sin());
                let vel = V3::UP * (3.2 + 0.4 * (i % 3) as f32) + out * 1.6;
                self.world.spill(top + out * (0.2 * r), vel, 0.24 * r, crate::sim::palette(i as f32 / 7.0));
            }
        }
        // After a while the open box wants them back (at night, left alone, they would only circle),
        // and once it has all come home, it can be round again.
        if let pandora::Phase::Open { t } = self.world.forms.pandora.phase {
            let home = self.world.body.pos;
            let pull = 0.25 * ((t - 6.0) / 10.0).clamp(0.0, 1.0);
            for d in self.world.drops.iter_mut().filter(|d| d.target.is_none()) {
                d.vel += (home - d.pos).norm() * (pull * dt);
            }
            if self.world.drops.is_empty() {
                self.world.forms.pandora.round();
            }
        }
        if self.look_back && !orbiting {
            let k = 1.0 - (-2.5 * dt).exp();
            self.cam.yaw += (self.look_from.0 - self.cam.yaw) * k;
            self.cam.pitch += (self.look_from.1 - self.cam.pitch) * k;
        }
        // The camera rises to look into the flower, and stays up to look down on the dust.
        let open = self.world.forms.bloom.open().max(self.world.forms.dust.view());
        if open > 0.0 && !orbiting {
            self.cam.pitch += (0.78 - self.cam.pitch) * (1.0 - (-1.5 * dt).exp()) * open;
        }
    }

    fn on_land(&mut self, e: crate::landscape::Event) {
        use crate::landscape::Event as L;
        match e {
            L::Chord | L::Planted => {}
            L::Grown => self.discover(Discovery::Grow),
            L::Weathered => self.discover(Discovery::Weather),
            L::Eroded => self.gesture = Gesture::Idle,
            L::Mended { last } => {
                // it comes back in the color of the last piece home
                let tint = crate::landscape::TINTS[last];
                self.world.body.color = crate::sim::PEARL.lerp(tint, 0.65);
                self.sounds.push(Sound::Chord);
                self.discover(Discovery::Mend);
                self.gesture = Gesture::Idle;
            }
        }
    }

    fn on_kin(&mut self, e: crate::kin::Event) {
        use crate::kin::Event as K;
        match e {
            K::Joined => {
                self.sounds.push(Sound::Merge);
                self.discover(Discovery::Kin);
            }
            K::Loosed => {
                self.sounds.push(Sound::Tear);
                self.discover(Discovery::Loose);
            }
            K::Passed => self.discover(Discovery::Pass),
            K::Felt { i } => self.sounds.push(Sound::Shard { i }),
            K::Many => self.discover(Discovery::Many),
        }
    }

    fn on_river(&mut self, e: crate::river::Event) {
        use crate::river::Event as R;
        match e {
            R::Bled => {
                self.sounds.push(Sound::Become);
                self.discover(Discovery::River);
                self.gesture = Gesture::Idle;
            }
            R::Raised => self.discover(Discovery::Column),
            R::Filled => {
                self.sounds.push(Sound::Beat);
                self.cam.shake = self.cam.shake.max(0.03);
                self.discover(Discovery::Pulse);
            }
            R::Beat => {
                self.sounds.push(Sound::Beat);
                self.cam.shake = self.cam.shake.max(0.03);
            }
            R::Pulsed => {
                // it comes up out of the river dark red, and keeps the beat a while
                self.world.body.color = crate::sim::PEARL.lerp(crate::river::BLOOD, 0.8);
                self.heart = (self.world.burst.river.rhythm(), 0.0, 40);
                self.sounds.push(Sound::Beat);
                self.gesture = Gesture::Idle;
            }
        }
    }

    /// The beat it kept from the river: a small pulse through the body, fainter each time.
    fn update_heart(&mut self, dt: f32) {
        let (period, t, left) = &mut self.heart;
        if *left == 0 || !self.world.burst.whole() {
            return;
        }
        *t += dt;
        if *t >= *period {
            *t -= *period;
            *left -= 1;
            let amp = 0.12 + 0.3 * (*left as f32 / 40.0);
            self.world.add_ripple(V3::UP, amp);
            self.world.add_ripple(-V3::UP, amp * 0.6);
        }
    }

    fn morula_event(&mut self, e: morula::Event) {
        match e {
            morula::Event::Cleaved => self.discover(Discovery::Cleave),
            morula::Event::BecameGrain => self.discover(Discovery::Grain),
            morula::Event::Differentiated => self.discover(Discovery::Eye),
            morula::Event::Died => {
                self.discover(Discovery::Shed);
                self.cam.shake = self.cam.shake.max(0.02);
            }
            morula::Event::Compacting => {
                // The slab will stand facing whoever pressed it.
                if let Some(m) = &mut self.world.division.morula {
                    m.slab_yaw = self.cam.yaw;
                }
                self.discover(Discovery::Stone);
            }
            morula::Event::Slab => {
                self.discover(Discovery::Slab);
                self.cam.shake = 0.06;
                self.hit_stop = 0.1;
            }
            morula::Event::Thawed => {
                self.discover(Discovery::Thaw);
                self.world.add_ripple(V3::UP, 0.9);
                self.world.add_ripple(-V3::UP, 0.9);
            }
        }
    }

    /// Distance at which both cells, side by side, just fit across the screen.
    fn cells_dist(&self) -> f32 {
        let aspect = self.w / self.h.max(1.0);
        let half = (meiosis::SEP + meiosis::CELL_SCALE) * self.world.body.radius();
        (FOCAL * half / (0.82 * aspect)).clamp(2.8, 7.5)
    }

    /// A pointer moving with nothing pressed: what it would be looking at.
    fn hover(&mut self, x: f32, y: f32, dt: f32) {
        let b = self.cam.basis();
        let to = self.plane_point(x, y, self.world.body.pos);
        // Reach toward the viewer, leaning toward the cursor.
        self.world.gaze_dir = ((to - self.world.body.pos) * 0.8 - b.fwd * 1.6).norm();
        self.pointer_at = Some(to);
        let over = match self.pick(x, y) {
            Some(Hit { target: Target::Body, pos, .. }) => Some((pos - self.world.body.pos).norm()),
            _ => None,
        };
        let stage = self.stage();
        self.contact.hover(over, dt.min(0.1), stage, &mut self.world);
    }

    /// It learns the player back, and now and then does something about it.
    fn update_contact(&mut self, dt: f32, b: &Basis) {
        use contact::Event as C;
        let sense = Sense {
            stage: self.stage(),
            found: self.progress.count(),
            idle: self.still,
            fwd: b.fwd,
            right: b.right,
            up: b.up,
            pointer: self.pointer_at,
        };
        self.contact.step(dt, &mut self.world, &sense);
        for e in self.contact.drain_events() {
            match e {
                C::Regarded => self.discover(Discovery::Regard),
                C::Flinched => self.discover(Discovery::Flinch),
                C::Tap { pitch } => self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() }),
                C::Echoed => self.discover(Discovery::Echo),
                C::Mimicked => self.discover(Discovery::Mimic),
                C::Turned(turn) => {
                    self.discover(Discovery::Initiate);
                    match turn {
                        contact::Turn::Offer => self.discover(Discovery::Offer),
                        contact::Turn::Beckon => self.sounds.push(Sound::Call),
                        _ => {}
                    }
                }
                C::Answered => {
                    self.sounds.push(Sound::Merge);
                    self.world.add_ripple(V3::UP, 0.6);
                    self.world.add_ripple(-V3::UP, 0.6);
                    self.discover(Discovery::Answer);
                }
                C::Reaching => self.sounds.push(Sound::Become),
                C::Said => self.discover(Discovery::Contact),
                C::Asked => {
                    self.sounds.push(Sound::Call);
                    self.question.push(e);
                }
                C::Kept(n) => {
                    let pitch = n as f32 / contact::KEEP_STEPS as f32;
                    self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() * 0.3 });
                }
                C::Answer(_) => {
                    self.sounds.push(Sound::Merge);
                    self.world.add_ripple(-b.fwd, 0.5);
                    self.question.push(e);
                }
            }
        }
    }

    /// Hearing and sight, once the player has let it have them.
    fn update_senses(&mut self, dt: f32, b: &Basis) {
        let look = senses::Look { stage: self.stage(), still: self.still, busy: self.contact.acting(), fwd: b.fwd, right: b.right, up: b.up };
        self.senses.step(dt, &mut self.world, &look);
        for d in self.senses.drain_found() {
            self.discover(d);
        }
    }

    pub fn wheel(&mut self, dy: f32) {
        self.wheel_squeeze = (self.wheel_squeeze - dy * 0.0025).clamp(-1.0, 1.0);
        if self.wheel_squeeze.abs() > 0.3 {
            self.discover(Discovery::Squeeze);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt;
        if let Gesture::Pending { start, hit, since } = self.gesture {
            if self.world.time - since > HOLD {
                self.gesture = Gesture::Warming { start, hit, since: self.world.time };
                // Held still, a thread winds back into a sphere instead of warming; in the fold, time turns round.
                self.world.warming = !self.world.forms.filament.wind() && self.world.forms.filament.whole() && self.world.forms.fold.whole();
                if self.world.warming {
                    self.contact.held();
                }
            }
        }
        // Held taut, just short of tearing, it stops resisting and becomes a thread.
        if let Gesture::Stretching { handle, taut, .. } = &mut self.gesture {
            let handle = *handle;
            let stretched = self.world.handles.iter().find(|h| h.id == handle).map(|h| (h.stretch(&self.world.body), h.tip, h.vel.len()));
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
                    self.world.handles.retain(|h| h.id != handle);
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
        if self.idle > IDLE_WATCH - 2.0 * self.stage() as u8 as f32 {
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
            self.wheel_squeeze = fade(self.wheel_squeeze, -2.5 * dt);
            self.world.body.squeeze = self.wheel_squeeze;
        }
        crate::motion::step(self, dt);
        crate::keys::step(self, dt);
        crate::words::step(self, dt);

        let sub = 3;
        for _ in 0..sub {
            self.world.step(dt / sub as f32);
        }

        let blend = self.world.division.blend();
        let base = Camera::new().dist;
        let spread = self.world.burst.spread();
        self.cam.dist = base + (self.cells_dist() - base) * blend.zoom + (burst::LANDSCAPE_DIST - base) * spread
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
        if let Gesture::Dust { since, mound: true, moved: false, .. } = self.gesture {
            let held = (self.clock - since) / forms::dust::RISE_HOLD_SECS;
            self.world.forms.dust.press = held.min(1.0);
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
            if self.clock - since > crate::landscape::PLANT_HOLD_SECS {
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
                    self.world.add_ripple(V3::UP, 0.9);
                    self.world.add_ripple(-V3::UP, 0.9);
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
                    self.world.add_ripple(V3::UP, 0.8);
                    self.world.add_ripple(-V3::UP, 0.8);
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
                    self.world.add_ripple(V3::UP, 0.7);
                    self.world.add_ripple(-V3::UP, 0.7);
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
        crate::remembrance::step(self, dt);
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g
    }

    #[test]
    fn tap_on_center_pokes() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        g.pointer_up(1);
        assert!(g.progress.has(Discovery::Poke));
        assert_eq!(g.world.ripples.len(), 1);
    }

    #[test]
    fn drag_from_center_far_tears_and_keeps_holding_the_drop() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        for i in 1..=60 {
            g.pointer_move(1, 400.0 + i as f32 * 8.0, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Stretch));
        assert!(g.progress.has(Discovery::Tear));
        assert!(matches!(g.gesture, Gesture::Dragging { .. }));
        assert!(g.world.drops[0].target.is_some());
    }

    #[test]
    fn a_fast_swipe_through_the_body_slices_it_into_two_cells() {
        let mut g = game();
        g.pointer_down(1, 400.0, 20.0);
        for i in 1..=6 {
            g.pointer_move(1, 400.0, 20.0 + i as f32 * 95.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        assert!(g.progress.has(Discovery::Divide));
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.division.phase, meiosis::Phase::Two);
        // the cut was vertical on screen, so the cells sit side by side
        assert!(g.world.division.axis.dot(g.cam.basis().right).abs() > 0.9);
        assert!(g.cam.dist < Camera::new().dist);
        // tapping a cell pokes it, and its twin feels it
        g.pointer_down(2, 577.0, 300.0);
        g.pointer_up(2);
        assert!(g.progress.has(Discovery::Twin));
    }

    fn swipe_down(g: &mut Game, id: i32, x: f32) {
        g.pointer_down(id, x, 20.0);
        for i in 1..=6 {
            g.pointer_move(id, x, 20.0 + i as f32 * 95.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(id);
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn cut_again_and_again_it_multiplies_one_cell_dies_and_squeezed_it_turns_to_stone_until_held() {
        let mut g = game();
        swipe_down(&mut g, 1, 400.0);
        assert_eq!(g.world.division.phase, meiosis::Phase::Two);
        swipe_down(&mut g, 2, 577.0);
        swipe_down(&mut g, 3, 400.0);
        assert!(g.progress.has(Discovery::Cleave));
        let m = g.world.division.morula.as_ref().unwrap();
        assert_eq!(m.cells.len(), 8);
        assert!(g.progress.has(Discovery::Eye));

        for i in 0..3 {
            g.pointer_down(10 + i, 400.0, 300.0);
            g.pointer_up(10 + i);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Shed));
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.division.morula.as_ref().unwrap().cells.len(), 7);

        for _ in 0..120 {
            g.wheel(-200.0);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Stone));
        for _ in 0..(60 * 7) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Slab));
        assert!(g.cam.dist > 8.0);
        assert!(g.pick(400.0, 300.0).is_some(), "the slab can be touched");

        g.pointer_down(20, 400.0, 300.0);
        for _ in 0..(60 * 3) {
            g.update(1.0 / 60.0);
        }
        g.pointer_up(20);
        for _ in 0..(60 * 4) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Thaw));
        assert!(g.world.division.whole());
        assert!(g.world.division.morula.is_none());
    }

    #[test]
    fn a_slow_drag_across_the_body_only_orbits() {
        let mut g = game();
        g.pointer_down(1, 400.0, 20.0);
        for i in 1..=60 {
            g.pointer_move(1, 400.0, 20.0 + i as f32 * 9.5, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        assert!(g.world.division.whole());
    }

    #[test]
    fn far_enough_along_it_takes_a_turn_when_you_go_still() {
        let mut g = game();
        for d in crate::progress::ALL.iter().take(contact::TURN_AT) {
            g.progress.discover(*d);
        }
        for _ in 0..4 {
            g.pointer_down(1, 400.0, 300.0);
            g.pointer_up(1);
            g.update(1.0 / 60.0);
        }
        for _ in 0..(60 * 8) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Initiate));
        assert!(g.world.regard > 0.0, "it has started to regard the pointer");
    }

    #[test]
    fn early_on_it_is_only_matter() {
        let mut g = game();
        for _ in 0..(60 * 20) {
            g.update(1.0 / 60.0);
        }
        assert!(!g.progress.has(Discovery::Initiate));
        assert_eq!(g.world.regard, 0.0);
    }

    #[test]
    fn a_pinch_that_starts_on_empty_space_is_only_a_squeeze() {
        let mut g = game();
        g.pointer_down(1, 40.0, 300.0);
        g.pointer_down(2, 760.0, 300.0);
        g.pointer_move(2, 700.0, 300.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
        g.pointer_up(2);
        g.pointer_up(1);
        assert!(!g.progress.has(Discovery::Call), "the first finger was not a tap");
        assert_eq!(g.world.gaze, 0.0);
    }

    #[test]
    fn a_second_finger_on_a_fresh_stretch_does_not_pinch() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        g.pointer_move(1, 440.0, 300.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
        g.pointer_down(2, 600.0, 300.0);
        assert!(g.world.handles.iter().all(|h| h.target.is_none()), "the tendril was let go");
        g.pointer_up(2);
        g.pointer_up(1);
        for _ in 0..30 {
            g.update(1.0 / 60.0);
        }
        assert!(!g.progress.has(Discovery::Pinch));
    }

    #[test]
    fn a_third_finger_lifting_does_not_end_the_squeeze() {
        let mut g = game();
        g.pointer_down(1, 300.0, 300.0);
        g.pointer_down(2, 500.0, 300.0);
        g.pointer_down(3, 400.0, 500.0);
        g.pointer_up(3);
        g.pointer_move(2, 700.0, 300.0, 1.0 / 60.0);
        assert!(g.world.body.squeeze > 0.3);
    }

    #[test]
    fn holding_still_warms_instead_of_poking() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.warming);
        assert!(g.world.body.color != crate::sim::PEARL);
        g.pointer_up(1);
        assert!(!g.world.warming);
        assert!(g.world.ripples.is_empty());
        assert!(g.progress.has(Discovery::Warm));
    }

    #[test]
    fn left_alone_it_watches() {
        let mut g = game();
        for _ in 0..(60 * 25) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Patience));
    }

    #[test]
    fn poking_fast_provokes_it() {
        let mut g = game();
        for _ in 0..7 {
            g.pointer_down(1, 400.0, 300.0);
            g.pointer_up(1);
            g.update(0.1);
        }
        assert!(g.progress.has(Discovery::Provoke));
        assert!(g.world.agitation > 0.5);
    }

    #[test]
    fn tearing_many_pieces_opens_it_and_they_all_come_home() {
        let mut g = game();
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            let (dx, dy) = (a.cos(), a.sin());
            g.pointer_down(1, 400.0 + dx * 60.0, 300.0 + dy * 60.0);
            for k in 1..=60 {
                let s = 60.0 + k as f32 * 8.0;
                g.pointer_move(1, 400.0 + dx * s, 300.0 + dy * s, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
            g.pointer_up(1);
            for _ in 0..20 {
                g.update(1.0 / 60.0);
            }
        }
        assert!(g.progress.has(Discovery::Open), "{} drops", g.world.drops.len());
        for _ in 0..(60 * 120) {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.drops.is_empty(), "{:?} drops left", g.world.drops.iter().map(|d| (d.pos.len(), d.vel.len(), d.target.is_some())).collect::<Vec<_>>());
        assert!(g.progress.has(Discovery::Close));
    }

    #[test]
    fn tap_on_empty_space_makes_it_lean_toward_the_tap() {
        let mut g = game();
        g.pointer_down(1, 40.0, 300.0);
        g.pointer_up(1);
        assert!(g.progress.has(Discovery::Call));
        assert!(g.world.gaze > 0.5 && g.world.gaze_dir.x < -0.5);
    }

    #[test]
    fn a_hard_downward_throw_slams_it_into_the_floor_and_pulling_up_brings_it_back() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        for i in 1..=4 {
            g.pointer_move(1, 400.0, 300.0 + i as f32 * 40.0, 1.0 / 120.0);
            g.update(1.0 / 120.0);
        }
        g.pointer_up(1);
        assert!(g.progress.has(Discovery::Throw));
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Slam));
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
        assert!(g.world.body.pos.y < fusion::FLOOR_Y);

        // tap the liquid floor, then pull up from it
        g.pointer_down(2, 400.0, 560.0);
        g.pointer_up(2);
        assert!(g.progress.has(Discovery::Lake));
        g.pointer_down(3, 400.0, 560.0);
        g.pointer_move(3, 400.0, 400.0, 1.0 / 60.0);
        g.pointer_up(3);
        assert!(g.progress.has(Discovery::Rise));
        for _ in 0..360 {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.fusion.whole());
        assert!(g.world.body.pos.len() < 0.2, "back home in the middle");
    }

    fn slammed() -> Game {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        for i in 1..=4 {
            g.pointer_move(1, 400.0, 300.0 + i as f32 * 40.0, 1.0 / 120.0);
            g.update(1.0 / 120.0);
        }
        g.pointer_up(1);
        for _ in 0..360 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
        g
    }

    #[test]
    fn held_still_the_lake_freezes_and_breaks_under_taps_and_pushed_down_it_comes_up_ink() {
        let mut g = slammed();
        g.pointer_down(2, 400.0, 560.0);
        for _ in 0..70 {
            g.update(1.0 / 60.0);
        }
        g.pointer_up(2);
        assert!(g.progress.has(Discovery::Freeze));
        assert!(g.world.fusion.mere.frozen());
        // it can't be pulled up through the ice
        g.pointer_down(3, 400.0, 560.0);
        g.pointer_move(3, 400.0, 400.0, 1.0 / 60.0);
        g.pointer_up(3);
        assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
        for i in 0..mere::CRACKS_TO_BREAK {
            g.pointer_down(10 + i as i32, 380.0 + i as f32 * 10.0, 560.0);
            g.pointer_up(10 + i as i32);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Shatter));
        for _ in 0..90 {
            g.update(1.0 / 60.0);
        }
        assert!(!g.world.fusion.mere.frozen());

        // push down: it goes deep, and comes back out ink
        g.pointer_down(4, 400.0, 500.0);
        g.pointer_move(4, 400.0, 590.0, 1.0 / 60.0);
        g.pointer_up(4);
        assert!(g.progress.has(Discovery::Deep));
        for _ in 0..180 {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.body.pos.y < fusion::FLOOR_Y - 2.0, "it went further down");
        g.pointer_down(5, 400.0, 560.0);
        g.pointer_move(5, 400.0, 400.0, 1.0 / 60.0);
        g.pointer_up(5);
        assert!(g.progress.has(Discovery::Ink));
        for _ in 0..420 {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.fusion.whole());
        assert!(g.world.body.pos.len() < 0.2);
        assert!(g.world.fusion.mere.coat_mix().1 > 0.9);
    }

    #[test]
    fn left_alone_the_lake_becomes_a_mirror_and_it_comes_up_quicksilver() {
        let mut g = slammed();
        for _ in 0..(60 * 10) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Mirror));
        g.pointer_down(2, 400.0, 560.0);
        g.pointer_move(2, 400.0, 400.0, 1.0 / 60.0);
        g.pointer_up(2);
        assert!(g.progress.has(Discovery::Quicksilver));
    }

    #[test]
    fn three_taps_in_one_place_on_the_lake_throw_up_a_fountain() {
        let mut g = slammed();
        for i in 0..3 {
            g.pointer_down(2 + i, 400.0, 560.0);
            g.pointer_up(2 + i);
            for _ in 0..10 {
                g.update(1.0 / 60.0);
            }
        }
        assert!(g.progress.has(Discovery::Fountain));
        assert!(g.world.fusion.mere.fountain.is_some());
    }

    #[test]
    fn poked_to_death_it_bursts_into_a_landscape_and_holding_still_gathers_it() {
        let mut g = game();
        for i in 0..12 {
            g.pointer_down(i, 400.0, 300.0);
            g.pointer_up(i);
            for _ in 0..12 {
                g.update(1.0 / 60.0);
            }
        }
        assert!(g.progress.has(Discovery::Burst), "wounds {}", g.world.burst.wounds);
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.burst.phase, burst::Phase::Landscape);
        assert!(g.cam.dist > 15.0);
        assert!(g.pick(400.0, 300.0).is_none(), "the body is gone");
        // tap the sky (nothing there), hold still on the ground, and the pieces come home
        g.pointer_down(20, 400.0, 590.0);
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        g.pointer_up(20);
        assert!(g.progress.has(Discovery::Gather));
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert!(g.world.burst.whole());
    }

    fn wait(g: &mut Game, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn five_pinches_make_a_flower_and_a_tap_folds_it() {
        let mut g = game();
        for i in 0..5 {
            let a = i as f32 * std::f32::consts::TAU / 5.0;
            let (dx, dy) = (a.cos(), a.sin());
            let (x0, y0) = (400.0 + dx * 50.0, 300.0 + dy * 50.0);
            g.pointer_down(1, x0, y0);
            for k in 1..=3 {
                g.pointer_move(1, x0 + dx * 14.0 * k as f32, y0 + dy * 14.0 * k as f32, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
            g.pointer_up(1);
            assert!(g.world.drops.is_empty() && !g.progress.has(Discovery::Throw));
            wait(&mut g, 0.6);
        }
        assert!(g.progress.has(Discovery::Pinch));
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Bloom), "{} buds", g.world.forms.bloom.buds.len());
        assert!(g.cam.pitch > 0.5, "it looks down into the flower");
        g.pointer_down(2, 400.0, 300.0);
        g.pointer_up(2);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Fold));
        assert!(g.world.ordinary());
    }

    fn flower(g: &mut Game) {
        for i in 0..5 {
            let a = i as f32 * std::f32::consts::TAU / 5.0;
            let (dx, dy) = (a.cos(), a.sin());
            let (x0, y0) = (400.0 + dx * 50.0, 300.0 + dy * 50.0);
            g.pointer_down(1, x0, y0);
            for k in 1..=3 {
                g.pointer_move(1, x0 + dx * 14.0 * k as f32, y0 + dy * 14.0 * k as f32, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
            g.pointer_up(1);
            wait(g, 0.6);
        }
        wait(g, 4.0);
        assert!(g.progress.has(Discovery::Bloom));
    }

    #[test]
    fn a_pinched_flower_goes_to_dust_that_sweeps_into_a_mound_and_held_it_rises() {
        let mut g = game();
        flower(&mut g);
        // pulled a long way, the flower only looks around
        g.pointer_down(1, 400.0, 300.0);
        g.pointer_move(1, 400.0, 420.0, 1.0 / 60.0);
        g.pointer_up(1);
        wait(&mut g, 0.5);
        assert!(g.world.forms.dust.whole());
        // a short pull let go of at once: it comes apart
        g.pointer_down(1, 400.0, 300.0);
        g.update(1.0 / 60.0);
        g.pointer_move(1, 400.0, 325.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
        g.pointer_up(1);
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Dust));
        assert!(g.world.forms.bloom.whole() && !g.world.ordinary());
        assert!(g.world.raycast(g.cam.basis().pos, (g.world.body.pos - g.cam.basis().pos).norm()).is_none(), "nothing left to touch");
        // sweep each drift into the fullest one, with strokes across the floor
        let floor = |x: f32, z: f32| v3(x, crate::fusion::FLOOR_Y, z);
        for _ in 0..12 {
            if g.world.forms.dust.phase != forms::dust::Phase::Dust {
                break;
            }
            let k = g.world.forms.dust.biggest();
            let to = g.world.forms.dust.clumps[k];
            for i in 0..forms::dust::CLUMPS {
                let c = g.world.forms.dust.clumps[i];
                if i == k || c.amount <= 0.0 {
                    continue;
                }
                let (ox, oz) = (c.x - to.x, c.z - to.z);
                let from = project(&g, floor(c.x + ox * 0.15, c.z + oz * 0.15));
                let end = project(&g, floor(to.x, to.z));
                g.pointer_down(3, from.0, from.1);
                for s in 1..=8 {
                    let f = s as f32 / 8.0;
                    g.pointer_move(3, from.0 + (end.0 - from.0) * f, from.1 + (end.1 - from.1) * f, 1.0 / 60.0);
                }
                g.pointer_up(3);
            }
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Sweep));
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Mound), "{:?}", g.world.forms.dust.clumps);
        let (hx, hz, _, h) = g.world.forms.dust.heap();
        let on = project(&g, floor(hx, hz) + V3::UP * (h * 0.5));
        g.pointer_down(4, on.0, on.1);
        g.pointer_up(4);
        assert!(g.world.forms.dust.thud > 0.5, "it answers a tap");
        g.pointer_down(5, on.0, on.1);
        wait(&mut g, 1.5);
        g.pointer_up(5);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Unearth));
        assert!(g.world.ordinary());
        assert!(g.world.body.pos.len() < 0.3, "it floats where it did");
    }

    #[test]
    fn held_taut_it_becomes_a_thread_that_plucks_and_winds_back() {
        let mut g = game();
        g.pointer_down(1, 400.0, 300.0);
        for k in 1..=20 {
            g.pointer_move(1, 400.0 + 7.5 * k as f32, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        wait(&mut g, 1.5);
        assert!(g.world.drops.is_empty(), "it did not tear");
        assert!(!g.world.forms.filament.whole(), "stretch {:?}", g.world.handles.iter().map(|h| h.stretch(&g.world.body)).collect::<Vec<_>>());
        g.pointer_up(1);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Thread));
        // the thread runs across the screen, far past where the sphere was
        let ax = g.world.forms.filament.axis;
        assert!(ax.dot(g.cam.basis().fwd).abs() < 0.05);
        g.pointer_down(2, 400.0, 300.0);
        g.pointer_up(2);
        wait(&mut g, 0.1);
        assert!(g.progress.has(Discovery::Pluck));
        g.pointer_down(3, 400.0, 300.0);
        wait(&mut g, 1.0);
        g.pointer_up(3);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Wind));
        assert!(g.world.ordinary());
    }

    #[test]
    fn stirring_round_and_round_makes_a_universe() {
        let mut g = game();
        g.pointer_down(1, 400.0 + 60.0, 300.0);
        for k in 1..=(60 * 3) {
            let a = k as f32 / 60.0 * std::f32::consts::TAU;
            g.pointer_move(1, 400.0 + 95.0 * a.cos(), 300.0 - 95.0 * a.sin(), 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        assert!(g.progress.has(Discovery::Stir));
        assert!(g.world.drops.is_empty(), "stirring did not tear it");
        assert!(g.world.forms.filament.whole(), "stirring is not holding it taut");
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Cosmos));
        assert_eq!(g.world.forms.cosmos.sense, 1.0, "counterclockwise");
    }

    /// Stir round and round, `turns` times, counterclockwise if positive.
    fn stir(g: &mut Game, turns: f32) {
        g.pointer_down(1, 400.0 + 50.0, 300.0);
        for k in 1..=((60.0 * turns.abs()) as usize) {
            let a = k as f32 / 60.0 * std::f32::consts::TAU * turns.signum();
            g.pointer_move(1, 400.0 + 90.0 * a.cos(), 300.0 - 90.0 * a.sin(), 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
    }

    #[test]
    fn a_universe_stirred_on_folds_the_room() {
        let mut g = game();
        stir(&mut g, 3.0);
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Cosmos));
        stir(&mut g, 4.0);
        wait(&mut g, 5.0);
        assert!(g.progress.has(Discovery::Everywhere));
        assert_eq!(g.world.forms.fold.phase, forms::fold::Phase::Folded);
        // a poke: another mirror
        g.pointer_down(2, 400.0, 300.0);
        g.pointer_up(2);
        wait(&mut g, 0.1);
        assert!(g.progress.has(Discovery::Facet));
        // held: its time runs backward
        g.pointer_down(3, 400.0, 300.0);
        wait(&mut g, 2.5);
        g.pointer_up(3);
        assert!(g.progress.has(Discovery::Backward));
        // the other way folds the room back up around the universe
        stir(&mut g, -3.0);
        wait(&mut g, 4.0);
        assert!(g.progress.has(Discovery::Crease));
        assert!(g.world.forms.fold.whole());
        assert_eq!(g.world.forms.cosmos.phase, forms::cosmos::Phase::Cosmos);
        // and once more the other way, it is liquid again
        stir(&mut g, -3.0);
        wait(&mut g, 4.0);
        assert!(g.world.ordinary());
    }

    #[test]
    fn a_square_drawn_around_it_makes_a_box_that_opens_on_the_third_knock() {
        let mut g = game();
        let corners = [(150.0, 80.0), (650.0, 80.0), (650.0, 520.0), (150.0, 520.0), (150.0, 80.0)];
        g.pointer_down(1, corners[0].0, corners[0].1);
        for w in corners.windows(2) {
            for k in 1..=25 {
                let f = k as f32 / 25.0;
                g.pointer_move(1, w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
        }
        g.pointer_up(1);
        wait(&mut g, 0.1);
        assert!(g.progress.has(Discovery::Corners));
        assert!(g.world.division.whole(), "drawing around it does not cut it");
        wait(&mut g, 2.5);
        for i in 0..3 {
            g.pointer_down(10 + i, 400.0, 300.0);
            g.pointer_up(10 + i);
            wait(&mut g, 0.3);
        }
        assert!(g.progress.has(Discovery::Knock) && g.progress.has(Discovery::Lid));
        assert!(g.world.burst.wounds < 1.0, "knocking is not wounding");
        wait(&mut g, 3.0);
        assert!(g.world.drops.len() >= crate::sim::OPEN_AT, "{} drops", g.world.drops.len());
        assert!(g.progress.has(Discovery::Open));
        wait(&mut g, 150.0);
        assert!(g.progress.has(Discovery::Close), "{:?}", g.world.drops.iter().map(|d| (d.pos, d.vel.len())).collect::<Vec<_>>());
        assert!(g.progress.has(Discovery::Round));
        assert!(g.world.ordinary());
    }

    /// Where a world point lands on the 800x600 canvas.
    fn project(g: &Game, p: V3) -> (f32, f32) {
        let b = g.cam.basis();
        let d = p - b.pos;
        let z = d.dot(b.fwd);
        (400.0 + d.dot(b.right) * FOCAL / z * 300.0, 300.0 - d.dot(b.up) * FOCAL / z * 300.0)
    }

    fn burst_it(g: &mut Game) {
        for i in 0..12 {
            g.pointer_down(i, 400.0, 300.0);
            g.pointer_up(i);
            for _ in 0..12 {
                g.update(1.0 / 60.0);
            }
        }
        for _ in 0..240 {
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.burst.phase, burst::Phase::Landscape);
    }

    /// The screen point of piece `i`, partway up so the press lands on it.
    fn shard_at(g: &Game, i: usize) -> (f32, f32) {
        let (pos, _) = g.world.burst.shard_pose(i);
        project(g, pos + V3::UP * (g.world.burst.shard_size(i) * 0.3))
    }

    #[test]
    fn in_the_landscape_pieces_sing_root_and_can_be_carried_home() {
        let mut g = game();
        burst_it(&mut g);
        // three quick taps on different pieces: a chord
        for (n, i) in [0usize, 1, 2].into_iter().enumerate() {
            let (x, y) = shard_at(&g, i);
            g.pointer_down(30 + n as i32, x, y);
            g.pointer_up(30 + n as i32);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Chord));
        assert!(g.sounds.contains(&Sound::Chord));

        // hold one still and it roots
        let (x, y) = shard_at(&g, 5);
        g.pointer_down(40, x, y);
        for _ in 0..70 {
            g.update(1.0 / 60.0);
        }
        g.pointer_up(40);
        assert!(g.progress.has(Discovery::Plant));
        assert!(g.world.burst.land.roots[5].rooted);
        // and holding it again lets it go
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        let (x, y) = shard_at(&g, 5);
        g.pointer_down(41, x, y);
        for _ in 0..70 {
            g.update(1.0 / 60.0);
        }
        g.pointer_up(41);
        assert!(!g.world.burst.land.roots[5].rooted);
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }

        // carry every piece to the middle and it is whole again, in the last one's color
        let middle = v3(0.0, fusion::FLOOR_Y, 0.6);
        let mut id = 50;
        for _ in 0..60 {
            let Some(i) = (0..burst::SHARDS).find(|&i| !g.world.burst.land.returned(i)) else { break };
            let (x0, y0) = shard_at(&g, i);
            let (x1, y1) = project(&g, middle);
            id += 1;
            g.pointer_down(id, x0, y0);
            for k in 1..=30 {
                let f = k as f32 / 30.0;
                g.pointer_move(id, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
            g.pointer_up(id);
            for _ in 0..30 {
                g.update(1.0 / 60.0);
            }
        }
        assert!(g.progress.has(Discovery::Carry));
        for _ in 0..300 {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Mend), "{:?}", (0..12).map(|i| g.world.burst.land.returned(i)).collect::<Vec<_>>());
        assert!(g.world.burst.whole());
        assert!(g.world.body.color != crate::sim::PEARL);
    }

    /// Drag piece `i` slowly over the ground to `to`, and let go.
    fn carry_to(g: &mut Game, id: i32, i: usize, to: V3) {
        let (x0, y0) = shard_at(g, i);
        let (x1, y1) = project(g, to);
        g.pointer_down(id, x0, y0);
        for k in 1..=60 {
            let f = k as f32 / 60.0;
            g.pointer_move(id, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(id);
        for _ in 0..30 {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn set_down_together_they_keep_together_in_as_many_groups_as_you_like_and_a_cut_parts_them() {
        let mut g = game();
        burst_it(&mut g);
        // three pairs, each in its own place
        for (n, (a, b)) in [(0usize, 1usize), (2, 3), (4, 5)].into_iter().enumerate() {
            let turn = n as f32 * 2.1 + 0.4;
            let at = v3(turn.cos() * 22.0, fusion::FLOOR_Y, turn.sin() * 22.0);
            let side = v3(-at.z, 0.0, at.x).norm();
            carry_to(&mut g, 70 + 2 * n as i32, a, at);
            carry_to(&mut g, 71 + 2 * n as i32, b, at + side * 3.6);
            assert_eq!(g.world.burst.kin.group(a), (1 << a) | (1 << b), "{a} and {b} hold on, only to each other: {:?} {:?} {:?}", g.world.burst.shards[a].home, g.world.burst.shards[b].home, g.cam.basis().pos);
        }
        assert!(g.progress.has(Discovery::Kin));
        assert!(g.progress.has(Discovery::Many));
        assert!(!g.world.burst.river.active, "small groups do not run");

        // touch one and the other feels it after
        let (x, y) = shard_at(&g, 0);
        g.pointer_down(80, x, y);
        g.pointer_up(80);
        g.sounds.clear();
        for _ in 0..30 {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Pass));
        assert!(g.sounds.iter().any(|s| matches!(s, Sound::Shard { i: 0 | 1 })), "the other one of the pair answers after");

        // a fast cut across the ground between them, and they part
        let (pa, pb) = (shard_at(&g, 4), shard_at(&g, 5));
        let mid = ((pa.0 + pb.0) * 0.5, (pa.1 + pb.1) * 0.5);
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let l = (dx * dx + dy * dy).sqrt();
        let across = (-dy / l, dx / l);
        let from = (mid.0 - across.0 * 80.0, mid.1 - across.1 * 80.0);
        g.pointer_down(81, from.0, from.1);
        assert!(matches!(g.gesture, Gesture::Ground { .. }), "the cut starts on the ground");
        for k in 1..=4 {
            let f = k as f32 * 40.0;
            g.pointer_move(81, from.0 + across.0 * f, from.1 + across.1 * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(81);
        assert!(g.progress.has(Discovery::Part));
        assert!(!g.world.burst.kin.bonded(4, 5));
        assert!(g.world.burst.kin.bonded(0, 1) && g.world.burst.kin.bonded(2, 3), "the other groups were not touched");
        // and each side steps back from the cut
        let gap = (g.world.burst.shards[4].home - g.world.burst.shards[5].home).len();
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        assert!((g.world.burst.shards[4].home - g.world.burst.shards[5].home).len() > gap + 1.0);
    }

    #[test]
    fn a_group_carried_to_the_middle_goes_home_together() {
        let mut g = game();
        burst_it(&mut g);
        let home = g.world.burst.shards[0].home;
        let side = v3(-home.z, 0.0, home.x).norm();
        carry_to(&mut g, 90, 1, home + side * 3.6);
        let group = g.world.burst.kin.group(0);
        assert!(group & 0b10 != 0, "they hold on");
        // slowly, so the others keep up
        let (x0, y0) = shard_at(&g, 0);
        let (x1, y1) = project(&g, v3(0.0, fusion::FLOOR_Y, 0.6));
        g.pointer_down(91, x0, y0);
        for k in 1..=400 {
            let f = k as f32 / 400.0;
            g.pointer_move(91, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        assert_eq!(g.world.burst.kin.group(0), group, "they kept up");
        g.pointer_up(91);
        for i in 0..burst::SHARDS {
            assert_eq!(g.world.burst.land.returned(i), group & (1 << i) != 0, "piece {i}");
        }
    }

    #[test]
    fn crowded_together_they_run_red_and_a_view_full_of_columns_gives_it_a_pulse() {
        let mut g = game();
        burst_it(&mut g);
        // crowd every piece together off to one side
        let c = v3(9.0, fusion::FLOOR_Y, -5.0);
        for i in 0..burst::SHARDS {
            let a = i as f32 * 2.399_963;
            let r = 1.9 * (i as f32).sqrt();
            assert!(g.world.burst.carry(i, c + v3(a.cos() * r, 0.0, a.sin() * r)));
            g.world.burst.release(i, g.world.body.radius());
        }
        wait(&mut g, 6.0);
        assert!(g.progress.has(Discovery::River));
        assert!(!g.progress.has(Discovery::Mend));

        // a tap on the river stands a column up out of it
        let river = &g.world.burst.river;
        let slot = crate::river::LANES * 12 + 1;
        let (x, z) = river.slot_pos(slot);
        let (px, py) = project(&g, v3(x, fusion::FLOOR_Y, z));
        g.pointer_down(60, px, py);
        g.pointer_up(60);
        assert!(g.sounds.contains(&Sound::Column { slot }));
        wait(&mut g, 0.6);
        assert!(g.progress.has(Discovery::Column));
        assert!(g.world.burst.river.height(slot) > 2.0);
        assert!(!g.world.burst.whole());

        // drag along it, lane by lane, until the view is full of them
        for lane in 0..crate::river::LANES {
            let id = 61 + lane as i32;
            let at = |g: &Game, row: usize| {
                let (x, z) = g.world.burst.river.slot_pos(row * crate::river::LANES + lane);
                project(g, v3(x, fusion::FLOOR_Y, z))
            };
            let (x0, y0) = at(&g, 0);
            g.pointer_down(id, x0, y0);
            for row in 1..crate::river::ROWS {
                let (x, y) = at(&g, row);
                g.pointer_move(id, x, y, 1.0 / 60.0);
                g.update(1.0 / 60.0);
            }
            g.pointer_up(id);
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Pulse), "{} standing", g.world.burst.river.standing_count());
        assert!(g.sounds.contains(&Sound::Beat));

        // a few beats, then it comes back up out of the river: red, and still beating
        wait(&mut g, 14.0);
        assert!(g.world.burst.whole());
        let col = g.world.body.color;
        assert!(col.x > 0.4 && col.y < 0.4, "{col:?}");
        assert!(g.heart.2 > 0);
    }

    #[test]
    fn left_alone_the_landscape_weathers_and_rolls_home() {
        let mut g = game();
        burst_it(&mut g);
        for _ in 0..(60 * 60) {
            g.update(1.0 / 60.0);
        }
        assert!(g.progress.has(Discovery::Weather));
        assert!(g.world.burst.whole());
    }

    #[test]
    fn drag_on_empty_space_orbits() {
        let mut g = game();
        let yaw = g.cam.yaw;
        g.pointer_down(1, 20.0, 20.0);
        g.pointer_move(1, 120.0, 20.0, 1.0 / 60.0);
        g.pointer_up(1);
        assert!(g.cam.yaw != yaw);
        assert!(g.world.ripples.is_empty());
    }
}
