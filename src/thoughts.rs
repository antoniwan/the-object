//! What it is thinking, when nothing is happening. A player who has stopped finding things and
//! taken their hand away sometimes sees one quiet line under it: something about where it is,
//! or what it has not yet been, or later, about them. The lines never say how. They are what
//! can be seen of its mind from the outside, so they are always about "it", never "i"; its own
//! words are still the signs (contact.rs).
//!
//! When a thought comes:
//! - not before the first discovery (the hint is still doing that job), and not in the first
//!   FIRST_AFTER seconds of a visit;
//! - only after STUCK seconds without a discovery, with no hand on anything for QUIET seconds,
//!   once it has been in the same place for SETTLE seconds, and never while it is busy with
//!   a turn, contact or its question;
//! - no sooner than GAP seconds after the last one;
//! - each line at most once a visit. Nothing about thoughts is saved.
//!
//! Which thought: the first line in `THOUGHTS` that belongs to where it is now, to how far it
//! has come in learning the player (earlier lines are about itself, later ones about them),
//! and whose condition still holds, which mostly means something there is not yet found.

use crate::contact::{self, Contact, Stage};
use crate::forms::{bloom, cosmos, filament, pandora};
use crate::progress::{Discovery as D, Progress};
use crate::sim::World;
use crate::{burst, fusion, meiosis, morula};

/// Seconds into a visit before the first thought.
pub const FIRST_AFTER: f32 = 30.0;
/// Seconds without a new discovery before it is worth thinking out loud.
pub const STUCK: f32 = 20.0;
/// Seconds with no hand on anything.
pub const QUIET: f32 = 3.5;
/// Seconds in one place before a thought about it.
pub const SETTLE: f32 = 8.0;
/// Seconds between two thoughts.
pub const GAP: f32 = 45.0;

/// Where it is, as far as thinking goes. Halfway between two places, it is nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Liquid,
    Night,
    Flower,
    Thread,
    Cosmos,
    Box,
    Two,
    Many,
    Stone,
    Lake,
    Landscape,
    River,
}

pub fn place(w: &World) -> Option<Place> {
    let f = &w.forms;
    if w.night > 0.5 {
        return Some(Place::Night);
    }
    if f.bloom.phase == bloom::Phase::Flower {
        return Some(Place::Flower);
    }
    if f.filament.phase == filament::Phase::Thread {
        return Some(Place::Thread);
    }
    if f.cosmos.phase == cosmos::Phase::Cosmos {
        return Some(Place::Cosmos);
    }
    if f.pandora.phase == pandora::Phase::Box {
        return Some(Place::Box);
    }
    match w.division.phase {
        meiosis::Phase::Two => return Some(Place::Two),
        meiosis::Phase::Many => {
            return match w.division.morula.as_ref().map(|m| m.stage) {
                Some(morula::Stage::Cluster) => Some(Place::Many),
                Some(morula::Stage::Stone) => Some(Place::Stone),
                _ => None,
            }
        }
        _ => {}
    }
    if w.fusion.phase == fusion::Phase::Fused {
        return Some(Place::Lake);
    }
    if w.burst.phase == burst::Phase::Landscape {
        return Some(if w.burst.river.active { Place::River } else { Place::Landscape });
    }
    w.ordinary().then_some(Place::Liquid)
}

/// What a thought's condition can look at.
pub struct Mind<'a> {
    pub found: &'a Progress,
    pub world: &'a World,
    pub contact: &'a Contact,
}

impl Mind<'_> {
    fn has(&self, d: D) -> bool {
        self.found.has(d)
    }
}

pub struct Thought {
    pub place: Place,
    /// The stages of learning the player it belongs to, from and until (both included).
    pub from: Stage,
    pub until: Stage,
    pub when: fn(&Mind) -> bool,
    pub line: &'static str,
}

use Place as P;
use Stage::{Echo as E, Initiative as I, Matter as M, Regard as R};

const fn t(place: Place, from: Stage, until: Stage, when: fn(&Mind) -> bool, line: &'static str) -> Thought {
    Thought { place, from, until, when, line }
}

