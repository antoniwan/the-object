//! Turns raw pointer input into verbs on the world. Everything here is in canvas
//! pixels and has no browser types, so new gestures can be added and tested natively.
//!
//! Gestures today:
//! - tap the liquid: poke
//! - press and drag the liquid: stretch (pull far enough and it tears)
//! - drag a drop: carry it; let go fast to fling it
//! - drag empty space: look around
//! - two-finger pinch or mouse wheel: squeeze
//! - press and hold still: warm it (it changes color)
//! - leave it alone: it starts to watch the pointer

use crate::camera::Camera;
use crate::math::*;
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
}

enum Gesture {
    Idle,
    Pending { start: (f32, f32), hit: Hit, since: f32 },
    Warming { start: (f32, f32), hit: Hit, since: f32 },
    Stretching { handle: u32, plane: V3 },
    Dragging { drop: u32, plane: V3, offset: V3 },
    Orbiting { last: (f32, f32) },
    Squeezing { start_dist: f32 },
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
            }
            Gesture::Warming { .. } => self.world.warming = false,
            Gesture::Stretching { handle, .. } => self.world.release_handle(handle),
            Gesture::Dragging { drop, .. } => {
                if self.world.release_drop(drop) > 4.0 {
                    self.discover(Discovery::Fling);
                }
            }
            Gesture::Squeezing { .. } => self.world.body.squeeze = 0.0,
            Gesture::Orbiting { .. } | Gesture::Idle => {}
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
        self.idle = 0.0;
        self.world.watching = false;
        let since = self.world.time;
        self.gesture = match self.pick(x, y) {
            Some(hit @ Hit { target: Target::Body, .. }) => Gesture::Pending { start: (x, y), hit, since },
            Some(Hit { target: Target::Drop(drop), pos, .. }) => {
                let center = self.world.drops.iter().find(|d| d.id == drop).map_or(pos, |d| d.pos);
                self.world.grab_drop(drop, center);
                Gesture::Dragging { drop, plane: pos, offset: center - pos }
            }
            None => Gesture::Orbiting { last: (x, y) },
        };
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32, dt: f32) {
        let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) else {
            self.hover(x, y);
            return;
        };
        p.1 = x;
        p.2 = y;
        match self.gesture {
            Gesture::Pending { start, hit, .. } | Gesture::Warming { start, hit, .. } => {
                let moved = ((x - start.0).powi(2) + (y - start.1).powi(2)).sqrt() / self.h;
                if moved > TAP_SLOP {
                    self.world.warming = false;
                    self.gesture = match self.world.begin_stretch(hit.pos) {
                        Some(handle) => Gesture::Stretching { handle, plane: hit.pos },
                        None => Gesture::Orbiting { last: (x, y) },
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
            Gesture::Orbiting { last } => {
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
                self.gesture = Gesture::Orbiting { last: (x, y) };
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
        if let Gesture::Pending { start, hit, since } = self.gesture {
            if self.world.time - since > HOLD {
                self.gesture = Gesture::Warming { start, hit, since: self.world.time };
                self.world.warming = true;
            }
        }
        if let Gesture::Warming { since, .. } = self.gesture {
            if self.world.time - since > 1.2 {
                self.discover(Discovery::Warm);
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

        if !matches!(self.gesture, Gesture::Squeezing { .. }) {
            self.wheel_squeeze *= (-2.5 * dt).exp();
            self.world.body.squeeze = self.wheel_squeeze;
        }

        let sub = 3;
        for _ in 0..sub {
            self.world.step(dt / sub as f32);
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
