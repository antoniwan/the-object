//! Words, once it hears you and the browser can make words of what it hears (platform/web/,
//! speech.rs). Nothing is explained to it; it simply takes some words as the hand they stand for.
//!
//! - say a verb and it is done to it, once for each time it is said: "poke poke poke" is three
//!   pokes, in the order said. Poke, cut, squeeze, hold, pull, tear, pinch, throw, come, in
//!   English or Spanish, mixed as you like, with a pronoun on the end or not ("córtalo"). The
//!   hand that does it is the same hand the pointer is, so every word does exactly what that
//!   gesture would, wherever it is.
//! - greet it, in any language, and what it does depends on how it has been treated, as
//!   remembrance.rs keeps it. Treated gently, it turns to you and answers in its own signs, each
//!   heard as a soft note as it appears: the same signs and the same short phrase for the same
//!   greeting, so a player can learn its word for theirs. Not sure of
//!   you yet, it only turns to listen. Hurt lately, it draws back and says nothing. Being
//!   greeted is itself remembered, as a small kindness.
//!
//! The words themselves are never kept: each piece of text is read for these words and dropped.

use crate::game::{signs, Game};
use crate::math::*;
use crate::mind::contact::SIGNS;
use crate::mind::remembrance::Deed;
use crate::progress::Discovery;

/// The pointer id the spoken hand uses; real pointers are never negative.
const HAND: i32 = -7;
/// Words waiting to be done, at most; more said at once are let go.
const QUEUE: usize = 8;
/// Between one spoken verb and the next (seconds).
const GAP: f32 = 0.14;
/// A greeting is answered at most this often (seconds).
const GREET_GAP: f32 = 6.0;
/// How long after a greeting (seconds) its answer starts to sound, a note for each sign.
const SIGNS_AFTER: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Poke,
    Cut,
    Squeeze,
    Hold,
    Pull,
    Tear,
    Pinch,
    Throw,
    Come,
}

/// Words that are the hand, by language. Accents are taken off before comparing, and a Spanish
/// word with a pronoun on the end ("córtalo", "tócala") is the word without it (`verb`).
const VERBS: [(Verb, &[&str]); 9] = [
    (
        Verb::Poke,
        &[
            "poke", "pokes", "poked", "poking", "prod", "boop", "tap", "touch", "nudge", "jab", "toca", "tocar",
            "toco", "toque", "pica", "picar", "pique", "puya", "puyar", "puyo", "puyalo",
        ],
    ),
    (
        Verb::Cut,
        &[
            "cut", "slice", "split", "chop", "halve", "cleave", "corta", "cortar", "corto", "corte", "parte", "partir",
            "divide", "dividir", "rebana", "rebanar",
        ],
    ),
    (
        Verb::Squeeze,
        &[
            "squeeze",
            "squish",
            "squash",
            "crush",
            "press",
            "compress",
            "aprieta",
            "apretar",
            "apreta",
            "exprime",
            "estruja",
            "estrujar",
            "apachurra",
            "apachurrar",
            "espachurra",
            "espachurrar",
            "aplasta",
            "aplastar",
            "presiona",
            "presionar",
        ],
    ),
    (
        Verb::Hold,
        &[
            "hold", "warm", "hug", "cuddle", "aguanta", "aguantar", "sosten", "sostener", "sujeta", "abraza",
            "abrazar", "calienta", "calentar", "agarra", "agarrar",
        ],
    ),
    (Verb::Pull, &["pull", "stretch", "drag", "tug", "estira", "estirar", "jala", "jalar", "hala", "halar"]),
    (Verb::Tear, &["tear", "rip", "break", "arranca", "arrancar", "rompe", "romper", "rasga", "rasgar", "desgarra"]),
    (Verb::Pinch, &["pinch", "nip", "pellizca", "pellizcar", "pellizco", "pellizque"]),
    (
        Verb::Throw,
        &[
            "throw", "slam", "drop", "toss", "fling", "lanza", "lanzar", "tira", "tirar", "bota", "botar", "avienta",
            "aventar", "arroja", "arrojar", "estrella",
        ],
    ),
    (Verb::Come, &["come", "closer", "ven", "venga", "vente", "acercate", "acercar", "aca", "aqui"]),
];

/// Pronouns Spanish puts on the end of a command, longest first.
const ENCLITICS: [&str; 7] = ["selo", "sela", "los", "las", "lo", "la", "le"];

