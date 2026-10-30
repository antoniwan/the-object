//! An old hurt (remembrance.rs). Hurt it enough, over enough visits, and it scars: a thin seam
//! where it is struck most often, kept between visits. Time alone does not take a scar away.
//!
//! - hurt it again while it is scarred: the seam opens into a wound, and four small spiked
//!   pieces of it fall out onto the floor around it, each slowly turning through every color
//! - carry one into the wound while it is green: it goes in, and the wound fills a third of
//!   the way with what it brought, dripping toward green
//! - carry one in at any other color, or anywhere else: it is spat back out, hard, falls,
//!   bounces on the floor, and goes on turning
//! - fill it: the wound closes over, and the scar is fainter than it was. Never gone.
//!
//! The pieces are drops (sim/) that it holds in place; a hand can still carry them. The seam
//! and the wound are drawn on the body's own surface (`surfaceDisp`, `sceneColor` in the
//! shader); the pieces' spikes are drawn in the drops' loop, so nothing adds a call to `scene()`.

use crate::floor::fusion::FLOOR_Y;
use crate::game::Game;
use crate::math::*;
use crate::progress::Discovery;
use crate::mind::remembrance::Deed;
use crate::sim::World;

/// How many pieces fall out of an open wound.
const CALTROPS: usize = 4;
/// How many green ones fill it.
const TO_HEAL: u32 = 3;
const RADIUS: f32 = 0.13;
/// Seconds for a piece to turn through every color once.
const CYCLE_SECS: f32 = 10.0;
/// Hue where it is green, and how far either side still counts, as fractions of the wheel.
pub const GREEN: f32 = 1.0 / 3.0;
const GREEN_BY: f32 = 0.07;
/// How far from the middle of the body, across the floor, the pieces lie.
const RING: f32 = 2.2;
/// Let go of, a piece is free this long before it is held in its place again.
const FREE_SECS: f32 = 2.5;
/// How near the wound (a dot of directions) a piece has to go in to be taken.
const NEAR_WOUND: f32 = 0.55;
/// Seconds to open, and to close over.
const OPEN_SECS: f32 = 1.2;
const CLOSE_SECS: f32 = 3.0;
/// Once closed, it does not open again for this long.
const REST_SECS: f32 = 20.0;
/// How hard a piece of the wrong color is spat back out, how hard it falls, and for how long
/// before it goes back to its place.
const SPIT: f32 = 4.5;
const FALL: f32 = 16.0;
const FLUNG_SECS: f32 = 2.2;
/// What a healed scar fades to.
const FAINT: f32 = 0.35;
/// Inside, before anything has gone in.
const RAW: V3 = v3(0.42, 0.02, 0.05);

/// A color on the wheel: 0 red, a third green, two thirds blue. Soft, so it sits in the room.
pub fn hue(t: f32) -> V3 {
    let t = t.rem_euclid(1.0) * 6.0;
    let c = |k: f32| {
        let x = ((t + k) % 6.0 - 3.0).abs() - 1.0;
        x.clamp(0.0, 1.0)
    };
    let rgb = v3(c(0.0), c(4.0), c(2.0));
    rgb * 0.75 + v3(0.2, 0.2, 0.2)
}

fn is_green(h: f32) -> bool {
    let d = (h.rem_euclid(1.0) - GREEN).abs();
    d <= GREEN_BY
}

#[derive(Clone, Copy, Debug)]
struct Caltrop {
    id: u32,
    rest: V3,
    /// Where it is on the wheel at time zero.
    phase: f32,
    /// Seconds left of being free after it was let go; 0 while it is held in place.
    free: f32,
    carried: bool,
    /// Seconds left of being spat out and falling.
    flung: f32,
    last_pos: V3,
    last_hue: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Closed { rest: f32 },
    Opening { t: f32 },
    Open,
    Closing { t: f32 },
}

