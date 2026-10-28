//! Turns raw pointer input into verbs on the world. Everything here is in canvas
//! pixels and has no browser types, so new gestures can be added and tested natively.
//!
//! Gestures today:
//! - tap the liquid: poke
//! - press and drag the liquid: stretch (pull far enough and it tears)
//! - drag a drop: carry it; let go fast to fling it
//! - drag empty space: look around
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

use crate::camera::{Camera, FOCAL};
use crate::math::*;
use crate::burst;
use crate::fusion;
use crate::meiosis::{self, Swipe};
use crate::mere;
use crate::morula;
use crate::progress::{Discovery, Progress};
use crate::sim::{Event, Hit, Target, World};

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
}

enum Gesture {
    Idle,
    Pending { start: (f32, f32), hit: Hit, since: f32 },
    Warming { start: (f32, f32), hit: Hit, since: f32 },
    Stretching { handle: u32, plane: V3 },
    Dragging { drop: u32, plane: V3, offset: V3 },
    Orbiting { start: (f32, f32), last: (f32, f32) },
    Squeezing { start_dist: f32 },
    /// Touching the liquid floor; `at` is where, on the floor. Held still, it freezes.
    Lake { start: (f32, f32), at: (f32, f32), since: f32, moved: bool },
    /// Pressing the empty ground of the landscape; held still long enough, it gathers.
    Ground { start: (f32, f32), since: f32 },
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
    idle: f32,
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
            idle: 0.0,
        }
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
        self.pointers.is_empty() && self.pick(x, y).is_some()
    }

    fn end_gesture(&mut self) {
        match std::mem::replace(&mut self.gesture, Gesture::Idle) {
            Gesture::Pending { hit, .. } => {
                self.world.poke(hit, 1.0);
                let pitch = ((hit.pos.y - self.world.body.pos.y) / self.world.body.radius() * 0.5 + 0.5).clamp(0.0, 1.0);
                self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() });
                self.discover(Discovery::Poke);
                if self.world.division.phase == meiosis::Phase::Two {
                    self.discover(Discovery::Twin);
                }
                let r = self.world.body.radius();
                let at = (hit.pos - self.world.body.pos) * (1.0 / r);
                if let Some(m) = &mut self.world.division.morula {
                    m.wound(at);
                }
                if self.world.division.whole() && self.world.fusion.whole() {
                    self.world.burst.wound();
                }
            }
            Gesture::Warming { .. } => self.world.warming = false,
            Gesture::Stretching { handle, .. } => {
                self.world.release_handle(handle);
                self.throw();
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
                let moved = ((last.0 - start.0).powi(2) + (last.1 - start.1).powi(2)).sqrt() / self.h;
                if moved < TAP_SLOP {
                    let to = self.plane_point(start.0, start.1, self.world.body.pos);
                    self.world.reach(to - self.world.body.pos);
                    self.sounds.push(Sound::Call);
                    self.discover(Discovery::Call);
                }
            }
            Gesture::Ground { .. } | Gesture::Idle => {}
        }
    }

    fn pinch_dist(&self) -> f32 {
        let (a, b) = (self.pointers[0], self.pointers[1]);
        ((a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt().max(1.0)
    }

    pub fn pointer_down(&mut self, id: i32, x: f32, y: f32) {
        self.pointers.retain(|p| p.0 != id);
        self.pointers.push((id, x, y));
        match self.pointers.len() {
            1 => {}
            2 => {
                // A second finger turns whatever was happening into a squeeze.
                if let Gesture::Pending { .. } = self.gesture {
                    self.gesture = Gesture::Idle;
                }
                self.world.warming = false;
                self.end_gesture();
                self.gesture = Gesture::Squeezing { start_dist: self.pinch_dist() };
                return;
            }
            _ => return,
        }
        self.cam.yaw_vel = 0.0;
        self.cam.pitch_vel = 0.0;
        self.swipe = Swipe::default();
        self.idle = 0.0;
        self.world.watching = false;
        let since = self.world.time;
        self.pvel = (0.0, 0.0);
        if !self.world.burst.whole() {
            let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
            self.gesture = match self.world.burst.pick(ro, rd) {
                Some(i) => {
                    self.world.burst.poke_shard(i);
                    self.discover(Discovery::Shards);
                    Gesture::Idle
                }
                None if self.world.burst.phase == burst::Phase::Landscape => {
                    Gesture::Ground { start: (x, y), since: self.clock }
                }
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
        self.gesture = match self.pick(x, y) {
            Some(hit @ Hit { target: Target::Body, .. }) => Gesture::Pending { start: (x, y), hit, since },
            Some(Hit { target: Target::Drop(drop), pos, .. }) => {
                let center = self.world.drops.iter().find(|d| d.id == drop).map_or(pos, |d| d.pos);
                self.world.grab_drop(drop, center);
                Gesture::Dragging { drop, plane: pos, offset: center - pos }
            }
            None => Gesture::Orbiting { start: (x, y), last: (x, y) },
        };
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32, dt: f32) {
        let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) else {
            self.hover(x, y);
            return;
        };
        if dt > 0.0 {
            let v = ((x - p.1) / dt, (y - p.2) / dt);
            self.pvel = (self.pvel.0 * 0.4 + v.0 * 0.6, self.pvel.1 * 0.4 + v.1 * 0.6);
        }
        p.1 = x;
        p.2 = y;
        match self.gesture {
            Gesture::Pending { start, hit, .. } | Gesture::Warming { start, hit, .. } => {
                let moved = ((x - start.0).powi(2) + (y - start.1).powi(2)).sqrt() / self.h;
                if moved > TAP_SLOP {
                    self.world.warming = false;
                    // Cells don't stretch into tendrils; dragging them just looks around.
                    let stretch = if self.world.division.whole() { self.world.begin_stretch(hit.pos) } else { None };
                    self.gesture = match stretch {
                        Some(handle) => Gesture::Stretching { handle, plane: hit.pos },
                        None => Gesture::Orbiting { start, last: (x, y) },
                    };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Stretching { handle, plane } => {
                let to = self.plane_point(x, y, plane);
                self.world.pull_handle(handle, to);
                if (to - plane).len() > 0.35 * self.world.body.radius() {
                    self.discover(Discovery::Stretch);
                }
            }
            Gesture::Dragging { drop, plane, offset } => {
                let to = self.plane_point(x, y, plane) + offset;
                self.world.grab_drop(drop, to);
            }
            Gesture::Orbiting { start, last } => {
                self.track_swipe(last, (x, y));
                let (dx, dy) = ((x - last.0) / self.h * 3.0, (y - last.1) / self.h * 3.0);
                self.cam.orbit(dx, dy);
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
            Gesture::Ground { start, .. } => {
                let moved = ((x - start.0).powi(2) + (y - start.1).powi(2)).sqrt() / self.h;
                if moved > TAP_SLOP {
                    self.gesture = Gesture::Orbiting { start, last: start };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Lake { start, at, since, .. } => {
                let moved = ((x - start.0).powi(2) + (y - start.1).powi(2)).sqrt() / self.h > TAP_SLOP;
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
        let was = self.pointers.len();
        self.pointers.retain(|p| p.0 != id);
        if was == self.pointers.len() {
            return;
        }
        // Lifting one finger of a squeeze ends it; the other finger does nothing until lifted.
        self.end_gesture();
    }

    /// Sample the drag segment finely, so a fast flick can't jump over the body.
    fn track_swipe(&mut self, from: (f32, f32), to: (f32, f32)) {
        if !self.world.division.can_slice() || !self.world.fusion.whole() || !self.world.burst.whole() {
            return;
        }
        let len = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
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
    fn throw(&mut self) {
        let (vx, vy) = (self.pvel.0 / self.h, self.pvel.1 / self.h);
        let speed = (vx * vx + vy * vy).sqrt();
        if speed < THROW_SPEED || !self.world.division.whole() || !self.world.fusion.whole() || !self.world.burst.whole() {
            return;
        }
        let b = self.cam.basis();
        let dir = (b.right * vx - b.up * vy).norm();
        self.world.body.vel += dir * (speed * 4.5).min(16.0);
        self.discover(Discovery::Throw);
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
    fn hover(&mut self, x: f32, y: f32) {
        let b = self.cam.basis();
        let to = self.plane_point(x, y, self.world.body.pos);
        // Reach toward the viewer, leaning toward the cursor.
        self.world.gaze_dir = ((to - self.world.body.pos) * 0.8 - b.fwd * 1.6).norm();
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
                self.world.warming = true;
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
        if let Gesture::Warming { since, .. } = self.gesture {
            if self.world.time - since > 1.2 {
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
        if self.idle > IDLE_WATCH {
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
            self.wheel_squeeze *= (-2.5 * dt).exp();
            self.world.body.squeeze = self.wheel_squeeze;
        }

        let sub = 3;
        for _ in 0..sub {
            self.world.step(dt / sub as f32);
        }

        let blend = self.world.division.blend();
        let base = Camera::new().dist;
        let spread = self.world.burst.spread();
        self.cam.dist = base + (self.cells_dist() - base) * blend.zoom + (burst::LANDSCAPE_DIST - base) * spread;
        // Step back and level the eye to take in the slab.
        let slab = self.world.division.morula.as_ref().map_or(0.0, |m| m.look().slab * (1.0 - m.look().thaw));
        if slab > 0.0 {
            self.cam.dist += (SLAB_DIST - self.cam.dist) * slab;
            if !orbiting {
                self.cam.pitch += (0.06 - self.cam.pitch) * (1.0 - (-1.2 * dt).exp()) * slab;
            }
        }
        if let Gesture::Ground { since, .. } = self.gesture {
            if self.clock - since > burst::GATHER_HOLD_SECS && self.world.burst.gather() {
                self.discover(Discovery::Gather);
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
                    self.sounds.push(Sound::Tear);
                    self.discover(Discovery::Tear);
                    // Keep holding what tore off.
                    if let Gesture::Stretching { handle: h, plane } = self.gesture {
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
