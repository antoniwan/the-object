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

use crate::camera::{Camera, FOCAL};
use crate::math::*;
use crate::meiosis::{self, Swipe};
use crate::progress::{Discovery, Progress};
use crate::sim::{Event, Hit, Target, World};

/// How far (in canvas pixels, scaled by canvas height) a press can move and still count as a tap.
const TAP_SLOP: f32 = 0.012;

enum Gesture {
    Idle,
    Pending { start: (f32, f32), hit: Hit },
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
    swipe: Swipe,
    /// Real time, for timing swipes even while the world is frozen.
    clock: f32,
    /// Real seconds left of the freeze right after a cut.
    hit_stop: f32,
    snapped: bool,
    /// New discoveries for the HUD to announce.
    pub announcements: Vec<Discovery>,
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
            announcements: Vec::new(),
        }
    }

    fn discover(&mut self, d: Discovery) {
        if self.progress.discover(d) {
            self.announcements.push(d);
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
                self.discover(Discovery::Poke);
                if self.world.division.phase == meiosis::Phase::Two {
                    self.discover(Discovery::Twin);
                }
            }
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
                self.end_gesture();
                self.gesture = Gesture::Squeezing { start_dist: self.pinch_dist() };
                return;
            }
            _ => return,
        }
        self.cam.yaw_vel = 0.0;
        self.cam.pitch_vel = 0.0;
        self.swipe = Swipe::default();
        self.gesture = match self.pick(x, y) {
            Some(hit @ Hit { target: Target::Body, .. }) => Gesture::Pending { start: (x, y), hit },
            Some(Hit { target: Target::Drop(drop), pos, .. }) => {
                let center = self.world.drops.iter().find(|d| d.id == drop).map_or(pos, |d| d.pos);
                self.world.grab_drop(drop, center);
                Gesture::Dragging { drop, plane: pos, offset: center - pos }
            }
            None => Gesture::Orbiting { last: (x, y) },
        };
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32, dt: f32) {
        let Some(p) = self.pointers.iter_mut().find(|p| p.0 == id) else { return };
        p.1 = x;
        p.2 = y;
        match self.gesture {
            Gesture::Pending { start, hit } => {
                let moved = ((x - start.0).powi(2) + (y - start.1).powi(2)).sqrt() / self.h;
                if moved > TAP_SLOP {
                    // Cells don't stretch into tendrils; dragging them just looks around.
                    let stretch = if self.world.division.whole() { self.world.begin_stretch(hit.pos) } else { None };
                    self.gesture = match stretch {
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

    /// Sample the drag segment finely, so a fast flick can't jump over the body.
    fn track_swipe(&mut self, from: (f32, f32), to: (f32, f32)) {
        if !self.world.division.whole() {
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
        if self.world.division.slice(along.cross(b.fwd)) {
            self.hit_stop = meiosis::HIT_STOP_SECS;
            self.snapped = false;
            self.cam.shake = 0.05;
            self.cam.yaw_vel = 0.0;
            self.cam.pitch_vel = 0.0;
            self.world.body.squash_vel -= 2.5;
            self.discover(Discovery::Divide);
        }
    }

    /// Distance at which both cells, side by side, just fit across the screen.
    fn cells_dist(&self) -> f32 {
        let aspect = self.w / self.h.max(1.0);
        let half = (meiosis::SEP + meiosis::CELL_SCALE) * self.world.body.radius();
        (FOCAL * half / (0.82 * aspect)).clamp(2.8, 7.5)
    }

    pub fn wheel(&mut self, dy: f32) {
        self.wheel_squeeze = (self.wheel_squeeze - dy * 0.0025).clamp(-1.0, 1.0);
        if self.wheel_squeeze.abs() > 0.3 {
            self.discover(Discovery::Squeeze);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt;
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
        self.cam.dist = Camera::new().dist + (self.cells_dist() - Camera::new().dist) * blend.zoom;
        // The neck snaps: each new cell recoils where they were joined.
        if !self.snapped && blend.split > 0.82 && matches!(self.world.division.phase, meiosis::Phase::Dividing { .. }) {
            self.snapped = true;
            self.world.add_ripple(self.world.division.axis, 0.9);
            self.cam.shake = self.cam.shake.max(0.035);
        }
        for e in self.world.division.drain_events() {
            if e == meiosis::Event::Rejoined {
                self.discover(Discovery::Rejoin);
                self.world.add_ripple(V3::UP, 0.7);
                self.world.add_ripple(-V3::UP, 0.7);
            }
        }

        for e in self.world.drain_events() {
            match e {
                Event::Tore { handle, drop } => {
                    self.discover(Discovery::Tear);
                    // Keep holding what tore off.
                    if let Gesture::Stretching { handle: h, plane } = self.gesture {
                        if h == handle {
                            self.gesture = Gesture::Dragging { drop, plane, offset: V3::ZERO };
                        }
                    }
                }
                Event::MergedIntoBody { drop } => {
                    self.discover(Discovery::Merge);
                    if let Gesture::Dragging { drop: d, .. } = self.gesture {
                        if d == drop {
                            self.gesture = Gesture::Idle;
                        }
                    }
                }
                Event::MergedDrops { absorbed, into } => {
                    self.discover(Discovery::Merge);
                    if let Gesture::Dragging { drop, plane, offset } = self.gesture {
                        if drop == absorbed {
                            self.gesture = Gesture::Dragging { drop: into, plane, offset };
                        }
                    }
                }
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
