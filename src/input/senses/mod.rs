//! It can be let hear you and see you. It never takes either; late in the arc it asks, once a
//! visit, in one word, and only the player's own tap on that word lets the browser ask for the
//! microphone or the camera (platform/web/, devices.rs). Hearing can also be given from the
//! first moment, by a tap on the small microphone in the corner, which waits there until it is.
//! On phones that only let a page feel how they are held after asking, it asks for that the
//! same way, a little earlier (`hold the room.`); what it does with it is in motion.rs. Words come out of hearing (words.rs): where the
//! browser can only make them by sending the voice away, that is asked for on its own, last
//! (`say something.`), with a line under the word saying so.
//!
//! - hearing (offered once it echoes, `speak.`): a voice makes the surface tremble where its
//!   pitch lands, high notes high on the body. A note held and then let go comes back as a hum
//!   of the same pitch, low and short; while it hums it does not listen, so it never hears
//!   itself. A whisper and it goes still and leans in, its near side shivering. A breath blown at
//!   it presses in the side nearest you and pushes it away; kept up, it takes a drop off the far
//!   side. A sudden loud sound and it flinches; a shout kept up and it is upset.
//! - sight (offered once it takes turns, `show yourself.`): only coarse things, never a face or
//!   a picture: how much light there is, how much changed since the last look, and where the
//!   change was. It turns toward wherever you moved last, flinches from a fast movement, and
//!   when the lens is covered the room goes dark with it. The eye, if there is one, follows.
//!
//! Every sound and every frame is reduced here to a few numbers and then dropped; nothing is
//! kept, nothing leaves the browser. Stopping is always one key or one tap away. A refusal is
//! remembered (`save` / `load`) and never asked again.

use crate::math::*;
use crate::mind::contact::Stage;
use crate::progress::Discovery;
use crate::sim::World;

mod hearing;
mod sight;
#[cfg(test)]
mod tests;

pub use hearing::{hear, Heard};
pub use sight::{Retina, Seen};
use hearing::{hum_pitch, pitch_height};

/// How long it waits, from the start of a visit, before it may ask anything (seconds).
const OFFER_NOT_BEFORE: f32 = 20.0;
/// Untouched this long, it may ask.
const OFFER_IDLE: f32 = 5.0;
/// How long the word stays up to be taken.
pub const OFFER_WINDOW: f32 = 9.0;
/// Between asking to hear and asking to see, at least.
const OFFER_GAP: f32 = 40.0;

/// Below this the room is silent, whatever the microphone says. Phones hand over the raw
/// microphone (devices.rs), which is quiet: a whisper at arm's length is about 0.003.
const QUIET: f32 = 0.0015;
/// A voice is loud enough; a breath at the microphone is louder and has no pitch.
const VOICE: f32 = 0.005;
/// Air blown across the microphone: this loud, and mostly low rumble, or with no clear pitch.
/// A rumble can look like a low note; only a clear one is a voice.
const BLOW: f32 = 0.03;
const BLOW_LOW: f32 = 0.65;
const BLOW_CLARITY: f32 = 0.9;
/// A voice this loud, kept up, is shouting.
const SHOUT: f32 = 0.16;
/// Rough, quiet, unpitched: a whisper. Roughness is the zero-crossing rate, 0..1.
const WHISPER_ROUGH: f32 = 0.07;
const WHISPER_MAX: f32 = 0.07;
/// Above the room by this much, a sound is someone's: a fan or traffic, however loud, is not.
const OVER_ROOM: f32 = 2.0;
/// A held note is this steady (fraction of its pitch) for this long, at least.
const HELD_STEADY: f32 = 0.06;
const HELD_SECS: f32 = 0.8;
/// How long it hums a note back.
pub const HUM_SECS: f32 = 2.4;
/// Pitches it hears, in Hz; the trembling maps this range onto the body, low to high.
const PITCH_LO: f32 = 70.0;
const PITCH_HI: f32 = 1000.0;

