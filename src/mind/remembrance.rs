//! Remembrance: what it remembers of what you did to it, after Lifesworn's Remembrance (as
//! the blinki in unblinking-desktop keeps it). Every visit it watches what happens to it and
//! keeps a ledger of deeds; how it is with you is worked out from that ledger, fresh each time.
//!
//! - **The ledger is the truth.** Only deeds are saved: how many of each, ever, and a
//!   "recent" sum that fades. How warm or wary it is never gets saved; `warmth_of` works it
//!   out from the ledger, so changing the formula (and `FORMULA`) changes every visitor at
//!   once, with nothing to migrate.
//! - **Nothing is shown.** No numbers, no currency, nothing spent. The player only sees how
//!   it is when they come back: it leans toward a hand that was kind to it and shies from one
//!   that was not, once a visit.
//! - **Hurt fades sooner than care.** Being upset or broken is remembered for less long than
//!   being held, sung to or answered, so a gentle visit or two outweighs a rough one. Every
//!   deed, hard or soft, still adds to how well it knows you (`bond`), and that never shrinks.
//! - **Time comes in from outside.** The page hands it the clock at load (`load`), and every
//!   frame adds `dt`; tests play out days in a few lines.
//! - **A broken entry is skipped, never a crash.** Anything unreadable starts that deed over.
//!
//! It watches without being told: `step` reads what the frame already says (the sounds it
//! made, how upset it is, whether it is being held, hummed to or sung back to, whether its
//! question was answered), so no verb has to call it. Other code can still add a deed with
//! `remember`.
//!
//! - **It learns your note.** Beside the deeds it keeps one number: the note you tend to sing,
//!   in Hz, a slow running middle of every note you held and let go (`sung`). Never a sound,
//!   only that number; a save without it loads with none.
//! - **It keeps where you last touched it**, one direction (`spot`), so that what it shows of
//!   your time away is where your hand was (absence.rs). A save without it loads with none.
//! - **A long time away is not only care.** Back after days, it counts the return, a little, and
//!   is a little more careful of you for the first minutes of the visit (`reserve`), never longer.

use crate::game::{Game, Sound};
use crate::math::*;
use crate::mind::contact::Stage;
use crate::progress::Discovery;

/// Bump when `warmth_of` or `bond_of` change meaning. Saved for whoever reads the value; never
/// used to read it back.
pub const FORMULA: u32 = 2;
const FILE: u32 = 1;

/// An hour and a day, in seconds.
pub const HOUR: f64 = 3600.0;
pub const DAY: f64 = 24.0 * HOUR;
/// Away longer than this, a return counts as a long one.
pub const LONG_AWAY: f64 = 3.0 * DAY;
/// Back after a long time away, it is this much less glad of you at first (of warmth's 0..1),
/// and that fades away at this rate a second: gone within the first few minutes of the visit.
const RESERVE: f32 = 0.1;
const RESERVE_RATE: f32 = 1.0 / 60.0;
/// How many lifetime points before it trusts its own memory fully; until then it stays near
/// neutral, so one visit is never the whole of you.
const FAMILIAR: f64 = 10.0;
/// Lifetime points for the bond to reach about two thirds.
const BOND_SCALE: f64 = 150.0;
/// At or past this, it is glad of you; at or under `WARY_AT`, it is careful of you.
pub const KIND_AT: f32 = 0.6;
pub const WARY_AT: f32 = 0.4;
/// Upset past this is a hurt; it has to calm below `CALM` before another one counts.
const HURT: f32 = 0.6;
const CALM: f32 = 0.2;
/// A hold this long (seconds) is a deed of its own; so is this long being here, untroubled.
const HELD_SECS: f32 = 1.2;
const STAY_SECS: f32 = 45.0;
/// It writes itself down at least this often (seconds) while you are here.
const SAVE_EVERY: f32 = 30.0;
/// Hurt this much, ever (a break counts twice), and it scars (wound.rs).
const SCAR_AT: f64 = 8.0;
/// How near (in body radii, on the plane through it) a pointer has to come to be greeted.
const GREET_NEAR: f32 = 1.8;
/// The notes it keeps of you lie within these (Hz); anything else read back is none.
const PITCH_LO: f32 = 50.0;
const PITCH_HI: f32 = 1200.0;
/// How far one note sung moves the note it knows you by, at most, in octaves: a semitone. Moved
/// by at most a step each way, it settles on the middle of what you sing, and one stray note
/// moves it little.
const PITCH_STEP: f32 = 1.0 / 12.0;

