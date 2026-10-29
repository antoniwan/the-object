//! The lines of text over it: the hint, the toast, the count, a thought now and then, the marks
//! while it hears or sees, and the same words read aloud.

use crate::aura::roman;
use crate::input::senses::{self, Kind};
use crate::mind::contact::{Contact, KEEP_STEPS};
use crate::progress::Discovery;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

const TOAST_SECONDS: f64 = 4.2;
/// What it says under that word, so the tap is an informed one.
const AWAY: &str = "to make words of it, this browser sends what you say to its maker. tap to allow.";
/// The question stays up while the drop is held out.
const ASK_SECONDS: f64 = 7.0;
/// Its reply stays up longer than anything else it says.
const REPLY_SECONDS: f64 = 10.0;
/// A thought surfaces, stays this long, then sinks (thoughts.rs).
const THOUGHT_SECONDS: f64 = 7.0;

/// Something for the toast to say.
pub(super) enum Line {
    Found(Discovery),
    /// It is holding out a drop after contact (contact.rs, `Ask`).
    Asked,
    /// The drop was let go after this many steps, of KEEP_STEPS.
    Answer(u8),
    /// It asks to hear or to see; the word can be tapped while it is up.
    Offer(Kind),
    /// The browser was told no.
    Refused(Kind),
    /// It answered a greeting, in signs (words.rs).
    Greeted(String),
}

pub(super) struct Hud {
    pub(super) hint: HtmlElement,
    pub(super) toast: HtmlElement,
    pub(super) count: HtmlElement,
    pub(super) thought: HtmlElement,
    /// The mark that shows it is hearing or seeing; tapped, it stops. A faint mark is one it
    /// asked for and was not given; tapped, the browser asks.
    pub(super) marks: HtmlElement,
    /// What the marks last showed, so the page is only touched when that changes.
    pub(super) marks_shown: String,
    /// Plain words for whoever cannot see the page, read out as they appear (`#said`).
    pub(super) said: HtmlElement,
    pub(super) queue: Vec<Line>,
    pub(super) toast_until: f64,
    pub(super) thought_until: f64,
    /// Whether the toast and the thought carry `show`, so the class is only touched when that changes.
    pub(super) toast_shown: bool,
    pub(super) thought_shown: bool,
}

impl Hud {
    pub(super) fn new(doc: &Document) -> Result<Hud, JsValue> {
        let get = |id: &str| -> Result<HtmlElement, JsValue> {
            Ok(doc.get_element_by_id(id).ok_or(format!("#{id} missing"))?.dyn_into()?)
        };
        Ok(Hud {
            hint: get("hint")?,
            toast: get("toast")?,
            count: get("count")?,
            thought: get("thought")?,
            marks: get("senses")?,
            marks_shown: String::new(),
            said: get("said")?,
            queue: Vec::new(),
            toast_until: 0.0,
            thought_until: 0.0,
            toast_shown: false,
            thought_shown: false,
        })
    }

    /// A thought surfaces under it. Anything found, or anything it says, sends the thought away.
    pub(super) fn think(&mut self, now: f64, line: Option<&str>) {
        if let Some(line) = line {
            self.thought.set_text_content(Some(line));
            self.say(line);
            let _ = self.thought.class_list().add_1("show");
            self.thought_shown = true;
            self.thought_until = now + THOUGHT_SECONDS;
        } else if now >= self.thought_until || !self.queue.is_empty() {
            self.thought_until = 0.0;
            if self.thought_shown {
                self.thought_shown = false;
                let _ = self.thought.class_list().remove_1("show");
            }
        }
    }

    /// Whatever the toast was saying is cut short, and this goes first.
    pub(super) fn urgent(&mut self, line: Line) {
        self.toast_until = 0.0;
        self.queue.insert(0, line);
    }

    /// Read out, never shown.
    pub(super) fn say(&self, words: &str) {
        self.said.set_text_content(Some(words));
    }

    pub(super) fn show_count(&self, found: usize) {
        if found > 0 {
            self.count.set_text_content(Some(&roman(found)));
        }
    }

    /// `contact` knows what it says at contact, which is different for everyone.
    pub(super) fn update(&mut self, now: f64, found: usize, contact: &Contact) {
        if now < self.toast_until {
            return;
        }
        if self.toast_shown {
            self.toast_shown = false;
            let _ = self.toast.class_list().remove_1("show");
        }
        if self.queue.is_empty() {
            return;
        }
        let alien = " class=\"alien\"";
        let offer = " class=\"offer\"";
        let (name, line, class, linger) = match self.queue.remove(0) {
            Line::Found(d @ Discovery::Contact) => (d.name(), contact.reply(), alien, REPLY_SECONDS),
            Line::Found(d) => (d.name(), d.line().to_string(), "", TOAST_SECONDS),
            Line::Asked => ("a question", "keep it for as long as this was worth".to_string(), "", ASK_SECONDS),
            Line::Answer(n) => {
                let kept = "⟡".repeat(n as usize) + &"∴".repeat((KEEP_STEPS - n.min(KEEP_STEPS)) as usize);
                (if n > 1 { "kept" } else { "let go" }, kept, alien, TOAST_SECONDS)
            }
            Line::Offer(Kind::Words) => {
                let line = format!("{}</span><small class=\"away\">{}</small><span>", Kind::Words.word(), AWAY);
                ("it asks", line, offer, senses::OFFER_WINDOW as f64)
            }
            Line::Offer(k) => ("it asks", k.word().to_string(), offer, senses::OFFER_WINDOW as f64),
            Line::Greeted(signs) => ("", signs, alien, TOAST_SECONDS),
            Line::Refused(k) => ("no", k.refused().to_string(), "", TOAST_SECONDS),
        };
        self.toast.set_inner_html(&format!("<b>{}</b><span{}>{}</span>", name, class, line));
        // Its signs are not words; only the letters are read out.
        // Whoever hears this instead of seeing it is told the one key that takes its word.
        if class != alien {
            let plain = self.toast.text_content().unwrap_or_default();
            let take = if class == offer { " press enter to allow." } else { "" };
            self.say(&format!("{name}. {}{take}", plain.trim_start_matches(name)));
        }
        let _ = self.toast.class_list().add_1("show");
        self.toast_shown = true;
        self.toast_until = now + linger;
        self.show_count(found);
        let _ = self.hint.class_list().add_1("gone");
    }
}
