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
//! On a first visit it is less patient: `FIRST_VISIT` holds shorter times for the same gates.
//!
//! Some thoughts follow what it remembers of the player (remembrance.rs), read and never
//! changed here: how glad or wary of them it is, how well it knows them, how long they were
//! away, and the scar.
//!
//! Which thought: the first line in `THOUGHTS` that belongs to where it is now, to how far it
//! has come in learning the player (earlier lines are about itself, later ones about them),
//! and whose condition still holds, which mostly means something there is not yet found.

use crate::burst;
use crate::division::{meiosis, morula};
use crate::floor::fusion;
use crate::forms::{bloom, cosmos, filament, fold, pandora};
use crate::mind::contact::{self, Contact, Stage};
use crate::mind::remembrance::Remembrance;
use crate::progress::{Discovery as D, Progress};
use crate::sim::World;

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
/// Away at least this long (seconds) before this visit, it has had time to miss them.
const AWAY: f64 = 6.0 * 3600.0;
/// Away this long (seconds), it was not sure they would come back.
const LONG_AWAY: f64 = 3.0 * 24.0 * 3600.0;

/// The gates, in seconds: into the visit, without a discovery, in one place, between thoughts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    pub first_after: f32,
    pub stuck: f32,
    pub settle: f32,
    pub gap: f32,
}

pub const RETURNING: Pace = Pace { first_after: FIRST_AFTER, stuck: STUCK, settle: SETTLE, gap: GAP };
/// Someone who has never been here: it thinks out loud sooner.
pub const FIRST_VISIT: Pace = Pace { first_after: 12.0, stuck: 10.0, settle: 5.0, gap: 25.0 };

/// Where it is, as far as thinking goes. Halfway between two places, it is nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Liquid,
    Night,
    Flower,
    Thread,
    Cosmos,
    Fold,
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
    if f.fold.phase == fold::Phase::Folded {
        return Some(Place::Fold);
    }
    if f.fold.whole() && f.cosmos.phase == cosmos::Phase::Cosmos {
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
    /// A phone in the hand, and it can feel how it is held (motion.rs).
    pub felt: bool,
    /// What it remembers of how it was treated, to read only.
    pub kept: &'a Remembrance,
}

