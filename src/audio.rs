//! Every sound is synthesized: a few sine and triangle tones, no samples.

use crate::game::Sound;
use crate::sim::World;
use wasm_bindgen::JsValue;
use web_sys::{AudioContext, GainNode, OscillatorNode, OscillatorType};

/// Major pentatonic steps, in semitones, across two octaves.
const SCALE: [f32; 10] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0, 19.0, 21.0];

pub struct Audio {
    ctx: AudioContext,
    master: GainNode,
    /// The hum while it is held still, and the low drone of the night.
    warm: (OscillatorNode, GainNode),
    drone: (OscillatorNode, OscillatorNode, GainNode),
    /// A letter held down, sung back (keys.rs).
    hum: (OscillatorNode, GainNode),
    /// A note the player held, hummed back (senses.rs).
    echo: (OscillatorNode, GainNode),
    pub muted: bool,
}

impl Audio {
    /// Must be called from inside a user gesture, or the browser keeps it silent.
    pub fn new() -> Result<Audio, JsValue> {
        let ctx = AudioContext::new()?;
        let master = ctx.create_gain()?;
        master.gain().set_value(0.7);
        master.connect_with_audio_node(&ctx.destination())?;

        let warm = Self::voice(&ctx, &master, OscillatorType::Sine, 196.0)?;
        let (a, ga) = Self::voice(&ctx, &master, OscillatorType::Sine, 55.0)?;
        let b = ctx.create_oscillator()?;
        b.frequency().set_value(82.6);
        b.connect_with_audio_node(&ga)?;
        b.start()?;
        let hum = Self::voice(&ctx, &master, OscillatorType::Sine, 130.8)?;
        let echo = Self::voice(&ctx, &master, OscillatorType::Sine, 110.0)?;
        Ok(Audio { ctx, master, warm, drone: (a, b, ga), hum, echo, muted: false })
    }

    /// A continuously running oscillator behind a silent gain.
    fn voice(ctx: &AudioContext, out: &GainNode, kind: OscillatorType, freq: f32) -> Result<(OscillatorNode, GainNode), JsValue> {
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

    /// Touch screens only grant audio on pointerup, so this is retried on every release.
    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let t = self.ctx.current_time();
        let _ = self.master.gain().set_target_at_time(if self.muted { 0.0 } else { 0.7 }, t, 0.1);
    }

