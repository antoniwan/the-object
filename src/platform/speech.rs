//! Words out of a voice, for words.rs, by the browser's own speech recognition. Where the browser
//! can do it on the device (`processLocally`), nothing leaves the page and hearing is all it
//! takes. Where it cannot, recognition sends the voice to the browser's maker and back, so it is
//! only started inside the player's tap on its own word (`say something.`), which says so.
//!
//! What comes back is text, handed to words.rs piece by piece and dropped. No audio is touched
//! here at all; the browser listens on its own.
//!
//! A recognizer listens in one language at a time. It starts in the player's own and keeps a
//! second, English or Spanish, whichever the first is not (`tongues`). When it keeps hearing
//! nothing it knows, the page has it listen in the other (`turn`), and it stays in whichever
//! one its words were last found in.

use super::get;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// What this browser can do with a voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Can {
    /// No speech recognition at all.
    Nothing,
    /// Perhaps on the device: only asked once the player has let it hear (`ready`).
    Perhaps,
    /// On the device.
    Here,
    /// Only by sending the voice away.
    Away,
}

/// What the recognizer said, for the next frame.
pub enum Note {
    /// The `index`th piece of text of start `session`, as it stands now: what the browser
    /// thinks was said, best guess first, and whether it is done changing.
    Text { session: u32, index: u32, guesses: Vec<String>, done: bool },
    /// The player (or the page's setting) said no.
    Refused,
    /// It stopped and will not start again.
    Gone,
}

fn set(target: &JsValue, key: &str, v: JsValue) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), &v);
}

fn recognizer() -> Option<js_sys::Function> {
    let w: JsValue = web_sys::window()?.into();
    ["SpeechRecognition", "webkitSpeechRecognition"]
        .iter()
        .find_map(|k| get(&w, k).dyn_into::<js_sys::Function>().ok())
}

/// The languages it listens in: the player's own, as the browser has it, and then English or
/// Spanish, whichever that is not (the words it takes as a hand are in both). A second one the
/// player keeps in the browser is preferred, so a Puerto Rican phone gets `es-PR`, not `es-US`.
pub fn tongues() -> Vec<String> {
    let nav = web_sys::window().map(|w| w.navigator());
    let first = nav.as_ref().and_then(|n| n.language()).unwrap_or_else(|| "en-US".into());
    let kept: Vec<String> = nav
        .map(|n| n.languages().iter().filter_map(|l| l.as_string()).collect())
        .unwrap_or_default();
    let family = |l: &str| l.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    let other = if family(&first) == "en" { "es" } else { "en" };
    let fallback = if other == "es" { "es-US" } else { "en-US" };
    let second = kept.into_iter().find(|l| family(l) == other).unwrap_or_else(|| fallback.into());
    vec![first, second]
}

fn local_options(lang: &str) -> JsValue {
    let o = js_sys::Object::new();
    let langs = js_sys::Array::of1(&JsValue::from_str(lang));
    set(&o, "langs", langs.into());
    set(&o, "processLocally", true.into());
    o.into()
}

/// Whether this browser could make words on the device. Nothing is asked of the browser yet:
/// in a Chromium without its on-device speech (most builds that are not Google Chrome), even
/// asking takes the whole page down, so only Google Chrome is asked, and only inside the tap
/// that lets it hear (`ready`).
pub fn can() -> Can {
    let Some(r) = recognizer() else { return Can::Nothing };
    let Some(w) = web_sys::window() else { return Can::Nothing };
    // Safari has a recognizer but no `userAgentData`, and `Array.from(undefined)` throws there,
    // which took the whole page down on load: only a real array is read.
    let brands = get(&get(&w.navigator(), "userAgentData"), "brands");
    let chrome = brands
        .dyn_ref::<js_sys::Array>()
        .is_some_and(|a| a.iter().any(|b| get(&b, "brand").as_string().as_deref() == Some("Google Chrome")));
    if chrome && get(&r, "install").is_function() {
        Can::Perhaps
    } else {
        Can::Away
    }
}

/// Asks for words on the device, fetching the language if it must. Call inside the player's tap.
/// It lands on `Here`, or on `Away` when this language cannot be had here.
/// The second language is fetched too, on its own: if it cannot be had, the first still can.
pub fn ready(can: Rc<Cell<Can>>, tongues: &[String]) {
    let away = || can.set(Can::Away);
    let Some(r) = recognizer() else { return away() };
    let Ok(install) = get(&r, "install").dyn_into::<js_sys::Function>() else { return away() };
    let Some(lang) = tongues.first() else { return away() };
    for other in &tongues[1..] {
        let _ = install.call1(&r, &local_options(other));
    }
    let Ok(promise) = install.call1(&r, &local_options(lang)).and_then(|p| p.dyn_into::<js_sys::Promise>()) else {
        return away();
    };
    let (yes, no) = (can.clone(), can);
    let ok = Closure::once(move |done: JsValue| yes.set(if done.is_truthy() { Can::Here } else { Can::Away }));
    let err = Closure::once(move |_: JsValue| no.set(Can::Away));
    let _ = promise.then2(&ok, &err);
    ok.forget();
    err.forget();
}

/// The recognizer, while it runs. It stops after silence on most browsers; it is started again
/// until it is closed or keeps failing.
pub struct Speech {
    rec: JsValue,
    want: Rc<Cell<bool>>,
    /// The languages it may listen in (which one it is in now, only its handlers know).
    tongues: Rc<RefCell<Vec<String>>>,
    /// Set to listen in the next language from the next start on.
    turn: Rc<Cell<bool>>,
    _handlers: Vec<Closure<dyn FnMut(JsValue)>>,
}

