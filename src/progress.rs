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
}

pub const ALL: [Discovery; 15] = [
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
            Discovery::Warm => "it was waiting to be held.",
            Discovery::Mix => "nothing comes back the same.",
            Discovery::Provoke => "it will remember that.",
            Discovery::Patience => "it noticed you.",
            Discovery::Open => "it was never empty.",
            Discovery::Close => "and it closed again.",
            Discovery::Return => "it remembers you.",
            Discovery::Call => "it heard you.",
            Discovery::Divide => "it was always two.",
            Discovery::Twin => "what one feels, the other feels.",
            Discovery::Rejoin => "it chose to be one.",
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
}
