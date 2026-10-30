//! Coming back after time away. How long you were gone is worked out once, when the page loads
//! (remembrance.rs `away`), and the room shows it from the first frame; nothing here is played
//! out while you are away or while you are here, and a first visit shows nothing at all.
//!
//! - after hours away: one small drop of it lies on the floor, where you last touched it. It is
//!   held there until a hand takes it; let go, it drifts home like any other drop.
//! - after days: it is faintly warm for its first twenty seconds, the warmth fading as you watch.
//! - after a long time away (remembrance.rs `LONG_AWAY`), once it has begun to regard you: it is
//!   facing away when the room appears, and turns slowly round to you. Wary of you, it turns
//!   slower, not quite all the way, and keeps back a little while it does.
//! - glad of you, and knowing the note you sing: after your first touch, it sounds that note
//!   once, hardly there. Nothing like it for a hand it is wary of.
//!
//! Everything here is drawn with what is already there: a drop, the body's color, the swell it
//! turns toward whoever it is looking at. The shader does not change.

use crate::floor::fusion::FLOOR_Y;
use crate::game::{Game, Sound};
use crate::math::*;
use crate::mind::contact::Stage;
use crate::mind::remembrance::{DAY, HOUR, LONG_AWAY};
use crate::progress::Discovery;
use crate::sim::PEARL;

/// Away at least this long (seconds), something is left on the floor.
pub const MARK_AFTER: f64 = 3.0 * HOUR;
/// Away at least this long, it is warm for a while.
pub const WARM_AFTER: f64 = 2.0 * DAY;
/// How big the drop is, and how far from its middle, across the floor, it lies.
const DROP_R: f32 = 0.1;
const DROP_OUT: f32 = 1.9;
/// The warmth: toward its light's color (`floorLight`), at most this much of the way, fading
/// over this long (seconds).
const WARM: V3 = v3(1.0, 0.58, 0.26);
const WARM_MOST: f32 = 0.14;
const WARM_SECS: f32 = 20.0;
/// The turn: facing away this long (seconds) while the room appears, then round to you over
/// `TURN_SECS` (`WARY_TURN_SECS` if it is wary of you), leaning `LEAN` (as regard leans 0.22),
/// the lean easing off over `LET_GO_SECS` after.
const AWAY_SECS: f32 = 2.0;
const TURN_SECS: f32 = 5.0;
const WARY_TURN_SECS: f32 = 9.0;
const LEAN: f32 = 0.3;
const LET_GO_SECS: f32 = 2.0;
/// Wary, it stops this far short of facing you (radians), and keeps this far back (units).
const WARY_SHORT: f32 = 0.5;
const KEEP_BACK: f32 = 0.12;
/// The fastest the way it faces may turn (radians a second): faster than the turn ever asks,
/// so it only matters after a hand has turned it somewhere else.
const TURN_RATE: f32 = 1.6;
/// The body's spring toward home while it is whole (sim/): what `KEEP_BACK` pushes against.
const HOME_SPRING: f32 = 28.0;
/// After the first touch is let go, this long (seconds) before its note.
const NOTE_AFTER: f32 = 0.6;

/// What the room shows of how long you were away.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marks {
    /// A drop on the floor where you last touched it.
    pub drop: bool,
    /// Warm, for its first seconds.
    pub warm: bool,
    /// Facing away, turning to you.
    pub turn: bool,
    /// Your note (Hz, folded under C4), once, after your first touch.
    pub note: Option<f32>,
}

/// The marks of `away` seconds gone (0: never here before), for a ledger that is `kind` and
/// knows your note at `pitch`.
pub fn marks(away: f64, kind: bool, pitch: Option<f32>) -> Marks {
    if away < MARK_AFTER {
        return Marks::default();
    }
    Marks { drop: true, warm: away >= WARM_AFTER, turn: away > LONG_AWAY, note: pitch.filter(|_| kind).map(under_c4) }
}

/// `f` moved by octaves into the octave under middle C, where it hums what it hears.
fn under_c4(mut f: f32) -> f32 {
    while f >= 261.63 {
        f *= 0.5;
    }
    while f < 130.81 {
        f *= 2.0;
    }
    f
}