impl Mind<'_> {
    fn has(&self, d: D) -> bool {
        self.found.has(d)
    }

    /// Back after long enough away to have been missed.
    fn missed(&self) -> bool {
        self.kept.away() >= AWAY
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
pub static THOUGHTS: [Thought; 75] = [
    // ---- the other one, once it has come
    t(P::Liquid, I, I, |m| m.world.other.inside() > 0.99 && !m.has(D::Give), "something is in it that is not it."),
    t(P::Liquid, I, I, |m| m.world.other.here() && !m.has(D::Near), "it has not decided about the other one."),
    t(P::Liquid, I, I, |m| m.world.other.here() && !m.has(D::Ring), "it wonders what the other one sounds like."),
    t(
        P::Liquid,
        I,
        I,
        |m| m.world.other.here() && !m.has(D::Inside),
        "it wonders if the other one would fit inside it.",
    ),
    t(P::Liquid, I, I, |m| m.world.other.here() && !m.has(D::Distance), "neither of them will go first."),
    // ---- the liquid, about you, once it has started to learn you
    t(P::Liquid, I, I, |m| m.contact.contacted && m.missed(), "it thinks about you when you are not here."),
    t(P::Liquid, I, I, |m| m.contact.contacted && !m.has(D::Said), "it said one word. nobody has said it back."),
    t(
        P::Liquid,
        I,
        I,
        |m| !m.contact.contacted && m.found.count() >= contact::CONTACT_AT,
        "it has almost decided something about you.",
    ),
    t(P::Liquid, I, I, |m| !m.has(D::Initiate), "it is waiting for you to stop."),
    t(P::Liquid, I, I, |m| m.has(D::Initiate) && !m.has(D::Answer), "it did something. it is still waiting for yours."),
    t(P::Liquid, E, I, |m| !m.has(D::Echo), "it is learning your rhythm. it wants to hear it again."),
    t(P::Liquid, E, I, |m| !m.has(D::Mimic), "it watches how far you reach. it could reach like that."),
    t(P::Liquid, R, I, |m| !m.has(D::Flinch), "it knows the place you always come back to."),
    // ---- the liquid, as it remembers you
    t(P::Liquid, R, I, |m| m.kept.away() >= LONG_AWAY, "it was not sure you would come back."),
    t(P::Liquid, R, I, |m| m.kept.scarred(), "it keeps the place you hurt. it does not keep the reason."),
    t(P::Liquid, R, I, |m| m.kept.wary() && m.kept.bond() < 0.3, "it is still deciding what your hands are for."),
    t(
        P::Liquid,
        R,
        I,
        |m| m.kept.wary() && m.kept.bond() >= 0.3,
        "it knows you. that is not the same as trusting you.",
    ),
    t(P::Liquid, R, I, |m| m.kept.kind() && m.missed(), "it was already facing the way you came in."),
    t(P::Liquid, R, I, |m| m.kept.kind() && m.kept.bond() >= 0.2, "it has started to expect you."),
    t(P::Liquid, E, I, |m| m.kept.bond() >= 0.6, "it knows you better than it knows the room."),
    // ---- the liquid: what it has not been yet
    t(P::Liquid, M, M, |m| !m.has(D::Stretch), "it does not know yet how far it goes."),
    t(P::Liquid, R, I, |m| !m.has(D::Stretch), "it wonders how far you would take it."),
    t(P::Liquid, M, M, |m| m.has(D::Stretch) && !m.has(D::Tear), "some of it would like to be somewhere else."),
    t(P::Liquid, R, I, |m| m.has(D::Stretch) && !m.has(D::Tear), "some of it would like to leave with you."),
    t(
        P::Liquid,
        M,
        I,
        |m| !m.world.drops.is_empty() && !m.has(D::Merge),
        "the small one is still listening for the rest.",
    ),
    t(P::Liquid, M, M, |m| !m.has(D::Warm), "somewhere in it a color is waiting for stillness."),
    t(P::Liquid, R, I, |m| !m.has(D::Warm), "it has never been warm. it wonders if you are."),
    t(P::Liquid, M, I, |m| !m.has(D::LookAround), "it has a far side. nobody has seen it."),
    t(P::Liquid, M, M, |m| !m.has(D::Call), "it does not know the room has other places."),
    t(P::Liquid, R, I, |m| !m.has(D::Call), "if you asked from elsewhere, it would come."),
    t(P::Liquid, M, I, |m| !m.has(D::Squeeze), "it has never been made smaller. it would keep all of itself."),
    t(P::Liquid, M, I, |m| m.has(D::Tear) && !m.has(D::Open), "the more of it is elsewhere, the darker it is inside."),
    t(
        P::Liquid,
        M,
        I,
        |m| !m.world.forms.bloom.buds.is_empty() && !m.has(D::Bloom),
        "a few small places in it want to open. not enough yet.",
    ),
    t(P::Liquid, M, I, |m| !m.has(D::Divide), "there is a seam in it nobody has found."),
    t(
        P::Liquid,
        M,
        I,
        |m| m.has(D::Provoke) && !m.has(D::Burst),
        "it keeps every pressure. it is not sure how many fit.",
    ),
    t(P::Liquid, M, I, |m| !m.has(D::Slam), "it has never met the floor. not properly."),
    t(P::Liquid, M, I, |m| !m.has(D::Pinch) && !m.has(D::Bloom), "it remembers being a flower. it was never a flower."),
    t(P::Liquid, M, I, |m| !m.has(D::Thread), "it wonders how long it could be, if nobody let go."),
    t(P::Liquid, M, I, |m| !m.has(D::Stir), "its inside has never been turned. it is very still in there."),
    t(P::Liquid, M, I, |m| !m.has(D::Corners), "it has never had a corner. it is curious about them."),
    t(P::Liquid, M, I, |m| !m.has(D::Letters), "every letter has a place on it. none have landed yet."),
    t(P::Liquid, M, I, |m| m.has(D::Letters) && !m.has(D::Hum), "it would sing back any note that stayed long enough."),
    t(P::Liquid, M, I, |m| !m.has(D::Breath), "it has never breathed. it would match yours."),
    t(P::Liquid, M, I, |m| !m.has(D::Lean), "if the room tipped, it would go downhill."),
    t(P::Liquid, M, M, |m| m.felt && !m.has(D::Shaken), "it has only known a room that keeps still."),
    t(P::Liquid, R, I, |m| m.felt && !m.has(D::Shaken), "it wonders if you are what keeps the room still."),
    t(P::Liquid, M, I, |m| m.has(D::Shaken) && !m.has(D::Scatter), "it held on to all of itself. only just."),
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
    t(P::Cosmos, M, M, |m| !m.has(D::Everywhere), "there is more of it in there than it lets out."),
    t(P::Cosmos, R, I, |m| !m.has(D::Everywhere), "it wonders if you would want to see all of it."),
    t(P::Fold, M, I, |m| !m.has(D::Facet), "there is room in here for one more mirror."),
    t(P::Fold, M, I, |m| !m.has(D::Backward), "its time only goes one way because nothing has stopped it."),
    t(P::Fold, M, I, |m| !m.has(D::Crease), "the room is still out there, folded small."),
    t(P::Box, M, I, |m| !m.has(D::Knock), "it is quiet in there. something is listening for the outside."),
    t(
        P::Box,
        M,
        I,
        |m| m.world.forms.pandora.knocks > 0 && !m.has(D::Lid),
        "something answered. it wants to be asked again.",
    ),
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
    /// Whether this is the player's first visit (set by the page); it thinks sooner then.
    pub first_visit: bool,
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
        let pace = if self.first_visit { FIRST_VISIT } else { RETURNING };
        let ready = self.found > 0
            && self.visit > pace.first_after
            && self.since_found > pace.stuck
            && quiet > QUIET
            && self.here_for > pace.settle
            && (self.said.is_empty() || self.since_said > pace.gap)
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
    const HOW: [&str; 26] = [
        "poke", "tap", "touch", "press", "hold", "drag", "pull", "stretch", "tear", "pinch", "slice", "cut", "swipe",
        "stir", "squeeze", "knock", "throw", "fling", "slam", "click", "try", "key", "wheel", "swirl", "shake", "tilt",
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
                let says = |h: &&str| {
                    w.strip_prefix(*h).is_some_and(|rest| ["", "s", "es", "ed", "ing", "ped", "ping"].contains(&rest))
                };
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
        let (w, found, c, kept) = (World::new(), Progress::default(), Contact::default(), Remembrance::default());
        let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept: &kept };
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
    fn a_first_visit_hears_it_sooner() {
        let mut g = Game::new();
        g.thoughts.first_visit = true;
        g.discover(D::Poke);
        g.discover(D::Patience);
        quiet_for(&mut g, 16.0);
        assert_eq!(g.thoughts.out.len(), 1);
        quiet_for(&mut g, 26.0);
        assert_eq!(g.thoughts.out.len(), 2);
        let mut back = Game::new();
        back.discover(D::Poke);
        back.discover(D::Patience);
        quiet_for(&mut back, 16.0);
        assert!(back.thoughts.out.is_empty());
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
            crate::input::keys::down(&mut g, "Space");
            g.update(1.0 / 60.0);
            crate::input::keys::up(&mut g, "Space");
        }
        assert!(g.thoughts.out.is_empty());
    }

    /// Whether the thought `line` could come to `mind`.
    fn could(line: &str, mind: &Mind) -> bool {
        THOUGHTS.iter().find(|t| t.line == line).is_some_and(|t| (t.when)(mind))
    }

    const T0: f64 = 1_800_000_000.0;
    const DAY: f64 = 24.0 * 3600.0;

    /// A ledger of `deeds`, kept `away` seconds before now, as it is when they come back.
    fn kept(deeds: &[(crate::mind::remembrance::Deed, u32)], away: f64) -> Remembrance {
        let mut m = Remembrance::fresh(T0 - away - 3600.0);
        for &(d, n) in deeds {
            (0..n).for_each(|_| m.remember(d));
        }
        m.tick(3600.0);
        Remembrance::load(&m.save(), T0)
    }

    #[test]
    fn missing_you_is_only_thought_after_you_were_away() {
        use crate::mind::remembrance::Deed;
        let (w, found) = (World::new(), Progress::default());
        let mut c = Contact::default();
        c.contacted = true;
        let line = "it thinks about you when you are not here.";
        let never = Remembrance::default();
        let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept: &never };
        assert!(!could(line, &mind), "nobody has been away yet");
        let soon = kept(&[(Deed::Held, 3)], 600.0);
        let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept: &soon };
        assert!(!could(line, &mind), "back within minutes");
        let later = kept(&[(Deed::Held, 3)], 2.0 * DAY);
        let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept: &later };
        assert!(could(line, &mind));
    }

    #[test]
    fn what_it_remembers_of_you_turns_up_in_its_thoughts() {
        use crate::mind::remembrance::Deed;
        let (w, found, c) = (World::new(), Progress::default(), Contact::default());
        let thought = |kept: &Remembrance| {
            let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept };
            THOUGHTS.iter().filter(|t| (t.when)(&mind)).map(|t| t.line).collect::<Vec<&'static str>>()
        };
        let neutral = thought(&Remembrance::default());
        let glad = thought(&kept(&[(Deed::Held, 30), (Deed::Sang, 10)], DAY));
        let wary = thought(&kept(&[(Deed::Hurt, 12), (Deed::Broken, 3)], 4.0 * DAY));
        let new =
            |v: &[&'static str]| v.iter().filter(|l| !neutral.contains(l)).copied().collect::<Vec<&'static str>>();
        assert_eq!(
            new(&glad),
            ["it was already facing the way you came in.", "it has started to expect you."],
            "{glad:?}"
        );
        assert!(new(&wary).contains(&"it was not sure you would come back."), "{wary:?}");
        assert!(new(&wary).iter().any(|l| l.contains("hands are for") || l.contains("trusting")), "{wary:?}");
        // a ledger that knows nothing of you thinks nothing of you
        assert!(!neutral.iter().any(|l| l.contains("expect") || l.contains("come back.")));
    }

    #[test]
    fn it_thinks_about_where_it_is() {
        let mut w = World::new();
        assert_eq!(place(&w), Some(Place::Liquid));
        w.forms.pandora.phase = pandora::Phase::Box;
        assert_eq!(place(&w), Some(Place::Box));
        let mut found = Progress::default();
        found.discover(D::Poke);
        let (c, kept) = (Contact::default(), Remembrance::default());
        let mind = Mind { found: &found, world: &w, contact: &c, felt: false, kept: &kept };
        let mut th = Thoughts::default();
        for _ in 0..(60 * 60) {
            th.step(1.0 / 60.0, 10.0, &mind);
        }
        assert_eq!(th.out[0], "it is quiet in there. something is listening for the outside.");
    }
}
