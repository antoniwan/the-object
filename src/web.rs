//! Browser glue: canvas, input events, the frame loop, and the small HUD.

use crate::audio::Audio;
use crate::game::Game;
use crate::progress::{Discovery, Progress};
use crate::render::Renderer;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    AddEventListenerOptions, Document, HtmlCanvasElement, HtmlElement, KeyboardEvent, PointerEvent,
    Storage, WebGl2RenderingContext, WheelEvent,
};

const TOAST_SECONDS: f64 = 2.6;
const KEY_FOUND: &str = "the-object.found";
const KEY_VISITS: &str = "the-object.visits";

/// Browser storage can be missing or throw (private windows); every use tolerates that.
fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

struct Hud {
    hint: HtmlElement,
    toast: HtmlElement,
    count: HtmlElement,
    queue: Vec<Discovery>,
    toast_until: f64,
}

impl Hud {
    fn new(doc: &Document) -> Result<Hud, JsValue> {
        let get = |id: &str| -> Result<HtmlElement, JsValue> {
            Ok(doc.get_element_by_id(id).ok_or(format!("#{id} missing"))?.dyn_into()?)
        };
        Ok(Hud { hint: get("hint")?, toast: get("toast")?, count: get("count")?, queue: Vec::new(), toast_until: 0.0 })
    }

    fn show_count(&self, found: usize) {
        if found > 0 {
            self.count.set_text_content(Some(&format!("{found} discovered")));
        }
    }

    fn update(&mut self, now: f64, found: usize) {
        if now < self.toast_until {
            return;
        }
        let _ = self.toast.class_list().remove_1("show");
        if self.queue.is_empty() {
            return;
        }
        let d = self.queue.remove(0);
        self.toast.set_inner_html(&format!("<b>{}</b><span>{}</span>", d.name(), d.line()));
        let _ = self.toast.class_list().add_1("show");
        self.toast_until = now + TOAST_SECONDS;
        self.show_count(found);
        let _ = self.hint.class_list().add_1("gone");
    }
}

struct App {
    game: Game,
    renderer: Renderer,
    canvas: HtmlCanvasElement,
    hud: Hud,
    audio: Option<Audio>,
    night_shown: bool,
    last_frame: Option<f64>,
    last_move: f64,
    /// Fraction of device pixels rendered; lowered automatically on slow devices.
    quality: f64,
    slow_time: f64,
}

impl App {
    /// Pointer position in canvas pixels.
    fn pos(&self, e: &PointerEvent) -> (f32, f32) {
        let sx = self.canvas.width() as f64 / self.canvas.client_width().max(1) as f64;
        let sy = self.canvas.height() as f64 / self.canvas.client_height().max(1) as f64;
        ((e.offset_x() as f64 * sx) as f32, (e.offset_y() as f64 * sy) as f32)
    }

    fn resize(&mut self) {
        let dpr = web_sys::window().unwrap().device_pixel_ratio().min(1.5) * self.quality;
        let w = (self.canvas.client_width() as f64 * dpr).max(1.0) as u32;
        let h = (self.canvas.client_height() as f64 * dpr).max(1.0) as u32;
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        self.game.w = w as f32;
        self.game.h = h as f32;
    }

    fn frame(&mut self, now: f64) {
        let dt = self.last_frame.map_or(1.0 / 60.0, |t| ((now - t) / 1000.0).clamp(0.0, 1.0 / 20.0));
        self.last_frame = Some(now);

        // Adaptive resolution: drop quality if we sit under ~35 fps for a second.
        if dt > 1.0 / 35.0 {
            self.slow_time += dt;
            if self.slow_time > 1.0 && self.quality > 0.5 {
                self.quality -= 0.15;
                self.slow_time = 0.0;
            }
        } else {
            self.slow_time = (self.slow_time - dt).max(0.0);
        }

        self.resize();
        self.game.update(dt as f32);
        if !self.game.announcements.is_empty() {
            if let Some(s) = storage() {
                let _ = s.set_item(KEY_FOUND, &self.game.progress.save());
            }
        }
        self.hud.queue.append(&mut self.game.announcements);
        let sounds = std::mem::take(&mut self.game.sounds);
        if let Some(a) = &self.audio {
            for s in sounds {
                a.play(s);
            }
            a.follow(&self.game.world);
        }
        let night = self.game.world.night > 0.5;
        if night != self.night_shown {
            self.night_shown = night;
            if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
                let _ = body.class_list().toggle_with_force("night", night);
            }
        }
        self.hud.update(now / 1000.0, self.game.progress.count());
        self.renderer.draw(&self.game.world, &self.game.cam, self.canvas.width(), self.canvas.height());
    }
}