/// Which way it faces at `t` seconds into the visit, as an angle from facing you (radians):
/// half a turn away at first, then easing round.
pub fn facing(t: f32, wary: bool) -> f32 {
    let (secs, short) = if wary { (WARY_TURN_SECS, WARY_SHORT) } else { (TURN_SECS, 0.0) };
    let k = smooth01((t - AWAY_SECS) / secs);
    std::f32::consts::PI + (short - std::f32::consts::PI) * k
}

/// How far it leans the way it faces at `t`: all of `LEAN` until it has turned, then less.
fn lean(t: f32, wary: bool) -> f32 {
    let end = AWAY_SECS + if wary { WARY_TURN_SECS } else { TURN_SECS };
    LEAN * (1.0 - smooth01((t - end) / LET_GO_SECS))
}

/// `from` turned toward `to`, by at most `most` radians.
fn turn_toward(from: V3, to: V3, most: f32) -> V3 {
    let c = from.dot(to).clamp(-1.0, 1.0);
    if c.acos() <= most {
        return to;
    }
    let side = to - from * c;
    let side = if side.len() < 1e-4 { V3::UP.cross(from) } else { side };
    let side = if side.len() < 1e-4 { v3(1.0, 0.0, 0.0) } else { side.norm() };
    (from * most.cos() + side * most.sin()).norm()
}

#[derive(Clone, Copy, Debug)]
struct Turn {
    wary: bool,
    /// The way it faces now.
    dir: V3,
}

#[derive(Default)]
pub struct Absence {
    begun: bool,
    pub marks: Marks,
    /// Seconds into the visit.
    t: f32,
    /// The drop it left, while it still holds it in place, and where.
    drop: Option<(u32, V3)>,
    /// How much warmth is in the body's color now, so it can all be taken back out.
    warmed: V3,
    turn: Option<Turn>,
    /// A hand has come down; seconds since it was let go.
    touched: bool,
    let_go: Option<f32>,
}

impl Absence {
    /// The drop it left on the floor, while it is still lying there.
    pub fn left(&self) -> Option<u32> {
        self.drop.map(|d| d.0)
    }

    /// Still turning round to you.
    pub fn turning(&self) -> bool {
        self.turn.is_some()
    }
}

/// Once a frame, at the end of `Game::update`.
pub fn step(g: &mut Game, dt: f32) {
    if !g.absence.begun {
        begin(g);
    }
    g.absence.t += dt;
    hold_drop(g);
    warm(g);
    turn(g, dt);
    note(g, dt);
}

/// The first frame: what it shows is decided now, from how long you were away, and never again.
fn begin(g: &mut Game) {
    let regards = g.stage() >= Stage::Regard && g.world.ordinary();
    let a = &mut g.absence;
    a.begun = true;
    let r = &g.remembrance;
    a.marks = marks(r.away(), r.kind(), r.pitch());
    let fwd = g.cam.basis().fwd;
    let toward_you = flat(-fwd).unwrap_or(v3(0.0, 0.0, 1.0));
    if a.marks.drop {
        // Where you last touched it, or, kept before it kept that, where it is touched most.
        let spot = r.spot().or_else(|| (g.contact.spot.len() > 0.1).then(|| g.contact.spot.norm()));
        let out = spot.and_then(flat).unwrap_or(toward_you);
        let w = &mut g.world;
        let at = v3(w.body.pos.x + out.x * DROP_OUT, FLOOR_Y + DROP_R, w.body.pos.z + out.z * DROP_OUT);
        if w.spill(at, V3::ZERO, DROP_R, w.body.color) {
            let id = w.drops.last().map_or(0, |d| d.id);
            if let Some(d) = w.drop_mut(id) {
                d.target = Some(at);
            }
            a.drop = Some((id, at));
        }
    }
    if a.marks.turn && regards {
        a.turn = Some(Turn { wary: r.wary(), dir: -toward_you });
    }
}

/// Across the floor: `v` without its height, if it has any length left.
fn flat(v: V3) -> Option<V3> {
    let f = v3(v.x, 0.0, v.z);
    (f.len() > 0.3).then(|| f.norm())
}

/// The drop stays where it was left until a hand takes it; then it is the hand's.
fn hold_drop(g: &mut Game) {
    let Some((id, at)) = g.absence.drop else { return };
    if g.carried_drop() == Some(id) {
        g.absence.drop = None;
        g.discover(Discovery::Waited);
        return;
    }
    match g.world.drop_mut(id) {
        Some(d) => d.target = Some(at),
        None => g.absence.drop = None,
    }
}