pub struct Wound {
    /// Where the seam is, from the body's middle, and how plain it is 0..1 (0: no scar).
    pub dir: V3,
    pub scar: f32,
    pub phase: Phase,
    /// 0..1 how full the wound is, and the color it is filling with.
    pub fill: f32,
    pub color: V3,
    /// A drip going in, 1 fading to 0.
    pub drip: f32,
    /// 0..1, how much of it shows: it is on the plain liquid only, and fades while it is anything else.
    pub shown: f32,
    caltrops: Vec<Caltrop>,
    /// Pieces of the wrong color taken in, waiting for room to be spat back out.
    taken: Vec<Caltrop>,
    time: f32,
}

impl Default for Wound {
    fn default() -> Self {
        Wound { dir: V3::UP, scar: 0.0, phase: Phase::Closed { rest: 0.0 }, fill: 0.0, color: RAW, drip: 0.0, shown: 1.0, caltrops: Vec::new(), taken: Vec::new(), time: 0.0 }
    }
}

impl Wound {
    /// 0..1, how open it is.
    pub fn open(&self) -> f32 {
        let k = match self.phase {
            Phase::Closed { .. } => 0.0,
            Phase::Opening { t } => t / OPEN_SECS,
            Phase::Open => 1.0,
            Phase::Closing { t } => 1.0 - t / CLOSE_SECS,
        };
        smooth01(k)
    }

    pub fn is_open(&self) -> bool {
        !matches!(self.phase, Phase::Closed { .. })
    }

    /// Which drops are its pieces.
    pub fn is_caltrop(&self, id: u32) -> bool {
        self.caltrops.iter().any(|c| c.id == id)
    }

    /// Opens, if it is scarred and has rested since it last closed.
    pub fn tear_open(&mut self, w: &mut World) -> bool {
        if self.scar <= 0.0 || !matches!(self.phase, Phase::Closed { rest } if rest <= 0.0) {
            return false;
        }
        let out = v3(self.dir.x, 0.0, self.dir.z).norm();
        let out = if out.len() < 0.5 { v3(1.0, 0.0, 0.0) } else { out };
        // Pieces of the last one may still be drifting home.
        let before = self.caltrops.len();
        for k in 0..CALTROPS {
            self.spill(w, out, k as f32 / CALTROPS as f32);
        }
        if self.caltrops.len() - before < TO_HEAL as usize {
            // Not room enough for it to be filled: what got out is let go of, and it stays shut.
            for c in self.caltrops.drain(before..) {
                if let Some(d) = w.drop_mut(c.id) {
                    d.target = None;
                }
            }
            return false;
        }
        self.phase = Phase::Opening { t: 0.0 };
        self.scar = 1.0;
        self.fill = 0.0;
        self.color = RAW;
        w.add_ripple(self.dir, 0.9);
        true
    }

    /// One piece out of the wound, toward its place on the floor; `phase` its place on the wheel.
    fn spill(&mut self, w: &mut World, out: V3, phase: f32) {
        // Spread around the side the wound faces.
        let a = out.z.atan2(out.x) + (phase - 0.375) * 2.4;
        let rest = v3(w.body.pos.x + a.cos() * RING, FLOOR_Y + RADIUS, w.body.pos.z + a.sin() * RING);
        let from = w.body.pos + self.dir * (w.body.radius() * 1.25);
        let phase = phase - self.time / CYCLE_SECS;
        let color = hue(phase + self.time / CYCLE_SECS);
        if w.spill(from, (rest - from) * 2.0, RADIUS, color) {
            let id = w.drops.last().map_or(0, |d| d.id);
            if let Some(d) = w.drop_mut(id) {
                d.target = Some(rest);
            }
            self.caltrops.push(Caltrop { id, rest, phase, free: 0.0, carried: false, flung: 0.0, last_pos: from, last_hue: phase });
        }
    }
}

/// What came of a piece this frame.
enum Went {
    Healed,
    Back,
}

