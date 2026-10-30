//! Whether the screen is kept from going dark. Much of what it does is done while nobody
//! touches it: it watches after a while, a thought comes, the dust drifts together, it lives
//! slowly on its own. A phone that dims and locks itself after half a minute would take all of
//! that away, so from the player's first touch the page asks the browser to keep the screen on
//! (`platform/web/keep.rs`, the Screen Wake Lock), and lets it go when the page is left, taking
//! it again when it is looked at.
//!
//! Nobody there for `LET_GO_AFTER` (no hand, no key, the phone not moved, nothing heard or seen
//! because it was let hear or see) and it lets the screen go dark as it would have; the next
//! touch keeps it on again. Plain Rust: the browser half only does what `Do` says.

/// No input and no open sense for this long (seconds), and the screen is let go.
pub const LET_GO_AFTER: f32 = 180.0;
/// The phone turned this far (degrees) since the last frame, or pushed this hard (m/s²,
/// gravity taken out), is a hand holding it; lying on a table it is neither.
const MOVED_DEG: f32 = 1.5;
const MOVED_PUSH: f32 = 0.8;

/// What the browser half should do now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Do {
    Nothing,
    /// Ask for the screen to be kept on.
    Take,
    /// Let it go.
    Let,
}

#[derive(Default)]
pub struct Wake {
    /// It should be held while the page is seen: taken by a touch, not yet let go for being left.
    on: bool,
    /// The page is hidden: the browser lets go of it then, and it is taken again on the way back.
    hidden: bool,
    /// Seconds since anyone was last there.
    idle: f32,
    /// How the phone was last held, to tell a hand from a table.
    held: Option<(f32, f32)>,
}

impl Wake {
    /// A press or a key: only inside one may the browser be asked.
    pub fn touched(&mut self) -> Do {
        self.idle = 0.0;
        if self.on || self.hidden {
            return Do::Nothing;
        }
        self.on = true;
        Do::Take
    }

    /// The pointer moved or the wheel turned: someone is there, but this is no gesture to ask in.
    pub fn stirred(&mut self) {
        self.idle = 0.0;
    }

    /// How the phone was held this frame and its hardest push, if anything arrived.
    pub fn felt(&mut self, held: Option<(f32, f32)>, push: Option<f32>) {
        let turned = match (held, self.held) {
            (Some(now), Some(was)) => (now.0 - was.0).abs().max((now.1 - was.1).abs()) > MOVED_DEG,
            _ => false,
        };
        if let Some(now) = held {
            self.held = Some(now);
        }
        if turned || push.is_some_and(|p| p > MOVED_PUSH) {
            self.idle = 0.0;
        }
    }

    /// A frame: `sensing` while it is let hear or see.
    pub fn step(&mut self, dt: f32, sensing: bool) -> Do {
        self.idle = if sensing { 0.0 } else { self.idle + dt };
        if self.on && self.idle >= LET_GO_AFTER {
            self.on = false;
            return if self.hidden { Do::Nothing } else { Do::Let };
        }
        Do::Nothing
    }

    /// The page was left.
    pub fn hide(&mut self) -> Do {
        let was = !self.hidden && self.on;
        self.hidden = true;
        if was {
            Do::Let
        } else {
            Do::Nothing
        }
    }

    /// The page is seen again: held before it was left, it is taken again.
    pub fn show(&mut self) -> Do {
        let was = self.hidden && self.on;
        self.hidden = false;
        if was {
            Do::Take
        } else {
            Do::Nothing
        }
    }

    /// The browser would not keep it, or let it go by itself while the page was seen: the next
    /// touch asks again.
    pub fn lost(&mut self) {
        self.on = false;
    }

