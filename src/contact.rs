//! It is learning you. Everything the player does is also something it watches, and the more it
//! has shown you, the more it shows that it has been paying attention:
//!
//! - matter: at first it only answers touch, the way any liquid would.
//! - regard (REGARD_AT discoveries): it turns a little toward the pointer, and it flinches
//!   before a hand reaches the place it is always touched.
//! - echo (ECHO_AT): a rhythm tapped on it comes back a moment later, slower, as if checking;
//!   a reach pulled out of it is made again, from the inside.
//! - initiative (TURN_AT): left alone, it takes a turn. It does what you do most: taps your
//!   rhythm, reaches toward where your hand last was, colors itself, holds out a drop, calls.
//!   You can answer it.
//! - contact (CONTACT_AT, and answered CONTACT_ANSWERS times): once a visit, it goes still and
//!   reaches toward you with everything it has, plays your rhythm back, and says one thing.
//!   The only word of it anyone can read is "you"; the rest is made from your habits.
//!
//! What it learns is kept between visits (`save` / `load`). Nothing here draws: it moves the
//! liquid with the same tendrils, ripples, drops and colors the player's own verbs use, so the
//! shader does not change.

use crate::math::*;
use crate::sim::{palette, World};

pub const REGARD_AT: usize = 6;
pub const ECHO_AT: usize = 14;
pub const TURN_AT: usize = 24;
pub const CONTACT_AT: usize = 36;
pub const CONTACT_ANSWERS: u32 = 3;
/// Untouched this long (seconds), it takes a turn; and no sooner than this after its last one.
pub const TURN_IDLE: f32 = 6.0;
const TURN_GAP: f32 = 11.0;
/// How long an answer may take, in seconds.
const ANSWER_WINDOW: f32 = 6.0;
/// Taps closer together than this (seconds) are one rhythm.
const BEAT_GAP: f32 = 1.0;
/// It plays a rhythm back this much slower.
const ECHO_SLOW: f32 = 1.15;
/// How the contact unfolds, in seconds from its start.
const CONTACT_REACH: f32 = 1.6;
const CONTACT_PLAY: f32 = 4.2;
const CONTACT_SAY: f32 = 5.4;
const CONTACT_LET_GO: f32 = 10.5;
const CONTACT_END: f32 = 12.5;
/// The signs its reply is written in; all of them are in the glyph font (web/fonts).
const SIGNS: [char; 12] = ['∴', '∵', '∷', '⊕', '⊗', '☉', '☽', '✶', '⟡', '⌬', '⍜', '⍟'];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Matter,
    Regard,
    Echo,
    Initiative,
}

pub fn stage(found: usize) -> Stage {
    match found {
        n if n >= TURN_AT => Stage::Initiative,
        n if n >= ECHO_AT => Stage::Echo,
        n if n >= REGARD_AT => Stage::Regard,
        _ => Stage::Matter,
    }
}

/// What it has noticed the player doing, and how often.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Habit {
    Poke,
    Reach,
    Hold,
    Call,
    Take,
}

const HABITS: [Habit; 5] = [Habit::Poke, Habit::Reach, Habit::Hold, Habit::Call, Habit::Take];

/// What it does on its turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn {
    Rhythm,
    Reach,
    Blush,
    Offer,
    Beckon,
}