/// Each frame (from remembrance.rs): the wound opens and closes, the pieces turn and are held,
/// and what goes in is taken or given back.
pub fn step(g: &mut Game, dt: f32) {
    let carried = g.carried_drop();
    let w = &mut g.world;
    let ordinary = w.ordinary();
    let wound = &mut w.wound;
    wound.time += dt;
    wound.drip = (wound.drip - dt * 1.5).max(0.0);
    let goal = if ordinary { 1.0 } else { 0.0 };
    wound.shown = ease(wound.shown, goal, -4.0 * dt);
    wound.phase = match wound.phase {
        Phase::Closed { rest } => Phase::Closed { rest: (rest - dt).max(0.0) },
        Phase::Opening { t } if t + dt >= OPEN_SECS => Phase::Open,
        Phase::Opening { t } => Phase::Opening { t: t + dt },
        Phase::Closing { t } if t + dt >= CLOSE_SECS => {
            wound.fill = 0.0;
            Phase::Closed { rest: REST_SECS }
        }
        Phase::Closing { t } => Phase::Closing { t: t + dt },
        Phase::Open => Phase::Open,
    };

    let mut went = Vec::new();
    let mut landed: Vec<f32> = Vec::new();
    let open_now = matches!(wound.phase, Phase::Opening { .. } | Phase::Open);
    let time = wound.time;
    let body = w.body.pos;
    let reach = w.body.radius() * 1.6;
    let mut i = 0;
    while i < wound.caltrops.len() {
        let mut c = wound.caltrops[i];
        let h = c.phase + time / CYCLE_SECS;
        match w.drops.iter_mut().find(|d| d.id == c.id) {
            Some(d) => {
                d.color = hue(h);
                let held = carried == Some(c.id);
                if !open_now {
                    // Closed over: nothing holds it any more.
                    if c.carried && !held {
                        d.target = None;
                    }
                } else if held {
                    c.free = 0.0;
                    c.flung = 0.0;
                } else if c.flung > 0.0 {
                    // Spat out: it falls hard, and bounces, before it goes back to its place.
                    d.target = None;
                    d.vel.y -= FALL * dt;
                    let floor = FLOOR_Y + RADIUS;
                    if d.pos.y < floor && d.vel.y < 0.0 {
                        d.pos.y = floor;
                        landed.push(-d.vel.y);
                        d.vel = v3(d.vel.x * 0.7, -d.vel.y * 0.45, d.vel.z * 0.7);
                    }
                    c.flung -= dt;
                    if c.flung <= 0.0 {
                        d.target = Some(c.rest);
                    }
                } else if c.carried {
                    // Just let go of: free for a moment, to go where it was sent.
                    c.free = FREE_SECS;
                } else if c.free > 0.0 {
                    c.free -= dt;
                    if c.free <= 0.0 {
                        d.target = Some(c.rest);
                    }
                } else if d.target.is_none() {
                    d.target = Some(c.rest);
                }
                c.carried = held;
                c.last_pos = d.pos;
                c.last_hue = h;
                wound.caltrops[i] = c;
                i += 1;
            }
            None => {
                // It went into something.
                wound.caltrops.remove(i);
                let into_body = (c.last_pos - body).len() < reach;
                let at = (c.last_pos - body).norm();
                if into_body && at.dot(wound.dir) > NEAR_WOUND && is_green(c.last_hue) && wound.phase == Phase::Open {
                    went.push((Went::Healed, c));
                } else {
                    went.push((Went::Back, c));
                }
            }
        }
    }

    for speed in landed {
        if speed > 1.5 {
            g.sounds.push(crate::game::Sound::Poke { pitch: 0.1, size: (speed * 0.08).min(0.6) });
        }
    }
    for (what, c) in went {
        let wound = &mut g.world.wound;
        match what {
            // Filled already, by another one this same frame.
            Went::Healed if wound.phase != Phase::Open => {}
            Went::Healed => {
                wound.fill = (wound.fill + 1.0 / TO_HEAL as f32).min(1.0);
                wound.color = wound.color.lerp(hue(c.last_hue), 0.6).lerp(hue(GREEN), 0.25);
                wound.drip = 1.0;
                let dir = wound.dir;
                g.world.add_ripple(dir, 0.5);
                g.remembrance.remember(Deed::Tended);
                g.discover(Discovery::Tend);
                if g.world.wound.fill >= 0.999 {
                    close(&mut g.world);
                    g.discover(Discovery::Heal);
                }
            }
            Went::Back => wound.taken.push(c),
        }
    }
    // What it has taken in to give back comes out as soon as there is room for it.
    let wound = &mut g.world.wound;
    if !matches!(wound.phase, Phase::Opening { .. } | Phase::Open) {
        wound.taken.clear();
    }
    if wound.taken.is_empty() {
        return;
    }
    let mut w = std::mem::take(&mut g.world.wound);
    for c in std::mem::take(&mut w.taken) {
        if w.spill_again(&mut g.world, c) {
            g.sounds.push(crate::game::Sound::Tear);
            g.cam.shake = g.cam.shake.max(0.03);
        } else {
            w.taken.push(c);
        }
    }
    g.world.wound = w;
}

