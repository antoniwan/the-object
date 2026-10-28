//! Late, once it has reached back (contact.rs) or shown nearly everything it can, it is no
//! longer alone in the room. Something else comes down from above and stays: small where the
//! first is large, hard where it is soft, dark where it is pale, many-sided where it is round.
//! It never touches the floor, it turns slowly, and it never ripples. What happens between the
//! two is up to the player.
//!
//! - tap it: it rings, and the soft one shivers on the side that faces it
//! - carry it close: the soft one reaches out toward it
//! - throw it at the soft one: they clash, and neither gives
//! - press them together and keep pressing: the soft one takes it in, and it can be seen in
//!   there, a dark shape in the milk
//! - squeeze while it is inside: it is given back, and neither keeps the color it had
//! - hurt the soft one again and again while it is here: it comes between you
//! - leave them both alone: they circle, and keep their distance
//!
//! When the soft one becomes anything else (cells, a flower, the floor, twelve pieces), the
//! other lets go of whatever it was doing, rises out of the room, and comes down again once
//! the soft one is itself. Its shape is the last thing joined in the shader's `scene()`, and
//! never smoothly: the two never melt into each other. Its material is `shadeOther()`.

use crate::fusion::FLOOR_Y;
use crate::game::{Game, Sound};
use crate::math::*;
use crate::progress::Discovery;
use crate::sim::World;

/// Discoveries enough to bring it even without contact.
pub const ARRIVE_FOUND: usize = 50;
/// It comes no sooner than this into a visit, and only after nothing has been touched for a while.
const ARRIVE_AFTER: f32 = 20.0;
const ARRIVE_STILL: f32 = 6.0;
/// How long it takes to come down.
const ARRIVE_SECS: f32 = 4.0;
/// Half its width. It is about twice as long as it is wide.
pub const SIZE: f32 = 0.4;
/// How close to the soft one's surface its middle can come, in sizes.
const BOUND: f32 = 1.45;
/// How far from its middle a touch still finds it.
const PICK_RADIUS: f32 = 0.6;
/// Faster than this into the soft one, and it is a clash.
const CLASH_SPEED: f32 = 3.0;
/// Pressed into the soft one this long, it is taken in.
const PRESS_SECS: f32 = 1.1;
const TAKE_SECS: f32 = 2.6;
const GIVE_SECS: f32 = 2.2;
/// Squeezed this long while it is inside, it is given back.
const SQUEEZE_SECS: f32 = 0.5;
/// Closer than this to the soft one's surface, the soft one reaches for it.
const NEAR: f32 = 0.9;
/// Hurts this close together bring it between.
const HURTS: usize = 3;
const HURT_WINDOW: f32 = 4.0;
const GUARD_SECS: f32 = 7.0;
/// Nobody doing anything for this long, and they circle.
const CIRCLE_AFTER: f32 = 10.0;
const CIRCLE_SPEED: f32 = 0.3;
/// While it is inside, the soft one cools toward this.
pub const COLD: V3 = v3(0.62, 0.64, 0.96);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Absent,
    Arriving { t: f32 },
    Here,
    /// Being taken into the soft one.
    Taking { t: f32 },
    Inside,
    /// Being given back; `asked` if a squeeze did it, not the soft one changing.
    Giving { t: f32, asked: bool },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Arrived,
    Rang,
    Reached,
    Clashed,
    TookIn,
    GaveBack,
    Between,
    Circled,
}

/// What it needs to know from outside the world each frame.
pub struct Look {
    /// Whether it is time for it to come at all.
    pub due: bool,
    /// The soft one is busy with a turn of its own; the two do not circle then.
    pub busy: bool,
    /// Seconds into the visit, and since the pointer last did anything.
    pub visit: f32,
    pub still: f32,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    /// Width over height of the view, and half the view's height where the soft one is.
    pub aspect: f32,
    pub view: f32,
}

