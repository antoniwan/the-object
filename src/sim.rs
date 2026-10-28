//! The liquid world: one body, the tendrils pulled out of it, ripples on its
//! surface, and free droplets. Pure Rust, no browser types, so it is testable natively.
//!
//! Shapes are signed distance fields joined with a smooth minimum, which is what
//! makes everything read as one liquid. The shader in `render.rs` mirrors `sdf()`.

use crate::math::*;
use crate::fusion::Fusion;
use crate::meiosis::Division;

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

pub struct Body {
    pub pos: V3,
    pub vel: V3,
    /// Volume in units of radius cubed, so radius = cbrt(volume).
    pub volume: f32,
    /// Vertical scale; horizontal scale is 1/sqrt(squash), so volume is preserved.
    pub squash: f32,
    pub squash_vel: f32,
    /// Squeeze input in -1..1, set by the player every frame while squeezing.
    pub squeeze: f32,
}

impl Body {
    pub fn radius(&self) -> f32 {
        self.volume.cbrt()
    }
    pub fn scale(&self) -> V3 {
        let s = 1.0 / self.squash.sqrt();
        v3(s, self.squash, s)
    }
}

/// A tendril of liquid pulled out of the body.
pub struct Handle {
    pub id: u32,
    /// Direction from the body center the tendril grows from.
    pub dir: V3,
    pub tip: V3,
    pub vel: V3,
    /// Where the player's pointer wants the tip to be, while held.
    pub target: Option<V3>,
}

impl Handle {
    pub fn anchor(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.45)
    }
    pub fn rest(&self, b: &Body) -> V3 {
        b.pos + self.dir * (b.radius() * 0.8)
    }
    pub fn stretch(&self, b: &Body) -> f32 {
        (self.tip - self.rest(b)).len() / b.radius()
    }
    /// (base radius, neck radius, bulb radius). The neck thins as it stretches.
    pub fn radii(&self, b: &Body) -> (f32, f32, f32) {
        let r = b.radius();
        let s = self.stretch(b).min(TEAR_RATIO);
        let neck = r * (0.24 - 0.13 * s).max(0.05);
        (r * 0.42, neck, r * 0.27)
    }
    pub fn sdf(&self, b: &Body, p: V3) -> f32 {
        let (base, neck, bulb) = self.radii(b);
        let stem = sd_taper(p, self.anchor(b), self.tip, base, neck);
        smin(stem, (p - self.tip).len() - bulb, 0.12 * b.radius())
    }
}

pub struct Ripple {
    pub dir: V3,
    pub amp: f32,
    pub age: f32,
}

pub struct Drop {
    pub id: u32,
    pub pos: V3,
    pub vel: V3,
    pub r: f32,
    pub target: Option<V3>,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Body,
    Drop(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub pos: V3,
    pub normal: V3,
    pub target: Target,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A tendril pinched off into a new drop.
    Tore { handle: u32, drop: u32 },
    /// A drop fell back into the body.
    MergedIntoBody { drop: u32 },
    /// Two drops became one; `absorbed` no longer exists, `into` does.
    MergedDrops { absorbed: u32, into: u32 },
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
    pub time: f32,
    next_id: u32,
    events: Vec<Event>,
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
            },
            handles: Vec::new(),
            ripples: Vec::new(),
            drops: Vec::new(),
            division: Division::default(),
            fusion: Fusion::default(),
            time: 0.0,
            next_id: 1,
            events: Vec::new(),
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

    fn handle_mut(&mut self, id: u32) -> Option<&mut Handle> {
        self.handles.iter_mut().find(|h| h.id == id)
    }

    pub fn add_ripple(&mut self, dir: V3, amp: f32) {
        if self.ripples.len() >= MAX_RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push(Ripple { dir, amp, age: 0.0 });
    }

    // ---- verbs -------------------------------------------------------------

