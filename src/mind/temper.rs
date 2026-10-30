//! How far its moods let your verbs go. Glad of you (remembrance.rs `kind`), it gives a little
//! more: a slower pinch still leaves a bud, a pull holds taut a little longer before it tears,
//! and dust swept together heaps a little sooner. Wary of you, it gives a little less: it tears
//! a little sooner, pulls out of a pinch sooner, and a thread it was drawn into winds itself
//! back if it is left alone.
//!
//! The mood is read once, when a hand comes down (`Game::pointer_down`), and kept for the whole
//! gesture, so no threshold moves while a hand is on it. It blends smoothly from neutral: a
//! ledger that knows nothing of you (warmth exactly 0.5) changes nothing, to the last bit.

use crate::math::smoothstep;
use crate::mind::remembrance::{KIND_AT, WARY_AT};

/// How much further each verb goes, glad of you, and how much less, wary.
const PINCH_GLAD: f32 = 0.15;
const PINCH_WARY: f32 = 0.15;
const TEAR_GLAD: f32 = 0.12;
const TEAR_WARY: f32 = 0.1;
const HEAP_GLAD: f32 = 0.15;

/// Its mood as the verbs need it: how glad and how wary of you, each 0..1, never both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Temper {
    pub glad: f32,
    pub wary: f32,
}

impl Temper {
    /// From how warm it is (`Remembrance::warmth`): nothing at 0.5, all of it at `KIND_AT` or
    /// `WARY_AT` and beyond.
    pub fn of(warmth: f32) -> Temper {
        Temper { glad: smoothstep(0.5, KIND_AT, warmth), wary: smoothstep(0.5, WARY_AT, warmth) }
    }

    /// What the time a pinch may take is multiplied by.
    pub fn pinch(self) -> f32 {
        1.0 + PINCH_GLAD * self.glad - PINCH_WARY * self.wary
    }

    /// What how far a pull goes before it tears is multiplied by.
    pub fn tear(self) -> f32 {
        1.0 + TEAR_GLAD * self.glad - TEAR_WARY * self.wary
    }

    /// What how much dust must lie in one drift before it heaps is multiplied by.
    pub fn heap(self) -> f32 {
        1.0 - HEAP_GLAD * self.glad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_changes_nothing() {
        let t = Temper::of(0.5);
        assert_eq!((t.pinch(), t.tear(), t.heap()), (1.0, 1.0, 1.0));
        assert_eq!(t.wary, 0.0);
        assert_eq!(Temper::default().pinch(), 1.0);
    }

    #[test]
    fn glad_gives_more_and_wary_less_and_it_blends() {
        let (glad, wary) = (Temper::of(0.9), Temper::of(0.1));
        assert!(glad.pinch() > 1.1 && glad.tear() > 1.1 && glad.heap() < 0.9);
        assert!(wary.pinch() < 0.9 && wary.tear() < 0.95 && wary.heap() == 1.0 && wary.wary == 1.0);
        let half = Temper::of(0.55);
        assert!(half.pinch() > 1.0 && half.pinch() < glad.pinch());
    }

    use crate::game::{Game, Sound};
    use crate::mind::remembrance::{Deed, Remembrance};
    use crate::progress::Discovery;

    const T0: f64 = 1_800_000_000.0;

    /// A game whose ledger says `deeds`, `n` times; none for neutral.
    fn game(deed: Option<Deed>, n: u32) -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.remembrance = Remembrance::fresh(T0);
        for _ in 0..n {
            deed.into_iter().for_each(|d| g.remembrance.remember(d));
        }
        g
    }

    fn glad() -> Game {
        let g = game(Some(Deed::Held), 20);
        assert!(g.remembrance.kind());
        g
    }

    fn wary() -> Game {
        let g = game(Some(Deed::Hurt), 10);
        assert!(g.remembrance.wary());
        g
    }

