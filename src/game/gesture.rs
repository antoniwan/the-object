//! Pointer verbs: what a press, a drag, a release or the wheel does, by what is under it.

use super::*;

pub(super) enum Gesture {
    Idle,
    Pending { start: (f32, f32), hit: Hit, since: f32 },
    Warming { start: (f32, f32), hit: Hit, since: f32 },
    /// `since` when it started (real time), `reach` the furthest pull so far in body radii,
    /// `taut` seconds held nearly tearing, `angle` where it was last around the body, for stirring;
    /// `hit` where and `pressed` when (real time) it was first touched, for a tap that slid a little.
    Stretching { handle: u32, plane: V3, since: f32, reach: f32, taut: f32, angle: Option<f32>, hit: Hit, pressed: f32 },
    Dragging { drop: u32, plane: V3, offset: V3 },
    Orbiting { start: (f32, f32), last: (f32, f32) },
    /// Two fingers: `since` they began (real time), `least` the closest they have come, as a share of
    /// `start_dist`, and `on` where on the body they began, if they both did.
    Squeezing { start_dist: f32, since: f32, least: f32, on: Option<V3> },
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

impl Game {
    /// Whether the pointer has gone far enough from `start` to stop being a tap.
    fn past_slop(&self, start: (f32, f32), x: f32, y: f32) -> bool {
        dist(start, (x, y)) / self.h > TAP_SLOP
    }