    pub fn poke(&mut self, hit: Hit, strength: f32) {
        match hit.target {
            Target::Body => {
                // On two cells, ripples spread from the touched cell's own center.
                let dir = self.division.ripple_dir(hit.pos, self.body.pos, self.body.radius());
                self.add_ripple(dir, strength);
                self.body.vel -= dir * (0.9 * strength);
                // Poking from above flattens it, from the side makes it bob up.
                self.body.squash_vel += if dir.y.abs() > 0.6 { -4.0 } else { 2.5 } * strength;
            }
            Target::Drop(id) => {
                if let Some(d) = self.drop_mut(id) {
                    d.vel -= hit.normal * (1.6 * strength);
                }
            }
        }
    }

    pub fn begin_stretch(&mut self, at: V3) -> Option<u32> {
        if self.handles.len() >= MAX_HANDLES {
            return None;
        }
        let id = self.id();
        let dir = (at - self.body.pos).norm();
        self.handles.push(Handle { id, dir, tip: at, vel: V3::ZERO, target: Some(at) });
        Some(id)
    }

    pub fn pull_handle(&mut self, id: u32, to: V3) {
        if let Some(h) = self.handle_mut(id) {
            h.target = Some(to);
        }
    }

    pub fn release_handle(&mut self, id: u32) {
        if let Some(h) = self.handle_mut(id) {
            h.target = None;
        }
    }

    pub fn grab_drop(&mut self, id: u32, to: V3) {
        if let Some(d) = self.drop_mut(id) {
            d.target = Some(to);
        }
    }

    pub fn release_drop(&mut self, id: u32) -> f32 {
        match self.drop_mut(id) {
            Some(d) => {
                d.target = None;
                d.vel.len()
            }
            None => 0.0,
        }
    }

    // ---- simulation --------------------------------------------------------

    pub fn step(&mut self, dt: f32) {
        self.time += dt;

        // Body: springs back home and to round, wobbling on the way. Home is the
        // middle of the room, or under the floor once it has fused with it.
        let b = &mut self.body;
        let home = self.fusion.home(b.radius());
        let (k, c) = if self.fusion.whole() { (28.0, 3.5) } else { (10.0, 5.0) };
        b.vel += ((home - b.pos) * k - b.vel * c) * dt;
        b.pos += b.vel * dt;
        let (pos, vel, kick) = self.fusion.collide(b.pos, b.vel, b.radius(), b.squash);
        b.pos = pos;
        b.vel = vel;
        b.squash_vel += kick;
        // It flattens as it melts into the floor.
        let melt = 1.0 - 0.5 * self.fusion.blend().sink;
        let squash_goal = (1.0 + b.squeeze * 0.55).clamp(0.45, 1.9).min(melt);
        b.squash_vel += ((squash_goal - b.squash) * 140.0 - b.squash_vel * 5.0) * dt;
        b.squash = (b.squash + b.squash_vel * dt).clamp(0.3, 2.4);

        self.division.step(dt, b.squeeze);
        self.fusion.step(dt);
        self.step_handles(dt);

        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 3.5);