/// Warm at first, fading. Only what it added is taken back out, so whatever else colors it
/// meanwhile stays.
fn warm(g: &mut Game) {
    let a = &mut g.absence;
    if !a.marks.warm && a.warmed == V3::ZERO {
        return;
    }
    let k = if a.marks.warm { WARM_MOST * (1.0 - smooth01(a.t / WARM_SECS)) } else { 0.0 };
    let now = (WARM - PEARL) * k;
    g.world.body.color += now - a.warmed;
    a.warmed = now;
    if k <= 0.0 {
        a.marks.warm = false;
        a.warmed = V3::ZERO;
    }
}

/// Facing away, then round to you. A hand on it turns it where the hand is; let go, it goes on
/// turning from there. Anything else it becomes ends the turn, and the lean eases off as any does.
fn turn(g: &mut Game, dt: f32) {
    let Some(mut tn) = g.absence.turn else { return };
    if !g.world.ordinary() || g.contact.in_contact() {
        g.absence.turn = None;
        return;
    }
    let t = g.absence.t;
    let b = g.cam.basis();
    let you = flat(-b.fwd).unwrap_or(v3(0.0, 0.0, 1.0));
    let side = flat(b.right).unwrap_or(v3(1.0, 0.0, 0.0));
    let a = facing(t, tn.wary);
    let goal = you * a.cos() + side * a.sin();
    let held = g.held();
    let w = &mut g.world;
    if held {
        tn.dir = w.gaze_dir;
    } else {
        tn.dir = turn_toward(tn.dir, goal, TURN_RATE * dt);
        w.gaze_dir = tn.dir;
    }
    let lean = lean(t, tn.wary);
    w.regard = w.regard.max(lean);
    if tn.wary {
        // Kept back from you, as far as the spring that holds it home lets a steady push.
        w.body.vel -= you * (KEEP_BACK * HOME_SPRING * dt * lean / LEAN);
    }
    g.absence.turn = if lean > 0.0 { Some(tn) } else { None };
}