/// Darker than this, the lens is covered (mean light, 0..1).
const COVERED: f32 = 0.07;
/// More of the picture changed than this, at once: too fast.
const STARTLE_MOTION: f32 = 0.16;
/// Less than this changed: nobody moved.
const STIR_MOTION: f32 = 0.004;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hear,
    See,
    /// How a phone is held and moved (motion.rs).
    Feel,
    /// Words, where the browser can only make words of a voice by sending it away to be
    /// understood (words.rs, speech.rs). Where it can do that on the device, hearing is enough
    /// and this is never asked.
    Words,
}

impl Kind {
    /// The one word it asks with.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Hear => "speak.",
            Kind::See => "show yourself.",
            Kind::Feel => "hold the room.",
            Kind::Words => "say something.",
        }
    }

    /// What it says when the browser is told no.
    pub fn refused(self) -> &'static str {
        match self {
            Kind::Hear => "silence is also something it can learn.",
            Kind::See => "it looks away. it will not ask again.",
            Kind::Feel => "it will stay where it is put.",
            Kind::Words => "it will listen to the sound of you instead.",
        }
    }
}

/// Whether it may hear (or see), as far as the player has let it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Leave {
    /// Not asked yet this visit, or asked and not taken up.
    Closed,
    /// The word is up; this long left to take it.
    Offered(f32),
    /// The player took it; the browser is asking them.
    Opening,
    Open,
    /// The browser was told no. Kept between visits.
    Refused,
}

/// What it asks the page to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Offer(Kind),
    Refused(Kind),
    /// It stopped hearing or seeing (the player asked, or the page was left).
    Closed(Kind),
}

/// What the game tells it each frame.
pub struct Look {
    pub stage: Stage,
    /// Seconds since the pointer last did anything.
    pub still: f32,
    /// Busy with something of its own (contact.rs).
    pub busy: bool,
    pub fwd: V3,
    pub right: V3,
    pub up: V3,
}

pub struct Senses {
    pub hear: Leave,
    pub see: Leave,
    /// Open from the start where the phone needs no asking; on a desk nothing ever arrives.
    pub feel: Leave,
    /// Words, where they must leave the page to become words; see `Kind::Words`.
    pub words: Leave,
    /// The browser can give it a microphone / a camera here, or must be asked before it can
    /// feel how the phone is held (set by platform/web/).
    pub can_hear: bool,
    pub can_see: bool,
    pub can_feel: bool,
    pub can_words: bool,
    offered_hear: bool,
    offered_see: bool,
    offered_feel: bool,
    offered_words: bool,
    since_offer: f32,
    time: f32,

    // ---- hearing, this moment only
    heard: Heard,
    /// The note being held, how long, and how long since the voice stopped.
    note: (f32, f32),
    unvoiced: f32,
    voiced: f32,
    whisper: f32,
    blow: f32,
    blow_wait: f32,
    shout: f32,
    /// Smoothed loudness, to tell a sudden sound from a loud room.
    loud: f32,
    /// How loud the room is with nobody in it: it sinks at once to a quieter moment and
    /// rises only slowly, so a voice is never taken for the room.
    room: f32,
    flinch_wait: f32,
    tremble: f32,
    /// A note it is humming back, and how far into it.
    hum: Option<(f32, f32)>,
    /// Seconds it will not listen for, because it is making a sound of its own.
    deaf: f32,

    // ---- sight, this moment only
    seen: Option<Seen>,
    /// Where it last saw a movement, eased, and how sure it is that someone is there.
    you: (f32, f32),
    present: f32,
    lit: f32,
    dark: f32,
    watched: f32,

    events: Vec<Event>,
    found: Vec<Discovery>,
    pub dirty: bool,
}

impl Default for Senses {
    fn default() -> Self {
        Senses {
            hear: Leave::Closed,
            see: Leave::Closed,
            feel: Leave::Closed,
            words: Leave::Closed,
            can_hear: false,
            can_see: false,
            can_feel: false,
            can_words: false,
            offered_hear: false,
            offered_see: false,
            offered_feel: false,
            offered_words: false,
            since_offer: OFFER_GAP,
            time: 0.0,
            heard: Heard::default(),
            note: (0.0, 0.0),
            unvoiced: 0.0,
            voiced: 0.0,
            whisper: 0.0,
            blow: 0.0,
            blow_wait: 0.0,
            shout: 0.0,
            loud: 0.0,
            room: QUIET,
            flinch_wait: 0.0,
            tremble: 0.0,
            hum: None,
            deaf: 0.0,
            seen: None,
            you: (0.0, 0.0),
            present: 0.0,
            lit: 0.0,
            dark: 0.0,
            watched: 0.0,
            events: Vec::new(),
            found: Vec::new(),
            dirty: false,
        }
    }
}