/// Something that happened to it, as it saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deed {
    /// Touched at all: a poke, a press. Neither kind nor unkind; it is how it gets to know you.
    Touch,
    /// Held still against it long enough to warm it.
    Held,
    /// A letter held until it hummed the note back.
    Hummed,
    /// A note sung to it, sung back.
    Sang,
    /// Its turn, answered.
    Answered,
    /// Came back.
    Returned,
    /// Came back after days away. Always also a `Returned`. Counted as a return, a little more
    /// than a short one, and for a while it is careful of you (`reserve`): not pure care.
    LongReturn,
    /// A while here together with nothing hurting it.
    Stayed,
    /// Upset: poked past bearing, shouted at, shaken, struck.
    Hurt,
    /// Poked to death.
    Broken,
    /// Greeted, out loud (words.rs).
    Greeted,
    /// Something carried into an open wound, and kept (wound.rs).
    Tended,
}

impl Deed {
    pub const COUNT: usize = 12;
    pub const ALL: [Deed; Deed::COUNT] = [
        Deed::Touch,
        Deed::Held,
        Deed::Hummed,
        Deed::Sang,
        Deed::Answered,
        Deed::Returned,
        Deed::LongReturn,
        Deed::Stayed,
        Deed::Hurt,
        Deed::Broken,
        Deed::Greeted,
        Deed::Tended,
    ];

    /// Its name in the saved string.
    pub fn key(self) -> &'static str {
        match self {
            Deed::Touch => "touch",
            Deed::Held => "held",
            Deed::Hummed => "hummed",
            Deed::Sang => "sang",
            Deed::Answered => "answered",
            Deed::Returned => "returned",
            Deed::LongReturn => "long",
            Deed::Stayed => "stayed",
            Deed::Hurt => "hurt",
            Deed::Broken => "broken",
            Deed::Greeted => "greeted",
            Deed::Tended => "tended",
        }
    }

    /// How much one of it counts toward knowing you, ever.
    fn known(self) -> f64 {
        match self {
            Deed::Touch => 0.1,
            Deed::Stayed | Deed::Hummed | Deed::Returned => 0.5,
            Deed::Broken | Deed::LongReturn => 2.0,
            _ => 1.0,
        }
    }

    /// How much one of it counts as care, lately (negative: as hurt).
    fn felt(self) -> f64 {
        match self {
            Deed::Touch => 0.0,
            Deed::Held | Deed::Sang | Deed::Answered | Deed::Tended => 1.0,
            Deed::Hummed | Deed::Returned | Deed::Greeted => 0.5,
            Deed::LongReturn => 0.3,
            Deed::Stayed => 0.25,
            Deed::Hurt => -1.0,
            Deed::Broken => -2.0,
        }
    }

    /// After this long, a deed counts half as much as it did. Care is kept twice as long as hurt.
    fn half_life(self) -> f64 {
        if self.felt() < 0.0 {
            2.0 * DAY
        } else {
            4.0 * DAY
        }
    }
}

/// Everything remembered about one kind of deed. `recent` is its value as of the ledger's `at`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tally {
    pub count: u64,
    pub recent: f64,
}

/// `value` after `secs` of fading at `half_life`.
fn decay(value: f64, secs: f64, half_life: f64) -> f64 {
    value * 0.5f64.powf(secs.max(0.0) / half_life)
}

/// 0 at 0, a half at `k`, never quite 1.
fn sat(x: f64, k: f64) -> f64 {
    let x = x.max(0.0);
    x / (x + k)
}

/// One tally per kind of deed, and when their recent sums were last brought up to date.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledger {
    tallies: [Tally; Deed::COUNT],
    at: f64,
}

impl Ledger {
    pub fn get(&self, d: Deed) -> Tally {
        self.tallies[d as usize]
    }

