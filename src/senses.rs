//! It can be let hear you and see you. It never takes either; late in the arc it asks, once a
//! visit, in one word, and only the player's own tap on that word lets the browser ask for the
//! microphone or the camera (web.rs, devices.rs). On phones that only let a page feel how they
//! are held after asking, it asks for that the same way, a little earlier (`hold the room.`);
//! what it does with it is in motion.rs.
//!
//! - hearing (offered once it echoes, `speak.`): a voice makes the surface tremble where its
//!   pitch lands, high notes high on the body. A note held and then let go comes back as a hum
//!   of the same pitch, low and short; while it hums it does not listen, so it never hears
//!   itself. A whisper and it goes still and leans in. A breath blown at it takes a drop off the
//!   far side. A sudden loud sound and it flinches; a shout kept up and it is upset.
//! - sight (offered once it takes turns, `show yourself.`): only coarse things, never a face or
//!   a picture: how much light there is, how much changed since the last look, and where the
//!   change was. It turns toward wherever you moved last, flinches from a fast movement, and
//!   when the lens is covered the room goes dark with it. The eye, if there is one, follows.
//!
//! Every sound and every frame is reduced here to a few numbers and then dropped; nothing is
//! kept, nothing leaves the browser. Stopping is always one key or one tap away. A refusal is
//! remembered (`save` / `load`) and never asked again.

use crate::contact::Stage;
use crate::math::*;
use crate::progress::Discovery;
use crate::sim::World;

/// How long it waits, from the start of a visit, before it may ask anything (seconds).
const OFFER_NOT_BEFORE: f32 = 20.0;
/// Untouched this long, it may ask.
const OFFER_IDLE: f32 = 5.0;
/// How long the word stays up to be taken.
pub const OFFER_WINDOW: f32 = 9.0;
/// Between asking to hear and asking to see, at least.
const OFFER_GAP: f32 = 40.0;

/// Below this the room is silent, whatever the microphone says.
const QUIET: f32 = 0.004;
/// A voice is loud enough; a breath at the microphone is louder and has no pitch.
const VOICE: f32 = 0.012;
const BLOW: f32 = 0.11;
/// A voice this loud, kept up, is shouting.
const SHOUT: f32 = 0.16;
/// Rough, quiet, unpitched: a whisper. Roughness is the zero-crossing rate, 0..1.
const WHISPER_ROUGH: f32 = 0.09;
const WHISPER_MAX: f32 = 0.06;
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
}

impl Kind {
    /// The one word it asks with.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Hear => "speak.",
            Kind::See => "show yourself.",
            Kind::Feel => "hold the room.",
        }
    }

    /// What it says when the browser is told no.
    pub fn refused(self) -> &'static str {
        match self {
            Kind::Hear => "silence is also something it can learn.",
            Kind::See => "it looks away. it will not ask again.",
            Kind::Feel => "it will stay where it is put.",
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

/// One short window of sound, reduced to what it acts on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Heard {
    /// Root mean square, 0..1.
    pub level: f32,
    /// The fundamental, in Hz, when there is one.
    pub pitch: Option<f32>,
    /// Zero crossings per sample, 0..1: high for breath and hiss, low for a voice.
    pub rough: f32,
}

