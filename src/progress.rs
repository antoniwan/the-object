//! What the player has found out they can do: every discovery, its name, and the one line
//! shown when it is found. Kept between visits. How many have been found is what contact.rs
//! reads to decide how much the object shows it has been paying attention.

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
    River,
    Column,
    Pulse,
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
    Regard,
    Flinch,
    Echo,
    Mimic,
    Initiate,
    Offer,
    Answer,
    Contact,
    Letters,
    Hum,
    Breath,
    Lean,
    Said,
    Voice,
    Sing,
    Whisper,
    Blow,
    Startle,
    Seen,
    Blind,
    Watched,
    Shout,
    Shaken,
    Scatter,
    Another,
    Ring,
    Near,
    Clash,
    Inside,
    Give,
    Between,
    Distance,
    Everywhere,
    Facet,
    Backward,
    Crease,
    Kin,
    Loose,
    Part,
    Pass,
    Many,
    Remembered,
    Wary,
    Word,
    Greeting,
    Unanswered,
    Dust,
    Sweep,
    Mound,
    Unearth,
    Scar,
    Wound,
    Tend,
    Heal,
    Root,
    Canopy,
    Wither,
    Sway,
    Ropes,
    Aloft,
    Braced,
    InStep,
    YourNote,
}

pub const ALL: [Discovery; 124] = [
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
    Discovery::River,
    Discovery::Column,
    Discovery::Pulse,
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
    Discovery::Regard,
    Discovery::Flinch,
    Discovery::Echo,
    Discovery::Mimic,
    Discovery::Initiate,
    Discovery::Offer,
    Discovery::Answer,
    Discovery::Contact,
    Discovery::Letters,
    Discovery::Hum,
    Discovery::Breath,
    Discovery::Lean,
    Discovery::Said,
    Discovery::Voice,
    Discovery::Sing,
    Discovery::Whisper,
    Discovery::Blow,
    Discovery::Startle,
    Discovery::Seen,
    Discovery::Blind,
    Discovery::Watched,
    Discovery::Shout,
    Discovery::Shaken,
    Discovery::Scatter,
    Discovery::Another,
    Discovery::Ring,
    Discovery::Near,
    Discovery::Clash,
    Discovery::Inside,
    Discovery::Give,
    Discovery::Between,
    Discovery::Distance,
    Discovery::Everywhere,
    Discovery::Facet,
    Discovery::Backward,
    Discovery::Crease,
    Discovery::Kin,
    Discovery::Loose,
    Discovery::Part,
    Discovery::Pass,
    Discovery::Many,
    Discovery::Remembered,
    Discovery::Wary,
    Discovery::Word,
    Discovery::Greeting,
    Discovery::Unanswered,
    Discovery::Dust,
    Discovery::Sweep,
    Discovery::Mound,
    Discovery::Unearth,
    Discovery::Scar,
    Discovery::Wound,
    Discovery::Tend,
    Discovery::Heal,
    Discovery::Root,
    Discovery::Canopy,
    Discovery::Wither,
    Discovery::Sway,
    Discovery::Ropes,
    Discovery::Aloft,
    Discovery::Braced,
    Discovery::InStep,
    Discovery::YourNote,
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
            Discovery::River => "river",
            Discovery::Column => "column",
            Discovery::Pulse => "pulse",
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
            Discovery::Regard => "regard",
            Discovery::Flinch => "flinch",
            Discovery::Echo => "echo",
            Discovery::Mimic => "mimic",
            Discovery::Initiate => "its turn",
            Discovery::Offer => "offer",
            Discovery::Answer => "answer",
            Discovery::Contact => "you",
            Discovery::Letters => "letters",
            Discovery::Hum => "hum",
            Discovery::Breath => "breath",
            Discovery::Lean => "lean",
            Discovery::Said => "said",
            Discovery::Voice => "voice",
            Discovery::Sing => "sing",
            Discovery::Whisper => "whisper",
            Discovery::Blow => "blow",
            Discovery::Startle => "startle",
            Discovery::Seen => "seen",
            Discovery::Blind => "blind",
            Discovery::Watched => "watched",
            Discovery::Shout => "shout",
            Discovery::Shaken => "shaken",
            Discovery::Scatter => "scatter",
            Discovery::Another => "another",
            Discovery::Ring => "ring",
            Discovery::Near => "near",
            Discovery::Clash => "clash",
            Discovery::Inside => "inside",
            Discovery::Give => "given back",
            Discovery::Between => "between",
            Discovery::Distance => "distance",
            Discovery::Everywhere => "everywhere",
            Discovery::Facet => "facet",
            Discovery::Backward => "backward",
            Discovery::Crease => "crease",
            Discovery::Kin => "kin",
            Discovery::Loose => "loose",
            Discovery::Part => "part",
            Discovery::Pass => "pass",
            Discovery::Many => "many",
            Discovery::Remembered => "remembered",
            Discovery::Wary => "wary",
            Discovery::Word => "word",
            Discovery::Greeting => "greeting",
            Discovery::Unanswered => "unanswered",
            Discovery::Dust => "dust",
            Discovery::Sweep => "sweep",
            Discovery::Mound => "mound",
            Discovery::Unearth => "unearth",
            Discovery::Scar => "scar",
            Discovery::Wound => "wound",
            Discovery::Tend => "tend",
            Discovery::Heal => "heal",
            Discovery::Root => "root",
            Discovery::Canopy => "canopy",
            Discovery::Wither => "wither",
            Discovery::Sway => "sway",
            Discovery::Ropes => "ropes",
            Discovery::Aloft => "aloft",
            Discovery::Braced => "braced",
            Discovery::InStep => "in step",
            Discovery::YourNote => "your note",
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
            Discovery::River => "close enough, they forgot they were twelve. now they run.",
            Discovery::Column => "it stands up to meet you. then it remembers it is liquid.",
            Discovery::Pulse => "you filled it. now it keeps your time.",
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
            Discovery::Regard => "it turns when you move. it did not used to.",
            Discovery::Flinch => "it moved before you did.",
            Discovery::Echo => "your rhythm, given back. slower, as if checking.",
            Discovery::Mimic => "your reach, made again from the inside.",
            Discovery::Initiate => "you stopped. it did not.",
            Discovery::Offer => "it held a piece of itself out. toward you.",
            Discovery::Answer => "it asked. you answered. neither of you knows what.",
            Discovery::Contact => "⍜ ∴⟡ ☽⊗⌬ ∵ ✶⍟",
            Discovery::Letters => "every letter lands somewhere. it keeps the places.",
            Discovery::Hum => "one note, held. it found the same one inside.",
            Discovery::Breath => "it breathes now. it did not need to before.",
            Discovery::Lean => "the room tipped. it went where the room went.",
            Discovery::Said => "its one word, given back. it is still listening.",
            Discovery::Voice => "it did not know you made sound.",
            Discovery::Sing => "your note came back, lower, from somewhere inside.",
            Discovery::Whisper => "it goes quiet to hear you better.",
            Discovery::Blow => "your breath took a piece of it away.",
            Discovery::Startle => "too fast. it remembers what fast means.",
            Discovery::Seen => "it has seen you now. it cannot unsee you.",
            Discovery::Blind => "you took the light away. the room went with it.",
            Discovery::Watched => "the eye found the one looking back.",
            Discovery::Shout => "it does not know the word. it knows the size of it.",
            Discovery::Shaken => "the room moved. it had thought rooms could not.",
            Discovery::Scatter => "it came loose where it was thinnest.",
            Discovery::Another => "it was never the only one.",
            Discovery::Ring => "it rang. the soft one felt it first.",
            Discovery::Near => "one of them reached. the other let it.",
            Discovery::Clash => "neither gave. both will remember.",
            Discovery::Inside => "it took the other one in. it is heavier now.",
            Discovery::Give => "it gave the other one back. neither is the color it was.",
            Discovery::Between => "you hurt the soft one. the hard one came between.",
            Discovery::Distance => "they keep a distance. neither will say why.",
            Discovery::Everywhere => "there was always more of it than one room could hold.",
            Discovery::Facet => "one more mirror. it is in all of them.",
            Discovery::Backward => "it unturned. the stars went back where they were.",
            Discovery::Crease => "the room folded back up. it kept the crease.",
            Discovery::Kin => "two of the twelve stood close, and chose to stay.",
            Discovery::Loose => "one came away. the rest closed around the gap.",
            Discovery::Part => "cut between them. now each side keeps its own.",
            Discovery::Pass => "one was touched. the others felt it after.",
            Discovery::Many => "not one, not twelve. as many as you leave them.",
            Discovery::Remembered => "it knew your hand before it touched it.",
            Discovery::Wary => "it has not forgotten what your hand did.",
            Discovery::Word => "a word landed where a hand would have.",
            Discovery::Greeting => "it answered in the only words it has.",
            Discovery::Unanswered => "it heard you. it remembers the rest.",
            Discovery::Dust => "it let go of every part of itself at once.",
            Discovery::Sweep => "pushed, every grain leans back toward the rest.",
            Discovery::Mound => "it is heavy now. it chose the ground.",
            Discovery::Unearth => "it came up out of itself and left nothing behind.",
            Discovery::Scar => "it closed. it did not forget where.",
            Discovery::Wound => "the old place opened. something fell out of it.",
            Discovery::Tend => "what went in stayed in. it is less open.",
            Discovery::Heal => "full again. the seam is thinner. it stays.",
            Discovery::Root => "it reached down. the ground let it.",
            Discovery::Canopy => "it grew toward the light, and kept the flower it was.",
            Discovery::Wither => "it forgot the light, a little at a time.",
            Discovery::Sway => "the room kept turning after it stopped.",
            Discovery::Ropes => "wound past holding, it let go all at once.",
            Discovery::Aloft => "turned enough, the ground let go of it.",
            Discovery::Braced => "it knows which way you turn it now. it turns back.",
            Discovery::InStep => "it knows which way you turn it now. it turns with you.",
            Discovery::YourNote => "it sang first. the note was yours.",
        }
    }
}

