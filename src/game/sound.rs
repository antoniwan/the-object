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
    /// It became something it wasn't: the same low sound for every form, each at its own step
    /// of the scale (`become_tones`).
    Become {
        form: Form,
    },
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
    /// The note it knows the player by, `f` Hz, faint and once: back after a while, to a hand it
    /// is glad of (mind/absence.rs).
    Kept {
        f: f32,
    },
    /// One of its signs, said: `i` which of the twelve, `delay` seconds from now, `first` of a
    /// phrase (`signs`).
    Sign {
        i: usize,
        delay: f32,
        first: bool,
    },
}

/// Major pentatonic steps, in semitones, across two octaves.
pub const SCALE: [f32; 10] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0, 19.0, 21.0];

/// What it became, for `Sound::Become`: each is the same sound at its own step of `SCALE`, so the
/// forms are told apart by ear. `Itself` is the sound unmoved: back from a form, reaching out,
/// another arriving or taken in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Itself,
    River,
    Box,
    Thread,
    Mound,
    Cosmos,
    Dust,
    Ropes,
    Whirl,
    Fold,
}

impl Form {
    pub const ALL: [Form; 10] = [
        Form::Itself,
        Form::River,
        Form::Box,
        Form::Thread,
        Form::Mound,
        Form::Cosmos,
        Form::Dust,
        Form::Ropes,
        Form::Whirl,
        Form::Fold,
    ];

    /// Its step of `SCALE`, in semitones: one fixed step for each.
    pub fn step(self) -> f32 {
        SCALE[Form::ALL.iter().position(|f| *f == self).unwrap_or(0)]
    }
}

/// The sound of becoming: two low sines, 65.4 Hz and the fifth over it, raised by the form's step.
pub fn become_tones(form: Form) -> [Tone; 2] {
    let up = 2f32.powf(form.step() / 12.0);
    let (a, b) = (65.4 * up, 98.0 * up);
    [
        Tone { wave: Wave::Sine, f0: a, f1: a, gain: 0.14, decay: 3.0, delay: 0.0 },
        Tone { wave: Wave::Sine, f0: b, f1: b, gain: 0.08, decay: 3.0, delay: 0.1 },
    ]
}

/// A pull as it is heard: `stretch` how far it is drawn out (body radii past its rest), `tear`
/// how far it can go before it tears, `taut` how near held taut has come to a thread, 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pull {
    pub stretch: f32,
    pub tear: f32,
    pub taut: f32,
}

/// The one quiet tone of a pull, as the browser layer plays it: a sine at `freq` and `gain`,
/// its loudness wavering `tremolo` times a second by `depth` (0..0.5 of it), and, past
/// `ROUGH_FROM`, a second sine `beat` Hz above it at `rough` (0..1) of its level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strain {
    pub freq: f32,
    pub gain: f32,
    pub tremolo: f32,
    pub depth: f32,
    pub rough: f32,
    pub beat: f32,
}

/// The pull's tone at its loudest, under the warm hum's 0.05.
pub const STRAIN_GAIN: f32 = 0.04;
/// Where it rests with no note of the player's known (Hz): A3.
pub const STRAIN_REST: f32 = 220.0;
/// A known note is folded into the octave from here (Hz), so it rests where it can be heard softly.
pub const STRAIN_LOW: f32 = 165.0;
/// Drawn out to its tear, it has risen this many semitones.
pub const STRAIN_RISE: f32 = 12.0;
/// Its wavering once held taut, a second: slow as it begins, quick as the thread nears.
pub const TREMOLO_FROM: f32 = 3.0;
pub const TREMOLO_TO: f32 = 12.0;
/// How deep the wavering goes, as a share of its loudness.
pub const TREMOLO_DEPTH: f32 = 0.45;
/// Past this stretch (body radii) it roughens: a second sine beating against it.
pub const ROUGH_FROM: f32 = 1.2;
/// Let go, it fades this fast (time constant, seconds); the pitch stays where it was.
pub const STRAIN_RELEASE: f64 = 0.25;

/// Where the pull's tone rests: the note it knows the player by, folded into one octave from
/// `STRAIN_LOW`, or `STRAIN_REST`.
pub fn strain_rest(note: Option<f32>) -> f32 {
    let Some(mut f) = note.filter(|f| f.is_finite() && *f > 0.0) else { return STRAIN_REST };
    while f >= STRAIN_LOW * 2.0 {
        f *= 0.5;
    }
    while f < STRAIN_LOW {
        f *= 2.0;
    }
    f
}

