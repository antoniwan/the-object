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
        Ok(Audio { ctx, master, warm, drone: (a, b, ga), muted: false })
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
            Sound::Call => self.pluck(Sine, 146.8, 196.0, 0.08, 0.7, 0.0),
            Sound::Provoked => self
                .pluck(Triangle, 73.4, 69.0, 0.25, 1.4, 0.0)
                .and_then(|_| self.pluck(Triangle, 77.8, 72.0, 0.2, 1.3, 0.0)),
        };
    }

    /// Continuous sounds that follow the world's state.
    pub fn follow(&self, world: &World) {
        let t = self.ctx.current_time();
        let c = world.body.color;
        let hum = if world.warming { 0.05 } else { 0.0 };
        let _ = self.warm.1.gain().set_target_at_time(hum, t, 0.25);
        let _ = self.warm.0.frequency().set_target_at_time(150.0 + 120.0 * (c.x - c.z + 0.5), t, 0.4);
        let _ = self.drone.2.gain().set_target_at_time(0.07 * world.night, t, 0.5);
    }
}