#[derive(Default)]
pub struct Progress {
    found: Vec<Discovery>,
    /// Names this build does not know, from a save made by a newer one (another tab, left open
    /// across a release): never counted, but written back, so this build never erases them.
    unknown: Vec<String>,
}

/// The most unknown names kept, so a hand-edited save cannot grow without end.
const UNKNOWN_KEPT: usize = 64;

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
        let known = self.found.iter().map(|d| d.name());
        known.chain(self.unknown.iter().map(String::as_str)).collect::<Vec<_>>().join(",")
    }

    /// Takes in what another tab kept: what was found only grows, so nothing found in one is lost
    /// to a save from the other. Returns how many were new here.
    pub fn merge(&mut self, saved: &str) -> usize {
        let kept = Progress::load(saved);
        for n in kept.unknown {
            self.keep_unknown(n);
        }
        kept.found.into_iter().filter(|d| self.discover(*d)).count()
    }

    fn keep_unknown(&mut self, name: String) {
        if self.unknown.len() < UNKNOWN_KEPT && !self.unknown.contains(&name) {
            self.unknown.push(name);
        }
    }

    /// Unknown names are kept aside, never counted, and repeats counted once, so a hand-edited or
    /// stale save can never inflate the count the contact arc is gated on.
    pub fn load(saved: &str) -> Progress {
        let mut p = Progress::default();
        for n in saved.split(',').map(str::trim) {
            match ALL.iter().copied().find(|d| d.name() == n) {
                Some(d) => {
                    p.discover(d);
                }
                None if !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') => {
                    p.keep_unknown(n.to_string());
                }
                None => {}
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_newer_build_found_is_kept_but_not_counted() {
        let mut here = Progress::default();
        here.discover(ALL[0]);
        here.merge(&format!("{},somethingnew", ALL[1].name()));
        assert_eq!(here.count(), 2);
        assert!(here.save().split(',').any(|n| n == "somethingnew"));
        assert_eq!(Progress::load(&here.save()).count(), 2);
    }

    #[test]
    fn what_another_tab_found_is_kept() {
        let mut here = Progress::default();
        here.discover(ALL[0]);
        let mut there = Progress::default();
        there.discover(ALL[1]);
        there.discover(ALL[0]);
        assert_eq!(here.merge(&there.save()), 1);
        assert!(here.has(ALL[0]) && here.has(ALL[1]) && here.count() == 2);
    }

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
        assert_eq!(Progress::load("poke,poke,tear,poke").count(), 2);
    }

    #[test]
    fn all_lists_every_discovery() {
        // Discoveries missing from ALL are forgotten between visits.
        // Keep the last variant here when appending to the enum.
        assert_eq!(ALL.len(), Discovery::YourNote as usize + 1);
        for (i, d) in ALL.iter().enumerate() {
            assert_eq!(*d as usize, i);
        }
    }
}