fn listen<E: JsCast + 'static>(
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

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let doc = window.document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = doc.get_element_by_id("stage").ok_or("#stage missing")?.dyn_into()?;
    let gl: WebGl2RenderingContext = canvas
        .get_context("webgl2")?
        .ok_or("This browser doesn't support WebGL2.")?
        .dyn_into()?;
    let renderer = Renderer::new(gl)?;

    // It keeps count, across visits.
    let mut game = Game::new();
    let hud = Hud::new(&doc)?;
    if let Some(s) = storage() {
        let found = s.get_item(KEY_FOUND).ok().flatten().unwrap_or_default();
        let visits = s.get_item(KEY_VISITS).ok().flatten().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) + 1;
        let _ = s.set_item(KEY_VISITS, &visits.to_string());
        game.progress = Progress::load(&found);
        hud.show_count(game.progress.count());
        if visits > 1 && game.progress.count() > 0 {
            game.discover(Discovery::Return);
            game.sounds.clear();
        }
    }

    let app = Rc::new(RefCell::new(App {
        game,
        renderer,
        canvas: canvas.clone(),
        hud,
        audio: None,
        night_shown: false,
        last_frame: None,
        last_move: 0.0,
        quality: 1.0,
        slow_time: 0.0,
    }));
    app.borrow_mut().resize();

    let target: &web_sys::EventTarget = canvas.as_ref();
    {
        let app = app.clone();
        let c = canvas.clone();
        listen(target, "pointerdown", false, move |e: PointerEvent| {
            e.prevent_default();
            let _ = c.set_pointer_capture(e.pointer_id());
            let mut a = app.borrow_mut();
            if a.audio.is_none() {
                a.audio = Audio::new().ok();
            }
            let (x, y) = a.pos(&e);
            a.last_move = e.time_stamp();
            a.game.pointer_down(e.pointer_id(), x, y);
        })?;
    }
    {
        let app = app.clone();
        let c = canvas.clone();
        listen(target, "pointermove", true, move |e: PointerEvent| {
            let mut a = app.borrow_mut();
            let (x, y) = a.pos(&e);
            let dt = ((e.time_stamp() - a.last_move) / 1000.0).max(0.001) as f32;
            a.last_move = e.time_stamp();
            a.game.pointer_move(e.pointer_id(), x, y, dt);
            let cursor = if a.game.hovering_liquid(x, y) { "grab" } else { "default" };
            let _ = c.style().set_property("cursor", cursor);
        })?;
    }
    for name in ["pointerup", "pointercancel"] {
        let app = app.clone();
        listen(target, name, true, move |e: PointerEvent| {
            app.borrow_mut().game.pointer_up(e.pointer_id());
        })?;
    }
    {
        let app = app.clone();
        listen(target, "wheel", false, move |e: WheelEvent| {
            e.prevent_default();
            app.borrow_mut().game.wheel(e.delta_y() as f32);
        })?;
    }
    listen(target, "contextmenu", false, |e: web_sys::Event| e.prevent_default())?;
    {
        let app = app.clone();
        listen(window.as_ref(), "keydown", true, move |e: KeyboardEvent| {
            if e.key() == "m" {
                if let Some(a) = &mut app.borrow_mut().audio {
                    a.toggle_mute();
                }
            }
        })?;
    }

    // requestAnimationFrame loop.
    let tick: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let tick2 = tick.clone();
    *tick2.borrow_mut() = Some(Closure::new(move |now: f64| {
        app.borrow_mut().frame(now);
        let w = web_sys::window().unwrap();
        let _ = w.request_animation_frame(tick.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    window.request_animation_frame(tick2.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
