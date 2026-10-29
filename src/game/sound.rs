//! Things worth hearing, as the game asks for them. The browser layer turns each into a tone.

/// Things worth hearing. The browser layer turns these into tones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    /// `pitch` is 0..1, higher where it was touched higher up.
    Poke { pitch: f32, size: f32 },
    Tear,
    Merge,
    Discover,
    Provoked,
    Call,
    Pinch,
    Bloom,
    /// The thread rings; `harmonic` is which overtone of its one string.
    Pluck { harmonic: u32 },
    Stir,
    /// A knock on the box, and the knock that answers it.
    Knock,
    Lid,
    /// It became something it wasn't.
    Become,
    /// One of the twelve, struck.
    Shard { i: usize },
    /// All twelve at once.
    Chord,
    /// A column standing up out of the river; each one its own pitch.
    Column { slot: usize },
    /// The river's heartbeat.
    Beat,
    /// The other one, struck; `pitch` 0..1.
    Ring { pitch: f32 },
}
