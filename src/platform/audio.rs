//! Every sound is synthesized: a few sine and triangle tones, no samples.

use crate::game::{
    become_tones, memory, memory_deaf, Sound, Strain, Tone, Wave, OWN_SOUND_DEAF, SCALE, STRAIN_RELEASE,
};
use crate::sim::World;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, AudioParam, GainNode, OscillatorNode, OscillatorType};

/// The step of the scale nearest `pitch`, 0..1 low to high, in semitones.
fn step(pitch: f32) -> f32 {
    SCALE[((pitch * (SCALE.len() - 1) as f32).round() as usize).min(SCALE.len() - 1)]
}

/// Each of the burst's pieces has its own note: the larger, the lower.
fn shard_note(i: usize) -> f32 {
    1400.0 / crate::burst::SIZES[i] * 2f32.powf(SCALE[i % SCALE.len()] / 24.0)
}

/// One of its signs, said: a soft sine gliding up into the note of the piece of the twelve it
/// shares a place with, quieter the higher it is (0.03 – 0.05).
fn sign_tone(i: usize, delay: f32) -> Tone {
    let f = shard_note(i);
    Tone {
        wave: Wave::Sine,
        f0: f * 0.94,
        f1: f,
        gain: (0.05 * 400.0 / f).clamp(0.03, 0.05),
        decay: 0.7,
        delay: f64::from(delay),
    }
}

/// The master level, unmuted.
const LEVEL: f32 = 0.7;
/// Leaving or coming back, the whole mix fades this fast (time constant, seconds), so a tone
/// cut off mid-swing never clicks.
const FADE: f64 = 0.012;
/// How long after the fade begins the context is suspended (ms): several time constants.
const FADE_MS: i32 = 90;
/// The note it keeps for you, played back once on a return: how loud, and how long it rings.
const KEPT_GAIN: f32 = 0.015;
const KEPT_SECS: f64 = 2.4;

/// How the phone plays the page's sound, where the browser lets it be said (Safari's
/// `navigator.audioSession`): at rest, as a page's quiet sounds (`ambient`: the ringer switch
/// silences it, and someone else's music goes on under it); while the microphone is open, as a
/// call on the speaker (`play-and-record`), or a note sung back would come out of the earpiece.
/// Elsewhere, or if the browser will not, nothing changes and nothing is said.
pub fn route(listening: bool) {
    let Some(w) = web_sys::window() else { return };
    let session = super::get(&w.navigator(), "audioSession");
    if !session.is_object() {
        return;
    }
    let kind = if listening { "play-and-record" } else { "ambient" };
    let _ = js_sys::Reflect::set(&session, &JsValue::from_str("type"), &JsValue::from_str(kind));
}

/// A continuous sound's frequency and gain, and where each was last sent.
struct Voice {
    freq: Glide,
    gain: Glide,
}

/// One parameter that glides toward a target. Sending the same target with the same time
/// constant again would draw the same curve, so it is only sent when one of them changes.
struct Glide {
    param: AudioParam,
    last: Cell<(f32, f64)>,
}

impl Glide {
    fn new(param: AudioParam) -> Glide {
        Glide { param, last: Cell::new((f32::NAN, 0.0)) }
    }

    fn toward(&self, ctx: &AudioContext, target: f32, tau: f64) {
        if self.last.get() == (target, tau) {
            return;
        }
        self.last.set((target, tau));
        let _ = self.param.set_target_at_time(target, ctx.current_time(), tau);
    }
}

/// The pull's tone (`game::strain`): a sine and, close to tearing, a second one beating against
/// it, both through one gain, wavering through another that a slow sine moves.
struct Strained {
    voice: Voice,
    second: Voice,
    /// Where the wavering rests (1 less its depth) and the slow sine that moves it: its rate and depth.
    still: Glide,
    rate: Glide,
    depth: Glide,
}