pub struct Other {
    pub phase: Phase,
    pub pos: V3,
    pub vel: V3,
    /// How far it has turned about its long axis, and how far that axis leans.
    pub yaw: f32,
    pub tilt: f32,
    spin: f32,
    /// 0..1: how sharp it has become; it lengthens when it comes between.
    pub edge: f32,
    /// 0..1: ringing from a blow.
    pub ring: f32,
    /// The color it kept from inside the soft one, and how much fire is in its edges.
    pub kept: V3,
    pub fire: f32,
    /// Where the player is holding it, if they are.
    carried: Option<V3>,
    pressed: f32,
    squeezed: f32,
    hurts: Vec<f32>,
    guard: f32,
    /// Where it is around the soft one, and how far it has circled with nobody there.
    orbit: f32,
    circled: f32,
    /// The soft one's tendril toward it, and how long it has been reaching.
    reach: Option<(u32, f32)>,
    reached: bool,
    /// Which way it is given back.
    out: V3,
    time: f32,
    events: Vec<Event>,
}

impl Default for Other {
    fn default() -> Self {
        Other {
            phase: Phase::Absent,
            pos: v3(0.0, 9.0, 0.0),
            vel: V3::ZERO,
            yaw: 0.0,
            tilt: 0.2,
            spin: 0.35,
            edge: 0.0,
            ring: 0.0,
            kept: V3::ZERO,
            fire: 0.35,
            carried: None,
            pressed: 0.0,
            squeezed: 0.0,
            hurts: Vec::new(),
            guard: 0.0,
            orbit: 0.0,
            circled: 0.0,
            reach: None,
            reached: false,
            out: v3(1.0, 0.0, 0.0),
            time: 0.0,
            events: Vec::new(),
        }
    }
}

impl Other {
    /// In the room and free to be touched.
    pub fn here(&self) -> bool {
        self.phase == Phase::Here
    }

    /// 0..1: how far it is inside the soft one.
    pub fn inside(&self) -> f32 {
        match self.phase {
            Phase::Taking { t } => smoothstep(0.0, TAKE_SECS, t),
            Phase::Inside => 1.0,
            Phase::Giving { t, .. } => 1.0 - smoothstep(0.0, GIVE_SECS, t),
            _ => 0.0,
        }
    }

    /// Its size for drawing; 0 while it is not in the room. Inside the soft one it is smaller.
    pub fn size(&self, body: V3) -> f32 {
        if self.phase == Phase::Absent || self.pos.y - body.y > 6.5 {
            return 0.0;
        }
        SIZE * (1.0 - 0.3 * self.inside())
    }

    /// Where a ray from the eye meets it, if it does and it can be touched.
    pub fn pick(&self, ro: V3, rd: V3) -> Option<f32> {
        if !self.here() || self.pos.y > 5.0 {
            return None;
        }
        let oc = ro - self.pos;
        let b = oc.dot(rd);
        let c = oc.dot(oc) - PICK_RADIUS * PICK_RADIUS;
        let h = b * b - c;
        (h >= 0.0 && -b - h.sqrt() > 0.0).then(|| -b - h.sqrt())
    }

    /// The player holds it toward `to`.
    pub fn carry(&mut self, to: V3) {
        if self.here() {
            self.carried = Some(to);
        }
    }

    /// The player lets go, with the pointer's velocity in the world.
    pub fn release(&mut self, fling: V3) {
        if self.carried.take().is_some() && fling.len() > self.vel.len() {
            self.vel = fling;
        }
    }