    fn wait(g: &mut Game, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    /// A short tug out from the right of the middle, let go of after `hold` more seconds.
    fn tug(g: &mut Game, hold: f32) {
        g.pointer_down(1, 450.0, 300.0);
        for k in 1..=3 {
            g.pointer_move(1, 450.0 + 14.0 * k as f32, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        wait(g, hold);
        g.pointer_up(1);
        wait(g, 0.1);
    }

    #[test]
    fn glad_of_you_a_slower_pinch_still_buds() {
        let mut g = glad();
        tug(&mut g, 0.38);
        assert!(g.progress.has(Discovery::Pinch));
        let mut g = game(None, 0);
        tug(&mut g, 0.38);
        assert!(!g.progress.has(Discovery::Pinch) && g.sounds.contains(&Sound::Almost));
    }

    #[test]
    fn wary_of_you_it_pulls_out_of_a_pinch_sooner() {
        let mut g = wary();
        tug(&mut g, 0.3);
        assert!(!g.progress.has(Discovery::Pinch));
        let mut g = game(None, 0);
        tug(&mut g, 0.3);
        assert!(g.progress.has(Discovery::Pinch));
    }

    /// How far it was pulled, in body radii, when it tore.
    fn tears_at(mut g: Game) -> f32 {
        g.pointer_down(1, 400.0, 300.0);
        let mut most = 0.0f32;
        for i in 1..=120 {
            g.pointer_move(1, 400.0 + i as f32 * 3.0, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
            if !g.world.drops.is_empty() {
                return most;
            }
            most = g.world.handles.iter().map(|h| h.stretch(&g.world.body)).fold(most, f32::max);
        }
        panic!("it never tore");
    }

    #[test]
    fn glad_it_goes_further_before_it_tears_and_wary_less_far() {
        let (glad, plain, wary) = (tears_at(glad()), tears_at(game(None, 0)), tears_at(wary()));
        assert!(wary < plain - 0.05 && plain < glad - 0.05, "{wary} {plain} {glad}");
    }

    #[test]
    fn glad_dust_heaps_from_less() {
        let mut g = glad();
        g.pointer_down(1, 10.0, 10.0);
        g.pointer_up(1);
        assert!(g.world.forms.dust.heap_at < 0.9);
        let mut g = game(None, 0);
        g.pointer_down(1, 10.0, 10.0);
        g.pointer_up(1);
        assert_eq!(g.world.forms.dust.heap_at, 1.0);
    }

    /// Drawn out into the thread, and let go; whether it is still a thread `after` seconds later.
    fn thread_after(mut g: Game, after: f32) -> bool {
        g.pointer_down(1, 400.0, 300.0);
        for k in 1..=20 {
            g.pointer_move(1, 400.0 + 7.5 * k as f32, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        wait(&mut g, 1.5);
        g.pointer_up(1);
        wait(&mut g, 3.0);
        assert!(g.progress.has(Discovery::Thread), "{:?}", g.world.forms.filament.phase);
        wait(&mut g, after);
        !g.world.forms.filament.whole()
    }

    #[test]
    fn wary_a_thread_left_alone_winds_itself_back() {
        assert!(!thread_after(wary(), 15.0));
        assert!(thread_after(game(None, 0), 15.0));
        assert!(thread_after(glad(), 15.0));
    }

    #[test]
    fn the_mood_is_read_when_the_hand_comes_down_and_kept() {
        let mut g = glad();
        g.pointer_down(1, 450.0, 300.0);
        for k in 1..=3 {
            g.pointer_move(1, 450.0 + 14.0 * k as f32, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        // hurt while the hand is on it: this gesture goes as far as it did when it began
        for _ in 0..20 {
            g.remembrance.remember(Deed::Hurt);
        }
        assert!(!g.remembrance.kind());
        wait(&mut g, 0.38);
        g.pointer_up(1);
        wait(&mut g, 0.1);
        assert!(g.progress.has(Discovery::Pinch));
    }
}