fn verb(word: &str) -> Option<Verb> {
    let find = |w: &str| VERBS.iter().find(|(_, words)| words.contains(&w)).map(|(v, _)| *v);
    find(word)
        .or_else(|| ENCLITICS.iter().find_map(|e| word.strip_suffix(e).filter(|stem| stem.len() >= 2).and_then(find)))
}

/// Greetings, written as they come back from the browser (lowercase, accents off). The ones in
/// scripts without spaces are found anywhere in what was said; the rest only as whole words.
const GREETINGS: &[&str] = &[
    // english
    "hi",
    "hello",
    "hey",
    "hiya",
    "howdy",
    "greetings",
    "good morning",
    "good afternoon",
    "good evening",
    "good day",
    "morning",
    "hey there",
    "hullo",
    // spanish
    "hola",
    "buenos dias",
    "buen dia",
    "buenas tardes",
    "buenas noches",
    "buenas",
    "saludos",
    "que tal",
    "wepa",
    "alo",
    "jelou",
    // and others
    "bonjour",
    "bonsoir",
    "salut",
    "ciao",
    "buongiorno",
    "buonasera",
    "salve",
    "hallo",
    "guten morgen",
    "guten tag",
    "guten abend",
    "servus",
    "moin",
    "ola",
    "oi",
    "bom dia",
    "boa tarde",
    "boa noite",
    "hej",
    "hei",
    "god morgen",
    "goedemorgen",
    "goedendag",
    "ahoj",
    "czesc",
    "dzien dobry",
    "szia",
    "merhaba",
    "gunaydin",
    "salam",
    "salaam",
    "marhaba",
    "shalom",
    "namaste",
    "namaskar",
    "aloha",
    "jambo",
    "habari",
    "sawubona",
    "kia ora",
    "konnichiwa",
    "ohayo",
    "ohayou",
    "annyeong",
    "annyeonghaseyo",
    "ni hao",
    "nihao",
    "xin chao",
    "sawasdee",
    "kamusta",
    "kumusta",
    "selamat pagi",
    "privet",
    "zdravstvuyte",
    "yassou",
    "geia",
    "terve",
    "moi",
    "tere",
    "labas",
    "sveiki",
    "zdravo",
    "bok",
    "dobar dan",
];

/// Greetings in scripts that do not put spaces between words.
const GREETINGS_ANYWHERE: &[&str] = &[
    "こんにちは",
    "おはよう",
    "こんばんは",
    "你好",
    "您好",
    "早上好",
    "안녕",
    "привет",
    "здравствуй",
    "доброе утро",
    "добрый день",
    "γεια",
    "καλημέρα",
    "مرحبا",
    "السلام عليكم",
    "صباح الخير",
    "שלום",
    "नमस्ते",
    "नमस्कार",
    "สวัสดี",
    "xin chào",
];

/// Lowercase, accents off, everything that is not a letter or a digit made a space.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push(' ');
    for c in text.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            'ś' => 's',
            'ł' => 'l',
            'ğ' => 'g',
            'ż' | 'ź' => 'z',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        out.push(c);
    }
    out.push(' ');
    out
}

/// Every verb in what was said, in order, as often as it was said.
pub fn verbs(text: &str) -> Vec<Verb> {
    plain(text).split_whitespace().filter_map(verb).collect()
}

/// Whether anything in what was said is a word it takes.
pub fn known(text: &str) -> bool {
    !verbs(text).is_empty() || greeting(text).is_some()
}

/// Of the browser's guesses at what was said, best first, the one with the most of its words in
/// it: a "poke" heard as "polk" first and "poke" second is still a poke.
pub fn likeliest(guesses: &[String]) -> &str {
    let score = |t: &str| verbs(t).len() + usize::from(greeting(t).is_some());
    let best = guesses.iter().map(|g| score(g)).max().unwrap_or(0);
    guesses.iter().find(|g| score(g) == best).map(String::as_str).unwrap_or("")
}

/// The greeting in what was said, if there is one.
pub fn greeting(text: &str) -> Option<&'static str> {
    let p = plain(text);
    let lower = text.to_lowercase();
    GREETINGS
        .iter()
        .find(|g| p.contains(&format!(" {g} ")))
        .or_else(|| GREETINGS_ANYWHERE.iter().find(|g| lower.contains(*g)))
        .copied()
}

/// What it makes of you.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Kind,
    Unsure,
    Wary,
}