/// How many guesses at what was said it asks for; the one with the most words in it is taken.
const GUESSES: u32 = 4;

/// Starts failing in a row, with nothing heard, before it gives up.
const TRIES: u32 = 6;

impl Speech {
    /// `here`: on the device only. Away, call this inside the player's tap.
    pub fn start(here: bool, tongues: &[String], notes: Rc<RefCell<Vec<Note>>>) -> Option<Speech> {
        let r = recognizer()?;
        let rec: JsValue = js_sys::Reflect::construct(&r, &js_sys::Array::new()).ok()?;
        set(&rec, "lang", JsValue::from_str(tongues.first()?));
        set(&rec, "continuous", true.into());
        set(&rec, "interimResults", true.into());
        set(&rec, "maxAlternatives", GUESSES.into());
        let tongues = Rc::new(RefCell::new(tongues.to_vec()));
        let tongue = Rc::new(Cell::new(0usize));
        let turn = Rc::new(Cell::new(false));
        if here {
            set(&rec, "processLocally", true.into());
        }
        let want = Rc::new(Cell::new(true));
        let session = Rc::new(Cell::new(0u32));
        let failed = Rc::new(Cell::new(0u32));

        let (n, s, f) = (notes.clone(), session.clone(), failed.clone());
        let on_result = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            let results = get(&e, "results");
            let from = get(&e, "resultIndex").as_f64().unwrap_or(0.0) as u32;
            let len = get(&results, "length").as_f64().unwrap_or(0.0) as u32;
            for i in from..len {
                let result = get(&results, &i.to_string());
                let count = get(&result, "length").as_f64().unwrap_or(0.0) as u32;
                let guesses: Vec<String> =
                    (0..count).filter_map(|k| get(&get(&result, &k.to_string()), "transcript").as_string()).collect();
                if !guesses.is_empty() {
                    f.set(0);
                    let done = get(&result, "isFinal").is_truthy();
                    n.borrow_mut().push(Note::Text { session: s.get(), index: i, guesses, done });
                }
            }
        });
        let (n, w, f) = (notes.clone(), want.clone(), failed.clone());
        let (ts, t) = (tongues.clone(), tongue.clone());
        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            let error = get(&e, "error").as_string().unwrap_or_default();
            match error.as_str() {
                "not-allowed" | "service-not-allowed" => {
                    w.set(false);
                    n.borrow_mut().push(Note::Refused);
                }
                // One language it cannot have here: it keeps listening in the others.
                "language-not-supported" if ts.borrow().len() > 1 => {
                    let mut ts = ts.borrow_mut();
                    let at = t.get().min(ts.len() - 1);
                    ts.remove(at);
                    t.set(0);
                }
                "language-not-supported" | "audio-capture" => {
                    w.set(false);
                    n.borrow_mut().push(Note::Gone);
                }
                // Silence, a lost connection: it is started again when it ends.
                _ => f.set(f.get() + 1),
            }
        });
        let (n, w, again) = (notes, want.clone(), rec.clone());
        let (ts, t, tn) = (tongues.clone(), tongue.clone(), turn.clone());
        let on_end = Closure::<dyn FnMut(JsValue)>::new(move |_: JsValue| {
            if !w.get() {
                return;
            }
            if failed.get() >= TRIES {
                w.set(false);
                n.borrow_mut().push(Note::Gone);
                return;
            }
            let ts = ts.borrow();
            if tn.replace(false) {
                t.set((t.get() + 1) % ts.len().max(1));
            }
            if let Some(lang) = ts.get(t.get()) {
                set(&again, "lang", JsValue::from_str(lang));
            }
            session.set(session.get() + 1);
            if let Ok(start) = get(&again, "start").dyn_into::<js_sys::Function>() {
                if start.call0(&again).is_err() {
                    failed.set(failed.get() + 1);
                }
            }
        });
        set(&rec, "onresult", on_result.as_ref().clone());
        set(&rec, "onerror", on_error.as_ref().clone());
        set(&rec, "onend", on_end.as_ref().clone());
        let start = get(&rec, "start").dyn_into::<js_sys::Function>().ok()?;
        start.call0(&rec).ok()?;
        Some(Speech { rec, want, tongues, turn, _handlers: vec![on_result, on_error, on_end] })
    }

    /// Whether there is another language to listen in.
    fn bilingual(&self) -> bool {
        self.tongues.borrow().len() > 1
    }

    /// Listens in the next language: what was being said is finished first, then it starts
    /// again in the other one.
    pub fn turn(&self) {
        if !self.bilingual() || self.turn.replace(true) {
            return;
        }
        if let Ok(stop) = get(&self.rec, "stop").dyn_into::<js_sys::Function>() {
            let _ = stop.call0(&self.rec);
        }
    }

    pub fn stop(self) {
        self.want.set(false);
        // Let go of the handlers first: the browser still says it ended, after they are gone.
        for k in ["onresult", "onerror", "onend"] {
            set(&self.rec, k, JsValue::NULL);
        }
        if let Ok(abort) = get(&self.rec, "abort").dyn_into::<js_sys::Function>() {
            let _ = abort.call0(&self.rec);
        }
    }
}