impl Habit {
    fn turn(self) -> Turn {
        match self {
            Habit::Poke => Turn::Rhythm,
            Habit::Reach => Turn::Reach,
            Habit::Hold => Turn::Blush,
            Habit::Take => Turn::Offer,
            Habit::Call => Turn::Beckon,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// It turned to follow the pointer for the first time.
    Regarded,
    Flinched,
    /// One tap of a rhythm it is giving back (or giving first); `pitch` 0..1 as for a poke.
    Tap { pitch: f32 },
    Echoed,
    Mimicked,
    /// It took a turn.
    Turned(Turn),
    /// The player answered its turn.
    Answered,
    /// It reached toward the viewer with everything.
    Reaching,
    /// The reply is ready to be said.
    Said,
}

/// A tendril of its own: out along `dir` to `len` body radii, held, then let go.
struct Reach {
    handle: Option<u32>,
    dir: V3,
    len: f32,
    t: f32,
    grow: f32,
    hold: f32,
}

impl Reach {
    fn new(dir: V3, len: f32, grow: f32, hold: f32) -> Reach {
        Reach { handle: None, dir: dir.norm(), len, t: 0.0, grow, hold }
    }

    /// Returns false once it is over and the tendril has been let go.
    fn step(&mut self, dt: f32, world: &mut World) -> bool {
        self.t += dt;
        let r = world.body.radius();
        if self.handle.is_none() {
            self.handle = world.begin_stretch(world.body.pos + self.dir * r);
        }
        let Some(id) = self.handle else { return false };
        if self.t > self.grow + self.hold {
            world.release_handle(id);
            return false;
        }
        let k = (self.t / self.grow).min(1.0);
        let k = k * k * (3.0 - 2.0 * k);
        // rest is at 0.8 radii; it never pulls as far as tearing
        let to = world.body.pos + self.dir * (r * (0.8 + self.len.min(1.1) * k));
        world.pull_handle(id, to);
        true
    }

    fn release(&mut self, world: &mut World) {
        if let Some(id) = self.handle.take() {
            world.release_handle(id);
        }
    }
}

/// A rhythm being played: when each beat falls (seconds from now on) and where on the body.
struct Phrase {
    beats: Vec<(f32, V3)>,
    t: f32,
}

/// What it is waiting for after taking a turn.
enum Await {
    Taps { want: usize, got: usize },
    Touch,
    Take(u32),
    Warm,
}

pub struct Contact {
    // ---- learned, kept between visits
    pub habits: [u32; 5],
    /// Sum of the directions it has been poked from (body space); its length says how sure it is.
    pub spot: V3,
    /// The player's rhythm, as the gaps between taps (seconds).
    pub rhythm: Vec<f32>,
    pub answered: u32,
    pub contacted: bool,

    // ---- this visit
    /// Recent pokes: when, from where, how high.
    taps: Vec<(f32, V3, f32)>,
    phrase: Option<Phrase>,
    reaches: Vec<Reach>,
    /// A reach of the player's it means to make again, and when.
    mimic: Option<(f32, V3, f32)>,
    awaiting: Option<(Await, f32)>,
    turn_wait: f32,
    turns: u32,
    /// Seconds into the contact, while it lasts.
    contact: Option<f32>,
    contacted_now: bool,
    flinch_wait: f32,
    hovered: f32,
    echo_wait: f32,
    blush: Option<(f32, f32)>,
    time: f32,
    events: Vec<Event>,
    pub dirty: bool,
}

impl Default for Contact {
    fn default() -> Self {
        Contact {
            habits: [0; 5],
            spot: V3::ZERO,
            rhythm: Vec::new(),
            answered: 0,
            contacted: false,
            taps: Vec::new(),
            phrase: None,
            reaches: Vec::new(),
            mimic: None,
            awaiting: None,
            turn_wait: 0.0,
            turns: 0,
            contact: None,
            contacted_now: false,
            flinch_wait: 0.0,
            hovered: 0.0,
            echo_wait: 0.0,
            blush: None,
            time: 0.0,
            events: Vec::new(),
            dirty: false,
        }
    }
}

/// What the game tells it each frame.
pub struct Sense {
    pub stage: Stage,
    pub found: usize,
    /// Seconds since anyone touched anything.
    pub idle: f32,
    /// Where the camera looks from and toward.
    pub fwd: V3,
    pub right: V3,
    pub up: V3,
    /// Where the pointer last pointed, on the plane through the body, if anywhere.
    pub pointer: Option<V3>,
}

impl Contact {
    fn noticed(&mut self, h: Habit) {
        let i = HABITS.iter().position(|x| *x == h).unwrap();
        self.habits[i] = self.habits[i].saturating_add(1);
        self.dirty = true;
    }

    /// Busy with something of its own.
    pub fn acting(&self) -> bool {
        self.phrase.is_some() || !self.reaches.is_empty() || self.contact.is_some()
    }

    pub fn in_contact(&self) -> bool {
        self.contact.is_some()
    }

    /// The player poked the body at `dir` (from its center), `pitch` high.
    pub fn poked(&mut self, dir: V3, pitch: f32) {
        self.noticed(Habit::Poke);
        // Recent touches count more than old ones.
        self.spot = self.spot * 0.9 + dir;
        self.taps.retain(|t| self.time - t.0 < BEAT_GAP * 6.0);
        if self.taps.last().is_some_and(|t| self.time - t.0 > BEAT_GAP) {
            self.taps.clear();
        }
        self.taps.push((self.time, dir, pitch));
        if let Some((Await::Taps { got, .. }, _)) = &mut self.awaiting {
            *got += 1;
        }
        if let Some((Await::Touch, _)) = self.awaiting {
            self.answer();
        }
    }

    pub fn reached(&mut self, dir: V3, reach: f32, stage: Stage) {
        self.noticed(Habit::Reach);
        if stage >= Stage::Echo && reach > 0.45 && self.mimic.is_none() && !self.acting() {
            self.mimic = Some((self.time + 2.2, dir, reach));
        }
    }

    pub fn held(&mut self) {
        self.noticed(Habit::Hold);
        if let Some((Await::Warm, _)) = self.awaiting {
            self.answer();
        }
    }

    pub fn called(&mut self) {
        self.noticed(Habit::Call);
    }

    /// The player took hold of a drop.
    pub fn took(&mut self, drop: u32) {
        self.noticed(Habit::Take);
        if let Some((Await::Take(d), _)) = self.awaiting {
            if d == drop {
                self.answer();
            }
        }
    }

    /// The player pressed on the body (before it is known whether this is a poke).
    pub fn touched(&mut self, dir: V3, stage: Stage, world: &mut World) {
        if let Some((Await::Touch, _)) = self.awaiting {
            self.answer();
        }
        self.approach(dir, stage, world);
    }

    /// The pointer is over the body at `dir`, nothing pressed; or `None`, over nothing.
    pub fn hover(&mut self, dir: Option<V3>, dt: f32, stage: Stage, world: &mut World) {
        if stage < Stage::Regard || !world.ordinary() {
            return;
        }
        if self.hovered < 2.5 {
            self.hovered += dt;
            if self.hovered >= 2.5 {
                self.events.push(Event::Regarded);
            }
        }
        if let Some(dir) = dir {
            self.approach(dir, stage, world);
        }
    }

    /// A hand coming toward the place it is always touched: it pulls away first.
    fn approach(&mut self, dir: V3, stage: Stage, world: &mut World) {
        let sure = self.habits[0] >= 4 && self.spot.len() > 2.0;
        if stage < Stage::Regard || !sure || self.flinch_wait > 0.0 || !world.ordinary() || self.acting() {
            return;
        }
        if dir.dot(self.spot.norm()) < 0.75 {
            return;
        }
        self.flinch_wait = 3.0;
        world.body.vel -= dir * 1.6;
        world.body.squash_vel -= 1.5;
        world.add_ripple(-dir, 0.25);
        self.events.push(Event::Flinched);
    }

    fn answer(&mut self) {
        self.awaiting = None;
        self.answered += 1;
        self.dirty = true;
        self.events.push(Event::Answered);
    }

    /// The rhythm it knows best: the player's, or a plain one if it has never heard one.
    fn known_rhythm(&self) -> Vec<f32> {
        if self.rhythm.is_empty() {
            vec![0.45, 0.45, 0.9]
        } else {
            self.rhythm.clone()
        }
    }

    fn play(&mut self, gaps: &[f32], dirs: &[V3], lead: f32) {
        let mut t = lead;
        let mut beats = Vec::new();
        for i in 0..=gaps.len() {
            beats.push((t, dirs[i % dirs.len()]));
            if i < gaps.len() {
                t += gaps[i] * ECHO_SLOW;
            }
        }
        self.phrase = Some(Phrase { beats, t: 0.0 });
    }

    /// The directions it was last touched from, or the one it is most often touched from.
    fn spots(&self) -> Vec<V3> {
        let mut dirs: Vec<V3> = self.taps.iter().map(|t| t.1).collect();
        if dirs.is_empty() {
            dirs.push(if self.spot.len() > 0.1 { self.spot.norm() } else { V3::UP });
        }
        dirs
    }

    pub fn step(&mut self, dt: f32, world: &mut World, sense: &Sense) {
        self.time += dt;
        self.flinch_wait = (self.flinch_wait - dt).max(0.0);
        self.echo_wait = (self.echo_wait - dt).max(0.0);
        self.turn_wait = (self.turn_wait - dt).max(0.0);
        let ordinary = world.ordinary();
        // It turns a little toward the pointer, once it has started to regard you.
        world.regard = if sense.stage >= Stage::Regard && ordinary && !self.in_contact() { 0.22 } else { 0.0 };

        if !ordinary {
            // It became something else; whatever it was doing is dropped.
            for mut r in self.reaches.drain(..) {
                r.release(world);
            }
            self.phrase = None;
            self.mimic = None;
            self.contact = None;
            self.blush = None;
            return;
        }

        self.step_contact(dt, world, sense);
        self.step_echo(sense);
        self.step_turn(world, sense);

        if let Some((at, dir, reach)) = self.mimic {
            if self.time >= at {
                self.mimic = None;
                self.reaches.push(Reach::new(dir, reach, 0.9, 1.0));
                self.events.push(Event::Mimicked);
            }
        }
        let mut i = 0;
        while i < self.reaches.len() {
            if self.reaches[i].step(dt, world) {
                i += 1;
            } else {
                self.reaches.remove(i);
            }
        }
        if let Some(p) = &mut self.phrase {
            let before = p.t;
            p.t += dt;
            let now = p.t;
            for &(_, dir) in p.beats.iter().filter(|b| b.0 >= before && b.0 < now) {
                world.add_ripple(dir, 0.45);
                self.events.push(Event::Tap { pitch: (dir.y * 0.5 + 0.5).clamp(0.0, 1.0) });
            }
            if p.beats.last().is_none_or(|b| now > b.0) {
                self.phrase = None;
            }
        }
        if let Some((from, k)) = &mut self.blush {
            *k += dt / 2.0;
            let goal = palette(*from);
            world.body.color = world.body.color.lerp(goal, (dt * 1.2).min(1.0));
            if *k >= 1.0 {
                self.blush = None;
            }
        }
        if let Some((_, left)) = &mut self.awaiting {
            *left -= dt;
            if *left <= 0.0 {
                self.awaiting = None;
            }
        }
        if let Some((Await::Taps { want, got }, _)) = self.awaiting {
            if got >= want {
                self.answer();
            }
        }
    }

    /// A rhythm tapped on it and then left alone: it gives it back.
    fn step_echo(&mut self, sense: &Sense) {
        let Some(last) = self.taps.last() else { return };
        if self.time - last.0 < 1.4 || self.taps.len() < 3 {
            return;
        }
        let gaps: Vec<f32> = self.taps.windows(2).map(|w| w[1].0 - w[0].0).collect();
        // It learns the rhythm whether or not it is ready to answer it.
        if gaps.len() <= 7 {
            self.rhythm = gaps.clone();
            self.dirty = true;
        }
        let dirs = self.spots();
        self.taps.clear();
        if sense.stage < Stage::Echo || self.echo_wait > 0.0 || self.acting() || self.awaiting.is_some() || gaps.len() > 7 {
            return;
        }
        self.echo_wait = 8.0;
        self.play(&gaps, &dirs, 0.2);
        self.events.push(Event::Echoed);
    }

    /// Left alone long enough, it goes first.
    fn step_turn(&mut self, world: &mut World, sense: &Sense) {
        if sense.stage < Stage::Initiative || sense.idle < TURN_IDLE || self.turn_wait > 0.0 || self.acting() {
            return;
        }
        let ready = sense.found >= CONTACT_AT && self.answered >= CONTACT_ANSWERS && !self.contacted_now;
        if ready {
            self.contact = Some(0.0);
            self.contacted_now = true;
            self.turn_wait = TURN_GAP * 2.0;
            return;
        }
        self.turn_wait = TURN_GAP;
        // What you do most, it does; taking turns among the things you do most.
        let mut order: Vec<usize> = (0..HABITS.len()).filter(|&i| self.habits[i] > 0).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.habits[i]));
        order.truncate(3);
        let turn = match order.len() {
            0 => Turn::Beckon,
            n => HABITS[order[self.turns as usize % n]].turn(),
        };
        self.turns += 1;
        let r = world.body.radius();
        match turn {
            Turn::Rhythm => {
                let gaps = self.known_rhythm();
                let dirs = self.spots();
                self.awaiting = Some((Await::Taps { want: gaps.len() + 1, got: 0 }, ANSWER_WINDOW + 3.0));
                self.play(&gaps, &dirs, 0.0);
            }
            Turn::Reach => {
                let toward = sense.pointer.map(|p| p - world.body.pos).filter(|d| d.len() > 0.2 * r).unwrap_or(-sense.fwd);
                self.reaches.push(Reach::new(toward, 1.0, 1.4, 3.0));
                self.awaiting = Some((Await::Touch, ANSWER_WINDOW));
            }
            Turn::Blush => {
                self.blush = Some((self.time * 0.13 + self.turns as f32 * 0.37, 0.0));
                self.awaiting = Some((Await::Warm, ANSWER_WINDOW));
            }
            Turn::Offer => {
                let out = (-sense.fwd + sense.up * 0.25).norm();
                let pos = world.body.pos + out * (r * 1.05);
                let color = world.body.color;
                if world.spill(pos, out * 1.3, 0.2 * r, color) {
                    let id = world.drops.last().map(|d| d.id).unwrap_or(0);
                    self.awaiting = Some((Await::Take(id), ANSWER_WINDOW + 2.0));
                }
            }
            Turn::Beckon => {
                world.reach(-sense.fwd);
            }
        }
        self.events.push(Event::Turned(turn));
    }

