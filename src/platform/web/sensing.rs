//! The microphone, the camera, the phone's motion and the browser's speech: opening and closing
//! them at the player's word, and taking in what they gave since the last frame.

use super::App;
use crate::input::senses::{self, Kind};
use crate::input::words;
use crate::platform::devices::{self, Ears, Eyes};
use crate::platform::speech::{self, Can, Note, Speech};
use wasm_bindgen::prelude::*;

/// Where the browser can only make words by sending the voice away, it may still ask for that,
/// in its own word, with the reason under it (`Kind::Words`).
const WORDS_AWAY: bool = true;

/// Things said in a row with no word in them it knows, before it listens in the other language.
const UNKNOWN_TURN: u32 = 2;

impl App {
    /// Stops hearing (or seeing) at once: the device is let go before anything else happens.
    pub(super) fn close(&mut self, kind: Kind) {
        self.game.senses.close(kind);
        match kind {
            Kind::Hear => {
                self.let_ear_context_go();
                self.ears.take().map(Ears::close)
            }
            Kind::See => self.eyes.take().map(Eyes::close),
            Kind::Feel => None,
            Kind::Words => self.speech.take().map(Speech::stop),
        };
        // Words come out of hearing: without it, there are none.
        if kind == Kind::Hear {
            self.close(Kind::Words);
        }
        if matches!(kind, Kind::Hear | Kind::Words) {
            self.game.words.close();
        }
        if self.offer.is_some_and(|o| o.0 == kind) {
            self.offer = None;
        }
        self.show_marks();
    }

    /// The context made for the microphone in the tap, if it never became ears: closed, so a
    /// browser's few audio contexts are not used up by asking.
    fn let_ear_context_go(&mut self) {
        if let Some(ctx) = self.ear_ctx.take() {
            let _ = ctx.close();
        }
    }

    pub(super) fn close_all(&mut self) {
        self.close(Kind::Hear);
        self.close(Kind::See);
    }

    /// A waiting mark (or the microphone) was tapped: the same as tapping the word it asks with.
    pub(super) fn take_waiting(&mut self, kind: Kind) {
        if self.game.senses.accept_waiting(kind) {
            // Its word may still be up: it has been answered.
            if self.offer.is_some_and(|o| o.0 == kind) {
                self.offer = None;
                self.hud.toast_until = 0.0;
            }
            self.open(kind);
            self.show_marks();
        }
    }

    /// The word was tapped while it was up: now, inside the tap, the browser may ask.
    pub(super) fn take_offer(&mut self, now: f64) {
        let Some((kind, until)) = self.offer.take() else { return };
        if now / 1000.0 > until || !self.game.senses.accept(kind) {
            return;
        }
        self.open(kind);
        self.hud.toast_until = 0.0;
    }

    /// Inside the player's tap: the browser asks for what it was let have.
    pub(super) fn open(&mut self, kind: Kind) {
        match kind {
            Kind::Feel => devices::ask_to_feel(self.feel_answers.clone()),
            Kind::Hear => {
                self.let_ear_context_go();
                self.ear_ctx = devices::ear_context();
                devices::ask(kind, self.answers.clone());
                // Whether words can be made on the device is only asked now, inside the tap.
                if self.can_speech.get() == Can::Perhaps {
                    speech::ready(self.can_speech.clone(), &self.tongues);
                }
            }
            Kind::See => devices::ask(kind, self.answers.clone()),
            Kind::Words => match Speech::start(false, &self.tongues, self.notes.clone()) {
                Some(sp) => {
                    self.speech = Some(sp);
                    self.game.senses.opened(Kind::Words);
                    self.show_marks();
                }
                None => self.game.senses.refused(Kind::Words, false),
            },
        }
    }

    /// Starts feeling the phone, once.
    pub(super) fn feel(&mut self) {
        if !self.feeling {
            self.feeling = true;
            devices::listen_to_feel(self.felt.clone());
        }
    }