    /// One plucked tone: a quick attack, a slide from `f0` to `f1`, and an exponential fade.
    fn pluck(&self, kind: OscillatorType, f0: f32, f1: f32, gain: f32, decay: f64, delay: f64) -> Result<(), JsValue> {
        let t = self.ctx.current_time() + delay;
        let osc = self.ctx.create_oscillator()?;
        osc.set_type(kind);
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

    pub fn play(&self, s: Sound) {
        use OscillatorType::*;
        let _ = match s {
            Sound::Poke { pitch, size } => {
                let step = SCALE[((pitch * (SCALE.len() - 1) as f32).round() as usize).min(SCALE.len() - 1)];
                let f = 220.0 / size * 2f32.powf(step / 12.0);
                self.pluck(Sine, f, f * 0.96, 0.22, 0.9, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.76, f * 2.7, 0.035, 0.35, 0.0))
            }
            Sound::Tear => self.pluck(Sine, 320.0, 1150.0, 0.16, 0.45, 0.0),
            Sound::Merge => self
                .pluck(Sine, 110.0, 108.0, 0.16, 1.6, 0.0)
                .and_then(|_| self.pluck(Sine, 164.8, 163.0, 0.09, 1.4, 0.03)),
            Sound::Discover => self
                .pluck(Sine, 659.3, 659.3, 0.05, 2.4, 0.15)
                .and_then(|_| self.pluck(Sine, 987.8, 987.8, 0.035, 2.6, 0.32))
                .and_then(|_| self.pluck(Sine, 1318.5, 1318.5, 0.02, 2.8, 0.5)),
            Sound::Shard { i } => {
                // each piece its own note: the larger, the lower; the hard ones ring, the soft ones thud
                let f = 1400.0 / crate::burst::SIZES[i] * 2f32.powf(SCALE[i % SCALE.len()] / 24.0);
                let (give, decay) = crate::burst::CONSISTENCIES[i].give();
                let ring = (2.6 / decay) as f64;
                let kind = if decay > 6.0 { Triangle } else { Sine };
                self.pluck(kind, f, f * (1.0 - 0.04 * give), 0.16, ring, 0.0)
                    .and_then(|_| self.pluck(Sine, f * 2.01, f * 2.0, 0.03 / decay.max(1.0), ring * 0.6, 0.0))
            }
            Sound::Chord => (0..crate::burst::SHARDS).try_for_each(|i| {
                let f = 1400.0 / crate::burst::SIZES[i] * 2f32.powf(SCALE[i % SCALE.len()] / 24.0);
                self.pluck(Sine, f, f, 0.035, 3.2, i as f64 * 0.045)
            }),
            // up the scale with the river, a row raised in one drag climbs
            Sound::Column { slot } => {
                let row = slot / crate::river::LANES;
                let f = 196.0 * 2f32.powf(SCALE[row % SCALE.len()] / 12.0);
                self.pluck(Sine, f * 0.5, f, 0.06, 0.6, 0.0).and_then(|_| self.pluck(Triangle, f * 2.0, f * 2.0, 0.015, 0.9, 0.05))
            }
            // two low thumps, the second softer
            Sound::Beat => self
                .pluck(Sine, 62.0, 44.0, 0.32, 0.35, 0.0)
                .and_then(|_| self.pluck(Sine, 58.0, 42.0, 0.2, 0.3, 0.2)),
            Sound::Call => self.pluck(Sine, 146.8, 196.0, 0.08, 0.7, 0.0),
            Sound::Pinch => self.pluck(Sine, 1046.5, 1174.7, 0.05, 0.3, 0.0),
            Sound::Bloom => self
                .pluck(Sine, 523.3, 523.3, 0.05, 2.6, 0.0)
                .and_then(|_| self.pluck(Sine, 659.3, 659.3, 0.04, 2.6, 0.18))
                .and_then(|_| self.pluck(Sine, 784.0, 784.0, 0.035, 2.8, 0.36))
                .and_then(|_| self.pluck(Sine, 1046.5, 1046.5, 0.025, 3.0, 0.54)),
            // Every pluck is an overtone of the same low string.
            Sound::Pluck { harmonic } => {
                let f = 98.0 * harmonic as f32;
                self.pluck(Triangle, f, f, 0.16, 3.2, 0.0).and_then(|_| self.pluck(Sine, f * 2.0, f * 2.0, 0.04, 1.6, 0.0))
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
            Sound::Become => self
                .pluck(Sine, 65.4, 65.4, 0.14, 3.0, 0.0)
                .and_then(|_| self.pluck(Sine, 98.0, 98.0, 0.08, 3.0, 0.1)),
            Sound::Provoked => self
                .pluck(Triangle, 73.4, 69.0, 0.25, 1.4, 0.0)
                .and_then(|_| self.pluck(Triangle, 77.8, 72.0, 0.2, 1.3, 0.0)),
        };
    }

    /// Continuous sounds that follow the world's state.
    /// `pitch` 0..1 picks a note from the scale, an octave under the pokes; `None` lets it fade.
    pub fn hum(&self, pitch: Option<f32>) {
        let t = self.ctx.current_time();
        let Some(p) = pitch else {
            let _ = self.hum.1.gain().set_target_at_time(0.0, t, 0.3);
            return;
        };
        let step = SCALE[((p * (SCALE.len() - 1) as f32).round() as usize).min(SCALE.len() - 1)];
        let _ = self.hum.0.frequency().set_target_at_time(130.8 * 2f32.powf(step / 12.0), t, 0.05);
        let _ = self.hum.1.gain().set_target_at_time(0.06, t, 0.25);
    }

    pub fn follow(&self, world: &World) {
        let t = self.ctx.current_time();
        let c = world.body.color;
        let hum = if world.warming { 0.05 } else { 0.0 };
        let _ = self.warm.1.gain().set_target_at_time(hum, t, 0.25);
        let _ = self.warm.0.frequency().set_target_at_time(150.0 + 120.0 * (c.x - c.z + 0.5), t, 0.4);
        let _ = self.drone.2.gain().set_target_at_time(0.07 * world.night, t, 0.5);
    }

    /// Sings a note the player held back to them, `(pitch, 0..1)`, or falls silent.
    pub fn echo(&self, hum: Option<(f32, f32)>) {
        let t = self.ctx.current_time();
        let (f, k) = hum.unwrap_or((0.0, 0.0));
        if f > 0.0 {
            let _ = self.echo.0.frequency().set_target_at_time(f, t, 0.03);
        }
        let _ = self.echo.1.gain().set_target_at_time(0.06 * k, t, 0.08);
    }
}