/// Samples in, three numbers out. Pitch is found by autocorrelation on every other sample.
pub fn hear(samples: &[f32], rate: f32) -> Heard {
    let n = samples.len();
    if n < 64 || rate <= 0.0 {
        return Heard::default();
    }
    let level = (samples.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
    let crossings = samples.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
    let rough = crossings as f32 / n as f32;
    let pitch = if level < QUIET { None } else { fundamental(samples, rate) };
    Heard { level, pitch, rough }
}

fn fundamental(samples: &[f32], rate: f32) -> Option<f32> {
    // Every other sample is plenty for a voice, and a quarter of the work.
    let x: Vec<f32> = samples.chunks_exact(2).map(|p| (p[0] + p[1]) * 0.5).take(1024).collect();
    let rate = rate * 0.5;
    let n = x.len();
    let lo = (rate / PITCH_HI) as usize;
    let hi = ((rate / PITCH_LO) as usize).min(n / 2);
    if lo < 2 || hi <= lo + 2 {
        return None;
    }
    let corr = |lag: usize| {
        let (mut xy, mut xx, mut yy) = (0.0, 0.0, 0.0);
        for i in 0..n - lag {
            xy += x[i] * x[i + lag];
            xx += x[i] * x[i];
            yy += x[i + lag] * x[i + lag];
        }
        xy / (xx * yy).sqrt().max(1e-9)
    };
    let c: Vec<f32> = (lo..=hi).map(corr).collect();
    let best = c.iter().cloned().fold(f32::MIN, f32::max);
    if best < 0.82 {
        return None;
    }
    // The first peak nearly as good as the best, so a note is not heard an octave low.
    let i = (1..c.len() - 1).find(|&i| c[i] >= best * 0.95 && c[i] >= c[i - 1] && c[i] >= c[i + 1])?;
    // Between samples: fit a parabola through the peak.
    let (a, b, d) = (c[i - 1], c[i], c[i + 1]);
    let shift = 0.5 * (a - d) / (a - 2.0 * b + d).min(-1e-6);
    Some(rate / (lo as f32 + i as f32 + shift))
}

/// One look, reduced to what it acts on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seen {
    /// Mean light, 0..1.
    pub light: f32,
    /// How much of the picture changed since the last look, 0..1.
    pub motion: f32,
    /// Where it changed, -1..1 each way, as the player sees it (left is their left, up is up).
    pub at: Option<(f32, f32)>,
}

/// Keeps only the last look's light, to see what changed. No picture is kept past one look.
#[derive(Default)]
pub struct Retina {
    last: Vec<u8>,
}

impl Retina {
    /// `rgba` is a tiny frame, `w` x `h`, as the camera faces: it is mirrored here.
    pub fn look(&mut self, rgba: &[u8], w: usize, h: usize) -> Seen {
        if w == 0 || h == 0 || rgba.len() < w * h * 4 {
            return Seen::default();
        }
        let luma: Vec<u8> = rgba
            .chunks_exact(4)
            .take(w * h)
            .map(|p| ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8)
            .collect();
        let light = luma.iter().map(|&l| l as f32).sum::<f32>() / (w * h) as f32 / 255.0;
        let mut seen = Seen { light, motion: 0.0, at: None };
        if self.last.len() == luma.len() {
            let (mut sum, mut wx, mut wy, mut weight) = (0.0, 0.0, 0.0, 0.0);
            for (i, (&a, &b)) in luma.iter().zip(&self.last).enumerate() {
                let d = (a as f32 - b as f32).abs() / 255.0;
                sum += d;
                // Small changes are the sensor's own noise.
                if d > 0.08 {
                    let (x, y) = ((i % w) as f32, (i / w) as f32);
                    wx += x * d;
                    wy += y * d;
                    weight += d;
                }
            }
            seen.motion = sum / luma.len() as f32;
            if weight > 0.5 {
                let x = wx / weight / (w - 1).max(1) as f32 * 2.0 - 1.0;
                let y = wy / weight / (h - 1).max(1) as f32 * 2.0 - 1.0;
                // The camera faces the player, so their left is its right; and rows run down.
                seen.at = Some((-x, -y));
            }
        }
        self.last = luma;
        seen
    }