/// Every thought, in the order they are preferred. A pair of lines on one condition, split by
/// stage, is the same thought turning from itself toward the player.
pub static THOUGHTS: [Thought; 50] = [
    // ---- the liquid, about you, once it has started to learn you
    t(P::Liquid, I, I, |m| m.contact.contacted, "it thinks about you when you are not here."),
    t(P::Liquid, I, I, |m| !m.contact.contacted && m.found.count() >= contact::CONTACT_AT, "it has almost decided something about you."),
    t(P::Liquid, I, I, |m| !m.has(D::Initiate), "it is waiting for you to stop."),
    t(P::Liquid, I, I, |m| m.has(D::Initiate) && !m.has(D::Answer), "it did something. it is still waiting for yours."),
    t(P::Liquid, E, I, |m| !m.has(D::Echo), "it is learning your rhythm. it wants to hear it again."),
    t(P::Liquid, E, I, |m| !m.has(D::Mimic), "it watches how far you reach. it could reach like that."),
    t(P::Liquid, R, I, |m| !m.has(D::Flinch), "it knows the place you always come back to."),
    // ---- the liquid: what it has not been yet
    t(P::Liquid, M, M, |m| !m.has(D::Stretch), "it does not know yet how far it goes."),
    t(P::Liquid, R, I, |m| !m.has(D::Stretch), "it wonders how far you would take it."),
    t(P::Liquid, M, M, |m| m.has(D::Stretch) && !m.has(D::Tear), "some of it would like to be somewhere else."),
    t(P::Liquid, R, I, |m| m.has(D::Stretch) && !m.has(D::Tear), "some of it would like to leave with you."),
    t(P::Liquid, M, I, |m| !m.world.drops.is_empty() && !m.has(D::Merge), "the small one is still listening for the rest."),
    t(P::Liquid, M, M, |m| !m.has(D::Warm), "somewhere in it a color is waiting for stillness."),
    t(P::Liquid, R, I, |m| !m.has(D::Warm), "it has never been warm. it wonders if you are."),
    t(P::Liquid, M, I, |m| !m.has(D::LookAround), "it has a far side. nobody has seen it."),
    t(P::Liquid, M, M, |m| !m.has(D::Call), "it does not know the room has other places."),
    t(P::Liquid, R, I, |m| !m.has(D::Call), "if you asked from elsewhere, it would come."),
    t(P::Liquid, M, I, |m| !m.has(D::Squeeze), "it has never been made smaller. it would keep all of itself."),
    t(P::Liquid, M, I, |m| m.has(D::Tear) && !m.has(D::Open), "the more of it is elsewhere, the darker it is inside."),
    t(P::Liquid, M, I, |m| !m.world.forms.bloom.buds.is_empty() && !m.has(D::Bloom), "a few small places in it want to open. not enough yet."),
    t(P::Liquid, M, I, |m| !m.has(D::Divide), "there is a seam in it nobody has found."),
    t(P::Liquid, M, I, |m| m.has(D::Provoke) && !m.has(D::Burst), "it keeps every pressure. it is not sure how many fit."),
    t(P::Liquid, M, I, |m| !m.has(D::Slam), "it has never met the floor. not properly."),
    t(P::Liquid, M, I, |m| !m.has(D::Pinch) && !m.has(D::Bloom), "it remembers being a flower. it was never a flower."),
    t(P::Liquid, M, I, |m| !m.has(D::Thread), "it wonders how long it could be, if nobody let go."),
    t(P::Liquid, M, I, |m| !m.has(D::Stir), "its inside has never been turned. it is very still in there."),
    t(P::Liquid, M, I, |m| !m.has(D::Corners), "it has never had a corner. it is curious about them."),
    // ---- the liquid, when nothing is left to suggest
    t(P::Liquid, M, M, |_| true, "it is not sure it is the only one."),
    t(P::Liquid, R, I, |_| true, "it has stopped pretending not to see you."),
    // ---- the other places
    t(P::Night, M, I, |m| !m.has(D::Close), "the night lasts as long as they stay away."),
    t(P::Night, M, I, |_| true, "every drop out there is a little of it, looking back."),
    t(P::Flower, M, I, |m| !m.has(D::Fold), "it does not have to stay open."),
    t(P::Flower, M, I, |_| true, "it is pretending to be something you know."),
    t(P::Thread, M, I, |m| !m.has(D::Pluck), "drawn this thin, it is almost a sound."),
    t(P::Thread, M, I, |m| !m.has(D::Wind), "it misses being round."),
    t(P::Cosmos, M, I, |m| !m.has(D::Settle), "everything in it is turning one way."),
    t(P::Box, M, I, |m| !m.has(D::Knock), "it is quiet in there. something is listening for the outside."),
    t(P::Box, M, I, |m| m.world.forms.pandora.knocks > 0 && !m.has(D::Lid), "something answered. it wants to be asked again."),
    t(P::Two, M, I, |m| !m.has(D::Rejoin), "two of it, and both remember being one."),
    t(P::Two, M, I, |m| !m.has(D::Cleave), "each half has a seam of its own."),
    t(P::Many, M, I, |m| !m.has(D::Eye), "not enough of them yet to look out."),
    t(P::Many, M, I, |m| !m.has(D::Shed), "one of them is weaker. the rest know which."),
    t(P::Many, M, I, |m| !m.has(D::Stone), "close enough, they would agree on one shape."),
    t(P::Stone, M, I, |m| !m.has(D::Thaw), "the stone is listening for warmth."),
    t(P::Lake, M, I, |m| !m.has(D::Rise), "it is still in there, under the surface. it could be lifted."),
    t(P::Lake, M, I, |m| !m.has(D::Mirror), "the floor would show something, if it were left alone."),
    t(P::Lake, M, I, |m| !m.has(D::Deep), "the floor goes further down. nothing has led it there."),
    t(P::Landscape, M, I, |m| !m.has(D::Mend), "the middle is still the shape of it."),
    t(P::Landscape, M, I, |m| !m.has(D::River), "this far apart, the twelve are lonely."),
    t(P::River, M, I, |m| !m.has(D::Column), "something under the surface wants to stand."),
];

