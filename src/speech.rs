//! Words out of a voice, for words.rs, by the browser's own speech recognition. Where the browser
//! can do it on the device (`processLocally`), nothing leaves the page and hearing is all it
//! takes. Where it cannot, recognition sends the voice to the browser's maker and back, so it is
//! only started inside the player's tap on its own word (`say something.`), which says so.
//!
//! What comes back is text, handed to words.rs piece by piece and dropped. No audio is touched
//! here at all; the browser listens on its own.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// What this browser can do with a voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Can {
    /// No speech recognition at all, or not asked yet.
    Nothing,
    /// On the device, now.
    Here,
    /// On the device, once its language is fetched; that must be asked for inside a tap.
    HereAfterFetch,
    /// Only by sending the voice away.
    Away,
}

/// What the recognizer said, for the next frame.
pub enum Note {
    /// The `index`th piece of text of start `session`, as it stands now.
    Text { session: u32, index: u32, text: String },
    /// The player (or the page's setting) said no.
    Refused,
    /// It stopped and will not start again.
    Gone,
}

fn get(target: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
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

/// The player's language, as the browser has it.
pub fn lang() -> String {
    web_sys::window().and_then(|w| w.navigator().language()).unwrap_or_else(|| "en-US".into())
}

fn local_options(lang: &str) -> JsValue {
    let o = js_sys::Object::new();
    let langs = js_sys::Array::of1(&JsValue::from_str(lang));
    set(&o, "langs", langs.into());
    set(&o, "processLocally", true.into());
    o.into()
}

/// Finds out, in the background, what this browser can do. Asks nothing of the player.
pub fn probe(lang: &str) -> Rc<Cell<Can>> {
    let can = Rc::new(Cell::new(Can::Nothing));
    let Some(r) = recognizer() else { return can };
    can.set(Can::Away);
    let Ok(available) = get(&r, "available").dyn_into::<js_sys::Function>() else { return can };
    let Ok(promise) = available.call1(&r, &local_options(lang)).and_then(|p| p.dyn_into::<js_sys::Promise>()) else {
        return can;
    };
    let answer = can.clone();
    let ok = Closure::once(move |s: JsValue| {
        answer.set(match s.as_string().as_deref() {
            Some("available") => Can::Here,
            Some("downloadable") | Some("downloading") => Can::HereAfterFetch,
            _ => Can::Away,
        })
    });
    let _ = promise.then(&ok);
    ok.forget();
    can
}

/// Fetches the language so words can be made on the device. Must be called inside a tap.
pub fn fetch(can: Rc<Cell<Can>>, lang: &str) {
    let Some(r) = recognizer() else { return };
    let Ok(install) = get(&r, "install").dyn_into::<js_sys::Function>() else { return };
    let Ok(promise) = install.call1(&r, &local_options(lang)).and_then(|p| p.dyn_into::<js_sys::Promise>()) else {
        return;
    };
    let ok = Closure::once(move |done: JsValue| {
        if done.is_truthy() {
            can.set(Can::Here);
        }
    });
    let _ = promise.then(&ok);
    ok.forget();
}

/// The recognizer, while it runs. It stops after silence on most browsers; it is started again
/// until it is closed or keeps failing.
pub struct Speech {
    rec: JsValue,
    want: Rc<Cell<bool>>,
    _handlers: Vec<Closure<dyn FnMut(JsValue)>>,
}

/// Starts failing in a row, with nothing heard, before it gives up.
const TRIES: u32 = 6;

impl Speech {
    /// `here`: on the device only. Away, call this inside the player's tap.
    pub fn start(here: bool, lang: &str, notes: Rc<RefCell<Vec<Note>>>) -> Option<Speech> {
        let r = recognizer()?;
        let rec: JsValue = js_sys::Reflect::construct(&r, &js_sys::Array::new()).ok()?;
        set(&rec, "lang", JsValue::from_str(lang));
        set(&rec, "continuous", true.into());
        set(&rec, "interimResults", true.into());
        set(&rec, "maxAlternatives", 1.into());
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
                let best = get(&get(&results, &i.to_string()), "0");
                if let Some(text) = get(&best, "transcript").as_string() {
                    f.set(0);
                    n.borrow_mut().push(Note::Text { session: s.get(), index: i, text });
                }
            }
        });
        let (n, w, f) = (notes.clone(), want.clone(), failed.clone());
        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            let error = get(&e, "error").as_string().unwrap_or_default();
            match error.as_str() {
                "not-allowed" | "service-not-allowed" => {
                    w.set(false);
                    n.borrow_mut().push(Note::Refused);
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
        let on_end = Closure::<dyn FnMut(JsValue)>::new(move |_: JsValue| {
            if !w.get() {
                return;
            }
            if failed.get() >= TRIES {
                w.set(false);
                n.borrow_mut().push(Note::Gone);
                return;
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
        Some(Speech { rec, want, _handlers: vec![on_result, on_error, on_end] })
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