/// What it makes of you, from what it remembers.
pub fn mood(g: &Game) -> Mood {
    if g.remembrance.kind() {
        Mood::Kind
    } else if g.remembrance.wary() {
        Mood::Wary
    } else {
        Mood::Unsure
    }
}

/// A spoken verb being done, `t` seconds in.
struct Doing {
    verb: Verb,
    t: f32,
    /// Where it is on the canvas, how big, and which way the hand comes from, when it began.
    at: (f32, f32),
    r: f32,
    side: (f32, f32),
    down: bool,
}

impl Doing {
    fn length(&self) -> f32 {
        match self.verb {
            Verb::Poke | Verb::Come => 0.05,
            Verb::Cut => 0.18,
            Verb::Squeeze => 0.6,
            Verb::Hold => 1.9,
            Verb::Pull => 1.5,
            Verb::Tear => 0.9,
            Verb::Pinch => 0.12,
            Verb::Throw => 0.3,
        }
    }

    /// Where the hand is at `t`, on the canvas, and whether it is pressed at all.
    fn hand(&self, t: f32, canvas: (f32, f32)) -> Option<(f32, f32)> {
        let (c, r, s) = (self.at, self.r, self.side);
        let along = |k: f32| (c.0 + s.0 * r * k, c.1 + s.1 * r * k);
        let ease = |x: f32| smoothstep(0.0, 1.0, x);
        Some(match self.verb {
            Verb::Poke => along(0.25),
            Verb::Squeeze => return None,
            Verb::Hold => c,
            // Out from just inside the edge, held a moment, then let back slowly.
            Verb::Pull => {
                let k = if t < 0.6 {
                    ease(t / 0.6)
                } else if t < 1.0 {
                    1.0
                } else {
                    1.0 - ease((t - 1.0) / 0.5) * 0.8
                };
                along(0.5 + 1.1 * k)
            }
            Verb::Tear => along(0.5 + 4.5 * ease(t / 0.55)),
            Verb::Pinch => along(0.55 + 0.5 * (t / 0.08).min(1.0)),
            // Lifted a little, then thrown down hard.
            Verb::Throw => {
                let up = if t < 0.18 { ease(t / 0.18) } else { 1.0 };
                let down = ((t - 0.18) / 0.1).clamp(0.0, 1.0);
                (c.0, c.1 - r * 0.4 * up + canvas.1 * 0.9 * down)
            }
            // Clean through, from off one side to off the other, crossing the middle.
            Verb::Cut => {
                let k = (t / self.length()).min(1.0) * 2.0 - 1.0;
                (c.0 + s.0 * r * 2.4 * k + s.1 * r * 0.2, c.1 + s.1 * r * 2.4 * k - s.0 * r * 0.2)
            }
            // A tap on the empty floor in front of it.
            Verb::Come => (c.0 + s.0 * r * 0.6, (c.1 + r * 2.4).min(canvas.1 * 0.94)),
        })
    }
}

#[derive(Default)]
pub struct Words {
    /// Words are reaching it now (set by the page).
    pub open: bool,
    /// For each piece of text still being said: which one it is, how many of its verbs have
    /// been done, and whether its greeting was answered. A browser rewrites a piece of text
    /// while it is being said, so only verbs beyond those already done are new.
    heard: Vec<((u32, u32), usize, bool)>,
    queue: Vec<Verb>,
    doing: Option<Doing>,
    wait: f32,
    /// Seconds since it last answered a greeting, if it has this visit.
    since_greet: Option<f32>,
    /// The signs it answered a greeting with, for the page to show.
    pub said: Vec<String>,
    pokes: u32,
    greeted: Option<&'static str>,
    turn: Option<(f32, Mood)>,
}

impl Words {
    /// A piece of what was said: the `index`th of browser session `session`, as it stands now.
    pub fn hear(&mut self, session: u32, index: u32, text: &str) {
        if !self.open {
            return;
        }
        let key = (session, index);
        let slot = match self.heard.iter().position(|h| h.0 == key) {
            Some(i) => i,
            None => {
                // Only the last few pieces can still change.
                if self.heard.len() >= 8 {
                    self.heard.remove(0);
                }
                self.heard.push((key, 0, false));
                self.heard.len() - 1
            }
        };
        let found = verbs(text);
        let done = self.heard[slot].1;
        for v in found.iter().skip(done) {
            if self.queue.len() < QUEUE {
                self.queue.push(*v);
            }
        }
        self.heard[slot].1 = done.max(found.len());
        if !self.heard[slot].2 {
            if let Some(g) = greeting(text) {
                self.heard[slot].2 = true;
                self.greeted = Some(g);
            }
        }
    }

