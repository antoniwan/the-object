//! What the keyboard can do that a hand cannot. It is not a second mouse: every key reaches
//! it from somewhere of its own.
//!
//! - type a letter: it lands on the surface where that key sits on the keyboard, q high on
//!   the left, m low on the right, each with its own note
//! - hold a letter down: it hums that note back, trembling in time, drifting toward that
//!   letter's color
//! - hold space: it draws itself up as you breathe in, and sighs out when you let go;
//!   breathe slowly enough, several times, and it goes on breathing after you, at your pace
//! - the arrow keys tip the room: it and any loose drops slide toward the low side,
//!   and come home when the room is level (a phone tipped in the hand does the same, motion.rs)
//! - after it has said its one word, type that word back
//!
//! Everything here acts only on the ordinary liquid. The page passes keys in by their place
//! on the keyboard (`KeyboardEvent.code`), so the places hold on any layout.

use crate::game::{Game, Sound};
use crate::math::*;
use crate::progress::Discovery;
use crate::sim::palette;

/// The letter rows, as they sit. `m` is left out: it mutes.
const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbn"];
/// Held this long, a letter stops being a letter and starts to hum.
const HUM_AFTER: f32 = 0.45;
const HUM_FOUND: f32 = 1.6;
/// Seconds between the small ripples of a hum.
const HUM_PULSE: f32 = 0.22;
/// Letters typed before it is clear they are letters.
const LETTERS_FOUND: u32 = 8;
/// A breath in this long or longer is slow; shorter is only a gasp.
const BREATH_SLOW: f32 = 1.2;
/// How much more of it there is, full of breath: a third more volume, a tenth more across.
const SWELL: f32 = 0.33;
/// Slow breaths in a row, none further apart than this, before it follows.
const BREATHS_TO_FOLLOW: usize = 3;
const BREATH_GAP: f32 = 9.0;
/// Breaths it takes on its own afterward.
const OWN_BREATHS: u32 = 4;
/// How far the room tips, and how long it must stay tipped to count.
const LEAN_PUSH: f32 = 16.0;
const LEAN_FOUND: f32 = 1.0;
/// Its one word (contact.rs). Typed back to it once it has said it.
const ITS_WORD: &str = "you";

#[derive(Default)]
pub struct Keys {
    /// Letters held down, and for how long.
    held: Vec<(char, f32)>,
    typed: String,
    letters: u32,
    hum_wait: f32,
    /// Seconds into the breath being drawn, while space is down.
    inhale: Option<f32>,
    /// 0..1, how full it is.
    breath: f32,
    /// Volume it is holding in on top of its own, given back exactly.
    swell: f32,
    /// When each recent slow breath began.
    slow: Vec<f32>,
    /// Breathing on its own: seconds per breath, seconds into this one, breaths left.
    own: Option<(f32, f32, u32)>,
    /// Left, right, up, down.
    arrows: [bool; 4],
    lean: (f32, f32),
    /// How far a phone in the hand tips the room, right and away, -1..1 (motion.rs).
    pub tilt: (f32, f32),
    leaned: f32,
    clock: f32,
}

impl Keys {
    /// What it is humming, 0..1 low to high, if anything.
    pub fn hum(&self) -> Option<f32> {
        self.humming().map(|(c, _)| place(c).map_or(0.5, |p| p.pitch))
    }

    fn humming(&self) -> Option<(char, f32)> {
        self.held.last().copied().filter(|&(_, t)| t > HUM_AFTER)
    }
}

/// Where a letter lands, as seen from the player: x and y in -1..1, and its note.
struct Place {
    x: f32,
    y: f32,
    pitch: f32,
    /// Where its color sits in the palette's cycle.
    hue: f32,
}

fn place(c: char) -> Option<Place> {
    ROWS.iter().enumerate().find_map(|(row, keys)| {
        let col = keys.find(c)? as f32;
        let x = ((col + row as f32 * 0.4 - 4.5) / 5.0).clamp(-1.0, 1.0);
        let y = 0.6 - row as f32 * 0.6;
        let pitch = (0.5 + x * 0.4 + y * 0.25).clamp(0.0, 1.0);
        let hue = (col + row as f32 * 10.0) / 26.0;
        Some(Place { x, y, pitch, hue })
    })
}