/// What a pull sounds like: rising and louder as it is drawn out, wavering faster the longer it
/// is held taut, rough just before it tears.
pub fn strain(p: Pull, note: Option<f32>) -> Strain {
    let tear = p.tear.max(0.1);
    let x = (p.stretch / tear).clamp(0.0, 1.0);
    let taut = p.taut.clamp(0.0, 1.0);
    let from = ROUGH_FROM.min(tear * 0.9);
    let r = ((p.stretch - from) / (tear - from)).clamp(0.0, 1.0);
    let rough = r * r * (3.0 - 2.0 * r);
    Strain {
        freq: strain_rest(note) * 2f32.powf(STRAIN_RISE * x / 12.0),
        // the two sines together are never louder than one at its loudest
        gain: STRAIN_GAIN * x / (1.0 + 0.7 * rough),
        tremolo: TREMOLO_FROM + (TREMOLO_TO - TREMOLO_FROM) * taut,
        depth: if taut > 0.0 { TREMOLO_DEPTH } else { 0.0 },
        rough: 0.7 * rough,
        beat: 4.0 + 5.0 * rough,
    }
}

/// How far the held tip trembles once taut, in body radii at most.
pub const TREMBLE: f32 = 0.012;

impl super::Game {
    /// The pull being held now, if any, as it is heard.
    pub(super) fn pull(&self) -> Option<(u32, Pull)> {
        let super::Gesture::Stretching { handle, taut, .. } = self.gesture else { return None };
        let h = self.world.handles.iter().find(|h| h.id == handle && h.target.is_some())?;
        let pull = Pull {
            stretch: h.stretch(&self.world.body),
            tear: crate::sim::TEAR_RATIO * h.give,
            taut: taut / crate::forms::filament::TAUT_SECS,
        };
        Some((handle, pull))
    }

    /// The tone of the pull being held now, tuned to the note it knows the player by; `None`
    /// when nothing is pulled, and the tone fades.
    pub fn strain(&self) -> Option<Strain> {
        self.pull().map(|(_, p)| strain(p, self.remembrance.pitch()))
    }

    /// The held tip trembling once taut, faster and wider the nearer the thread: which tendril,
    /// and how far its tip is shown off where it is. Only shown, never moved; still under
    /// reduced motion.
    pub fn tremble(&self) -> Option<(u32, crate::math::V3)> {
        let (id, p) = self.pull()?;
        if self.cam.still || p.taut <= 0.0 {
            return None;
        }
        let t = self.world.time;
        let a = TREMBLE * self.world.body.radius() * p.taut.min(1.0);
        Some((id, crate::math::v3((t * 53.0).sin(), (t * 61.0 + 1.0).sin(), (t * 47.0 + 2.0).sin()) * a))
    }
}

/// Between one sign of a phrase and the next (seconds), and the more between two words.
pub const SIGN_STEP: f32 = 0.12;
pub const SIGN_SPACE: f32 = 0.14;

