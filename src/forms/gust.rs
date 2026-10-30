//! Let go into dust, it can still be breathed on. Blow at it once it hears you, or hold
//! space and let it out, and the drifts move away from you over the floor, further the
//! harder the breath, the lighter ones a little further than the heavy; drifts blown into
//! each other run together as they always do (dust.rs). They never go past where they can
//! be seen, and the breath takes them up and lets them settle gradually, so nothing jumps.
//!
//! The rest of the senses act only on the ordinary liquid; this is one of the two places
//! they reach into a form (the other is burst/tilt.rs). The drifts only change where they
//! lie, so nothing new reaches the shader.

use super::dust::Phase;
use crate::game::Game;
use crate::progress::Discovery;

/// How fast the drifts go, in units a second, under the hardest breath.
const SPEED: f32 = 1.2;
/// How fast the drifts take up the breath, and lose it once it stops (per second, eased).
const TAKE_UP: f32 = 3.0;
/// How long a breath let out with space blows for, in seconds.
const PUFF_SECS: f32 = 0.9;
/// Moved this far by breath in all, and it has been found.
const FOUND_AFTER: f32 = 0.6;

#[derive(Default)]
pub struct Gust {
    /// How the drifts are moving now, (x, z) in units a second, eased.
    wind: (f32, f32),
    /// A breath let out with space: how hard, and how long it has left.
    puff: (f32, f32),
    /// How far breath has moved them, this time they lie on the floor.
    moved: f32,
}

/// Every frame, from `Game::update`, after keys.rs and before the world moves; hearing is the
/// last frame's (senses/).
pub fn step(g: &mut Game, dt: f32) {
    let puff = std::mem::take(&mut g.keys.puff);
    let s = &mut g.world.forms.gust;
    if g.world.forms.dust.phase != Phase::Dust {
        *s = Gust::default();
        return;
    }
    if puff > 0.0 {
        s.puff = (s.puff.0.max(puff), PUFF_SECS);
    }
    s.puff.1 = (s.puff.1 - dt).max(0.0);
    if s.puff.1 == 0.0 {
        s.puff.0 = 0.0;
    }
    let breath = g.senses.gust().max(s.puff.0);
    // away from the one breathing: the way the camera looks, along the floor
    let fwd = g.cam.basis().fwd;
    let away = fwd.x.hypot(fwd.z);
    let (ax, az) = if away > 1e-4 { (fwd.x / away, fwd.z / away) } else { (0.0, 0.0) };
    let k = 1.0 - (-TAKE_UP * dt).exp();
    s.wind.0 += (ax * breath * SPEED - s.wind.0) * k;
    s.wind.1 += (az * breath * SPEED - s.wind.1) * k;
    let (dx, dz) = (s.wind.0 * dt, s.wind.1 * dt);
    if g.world.forms.dust.blow(dx, dz) {
        s.moved += dx.hypot(dz);
        if s.moved > FOUND_AFTER {
            g.discover(Discovery::Gust);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::dust::Clump;
    use crate::input::keys;
    use crate::input::senses::{Heard, Kind};

    fn dusted() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        let d = &mut g.world.forms.dust;
        d.phase = Phase::Dust;
        d.clumps[0] = Clump { x: -1.5, z: 0.0, amount: 0.4 };
        d.clumps[1] = Clump { x: 1.5, z: 0.0, amount: 0.3 };
        d.clumps[2] = Clump { x: 0.0, z: 1.0, amount: 0.3 };
        g
    }

    /// How far, on average, the drifts lie away from the camera along the floor.
    fn far(g: &Game) -> f32 {
        let f = g.cam.basis().fwd;
        let n = f.x.hypot(f.z);
        let cs: Vec<_> = g.world.forms.dust.clumps.iter().filter(|c| c.amount > 0.0).collect();
        cs.iter().map(|c| (c.x * f.x + c.z * f.z) / n).sum::<f32>() / cs.len() as f32
    }

    fn step_only(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            step(g, 1.0 / 60.0);
            g.world.forms.dust.step(1.0 / 60.0);
        }
    }

    #[test]
    fn left_alone_the_dust_stays_where_it_lies() {
        let mut g = dusted();
        let was = g.world.forms.dust.clumps;
        step_only(&mut g, 5.0);
        assert_eq!(g.world.forms.dust.clumps[..3], was[..3]);
        assert!(!g.progress.has(Discovery::Gust));
    }

    #[test]
    fn a_breath_let_out_with_space_blows_the_drifts_away_from_you() {
        let mut g = dusted();
        let before = far(&g);
        keys::down(&mut g, "Space");
        for _ in 0..90 {
            g.update(1.0 / 60.0);
        }
        assert!((far(&g) - before).abs() < 1e-4, "drawing breath moves nothing");
        keys::up(&mut g, "Space");
        let mut last = g.world.forms.dust.clumps;
        let mut most = 0.0f32;
        for _ in 0..120 {
            g.update(1.0 / 60.0);
            let now = g.world.forms.dust.clumps;
            for (a, b) in last.iter().zip(&now) {
                most = most.max((a.x - b.x).hypot(a.z - b.z));
            }
            last = now;
        }
        assert!(far(&g) > before + 0.4, "{} -> {}", before, far(&g));
        assert!(most < SPEED / 60.0 + 1e-4, "a drift jumped {most}");
        assert!(g.progress.has(Discovery::Gust));
        // and it settles: the breath gone, they lie still
        let rest = g.world.forms.dust.clumps;
        step_only(&mut g, 1.0);
        let moved: f32 = rest.iter().zip(&g.world.forms.dust.clumps).map(|(a, b)| (a.x - b.x).hypot(a.z - b.z)).sum();
        assert!(moved < 0.1, "{moved}");
    }

    #[test]
    fn blown_at_it_once_it_hears_the_drifts_go_and_run_together() {
        let mut g = dusted();
        g.senses.hear = crate::input::senses::Leave::Open;
        assert!(g.senses.is_open(Kind::Hear));
        let before = far(&g);
        let total: f32 = g.world.forms.dust.clumps.iter().map(|c| c.amount).sum();
        for _ in 0..(8.0 * 60.0) as usize {
            g.senses.heard(Heard { level: 0.12, pitch: None, rough: 0.3, low: 0.8, clarity: 0.2 });
            g.update(1.0 / 60.0);
        }
        assert!(far(&g) > before + 1.0);
        assert!(g.progress.has(Discovery::Gust));
        let d = &g.world.forms.dust;
        let after: f32 = d.clumps.iter().map(|c| c.amount).sum();
        assert!((after - total).abs() < 1e-3, "no dust lost");
        assert!(d.clumps.iter().all(|c| c.x.hypot(c.z) <= 4.2 + 1e-3), "none blown out of sight");
        let lying = d.clumps.iter().filter(|c| c.amount > 0.0).count();
        assert!(lying < 3 || d.phase != Phase::Dust, "piled against the far side, they ran together: {:?}", d.clumps);
    }

    #[test]
    fn breath_on_the_ordinary_liquid_leaves_no_dust_moving() {
        let mut g = Game::new();
        keys::down(&mut g, "Space");
        for _ in 0..60 {
            g.update(1.0 / 60.0);
        }
        keys::up(&mut g, "Space");
        assert_eq!(g.keys.puff, 0.0, "a breath on the liquid is its own");
        g.update(1.0 / 60.0);
        assert_eq!(g.world.forms.gust.wind, (0.0, 0.0));
    }
}
