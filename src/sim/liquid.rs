//! The liquid's own motion each frame: tendrils springing and tearing into drops, drops
//! drifting home, pushing apart, falling back into the body and running together.

use super::{Drop, Event, World, MAX_DROPS, MERGE_COOLDOWN, MIN_BODY_RADIUS, TEAR_RATIO};
use crate::math::*;

impl World {
    pub(super) fn step_handles(&mut self, dt: f32) {
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
        let body = &self.body;
        let Some(h) = self.handles.iter().find(|h| h.id == handle_id) else { return };
        let (_, _, bulb) = h.radii(body);
        let r = bulb * 1.05;
        // what is left must still be at least the smallest body, the same rule as spill()
        if self.drops.len() >= MAX_DROPS || (body.volume - r * r * r).cbrt() < MIN_BODY_RADIUS {
            return;
        }
        let drop_id = self.id();
        let body = &self.body;
        let Some(h) = self.handles.iter_mut().find(|h| h.id == handle_id) else { return };
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

    pub(super) fn step_drops(&mut self, dt: f32) {
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
                let tangent = V3::UP.cross(to).norm();
                d.vel += (to.norm() * (0.22 * (1.0 - 0.85 * night)) + tangent * (0.05 * night) - d.vel * (1.2 - 1.0 * night)) * dt;
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
            let (gap, _) = self.division.nearest_cell(d.pos, self.body.pos, r);
            if d.age > MERGE_COOLDOWN && gap < d.r * 0.2 {
                let d = self.drops.remove(i);
                let v = d.r * d.r * d.r;
                let w = v / (self.body.volume + v);
                self.body.vel += d.vel * w;
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
                // The wound's pieces hold their places, and never run together (wound.rs).
                let pieces = self.wound.is_caltrop(a.id) && self.wound.is_caltrop(b.id);
                if close && carried && !pieces && a.age > MERGE_COOLDOWN && b.age > MERGE_COOLDOWN {
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
}