    /// The fading sum of `d` as of `now`.
    pub fn recent(&self, d: Deed, now: f64) -> f64 {
        decay(self.get(d).recent, now - self.at, d.half_life())
    }

    /// Bring every recent sum up to `now` (a clock that went backward counts as no time).
    fn settle(&mut self, now: f64) {
        if now <= self.at {
            return;
        }
        for d in Deed::ALL {
            let r = self.recent(d, now);
            self.tallies[d as usize].recent = r;
        }
        self.at = now;
    }

    pub fn add(&mut self, d: Deed, now: f64) {
        self.settle(now);
        let t = &mut self.tallies[d as usize];
        t.count += 1;
        t.recent += 1.0;
    }

    fn lifetime(&self) -> f64 {
        Deed::ALL.iter().map(|&d| d.known() * self.get(d).count as f64).sum()
    }
}

/// How glad of you it is, 0..1, 0.5 neutral: lately's care against lately's hurt, pulled toward
/// neutral by how little it knows you yet. An empty ledger is exactly neutral. Version `FORMULA`.
pub fn warmth_of(ledger: &Ledger, now: f64) -> f32 {
    let (mut care, mut hurt) = (0.0, 0.0);
    for d in Deed::ALL {
        let r = ledger.recent(d, now) * d.felt();
        if r > 0.0 {
            care += r;
        } else {
            hurt -= r;
        }
    }
    let target = 0.5 + 0.5 * (sat(care, 4.0) - sat(hurt, 2.0));
    let familiar = sat(ledger.lifetime(), FAMILIAR);
    (0.5 + (target - 0.5) * familiar).clamp(0.0, 1.0) as f32
}

/// How well it knows you, 0..1, from every deed ever. Never goes down.
pub fn bond_of(ledger: &Ledger) -> f32 {
    (1.0 - (-ledger.lifetime() / BOND_SCALE).exp()) as f32
}

/// How it meets you, once a visit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Greeting {
    /// Not yet: still waiting for a hand to come near.
    Due {
        warmth: f32,
    },
    Done,
}

pub struct Remembrance {
    ledger: Ledger,
    /// When it first met you, and when you were last here (unix seconds).
    born: f64,
    seen: f64,
    /// How long you had been away when this visit began (seconds); 0 on a first visit.
    away: f64,
    /// The clock, unix seconds: set at load, moved on by every frame.
    now: f64,
    greeting: Greeting,
    /// The scar, if any: which way it faces from the middle, and how plain it is (wound.rs).
    scar: (V3, f32),
    /// Whether the scar has been handed to the world yet this visit.
    scar_shown: bool,
    /// The flower spun nearly dizzy: how many times it kept, and the way it learned you turn it
    /// (+1 or -1; 0 not yet) (forms/spin.rs).
    spun: (u32, f32),
    /// The note it knows you by, in Hz, if you have ever sung to it.
    pitch: Option<f32>,
    /// Where you last touched it, from its middle, if you ever have.
    spot: Option<V3>,
    /// 1 at the start of a visit after a long time away, fading: how much of `RESERVE` it keeps.
    reserve: f32,
    // ---- what it was watching last frame
    upset: bool,
    held: f32,
    hummed: bool,
    whole: bool,
    answered: Option<u32>,
    stayed: f32,
    since_save: f32,
    /// Something changed that should be written down.
    pub dirty: bool,
}

impl Default for Remembrance {
    fn default() -> Self {
        Remembrance::fresh(0.0)
    }
}

impl Remembrance {
    /// Nothing remembered yet; nothing to greet.
    pub fn fresh(now: f64) -> Remembrance {
        Remembrance {
            ledger: Ledger { at: now, ..Default::default() },
            born: now,
            seen: now,
            away: 0.0,
            now,
            greeting: Greeting::Done,
            scar: (V3::UP, 0.0),
            scar_shown: false,
            spun: (0, 0.0),
            pitch: None,
            spot: None,
            reserve: 0.0,
            upset: false,
            held: 0.0,
            hummed: false,
            whole: true,
            answered: None,
            stayed: 0.0,
            since_save: 0.0,
            dirty: false,
        }
    }

