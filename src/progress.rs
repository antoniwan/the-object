//! What the player has found out they can do. This is the seed of the unlock
//! system: new verbs and new objects can be gated on what has been discovered.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discovery {
    Poke,
    Stretch,
    Tear,
    Merge,
    Squeeze,
    Fling,
    LookAround,
    Warm,
    Mix,
    Provoke,
    Patience,
    Open,
    Close,
    Return,
    Call,
    Divide,
    Twin,
    Rejoin,
    Throw,
    Slam,
    Lake,
    Rise,
    Burst,
    Shards,
    Gather,
    Cleave,
    Grain,
    Eye,
    Shed,
    Stone,
    Slab,
    Thaw,
    Mirror,
    Freeze,
    Shatter,
    Deep,
    Fountain,
    Quicksilver,
    Ink,
    Chord,
    Carry,
    Plant,
    Grow,
    Weather,
    Mend,
    Pinch,
    Bloom,
    Fold,
    Thread,
    Pluck,
    Wind,
    Stir,
    Cosmos,
    Settle,
    Corners,
    Knock,
    Lid,
    Round,
}

pub const ALL: [Discovery; 58] = [
    Discovery::Poke,
    Discovery::Stretch,
    Discovery::Tear,
    Discovery::Merge,
    Discovery::Squeeze,
    Discovery::Fling,
    Discovery::LookAround,
    Discovery::Warm,
    Discovery::Mix,
    Discovery::Provoke,
    Discovery::Patience,
    Discovery::Open,
    Discovery::Close,
    Discovery::Return,
    Discovery::Call,
    Discovery::Divide,
    Discovery::Twin,
    Discovery::Rejoin,
    Discovery::Throw,
    Discovery::Slam,
    Discovery::Lake,
    Discovery::Rise,
    Discovery::Burst,
    Discovery::Shards,
    Discovery::Gather,
    Discovery::Cleave,
    Discovery::Grain,
    Discovery::Eye,
    Discovery::Shed,
    Discovery::Stone,
    Discovery::Slab,
    Discovery::Thaw,
    Discovery::Mirror,
    Discovery::Freeze,
    Discovery::Shatter,
    Discovery::Deep,
    Discovery::Fountain,
    Discovery::Quicksilver,
    Discovery::Ink,
    Discovery::Chord,
    Discovery::Carry,
    Discovery::Plant,
    Discovery::Grow,
    Discovery::Weather,
    Discovery::Mend,
    Discovery::Pinch,
    Discovery::Bloom,
    Discovery::Fold,
    Discovery::Thread,
    Discovery::Pluck,
    Discovery::Wind,
    Discovery::Stir,
    Discovery::Cosmos,
    Discovery::Settle,
    Discovery::Corners,
    Discovery::Knock,
    Discovery::Lid,
    Discovery::Round,
];