impl Wound {
    /// The same piece, spat back out of the wound: up and away, hard, and down to the floor.
    /// False when there is no room for it yet.
    fn spill_again(&mut self, w: &mut World, c: Caltrop) -> bool {
        let from = w.body.pos + self.dir * (w.body.radius() * 1.3);
        let out = v3(self.dir.x, 0.0, self.dir.z);
        let out = if out.len() < 0.3 { (c.rest - w.body.pos).norm() } else { out.norm() };
        let h = c.phase + self.time / CYCLE_SECS;
        // Each its own way, a little, so they do not all land in one place.
        let side = V3::UP.cross(out).norm() * ((c.phase.rem_euclid(1.0) - 0.5) * SPIT);
        if !w.spill(from, out * SPIT + side + V3::UP * (SPIT * 0.6), RADIUS, hue(h)) {
            return false;
        }
        let id = w.drops.last().map_or(0, |d| d.id);
        self.caltrops.push(Caltrop { id, free: 0.0, carried: false, flung: FLUNG_SECS, last_pos: from, ..c });
        w.add_ripple(self.dir, 0.7);
        w.body.vel -= out * 0.8;
        w.body.squash_vel -= 1.5;
        true
    }
}

/// Filled: it closes over. The pieces still out are let go of, and drift home into it.
fn close(w: &mut World) {
    let wound = &mut w.wound;
    wound.phase = Phase::Closing { t: 0.0 };
    wound.scar = FAINT;
    let ids: Vec<u32> = wound.caltrops.iter().map(|c| c.id).collect();
    for id in ids {
        if let Some(d) = w.drop_mut(id) {
            d.target = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_comes_round_to_green() {
        assert!(is_green(GREEN));
        assert!(is_green(GREEN + 1.0));
        assert!(!is_green(0.0));
        assert!(!is_green(2.0 / 3.0));
        let g = hue(GREEN);
        assert!(g.y > g.x && g.y > g.z, "{g:?}");
    }

    #[test]
    fn no_scar_no_wound() {
        let mut w = World::new();
        assert!(!w.wound.is_open());
        let mut wound = std::mem::take(&mut w.wound);
        assert!(!wound.tear_open(&mut w));
        assert!(w.drops.is_empty());
    }

    #[test]
    fn a_scar_opens_and_lets_four_out() {
        let mut w = World::new();
        let mut wound = std::mem::take(&mut w.wound);
        wound.scar = 1.0;
        assert!(wound.tear_open(&mut w));
        assert_eq!(w.drops.len(), CALTROPS);
        assert!(w.drops.iter().all(|d| wound.is_caltrop(d.id)));
        // and not again while it is open
        assert!(!wound.tear_open(&mut w));
    }

    fn scarred() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.remembrance = crate::mind::remembrance::Remembrance::load("at=0;hurt=3,0;scar=0,0,1,0.35", 0.0);
        g.update(1.0 / 60.0);
        g
    }

    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0);
            g.sounds.clear();
        }
    }

    fn hurt(g: &mut Game) {
        g.world.agitation = 1.0;
        run(g, 0.1);
        g.world.agitation = 0.0;
        run(g, 0.1);
    }

    /// Send every piece still out into the wound, at `at` on the wheel.
    fn send_in(g: &mut Game, at: f32) -> usize {
        let time = g.world.wound.time;
        let into = g.world.body.pos + g.world.wound.dir * (g.world.body.radius() * 0.8);
        let ids: Vec<u32> = g.world.wound.caltrops.iter().map(|c| c.id).collect();
        for c in &mut g.world.wound.caltrops {
            c.phase = at - time / CYCLE_SECS;
        }
        for &id in &ids {
            if let Some(d) = g.world.drop_mut(id) {
                d.target = Some(into);
                d.age = 10.0;
            }
        }
        ids.len()
    }

    #[test]
    fn a_kept_scar_is_there_next_visit() {
        let g = scarred();
        assert_eq!(g.world.wound.scar, 0.35);
        assert!((g.world.wound.dir - v3(0.0, 0.0, 1.0)).len() < 1e-3);
    }

    #[test]
    fn hurt_again_it_opens_and_the_pieces_lie_still() {
        let mut g = scarred();
        hurt(&mut g);
        assert!(g.progress.has(Discovery::Wound));
        assert_eq!(g.world.wound.scar, 1.0);
        run(&mut g, 4.0);
        assert_eq!(g.world.wound.phase, Phase::Open);
        let n = g.world.drops.iter().filter(|d| g.world.wound.is_caltrop(d.id)).count();
        assert_eq!(n, CALTROPS, "held in place, none drifted home");
        for d in &g.world.drops {
            assert!(d.pos.y < FLOOR_Y + 0.5, "on the floor: {:?}", d.pos);
        }
    }

    #[test]
    fn green_heals_it_and_a_faint_scar_stays() {
        let mut g = scarred();
        hurt(&mut g);
        run(&mut g, 4.0);
        assert!(send_in(&mut g, GREEN) >= TO_HEAL as usize);
        run(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Tend));
        assert!(g.progress.has(Discovery::Heal));
        assert!(matches!(g.world.wound.phase, Phase::Closing { .. } | Phase::Closed { .. }));
        run(&mut g, 4.0);
        assert!(!g.world.wound.is_open());
        assert_eq!(g.world.wound.scar, FAINT);
        assert_eq!(g.remembrance.ledger().get(Deed::Tended).count, TO_HEAL as u64);
    }

    #[test]
    fn any_other_color_comes_back_out() {
        let mut g = scarred();
        hurt(&mut g);
        run(&mut g, 4.0);
        send_in(&mut g, GREEN + 0.4);
        // Spat out: up, past the middle of it, then down onto the floor and back to its place.
        let mut highest = f32::MIN;
        for _ in 0..90 {
            run(&mut g, 1.0 / 60.0);
            for d in &g.world.drops {
                if g.world.wound.is_caltrop(d.id) && g.world.wound.caltrops.iter().any(|c| c.id == d.id && c.flung > 0.0) {
                    highest = highest.max(d.pos.y);
                }
            }
        }
        assert!(highest > g.world.body.pos.y, "{highest}");
        run(&mut g, 4.0);
        for c in &g.world.wound.caltrops {
            let d = g.world.drops.iter().find(|d| d.id == c.id).unwrap();
            assert!((d.pos - c.rest).len() < 0.2, "back in its place: {:?} {:?} {}", d.pos, c.rest, c.flung);
        }
        assert!(!g.progress.has(Discovery::Tend));
        assert_eq!(g.world.wound.fill, 0.0);
        assert_eq!(g.world.wound.caltrops.len(), CALTROPS, "every one came back out");
    }

    #[test]
    fn enough_hurt_leaves_a_scar_where_it_is_struck() {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        for _ in 0..9 {
            hurt(&mut g);
        }
        assert!(g.progress.has(Discovery::Scar));
        assert_eq!(g.world.wound.scar, 1.0);
        assert!(g.remembrance.save().contains("scar="));
    }

    #[test]
    fn the_box_comes_round_while_the_pieces_lie_out() {
        let mut g = scarred();
        hurt(&mut g);
        run(&mut g, 4.0);
        assert!(g.world.ordinary());
        assert!(g.world.forms.pandora.corner(0.0));
        run(&mut g, 3.0);
        for _ in 0..3 {
            g.world.forms.pandora.knock();
        }
        run(&mut g, 60.0);
        assert!(g.world.wound.is_open());
        assert!(g.world.forms.pandora.whole(), "{:?}, {} drops", g.world.forms.pandora.phase, g.world.drops.len());
    }

    #[test]
    fn the_pieces_neither_bring_night_nor_keep_it() {
        let mut g = scarred();
        hurt(&mut g);
        run(&mut g, 4.0);
        let (p, c) = (g.world.body.pos, g.world.body.color);
        g.world.spill(p + v3(1.5, 0.5, 0.0), V3::ZERO, 0.15, c);
        g.world.spill(p + v3(-1.5, 0.5, 0.0), V3::ZERO, 0.15, c);
        run(&mut g, 0.1);
        assert!(!g.world.opened, "two free and the four pieces are not enough");
        let ids: Vec<u32> = g.world.drops.iter().filter(|d| !g.world.wound.is_caltrop(d.id)).map(|d| d.id).collect();
        for id in ids {
            let i = g.world.drops.iter().position(|d| d.id == id).unwrap();
            let d = g.world.drops.remove(i);
            g.world.body.volume += d.r * d.r * d.r;
        }
        g.world.opened = true;
        run(&mut g, 0.1);
        assert!(g.world.wound.is_open());
        assert!(!g.world.opened, "only the pieces are out, and night ends");
    }

    /// Fills the world with small free drops, leaving `room` for more.
    fn crowd(g: &mut Game, room: usize) {
        let (p, c) = (g.world.body.pos, g.world.body.color);
        let mut k = 0.0f32;
        while g.world.drops.len() < crate::sim::MAX_DROPS - room {
            g.world.spill(p + v3(2.5 * k.cos(), 0.5, 2.5 * k.sin()), V3::ZERO, 0.05, c);
            k += 1.0;
        }
    }

    #[test]
    fn with_no_room_for_enough_pieces_it_stays_closed() {
        let mut g = scarred();
        crowd(&mut g, TO_HEAL as usize - 1);
        hurt(&mut g);
        run(&mut g, 2.0);
        assert!(!g.world.wound.is_open(), "{} pieces", g.world.wound.caltrops.len());
        assert!(g.world.wound.caltrops.is_empty());
        assert!(g.world.drops.iter().all(|d| d.target.is_none()), "the few that got out are let go of");
    }

    #[test]
    fn a_piece_spat_out_with_no_room_comes_out_later() {
        let mut g = scarred();
        hurt(&mut g);
        run(&mut g, 4.0);
        crowd(&mut g, 0);
        // One piece goes into the body, and a free drop takes its room the same frame.
        let id = g.world.wound.caltrops[0].id;
        let i = g.world.drops.iter().position(|d| d.id == id).unwrap();
        let d = g.world.drops.remove(i);
        g.world.body.volume += d.r * d.r * d.r;
        let (p, c) = (g.world.body.pos, g.world.body.color);
        assert!(g.world.spill(p + v3(0.0, 0.5, 2.5), V3::ZERO, 0.05, c));
        run(&mut g, 0.5);
        assert_eq!(g.world.wound.caltrops.len(), CALTROPS - 1, "no room yet");
        let free = g.world.drops.iter().position(|d| !g.world.wound.is_caltrop(d.id)).unwrap();
        let d = g.world.drops.remove(free);
        g.world.body.volume += d.r * d.r * d.r;
        run(&mut g, 0.5);
        assert_eq!(g.world.wound.caltrops.len(), CALTROPS, "it came back out");
    }
}