    /// The soft one was hurt.
    pub fn hurt(&mut self) {
        self.hurts.push(self.time);
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Where it would be, left to itself: beside the soft one, or above it where the view is narrow.
    fn rest(&self, c: V3, look: &Look) -> V3 {
        let side = (look.view * look.aspect * 0.72).clamp(0.9, 2.2);
        let lift = ((2.0 - side) * 1.2).clamp(0.3, 1.5);
        let (s, k) = self.orbit.sin_cos();
        c + look.right * (k * side) + look.fwd * (s * 1.6) + look.up * (lift + 0.12 * (self.time * 0.7).sin())
    }
}

/// A tap on it: it rings, and the soft one shivers on the side that faces it.
pub fn strike(w: &mut World) {
    let o = &mut w.other;
    if !o.here() {
        return;
    }
    o.ring = 1.0;
    o.spin += 4.0;
    o.events.push(Event::Rang);
    let dir = (o.pos - w.body.pos).norm();
    w.add_ripple(dir, 0.45);
    w.body.squash_vel += 1.2;
}

pub fn step(w: &mut World, dt: f32, look: &Look) {
    let mut o = std::mem::take(&mut w.other);
    o.time += dt;
    let plain = w.ordinary();
    let c = w.body.pos;
    let r = w.body.radius();

    if o.phase == Phase::Absent {
        if !(look.due && plain && look.visit > ARRIVE_AFTER && look.still > ARRIVE_STILL) {
            w.other = o;
            return;
        }
        o.phase = Phase::Arriving { t: 0.0 };
        o.pos = o.rest(c, look) + V3::UP * 7.0;
        o.vel = V3::ZERO;
        o.spin = 2.5;
    }

    // The soft one became something else: let go of everything and rise out of the room.
    if !plain {
        o.carried = None;
        o.pressed = 0.0;
        match o.phase {
            Phase::Taking { t } => o.phase = Phase::Giving { t: GIVE_SECS * (1.0 - t / TAKE_SECS), asked: false },
            Phase::Inside => o.phase = Phase::Giving { t: 0.0, asked: false },
            _ => {}
        }
        if let Phase::Giving { asked, .. } = &mut o.phase {
            *asked = false;
        }
    }

    // Hurt again and again, the soft one gets someone between it and you.
    o.hurts.retain(|t| o.time - t < HURT_WINDOW);
    if o.hurts.len() >= HURTS && o.here() && plain && o.guard <= 0.0 {
        o.guard = GUARD_SECS;
        o.ring = o.ring.max(0.6);
        o.events.push(Event::Between);
        o.hurts.clear();
    }
    o.guard -= dt;
    let sharp = if o.guard > 0.0 { 1.0 } else { 0.0 };
    o.edge += (sharp - o.edge) * (1.0 - (-2.0 * dt).exp());

    // Left alone together, they circle, and the soft one watches it instead of you.
    let circling = o.here() && plain && o.guard <= 0.0 && o.carried.is_none() && !look.busy && look.still > CIRCLE_AFTER;
    if circling {
        o.orbit += CIRCLE_SPEED * dt;
        o.circled += CIRCLE_SPEED * dt;
        w.gaze_dir = (o.pos - c).norm();
        if o.circled >= std::f32::consts::TAU {
            o.circled = 0.0;
            o.events.push(Event::Circled);
        }
    } else {
        o.circled = 0.0;
    }

    let goal = if !plain {
        c + V3::UP * 8.0
    } else if o.guard > 0.0 {
        c - look.fwd * (r + 0.85) + look.up * 0.1
    } else {
        o.rest(c, look)
    };

    match o.phase {
        Phase::Arriving { .. } | Phase::Here => {
            let (target, k, damp) = match o.carried {
                Some(to) => (to, 60.0, 12.0),
                None => (goal, 6.0, 3.6),
            };
            o.vel += ((target - o.pos) * k - o.vel * damp) * dt;
            o.pos += o.vel * dt;
            // It never touches the floor.
            let floor = FLOOR_Y + 0.75;
            if o.pos.y < floor {
                o.pos.y = floor;
                o.vel.y = o.vel.y.max(0.0);
            }
            if plain {
                collide(&mut o, w, c, r, dt);
            }
        }
        Phase::Taking { .. } | Phase::Inside => {
            o.pos = o.pos.lerp(c, 1.0 - (-2.5 * dt).exp());
            o.vel = V3::ZERO;
            // It takes on some of the cold of what it holds.
            w.body.color = w.body.color.lerp(COLD, 1.0 - (-0.25 * o.inside() * dt).exp());
        }
        Phase::Giving { t, .. } => {
            let gone = smoothstep(0.0, GIVE_SECS, t);
            o.pos = c + o.out * (gone * (r + SIZE * BOUND + 0.1));
            o.vel = o.out * 1.2;
        }
        Phase::Absent => {}
    }

    // Turning: slowly on its own, faster when struck or sharpened.
    let base = 0.35 + 1.4 * o.edge;
    o.spin += (base - o.spin) * (1.0 - (-0.8 * dt).exp());
    o.yaw += o.spin * dt;
    o.tilt = 0.25 + 0.15 * (o.time * 0.4).sin() - 0.2 * o.edge;
    o.ring *= (-1.2 * dt).exp();

    // Squeezed while it is inside, the soft one gives it back.
    if o.phase == Phase::Inside && w.body.squeeze.abs() > 0.45 {
        o.squeezed += dt;
    } else {
        o.squeezed = 0.0;
    }
    if o.squeezed > SQUEEZE_SECS {
        o.squeezed = 0.0;
        o.out = look.right;
        o.phase = Phase::Giving { t: 0.0, asked: true };
        w.add_ripple(look.right, 0.9);
    }

    o.phase = match o.phase {
        Phase::Arriving { t } if t + dt >= ARRIVE_SECS => {
            o.events.push(Event::Arrived);
            Phase::Here
        }
        Phase::Arriving { t } => Phase::Arriving { t: t + dt },
        Phase::Taking { t } if t + dt >= TAKE_SECS => {
            o.events.push(Event::TookIn);
            w.add_ripple(V3::UP, 0.8);
            w.add_ripple(-V3::UP, 0.8);
            Phase::Inside
        }
        Phase::Taking { t } => Phase::Taking { t: t + dt },
        Phase::Giving { t, asked } if t + dt >= GIVE_SECS => {
            // Each keeps something of the other: it the soft one's color, the soft one its cold.
            o.kept = o.kept.lerp(w.body.color, 0.6);
            o.fire = (o.fire + 0.3).min(1.0);
            o.ring = 1.0;
            if asked {
                o.events.push(Event::GaveBack);
            }
            Phase::Here
        }
        Phase::Giving { t, asked } => Phase::Giving { t: t + dt, asked },
        p => p,
    };

    reach_for(&mut o, w, c, r, plain, dt);
    w.other = o;
}

/// Hard against the soft one: it never passes through on its own. Thrown in fast, it is a clash;
/// held pressed in long enough, it is taken in.
fn collide(o: &mut Other, w: &mut World, c: V3, r: f32, dt: f32) {
    let min = r + SIZE * BOUND;
    let pressing = o.carried.is_some_and(|to| (to - c).len() < r + SIZE * 0.6);
    o.pressed = if pressing { o.pressed + dt } else { 0.0 };
    if o.pressed > PRESS_SECS && o.phase == Phase::Here {
        o.carried = None;
        o.pressed = 0.0;
        o.phase = Phase::Taking { t: 0.0 };
        w.add_ripple((o.pos - c).norm(), 0.9);
        return;
    }
    let d = o.pos - c;
    if d.len() >= min {
        return;
    }
    let n = d.norm();
    let closing = -o.vel.dot(n);
    o.pos = c + n * min;
    if closing <= 0.0 {
        return;
    }
    o.vel += n * (closing * 1.6);
    if closing > CLASH_SPEED && o.carried.is_none() && o.phase == Phase::Here {
        w.add_ripple(n, 1.0);
        w.add_ripple(-n, 0.5);
        w.body.vel -= n * (closing * 0.6);
        w.body.squash_vel -= 3.0;
        w.agitation = w.agitation.max(0.7);
        o.ring = 1.0;
        o.spin += 6.0;
        o.events.push(Event::Clashed);
    } else if closing > 0.4 {
        w.add_ripple(n, (0.15 * closing).min(0.4));
    }
}

/// Carried near enough, the soft one puts out a tendril toward it, stopping just short.
fn reach_for(o: &mut Other, w: &mut World, c: V3, r: f32, plain: bool, dt: f32) {
    let d = o.pos - c;
    let gap = d.len() - r - SIZE * BOUND;
    let dir = d.norm();
    let carried = plain && o.here() && o.carried.is_some();
    let want = carried && gap < NEAR;
    if let Some((id, secs)) = o.reach {
        let anchor = w.handles.iter().find(|h| h.id == id).map(|h| h.dir);
        let keep = carried && gap < NEAR + 0.4 && anchor.is_some_and(|a| a.dot(dir) > 0.6);
        if !keep {
            w.release_handle(id);
            o.reach = None;
        } else {
            let secs = secs + dt;
            w.pull_handle(id, c + dir * (r + (gap + 0.15).clamp(0.1, r)));
            if secs > 1.0 && !o.reached {
                o.reached = true;
                o.events.push(Event::Reached);
            }
            o.reach = Some((id, secs));
        }
    } else if want {
        o.reach = w.begin_stretch(c + dir * r).map(|id| (id, 0.0));
    }
}

impl Game {
    /// What happened between the two this frame, as discoveries and sounds.
    pub(crate) fn other_news(&mut self) {
        for e in self.world.other.drain_events() {
            match e {
                Event::Arrived => {
                    self.sounds.push(Sound::Become);
                    self.sounds.push(Sound::Ring { pitch: 0.0 });
                    self.discover(Discovery::Another);
                }
                Event::Rang => {
                    self.sounds.push(Sound::Ring { pitch: 0.75 });
                    self.discover(Discovery::Ring);
                }
                Event::Reached => self.discover(Discovery::Near),
                Event::Clashed => {
                    self.sounds.push(Sound::Ring { pitch: 1.0 });
                    self.sounds.push(Sound::Provoked);
                    self.cam.shake = self.cam.shake.max(0.05);
                    self.discover(Discovery::Clash);
                }
                Event::TookIn => {
                    self.sounds.push(Sound::Become);
                    self.discover(Discovery::Inside);
                }
                Event::GaveBack => {
                    self.sounds.push(Sound::Merge);
                    self.sounds.push(Sound::Ring { pitch: 0.4 });
                    self.discover(Discovery::Give);
                }
                Event::Between => {
                    self.sounds.push(Sound::Ring { pitch: 0.2 });
                    self.discover(Discovery::Between);
                }
                Event::Circled => self.discover(Discovery::Distance),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::FOCAL;

    fn game() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.contact.contacted = true;
        g
    }

    fn wait(g: &mut Game, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    fn project(g: &Game, p: V3) -> (f32, f32) {
        let b = g.cam.basis();
        let d = p - b.pos;
        let z = d.dot(b.fwd);
        let (ux, uy) = (d.dot(b.right) * FOCAL / z, d.dot(b.up) * FOCAL / z);
        ((ux * g.h + g.w) * 0.5, (g.h - uy * g.h) * 0.5)
    }

    fn arrived() -> Game {
        let mut g = game();
        wait(&mut g, 32.0);
        assert!(g.world.other.here(), "{:?}", g.world.other.phase);
        assert!(g.progress.has(Discovery::Another));
        g
    }

    #[test]
    fn it_does_not_come_before_its_time() {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        wait(&mut g, 60.0);
        assert_eq!(g.world.other.phase, Phase::Absent);
        assert_eq!(g.world.other.size(g.world.body.pos), 0.0);
    }

    #[test]
    fn after_contact_it_comes_down_and_rings_when_tapped() {
        let mut g = arrived();
        assert!(g.world.other.pos.y < 3.0);
        let (x, y) = project(&g, g.world.other.pos);
        g.pointer_down(1, x, y);
        g.pointer_up(1);
        g.update(1.0 / 60.0);
        assert!(g.progress.has(Discovery::Ring));
        assert!(!g.progress.has(Discovery::Poke), "the tap was on it, not on the soft one");
        assert!(g.sounds.contains(&Sound::Ring { pitch: 0.75 }));
    }

    #[test]
    fn carried_close_the_soft_one_reaches_and_pressed_in_it_is_taken_then_squeezed_out() {
        let mut g = arrived();
        let (x, y) = project(&g, g.world.other.pos);
        g.pointer_down(1, x, y);
        // bring it to just beside the soft one
        let beside = g.world.body.pos + g.cam.basis().right * 1.9;
        let (bx, by) = project(&g, beside);
        for i in 1..=30 {
            let f = i as f32 / 30.0;
            g.pointer_move(1, x + (bx - x) * f, y + (by - y) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        wait(&mut g, 1.6);
        assert!(g.progress.has(Discovery::Near));
        // then into it, and hold
        let (cx, cy) = project(&g, g.world.body.pos);
        for i in 1..=30 {
            let f = i as f32 / 30.0;
            g.pointer_move(1, bx + (cx - bx) * f, by + (cy - by) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        wait(&mut g, 1.5);
        g.pointer_up(1);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Inside));
        assert_eq!(g.world.other.phase, Phase::Inside);
        assert!(g.world.ordinary(), "the soft one is still itself");
        let cold = g.world.body.color;
        assert!(cold.z > cold.x + 0.05, "it has cooled: {cold:?}");
        for _ in 0..40 {
            g.wheel(-60.0);
            g.update(1.0 / 60.0);
        }
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Give));
        assert!(g.world.other.here());
        assert!(g.world.other.kept.len() > 0.3);
    }

    #[test]
    fn thrown_hard_at_the_soft_one_they_clash() {
        let mut g = arrived();
        let o = &mut g.world.other;
        o.vel = (g.world.body.pos - o.pos).norm() * 9.0;
        wait(&mut g, 1.0);
        assert!(g.progress.has(Discovery::Clash));
        assert!(g.world.other.here());
        let gap = (g.world.other.pos - g.world.body.pos).len();
        assert!(gap > g.world.body.radius(), "it did not pass through");
    }

    #[test]
    fn hurt_it_and_the_other_comes_between() {
        let mut g = arrived();
        let (cx, cy) = project(&g, g.world.body.pos);
        for _ in 0..3 {
            g.pointer_down(1, cx, cy);
            g.pointer_up(1);
            wait(&mut g, 0.3);
        }
        assert!(g.progress.has(Discovery::Between));
        wait(&mut g, 3.0);
        let toward_you = (g.world.other.pos - g.world.body.pos).dot(-g.cam.basis().fwd);
        assert!(toward_you > 1.0, "{toward_you}");
    }

    #[test]
    fn left_alone_together_they_circle() {
        let mut g = arrived();
        wait(&mut g, 40.0);
        assert!(g.progress.has(Discovery::Distance));
    }

    #[test]
    fn when_the_soft_one_becomes_something_else_it_gives_back_and_leaves() {
        let mut g = arrived();
        g.world.other.phase = Phase::Inside;
        g.world.forms.pandora.phase = crate::forms::pandora::Phase::Box;
        wait(&mut g, 6.0);
        assert!(g.world.other.here());
        assert!(!g.progress.has(Discovery::Give), "it was not asked");
        assert!(g.world.other.pos.y > g.world.body.pos.y + 4.0);
        assert_eq!(g.world.other.pick(g.cam.basis().pos, g.cam.basis().fwd), None);
    }
}
