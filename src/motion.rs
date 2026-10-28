//! What a phone in the hand can do to it: the screen is the room, so moving the phone moves
//! the room around it.
//!
//! - tip the phone: the room tips with it, the same way the arrow keys tip it (keys.rs); it and
//!   any loose drops slide downhill, and come home when the phone is level again. Level is
//!   however the phone was held when it started to feel, and slowly becomes however it is held.
//! - shake the phone: it sloshes against the walls of the room, the other way from each jolt,
//!   and the shaking upsets it. Keep shaking and it comes apart where it is thinnest, drops
//!   flung off the way it was thrown; enough of them free and the room goes dark, as always.
//!
//! Most phones simply let the page feel how they are held. Some (iOS) only after asking: then
//! it asks once a visit, late enough in the arc, in one word (`hold the room.`, senses.rs), and
//! only the player's tap on that word lets the browser ask. On a desk nothing arrives and
//! nothing happens. Readings are a few numbers a frame, used and dropped. Everything here acts
//! only on the ordinary liquid.

use crate::game::{Game, Sound};
use crate::math::*;
use crate::progress::Discovery;
use crate::senses::Kind;

/// Degrees of tip that tip the room all the way; less than DEAD is the hand being a hand.
const FULL_TILT: f32 = 18.0;
const DEAD: f32 = 3.0;
/// Seconds for however the phone is held to become level.
const SETTLE: f32 = 20.0;
/// A jolt, in m/s² beyond gravity; walking with it is under this, shaking it is well over.
const JOLT: f32 = 13.0;
/// Jolts closer together than this are one jolt.
const JOLT_APART: f32 = 0.09;
/// This many jolts within SHAKE_WINDOW seconds is a shake.
const SHAKE_JOLTS: usize = 3;
const SHAKE_WINDOW: f32 = 1.2;
/// Shaken this long without stopping, it starts to come apart, a drop every SHED_EVERY.
const SHED_AFTER: f32 = 1.2;
const SHED_EVERY: f32 = 0.4;
/// How hard a jolt throws it, per m/s².
const SLOSH: f32 = 0.07;

/// One frame of what the phone felt, already turned to the screen: x right, y up, z out of
/// the glass toward the player.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Felt {
    /// How it is held: front-to-back and side-to-side, in degrees (`beta`, `gamma`), and which
    /// way the screen is turned (0, 90, 180, 270).
    pub held: Option<(f32, f32, f32)>,
    /// The hardest push since the last frame, gravity taken out, in m/s².
    pub push: Option<V3>,
}

#[derive(Default)]
pub struct Motion {
    /// Something has been felt at all: this is a thing that can be held.
    pub alive: bool,
    felt: Felt,
    /// What level is, in degrees, and which way the screen was turned when it was decided.
    level: Option<(f32, f32, f32)>,
    jolts: Vec<f32>,
    last_jolt: f32,
    /// Seconds it has been shaken without a pause.
    shaking: f32,
    shed_wait: f32,
    /// The way it was last thrown, on screen.
    thrown: V3,
    clock: f32,
}

impl Motion {
    /// The latest reading, from the page.
    pub fn feel(&mut self, f: Felt) {
        if f.held.is_some() || f.push.is_some() {
            self.alive = true;
        }
        if f.held.is_some() {
            self.felt.held = f.held;
        }
        if let Some(p) = f.push {
            if self.felt.push.map_or(true, |q| p.len() > q.len()) {
                self.felt.push = Some(p);
            }
        }
    }

    /// How far the room is tipped, -1..1 to the right and away, as the player sees it.
    fn tipped(&mut self, dt: f32) -> (f32, f32) {
        let Some((beta, gamma, turn)) = self.felt.held else { return (0.0, 0.0) };
        let level = match self.level {
            Some(l) if l.2 == turn => l,
            // A first reading, or the screen turned: however it is held now is level.
            _ => (beta, gamma, turn),
        };
        // Level slowly becomes however it is held.
        let k = 1.0 - (-dt / SETTLE).exp();
        let level = (level.0 + wrap(beta - level.0) * k, level.1 + (gamma - level.1) * k, turn);
        self.level = Some(level);
        downhill(wrap(beta - level.0), gamma - level.1, turn)
    }
}