pub struct Audio {
    ctx: AudioContext,
    master: GainNode,
    /// The hum while it is held still, and the low drone of the night.
    warm: Voice,
    drone: Glide,
    /// A letter held down, sung back (keys.rs).
    hum: Voice,
    /// A note the player held, hummed back (senses/).
    echo: Voice,
    /// A pull, held.
    strain: Strained,
    /// The last sound it made: when it began, and its tones, to come back when something is found.
    last: RefCell<(f64, Vec<Tone>)>,
    pub muted: bool,
    /// Nobody is looking: faded out, and suspended once the fade is done unless they came back.
    away: Rc<Cell<bool>>,
}

impl Audio {
    /// Must be called from inside a user gesture, or the browser keeps it silent.
    pub fn new() -> Result<Audio, JsValue> {
        let ctx = AudioContext::new()?;
        let master = ctx.create_gain()?;
        master.gain().set_value(LEVEL);
        master.connect_with_audio_node(&ctx.destination())?;

        let warm = Self::voice(&ctx, &master, OscillatorType::Sine, 196.0)?;
        let drone = Self::voice(&ctx, &master, OscillatorType::Sine, 55.0)?;
        let b = ctx.create_oscillator()?;
        b.frequency().set_value(82.6);
        b.connect_with_audio_node(&drone.1)?;
        b.start()?;
        let hum = Self::voice(&ctx, &master, OscillatorType::Sine, 130.8)?;
        let echo = Self::voice(&ctx, &master, OscillatorType::Sine, 110.0)?;
        let glides = |(osc, g): (OscillatorNode, GainNode)| Voice {
            freq: Glide::new(osc.frequency()),
            gain: Glide::new(g.gain()),
        };
        let strain = Self::strained(&ctx, &master)?;
        Ok(Audio {
            master,
            warm: glides(warm),
            drone: Glide::new(drone.1.gain()),
            hum: glides(hum),
            echo: glides(echo),
            strain,
            ctx,
            last: RefCell::new((0.0, Vec::new())),
            muted: false,
            away: Rc::new(Cell::new(false)),
        })
    }

    /// A continuously running oscillator behind a silent gain.
    fn voice(
        ctx: &AudioContext,
        out: &GainNode,
        kind: OscillatorType,
        freq: f32,
    ) -> Result<(OscillatorNode, GainNode), JsValue> {
        let osc = ctx.create_oscillator()?;
        osc.set_type(kind);
        osc.frequency().set_value(freq);
        let g = ctx.create_gain()?;
        g.gain().set_value(0.0);
        osc.connect_with_audio_node(&g)?;
        g.connect_with_audio_node(out)?;
        osc.start()?;
        Ok((osc, g))
    }

    /// The pull's voice: two sines into one silent gain, through a gain a slow sine wavers.
    fn strained(ctx: &AudioContext, out: &GainNode) -> Result<Strained, JsValue> {
        let waver = ctx.create_gain()?;
        waver.gain().set_value(1.0);
        waver.connect_with_audio_node(out)?;
        let (osc, g) = Self::voice(ctx, &waver, OscillatorType::Sine, 220.0)?;
        // the second sine goes through its own level into the same gain
        let (b, level) = Self::voice(ctx, &g, OscillatorType::Sine, 224.0)?;
        let (lfo, depth) = Self::voice(ctx, &waver, OscillatorType::Sine, 3.0)?;
        depth.disconnect()?;
        depth.connect_with_audio_param(&waver.gain())?;
        Ok(Strained {
            voice: Voice { freq: Glide::new(osc.frequency()), gain: Glide::new(g.gain()) },
            second: Voice { freq: Glide::new(b.frequency()), gain: Glide::new(level.gain()) },
            still: Glide::new(waver.gain()),
            rate: Glide::new(lfo.frequency()),
            depth: Glide::new(depth.gain()),
        })
    }