    /// What was kept (`save`'s string, or anything else: unreadable parts start over), at `now`,
    /// the start of a visit: it counts the return and gets ready to meet you.
    pub fn load(saved: &str, now: f64) -> Remembrance {
        let mut m = Remembrance::fresh(now);
        let mut had = false;
        for part in saved.split(';') {
            let Some((k, v)) = part.split_once('=') else { continue };
            let num = v.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0);
            match k {
                "born" => m.born = num.unwrap_or(now).min(now),
                "seen" => m.seen = num.unwrap_or(now).min(now),
                "at" => m.ledger.at = num.unwrap_or(now).min(now),
                "scar" => {
                    let n: Vec<f32> =
                        v.split(',').filter_map(|x| x.parse::<f32>().ok()).filter(|x| x.is_finite()).collect();
                    if let [x, y, z, k] = n[..] {
                        let dir = v3(x, y, z);
                        if dir.len() > 0.5 {
                            m.scar = (dir.norm(), k.clamp(0.0, 1.0));
                        }
                    }
                }
                "pitch" => {
                    m.pitch = v.parse::<f32>().ok().filter(|f| (PITCH_LO..=PITCH_HI).contains(f));
                }
                "spot" => {
                    let n: Vec<f32> =
                        v.split(',').filter_map(|x| x.parse::<f32>().ok()).filter(|x| x.is_finite()).collect();
                    if let [x, y, z] = n[..] {
                        let dir = v3(x, y, z);
                        m.spot = (dir.len() > 0.5).then(|| dir.norm());
                    }
                }
                "spun" => {
                    if let Some((n, s)) = v.split_once(',') {
                        if let (Ok(n), Ok(s)) = (n.parse::<u32>(), s.parse::<f32>()) {
                            m.spun = (
                                n.min(99),
                                if s > 0.0 {
                                    1.0
                                } else if s < 0.0 {
                                    -1.0
                                } else {
                                    0.0
                                },
                            );
                        }
                    }
                }
                _ => {
                    let Some(d) = Deed::ALL.iter().find(|d| d.key() == k) else { continue };
                    let mut it = v.split(',').map(|x| x.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0));
                    if let (Some(Some(count)), Some(Some(recent))) = (it.next(), it.next()) {
                        let count = count.round();
                        m.ledger.tallies[*d as usize] = Tally { count: count as u64, recent: recent.min(count) };
                        had = true;
                    }
                }
            }
        }
        if had {
            let away = now - m.seen;
            m.away = away.max(0.0);
            m.ledger.add(Deed::Returned, now);
            if away > LONG_AWAY {
                m.ledger.add(Deed::LongReturn, now);
                m.reserve = 1.0;
            }
            // How it meets you is how it was with you before you came back.
            m.greeting = Greeting::Due { warmth: m.warmth() };
        }
        m.seen = now;
        m.dirty = true;
        m
    }

    /// `v=1;f=1;born=..;seen=..;at=..;touch=count,recent;...`
    pub fn save(&mut self) -> String {
        self.ledger.settle(self.now);
        self.seen = self.now;
        let mut s =
            format!("v={};f={};born={:.0};seen={:.0};at={:.0}", FILE, FORMULA, self.born, self.seen, self.ledger.at);
        for d in Deed::ALL {
            let t = self.ledger.get(d);
            if t.count > 0 {
                s.push_str(&format!(";{}={},{:.3}", d.key(), t.count, t.recent));
            }
        }
        let (dir, k) = self.scar;
        if k > 0.0 {
            s.push_str(&format!(";scar={:.3},{:.3},{:.3},{:.2}", dir.x, dir.y, dir.z, k));
        }
        if self.spun.0 > 0 {
            s.push_str(&format!(";spun={},{}", self.spun.0, self.spun.1));
        }
        if let Some(f) = self.pitch {
            s.push_str(&format!(";pitch={f:.1}"));
        }
        if let Some(d) = self.spot {
            s.push_str(&format!(";spot={:.2},{:.2},{:.2}", d.x, d.y, d.z));
        }
        s
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// How glad of you it is now, 0..1; 0.5 until it knows you. Back after a long time away, a
    /// little less, for the first minutes of the visit.
    pub fn warmth(&self) -> f32 {
        (warmth_of(&self.ledger, self.now) - RESERVE * self.reserve).clamp(0.0, 1.0)
    }

    /// It has been treated well, lately, by someone it knows.
    pub fn kind(&self) -> bool {
        self.warmth() >= KIND_AT
    }

    /// It has been hurt, lately, by someone it knows.
    pub fn wary(&self) -> bool {
        self.warmth() <= WARY_AT
    }

    /// How well it knows you, 0..1. Never goes down.
    pub fn bond(&self) -> f32 {
        bond_of(&self.ledger)
    }

    /// How long you had been away before this visit (seconds); 0 if it never knew you before.
    pub fn away(&self) -> f64 {
        self.away
    }

    /// Whether it carries a scar (wound.rs).
    pub fn scarred(&self) -> bool {
        self.scar.1 > 0.0
    }

    /// The note it knows you by (Hz), if it has heard you sing.
    pub fn pitch(&self) -> Option<f32> {
        self.pitch
    }

    /// Where you last touched it (a direction from its middle), if it has kept one.
    pub fn spot(&self) -> Option<V3> {
        self.spot
    }

    /// You touched it at `dir` (from its middle): kept as where you last touched it.
    pub fn touched(&mut self, dir: V3) {
        if dir.len() > 0.5 && dir.len().is_finite() {
            self.spot = Some(dir.norm());
        }
    }

    /// You held a note `f` (Hz) and let it go: it was sung to, and it moves the note it knows
    /// you by toward it, a semitone at most.
    pub fn sung(&mut self, f: f32) {
        self.remember(Deed::Sang);
        if !(PITCH_LO..=PITCH_HI).contains(&f) {
            return;
        }
        self.pitch = Some(match self.pitch {
            None => f,
            Some(p) => p * 2f32.powf((f / p).log2().clamp(-PITCH_STEP, PITCH_STEP)),
        });
    }

    /// How much it has been hurt, ever.
    fn hurt_ever(&self) -> f64 {
        self.ledger.get(Deed::Hurt).count as f64 + 2.0 * self.ledger.get(Deed::Broken).count as f64
    }

    /// Something happened to it.
    pub fn remember(&mut self, d: Deed) {
        self.ledger.add(d, self.now);
        self.dirty = true;
    }

    /// Time passes.
    pub fn tick(&mut self, dt: f32) {
        self.now += f64::from(dt.max(0.0));
        self.reserve = fade(self.reserve, -RESERVE_RATE * dt.max(0.0));
        self.since_save += dt;
        if self.since_save >= SAVE_EVERY {
            self.since_save = 0.0;
            self.dirty = true;
        }
    }
}

