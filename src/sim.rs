//! The liquid world: one body, the tendrils pulled out of it, ripples on its
//! surface, and free droplets. Pure Rust, no browser types, so it is testable natively.
//!
//! Shapes are signed distance fields joined with a smooth minimum, which is what
//! makes everything read as one liquid. The shader in `render.rs` mirrors `sdf()`.

use crate::math::*;

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
    pub color: V3,
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
    pub color: V3,
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
    /// Two different colors became one.
    Mixed,
    /// Poked too often, too fast.
    Provoked,
    /// Enough of it is free that the world turns to night.
    Opened,
    /// Everything came back; day returns.
    Closed,
}

pub struct World {
    pub body: Body,
    pub handles: Vec<Handle>,
    pub ripples: Vec<Ripple>,
    pub drops: Vec<Drop>,
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

    fn add_ripple(&mut self, dir: V3, amp: f32) {
        if self.ripples.len() >= MAX_RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push(Ripple { dir, amp, age: 0.0 });
    }

    // ---- verbs -------------------------------------------------------------

    pub fn poke(&mut self, hit: Hit, strength: f32) {
        match hit.target {
            Target::Body => {
                let now = self.time;
                self.recent_pokes.retain(|t| now - *t < PROVOKE_WINDOW);
                self.recent_pokes.push(now);
                if self.recent_pokes.len() >= PROVOKE_POKES && self.agitation < 0.5 {
                    self.events.push(Event::Provoked);
                }
                if self.recent_pokes.len() >= PROVOKE_POKES {
                    self.agitation = 1.0;
                }
                // Touching it startles it out of watching.
                self.gaze *= 0.2;
                // Upset, it jumps further, but its shape stays its own.
                let dir = (hit.pos - self.body.pos).norm();
                self.add_ripple(dir, strength);
                self.body.vel -= dir * (0.9 * strength * (1.0 + self.agitation));
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

    /// Lean briefly toward something.
    pub fn reach(&mut self, toward: V3) {
        self.gaze_dir = toward.norm();
        self.gaze = self.gaze.max(0.7);
    }

    pub fn begin_stretch(&mut self, at: V3) -> Option<u32> {
        if self.handles.len() >= MAX_HANDLES {
            // Let go of the oldest stump that is still settling, if there is one.
            let i = self.handles.iter().position(|h| h.target.is_none())?;
            self.handles.remove(i);
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

        // Body: springs back to the center and to round, wobbling on the way.
        let b = &mut self.body;
        b.vel += (-b.pos * 28.0 - b.vel * 3.5) * dt;
        b.pos += b.vel * dt;
        let squash_goal = (1.0 + b.squeeze * 0.55).clamp(0.45, 1.9);
        b.squash_vel += ((squash_goal - b.squash) * 140.0 - b.squash_vel * 5.0) * dt;
        b.squash = (b.squash + b.squash_vel * dt).clamp(0.5, 1.9);

        self.step_handles(dt);

        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 3.5);

        self.step_drops(dt);
        self.step_moods(dt);
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
        let color = self.body.color;
        let drop = Drop { id: drop_id, pos: h.tip, vel: h.vel, r, target: h.target, age: 0.0, color };
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
        let night = self.night;
        for d in &mut self.drops {
            d.age += dt;
            if let Some(t) = d.target {
                d.vel += ((t - d.pos) * 220.0 - d.vel * 22.0) * dt;
            } else {
                // Free drops drift slowly home.
                let to = body_pos - d.pos;
                // At night they orbit instead, spiralling home slowly.
                let n = night;
                let tangent = V3::UP.cross(to).norm();
                d.vel += (to.norm() * (0.22 * (1.0 - 0.85 * n)) + tangent * (0.05 * n) - d.vel * (1.2 - 1.0 * n)) * dt;
                // Soft wall so nothing floats out of view.
                let out = d.pos.len() - 3.2;
                if out > 0.0 {
                    let n = d.pos.norm();
                    let outward = d.vel.dot(n).max(0.0);
                    d.vel -= n * (outward * (1.0 - (-8.0 * dt).exp()) + out * 6.0 * dt);
                }
            }
            d.pos += d.vel * dt;
        }

        // Free drops gently push each other apart, so they spread out like a constellation.
        for i in 0..self.drops.len() {
            for j in (i + 1)..self.drops.len() {
                let d = self.drops[i].pos - self.drops[j].pos;
                let gap = d.len() - (self.drops[i].r + self.drops[j].r) * 1.6;
                if gap < 0.0 {
                    let push = d.norm() * (-gap * 4.0 * dt);
                    if self.drops[i].target.is_none() {
                        self.drops[i].vel += push;
                    }
                    if self.drops[j].target.is_none() {
                        self.drops[j].vel -= push;
                    }
                }
            }
        }

        // Drops falling back into the body.
        let r = self.body.radius();
        let mut i = 0;
        while i < self.drops.len() {
            let d = &self.drops[i];
            let dist = (d.pos - self.body.pos).len();
            if d.age > MERGE_COOLDOWN && dist < r + d.r * 0.2 {
                let d = self.drops.remove(i);
                let v = d.r * d.r * d.r;
                self.body.vel += d.vel * (v / (self.body.volume + v));
                let w = v / (self.body.volume + v);
                if (self.body.color - d.color).len() > 0.08 {
                    self.events.push(Event::Mixed);
                }
                self.body.color = self.body.color.lerp(d.color, w);
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
                // Free drops keep their distance; one has to be carried into another.
                let carried = a.target.is_some() || b.target.is_some();
                if close && carried && a.age > MERGE_COOLDOWN && b.age > MERGE_COOLDOWN {
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
                    let mixed = (k.color - gone.color).len() > 0.08;
                    k.color = k.color.lerp(gone.color, w);
                    if mixed {
                        self.events.push(Event::Mixed);
                    }
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

    fn step_moods(&mut self, dt: f32) {
        // Agitation: it shivers, then calms down.
        self.agitation *= (-dt / 3.0).exp();
        if self.agitation > 0.01 {
            let t = self.time;
            let shake = v3((t * 37.0).sin(), (t * 41.0).cos(), (t * 29.0).sin());
            self.body.vel += shake * (9.0 * self.agitation * dt);
        }

        // Warmth: held still, it drifts through colors.
        if self.warming {
            self.warm_phase += dt * 0.22;
            let goal = palette(self.warm_phase);
            self.body.color = self.body.color.lerp(goal, 1.0 - (-1.5 * dt).exp());
        }

        // Watching: it slowly grows toward whoever is looking.
        let goal = if self.watching { 1.0 } else { 0.0 };
        let rate = if self.watching { 0.25 } else { 3.0 };
        self.gaze += (goal - self.gaze) * (1.0 - (-rate * dt).exp());

        // Opening: enough of it free at once and the world turns to night.
        if !self.opened && self.drops.len() >= OPEN_AT {
            self.opened = true;
            self.events.push(Event::Opened);
        } else if self.opened && self.drops.is_empty() {
            self.opened = false;
            self.events.push(Event::Closed);
        }
        let goal = if self.opened { 1.0 } else { 0.0 };
        self.night += (goal - self.night) * (1.0 - (-0.7 * dt).exp());
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
        let q = (p - b.pos).div(s);
        let mut d = (q.len() - r) * s.x.min(s.y);
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