/// An angle difference, into -180..180.
fn wrap(d: f32) -> f32 {
    (d + 540.0).rem_euclid(360.0) - 180.0
}

/// Which way is downhill on the screen, -1..1 right and away, from how far the phone has tipped
/// from level. The phone's own axes turn with the screen: at 90 its top is on the player's left.
pub fn downhill(d_beta: f32, d_gamma: f32, turn: f32) -> (f32, f32) {
    let soft = |d: f32| (d.signum() * (d.abs() - DEAD).max(0.0) / (FULL_TILT - DEAD)).clamp(-1.0, 1.0);
    // In the phone's own terms: its right edge down is downhill right; its top edge down
    // (front-to-back tipping less) is downhill toward its top.
    let (x, y) = (soft(d_gamma), soft(-d_beta));
    let (s, c) = turn.to_radians().sin_cos();
    ((x * c - y * s).clamp(-1.0, 1.0), (x * s + y * c).clamp(-1.0, 1.0))
}

/// Every frame, from `Game::update`, before keys.rs tips the room.
pub fn step(g: &mut Game, dt: f32) {
    let m = &mut g.motion;
    m.clock += dt;
    m.shed_wait = (m.shed_wait - dt).max(0.0);
    let open = g.senses.is_open(Kind::Feel);
    let ordinary = g.world.ordinary();
    let tip = if open { m.tipped(dt) } else { (0.0, 0.0) };
    g.keys.tilt = tip;

    let push = m.felt.push.take().filter(|_| open);
    let now = m.clock;
    m.jolts.retain(|&t| now - t < SHAKE_WINDOW);
    let jolt = push.filter(|p| p.len() > JOLT && now - m.last_jolt > JOLT_APART);
    if jolt.is_none() && now - m.last_jolt > 0.5 {
        m.shaking = 0.0;
    } else if m.jolts.len() >= SHAKE_JOLTS {
        m.shaking += dt;
    }
    if !ordinary {
        return;
    }
    let mut found = Vec::new();
    if let Some(p) = jolt {
        m.last_jolt = now;
        m.jolts.push(now);
        m.thrown = p.norm();
        let b = g.cam.basis();
        // On the screen, toward the player is out of the glass.
        let way = (b.right * m.thrown.x + b.up * m.thrown.y - b.fwd * m.thrown.z).norm();
        let hard = p.len().min(40.0);
        // The room went one way; it stays behind, and hits the other side.
        g.world.body.vel -= way * (hard * SLOSH);
        g.world.body.squash_vel += if m.thrown.y.abs() > 0.6 { -2.5 } else { 1.8 };
        for d in g.world.drops.iter_mut().filter(|d| d.target.is_none()) {
            d.vel -= way * (hard * SLOSH * 0.6);
        }
        g.world.add_ripple(-way, (hard / 40.0).clamp(0.25, 0.8));
        g.cam.shake = g.cam.shake.max(0.012);
        g.sounds.push(Sound::Poke { pitch: 0.08, size: g.world.body.radius() * 1.4 });
        if m.jolts.len() >= SHAKE_JOLTS {
            found.push(Discovery::Shaken);
        }
    }
    if m.shaking > 0.3 {
        g.world.agitation = g.world.agitation.max((m.shaking * 0.4).min(1.0));
    }
    // Shaken past what it can hold: it comes loose on the side it was flung toward.
    if m.shaking > SHED_AFTER && m.shed_wait == 0.0 {
        m.shed_wait = SHED_EVERY;
        let b = g.cam.basis();
        let t = m.thrown;
        let away = -(b.right * t.x + b.up * t.y - b.fwd * t.z);
        let wobble = b.right * (m.clock * 9.0).sin() * 0.5 + b.up * (m.clock * 6.0).cos() * 0.4;
        let out = (away.norm() + wobble).norm();
        let r = g.world.body.radius();
        let color = g.world.body.color;
        if g.world.spill(g.world.body.pos + out * (r * 1.05), out * 2.6, 0.17 * r, color) {
            g.sounds.push(Sound::Tear);
            found.push(Discovery::Scatter);
        }
    }
    for d in found {
        g.discover(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::senses::Leave;

    fn game() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.senses.feel = Leave::Open;
        g
    }

    fn run(g: &mut Game, secs: f32, mut each: impl FnMut(&mut Game, usize)) {
        for i in 0..(secs * 60.0) as usize {
            each(g, i);
            g.update(1.0 / 60.0);
        }
    }

    /// Held at a comfortable angle, then tipped `side` degrees to the right.
    fn held(side: f32) -> Felt {
        Felt { held: Some((45.0, side, 0.0)), push: None }
    }

    /// A phone shaken left and right, about five times a second.
    fn shaken(g: &mut Game, i: usize) {
        let x = if (i / 6) % 2 == 0 { 22.0 } else { -22.0 };
        g.motion.feel(Felt { held: None, push: Some(v3(x, 0.0, 0.0)) });
    }

    #[test]
    fn downhill_turns_with_the_screen() {
        assert_eq!(downhill(0.0, 1.0, 0.0), (0.0, 0.0), "a hand is not a tilt");
        let (x, y) = downhill(0.0, 30.0, 0.0);
        assert!(x > 0.99 && y.abs() < 0.01, "right edge down: downhill right");
        let (x, y) = downhill(-30.0, 0.0, 0.0);
        assert!(x.abs() < 0.01 && y > 0.99, "top edge down: downhill away");
        // Turned on its side, its top is on the left: the top going down is downhill left.
        let (x, y) = downhill(-30.0, 0.0, 90.0);
        assert!(x < -0.99 && y.abs() < 0.01, "{x} {y}");
        assert_eq!(wrap(350.0), -10.0);
        assert_eq!(wrap(-190.0), 170.0);
    }

    #[test]
    fn tipping_the_phone_tips_the_room_and_level_is_how_it_was_held() {
        let mut g = game();
        let home = g.world.body.pos;
        run(&mut g, 0.5, |g, _| g.motion.feel(held(0.0)));
        assert!((g.world.body.pos - home).len() < 0.01, "however it is held to begin with is level");
        run(&mut g, 1.6, |g, _| g.motion.feel(held(25.0)));
        let right = g.cam.basis().right;
        assert!((g.world.body.pos - home).dot(right) > 0.1);
        assert!(g.progress.has(Discovery::Lean));
        run(&mut g, 4.0, |g, _| g.motion.feel(held(0.0)));
        assert!((g.world.body.pos - home).len() < 0.05);
    }

    #[test]
    fn shaken_it_sloshes_then_comes_loose_and_the_room_goes_dark() {
        let mut g = game();
        run(&mut g, 0.25, shaken);
        assert!(!g.progress.has(Discovery::Shaken), "one jolt is not a shake");
        run(&mut g, 0.6, shaken);
        assert!(g.progress.has(Discovery::Shaken));
        assert!(g.world.drops.is_empty(), "not yet");
        assert!(!g.world.ripples.is_empty());
        run(&mut g, 3.0, shaken);
        assert!(g.progress.has(Discovery::Scatter));
        assert!(g.world.agitation > 0.5);
        run(&mut g, 2.0, shaken);
        assert!(g.world.drops.len() >= crate::sim::OPEN_AT, "{}", g.world.drops.len());
        assert!(g.progress.has(Discovery::Open));
    }

    #[test]
    fn walking_with_it_is_not_shaking_it() {
        let mut g = game();
        run(&mut g, 4.0, |g, i| {
            let x = if (i / 15) % 2 == 0 { 4.0 } else { -4.0 };
            g.motion.feel(Felt { held: Some((45.0, 0.0, 0.0)), push: Some(v3(x, 2.0, 1.0)) });
        });
        assert!(!g.progress.has(Discovery::Shaken));
    }

    #[test]
    fn nothing_is_felt_until_it_is_let_feel_or_outside_the_ordinary_liquid() {
        let mut g = game();
        g.senses.feel = Leave::Closed;
        run(&mut g, 3.0, |g, i| {
            g.motion.feel(held(30.0));
            shaken(g, i);
        });
        assert!(!g.progress.has(Discovery::Shaken) && !g.progress.has(Discovery::Lean));
        assert!(g.motion.alive, "but it knows it could be");

        let mut g = game();
        g.world.division.phase = crate::meiosis::Phase::Two;
        run(&mut g, 3.0, shaken);
        assert!(g.world.drops.is_empty() && !g.progress.has(Discovery::Shaken));
    }
}