impl Senses {
    fn leave(&mut self, kind: Kind) -> &mut Leave {
        match kind {
            Kind::Hear => &mut self.hear,
            Kind::See => &mut self.see,
            Kind::Feel => &mut self.feel,
            Kind::Words => &mut self.words,
        }
    }

    pub fn is_open(&self, kind: Kind) -> bool {
        match kind {
            Kind::Hear => self.hear == Leave::Open,
            Kind::See => self.see == Leave::Open,
            Kind::Feel => self.feel == Leave::Open,
            Kind::Words => self.words == Leave::Open,
        }
    }

    /// The player took the word. True if it was up, and the browser should now ask.
    pub fn accept(&mut self, kind: Kind) -> bool {
        let l = self.leave(kind);
        if matches!(l, Leave::Offered(_)) {
            *l = Leave::Opening;
            true
        } else {
            false
        }
    }

    /// Its mark waits in the corner, and a tap on it is the same as on the word. Hearing waits
    /// from the start of every visit, as the one mark that can always be seen (even while its word
    /// is up), until it hears or is told no. Sight waits only once asked this visit and not taken
    /// up (the word went by, or it was stopped since), faint. Words need the line under them, and
    /// feeling has no mark.
    pub fn waiting(&self, kind: Kind) -> bool {
        match kind {
            Kind::Hear => self.can_hear && matches!(self.hear, Leave::Closed | Leave::Offered(_)),
            Kind::See => self.can_see && self.offered_see && self.see == Leave::Closed,
            Kind::Feel | Kind::Words => false,
        }
    }

    /// The player tapped a waiting mark. True if the browser should now ask.
    pub fn accept_waiting(&mut self, kind: Kind) -> bool {
        if !self.waiting(kind) {
            return false;
        }
        *self.leave(kind) = Leave::Opening;
        true
    }

    /// Its word was cut short before it was taken up: it is no longer on offer, so a sight mark
    /// can wait at once.
    pub fn withdraw(&mut self, kind: Kind) {
        let l = self.leave(kind);
        if matches!(l, Leave::Offered(_)) {
            *l = Leave::Closed;
        }
    }

    /// The browser gave it what it asked for.
    pub fn opened(&mut self, kind: Kind) {
        let l = self.leave(kind);
        if *l == Leave::Opening {
            *l = Leave::Open;
        }
    }

    /// The browser could not. `asked_no` when the player (or the page's setting) said no,
    /// which it remembers; otherwise there was simply nothing there, and it may ask next visit.
    pub fn refused(&mut self, kind: Kind, asked_no: bool) {
        *self.leave(kind) = if asked_no { Leave::Refused } else { Leave::Closed };
        if asked_no {
            self.dirty = true;
            self.events.push(Event::Refused(kind));
        }
    }

    /// Stop hearing (or seeing) now. Returns true if it was.
    pub fn close(&mut self, kind: Kind) -> bool {
        let l = self.leave(kind);
        let was = matches!(l, Leave::Open | Leave::Opening);
        if was || matches!(l, Leave::Offered(_)) {
            *l = Leave::Closed;
        }
        match kind {
            Kind::Hear => {
                self.heard = Heard::default();
                self.hum = None;
                self.note = (0.0, 0.0);
                (self.voiced, self.unvoiced, self.whisper, self.shout, self.blow) = (0.0, 0.0, 0.0, 0.0, 0.0);
            }
            Kind::See => {
                self.seen = None;
                self.present = 0.0;
                // opened again, the camera is waking up again
                self.lit = 0.0;
                self.watched = 0.0;
            }
            Kind::Feel | Kind::Words => {}
        }
        if was {
            self.events.push(Event::Closed(kind));
        }
        was
    }

    /// The latest sound, while it may hear.
    pub fn heard(&mut self, h: Heard) {
        if self.hear == Leave::Open {
            self.heard = h;
        }
    }