/// What its signs sound like, said `lead` seconds from now: one tone for each sign, in the order
/// they are shown, so the same signs always make the same short phrase.
pub fn signs(text: &str, lead: f32) -> Vec<Sound> {
    let signs = crate::mind::contact::SIGNS;
    let mut at = lead;
    let mut out = Vec::new();
    for c in text.chars() {
        match signs.iter().position(|s| *s == c) {
            Some(i) => {
                out.push(Sound::Sign { i, delay: at, first: out.is_empty() });
                at += SIGN_STEP;
            }
            None => at += SIGN_SPACE,
        }
    }
    out
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
    fn the_same_signs_are_always_the_same_phrase() {
        let a = signs("∴⊕ ∴", 0.35);
        assert_eq!(a, signs("∴⊕ ∴", 0.35));
        assert_eq!(a.len(), 3);
        let at: Vec<f32> = a.iter().map(|s| if let Sound::Sign { delay, .. } = s { *delay } else { -1.0 }).collect();
        assert!(at[0] == 0.35 && at[1] > at[0] && at[2] - at[1] > at[1] - at[0], "a word apart: {at:?}");
        assert!(
            matches!(a[0], Sound::Sign { i: 0, first: true, .. })
                && matches!(a[1], Sound::Sign { i: 3, first: false, .. })
        );
        assert_ne!(a, signs("⊕∴ ⊕", 0.35));
    }

    fn pull(stretch: f32, taut: f32) -> Pull {
        Pull { stretch, tear: crate::sim::TEAR_RATIO, taut }
    }

    #[test]
    fn a_pull_rises_as_it_is_drawn_out() {
        let mut last = strain(pull(0.0, 0.0), None);
        assert_eq!((last.freq, last.gain), (STRAIN_REST, 0.0), "at rest, silent, where it rests");
        let loud = |s: Strain| s.gain * (1.0 + s.rough);
        for k in 1..=38 {
            let s = strain(pull(k as f32 * 0.035, 0.0), None);
            assert!(s.freq > last.freq && loud(s) > loud(last), "{k}: {s:?} after {last:?}");
            assert!(loud(s) <= STRAIN_GAIN + 1e-6, "under the warm hum: {s:?}");
            last = s;
        }
        let torn = strain(pull(crate::sim::TEAR_RATIO, 0.0), None);
        assert!((torn.freq / STRAIN_REST - 2.0).abs() < 1e-3, "an octave up at the tear");
        // it roughens only near the tear
        assert_eq!(strain(pull(1.1, 0.0), None).rough, 0.0);
        assert!(strain(pull(1.3, 0.0), None).rough > 0.0);
        // so wary a hand that it tears before 1.2, it still roughens first
        let wary = strain(Pull { stretch: 1.15, tear: 1.2, taut: 0.0 }, None);
        assert!(wary.rough > 0.0 && wary.rough <= 0.7);
    }

    #[test]
    fn held_taut_it_wavers_faster_toward_the_thread() {
        assert_eq!(strain(pull(1.0, 0.0), None).depth, 0.0, "not taut, steady");
        let mut last = 0.0;
        for k in 1..=10 {
            let s = strain(pull(1.0, k as f32 / 10.0), None);
            assert!(s.depth > 0.0 && s.depth <= 0.5 && s.tremolo > last, "{k}: {s:?}");
            last = s.tremolo;
        }
        assert_eq!(last, TREMOLO_TO);
    }

    #[test]
    fn let_go_it_fades_without_a_jump() {
        // what the browser draws for `toward(0, STRAIN_RELEASE)`: a curve, from where it was
        let g0 = strain(pull(1.3, 0.8), None).gain;
        let at = |t: f64| g0 * (-t / STRAIN_RELEASE).exp() as f32;
        let frame = 1.0 / 60.0;
        assert!(at(frame) > g0 * 0.9, "the first frame keeps most of it");
        assert!((1..120).all(|k| at(f64::from(k) * frame) - at(f64::from(k + 1) * frame) < g0 * 0.1));
        assert!(at(1.5) < g0 * 0.01, "and it is gone in a moment");
    }

    #[test]
    fn a_pull_rests_on_the_players_note() {
        assert_eq!(strain_rest(None), STRAIN_REST);
        for (sung, rest) in [(196.0, 196.0), (98.0, 196.0), (440.0, 220.0), (880.0, 220.0), (130.0, 260.0)] {
            assert!((strain_rest(Some(sung)) - rest).abs() < 1e-3, "{sung} rests at {rest}");
            assert!((strain(pull(0.0, 0.0), Some(sung)).freq - rest).abs() < 1e-3);
        }
        for f in [55.0, 100.0, 300.0, 1100.0] {
            let r = strain_rest(Some(f));
            assert!((STRAIN_LOW..STRAIN_LOW * 2.0).contains(&r), "{f} folds to {r}");
            assert!(((r / f).log2().round() - (r / f).log2()).abs() < 1e-4, "an octave of it");
        }
    }

    #[test]
    fn each_form_becomes_at_its_own_step() {
        let root = become_tones(Form::Itself);
        assert_eq!((root[0].f0, root[1].f0), (65.4, 98.0), "unmoved, as it was");
        let mut seen: Vec<f32> = Vec::new();
        for f in Form::ALL {
            let t = become_tones(f);
            assert!(!seen.iter().any(|s| (s - t[0].f0).abs() < 0.5), "{f:?} shares a pitch");
            seen.push(t[0].f0);
            // the same sound: a fifth, the same loudness and length, only moved
            assert!((t[1].f0 / t[0].f0 - 98.0 / 65.4).abs() < 1e-4);
            assert!(t.iter().zip(root).all(|(a, b)| a.gain == b.gain && a.decay == b.decay && a.wave == Wave::Sine));
            let semis = 12.0 * (t[0].f0 / 65.4).log2();
            assert!(SCALE.iter().any(|s| (s - semis).abs() < 1e-3), "{f:?} is on the scale");
        }
    }

    #[test]
    fn the_lowest_stays_where_it_can_be_heard() {
        let beat = [tone(62.0, 0.32, 0.35, 0.0)];
        assert_eq!(memory(&beat, 0.1)[0].f0, 62.0);
    }
}
