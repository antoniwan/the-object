//! The liquid world: one body, the tendrils pulled out of it, ripples on its
//! surface, and free droplets. Pure Rust, no browser types, so it is testable natively.
//!
//! Shapes are signed distance fields joined with a smooth minimum, which is what
//! makes everything read as one liquid. The shader in `src/shader.frag` mirrors `sdf()`.

use crate::math::*;
use crate::burst::Burst;
use crate::forms::Forms;
use crate::floor::fusion::Fusion;
use crate::division::meiosis::Division;
use crate::other::Other;

mod geometry;
mod liquid;
mod moods;
mod parts;
mod verbs;
#[cfg(test)]
mod tests;

pub use parts::{Body, Drop, Event, Handle, Hit, Ripple, Target};

pub const MAX_HANDLES: usize = 4;
pub const MAX_RIPPLES: usize = 8;
pub const MAX_DROPS: usize = 12;

/// The body never gets smaller than this by tearing pieces off it.
const MIN_BODY_RADIUS: f32 = 0.45;
/// A tendril pinches off once it is this many body radii past its rest point.
const TEAR_RATIO: f32 = 1.35;
/// Freshly torn or merged drops can't merge again until they are this old (seconds).
const MERGE_COOLDOWN: f32 = 0.5;
/// Smoothing radii for the liquid joins, as a fraction of the body radius.
pub const K_HANDLE: f32 = 0.30;
pub const K_DROP: f32 = 0.22;
/// This many free drops at once and it opens.
pub const OPEN_AT: usize = 6;
/// Pokes within PROVOKE_WINDOW seconds that count as harassment.
const PROVOKE_POKES: usize = 6;
const PROVOKE_WINDOW: f32 = 2.0;

pub const PEARL: V3 = v3(0.96, 0.96, 0.98);

/// Soft pastel cycle the body drifts through while it is held still.
pub fn palette(t: f32) -> V3 {
    let tau = std::f32::consts::TAU;
    let c = |o: f32| 0.70 + 0.30 * (tau * (t + o)).cos();
    v3(c(0.0), c(0.33), c(0.67))
}

