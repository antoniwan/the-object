//! Keeping the screen on (the Screen Wake Lock), as `wake.rs` decides. Where the browser has
//! no such thing, or will not give it, nothing happens and nothing is said: the screen goes dark
//! as it always did.

use crate::platform::get;
use crate::wake::Do;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Default)]
pub struct Keep {
    /// What the browser handed back, while it holds the screen on.
    sentinel: Rc<RefCell<Option<JsValue>>>,
    /// Whether it is wanted now: an answer that comes after it was let go is let go at once.
    want: Rc<Cell<bool>>,
    /// Asked, not yet answered.
    asking: Rc<Cell<bool>>,
    /// The browser would not, or let go by itself while the page was seen.
    lost: Rc<Cell<bool>>,
}

impl Keep {
    pub fn apply(&self, what: Do) {
        match what {
            Do::Take => self.take(),
            Do::Let => self.let_go(),
            Do::Nothing => {}
        }
    }

    /// True once after the browser would not keep it, or stopped keeping it on its own.
    pub fn lost(&self) -> bool {
        self.lost.replace(false)
    }

    fn take(&self) {
        self.want.set(true);
        if self.asking.get() || self.sentinel.borrow().is_some() {
            return;
        }
        let Some(w) = web_sys::window() else { return };
        let lock = get(&w.navigator(), "wakeLock");
        let Ok(request) = get(&lock, "request").dyn_into::<js_sys::Function>() else { return };
        let Ok(promise) = request.call1(&lock, &JsValue::from_str("screen")) else { return };
        self.asking.set(true);

        let (sentinel, want, asking, lost) =
            (self.sentinel.clone(), self.want.clone(), self.asking.clone(), self.lost.clone());
        let ok = Closure::once_into_js(move |got: JsValue| {
            asking.set(false);
            if !want.get() {
                release(&got);
                return;
            }
            // The browser lets go by itself when the page is hidden (taken again on the way
            // back) and sometimes when it is not (a low battery): then the next touch asks again.
            let (held, lost, me) = (sentinel.clone(), lost.clone(), got.clone());
            let on_release = Closure::once_into_js(move |_: JsValue| {
                let ours = held.borrow().as_ref() == Some(&me);
                if ours {
                    held.borrow_mut().take();
                    let hidden = web_sys::window().and_then(|w| w.document()).is_some_and(|d| d.hidden());
                    if !hidden {
                        lost.set(true);
                    }
                }
            });
            if let Some(target) = got.dyn_ref::<web_sys::EventTarget>() {
                let _ = target.add_event_listener_with_callback("release", on_release.unchecked_ref());
            }
            *sentinel.borrow_mut() = Some(got);
        });
        let (asking, lost) = (self.asking.clone(), self.lost.clone());
        let err = Closure::once_into_js(move |_: JsValue| {
            asking.set(false);
            lost.set(true);
        });
        then(&promise, &ok, &err);
    }

    fn let_go(&self) {
        self.want.set(false);
        // Taken out first, so its release is known to be ours.
        let held = self.sentinel.borrow_mut().take();
        if let Some(s) = held {
            release(&s);
        }
    }
}

fn release(sentinel: &JsValue) {
    if let Ok(f) = get(sentinel, "release").dyn_into::<js_sys::Function>() {
        // A promise; a refusal of it is nothing to us.
        if let Ok(p) = f.call0(sentinel) {
            then(&p, &JsValue::UNDEFINED, &Closure::once_into_js(|_: JsValue| {}));
        }
    }
}

/// `promise.then(ok, err)`, on whatever came back; not a promise, nothing is waited for.
fn then(promise: &JsValue, ok: &JsValue, err: &JsValue) {
    if let Ok(f) = get(promise, "then").dyn_into::<js_sys::Function>() {
        let _ = f.call2(promise, ok, err);
    }
}
