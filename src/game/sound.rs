//! Things worth hearing, as the game asks for them. The browser layer turns each into a tone.

/// Things worth hearing. The browser layer turns these into tones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    /// `pitch` is 0..1, higher where it was touched higher up.
    Poke {
        pitch: f32,
        size: f32,
    },
    Tear,
    Merge,
    /// Something was found: the last sound it made comes back, an octave under and faint
    /// (`memory`).
    Discover,
    Provoked,
    Call,
    Pinch,
    /// A pinch let go of a little too late: the bud's note, falling, fainter.
    Almost,
    Bloom,
    /// The thread rings; `harmonic` is which overtone of its one string.
    Pluck {
        harmonic: u32,
    },
    Stir,
    /// A knock on the box, and the knock that answers it.
    Knock,
    Lid,
    /// It became something it wasn't.
    Become,
    /// One of the twelve, struck.
    Shard {
        i: usize,
    },
    /// All twelve at once.
    Chord,
    /// A column standing up out of the river; each one its own pitch.
    Column {
        slot: usize,
    },
    /// The river's heartbeat.
    Beat,
    /// The other one, struck; `pitch` 0..1.
    Ring {
        pitch: f32,
    },
}

/// A tone's wave: sine for the soft things, triangle for the hard ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Sine,
    Triangle,
}

/// One plucked tone as the browser layer plays it: a slide from `f0` to `f1` (Hz) over half of
/// `decay`, peaking at `gain`, beginning `delay` seconds after the sound does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    pub wave: Wave,
    pub f0: f32,
    pub f1: f32,
    pub gain: f32,
    pub decay: f64,
    pub delay: f64,
}

/// How long after a discovery the last sound comes back (seconds).
pub const MEMORY_AFTER: f64 = 0.6;
/// A sound older than this (seconds since it began) is not remembered; a low tone comes instead.
pub const MEMORY_WITHIN: f64 = 2.0;
/// The longest a remembered sound lasts (seconds), so a long ringing one never smears.
pub const MEMORY_LONGEST: f64 = 1.2;
/// The loudest tone in what comes back.
pub const MEMORY_GAIN: f32 = 0.02;
/// Under this (Hz) an octave down could not be heard, so the tone keeps its own pitch.
const MEMORY_FLOOR: f32 = 40.0;
/// For each of its own sounds the microphone is deaf this long from its start (seconds): most
/// of each one is gone in under a second.
pub const OWN_SOUND_DEAF: f32 = 0.6;

/// What sounds when something is found: `last` is the last sound it made, as tones, begun `ago`
/// seconds before. It comes back `MEMORY_AFTER` later, an octave lower and faint, cut to
/// `MEMORY_LONGEST`. With nothing heard recently, one soft low sine instead.
pub fn memory(last: &[Tone], ago: f64) -> Vec<Tone> {
    let loudest = last.iter().map(|t| t.gain).fold(0.0, f32::max);
    if last.is_empty() || ago > MEMORY_WITHIN || loudest <= 0.0 {
        return vec![Tone {
            wave: Wave::Sine,
            f0: 130.8,
            f1: 130.8,
            gain: MEMORY_GAIN,
            decay: 1.0,
            delay: MEMORY_AFTER,
        }];
    }
    let down = |f: f32| if f * 0.5 < MEMORY_FLOOR { f } else { f * 0.5 };
    last.iter()
        .filter(|t| t.delay < MEMORY_LONGEST - 0.1)
        .map(|t| Tone {
            wave: t.wave,
            f0: down(t.f0),
            f1: down(t.f1),
            gain: (t.gain / loudest * MEMORY_GAIN).max(0.0002),
            decay: t.decay.min(MEMORY_LONGEST - t.delay),
            delay: MEMORY_AFTER + t.delay,
        })
        .collect()
}

/// How long the microphone stays deaf once a memory is scheduled (seconds from now): until the
/// last of its tones has begun, and then as long as after any other sound.
pub fn memory_deaf(tones: &[Tone]) -> f32 {
    tones.iter().map(|t| t.delay as f32).fold(0.0, f32::max) + OWN_SOUND_DEAF
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(f: f32, gain: f32, decay: f64, delay: f64) -> Tone {
        Tone { wave: Wave::Sine, f0: f, f1: f, gain, decay, delay }
    }

    #[test]
    fn the_last_sound_comes_back_lower_and_fainter() {
        let poke = [tone(440.0, 0.22, 0.9, 0.0), tone(1214.4, 0.035, 0.35, 0.0)];
        let m = memory(&poke, 0.3);
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].f0, m[1].f0), (220.0, 607.2));
        assert!((m[0].gain - MEMORY_GAIN).abs() < 1e-6 && m[1].gain < m[0].gain);
        assert!(m.iter().all(|t| t.delay == MEMORY_AFTER));
        assert!((memory_deaf(&m) - 1.2).abs() < 1e-6, "{}", memory_deaf(&m));
    }

    #[test]
    fn a_long_sound_is_remembered_short() {
        let lid = [
            tone(196.0, 0.07, 3.0, 0.0),
            tone(293.7, 0.06, 3.0, 0.25),
            tone(392.0, 0.05, 3.2, 0.5),
            tone(587.3, 0.04, 3.4, 0.75),
        ];
        let m = memory(&lid, 0.1);
        assert!(!m.is_empty());
        for t in &m {
            assert!(t.delay - MEMORY_AFTER + t.decay <= MEMORY_LONGEST + 1e-9, "{t:?}");
            assert!(t.gain <= MEMORY_GAIN);
        }
        assert!(memory_deaf(&m) < 2.0);
    }

    #[test]
    fn with_nothing_heard_one_low_tone() {
        for m in [memory(&[], 0.0), memory(&[tone(659.3, 0.05, 2.4, 0.0)], MEMORY_WITHIN + 0.5)] {
            assert_eq!(m.len(), 1, "no figure, one tone");
            let t = m[0];
            assert!(t.wave == Wave::Sine && t.f0 == t.f1 && t.f0 < 200.0 && t.gain <= MEMORY_GAIN);
        }
    }

    #[test]
    fn the_lowest_stays_where_it_can_be_heard() {
        let beat = [tone(62.0, 0.32, 0.35, 0.0)];
        assert_eq!(memory(&beat, 0.1)[0].f0, 62.0);
    }
}
