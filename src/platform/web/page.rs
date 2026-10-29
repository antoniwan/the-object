//! Small pieces of the page: the body, failing quietly, the answer sent on, storage, event
//! listeners, and what the page shows of it around the canvas (the aura, the marks).

use super::App;
use crate::aura::aura;
use crate::input::senses::{self, Kind};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AddEventListenerOptions, HtmlElement, Storage};

/// The marks' own words, read aloud and shown on hover (they have no letters of their own).
const STOP: &str = "tap here or press escape to stop.";

pub(super) fn body() -> Option<HtmlElement> {
    web_sys::window()?.document()?.body()
}

/// When it can't appear: the loader says so, quietly, and the reason goes to the console.
pub(super) fn fail(why: &JsValue) {
    web_sys::console::error_1(why);
    if let Some(b) = body() {
        let _ = b.class_list().add_1("failed");
    }
}

/// The answer to its question goes to the page (`the-object:answer` on window, detail 1..=5),
/// which sends it on; see web/index.html.
pub(super) fn send_answer(n: u8) {
    let Some(w) = web_sys::window() else { return };
    let init = web_sys::CustomEventInit::new();
    init.set_detail(&JsValue::from(n));
    if let Ok(e) = web_sys::CustomEvent::new_with_event_init_dict("the-object:answer", &init) {
        let _ = w.dispatch_event(&e);
    }
}

/// Browser storage can be missing or throw (private windows); every use tolerates that.
pub(super) fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub(super) fn listen<E: JsCast + 'static>(
    target: &web_sys::EventTarget,
    name: &str,
    passive: bool,
    f: impl FnMut(E) + 'static,
) -> Result<(), JsValue> {
    let mut f = f;
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into()));
    let opts = AddEventListenerOptions::new();
    opts.set_passive(passive);
    target.add_event_listener_with_callback_and_add_event_listener_options(name, cb.as_ref().unchecked_ref(), &opts)?;
    cb.forget();
    Ok(())
}

impl App {
    /// Lets the words on the page take the object's color, unrest and place (see web/glyph.js).
    pub(super) fn show_aura(&mut self) {
        let a = aura(&self.game.world, &self.game.cam, self.game.w, self.game.h);
        let n = |v: f32| (v * 100.0).round() as i32;
        let now = [n(a.tint.x), n(a.tint.y), n(a.tint.z), n(a.unrest), n(a.x), n(a.y)];
        let was = std::mem::replace(&mut self.aura_shown, now);
        if now == was {
            return;
        }
        let Some(style) = &self.root_style else { return };
        let q = |i: usize| now[i] as f32 / 100.0;
        if now[..3] != was[..3] {
            let rgb = |i: usize| (q(i) * 255.0).round();
            let _ = style.set_property("--tint", &format!("{} {} {}", rgb(0), rgb(1), rgb(2)));
        }
        if now[3] != was[3] {
            let _ = style.set_property("--unrest", &q(3).to_string());
        }
        if now[4] != was[4] {
            let _ = style.set_property("--ox", &format!("{}%", q(4) * 100.0));
        }
        if now[5] != was[5] {
            let _ = style.set_property("--oy", &format!("{}%", q(5) * 100.0));
        }
    }

    /// One quiet mark for each thing it is let have, in the corner opposite the count, and a
    /// fainter one for each it asked for this visit and was not given. Before it hears, the
    /// microphone waits in that corner instead, and fades as the mark for hearing comes.
    pub(super) fn show_marks(&mut self) {
        let (ear, eye) = (self.ears.is_some(), self.eyes.is_some());
        let senses = &self.game.senses;
        let mic = senses.waiting(Kind::Hear) && !ear;
        if mic != self.hud.mic_shown {
            self.hud.mic_shown = mic;
            let _ = self.hud.mic.class_list().toggle_with_force("on", mic);
            let _ = self.hud.mic.set_attribute("tabindex", if mic { "0" } else { "-1" });
            let _ = self.hud.mic.set_attribute("aria-hidden", if mic { "false" } else { "true" });
            let _ = self.hud.marks.class_list().toggle_with_force("beside", mic);
        }
        let wait_eye = senses.waiting(Kind::See) && !eye;
        let mark = |on: bool, wait: bool, k: &str, g: &str| match (on, wait) {
            (true, _) => format!("<span>{g}</span>"),
            (false, true) => format!("<span class=\"w\" data-k=\"{k}\">{g}</span>"),
            _ => String::new(),
        };
        let html = [mark(ear, false, "hear", "◍"), mark(eye, wait_eye, "see", "◉")]
            .into_iter()
            .filter(|m| !m.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if html == self.hud.marks_shown {
            return;
        }
        let what = match (ear, eye) {
            (true, true) => format!("it is listening and watching. {STOP}"),
            (true, false) => format!("it is listening. {STOP}"),
            (false, true) => format!("it is watching. {STOP}"),
            _ => String::new(),
        };
        let what = if senses.words == senses::Leave::Open {
            format!("{what} what you say goes to this browser's maker to become words.")
        } else {
            what
        };
        let asked = if wait_eye { " it asked to see you. tap the faint mark to let it." } else { "" };
        let what = format!("{what}{asked}");
        let what = what.trim();
        let marks = &self.hud.marks;
        marks.set_inner_html(&html);
        let _ = marks.set_attribute("aria-label", what);
        let _ = marks.set_attribute("title", what);
        let _ = marks.class_list().toggle_with_force("on", ear || eye);
        let _ = marks.class_list().toggle_with_force("wait", wait_eye);
        self.hud.marks_shown = html;
    }
}