/// Once a frame, at the end of `Game::update`: what happened to it, and, once a visit, how it
/// meets you. Reads `g.sounds` whole, so the page must empty it every frame (it does).
pub fn step(g: &mut Game, dt: f32) {
    let m = &mut g.remembrance;
    m.tick(dt);

    for s in &g.sounds {
        if let Sound::Poke { .. } = s {
            m.remember(Deed::Touch);
        }
    }
    if !m.scar_shown {
        m.scar_shown = true;
        g.world.wound.dir = m.scar.0;
        g.world.wound.scar = m.scar.1;
        let sp = &mut g.world.forms.spin;
        sp.nearly = m.spun.0;
        sp.learned = Some(m.spun.1).filter(|s| *s != 0.0);
    }
    let agitation = g.world.agitation;
    let mut hurt = false;
    if !m.upset && agitation > HURT {
        m.upset = true;
        hurt = true;
        m.remember(Deed::Hurt);
    } else if m.upset && agitation < CALM {
        m.upset = false;
    }
    let whole = g.world.burst.whole();
    if m.whole && !whole {
        m.remember(Deed::Broken);
    }
    m.whole = whole;
    if g.world.warming {
        let before = m.held;
        m.held += dt;
        if before < HELD_SECS && m.held >= HELD_SECS {
            m.remember(Deed::Held);
        }
    } else {
        m.held = 0.0;
    }
    let hummed = g.keys.hum().is_some();
    if hummed && !m.hummed {
        m.remember(Deed::Hummed);
    }
    m.hummed = hummed;
    // Being sung to is told to it when a note ends (`sung`, from game/learn.rs).
    // What was answered before this visit is the contact's to keep, not a new deed.
    let answered = g.contact.answered;
    if m.answered.is_some_and(|a| answered > a) {
        m.remember(Deed::Answered);
    }
    m.answered = Some(answered);
    if !m.upset && agitation < CALM {
        m.stayed += dt;
        if m.stayed >= STAY_SECS {
            m.stayed = 0.0;
            m.remember(Deed::Stayed);
        }
    }

    scar(g, hurt);
    crate::wound::step(g, dt);
    let w = &g.world.wound;
    if g.remembrance.scar != (w.dir, w.scar) {
        g.remembrance.scar = (w.dir, w.scar);
        g.remembrance.dirty = true;
    }
    let sp = &g.world.forms.spin;
    let spun = (sp.nearly, sp.learned.unwrap_or(0.0));
    if g.remembrance.spun != spun {
        g.remembrance.spun = spun;
        g.remembrance.dirty = true;
    }
    greet(g);
}