    /// Stop: nothing more arrives, and nothing said is still waiting to be done.
    pub fn close(&mut self) {
        self.open = false;
        self.queue.clear();
        self.heard.clear();
        self.greeted = None;
    }

    /// Whether the spoken hand is pressing now.
    pub fn pressing(&self) -> bool {
        self.doing.as_ref().is_some_and(|d| d.down)
    }
}

/// The signs it answers a greeting with: the same greeting always gets the same signs.
pub fn answer(greeting: &str) -> String {
    let mut seed = greeting.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let len = 2 + next() % 3;
    let mut word: String = (0..len).map(|_| SIGNS[(next() % SIGNS.len() as u32) as usize]).collect();
    // Said twice, the second time a little shorter, the way a greeting is given back.
    let short: String = word.chars().take(len as usize - 1).collect();
    word.push(' ');
    word.push_str(&short);
    word
}

pub fn step(g: &mut Game, dt: f32) {
    let w = &mut g.words;
    if let Some(t) = &mut w.since_greet {
        *t += dt;
    }
    if let Some(greeting) = w.greeted.take() {
        if w.since_greet.is_none_or(|t| t >= GREET_GAP) {
            w.since_greet = Some(0.0);
            greet(g, greeting);
        }
    }
    step_turn(g, dt);
    step_hand(g, dt);
}

/// What it is doing about a greeting, for a moment after.
fn step_turn(g: &mut Game, dt: f32) {
    let Some((t, mood)) = &mut g.words.turn else { return };
    *t += dt;
    let (t, mood) = (*t, *mood);
    let b = g.cam.basis();
    match mood {
        Mood::Kind => {
            g.world.gaze_dir = -b.fwd;
            g.world.watching = true;
            if (t - dt..t).contains(&0.6) {
                g.world.add_ripple((-b.fwd + b.up * 0.35).norm(), 0.35);
            }
        }
        Mood::Unsure => {
            g.world.gaze_dir = -b.fwd;
            g.world.watching = true;
        }
        Mood::Wary => {
            g.world.watching = false;
            g.world.body.vel += b.fwd * (0.6 * dt);
        }
    }
    if t > 2.5 {
        g.words.turn = None;
    }
}

fn greet(g: &mut Game, greeting: &'static str) {
    let mood = mood(g);
    let b = g.cam.basis();
    let ordinary = g.world.ordinary();
    g.remembrance.remember(Deed::Greeted);
    match mood {
        Mood::Kind => {
            if ordinary {
                g.world.reach(-b.fwd);
            }
            let said = answer(greeting);
            // Each sign is heard as it appears: the same greeting, the same short phrase.
            g.sounds.extend(signs(&said, SIGNS_AFTER));
            g.words.said.push(said);
            g.discover(Discovery::Greeting);
        }
        Mood::Unsure => {
            if ordinary {
                g.world.reach(-b.fwd);
            }
        }
        Mood::Wary => {
            g.world.body.vel += b.fwd * 0.5;
            g.world.agitation = g.world.agitation.max(0.15);
            g.discover(Discovery::Unanswered);
        }
    }
    g.words.turn = Some((0.0, mood));
}

/// A real finger has come down (`Game::pointer_down`): the spoken hand lifts and waits, so
/// the two are never taken for one squeeze.
pub fn give_way(g: &mut Game, id: i32) {
    if id < 0 || g.words.doing.is_none() {
        return;
    }
    if g.words.pressing() {
        g.drop_hand(HAND);
    }
    g.words.doing = None;
    g.words.wait = GAP;
}