/// Glad of you, it sounds your note once, faint, after your first touch is let go.
fn note(g: &mut Game, dt: f32) {
    let Some(f) = g.absence.marks.note else { return };
    let held = g.held();
    let a = &mut g.absence;
    if held {
        a.touched = true;
        a.let_go = None;
        return;
    }
    if !a.touched {
        return;
    }
    let since = a.let_go.unwrap_or(0.0) + dt;
    a.let_go = Some(since);
    if since >= NOTE_AFTER {
        a.marks.note = None;
        g.sounds.push(Sound::Kept { f });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::remembrance::{Deed, Remembrance};

    const T0: f64 = 1_800_000_000.0;
    const DT: f32 = 1.0 / 60.0;

    /// What was kept of a visit `away` seconds ago: `deeds`, and maybe a note and a touch.
    fn kept(deeds: &[(Deed, u32)], pitch: Option<f32>, spot: Option<V3>) -> String {
        let mut m = Remembrance::fresh(T0 - 3600.0);
        for &(d, n) in deeds {
            (0..n).for_each(|_| m.remember(d));
        }
        if let Some(f) = pitch {
            m.sung(f);
        }
        if let Some(s) = spot {
            m.touched(s);
        }
        m.save()
    }

    fn game(found: usize, saved: &str, away: f64) -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        for &d in crate::progress::ALL.iter().take(found) {
            g.progress.discover(d);
        }
        g.remembrance = Remembrance::load(saved, T0 - 3600.0 + away);
        g
    }

    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs / DT) as usize {
            g.update(DT);
            g.sounds.clear();
        }
    }

    const KIND: &[(Deed, u32)] = &[(Deed::Held, 8), (Deed::Sang, 3)];
    const ROUGH: &[(Deed, u32)] = &[(Deed::Hurt, 6), (Deed::Broken, 2)];

    #[test]
    fn each_time_away_has_exactly_its_marks() {
        let none = Marks::default();
        assert_eq!(marks(0.0, true, Some(220.0)), none, "a first visit");
        assert_eq!(marks(2.0 * HOUR, true, Some(220.0)), none, "back within hours");
        let hours = marks(4.0 * HOUR, false, None);
        assert_eq!(hours, Marks { drop: true, ..none });
        let days = marks(2.5 * DAY, false, None);
        assert_eq!(days, Marks { drop: true, warm: true, ..none });
        let long = marks(8.0 * DAY, false, None);
        assert_eq!(long, Marks { drop: true, warm: true, turn: true, note: None });
        // the note only for a hand it is glad of, and only if it knows one
        assert_eq!(marks(4.0 * HOUR, true, None).note, None);
        assert_eq!(marks(4.0 * HOUR, false, Some(220.0)).note, None);
        let f = marks(4.0 * HOUR, true, Some(440.0)).note.unwrap();
        assert!((f - 220.0).abs() < 0.01, "folded under middle C: {f}");
    }

    #[test]
    fn a_first_visit_shows_nothing() {
        let mut g = game(10, "", 0.0);
        run(&mut g, 1.0);
        assert_eq!(g.absence.marks, Marks::default());
        assert!(g.world.drops.is_empty());
        assert!(!g.absence.turning());
        assert_eq!(g.world.body.color, PEARL);
    }

    #[test]
    fn nothing_is_played_out_while_you_are_here() {
        // Back within minutes, and then here for days: nothing is ever left, warmed or turned.
        let mut g = game(10, &kept(KIND, Some(220.0), Some(V3::UP)), 600.0);
        run(&mut g, 0.5);
        g.remembrance.tick(10.0 * DAY as f32);
        run(&mut g, 2.0);
        assert_eq!(g.absence.marks, Marks::default());
        assert!(g.world.drops.is_empty());
        assert!(!g.absence.turning());
    }

    #[test]
    fn the_drop_lies_where_you_last_touched_it() {
        let spot = v3(0.6, 0.3, -0.75);
        let mut g = game(2, &kept(KIND, None, Some(spot)), 5.0 * HOUR);
        run(&mut g, 2.0);
        assert_eq!(g.world.drops.len(), 1);
        let (id, pos) = (g.world.drops[0].id, g.world.drops[0].pos);
        assert_eq!(g.absence.left(), Some(id));
        let out = flat(spot).unwrap();
        let on = flat(pos - g.world.body.pos).unwrap();
        assert!(on.dot(out) > 0.99, "{on:?} {out:?}");
        assert!((pos.y - (FLOOR_Y + DROP_R)).abs() < 0.02, "on the floor: {}", pos.y);
        // still there a while later: it does not drift home by itself
        run(&mut g, 20.0);
        assert!(g.world.drops.iter().any(|x| x.id == id && (x.pos - pos).len() < 0.02));
        // only hours: no warmth, no turn
        assert!(!g.absence.marks.warm && !g.absence.turning());
    }

    #[test]
    fn where_you_touch_it_is_kept() {
        let mut g = game(0, "", 0.0);
        g.update(DT);
        // a tap on its right side, as the camera sees it
        let (x, y) = g.screen_of(g.world.body.pos + g.cam.basis().right * 0.9);
        g.pointer_down(1, x, y);
        g.pointer_up(1);
        g.update(DT);
        let s = g.remembrance.spot().expect("kept");
        assert!(s.dot(g.cam.basis().right) > 0.5, "{s:?}");
        let back = Remembrance::load(&g.remembrance.save(), T0 + DAY);
        assert!((back.spot().unwrap() - s).len() < 0.02);
    }

    #[test]
    fn the_drop_is_taken_back_like_any_other() {
        let mut g = game(2, &kept(KIND, None, Some(g_right())), 5.0 * HOUR);
        run(&mut g, 1.0);
        let id = g.absence.left().unwrap();
        let pos = g.world.drops.iter().find(|d| d.id == id).unwrap().pos;
        let (x, y) = g.screen_of(pos);
        g.pointer_down(1, x, y);
        assert_eq!(g.carried_drop(), Some(id));
        g.update(DT);
        assert!(g.progress.has(Discovery::Waited));
        assert_eq!(g.absence.left(), None);
        g.pointer_up(1);
        // let go, it drifts home and goes back in
        run(&mut g, 30.0);
        assert!(g.world.drops.iter().all(|d| d.id != id), "taken back in");
    }

    fn g_right() -> V3 {
        Game::new().cam.basis().right
    }

    #[test]
    fn after_days_it_is_warm_at_first_and_then_not() {
        let mut g = game(2, &kept(KIND, None, None), 2.5 * DAY);
        run(&mut g, 0.1);
        let c = g.world.body.color;
        assert!(c.x > c.z + 0.05, "warm: {c:?}");
        let mut last = c;
        for _ in 0..(25.0 / DT) as usize {
            g.update(DT);
            g.sounds.clear();
            let now = g.world.body.color;
            assert!((now - last).len() < 0.01, "it fades, never jumps");
            last = now;
        }
        assert!((g.world.body.color - PEARL).len() < 1e-4, "{:?}", g.world.body.color);
    }

    /// The angle between where it faces and you, frame by frame, over `secs`.
    fn turning(g: &mut Game, secs: f32) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..(secs / DT) as usize {
            g.update(DT);
            g.sounds.clear();
            let you = flat(-g.cam.basis().fwd).unwrap();
            out.push(flat(g.world.gaze_dir).unwrap_or(you).dot(you).clamp(-1.0, 1.0).acos());
        }
        out
    }

    #[test]
    fn after_a_long_time_it_faces_away_and_turns_round_slowly() {
        let mut g = game(10, &kept(KIND, None, None), 8.0 * DAY);
        let a = turning(&mut g, 14.0);
        assert!(a[0] > 3.0, "facing away at first: {}", a[0]);
        assert!(a[(1.5 / DT) as usize] > 3.0, "still, while the room appears");
        assert!(a.last().unwrap() < &0.05, "round to you: {}", a.last().unwrap());
        for w in a.windows(2) {
            assert!((w[1] - w[0]).abs() <= TURN_RATE * DT + 1e-3, "no jump: {w:?}");
        }
        // it took seconds, not a moment
        let half = a.iter().position(|x| *x < std::f32::consts::FRAC_PI_2).unwrap() as f32 * DT;
        assert!(half > AWAY_SECS + 1.5, "{half}");
        assert!(g.world.gaze > 0.0 && !g.absence.turning());
    }

    #[test]
    fn wary_it_turns_slower_and_keeps_back() {
        let mut kind = game(10, &kept(KIND, None, None), 8.0 * DAY);
        let mut wary = game(10, &kept(ROUGH, None, None), 8.0 * DAY);
        wary.update(DT);
        assert!(wary.remembrance.wary());
        let (a, b) = (turning(&mut kind, 6.0), turning(&mut wary, 6.0));
        assert!(b.last().unwrap() > &(a.last().unwrap() + 0.5), "slower: {} {}", a.last().unwrap(), b.last().unwrap());
        let you = flat(-wary.cam.basis().fwd).unwrap();
        assert!(wary.world.body.pos.dot(you) < -0.05, "kept back: {:?}", wary.world.body.pos);
        let b = turning(&mut wary, 10.0);
        assert!(b.last().unwrap() > &0.3, "not quite all the way round");
    }

    #[test]
    fn before_regard_it_does_not_turn() {
        let mut g = game(2, &kept(KIND, None, None), 8.0 * DAY);
        g.update(DT);
        assert!(!g.absence.turning());
        assert!(g.absence.marks.turn && g.absence.left().is_some());
    }

    /// Whether its note sounded this frame.
    fn sang(g: &mut Game) -> Option<f32> {
        g.update(DT);
        let f = g.sounds.iter().find_map(|s| if let Sound::Kept { f } = s { Some(*f) } else { None });
        g.sounds.clear();
        f
    }

    #[test]
    fn glad_of_you_it_sounds_your_note_after_your_first_touch() {
        let mut g = game(10, &kept(KIND, Some(196.0), None), 5.0 * HOUR);
        assert!((0..300).all(|_| sang(&mut g).is_none()), "never before a hand");
        g.pointer_down(1, 50.0, 50.0);
        assert!((0..60).all(|_| sang(&mut g).is_none()), "not while it is down");
        g.pointer_up(1);
        let heard: Vec<f32> = (0..120).filter_map(|_| sang(&mut g)).collect();
        assert_eq!(heard.len(), 1, "once");
        assert!((heard[0] - 196.0).abs() < 0.01);
        g.pointer_down(1, 50.0, 50.0);
        g.pointer_up(1);
        assert!((0..120).all(|_| sang(&mut g).is_none()), "and not again");
    }

    #[test]
    fn wary_of_you_there_is_no_note() {
        let mut g = game(10, &kept(ROUGH, Some(196.0), None), 5.0 * HOUR);
        g.update(DT);
        g.pointer_down(1, 50.0, 50.0);
        g.pointer_up(1);
        assert!((0..120).all(|_| sang(&mut g).is_none()));
    }
}