/// The letter a key's code names, `KeyQ` is `q`.
fn letter(code: &str) -> Option<char> {
    let c = code.strip_prefix("Key")?.chars().next()?.to_ascii_lowercase();
    place(c).map(|_| c)
}

/// Direction from the body's center toward where a letter lands, facing the player.
fn toward(g: &Game, p: &Place) -> V3 {
    let b = g.cam.basis();
    (b.right * p.x + b.up * p.y - b.fwd * 0.9).norm()
}

/// A key went down. Returns whether it meant anything here, so the page can keep it from the browser.
pub fn down(g: &mut Game, code: &str) -> bool {
    g.thoughts.keyed();
    if let Some(c) = letter(code) {
        if g.keys.held.iter().any(|h| h.0 == c) {
            return true;
        }
        g.keys.held.push((c, 0.0));
        g.keys.hum_wait = 0.0;
        if g.world.ordinary() {
            type_letter(g, c);
        }
        return true;
    }
    match code {
        "Space" => {
            if g.keys.inhale.is_none() {
                g.keys.inhale = Some(0.0);
                g.keys.own = None;
            }
        }
        "ArrowLeft" => g.keys.arrows[0] = true,
        "ArrowRight" => g.keys.arrows[1] = true,
        "ArrowUp" => g.keys.arrows[2] = true,
        "ArrowDown" => g.keys.arrows[3] = true,
        _ => return false,
    }
    true
}

pub fn up(g: &mut Game, code: &str) {
    if let Some(c) = letter(code) {
        g.keys.held.retain(|h| h.0 != c);
        return;
    }
    match code {
        "Space" => exhale(g),
        "ArrowLeft" => g.keys.arrows[0] = false,
        "ArrowRight" => g.keys.arrows[1] = false,
        "ArrowUp" => g.keys.arrows[2] = false,
        "ArrowDown" => g.keys.arrows[3] = false,
        _ => {}
    }
}

/// The page lost the keyboard (another window, another tab): every key is let go.
pub fn release(g: &mut Game) {
    exhale(g);
    g.keys.held.clear();
    g.keys.arrows = [false; 4];
}

fn type_letter(g: &mut Game, c: char) {
    let Some(p) = place(c) else { return };
    let dir = toward(g, &p);
    g.world.add_ripple(dir, 0.55);
    g.world.body.vel -= dir * 0.35;
    g.sounds.push(Sound::Poke { pitch: p.pitch, size: g.world.body.radius() });
    g.keys.letters += 1;
    g.keys.typed.push(c);
    if g.keys.typed.len() > 8 {
        g.keys.typed.remove(0);
    }
    if g.keys.letters >= LETTERS_FOUND {
        g.discover(Discovery::Letters);
    }
    if g.keys.typed.ends_with(ITS_WORD) && g.progress.has(Discovery::Contact) {
        g.keys.typed.clear();
        let face = -g.cam.basis().fwd;
        g.world.reach(face);
        g.world.add_ripple(face, 1.0);
        g.sounds.push(Sound::Merge);
        g.discover(Discovery::Said);
    }
}

fn exhale(g: &mut Game) {
    let Some(t) = g.keys.inhale.take() else { return };
    if !g.world.ordinary() {
        return;
    }
    if t > 0.4 {
        g.world.add_ripple(-V3::UP, 0.35 * g.keys.breath);
        g.sounds.push(Sound::Merge);
    }
    let k = &mut g.keys;
    if t < BREATH_SLOW {
        k.slow.clear();
        return;
    }
    let now = k.clock;
    k.slow.retain(|&at| now - at < BREATH_GAP * BREATHS_TO_FOLLOW as f32);
    if k.slow.last().is_some_and(|&at| now - t - at > BREATH_GAP) {
        k.slow.clear();
    }
    k.slow.push(now - t);
    if k.slow.len() >= BREATHS_TO_FOLLOW {
        // Your pace: from the start of one breath to the start of the next.
        let n = k.slow.len();
        let period = ((k.slow[n - 1] - k.slow[0]) / (n - 1) as f32).clamp(2.5, 9.0);
        // It waits out the rest of yours before its first.
        k.own = Some((period, -(period - t).max(0.5), OWN_BREATHS));
        k.slow.clear();
        g.discover(Discovery::Breath);
    }
}