    /// The latest look, while it may see.
    pub fn seen(&mut self, s: Seen) {
        if self.see == Leave::Open {
            self.seen = Some(s);
        }
    }

    /// It is making a sound (audio.rs) and should not take it for the player's.
    pub fn deafen(&mut self, secs: f32) {
        self.deaf = self.deaf.max(secs);
    }

    /// The note it is humming back now, in Hz, and how loud (0..1), if any.
    pub fn hum(&self) -> Option<(f32, f32)> {
        let (f, t) = self.hum?;
        let k = (t / HUM_SECS).clamp(0.0, 1.0);
        Some((f, (k * 8.0).min(1.0) * (1.0 - k).powf(1.5)))
    }

    /// How dark it wants the room, 0..1 (`World::dim`).
    pub fn dark(&self) -> f32 {
        self.dark
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_found(&mut self) -> Vec<Discovery> {
        std::mem::take(&mut self.found)
    }

    pub fn step(&mut self, dt: f32, world: &mut World, look: &Look) {
        self.time += dt;
        self.since_offer += dt;
        self.flinch_wait = (self.flinch_wait - dt).max(0.0);
        self.step_offers(dt, world, look);
        if self.hear == Leave::Open {
            self.step_hearing(dt, world, look);
        }
        if self.see == Leave::Open {
            self.step_sight(dt, world, look);
        } else {
            self.dark = ease(self.dark, 0.0, -2.0 * dt);
        }
        world.dim = self.dark;
    }

    /// Once a visit for each, late enough in the arc, when nothing else is going on.
    fn step_offers(&mut self, dt: f32, world: &mut World, look: &Look) {
        for kind in [Kind::Feel, Kind::Hear, Kind::See, Kind::Words] {
            if let Leave::Offered(left) = self.leave(kind) {
                *left -= dt;
                if *left <= 0.0 {
                    *self.leave(kind) = Leave::Closed;
                }
                return;
            }
        }
        let calm = self.time >= OFFER_NOT_BEFORE
            && look.still >= OFFER_IDLE
            && !look.busy
            && world.ordinary()
            && self.since_offer >= OFFER_GAP;
        if !calm {
            return;
        }
        // Feeling first: it is the least of the three to be let have.
        let kind = if self.can_feel && !self.offered_feel && self.feel == Leave::Closed && look.stage >= Stage::Regard {
            self.offered_feel = true;
            Kind::Feel
        } else if self.can_hear && !self.offered_hear && self.hear == Leave::Closed && look.stage >= Stage::Echo {
            self.offered_hear = true;
            Kind::Hear
        } else if self.can_see && !self.offered_see && self.see == Leave::Closed && look.stage >= Stage::Initiative {
            self.offered_see = true;
            Kind::See
        } else if self.can_words
            && !self.offered_words
            && self.words == Leave::Closed
            && self.hear == Leave::Open
        {
            // Only once it already hears you, however early that was: words are more than sound.
            self.offered_words = true;
            Kind::Words
        } else {
            return;
        };
        *self.leave(kind) = Leave::Offered(OFFER_WINDOW);
        self.since_offer = 0.0;
        // It turns to whoever it is asking.
        world.reach(-look.fwd);
        world.body.vel *= 0.3;
        self.events.push(Event::Offer(kind));
    }

    fn step_hearing(&mut self, dt: f32, world: &mut World, look: &Look) {
        let ordinary = world.ordinary();
        self.blow_wait = (self.blow_wait - dt).max(0.0);

        // Humming back, it does not listen: it would only hear itself.
        if let Some((f, t)) = &mut self.hum {
            *t += dt;
            let (f, t) = (*f, *t);
            self.tremble += dt;
            if self.tremble > 0.45 && ordinary {
                self.tremble = 0.0;
                world.add_ripple((-look.fwd + look.up * (pitch_height(f) * 1.6 - 0.8)).norm(), 0.18);
            }
            if t >= HUM_SECS {
                self.hum = None;
            }
            return;
        }

        if self.deaf > 0.0 {
            self.deaf -= dt;
            self.note = (0.0, 0.0);
            return;
        }
        let h = self.heard;
        let breath = h.level > BLOW && (h.pitch.is_none() || (h.low > BLOW_LOW && h.clarity < BLOW_CLARITY));
        // Something sudden and loud: it flinches away from the sound. A breath is not sudden.
        if !breath && h.level > 0.22 && h.level > self.loud * 5.0 && self.flinch_wait == 0.0 {
            self.flinch_wait = 2.5;
            world.body.vel += look.fwd * 1.3;
            world.body.squash_vel -= 1.6;
            world.agitation = world.agitation.max(0.35);
            self.found.push(Discovery::Startle);
        }
        self.loud += (h.level - self.loud) * (1.0 - (-3.0 * dt).exp());
        let rise = if h.level < self.room { 4.0 } else { 0.05 };
        self.room = (self.room + (h.level - self.room) * (1.0 - (-rise * dt).exp())).max(QUIET * 0.5);
        let over = self.room * OVER_ROOM;

        let voice = h.pitch.filter(|_| !breath && h.level > VOICE.max(over));
        if let Some(f) = voice {
            self.voiced += dt;
            self.unvoiced = 0.0;
            if self.voiced > 0.5 {
                self.found.push(Discovery::Voice);
            }
            // It trembles where the note lands: low notes low on it, high notes high.
            self.tremble += dt;
            if self.tremble > 0.08 && ordinary {
                self.tremble = 0.0;
                let height = pitch_height(f) * 2.0 - 1.0;
                let wobble = (self.time * 13.0).sin() * 0.35;
                let dir = -look.fwd + look.up * (height * 0.9) + look.right * wobble;
                world.add_ripple(dir.norm(), (h.level * 6.0).clamp(0.12, 0.5));
            }
            // Shouted at, it is upset, and it shakes harder the longer it goes on.
            self.shout = if h.level > SHOUT { self.shout + dt } else { (self.shout - dt).max(0.0) };
            if self.shout > 0.4 && ordinary {
                world.agitation = world.agitation.max((self.shout * 0.5).min(1.0));
                if self.shout > 1.0 {
                    self.found.push(Discovery::Shout);
                }
            }
            // A note held steady, then let go, comes back.
            if (f - self.note.0).abs() < self.note.0 * HELD_STEADY {
                self.note.1 += dt;
                self.note.0 += (f - self.note.0) * 0.1;
            } else {
                self.note = (f, 0.0);
            }
        } else {
            self.unvoiced += dt;
            if self.unvoiced > 0.3 {
                self.voiced = 0.0;
                if self.note.1 >= HELD_SECS {
                    self.hum = Some((hum_pitch(self.note.0), 0.0));
                    self.found.push(Discovery::Sing);
                }
                self.note = (0.0, 0.0);
            }
        }

        // A whisper: rough, quiet, no pitch. It goes still to hear it, and leans in.
        let whisper = voice.is_none() && h.level > QUIET.max(over) && h.level < WHISPER_MAX && h.rough > WHISPER_ROUGH;
        self.whisper = if whisper { self.whisper + dt } else { (self.whisper - dt * 2.0).max(0.0) };
        if self.whisper > 0.25 && ordinary {
            world.body.vel *= (-5.0 * dt).exp();
            world.agitation *= (-4.0 * dt).exp();
            world.reach(-look.fwd);
            // The side nearest you shivers, faintly, as the whisper reaches it.
            self.tremble += dt;
            if self.tremble > 0.3 {
                self.tremble = 0.0;
                let wobble = look.right * ((self.time * 5.0).sin() * 0.3) + look.up * ((self.time * 3.0).cos() * 0.2);
                world.add_ripple((-look.fwd + wobble).norm(), 0.1);
            }
            if self.whisper > 0.9 {
                self.found.push(Discovery::Whisper);
            }
        }

        // A breath blown at it. At once the side nearest you is pressed in, ripple after
        // ripple, and the whole of it is pushed away, harder the harder it is blown; held a
        // moment, it takes a drop off the far side.
        self.blow = if breath { self.blow + dt } else { 0.0 };
        if self.blow > 0.0 && ordinary {
            let push = ((h.level - BLOW) / 0.15).clamp(0.0, 1.0) * 0.6 + 0.4;
            world.body.vel += look.fwd * (push * 3.5 * dt);
            world.agitation = world.agitation.max(0.12 * push);
            self.tremble += dt;
            if self.tremble > 0.07 {
                self.tremble = 0.0;
                let gust = look.right * ((self.time * 11.0).sin() * 0.45) + look.up * ((self.time * 7.0).cos() * 0.35);
                world.add_ripple((-look.fwd + gust).norm(), 0.2 + 0.3 * push);
            }
            if self.blow > 0.8 {
                self.found.push(Discovery::Blow);
            }
        }
        if self.blow > 0.4 && self.blow_wait == 0.0 && ordinary {
            self.blow_wait = 1.1;
            let r = world.body.radius();
            world.body.vel += look.fwd * 0.7;
            let side = look.right * ((self.time * 7.0).sin() * 0.4) + look.up * 0.25;
            let out = (look.fwd + side).norm();
            let color = world.body.color;
            if world.spill(world.body.pos + out * (r * 1.05), out * 1.8, 0.16 * r, color) {
                self.found.push(Discovery::Blow);
            }
        }
    }

    fn step_sight(&mut self, dt: f32, world: &mut World, look: &Look) {
        let Some(s) = self.seen else { return };
        let ease = |k: f32| 1.0 - (-k * dt).exp();
        // The lens covered: the room goes dark with it. Only once it has seen light at all,
        // so a camera that is still waking up does not count.
        if s.light > COVERED * 2.0 {
            self.lit += dt;
        }
        let covered = self.lit > 1.0 && s.light < COVERED;
        self.dark += ((covered as u8 as f32) - self.dark) * ease(if covered { 1.2 } else { 0.8 });
        if self.dark > 0.6 {
            self.found.push(Discovery::Blind);
        }
        if covered {
            self.present *= (-0.5 * dt).exp();
            return;
        }

        // Too fast: it pulls away, as from a hand.
        if s.motion > STARTLE_MOTION && self.lit > 1.0 && self.flinch_wait == 0.0 {
            self.flinch_wait = 2.5;
            world.body.vel += look.fwd * 1.4;
            world.body.squash_vel -= 1.4;
            world.agitation = world.agitation.max(0.3);
            self.found.push(Discovery::Startle);
        }

        // It keeps where it last saw you move, and slowly stops being sure.
        if let (Some(at), true) = (s.at, s.motion > STIR_MOTION) {
            self.you.0 += (at.0 - self.you.0) * ease(2.5);
            self.you.1 += (at.1 - self.you.1) * ease(2.5);
            self.present += (1.0 - self.present) * ease(1.5);
        } else {
            self.present *= (-0.08 * dt).exp();
        }
        if self.present > 0.5 {
            self.found.push(Discovery::Seen);
        }
        // While the hand is away, what it sees decides where it looks. The eye, if there is one,
        // follows the same way (morula.rs reads `gaze_dir`).
        if self.present > 0.15 && look.still > 1.5 && !look.busy {
            let toward = (look.right * self.you.0 + look.up * self.you.1) * 0.9 - look.fwd * 1.6;
            world.gaze_dir = toward.norm();
            if world.ordinary() {
                world.regard = world.regard.max(0.35 * self.present);
            }
        }
        let eye = world.division.morula.as_ref().is_some_and(|m| m.eye.is_some());
        self.watched = if eye && self.present > 0.4 { self.watched + dt } else { 0.0 };
        if self.watched > 2.0 {
            self.found.push(Discovery::Watched);
        }
    }

    /// `hear=no;see=no;feel=no;words=no`: only refusals are kept.
    pub fn save(&self) -> String {
        let no = |l: Leave| if l == Leave::Refused { "no" } else { "" };
        format!("hear={};see={};feel={};words={}", no(self.hear), no(self.see), no(self.feel), no(self.words))
    }

    pub fn load(saved: &str) -> Senses {
        let mut s = Senses::default();
        for part in saved.split(';') {
            match part.trim() {
                "hear=no" => s.hear = Leave::Refused,
                "see=no" => s.see = Leave::Refused,
                "feel=no" => s.feel = Leave::Refused,
                "words=no" => s.words = Leave::Refused,
                _ => {}
            }
        }
        s
    }
}