impl Discovery {
    pub fn name(self) -> &'static str {
        match self {
            Discovery::Poke => "poke",
            Discovery::Stretch => "stretch",
            Discovery::Tear => "tear",
            Discovery::Merge => "merge",
            Discovery::Squeeze => "squeeze",
            Discovery::Fling => "fling",
            Discovery::LookAround => "look around",
            Discovery::Warm => "warm",
            Discovery::Mix => "mix",
            Discovery::Provoke => "provoke",
            Discovery::Patience => "patience",
            Discovery::Open => "open",
            Discovery::Close => "close",
            Discovery::Return => "return",
            Discovery::Call => "call",
            Discovery::Divide => "divide",
            Discovery::Twin => "twin",
            Discovery::Rejoin => "rejoin",
            Discovery::Throw => "throw",
            Discovery::Slam => "slam",
            Discovery::Lake => "lake",
            Discovery::Rise => "rise",
            Discovery::Burst => "burst",
            Discovery::Shards => "twelve",
            Discovery::Gather => "gather",
            Discovery::Cleave => "cleave",
            Discovery::Grain => "grain",
            Discovery::Eye => "eye",
            Discovery::Shed => "shed",
            Discovery::Stone => "stone",
            Discovery::Slab => "1 : 4 : 9",
            Discovery::Thaw => "thaw",
            Discovery::Mirror => "mirror",
            Discovery::Freeze => "freeze",
            Discovery::Shatter => "shatter",
            Discovery::Deep => "deep",
            Discovery::Fountain => "fountain",
            Discovery::Quicksilver => "quicksilver",
            Discovery::Ink => "ink",
            Discovery::Chord => "chord",
            Discovery::Carry => "carry",
            Discovery::Plant => "plant",
            Discovery::Grow => "grow",
            Discovery::Weather => "weather",
            Discovery::Mend => "mend",
            Discovery::Pinch => "pinch",
            Discovery::Bloom => "bloom",
            Discovery::Fold => "fold",
            Discovery::Thread => "thread",
            Discovery::Pluck => "pluck",
            Discovery::Wind => "wind",
            Discovery::Stir => "stir",
            Discovery::Cosmos => "cosmos",
            Discovery::Settle => "settle",
            Discovery::Corners => "corners",
            Discovery::Knock => "knock",
            Discovery::Lid => "lid",
            Discovery::Round => "round",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            Discovery::Poke => "a pressure, and then the memory of a pressure.",
            Discovery::Stretch => "it gives, and keeps what it gave.",
            Discovery::Tear => "something left it. it is still counted.",
            Discovery::Merge => "the seam forgets it was ever a seam.",
            Discovery::Squeeze => "less room. the same amount of it.",
            Discovery::Fling => "distance is a slower way of coming back.",
            Discovery::LookAround => "there is nothing else here. not yet.",
            Discovery::Warm => "your stillness has a temperature.",
            Discovery::Mix => "no color returns the way it left.",
            Discovery::Provoke => "it keeps the shape of every blow.",
            Discovery::Patience => "you waited. now it waits for you.",
            Discovery::Open => "the inside was a sky all along.",
            Discovery::Close => "it folds the night back into itself.",
            Discovery::Return => "it kept your place.",
            Discovery::Call => "a sound went in. something answered.",
            Discovery::Divide => "it was two before it was one.",
            Discovery::Twin => "one wound. two bodies.",
            Discovery::Rejoin => "it chose the harder number.",
            Discovery::Throw => "it accepts the direction, not the reason.",
            Discovery::Slam => "the ground was the softer thing.",
            Discovery::Lake => "now the floor remembers its shape.",
            Discovery::Rise => "what falls here does not stay fallen.",
            Discovery::Burst => "too much inside. the outside won.",
            Discovery::Shards => "twelve pieces. none of them agree.",
            Discovery::Gather => "broken is not the same as gone.",
            Discovery::Cleave => "more of it. never more than it.",
            Discovery::Grain => "too many to be cells. a material now.",
            Discovery::Eye => "one of them stopped agreeing. it looks.",
            Discovery::Shed => "it gave one up so the rest could close.",
            Discovery::Stone => "every seam, closed from the inside.",
            Discovery::Slab => "one, four, nine. it has decided.",
            Discovery::Thaw => "even stone remembers being poured.",
            Discovery::Mirror => "still enough, and it shows you what it was.",
            Discovery::Freeze => "you held it until it held still.",
            Discovery::Shatter => "the cold was only holding its breath.",
            Discovery::Deep => "the floor goes further down than the room.",
            Discovery::Fountain => "three blows in one place. it gives one back.",
            Discovery::Quicksilver => "it came up wearing the light.",
            Discovery::Ink => "it came up wearing the dark.",
            Discovery::Chord => "for one breath, none of them disagreed.",
            Discovery::Carry => "it goes where it is put. it is not yours.",
            Discovery::Plant => "what is buried is not finished.",
            Discovery::Grow => "it came back larger than it left.",
            Discovery::Weather => "nothing here stays sharp. not even this.",
            Discovery::Mend => "you put it back. it is not the same.",
            Discovery::Pinch => "the smallest hurt. it grows something there.",
            Discovery::Bloom => "five small wounds agreed to open.",
            Discovery::Fold => "it closes the way it was first closed.",
            Discovery::Thread => "held gently enough, it never breaks. it only leaves the room.",
            Discovery::Pluck => "it was always a note. it only needed a length.",
            Discovery::Wind => "everything long is a sphere, waiting.",
            Discovery::Stir => "round and round. the inside starts to listen.",
            Discovery::Cosmos => "a drop, stirred long enough, holds every drop.",
            Discovery::Settle => "the other way round, and time lies down again.",
            Discovery::Corners => "you drew it a shape. it agreed.",
            Discovery::Knock => "something inside knocks back.",
            Discovery::Lid => "it was never empty. only closed.",
            Discovery::Round => "it forgets its corners last.",
        }
    }
}

#[derive(Default)]
pub struct Progress {
    found: Vec<Discovery>,
}

impl Progress {
    /// Returns true the first time something is discovered.
    pub fn discover(&mut self, d: Discovery) -> bool {
        if self.found.contains(&d) {
            return false;
        }
        self.found.push(d);
        true
    }

    pub fn has(&self, d: Discovery) -> bool {
        self.found.contains(&d)
    }

    pub fn count(&self) -> usize {
        self.found.len()
    }

    /// Comma-separated names, for saving between visits.
    pub fn save(&self) -> String {
        self.found.iter().map(|d| d.name()).collect::<Vec<_>>().join(",")
    }

    pub fn load(saved: &str) -> Progress {
        let found = saved
            .split(',')
            .filter_map(|n| ALL.iter().copied().find(|d| d.name() == n))
            .collect();
        Progress { found }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let mut p = Progress::default();
        p.discover(Discovery::Tear);
        p.discover(Discovery::LookAround);
        let q = Progress::load(&p.save());
        assert!(q.has(Discovery::Tear) && q.has(Discovery::LookAround));
        assert_eq!(q.count(), 2);
        assert_eq!(Progress::load("").count(), 0);
        assert_eq!(Progress::load("nonsense,poke").count(), 1);
    }

    #[test]
    fn all_lists_every_discovery() {
        // Discoveries missing from ALL are forgotten between visits.
        // Keep the last variant here when appending to the enum.
        assert_eq!(ALL.len(), Discovery::Round as usize + 1);
        for (i, d) in ALL.iter().enumerate() {
            assert_eq!(*d as usize, i);
        }
    }
}