    /// What the browser answered; and, while it is let, what it hears, sees and feels this frame.
    pub(super) fn sense(&mut self, now: f64) {
        let answers = std::mem::take(&mut *self.answers.borrow_mut());
        for (kind, answer) in answers {
            let waiting = match kind {
                Kind::Hear => self.game.senses.hear == senses::Leave::Opening,
                Kind::See => self.game.senses.see == senses::Leave::Opening,
                Kind::Feel | Kind::Words => false,
            };
            match answer {
                // Stopped while the browser was still asking: let it go unused.
                Ok(stream) if !waiting => {
                    devices::stop(&stream);
                    if kind == Kind::Hear {
                        self.let_ear_context_go();
                    }
                }
                Ok(stream) => {
                    let opened = match kind {
                        Kind::Hear => match self.ear_ctx.take() {
                            Some(ctx) => Ears::new(ctx.clone(), stream.clone()).map(|e| self.ears = Some(e)).inspect_err(|_| {
                                let _ = ctx.close();
                            }),
                            None => Err(JsValue::from_str("no audio context")),
                        },
                        Kind::See => Eyes::new(stream.clone()).map(|e| self.eyes = Some(e)),
                        Kind::Feel | Kind::Words => Err(JsValue::from_str("nothing to open")),
                    };
                    match opened {
                        Ok(()) => self.game.senses.opened(kind),
                        Err(e) => {
                            web_sys::console::warn_1(&e);
                            devices::stop(&stream);
                            self.game.senses.refused(kind, false);
                        }
                    }
                    self.show_marks();
                }
                Err(no) => {
                    if kind == Kind::Hear {
                        self.let_ear_context_go();
                    }
                    self.game.senses.refused(kind, no);
                }
            }
        }
        let felt = std::mem::take(&mut *self.feel_answers.borrow_mut());
        for answer in felt {
            match answer {
                Ok(()) => {
                    self.game.senses.opened(Kind::Feel);
                    self.feel();
                }
                Err(no) => self.game.senses.refused(Kind::Feel, no),
            }
        }
        let f = std::mem::take(&mut *self.felt.borrow_mut());
        self.game.motion.feel(f);
        if let Some(ears) = &mut self.ears {
            let heard = ears.listen();
            self.game.senses.heard(heard);
        }
        if let Some(seen) = self.eyes.as_mut().and_then(|e| e.look(now)) {
            self.game.senses.seen(seen);
        }
        self.words();
    }

    /// Words, while it hears: on the device as soon as it hears, away only once let (`take_offer`).
    pub(super) fn words(&mut self) {
        let can = self.can_speech.get();
        self.game.senses.can_words = WORDS_AWAY && can == Can::Away && self.game.senses.can_hear;
        if self.ears.is_some() && self.speech.is_none() && can == Can::Here && !self.speech_gone {
            self.speech = Speech::start(true, &self.tongues, self.notes.clone());
            self.speech_gone = self.speech.is_none();
        }
        let notes = std::mem::take(&mut *self.notes.borrow_mut());
        for note in notes {
            match note {
                Note::Text { session, index, guesses, done } => {
                    let text = words::likeliest(&guesses);
                    self.game.words.hear(session, index, text);
                    // Twice in a row nothing it knows: perhaps it is listening in the wrong language.
                    if done {
                        self.unknown = if words::known(text) { 0 } else { self.unknown + 1 };
                        if self.unknown >= UNKNOWN_TURN {
                            self.unknown = 0;
                            if let Some(sp) = &self.speech {
                                sp.turn();
                            }
                        }
                    }
                }
                Note::Refused | Note::Gone => {
                    self.speech_gone = true;
                    let away = self.game.senses.words == senses::Leave::Open;
                    self.close(Kind::Words);
                    if away && matches!(note, Note::Refused) {
                        self.game.senses.refused(Kind::Words, true);
                    }
                }
            }
        }
        self.game.words.open = self.speech.is_some();
    }
}