    pub fn forget(&mut self) {
        self.last.clear();
    }
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
    /// The browser can give it a microphone / a camera here, or must be asked before it can
    /// feel how the phone is held (set by web.rs).
    pub can_hear: bool,
    pub can_see: bool,
    pub can_feel: bool,
    offered_hear: bool,
    offered_see: bool,
    offered_feel: bool,
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
            can_hear: false,
            can_see: false,
            can_feel: false,
            offered_hear: false,
            offered_see: false,
            offered_feel: false,
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
        }
    }

    pub fn is_open(&self, kind: Kind) -> bool {
        match kind {
            Kind::Hear => self.hear == Leave::Open,
            Kind::See => self.see == Leave::Open,
            Kind::Feel => self.feel == Leave::Open,
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
            }
            Kind::See => {
                self.seen = None;
                self.present = 0.0;
            }
            Kind::Feel => {}
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

    /// How dark it wants the room, 0..1 (sim.rs `dim`).
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
        self.step_offers(dt, world, look);
        if self.hear == Leave::Open {
            self.step_hearing(dt, world, look);
        }
        if self.see == Leave::Open {
            self.step_sight(dt, world, look);
        } else {
            self.dark += (0.0 - self.dark) * (1.0 - (-2.0 * dt).exp());
        }
        world.dim = self.dark;
    }

    /// Once a visit for each, late enough in the arc, when nothing else is going on.
    fn step_offers(&mut self, dt: f32, world: &mut World, look: &Look) {
        for kind in [Kind::Feel, Kind::Hear, Kind::See] {
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
        self.flinch_wait = (self.flinch_wait - dt).max(0.0);

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
                self.loud = 0.0;
            }
            return;
        }

        if self.deaf > 0.0 {
            self.deaf -= dt;
            self.note = (0.0, 0.0);
            self.loud = 0.0;
            return;
        }
        let h = self.heard;
        // Something sudden and loud: it flinches away from the sound.
        if h.level > 0.22 && h.level > self.loud * 5.0 && self.flinch_wait == 0.0 {
            self.flinch_wait = 2.5;
            world.body.vel += look.fwd * 1.3;
            world.body.squash_vel -= 1.6;
            world.agitation = world.agitation.max(0.35);
            self.found.push(Discovery::Startle);
        }
        self.loud += (h.level - self.loud) * (1.0 - (-3.0 * dt).exp());

        let voice = h.pitch.filter(|_| h.level > VOICE);
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
        let whisper = voice.is_none() && h.level > QUIET * 1.5 && h.level < WHISPER_MAX && h.rough > WHISPER_ROUGH;
        self.whisper = if whisper { self.whisper + dt } else { (self.whisper - dt * 2.0).max(0.0) };
        if self.whisper > 0.35 && ordinary {
            world.body.vel *= (-5.0 * dt).exp();
            world.agitation *= (-4.0 * dt).exp();
            world.reach(-look.fwd);
            if self.whisper > 1.2 {
                self.found.push(Discovery::Whisper);
            }
        }

        // A breath blown at it: loud, no pitch, held a moment. It takes a drop off the far side.
        self.blow = if voice.is_none() && h.level > BLOW { self.blow + dt } else { 0.0 };
        if self.blow > 0.25 && self.blow_wait == 0.0 && ordinary {
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
        self.flinch_wait = (self.flinch_wait - dt).max(0.0);

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

    /// `hear=no;see=no;feel=no`: only refusals are kept.
    pub fn save(&self) -> String {
        let no = |l: Leave| if l == Leave::Refused { "no" } else { "" };
        format!("hear={};see={};feel={}", no(self.hear), no(self.see), no(self.feel))
    }

    pub fn load(saved: &str) -> Senses {
        let mut s = Senses::default();
        for part in saved.split(';') {
            match part.trim() {
                "hear=no" => s.hear = Leave::Refused,
                "see=no" => s.see = Leave::Refused,
                "feel=no" => s.feel = Leave::Refused,
                _ => {}
            }
        }
        s
    }
}

/// Where a pitch lands on the body, 0 (low) to 1 (high), by octaves.
fn pitch_height(f: f32) -> f32 {
    ((f / PITCH_LO).log2() / (PITCH_HI / PITCH_LO).log2()).clamp(0.0, 1.0)
}