/// Every frame, from `Game::update`, after the player's squeeze is set and before the world moves.
pub fn step(g: &mut Game, dt: f32) {
    let k = &mut g.keys;
    k.clock += dt;
    for h in &mut k.held {
        h.1 += dt;
    }
    let ordinary = g.world.ordinary();

    // Breath: full while space is down, empty when it is let go, or its own, after you.
    let mut goal = 0.0;
    let mut rate = 1.6;
    if let Some(t) = &mut k.inhale {
        *t += dt;
        goal = 1.0;
        rate = 0.9;
    } else if let Some((period, t, left)) = &mut k.own {
        *t += dt;
        if *t > *period {
            *t -= *period;
            *left -= 1;
        }
        let tau = std::f32::consts::TAU;
        // In for a little under half the breath, then out.
        goal = if *t > 0.0 { (0.5 - 0.5 * (tau * *t / *period).cos()).powf(1.5) } else { 0.0 };
        rate = 2.5;
        if *left == 0 {
            k.own = None;
        }
    }
    if !ordinary {
        goal = 0.0;
        k.own = None;
    }
    k.breath = ease(k.breath, goal, -rate * dt);
    // Drawn in, it is more of itself for a moment; what it borrowed is given back as it empties.
    let base = g.world.body.volume - k.swell;
    k.swell = base * SWELL * k.breath;
    g.world.body.volume = base + k.swell;

    // The room tips toward whichever way the arrows say.
    let a = k.arrows;
    let side = ((a[1] as i8 - a[0] as i8) as f32 + k.tilt.0).clamp(-1.0, 1.0);
    let away = ((a[2] as i8 - a[3] as i8) as f32 + k.tilt.1).clamp(-1.0, 1.0);
    let ease = 1.0 - (-4.0 * dt).exp();
    k.lean.0 += (side - k.lean.0) * ease;
    k.lean.1 += (away - k.lean.1) * ease;
    let tipped = k.lean.0.hypot(k.lean.1);
    // At most one of each, and held they come every frame: kept off the heap.
    let mut found = [None; 2];
    if ordinary && tipped > 0.01 {
        let b = g.cam.basis();
        let mut level = b.fwd;
        level.y = 0.0;
        let slope = b.right * k.lean.0 + level.norm() * k.lean.1;
        g.world.body.vel += slope * (LEAN_PUSH * dt);
        for d in g.world.drops.iter_mut().filter(|d| d.target.is_none()) {
            d.vel += slope * (1.6 * dt);
        }
        k.leaned = if tipped > 0.6 { k.leaned + dt } else { 0.0 };
        if k.leaned > LEAN_FOUND {
            found[0] = Some(Discovery::Lean);
        }
    } else {
        k.leaned = 0.0;
    }

    // A held letter hums: small ripples from its place, in time, and its color seeps in.
    if let (Some((c, held)), true) = (k.humming(), ordinary) {
        let p = place(c).unwrap();
        k.hum_wait -= dt;
        if k.hum_wait <= 0.0 {
            k.hum_wait = HUM_PULSE;
            let dir = toward(g, &p);
            g.world.add_ripple(dir, 0.16);
        }
        let goal = palette(p.hue + held * 0.04);
        g.world.body.color = g.world.body.color.lerp(goal, 1.0 - (-0.9 * dt).exp());
        if held > HUM_FOUND {
            found[1] = Some(Discovery::Hum);
        }
    }
    for d in found.into_iter().flatten() {
        g.discover(d);
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

    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn letters_land_where_their_keys_sit() {
        let q = place('q').unwrap();
        let n = place('n').unwrap();
        assert!(q.x < 0.0 && q.y > 0.0 && n.x > 0.0 && n.y < 0.0);
        assert!(q.pitch != n.pitch);
        assert_eq!(letter("KeyM"), None);
        assert_eq!(letter("Digit1"), None);

        let mut g = game();
        for code in ["KeyH", "KeyE", "KeyL", "KeyL", "KeyO", "KeyT", "KeyH", "KeyE"] {
            assert!(down(&mut g, code));
            up(&mut g, code);
            run(&mut g, 0.1);
        }
        assert!(g.progress.has(Discovery::Letters));
        assert!(!g.world.ripples.is_empty());
    }

    #[test]
    fn a_held_letter_hums_and_colors_it() {
        let mut g = game();
        let before = g.world.body.color;
        down(&mut g, "KeyG");
        assert_eq!(g.keys.hum(), None);
        run(&mut g, 2.0);
        assert!(g.keys.hum().is_some());
        assert!(g.progress.has(Discovery::Hum));
        assert!((g.world.body.color - before).len() > 0.02);
        up(&mut g, "KeyG");
        assert_eq!(g.keys.hum(), None);
    }

    #[test]
    fn breathed_with_slowly_it_breathes_on_after_you() {
        let mut g = game();
        let r = g.world.body.radius();
        // A gasp is not a breath.
        down(&mut g, "Space");
        run(&mut g, 0.3);
        up(&mut g, "Space");
        run(&mut g, 1.0);
        for _ in 0..BREATHS_TO_FOLLOW {
            down(&mut g, "Space");
            run(&mut g, 2.0);
            assert!(g.world.body.radius() > r * 1.06, "it draws itself up");
            up(&mut g, "Space");
            run(&mut g, 2.5);
        }
        assert!(g.progress.has(Discovery::Breath));
        // Now nobody is pressing anything, and it still rises and falls.
        let mut tallest: f32 = 0.0;
        for _ in 0..(6.0 * 60.0) as usize {
            g.update(1.0 / 60.0);
            tallest = tallest.max(g.world.body.radius());
        }
        assert!(tallest > r * 1.05);
        run(&mut g, 40.0);
        assert!(g.keys.own.is_none() && g.keys.breath < 0.05);
        assert!((g.world.body.radius() - r).abs() < r * 0.01, "all of it given back");
    }

    #[test]
    fn hurried_breaths_are_not_followed() {
        let mut g = game();
        for _ in 0..6 {
            down(&mut g, "Space");
            run(&mut g, 0.5);
            up(&mut g, "Space");
            run(&mut g, 0.5);
        }
        assert!(!g.progress.has(Discovery::Breath));
    }

    #[test]
    fn tipping_the_room_slides_it_and_level_brings_it_home() {
        let mut g = game();
        let home = g.world.body.pos;
        down(&mut g, "ArrowLeft");
        run(&mut g, 1.6);
        let right = g.cam.basis().right;
        assert!((g.world.body.pos - home).dot(right) < -0.1);
        assert!(g.progress.has(Discovery::Lean));
        up(&mut g, "ArrowLeft");
        run(&mut g, 4.0);
        assert!((g.world.body.pos - home).len() < 0.05);
    }

    #[test]
    fn its_word_given_back_only_once_it_has_said_it() {
        let mut g = game();
        for code in ["KeyY", "KeyO", "KeyU"] {
            down(&mut g, code);
            up(&mut g, code);
        }
        assert!(!g.progress.has(Discovery::Said));
        g.progress.discover(Discovery::Contact);
        for code in ["KeyY", "KeyO", "KeyU"] {
            down(&mut g, code);
            up(&mut g, code);
        }
        assert!(g.progress.has(Discovery::Said));
    }

    #[test]
    fn keys_do_nothing_to_anything_but_the_ordinary_liquid() {
        let mut g = game();
        g.world.division.phase = crate::meiosis::Phase::Two;
        assert!(!g.world.ordinary());
        down(&mut g, "Space");
        down(&mut g, "ArrowRight");
        for code in ["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK"] {
            down(&mut g, code);
            up(&mut g, code);
        }
        run(&mut g, 2.0);
        assert!(g.keys.breath < 0.01);
        assert!(!g.progress.has(Discovery::Letters) && !g.progress.has(Discovery::Lean));
        release(&mut g);
        assert!(g.keys.inhale.is_none() && g.keys.arrows == [false; 4]);
    }
}