    pub(super) fn end_gesture(&mut self) {
        match std::mem::replace(&mut self.gesture, Gesture::Idle) {
            Gesture::Pending { hit, .. } => self.poke(hit),
            Gesture::Warming { .. } => self.world.warming = false,
            Gesture::Stretching { handle, since, reach, hit, pressed, .. } => {
                let stage = self.stage();
                let dir = self.world.handles.iter().find(|h| h.id == handle).map(|h| h.dir);
                self.world.release_handle(handle);
                let secs = self.clock - since;
                let short = (bloom::PINCH_MIN..bloom::PINCH_MAX).contains(&reach);
                // A short tug let go of at once is a pinch, however fast the hand flew off it.
                let quick = secs < bloom::PINCH_MAX_SECS && short && self.world.ordinary();
                let thrown = !quick && self.throw();
                match dir {
                    // Too little and too quick to be a pull at all: a tap the finger slid on.
                    _ if reach < bloom::PINCH_MIN && self.clock - pressed < SLID_TAP_SECS && !thrown => self.poke(hit),
                    Some(dir) if quick => {
                        if self.world.forms.bloom.pinch(dir) {
                            self.world.add_ripple(dir, 0.35);
                        }
                    }
                    Some(dir) if !thrown && self.world.ordinary() => {
                        // Let go of a little too late, a bud nearly rises there, and sinks back.
                        if short && secs < bloom::NEAR_MAX_SECS && self.world.forms.bloom.almost(dir) {
                            self.sounds.push(Sound::Almost);
                        }
                        self.contact.reached(dir, reach, stage);
                    }
                    _ => {}
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
            Gesture::Squeezing { since, least, on, .. } => {
                self.world.body.squeeze = 0.0;
                // Two fingers on it, pinched together and let go of at once: a pinch, between them.
                let pinched = on.filter(|_| bloom::Bloom::fingers_pinched(self.clock - since, least));
                if let (Some(dir), true) = (pinched, self.world.ordinary()) {
                    if self.world.forms.bloom.pinch(dir) {
                        self.world.add_ripple(dir, 0.35);
                    }
                } else if pinched.is_some() && self.world.forms.pinch_flower(self.world.body.pos) {
                    self.world.add_ripple(V3::UP, 0.5);
                }
            }
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
                let secs = self.clock - since;
                if secs < bloom::PINCH_MAX_SECS && self.world.forms.pinch_flower(self.world.body.pos) {
                    self.world.add_ripple(V3::UP, 0.5);
                } else if secs < bloom::NEAR_MAX_SECS && self.world.forms.bloom.flinch() {
                    // A little too slow: its petals only flinch in.
                    self.sounds.push(Sound::Almost);
                }
            }
            Gesture::Dust { mound: true, moved: false, .. } => {
                if let Some(grown) = self.world.forms.dust.tap() {
                    self.sounds.push(Sound::Poke { pitch: grown, size: 1.6 - grown });
                }
            }
            Gesture::Dust { .. } => {}
            Gesture::Ground { .. } | Gesture::Cutting { .. } | Gesture::River | Gesture::Idle => {}
        }
        self.world.forms.dust.press = 0.0;
    }

    /// A tap on the liquid, or on whatever it has become.
    fn poke(&mut self, hit: Hit) {
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

    /// A tap on a body that has become something else. Returns true if that form answered it.
    pub(super) fn tap_form(&mut self, hit: Hit) -> bool {
        let pos = self.world.body.pos;
        let f = &mut self.world.forms;
        f.filament.pluck(hit.pos, pos).is_some() || f.bloom.fold() || f.pandora.knock() || !(f.filament.whole() && f.bloom.whole() && f.pandora.whole())
    }

    /// Stand up the column of the river under the pointer. Returns true if the river is there.
    pub(super) fn raise_column(&mut self, x: f32, y: f32, tapped: bool) -> bool {
        let Some(at) = self.floor_point(x, y) else { return false };
        let river = &mut self.world.burst.river;
        let Some(slot) = river.slot_at(at.x, at.z) else { return false };
        if river.raise(slot, tapped) {
            self.sounds.push(Sound::Column { slot });
        }
        true
    }

    pub(super) fn pinch_dist(&self) -> f32 {
        let (a, b) = (self.pointers[0], self.pointers[1]);
        dist((a.1, a.2), (b.1, b.2)).max(1.0)
    }

    /// Where on the body two fingers are, if both are on it: the place between them.
    fn pinched_on(&self) -> Option<V3> {
        let on = |p: &(i32, f32, f32)| matches!(self.pick(p.1, p.2), Some(Hit { target: Target::Body, .. }));
        let (a, b) = (self.pointers[0], self.pointers[1]);
        if !(on(&a) && on(&b)) {
            return None;
        }
        match self.pick((a.1 + b.1) * 0.5, (a.2 + b.2) * 0.5) {
            Some(Hit { target: Target::Body, pos, .. }) => Some(pos - self.world.body.pos),
            _ => None,
        }
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
                let on = self.pinched_on();
                self.gesture = Gesture::Squeezing { start_dist: self.pinch_dist(), since: self.clock, least: 1.0, on };
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
                    mound: self.world.forms.dust.on_mound(at.x, at.z) || {
                        let (ro, rd) = self.cam.ray(x, y, self.w, self.h);
                        self.world.forms.dust.touches_growth(ro, rd)
                    },
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
                    let pressed = self.clock - (self.world.time - since);
                    self.gesture = match stretch {
                        Some(handle) => Gesture::Stretching { handle, plane: hit.pos, since: self.clock, reach: 0.0, taut: 0.0, angle: None, hit, pressed },
                        None if flower => Gesture::Tug { start, since: pressed },
                        None => Gesture::Orbiting { start, last: (x, y) },
                    };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Stretching { handle, plane, since, reach, taut, angle, hit, pressed } => {
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
                self.gesture = Gesture::Stretching { handle, plane, since, reach, taut, angle: Some(now), hit, pressed };
            }
            Gesture::Dragging { drop, plane, offset } => {
                let to = self.plane_point(x, y, plane) + offset;
                self.world.grab_drop(drop, to);
            }
            Gesture::Other { start, plane, offset, carried } => {
                if carried || self.past_slop(start, x, y) {
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
                let moved = moved || self.past_slop(start, x, y);
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
                    let fast = dt > 0.0 && moved / dt > crate::burst::kin::CUT_SPEED && !self.world.burst.kin.groups().is_empty();
                    self.gesture = if fast { Gesture::Cutting { last: start } } else { Gesture::Orbiting { start, last: start } };
                    self.pointer_move(id, x, y, dt);
                }
            }
            Gesture::Cutting { last } => {
                self.cut_kin(last, (x, y));
                self.gesture = Gesture::Cutting { last: (x, y) };
            }
            Gesture::Shard { i, start, since, carried } => {
                if carried || self.past_slop(start, x, y) {
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
                let moved = self.past_slop(start, x, y);
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
            Gesture::Squeezing { start_dist, since, least, on } if self.pointers.len() >= 2 => {
                let now = self.pinch_dist();
                self.gesture = Gesture::Squeezing { start_dist, since, least: least.min(now / start_dist), on };
                let s = crate::input::pad::fingers(start_dist, now, self.h);
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
    pub(super) fn track_swipe(&mut self, from: (f32, f32), to: (f32, f32)) {
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
    pub(super) fn cut_kin(&mut self, from: (f32, f32), to: (f32, f32)) {
        let bu = &self.world.burst;
        if bu.phase != burst::Phase::Landscape || bu.river.active {
            return;
        }
        let at: Vec<(f32, f32)> = (0..burst::SHARDS).map(|i| self.screen(bu.shard_pose(i).0 + V3::UP * (bu.shard_size(i) * 0.3))).collect();
        let cuts: Vec<(usize, usize)> = (0..burst::SHARDS)
            .flat_map(|a| (a + 1..burst::SHARDS).map(move |b| (a, b)))
            .filter(|&(a, b)| bu.kin.bonded(a, b) && crate::burst::kin::crosses(from, to, at[a], at[b]))
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
    pub(super) fn slice(&mut self, dx: f32, dy: f32) {
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
    pub(super) fn throw(&mut self) -> bool {
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

    /// A pointer moving with nothing pressed: what it would be looking at.
    pub(super) fn hover(&mut self, x: f32, y: f32, dt: f32) {
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

    pub fn wheel(&mut self, dy: f32) {
        self.wheel_squeeze = (self.wheel_squeeze - dy * 0.0025).clamp(-1.0, 1.0);
        if self.wheel_squeeze.abs() > 0.3 {
            self.discover(Discovery::Squeeze);
        }
    }
}
