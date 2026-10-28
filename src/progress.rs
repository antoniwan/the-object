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
}

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
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            Discovery::Poke => "it felt that.",
            Discovery::Stretch => "it gives.",
            Discovery::Tear => "a piece of it is free.",
            Discovery::Merge => "it remembers being whole.",
            Discovery::Squeeze => "it holds its volume.",
            Discovery::Fling => "it always comes back.",
            Discovery::LookAround => "there is nothing else. yet.",
            Discovery::Divide => "it was always two.",
            Discovery::Twin => "what one feels, the other feels.",
            Discovery::Rejoin => "it chose to be one.",
            Discovery::Throw => "it goes where it is sent.",
            Discovery::Slam => "it hit the ground and stayed.",
            Discovery::Lake => "the ground remembers.",
            Discovery::Rise => "it was never gone.",
            Discovery::Burst => "it couldn't hold any more.",
            Discovery::Shards => "every piece is different.",
            Discovery::Gather => "it was in pieces, not gone.",
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
}
