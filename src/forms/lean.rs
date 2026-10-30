//! A form begun and not finished: after contact, on its own turn (mind/contact.rs), it may begin
//! something the player once made it become, and stop partway: a single bud, the first of its
//! corners, a slow turning inside. It holds there and waits. The form's own verb, done to it,
//! takes the form on from exactly as far as it got; left alone, it sinks back into the liquid.
//!
//! A lean is only ever a value from 0 up to how far it means to go, rising, held, or sinking,
//! each smoothstepped from where it is, so the form it belongs to never jumps either way.

use crate::math::smoothstep;

/// How long it takes to rise as far as it goes, how long it waits there, and how long it sinks.
pub const RISE_SECS: f32 = 3.0;
pub const WAIT_SECS: f32 = 9.0;
pub const SINK_SECS: f32 = 2.4;
/// The whole of one left alone, in seconds.
pub const LEAN_SECS: f32 = RISE_SECS + WAIT_SECS + SINK_SECS;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Rising { t: f32 },
    Held { t: f32 },
    Sinking { from: f32, t: f32 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lean {
    phase: Option<Phase>,
    /// How far it goes, 0..1 of the form.
    to: f32,
    /// The verb took it on from where it was.
    taken: bool,
}

impl Lean {
    /// Begin rising toward `to`. Returns false if it is already leaning.
    pub fn begin(&mut self, to: f32) -> bool {
        if self.phase.is_some() {
            return false;
        }
        *self = Lean { phase: Some(Phase::Rising { t: 0.0 }), to: to.clamp(0.0, 1.0), taken: false };
        true
    }

    pub fn leaning(&self) -> bool {
        self.phase.is_some()
    }

    /// Whether the last one was taken on by the verb, rather than left to sink.
    pub fn taken(&self) -> bool {
        self.taken
    }

    /// How far into the form it is right now.
    pub fn value(&self) -> f32 {
        match self.phase {
            None => 0.0,
            Some(Phase::Rising { t }) => self.to * smoothstep(0.0, RISE_SECS, t),
            Some(Phase::Held { .. }) => self.to,
            Some(Phase::Sinking { from, t }) => from * (1.0 - smoothstep(0.0, SINK_SECS, t)),
        }
    }

    /// Let it go back from wherever it is now.
    pub fn sink(&mut self) {
        if let Some(Phase::Rising { .. } | Phase::Held { .. }) = self.phase {
            self.phase = Some(Phase::Sinking { from: self.value(), t: 0.0 });
        }
    }

    /// The verb takes it on: how far it had got, for the form to go on from. It is over.
    pub fn take(&mut self) -> f32 {
        let v = self.value();
        if self.phase.take().is_some() {
            self.taken = true;
        }
        v
    }

    pub fn step(&mut self, dt: f32) {
        self.phase = match self.phase {
            Some(Phase::Rising { t }) if t + dt >= RISE_SECS => Some(Phase::Held { t: 0.0 }),
            Some(Phase::Rising { t }) => Some(Phase::Rising { t: t + dt }),
            Some(Phase::Held { t }) if t + dt >= WAIT_SECS => Some(Phase::Sinking { from: self.to, t: 0.0 }),
            Some(Phase::Held { t }) => Some(Phase::Held { t: t + dt }),
            Some(Phase::Sinking { t, .. }) if t + dt >= SINK_SECS => None,
            Some(Phase::Sinking { from, t }) => Some(Phase::Sinking { from, t: t + dt }),
            None => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_rises_holds_and_sinks_without_a_jump() {
        let mut l = Lean::default();
        assert!(l.begin(0.3));
        assert!(!l.begin(0.3), "one at a time");
        let (mut last, mut top) = (0.0f32, 0.0f32);
        for _ in 0..((LEAN_SECS + 1.0) * 60.0) as usize {
            l.step(1.0 / 60.0);
            let v = l.value();
            assert!((v - last).abs() < 0.01, "{last} -> {v}");
            top = top.max(v);
            last = v;
        }
        assert!((top - 0.3).abs() < 1e-4);
        assert!(!l.leaning() && l.value() == 0.0 && !l.taken());
    }

    #[test]
    fn sunk_or_taken_from_anywhere_it_goes_on_from_there() {
        for at in [0.5, 2.0, 6.0, 13.0] {
            let mut l = Lean::default();
            l.begin(0.3);
            l.step(at);
            let was = l.value();
            l.sink();
            assert!((l.value() - was).abs() < 1e-5, "sinking at {at}");
            let mut m = Lean::default();
            m.begin(0.3);
            m.step(at);
            assert!((m.take() - was).abs() < 1e-5);
            assert!(m.taken() && !m.leaning());
        }
    }
}
