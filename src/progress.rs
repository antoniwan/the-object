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