/// Hurt enough, ever, and it scars where it is struck most; hurt again while scarred, and the
/// scar opens (wound.rs).
fn scar(g: &mut Game, hurt: bool) {
    if g.world.wound.scar <= 0.0 && g.remembrance.hurt_ever() >= SCAR_AT {
        let spot = g.contact.spot;
        let dir = if spot.len() > 0.5 { spot.norm() } else { -g.cam.basis().fwd };
        g.world.wound.dir = dir;
        g.world.wound.scar = 1.0;
        g.world.add_ripple(dir, 0.6);
        g.discover(Discovery::Scar);
    } else if hurt && g.world.ordinary() {
        let mut wound = std::mem::take(&mut g.world.wound);
        let opened = wound.tear_open(&mut g.world);
        g.world.wound = wound;
        if opened {
            g.discover(Discovery::Wound);
        }
    }
}

/// The first time this visit a hand comes near it, it shows how it remembers you: it leans
/// toward a hand that was kind to it and pulls away from one that hurt it. Only once it has
/// begun to regard you, only when it is itself, and only if it knows you well enough to care.
fn greet(g: &mut Game) {
    let Greeting::Due { warmth } = g.remembrance.greeting else { return };
    if g.stage() < Stage::Regard || !g.world.ordinary() || g.contact.acting() {
        return;
    }
    let Some(p) = g.pointer_at else { return };
    let to = p - g.world.body.pos;
    if to.len() > g.world.body.radius() * GREET_NEAR {
        return;
    }
    g.remembrance.greeting = Greeting::Done;
    let fwd = g.cam.basis().fwd;
    // Toward the hand, and a little toward whoever is behind it.
    let dir = (to.norm() - fwd * 0.5).norm();
    let w = &mut g.world;
    if warmth >= KIND_AT {
        w.body.vel += dir * 1.1;
        w.body.squash_vel += 1.2;
        w.add_ripple(dir, 0.22);
        g.discover(Discovery::Remembered);
    } else if warmth <= WARY_AT {
        w.body.vel -= dir * 2.2;
        w.body.squash_vel -= 2.0;
        w.add_ripple(-dir, 0.35);
        w.agitation = w.agitation.max(0.3);
        g.discover(Discovery::Wary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: f64 = 1_800_000_000.0;

    fn with(deeds: &[(Deed, u32)], now: f64) -> Ledger {
        let mut l = Ledger { at: now, ..Default::default() };
        for &(d, n) in deeds {
            for _ in 0..n {
                l.add(d, now);
            }
        }
        l
    }

    #[test]
    fn an_empty_ledger_is_neutral_and_unknown() {
        let l = Ledger::default();
        assert_eq!(warmth_of(&l, T0), 0.5);
        assert_eq!(bond_of(&l), 0.0);
    }

    #[test]
    fn care_warms_and_hurt_cools() {
        let kind = with(&[(Deed::Held, 6), (Deed::Sang, 2), (Deed::Returned, 2)], T0);
        let rough = with(&[(Deed::Hurt, 4), (Deed::Broken, 1), (Deed::Returned, 1)], T0);
        assert!(warmth_of(&kind, T0) >= KIND_AT, "{}", warmth_of(&kind, T0));
        assert!(warmth_of(&rough, T0) <= WARY_AT, "{}", warmth_of(&rough, T0));
    }

    #[test]
    fn touching_alone_is_neither() {
        let l = with(&[(Deed::Touch, 200)], T0);
        assert_eq!(warmth_of(&l, T0), 0.5);
        assert!(bond_of(&l) > 0.0);
    }

    #[test]
    fn hurt_fades_sooner_than_care() {
        let l = with(&[(Deed::Held, 4), (Deed::Hurt, 4)], T0);
        let later = T0 + 6.0 * DAY;
        assert!(l.recent(Deed::Hurt, later) < l.recent(Deed::Held, later));
        assert!(warmth_of(&l, later) > warmth_of(&l, T0));
    }

    #[test]
    fn the_bond_never_shrinks() {
        let mut l = with(&[(Deed::Held, 3)], T0);
        let before = bond_of(&l);
        l.add(Deed::Hurt, T0 + DAY);
        assert!(bond_of(&l) > before);
        assert_eq!(bond_of(&l), bond_of(&l.clone()));
    }

    #[test]
    fn it_survives_being_written_down() {
        let mut m = Remembrance::fresh(T0);
        for _ in 0..5 {
            m.remember(Deed::Held);
        }
        m.remember(Deed::Hurt);
        m.tick(60.0);
        let s = m.save();
        let back = Remembrance::load(&s, T0 + 60.0);
        assert_eq!(back.ledger.get(Deed::Held).count, 5);
        assert_eq!(back.ledger.get(Deed::Hurt).count, 1);
        // Coming back is remembered too.
        assert_eq!(back.ledger.get(Deed::Returned).count, 1);
        assert_eq!(back.ledger.get(Deed::LongReturn).count, 0);
        assert!((back.warmth() - m.warmth()).abs() < 0.05);
    }

    #[test]
    fn days_away_is_a_long_return() {
        let mut m = Remembrance::fresh(T0);
        m.remember(Deed::Held);
        let s = m.save();
        let back = Remembrance::load(&s, T0 + 5.0 * DAY);
        assert_eq!(back.ledger.get(Deed::LongReturn).count, 1);
    }

    #[test]
    fn a_long_time_away_is_a_return_and_a_little_care_not_pure_care() {
        assert_eq!(FORMULA, 2);
        assert_eq!(Deed::LongReturn.felt(), 0.3);
        assert!(Deed::LongReturn.felt() < Deed::Held.felt());
        // a ledger near neutral: back after a week, it is warier at first than back after a day
        let mut m = Remembrance::fresh(T0);
        for _ in 0..12 {
            m.remember(Deed::Touch);
        }
        m.remember(Deed::Held);
        let s = m.save();
        let day = Remembrance::load(&s, T0 + DAY);
        let week = Remembrance::load(&s, T0 + 7.0 * DAY);
        assert_eq!(week.ledger.get(Deed::LongReturn).count, 1);
        let quiet = warmth_of(&week.ledger, week.now);
        assert!((week.warmth() - (quiet - RESERVE)).abs() < 1e-4, "careful of you at first");
        assert_eq!(day.warmth(), warmth_of(&day.ledger, day.now), "only after a long time away");
        // and it fades within the visit
        let mut week = week;
        week.tick(300.0);
        assert!((week.warmth() - warmth_of(&week.ledger, week.now)).abs() < 0.01);
        // nothing of it is written down: the next visit starts from the ledger alone
        assert!(!week.save().contains("reserve"));
    }

    #[test]
    fn a_save_from_before_it_kept_your_touch_loads_with_none() {
        for old in ["v=1;f=1;born=1;seen=1;at=1;held=2,1;long=1,0.5", "held=2,1;spot=nan,1,2", "spot=0,0,0", "spot=1,2"]
        {
            let m = Remembrance::load(old, T0);
            assert_eq!(m.spot(), None, "{old}");
        }
        let m = Remembrance::load("v=1;f=1;born=1;seen=1;at=1;held=2,1;long=1,0.5", T0);
        assert_eq!(m.ledger.get(Deed::LongReturn).count, 2, "kept, and this return counted");
        assert_eq!(m.ledger.get(Deed::Held).count, 2);
    }

    #[test]
    fn nothing_kept_means_nothing_to_greet() {
        for s in ["", "garbage", "held=x,y;at=-4;born=nan"] {
            let m = Remembrance::load(s, T0);
            assert_eq!(m.greeting, Greeting::Done, "{s}");
            assert_eq!(m.ledger.lifetime(), 0.0, "{s}");
        }
    }

    #[test]
    fn it_learns_the_middle_of_the_notes_you_sing_and_keeps_only_that() {
        let mut m = Remembrance::fresh(T0);
        assert_eq!(m.pitch(), None);
        for f in [220.0, 230.0, 210.0, 900.0, 225.0, 215.0, 220.0, 220.0] {
            m.sung(f);
        }
        let p = m.pitch().unwrap();
        assert!((205.0..240.0).contains(&p), "one stray note moves it little: {p}");
        assert_eq!(m.ledger.get(Deed::Sang).count, 8);
        let s = m.save();
        assert!(s.contains(";pitch="), "{s}");
        let back = Remembrance::load(&s, T0 + 60.0);
        assert!((back.pitch().unwrap() - p).abs() < 0.1);
        // a save from before it learned notes, or a broken one, has none
        for old in ["v=1;f=1;born=1;seen=1;at=1;held=2,1", "held=2,1;pitch=nan", "pitch=-4", "pitch=99999"] {
            assert_eq!(Remembrance::load(old, T0).pitch(), None, "{old}");
        }
    }

    #[test]
    fn a_tampered_sum_is_held_to_its_count() {
        let m = Remembrance::load("at=1800000000;held=2,900", T0);
        assert!(m.ledger.get(Deed::Held).recent <= 2.0);
    }

    fn game_at(found: usize, saved: &str) -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        for &d in crate::progress::ALL.iter().take(found) {
            g.progress.discover(d);
        }
        g.remembrance = Remembrance::load(saved, T0);
        g
    }

    fn hover_center(g: &mut Game) {
        for _ in 0..10 {
            g.pointer_move(1, 400.0, 300.0, 1.0 / 60.0);
            g.update(1.0 / 60.0);
            g.sounds.clear();
        }
    }

    fn kept(deeds: &[(Deed, u32)]) -> String {
        let mut m = Remembrance::fresh(T0 - 3600.0);
        for &(d, n) in deeds {
            for _ in 0..n {
                m.remember(d);
            }
        }
        m.save()
    }

    #[test]
    fn a_kind_hand_is_leaned_toward() {
        let mut g = game_at(10, &kept(&[(Deed::Held, 8), (Deed::Sang, 3)]));
        hover_center(&mut g);
        assert!(g.progress.has(Discovery::Remembered));
        assert!(!g.progress.has(Discovery::Wary));
    }

    #[test]
    fn a_rough_hand_is_shied_from() {
        let mut g = game_at(10, &kept(&[(Deed::Hurt, 6), (Deed::Broken, 2)]));
        hover_center(&mut g);
        assert!(g.progress.has(Discovery::Wary));
        assert!(!g.progress.has(Discovery::Remembered));
    }

    #[test]
    fn before_regard_it_does_not_show_it() {
        let mut g = game_at(2, &kept(&[(Deed::Held, 8), (Deed::Sang, 3)]));
        hover_center(&mut g);
        assert!(!g.progress.has(Discovery::Remembered));
    }

    #[test]
    fn provoking_it_is_remembered_as_hurt() {
        let mut g = game_at(0, "");
        for _ in 0..12 {
            g.pointer_down(1, 400.0, 300.0);
            g.pointer_up(1);
            g.update(1.0 / 60.0);
            g.sounds.clear();
        }
        assert!(g.remembrance.ledger().get(Deed::Touch).count >= 6);
        assert!(g.remembrance.ledger().get(Deed::Hurt).count >= 1);
    }

    #[test]
    fn holding_it_still_is_remembered_as_held() {
        let mut g = game_at(0, "");
        g.pointer_down(1, 400.0, 300.0);
        for _ in 0..180 {
            g.update(1.0 / 60.0);
            g.sounds.clear();
        }
        g.pointer_up(1);
        assert_eq!(g.remembrance.ledger().get(Deed::Held).count, 1);
    }
}