/// The same note, brought down into its own low register.
fn hum_pitch(mut f: f32) -> f32 {
    while f > 262.0 {
        f *= 0.5;
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn tone(f: f32, amp: f32) -> Vec<f32> {
        (0..2048).map(|i| (i as f32 / RATE * f * std::f32::consts::TAU).sin() * amp).collect()
    }

    /// A hiss: the same every time.
    fn hiss(amp: f32) -> Vec<f32> {
        let mut s: u32 = 0x1234_5678;
        (0..2048)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                (s as f32 / u32::MAX as f32 * 2.0 - 1.0) * amp
            })
            .collect()
    }

    fn look(stage: Stage, still: f32) -> Look {
        Look { stage, still, busy: false, fwd: v3(0.0, 0.0, -1.0), right: v3(1.0, 0.0, 0.0), up: V3::UP }
    }

    fn run(s: &mut Senses, w: &mut World, l: &Look, seconds: f32, mut feed: impl FnMut(&mut Senses)) -> Vec<Discovery> {
        let mut found = Vec::new();
        for _ in 0..(seconds * 60.0) as usize {
            feed(s);
            s.step(1.0 / 60.0, w, l);
            w.step(1.0 / 60.0);
            found.extend(s.drain_found());
        }
        found
    }

    fn open(kind: Kind) -> Senses {
        let mut s = Senses { can_hear: true, can_see: true, ..Default::default() };
        *s.leave(kind) = Leave::Open;
        s
    }

    #[test]
    fn a_voice_has_a_pitch_and_a_hiss_does_not() {
        for f in [110.0, 220.0, 440.0, 660.0] {
            let h = hear(&tone(f, 0.2), RATE);
            let p = h.pitch.expect("a pitch");
            assert!((p - f).abs() < f * 0.02, "{f} heard as {p}");
            assert!(h.rough < 0.05);
        }
        let h = hear(&hiss(0.05), RATE);
        assert_eq!(h.pitch, None);
        assert!(h.rough > WHISPER_ROUGH, "{}", h.rough);
        assert_eq!(hear(&[0.0; 2048], RATE).level, 0.0);
    }

    #[test]
    fn it_asks_late_once_a_visit_and_only_when_left_alone() {
        let mut w = World::new();
        let mut s = Senses { can_hear: true, can_see: true, ..Default::default() };
        run(&mut s, &mut w, &look(Stage::Regard, 30.0), 40.0, |_| {});
        assert!(s.drain_events().is_empty(), "too early in the arc");
        run(&mut s, &mut w, &look(Stage::Echo, 0.5), 5.0, |_| {});
        assert!(s.drain_events().is_empty(), "not while being touched");
        run(&mut s, &mut w, &look(Stage::Echo, 30.0), 1.0, |_| {});
        assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)]);
        run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_WINDOW + 1.0, |_| {});
        assert_eq!(s.hear, Leave::Closed, "the word went untaken");
        assert!(s.drain_events().is_empty(), "and it does not ask to see straight after");
        run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_GAP - OFFER_WINDOW, |_| {});
        assert_eq!(s.drain_events(), vec![Event::Offer(Kind::See)]);
        assert!(!s.accept(Kind::Hear), "the first word is gone");
        assert!(s.accept(Kind::See));
        run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 200.0, |_| {});
        assert!(s.drain_events().is_empty(), "once a visit");
    }

    #[test]
    fn it_asks_to_feel_first_and_earlier() {
        let mut w = World::new();
        let mut s = Senses { can_hear: true, can_feel: true, ..Default::default() };
        run(&mut s, &mut w, &look(Stage::Matter, 30.0), 40.0, |_| {});
        assert!(s.drain_events().is_empty(), "not before regard");
        run(&mut s, &mut w, &look(Stage::Echo, 30.0), 1.0, |_| {});
        assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Feel)]);
        assert_eq!(Kind::Feel.word(), "hold the room.");
        assert!(s.accept(Kind::Feel));
        s.refused(Kind::Feel, true);
        assert_eq!(s.drain_events(), vec![Event::Refused(Kind::Feel)]);
        run(&mut s, &mut w, &look(Stage::Echo, 30.0), OFFER_GAP, |_| {});
        assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)], "then, later, to hear");
        assert_eq!(Senses::load(&s.save()).feel, Leave::Refused);
        assert_eq!(Senses::load("hear=no;see=").feel, Leave::Closed, "an older save still loads");
    }

    #[test]
    fn where_it_cannot_hear_it_never_asks() {
        let mut w = World::new();
        let mut s = Senses::default();
        run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 200.0, |_| {});
        assert!(s.drain_events().is_empty());
    }

    #[test]
    fn a_refusal_is_kept_and_never_asked_again() {
        let mut s = Senses { can_hear: true, ..Default::default() };
        s.hear = Leave::Offered(3.0);
        assert!(s.accept(Kind::Hear));
        s.refused(Kind::Hear, true);
        assert_eq!(s.drain_events(), vec![Event::Refused(Kind::Hear)]);
        let mut next = Senses::load(&s.save());
        assert_eq!(next.hear, Leave::Refused);
        assert_eq!(next.see, Leave::Closed);
        next.can_hear = true;
        let mut w = World::new();
        run(&mut next, &mut w, &look(Stage::Echo, 30.0), 200.0, |_| {});
        assert!(next.drain_events().is_empty());
        // A missing microphone is not a no.
        let mut s = Senses::default();
        s.hear = Leave::Opening;
        s.refused(Kind::Hear, false);
        assert_eq!((s.hear, s.save().as_str()), (Leave::Closed, "hear=;see=;feel="));
    }

    #[test]
    fn a_voice_makes_it_tremble_high_or_low() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(880.0, 0.1), RATE)));
        assert!(found.contains(&Discovery::Voice));
        assert!(w.ripples.len() > 5 && w.ripples.iter().all(|r| r.dir.y > 0.3), "high on it");
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(80.0, 0.1), RATE)));
        assert!(w.ripples.iter().all(|r| r.dir.y < -0.3), "low on it");
    }

    #[test]
    fn a_held_note_comes_back_as_a_hum_and_it_does_not_hear_itself() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        let l = look(Stage::Echo, 0.0);
        run(&mut s, &mut w, &l, 1.2, |s| s.heard(hear(&tone(440.0, 0.1), RATE)));
        assert_eq!(s.hum(), None, "not while you are still singing");
        let found = run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
        assert!(found.contains(&Discovery::Sing));
        let (f, _) = s.hum().expect("humming");
        assert!((f - 220.0).abs() < 5.0, "the same note, lower: {f}");
        // its own hum, picked up by the microphone, is not a new note
        run(&mut s, &mut w, &l, HUM_SECS, |s| s.heard(hear(&tone(220.0, 0.1), RATE)));
        let found = run(&mut s, &mut w, &l, 0.6, |s| s.heard(Heard::default()));
        assert!(!found.contains(&Discovery::Sing) && s.hum().is_none());
        // a note too short is not held
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        run(&mut s, &mut w, &l, 0.4, |s| s.heard(hear(&tone(300.0, 0.1), RATE)));
        run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
        assert_eq!(s.hum(), None);
    }

    #[test]
    fn a_whisper_stills_it_and_a_breath_takes_a_drop() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        w.agitation = 1.0;
        let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&hiss(0.03), RATE)));
        assert!(found.contains(&Discovery::Whisper));
        assert!(w.agitation < 0.1 && w.gaze > 0.5, "still, and leaning in");
        assert!(w.drops.is_empty());

        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 0.6, |s| s.heard(hear(&hiss(0.4), RATE)));
        assert!(found.contains(&Discovery::Blow));
        assert_eq!(w.drops.len(), 1);
        assert!(w.drops[0].pos.z < -0.5, "off the far side, away from you");
    }

    #[test]
    fn its_own_sounds_are_not_yours() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        let l = look(Stage::Echo, 0.0);
        s.deafen(1.5);
        let found = run(&mut s, &mut w, &l, 1.4, |s| s.heard(hear(&tone(659.0, 0.1), RATE)));
        run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
        assert!(found.is_empty() && s.hum().is_none());
    }

    #[test]
    fn shouted_at_it_is_upset() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&tone(300.0, 0.4), RATE)));
        assert!(found.contains(&Discovery::Shout));
        assert!(w.agitation > 0.5, "{}", w.agitation);
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&tone(300.0, 0.1), RATE)));
        assert!(w.agitation < 0.1, "a voice is not a shout");
    }

    #[test]
    fn closed_it_hears_nothing() {
        let (mut w, mut s) = (World::new(), open(Kind::Hear));
        assert!(s.close(Kind::Hear));
        assert_eq!(s.drain_events(), vec![Event::Closed(Kind::Hear)]);
        let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(440.0, 0.2), RATE)));
        assert!(found.is_empty() && w.ripples.is_empty());
        assert!(!s.close(Kind::Hear));
    }

    /// A tiny frame: grey, with a bright square at (x, y).
    fn frame(light: u8, square: Option<(usize, usize)>) -> Vec<u8> {
        let (w, h) = (32, 24);
        let mut px = vec![light; w * h * 4];
        if let Some((sx, sy)) = square {
            for y in sy..(sy + 5).min(h) {
                for x in sx..(sx + 5).min(w) {
                    let i = (y * w + x) * 4;
                    px[i..i + 3].copy_from_slice(&[250, 250, 250]);
                }
            }
        }
        px
    }

    #[test]
    fn it_sees_light_and_where_things_moved_as_you_see_them() {
        let mut r = Retina::default();
        let first = r.look(&frame(100, None), 32, 24);
        assert!((first.light - 100.0 / 255.0).abs() < 0.02);
        assert_eq!(first.at, None, "nothing to compare with yet");
        // something appears on the camera's right, near the top: the player's left, up
        let s = r.look(&frame(100, Some((26, 1))), 32, 24);
        let (x, y) = s.at.expect("it moved");
        assert!(x < -0.5 && y > 0.5, "{x} {y}");
        assert!(s.motion > 0.0);
        let s = r.look(&frame(100, Some((26, 1))), 32, 24);
        assert_eq!((s.motion, s.at), (0.0, None), "nothing moved");
    }

    #[test]
    fn it_turns_toward_you_and_flinches_from_fast_movement() {
        let (mut w, mut s) = (World::new(), open(Kind::See));
        let l = look(Stage::Initiative, 5.0);
        let mut r = Retina::default();
        let mut k = 0;
        let found = run(&mut s, &mut w, &l, 2.0, |s| {
            k += 1;
            // a small movement, to the player's right
            s.seen(r.look(&frame(120, Some((2 + k % 2, 10))), 32, 24));
        });
        assert!(found.contains(&Discovery::Seen));
        assert!(w.gaze_dir.x > 0.2 && w.gaze_dir.z > 0.5, "toward you, on your right: {:?}", w.gaze_dir);
        assert!(!found.contains(&Discovery::Startle));
        let found = run(&mut s, &mut w, &l, 0.2, |s| s.seen(Seen { light: 0.5, motion: 0.4, at: Some((0.0, 0.0)) }));
        assert!(found.contains(&Discovery::Startle));
        assert!(w.body.vel.z < 0.0, "away from you");
    }

    #[test]
    fn covered_the_room_goes_dark_and_comes_back() {
        let (mut w, mut s) = (World::new(), open(Kind::See));
        let l = look(Stage::Initiative, 5.0);
        let found = run(&mut s, &mut w, &l, 0.5, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
        assert!(found.is_empty(), "a camera still waking up is not a covered one");
        run(&mut s, &mut w, &l, 1.5, |s| s.seen(Seen { light: 0.5, motion: 0.0, at: None }));
        let found = run(&mut s, &mut w, &l, 3.0, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
        assert!(found.contains(&Discovery::Blind));
        assert!(w.night > 0.5, "{}", w.night);
        run(&mut s, &mut w, &l, 8.0, |s| s.seen(Seen { light: 0.5, motion: 0.0, at: None }));
        assert!(w.night < 0.1, "{}", w.night);
        // and closed, it lets the room go
        run(&mut s, &mut w, &l, 3.0, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
        s.close(Kind::See);
        run(&mut s, &mut w, &l, 8.0, |_| {});
        assert!(w.night < 0.1);
    }
}