    /// Everything it has: stillness, a reach toward you, your rhythm, and one thing said.
    fn step_contact(&mut self, dt: f32, world: &mut World, sense: &Sense) {
        let Some(t) = self.contact else { return };
        let now = t + dt;
        self.contact = Some(now);
        world.agitation = 0.0;
        world.body.vel = world.body.vel * (-3.0 * dt).exp();
        if t < CONTACT_REACH && now >= CONTACT_REACH {
            // four fingers, opened toward whoever is looking
            for a in 0..4 {
                let a = a as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
                let dir = -sense.fwd + (sense.right * a.cos() + sense.up * a.sin()) * 0.5;
                self.reaches.push(Reach::new(dir, 1.1, 2.4, CONTACT_LET_GO - CONTACT_REACH - 2.4));
            }
            self.events.push(Event::Reaching);
        }
        if t < CONTACT_PLAY && now >= CONTACT_PLAY {
            let gaps = self.known_rhythm();
            let dirs = self.spots();
            self.play(&gaps, &dirs, 0.0);
        }
        if t < CONTACT_SAY && now >= CONTACT_SAY {
            self.contacted = true;
            self.dirty = true;
            self.events.push(Event::Said);
        }
        if now >= CONTACT_END {
            self.contact = None;
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// What it says at contact: one sign-word for each thing you do, longer the more you do it,
    /// and spaced like your rhythm. No two players are told the same thing.
    pub fn reply(&self) -> String {
        let mut seed: u32 = 0x9e37_79b9 ^ self.answered.wrapping_mul(0x85eb_ca6b);
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            SIGNS[(seed % SIGNS.len() as u32) as usize]
        };
        let mut words = Vec::new();
        for (i, &n) in self.habits.iter().enumerate() {
            if n == 0 {
                continue;
            }
            for _ in 0..(i as u32 + n) % 7 {
                next();
            }
            let len = (1 + (32 - n.leading_zeros()) / 2).clamp(1, 5);
            words.push((0..len).map(|_| next()).collect::<String>());
        }
        for g in self.known_rhythm() {
            if words.len() >= 5 {
                break;
            }
            words.push((0..((g * 4.0) as usize).clamp(1, 4)).map(|_| next()).collect());
        }
        words.join(" ")
    }

    /// `habits=..;spot=x,y,z;rhythm=..;answered=n;contacted=0|1`
    pub fn save(&self) -> String {
        let list = |v: &[String]| v.join(",");
        let habits: Vec<String> = self.habits.iter().map(|h| h.to_string()).collect();
        let rhythm: Vec<String> = self.rhythm.iter().map(|g| format!("{g:.2}")).collect();
        format!(
            "habits={};spot={:.2},{:.2},{:.2};rhythm={};answered={};contacted={}",
            list(&habits),
            self.spot.x,
            self.spot.y,
            self.spot.z,
            list(&rhythm),
            self.answered,
            self.contacted as u8
        )
    }

    pub fn load(saved: &str) -> Contact {
        let mut c = Contact::default();
        for part in saved.split(';') {
            let Some((k, v)) = part.split_once('=') else { continue };
            let nums: Vec<f32> = v.split(',').filter_map(|n| n.trim().parse().ok()).collect();
            match k {
                "habits" => {
                    for (h, n) in c.habits.iter_mut().zip(&nums) {
                        *h = *n as u32;
                    }
                }
                "spot" if nums.len() == 3 => c.spot = v3(nums[0], nums[1], nums[2]),
                "rhythm" => c.rhythm = nums.into_iter().filter(|g| *g > 0.0 && *g < 2.0).take(7).collect(),
                "answered" => c.answered = nums.first().map_or(0, |n| *n as u32),
                "contacted" => c.contacted = nums.first() == Some(&1.0),
                _ => {}
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sense(stage: Stage, found: usize, idle: f32) -> Sense {
        Sense { stage, found, idle, fwd: v3(0.0, 0.0, -1.0), right: v3(1.0, 0.0, 0.0), up: V3::UP, pointer: None }
    }

    fn run(c: &mut Contact, w: &mut World, s: &Sense, seconds: f32) -> Vec<Event> {
        let mut out = Vec::new();
        for _ in 0..(seconds * 60.0) as usize {
            c.step(1.0 / 60.0, w, s);
            w.step(1.0 / 60.0);
            out.extend(c.drain_events());
        }
        out
    }

    #[test]
    fn stages_follow_what_has_been_found() {
        assert_eq!(stage(0), Stage::Matter);
        assert_eq!(stage(REGARD_AT), Stage::Regard);
        assert_eq!(stage(ECHO_AT + 1), Stage::Echo);
        assert_eq!(stage(58), Stage::Initiative);
    }

    #[test]
    fn a_rhythm_comes_back_slower_and_only_once_it_listens() {
        let (mut c, mut w) = (Contact::default(), World::new());
        for _ in 0..3 {
            c.poked(V3::UP, 0.9);
            run(&mut c, &mut w, &sense(Stage::Regard, 8, 0.0), 0.4);
        }
        let heard = run(&mut c, &mut w, &sense(Stage::Regard, 8, 0.0), 3.0);
        assert!(!heard.contains(&Event::Echoed), "too early in the arc to answer");
        assert_eq!(c.rhythm.len(), 2, "but it learned the rhythm anyway");

        for _ in 0..3 {
            c.poked(V3::UP, 0.9);
            run(&mut c, &mut w, &sense(Stage::Echo, 20, 0.0), 0.4);
        }
        let heard = run(&mut c, &mut w, &sense(Stage::Echo, 20, 0.0), 3.0);
        assert!(heard.contains(&Event::Echoed));
        assert_eq!(heard.iter().filter(|e| matches!(e, Event::Tap { .. })).count(), 3);
    }

    #[test]
    fn it_flinches_from_the_place_it_is_always_touched() {
        let (mut c, mut w) = (Contact::default(), World::new());
        for _ in 0..6 {
            c.poked(V3::UP, 1.0);
        }
        c.hover(Some(v3(1.0, 0.0, 0.0)), 0.1, Stage::Regard, &mut w);
        assert!(!c.drain_events().contains(&Event::Flinched), "not there");
        c.hover(Some(V3::UP), 0.1, Stage::Regard, &mut w);
        assert!(c.drain_events().contains(&Event::Flinched));
        assert!(w.body.vel.y < 0.0, "it pulled away from the hand above it");
    }

    #[test]
    fn it_makes_your_reach_again_without_tearing() {
        let (mut c, mut w) = (Contact::default(), World::new());
        c.reached(v3(1.0, 0.0, 0.0), 0.9, Stage::Echo);
        let e = run(&mut c, &mut w, &sense(Stage::Echo, 20, 0.0), 3.0);
        assert!(e.contains(&Event::Mimicked));
        assert!(!w.handles.is_empty() && w.handles[0].tip.x > 0.9);
        run(&mut c, &mut w, &sense(Stage::Echo, 20, 0.0), 4.0);
        assert!(w.drops.is_empty(), "it never tore");
        assert!(w.handles.is_empty(), "and it let go");
    }

    #[test]
    fn left_alone_it_takes_a_turn_and_can_be_answered() {
        let (mut c, mut w) = (Contact::default(), World::new());
        for _ in 0..5 {
            c.noticed(Habit::Take);
        }
        let e = run(&mut c, &mut w, &sense(Stage::Initiative, 30, TURN_IDLE + 1.0), 0.5);
        assert!(e.contains(&Event::Turned(Turn::Offer)));
        assert_eq!(w.drops.len(), 1, "it held a piece of itself out");
        let id = w.drops[0].id;
        c.took(id);
        assert!(c.drain_events().contains(&Event::Answered));
        assert_eq!(c.answered, 1);
    }

    #[test]
    fn contact_comes_once_a_visit_after_enough_answers() {
        let (mut c, mut w) = (Contact::default(), World::new());
        c.answered = CONTACT_ANSWERS;
        c.habits = [9, 2, 1, 0, 3];
        let s = sense(Stage::Initiative, CONTACT_AT, TURN_IDLE + 1.0);
        let e = run(&mut c, &mut w, &s, CONTACT_REACH + 3.0);
        assert!(e.contains(&Event::Reaching));
        assert_eq!(w.handles.len(), 4, "every tendril, toward the viewer");
        assert!(w.handles.iter().all(|h| h.tip.z > 0.5), "{:?}", w.handles.iter().map(|h| h.tip).collect::<Vec<_>>());
        let e = run(&mut c, &mut w, &s, CONTACT_END);
        assert!(e.contains(&Event::Said));
        assert!(c.contacted && w.drops.is_empty());
        let e = run(&mut c, &mut w, &s, 30.0);
        assert!(!e.contains(&Event::Reaching), "once a visit");
    }

    #[test]
    fn the_reply_is_signs_made_from_you() {
        let mut a = Contact::default();
        a.habits = [12, 3, 0, 1, 0];
        let mut b = Contact::default();
        b.habits = [1, 0, 7, 0, 2];
        let (ra, rb) = (a.reply(), b.reply());
        assert!(ra.chars().all(|ch| ch == ' ' || SIGNS.contains(&ch)), "{ra}");
        assert_ne!(ra, rb);
        assert!(ra.split(' ').count() >= 3);
    }

    #[test]
    fn what_it_learned_survives_a_visit() {
        let mut c = Contact::default();
        c.habits = [4, 1, 0, 2, 9];
        c.spot = v3(0.5, 1.5, -0.25);
        c.rhythm = vec![0.3, 0.6];
        c.answered = 2;
        c.contacted = true;
        let d = Contact::load(&c.save());
        assert_eq!(d.habits, c.habits);
        assert!((d.spot - c.spot).len() < 0.01);
        assert_eq!(d.rhythm, c.rhythm);
        assert_eq!((d.answered, d.contacted), (2, true));
        assert_eq!(Contact::load("").habits, [0; 5]);
        assert_eq!(Contact::load("garbage;habits=x").answered, 0);
    }
}