    /// Whether it means the screen to be kept on now.
    pub fn holding(&self) -> bool {
        self.on && !self.hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(w: &mut Wake, secs: f32, sensing: bool) -> Vec<Do> {
        (0..(secs * 10.0) as usize).map(|_| w.step(0.1, sensing)).filter(|d| *d != Do::Nothing).collect()
    }

    #[test]
    fn the_first_touch_keeps_the_screen_on_and_only_once() {
        let mut w = Wake::default();
        assert!(wait(&mut w, 10.0, false).is_empty(), "nothing before a touch");
        assert!(!w.holding());
        assert_eq!(w.touched(), Do::Take);
        assert_eq!(w.touched(), Do::Nothing, "already held");
        assert!(w.holding());
    }

    #[test]
    fn left_alone_it_lets_the_screen_go_and_a_touch_takes_it_again() {
        let mut w = Wake::default();
        w.touched();
        assert!(wait(&mut w, LET_GO_AFTER - 1.0, false).is_empty());
        w.stirred();
        assert!(wait(&mut w, LET_GO_AFTER - 1.0, false).is_empty(), "a moving pointer is someone");
        assert_eq!(wait(&mut w, 2.0, false), vec![Do::Let]);
        assert!(!w.holding());
        assert!(wait(&mut w, 600.0, false).is_empty(), "let go once");
        w.stirred();
        assert!(!w.holding(), "a pointer passing over is no gesture to ask in");
        assert_eq!(w.touched(), Do::Take);
    }

    #[test]
    fn while_it_hears_or_sees_it_keeps_the_screen_on() {
        let mut w = Wake::default();
        w.touched();
        assert!(wait(&mut w, LET_GO_AFTER * 3.0, true).is_empty());
        assert_eq!(wait(&mut w, LET_GO_AFTER + 1.0, false), vec![Do::Let], "stopped, it counts from then");
    }

    #[test]
    fn a_phone_in_the_hand_is_someone_and_a_phone_on_a_table_is_not() {
        let mut w = Wake::default();
        w.touched();
        for i in 0..(LET_GO_AFTER as usize * 2) {
            // a hand: it never stays quite still
            let lean = if i % 2 == 0 { 20.0 } else { 22.0 };
            w.felt(Some((lean, 3.0)), Some(0.3));
            assert_eq!(w.step(1.0, false), Do::Nothing);
        }
        let mut lets = Vec::new();
        for _ in 0..(LET_GO_AFTER as usize + 2) {
            // a table: the same angles, give or take the sensor's noise
            w.felt(Some((0.1, -0.2)), Some(0.05));
            lets.push(w.step(1.0, false));
        }
        assert_eq!(lets.iter().filter(|d| **d == Do::Let).count(), 1);
        let mut w = Wake::default();
        w.touched();
        w.felt(None, Some(2.0));
        assert!(wait(&mut w, LET_GO_AFTER - 1.0, false).is_empty(), "a jolt is someone");
    }

    #[test]
    fn leaving_the_page_lets_it_go_and_coming_back_takes_it_again() {
        let mut w = Wake::default();
        assert_eq!((w.hide(), w.show()), (Do::Nothing, Do::Nothing), "never taken, never taken again");
        w.touched();
        assert_eq!(w.hide(), Do::Let);
        assert_eq!(w.hide(), Do::Nothing);
        assert_eq!(w.touched(), Do::Nothing, "not while nobody can see it");
        assert_eq!(w.show(), Do::Take);
        assert!(w.holding());
        // Let go for being left alone, it is not taken again just by coming back.
        wait(&mut w, LET_GO_AFTER + 1.0, false);
        w.hide();
        assert_eq!(w.show(), Do::Nothing);
        assert_eq!(w.touched(), Do::Take);
    }

    #[test]
    fn a_lock_the_browser_would_not_keep_is_asked_for_at_the_next_touch() {
        let mut w = Wake::default();
        w.touched();
        w.lost();
        assert!(!w.holding());
        assert!(wait(&mut w, LET_GO_AFTER * 2.0, false).is_empty(), "nothing to let go");
        assert_eq!(w.touched(), Do::Take);
    }
}