pub struct World {
    pub body: Body,
    pub handles: Vec<Handle>,
    pub ripples: Vec<Ripple>,
    pub drops: Vec<Drop>,
    /// Slicing: whether the body is one liquid or two cells.
    pub division: Division,
    /// Slamming: whether it has fused with the floor.
    pub fusion: Fusion,
    /// Poking it to death: wounds, and the twelve pieces it bursts into.
    pub burst: Burst,
    /// Pinch, stretch, stir and draw around it: what else it can become (see forms/).
    pub forms: Forms,
    /// Something else in the room, late on (see other.rs). Not part of the body: it never
    /// changes whether the body is ordinary.
    pub other: Other,
    /// An old hurt, a scar and what opens out of it (see wound.rs).
    pub wound: crate::wound::Wound,
    pub time: f32,
    next_id: u32,
    events: Vec<Event>,
    /// 0..1, how upset it is. Decays.
    pub agitation: f32,
    recent_pokes: Vec<f32>,
    /// 0..1 blend toward the night world.
    pub night: f32,
    pub opened: bool,
    /// Set by the player while holding it still: it slowly changes color.
    pub warming: bool,
    warm_phase: f32,
    /// Set when nobody has touched it for a while: it reaches toward the viewer.
    pub watching: bool,
    pub gaze_dir: V3,
    pub gaze: f32,
    /// 0..1: how far it turns toward the pointer even while someone is there (see contact.rs).
    pub regard: f32,
    /// 0..1: the room darkened from outside the touch, as when it cannot see (senses.rs).
    pub dim: f32,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> World {
        World {
            body: Body {
                pos: V3::ZERO,
                vel: V3::ZERO,
                volume: 1.0,
                squash: 1.0,
                squash_vel: 0.0,
                squeeze: 0.0,
                color: PEARL,
            },
            handles: Vec::new(),
            ripples: Vec::new(),
            drops: Vec::new(),
            division: Division::default(),
            fusion: Fusion::default(),
            burst: Burst::default(),
            forms: Forms::default(),
            other: Other::default(),
            wound: Default::default(),
            time: 0.0,
            next_id: 1,
            events: Vec::new(),
            agitation: 0.0,
            recent_pokes: Vec::new(),
            night: 0.0,
            opened: false,
            warming: false,
            warm_phase: 0.0,
            watching: false,
            gaze_dir: v3(0.0, 0.0, 1.0),
            gaze: 0.0,
            regard: 0.0,
            dim: 0.0,
        }
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Total liquid in the world. Tearing and merging move it around but never change it.
    pub fn total_volume(&self) -> f32 {
        self.body.volume + self.drops.iter().map(|d| d.r * d.r * d.r).sum::<f32>()
    }

    pub fn drop_mut(&mut self, id: u32) -> Option<&mut Drop> {
        self.drops.iter_mut().find(|d| d.id == id)
    }

    /// Still the plain liquid: not divided, fused, burst, or turned into any other form.
    pub fn ordinary(&self) -> bool {
        self.division.whole() && self.fusion.whole() && self.burst.whole() && self.forms.whole()
    }

    /// Let some of the body out as a free drop. Returns false when there is no room or not enough of it.
    pub fn spill(&mut self, pos: V3, vel: V3, r: f32, color: V3) -> bool {
        if self.drops.len() >= MAX_DROPS || (self.body.volume - r * r * r).cbrt() < MIN_BODY_RADIUS {
            return false;
        }
        let id = self.id();
        self.body.volume -= r * r * r;
        self.drops.push(Drop { id, pos, vel, r, target: None, age: 0.0, color });
        true
    }

    pub fn add_ripple(&mut self, dir: V3, amp: f32) {
        if self.ripples.len() >= MAX_RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push(Ripple { dir, amp, age: 0.0 });
    }

    // ---- simulation --------------------------------------------------------

    pub fn step(&mut self, dt: f32) {
        self.time += dt;

        // Body: springs back home and to round, wobbling on the way. Home is the
        // middle of the room, or under the floor once it has fused with it.
        let b = &mut self.body;
        let r = b.radius();
        let home = self.fusion.home(r);
        let (k, c) = if self.fusion.whole() { (28.0, 3.5) } else { (10.0, 5.0) };
        b.vel += ((home - b.pos) * k - b.vel * c) * dt;
        b.pos += b.vel * dt;
        let (pos, vel, kick) = self.fusion.collide(b.pos, b.vel, r, b.squash);
        b.pos = pos;
        b.vel = vel;
        b.squash_vel += kick;
        // It flattens as it melts into the floor.
        let melt = 1.0 - 0.5 * self.fusion.blend().sink;
        let squash_goal = (1.0 + b.squeeze * 0.55).clamp(0.45, 1.9).min(melt);
        b.squash_vel += ((squash_goal - b.squash) * 140.0 - b.squash_vel * 5.0) * dt;
        b.squash = (b.squash + b.squash_vel * dt).clamp(0.5, 1.9);

        let watch = self.gaze_dir;
        self.division.step(dt, b.squeeze, watch);
        if self.division.is_stone() {
            // Stone doesn't move, and doesn't give.
            let b = &mut self.body;
            b.vel = V3::ZERO;
            b.pos = b.pos.lerp(home, 1.0 - (-4.0 * dt).exp());
            b.squash = 1.0;
            b.squash_vel = 0.0;
        }
        self.fusion.step(dt);
        self.burst.step(dt);
        self.forms.step(dt);
        // gone to dust, it is in the mound, and comes back up out of it (forms/dust.rs)
        if let Some(at) = self.forms.dust.body_at(home) {
            self.body.pos = at;
            self.body.vel = V3::ZERO;
        }
        self.step_handles(dt);

        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 3.5);

        self.step_drops(dt);
        self.step_moods(dt);
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}