        self.step_drops(dt);
    }

    fn step_handles(&mut self, dt: f32) {
        let mut tears = Vec::new();
        for h in &mut self.handles {
            let rest = h.rest(&self.body);
            let r = self.body.radius();
            let (k, c) = if h.target.is_some() { (300.0, 24.0) } else { (110.0, 6.0) };
            let mut goal = h.target.unwrap_or(rest);
            // Can't pull it further than this when it isn't allowed to tear.
            let max = r * (TEAR_RATIO + 0.25);
            if (goal - rest).len() > max {
                goal = rest + (goal - rest).norm() * max;
            }
            h.vel += ((goal - h.tip) * k - h.vel * c) * dt;
            h.tip += h.vel * dt;
            if h.target.is_some() {
                // The body leans toward whatever is pulling it.
                self.body.vel += (h.tip - rest) * (1.5 * dt);
                if h.stretch(&self.body) > TEAR_RATIO {
                    tears.push(h.id);
                }
            }
        }
        for id in tears {
            self.tear(id);
        }
        let body = &self.body;
        self.handles.retain(|h| {
            h.target.is_some() || (h.tip - h.rest(body)).len() > 0.004 || h.vel.len() > 0.05
        });
    }

    fn tear(&mut self, handle_id: u32) {
        if self.drops.len() >= MAX_DROPS || self.body.radius() < MIN_BODY_RADIUS {
            return;
        }
        let drop_id = self.id();
        let body = &self.body;
        let Some(h) = self.handles.iter_mut().find(|h| h.id == handle_id) else { return };
        let (_, _, bulb) = h.radii(body);
        let r = bulb * 1.05;
        let rest = h.rest(body);
        let drop = Drop { id: drop_id, pos: h.tip, vel: h.vel, r, target: h.target, age: 0.0 };
        // The stump snaps back toward the body with some overshoot.
        h.target = None;
        h.vel = (rest - h.tip) * 4.0;
        h.tip = rest + (h.tip - rest) * 0.45;
        let dir = h.dir;
        self.body.volume -= r * r * r;
        self.drops.push(drop);
        self.add_ripple(dir, 0.5);
        self.events.push(Event::Tore { handle: handle_id, drop: drop_id });
    }

    fn step_drops(&mut self, dt: f32) {
        let body_pos = self.body.pos;
        for d in &mut self.drops {
            d.age += dt;
            if let Some(t) = d.target {
                d.vel += ((t - d.pos) * 220.0 - d.vel * 22.0) * dt;
            } else {
                // Free drops drift slowly home.
                let to = body_pos - d.pos;
                d.vel += (to.norm() * 0.55 - d.vel * 0.5) * dt;
                // Soft wall so nothing floats out of view.
                let out = d.pos.len() - 3.2;
                if out > 0.0 {
                    d.vel -= d.pos.norm() * (out * 30.0 * dt);
                }
            }
            d.pos += d.vel * dt;
        }

        // Drops falling back into the body.
        let r = self.body.radius();
        let mut i = 0;
        while i < self.drops.len() {
            let d = &self.drops[i];
            let (gap, _) = self.division.nearest_cell(d.pos, self.body.pos, r);
            if d.age > MERGE_COOLDOWN && gap < d.r * 0.2 {
                let d = self.drops.remove(i);
                let v = d.r * d.r * d.r;
                self.body.vel += d.vel * (v / (self.body.volume + v));
                self.body.volume += v;
                self.add_ripple((d.pos - self.body.pos).norm(), 0.8);
                self.events.push(Event::MergedIntoBody { drop: d.id });
            } else {
                i += 1;
            }
        }

        // Drops merging with each other. The held drop (or the larger one) survives.
        let mut i = 0;
        while i < self.drops.len() {
            let mut j = i + 1;
            while j < self.drops.len() {
                let (a, b) = (&self.drops[i], &self.drops[j]);
                let close = (a.pos - b.pos).len() < (a.r + b.r) * 0.8;
                if close && a.age > MERGE_COOLDOWN && b.age > MERGE_COOLDOWN {
                    let keep_a = a.target.is_some() || (b.target.is_none() && a.r >= b.r);
                    let (ki, gi) = if keep_a { (i, j) } else { (j, i) };
                    let gone = self.drops.remove(gi);
                    let ki = if gi < ki { ki - 1 } else { ki };
                    let k = &mut self.drops[ki];
                    let (vk, vg) = (k.r.powi(3), gone.r.powi(3));
                    let w = vg / (vk + vg);
                    if k.target.is_none() {
                        k.pos = k.pos.lerp(gone.pos, w);
                    }
                    k.vel = k.vel.lerp(gone.vel, w);
                    k.r = (vk + vg).cbrt();
                    let into = k.id;
                    self.events.push(Event::MergedDrops { absorbed: gone.id, into });
                    // Restart the scan; indices have shifted.
                    i = 0;
                    j = 1;
                    continue;
                }
                j += 1;
            }
            i += 1;
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ---- geometry ----------------------------------------------------------

    /// Distance to the liquid surface, and which object is nearest. Ripples are
    /// left out: they are cosmetic and only exist in the shader.
    pub fn sdf(&self, p: V3) -> (f32, Target) {
        let b = &self.body;
        let r = b.radius();
        let s = b.scale();
        let mut d = self.division.body_sdf(p, b.pos, r, s);
        for h in &self.handles {
            d = smin(d, h.sdf(b, p), K_HANDLE * r);
        }
        let mut target = Target::Body;
        let mut nearest = d;
        for drop in &self.drops {
            let dd = (p - drop.pos).len() - drop.r;
            if dd < nearest {
                nearest = dd;
                target = Target::Drop(drop.id);
            }
            d = smin(d, dd, K_DROP * r);
        }
        (d, target)
    }

    pub fn normal(&self, p: V3) -> V3 {
        let e = 0.002;
        let f = |o: V3| self.sdf(p + o).0;
        v3(
            f(v3(e, 0.0, 0.0)) - f(v3(-e, 0.0, 0.0)),
            f(v3(0.0, e, 0.0)) - f(v3(0.0, -e, 0.0)),
            f(v3(0.0, 0.0, e)) - f(v3(0.0, 0.0, -e)),
        )
        .norm()
    }

    pub fn raycast(&self, ro: V3, rd: V3) -> Option<Hit> {
        let mut t = 0.0;
        for _ in 0..160 {
            let p = ro + rd * t;
            let (d, target) = self.sdf(p);
            if d < 0.001 {
                return Some(Hit { pos: p, normal: self.normal(p), target });
            }
            t += d * 0.9;
            if t > 20.0 {
                break;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &mut World, seconds: f32) {
        for _ in 0..(seconds * 120.0) as usize {
            w.step(1.0 / 120.0);
        }
    }

    #[test]
    fn camera_ray_hits_the_body() {
        let w = World::new();
        let hit = w.raycast(v3(0.0, 0.0, 4.0), v3(0.0, 0.0, -1.0)).unwrap();
        assert_eq!(hit.target, Target::Body);
        assert!((hit.pos.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn pulling_far_tears_a_drop_and_volume_is_conserved() {
        let mut w = World::new();
        let v0 = w.total_volume();
        let h = w.begin_stretch(v3(1.0, 0.0, 0.0)).unwrap();
        w.pull_handle(h, v3(4.0, 0.0, 0.0));
        run(&mut w, 1.0);
        let events = w.drain_events();
        assert!(matches!(events[0], Event::Tore { .. }), "{events:?}");
        assert_eq!(w.drops.len(), 1);
        assert!((w.total_volume() - v0).abs() < 1e-4);
    }

    #[test]
    fn a_released_drop_drifts_home_and_merges() {
        let mut w = World::new();
        let v0 = w.total_volume();
        let h = w.begin_stretch(v3(1.0, 0.0, 0.0)).unwrap();
        w.pull_handle(h, v3(4.0, 0.0, 0.0));
        run(&mut w, 1.0);
        let id = w.drops[0].id;
        w.release_drop(id);
        run(&mut w, 15.0);
        assert!(w.drops.is_empty());
        assert!(w.drain_events().contains(&Event::MergedIntoBody { drop: id }));
        assert!((w.total_volume() - v0).abs() < 1e-4);
        assert!(w.handles.is_empty(), "stump should settle and disappear");
    }

    #[test]
    fn a_short_pull_springs_back() {
        let mut w = World::new();
        let h = w.begin_stretch(v3(0.0, 1.0, 0.0)).unwrap();
        w.pull_handle(h, v3(0.0, 1.5, 0.0));
        run(&mut w, 0.5);
        assert!(w.drops.is_empty());
        w.release_handle(h);
        run(&mut w, 4.0);
        assert!(w.handles.is_empty());
    }
}