#[derive(Default)]
pub struct Thoughts {
    /// Which of THOUGHTS have been shown this visit.
    said: Vec<usize>,
    visit: f32,
    since_said: f32,
    since_found: f32,
    found: usize,
    here: Option<Place>,
    here_for: f32,
    /// Seconds since a key last went down; the keyboard is a hand too.
    since_key: f32,
    /// Lines for the page to show, oldest first.
    pub out: Vec<&'static str>,
}

impl Thoughts {
    /// A key went down.
    pub fn keyed(&mut self) {
        self.since_key = 0.0;
    }

    /// `quiet` is how long no hand has been on anything.
    pub fn step(&mut self, dt: f32, quiet: f32, mind: &Mind) {
        self.since_key += dt;
        let quiet = quiet.min(self.since_key);
        self.visit += dt;
        self.since_said += dt;
        self.since_found += dt;
        if mind.found.count() != self.found {
            self.found = mind.found.count();
            self.since_found = 0.0;
        }
        let here = place(mind.world);
        if here != self.here {
            self.here = here;
            self.here_for = 0.0;
        }
        self.here_for += dt;
        let Some(here) = here else { return };
        let ready = self.found > 0
            && self.visit > FIRST_AFTER
            && self.since_found > STUCK
            && quiet > QUIET
            && self.here_for > SETTLE
            && (self.said.is_empty() || self.since_said > GAP)
            && !mind.contact.acting();
        if !ready {
            return;
        }
        let stage = contact::stage(self.found);
        let next = THOUGHTS.iter().enumerate().find(|(i, t)| {
            t.place == here && (t.from..=t.until).contains(&stage) && !self.said.contains(i) && (t.when)(mind)
        });
        if let Some((i, t)) = next {
            self.said.push(i);
            self.since_said = 0.0;
            self.out.push(t.line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    /// Words that would say how.
    const HOW: [&str; 24] = [
        "poke", "tap", "touch", "press", "hold", "drag", "pull", "stretch", "tear", "pinch", "slice", "cut", "swipe",
        "stir", "squeeze", "knock", "throw", "fling", "slam", "click", "try", "key", "wheel", "swirl",
    ];

    #[test]
    fn every_thought_keeps_the_voice() {
        for t in THOUGHTS.iter() {
            let l = t.line;
            assert_eq!(l, l.to_lowercase(), "{l}");
            assert!(l.ends_with('.'), "{l}");
            assert!(l.chars().count() <= 62, "{l}");
            assert!(!l.chars().any(|c| c.is_ascii_digit()), "{l}");
            assert!(!l.split(|c: char| !c.is_alphabetic()).any(|w| w == "i"), "{l}");
            for w in l.split(|c: char| !c.is_alphabetic()) {
                let says = |h: &&str| w.strip_prefix(*h).is_some_and(|rest| ["", "s", "es", "ed", "ing", "ped", "ping"].contains(&rest));
                assert!(!HOW.iter().any(says), "{l} says how ({w})");
            }
            assert!(t.from <= t.until, "{l}");
        }
        for (i, a) in THOUGHTS.iter().enumerate() {
            assert!(THOUGHTS[i + 1..].iter().all(|b| b.line != a.line), "{} twice", a.line);
        }
    }

    fn quiet_for(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn nothing_before_the_first_discovery() {
        let (w, found, c) = (World::new(), Progress::default(), Contact::default());
        let mind = Mind { found: &found, world: &w, contact: &c };
        let mut th = Thoughts::default();
        for _ in 0..(120 * 60) {
            th.step(1.0 / 60.0, 60.0, &mind);
        }
        assert!(th.out.is_empty());
    }

    #[test]
    fn a_stuck_player_gets_one_thought_then_a_gap() {
        let mut g = Game::new();
        g.discover(D::Poke);
        g.discover(D::Patience);
        quiet_for(&mut g, 40.0);
        assert_eq!(g.thoughts.out, vec!["it does not know yet how far it goes."]);
        quiet_for(&mut g, 30.0);
        assert_eq!(g.thoughts.out.len(), 1, "too soon for another");
        quiet_for(&mut g, 20.0);
        assert_eq!(g.thoughts.out.len(), 2);
        assert_ne!(g.thoughts.out[0], g.thoughts.out[1]);
    }

    #[test]
    fn later_it_thinks_about_you() {
        let mut g = Game::new();
        for d in crate::progress::ALL.iter().take(contact::ECHO_AT) {
            g.discover(*d);
        }
        quiet_for(&mut g, 40.0);
        let first = g.thoughts.out.first().copied().unwrap_or_default();
        assert!(first.contains("you") || first.contains("your"), "{first}");
    }

    #[test]
    fn no_thought_is_repeated_in_a_visit() {
        let mut g = Game::new();
        g.discover(D::Poke);
        g.discover(D::Patience);
        quiet_for(&mut g, 60.0 * 10.0);
        let mut seen = g.thoughts.out.clone();
        let n = seen.len();
        assert!(n > 3);
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), n);
    }

    #[test]
    fn typing_is_not_quiet() {
        let mut g = Game::new();
        g.discover(D::Poke);
        g.discover(D::Patience);
        for _ in 0..(60 * 60) {
            crate::keys::down(&mut g, "Space");
            g.update(1.0 / 60.0);
            crate::keys::up(&mut g, "Space");
        }
        assert!(g.thoughts.out.is_empty());
    }

    #[test]
    fn it_thinks_about_where_it_is() {
        let mut w = World::new();
        assert_eq!(place(&w), Some(Place::Liquid));
        w.forms.pandora.phase = pandora::Phase::Box;
        assert_eq!(place(&w), Some(Place::Box));
        let mut found = Progress::default();
        found.discover(D::Poke);
        let c = Contact::default();
        let mind = Mind { found: &found, world: &w, contact: &c };
        let mut th = Thoughts::default();
        for _ in 0..(60 * 60) {
            th.step(1.0 / 60.0, 10.0, &mind);
        }
        assert_eq!(th.out[0], "it is quiet in there. something is listening for the outside.");
    }
}