/// The spoken hand: one verb at a time, never while a real hand is on it.
fn step_hand(g: &mut Game, dt: f32) {
    let canvas = (g.w, g.h);
    if let Some(d) = &mut g.words.doing {
        d.t += dt;
        let (t, len) = (d.t, d.length());
        match d.hand(t, canvas) {
            // said, a squeeze presses it flat
            None => g.wheel(60.0 * dt * 60.0),
            Some((x, y)) if !d.down => {
                d.down = true;
                g.pointer_down(HAND, x, y);
            }
            Some((x, y)) => g.pointer_move(HAND, x, y, dt),
        }
        if t >= len {
            if g.words.pressing() {
                g.pointer_up(HAND);
            }
            g.words.doing = None;
            g.words.wait = GAP;
        }
        return;
    }
    g.words.wait = (g.words.wait - dt).max(0.0);
    if g.words.wait > 0.0 || g.words.queue.is_empty() || g.hands() > 0 {
        return;
    }
    let at = g.screen_of(g.world.body.pos);
    let edge = g.screen_of(g.world.body.pos + g.cam.basis().right * g.world.body.radius());
    let r = ((edge.0 - at.0).powi(2) + (edge.1 - at.1).powi(2)).sqrt().max(g.h * 0.03);
    let w = &mut g.words;
    let verb = w.queue.remove(0);
    // Each word comes from a little further round, so a row of pokes lands in a row of places.
    w.pokes = w.pokes.wrapping_add(1);
    let a = w.pokes as f32 * 2.4;
    let side = match verb {
        Verb::Cut | Verb::Pull | Verb::Tear | Verb::Pinch => (1.0, 0.0),
        _ => (a.cos(), a.sin()),
    };
    w.doing = Some(Doing { verb, t: 0.0, at, r, side, down: false });
    g.discover(Discovery::Word);
    step_hand(g, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::new();
        g.w = 800.0;
        g.h = 600.0;
        g.words.open = true;
        g
    }

    /// Treated gently, or not, often enough for it to know.
    fn treated(g: &mut Game, deed: Deed, times: usize) {
        for _ in 0..times {
            g.remembrance.remember(deed);
        }
    }

    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0);
        }
    }

    #[test]
    fn it_finds_verbs_as_often_as_they_are_said() {
        assert_eq!(verbs("poke poke poke poke poke"), vec![Verb::Poke; 5]);
        assert_eq!(verbs("Poke-poke, POKE!"), vec![Verb::Poke; 3]);
        assert_eq!(verbs("aprieta y luego córtalo"), vec![Verb::Squeeze, Verb::Cut]);
        assert_eq!(verbs("Tócala, estíralo, ¡apachúrralo!"), vec![Verb::Poke, Verb::Pull, Verb::Squeeze]);
        assert_eq!(verbs("poke it, ahora puya, then cut"), vec![Verb::Poke, Verb::Poke, Verb::Cut]);
        assert!(verbs("hola la luz lo sé").is_empty(), "a pronoun alone is not a verb");
        assert_eq!(verbs("corta, estira, ven"), vec![Verb::Cut, Verb::Pull, Verb::Come]);
        assert!(verbs("this is a spoken sentence").is_empty(), "not inside other words");
    }

    #[test]
    fn of_the_guesses_it_takes_the_one_it_understands() {
        let g = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(likeliest(&g(&["polk polk", "poke poke", "poke"])), "poke poke");
        assert_eq!(likeliest(&g(&["what is this", "what is these"])), "what is this");
        assert_eq!(likeliest(&[]), "");
        assert!(known("ven acá") && known("buenas") && !known("what is this"));
    }

    #[test]
    fn it_knows_a_greeting_in_many_languages() {
        for g in
            ["hi", "Good morning!", "hola", "Buenos días", "bonjour", "おはようございます", "你好", "привет", "Wepa"]
        {
            assert!(greeting(g).is_some(), "{g}");
        }
        for g in ["this", "high", "yo quiero", "chill"] {
            assert_eq!(greeting(g), None, "{g}");
        }
        assert_eq!(answer("hola"), answer("hola"));
        assert!(answer("hola").chars().all(|c| c == ' ' || SIGNS.contains(&c)));
    }

    #[test]
    fn five_pokes_said_are_five_pokes_done() {
        let mut g = game();
        // The browser rewrites what is being said as it goes; nothing is done twice.
        g.words.hear(0, 0, "poke");
        g.words.hear(0, 0, "poke poke");
        g.words.hear(0, 0, "poke poke poke");
        g.words.hear(0, 0, "poke poke poke poke poke");
        g.words.hear(0, 0, "poke poke poke poke poke");
        run(&mut g, 3.0);
        assert_eq!(g.contact.habits[0], 5);
        assert!(g.progress.has(Discovery::Poke) && g.progress.has(Discovery::Word));
    }

    #[test]
    fn closed_it_takes_no_words() {
        let mut g = game();
        g.words.close();
        g.words.hear(0, 0, "poke");
        run(&mut g, 1.0);
        assert_eq!(g.contact.habits[0], 0);
    }

    #[test]
    fn each_verb_does_what_its_gesture_does() {
        let cases = [
            ("cut", Discovery::Divide),
            ("squeeze", Discovery::Squeeze),
            ("hold", Discovery::Warm),
            ("pull", Discovery::Stretch),
            ("tear", Discovery::Tear),
            ("pinch", Discovery::Pinch),
            ("throw", Discovery::Throw),
            ("come", Discovery::Call),
        ];
        for (word, d) in cases {
            let mut g = game();
            g.words.hear(0, 0, word);
            run(&mut g, 3.0);
            assert!(g.progress.has(d), "{word} did not {d:?}");
            assert!(!g.words.pressing(), "{word} let go");
        }
        // said, a squeeze presses it flat, as the wheel turned down does
        let mut g = game();
        g.words.hear(0, 0, "squeeze");
        let mut least = f32::MAX;
        for _ in 0..120 {
            g.update(1.0 / 60.0);
            least = least.min(g.world.body.squash);
        }
        assert!(least < 0.85, "{least}");
        // pulled, not torn
        let mut g = game();
        g.words.hear(0, 0, "pull");
        run(&mut g, 3.0);
        assert!(!g.progress.has(Discovery::Tear) && !g.progress.has(Discovery::Thread));
    }

    #[test]
    fn it_waits_for_a_real_hand_to_let_go() {
        let mut g = game();
        g.pointer_down(1, 100.0, 100.0);
        g.words.hear(0, 0, "poke");
        run(&mut g, 0.5);
        assert_eq!(g.contact.habits[0], 0);
        g.pointer_up(1);
        run(&mut g, 0.5);
        assert_eq!(g.contact.habits[0], 1);
    }

    #[test]
    fn a_real_finger_takes_over_from_the_spoken_hand() {
        let mut g = game();
        g.words.hear(0, 0, "pull");
        run(&mut g, 10.0 / 60.0);
        assert!(g.words.pressing());
        g.pointer_down(1, 100.0, 100.0);
        assert_eq!(g.hands(), 1, "the spoken hand gives way to the real one");
    }

    #[test]
    fn a_greeting_is_answered_only_if_it_has_been_treated_gently() {
        // kind
        let mut g = game();
        treated(&mut g, Deed::Held, 15);
        assert!(g.remembrance.kind());
        g.words.hear(0, 0, "buenos días");
        run(&mut g, 1.0);
        assert!(g.progress.has(Discovery::Greeting));
        assert_eq!(g.words.said, vec![answer("buenos dias")]);
        // hurt
        let mut g = game();
        treated(&mut g, Deed::Broken, 8);
        assert!(g.remembrance.wary());
        g.words.hear(0, 0, "hello");
        run(&mut g, 1.0);
        assert!(g.words.said.is_empty() && g.progress.has(Discovery::Unanswered));
        // not sure yet: it only listens
        let mut g = game();
        g.words.hear(0, 0, "hi");
        run(&mut g, 1.0);
        assert!(g.words.said.is_empty() && !g.progress.has(Discovery::Unanswered));
        assert!(g.world.gaze > 0.3, "it turned to listen: {}", g.world.gaze);
    }

    #[test]
    fn a_greeting_said_once_is_answered_once() {
        let mut g = game();
        treated(&mut g, Deed::Held, 15);
        g.words.hear(0, 0, "hi");
        g.words.hear(0, 0, "hi there");
        g.words.hear(0, 1, "hello");
        run(&mut g, 1.0);
        assert_eq!(g.words.said.len(), 1);
    }

    #[test]
    fn being_greeted_is_remembered() {
        let mut g = game();
        let before = g.remembrance.ledger().get(Deed::Greeted).count;
        g.words.hear(0, 0, "hola");
        run(&mut g, 0.5);
        assert_eq!(g.remembrance.ledger().get(Deed::Greeted).count, before + 1);
    }

    #[test]
    fn a_spoken_hold_cut_short_by_a_finger_is_not_a_poke() {
        let mut g = game();
        g.words.hear(0, 0, "hold");
        run(&mut g, 10.0 / 60.0);
        assert!(g.words.pressing());
        assert!(!g.progress.has(Discovery::Poke));
        // a real finger on empty space, far from the body
        g.pointer_down(1, 5.0, 5.0);
        assert_eq!(g.contact.habits[0], 0, "the cut-short hold landed as a poke");
        assert!(!g.progress.has(Discovery::Poke), "the cut-short hold landed as a poke");
    }
}