    fn level(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            LEVEL
        }
    }

    /// Touch screens only grant audio on pointerup, so this is retried on every release.
    /// Back from away, the sound fades in where it stopped instead of starting mid-swing.
    pub fn resume(&self) {
        if self.away.replace(false) {
            let g = self.master.gain();
            let t = self.ctx.current_time();
            let _ = g.cancel_scheduled_values(t);
            let _ = g.set_target_at_time(self.level(), t, FADE);
        }
        let _ = self.ctx.resume();
    }

    /// Nobody is looking at the page: everything it was sounding fades in a breath and then
    /// stops where it is. Cut off at once, a tone would click.
    pub fn suspend(&self) {
        if self.away.replace(true) {
            return;
        }
        let g = self.master.gain();
        let t = self.ctx.current_time();
        let _ = g.cancel_scheduled_values(t);
        let _ = g.set_target_at_time(0.0, t, FADE);
        let (ctx, away) = (self.ctx.clone(), self.away.clone());
        let later = Closure::once_into_js(move || {
            if away.get() {
                let _ = ctx.suspend();
            }
        });
        let sent = web_sys::window()
            .map(|w| w.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), FADE_MS));
        if !matches!(sent, Some(Ok(_))) {
            let _ = self.ctx.suspend();
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let t = self.ctx.current_time();
        let _ = self.master.gain().set_target_at_time(self.level(), t, 0.1);
    }

    /// One plucked tone, kept as part of the last sound it made.
    fn pluck(&self, kind: OscillatorType, f0: f32, f1: f32, gain: f32, decay: f64, delay: f64) -> Result<(), JsValue> {
        let wave = if kind == OscillatorType::Triangle { Wave::Triangle } else { Wave::Sine };
        let tone = Tone { wave, f0, f1, gain, decay, delay };
        self.last.borrow_mut().1.push(tone);
        self.tone(tone)
    }

    /// One tone: a quick attack, a slide from `f0` to `f1`, and an exponential fade.
    fn tone(&self, Tone { wave, f0, f1, gain, decay, delay }: Tone) -> Result<(), JsValue> {
        let t = self.ctx.current_time() + delay;
        let osc = self.ctx.create_oscillator()?;
        osc.set_type(match wave {
            Wave::Sine => OscillatorType::Sine,
            Wave::Triangle => OscillatorType::Triangle,
        });
        osc.frequency().set_value_at_time(f0, t)?;
        osc.frequency().exponential_ramp_to_value_at_time(f1, t + decay * 0.5)?;
        let g = self.ctx.create_gain()?;
        g.gain().set_value_at_time(0.0001, t)?;
        g.gain().exponential_ramp_to_value_at_time(gain, t + 0.008)?;
        g.gain().exponential_ramp_to_value_at_time(0.0001, t + decay)?;
        osc.connect_with_audio_node(&g)?;
        g.connect_with_audio_node(&self.master)?;
        osc.start_with_when(t)?;
        osc.stop_with_when(t + decay + 0.05)?;
        Ok(())
    }

    /// Plays one sound, and says how long a microphone should not listen (seconds from now).
    pub fn play(&self, s: Sound) -> f32 {
        if s == Sound::Discover {
            return self.remember();
        }
        // A phrase of signs is one sound: only its first begins it.
        if !matches!(s, Sound::Sign { first: false, .. }) {
            *self.last.borrow_mut() = (self.ctx.current_time(), Vec::new());
        }
        use OscillatorType::*;
        let _ = match s {
            Sound::Poke { pitch, size } => {
                let f = 220.0 / size * 2f32.powf(step(pitch) / 12.0);
                self.pluck(Sine, f, f * 0.96, 0.22, 0.9, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.76, f * 2.7, 0.035, 0.35, 0.0))
            }
            Sound::Tear => self.pluck(Sine, 320.0, 1150.0, 0.16, 0.45, 0.0),
            Sound::Merge => self
                .pluck(Sine, 110.0, 108.0, 0.16, 1.6, 0.0)
                .and_then(|_| self.pluck(Sine, 164.8, 163.0, 0.09, 1.4, 0.03)),
            Sound::Discover => Ok(()),
            Sound::Shard { i } => {
                // the hard ones ring, the soft ones thud
                let f = shard_note(i);
                let (give, decay) = crate::burst::CONSISTENCIES[i].give();
                let ring = f64::from(2.6 / decay);
                let kind = if decay > 6.0 { Triangle } else { Sine };
                self.pluck(kind, f, f * (1.0 - 0.04 * give), 0.16, ring, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.01, f * 2.0, 0.03 / decay.max(1.0), ring * 0.6, 0.0))
            }
            Sound::Chord => (0..crate::burst::SHARDS).try_for_each(|i| {
                let f = shard_note(i);
                self.pluck(Sine, f, f, 0.035, 3.2, i as f64 * 0.045)
            }),
            // up the scale with the river, a row raised in one drag climbs
            Sound::Column { slot } => {
                let row = slot / crate::burst::river::LANES;
                let f = 196.0 * 2f32.powf(SCALE[row % SCALE.len()] / 12.0);
                self.pluck(Sine, f * 0.5, f, 0.06, 0.6, 0.0)
                    .and_then(|_| self.pluck(Triangle, f * 2.0, f * 2.0, 0.015, 0.9, 0.05))
            }
            // two low thumps, the second softer
            Sound::Beat => {
                self.pluck(Sine, 62.0, 44.0, 0.32, 0.35, 0.0).and_then(|_| self.pluck(Sine, 58.0, 42.0, 0.2, 0.3, 0.2))
            }
            Sound::Call => self.pluck(Sine, 146.8, 196.0, 0.08, 0.7, 0.0),
            Sound::Pinch => self.pluck(Sine, 1046.5, 1174.7, 0.05, 0.3, 0.0),
            Sound::Almost => self.pluck(Sine, 1046.5, 880.0, 0.018, 0.25, 0.0),
            Sound::Bloom => self
                .pluck(Sine, 523.3, 523.3, 0.05, 2.6, 0.0)
                .and_then(|_| self.pluck(Sine, 659.3, 659.3, 0.04, 2.6, 0.18))
                .and_then(|_| self.pluck(Sine, 784.0, 784.0, 0.035, 2.8, 0.36))
                .and_then(|_| self.pluck(Sine, 1046.5, 1046.5, 0.025, 3.0, 0.54)),
            // Every pluck is an overtone of the same low string.
            Sound::Pluck { harmonic } => {
                let f = 98.0 * harmonic as f32;
                self.pluck(Triangle, f, f, 0.16, 3.2, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.0, f * 2.0, 0.04, 1.6, 0.0))
            }
            Sound::Stir => self.pluck(Sine, 110.0, 220.0, 0.08, 1.4, 0.0),
            // A knock, and a moment later the knock from inside.
            Sound::Knock => self
                .pluck(Triangle, 92.0, 70.0, 0.3, 0.22, 0.0)
                .and_then(|_| self.pluck(Triangle, 82.0, 64.0, 0.14, 0.22, 0.55)),
            Sound::Lid => self
                .pluck(Sine, 196.0, 196.0, 0.07, 3.0, 0.0)
                .and_then(|_| self.pluck(Sine, 293.7, 293.7, 0.06, 3.0, 0.25))
                .and_then(|_| self.pluck(Sine, 392.0, 392.0, 0.05, 3.2, 0.5))
                .and_then(|_| self.pluck(Sine, 587.3, 587.3, 0.04, 3.4, 0.75)),
            // the same low sound for every form, each at its own step of the scale
            Sound::Become { form } => {
                become_tones(form).into_iter().try_for_each(|t| self.pluck(Sine, t.f0, t.f1, t.gain, t.decay, t.delay))
            }
            // The other one: hard, so a triangle, with a bell's partial over it; high struck, low arriving.
            Sound::Ring { pitch } => {
                let f = 329.6 * 2f32.powf(step(pitch) / 12.0);
                self.pluck(Triangle, f, f * 0.998, 0.1, 2.8, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.76, f * 2.75, 0.025, 1.4, 0.0))
            }
            Sound::Sign { i, delay, .. } => {
                let t = sign_tone(i, delay);
                self.pluck(Sine, t.f0, t.f1, t.gain, t.decay, t.delay)
            }
            // Its note for you, hardly there: the quietest thing it plays.
            Sound::Kept { f } => self.pluck(Sine, f, f, KEPT_GAIN, KEPT_SECS, 0.0),
            Sound::Provoked => self
                .pluck(Triangle, 73.4, 69.0, 0.25, 1.4, 0.0)
                .and_then(|_| self.pluck(Triangle, 77.8, 72.0, 0.2, 1.3, 0.0)),
        };
        // A sign said later is listened over until it, too, has sounded.
        match s {
            Sound::Sign { delay, .. } => delay + OWN_SOUND_DEAF,
            // A held tone of its own: listened over until it has gone.
            Sound::Kept { .. } => KEPT_SECS as f32 + OWN_SOUND_DEAF,
            _ => OWN_SOUND_DEAF,
        }
    }

    /// Something was found: a pressure, and then the memory of a pressure. The last sound it
    /// made comes back, lower and faint; it is not itself remembered.
    fn remember(&self) -> f32 {
        let tones = {
            let (at, last) = &*self.last.borrow();
            memory(last, self.ctx.current_time() - at)
        };
        for t in &tones {
            let _ = self.tone(*t);
        }
        memory_deaf(&tones)
    }

    /// A letter held down, sung back: `pitch` 0..1 picks a note from the scale, an octave under
    /// the pokes; `None` lets it fade.
    pub fn hum(&self, pitch: Option<f32>) {
        let Some(p) = pitch else {
            self.hum.gain.toward(&self.ctx, 0.0, 0.3);
            return;
        };
        self.hum.freq.toward(&self.ctx, 130.8 * 2f32.powf(step(p) / 12.0), 0.05);
        self.hum.gain.toward(&self.ctx, 0.06, 0.25);
    }

    /// Continuous sounds that follow the world's state: the warm hum and the night's drone.
    pub fn follow(&self, world: &World) {
        let c = world.body.color;
        let hum = if world.warming { 0.05 } else { 0.0 };
        self.warm.gain.toward(&self.ctx, hum, 0.25);
        self.warm.freq.toward(&self.ctx, 150.0 + 120.0 * (c.x - c.z + 0.5), 0.4);
        self.drone.toward(&self.ctx, 0.07 * world.night, 0.5);
    }

    /// A pull held: its one quiet tone follows it, rising, wavering, roughening (`game::strain`).
    /// Let go (`None`), it fades where it is, its pitch left as it was.
    pub fn strain(&self, s: Option<Strain>) {
        let (ctx, v) = (&self.ctx, &self.strain);
        let Some(s) = s else {
            v.voice.gain.toward(ctx, 0.0, STRAIN_RELEASE);
            return;
        };
        v.voice.freq.toward(ctx, s.freq, 0.06);
        v.voice.gain.toward(ctx, s.gain, 0.12);
        v.second.freq.toward(ctx, s.freq + s.beat, 0.06);
        v.second.gain.toward(ctx, s.rough, 0.15);
        v.rate.toward(ctx, s.tremolo, 0.1);
        v.depth.toward(ctx, s.depth, 0.15);
        v.still.toward(ctx, 1.0 - s.depth, 0.15);
    }

    /// Sings a note the player held back to them, `(pitch, 0..1)`, or falls silent.
    pub fn echo(&self, hum: Option<(f32, f32)>) {
        let (f, k) = hum.unwrap_or((0.0, 0.0));
        if f > 0.0 {
            self.echo.freq.toward(&self.ctx, f, 0.03);
        }
        self.echo.gain.toward(&self.ctx, 0.06 * k, 0.08);
    }
}
