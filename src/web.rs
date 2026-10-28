//! Browser glue: canvas, input events, the frame loop, the three lines of text
//! (hint, toast, count), and what is saved between visits.

use crate::audio::Audio;
use crate::aura::{aura, roman};
use crate::contact::Contact;
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

const TOAST_SECONDS: f64 = 4.2;
const KEY_FOUND: &str = "the-object.found";
const KEY_VISITS: &str = "the-object.visits";
/// What it has learned about whoever keeps visiting (see contact.rs).
const KEY_CONTACT: &str = "the-object.contact";
/// Its reply stays up longer than anything else it says.
const REPLY_SECONDS: f64 = 10.0;
/// A big screen at full density is millions of raymarched pixels; past this it renders smaller and scales up.
const MAX_PIXELS: f64 = 1.6e6;
/// Frames the loader gets to paint before anything that might block.
const LOADER_FRAMES: u32 = 2;

fn body() -> Option<HtmlElement> {
    web_sys::window()?.document()?.body()
}

/// When it can't appear: the loader says so, quietly, and the reason goes to the console.
fn fail(why: &JsValue) {
    web_sys::console::error_1(why);
    if let Some(b) = body() {
        let _ = b.class_list().add_1("failed");
    }
}

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
            self.count.set_text_content(Some(&roman(found)));
        }
    }

    /// `contact` knows what it says at contact, which is different for everyone.
    fn update(&mut self, now: f64, found: usize, contact: &Contact) {
        if now < self.toast_until {
            return;
        }
        let _ = self.toast.class_list().remove_1("show");
        if self.queue.is_empty() {
            return;
        }
        let d = self.queue.remove(0);
        let (line, class, linger) = match d {
            Discovery::Contact => (contact.reply(), " class=\"alien\"", REPLY_SECONDS),
            _ => (d.line().to_string(), "", TOAST_SECONDS),
        };
        self.toast.set_inner_html(&format!("<b>{}</b><span{}>{}</span>", d.name(), class, line));
        let _ = self.toast.class_list().add_1("show");
        self.toast_until = now + linger;
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
    /// Frames since the page opened or the context came back; the loader covers the first ones.
    waiting: u32,
    awake: bool,
    /// The GPU dropped the context (driver reset, too much work); we wait for it to come back.
    lost: bool,
    /// The aura last handed to the page, so the CSS only changes when the object does.
    aura_shown: String,
}

impl App {
    /// Pointer position in canvas pixels.
    fn pos(&self, e: &PointerEvent) -> (f32, f32) {
        let sx = self.canvas.width() as f64 / self.canvas.client_width().max(1) as f64;
        let sy = self.canvas.height() as f64 / self.canvas.client_height().max(1) as f64;
        ((e.offset_x() as f64 * sx) as f32, (e.offset_y() as f64 * sy) as f32)
    }

    fn resize(&mut self) {
        let (cw, ch) = (self.canvas.client_width() as f64, self.canvas.client_height() as f64);
        let mut dpr = web_sys::window().unwrap().device_pixel_ratio().min(1.5);
        dpr = dpr.min((MAX_PIXELS / (cw * ch).max(1.0)).sqrt()) * self.quality;
        let w = (cw * dpr).max(1.0) as u32;
        let h = (ch * dpr).max(1.0) as u32;
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        self.game.w = w as f32;
        self.game.h = h as f32;
    }

    /// Waits behind the loader until the shader is compiled, then lets the object in.
    fn wake(&mut self) -> bool {
        if self.awake {
            return true;
        }
        self.waiting += 1;
        if self.waiting <= LOADER_FRAMES {
            return false;
        }
        match self.renderer.poll() {
            Ok(false) => false,
            Ok(true) => {
                self.awake = true;
                if let Some(b) = body() {
                    let _ = b.class_list().add_1("awake");
                }
                true
            }
            Err(e) => {
                fail(&e);
                self.lost = true;
                false
            }
        }
    }

    /// Lets the words on the page take the object's color, unrest and place (see web/glyph.js).
    fn show_aura(&mut self) {
        let a = aura(&self.game.world, &self.game.cam, self.game.w, self.game.h);
        let q = |v: f32| (v * 100.0).round() / 100.0;
        let key = format!("{} {} {} {} {} {}", q(a.tint.x), q(a.tint.y), q(a.tint.z), q(a.unrest), q(a.x), q(a.y));
        if key == self.aura_shown {
            return;
        }
        self.aura_shown = key;
        let Some(root) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) else { return };
        let Ok(root) = root.dyn_into::<HtmlElement>() else { return };
        let style = root.style();
        let rgb = |v: f32| (q(v) * 255.0).round();
        let _ = style.set_property("--tint", &format!("{} {} {}", rgb(a.tint.x), rgb(a.tint.y), rgb(a.tint.z)));
        let _ = style.set_property("--unrest", &q(a.unrest).to_string());
        let _ = style.set_property("--ox", &format!("{}%", q(a.x) * 100.0));
        let _ = style.set_property("--oy", &format!("{}%", q(a.y) * 100.0));
    }

    fn frame(&mut self, now: f64) {
        if self.lost || !self.wake() {
            self.last_frame = None;
            return;
        }
        let raw = self.last_frame.map_or(1.0 / 60.0, |t| (now - t) / 1000.0);
        let dt = raw.clamp(0.0, 1.0 / 20.0);
        self.last_frame = Some(now);

        // Adaptive resolution: a frame that stalls drops quality at once; sitting
        // under ~35 fps for a second drops it gradually.
        // (The first frames after waking can stall on the driver's last compile step; that says nothing about speed.)
        self.waiting = self.waiting.saturating_add(1);
        if raw > 0.25 && self.quality > 0.4 && self.waiting > LOADER_FRAMES + 4 {
            self.quality = (self.quality - 0.25).max(0.4);
            self.slow_time = 0.0;
        } else if dt > 1.0 / 35.0 {
            self.slow_time += dt;
            if self.slow_time > 1.0 && self.quality > 0.4 {
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
        if self.game.contact.dirty {
            self.game.contact.dirty = false;
            if let Some(s) = storage() {
                let _ = s.set_item(KEY_CONTACT, &self.game.contact.save());
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
        self.hud.update(now / 1000.0, self.game.progress.count(), &self.game.contact);
        self.show_aura();
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
        game.contact = Contact::load(&s.get_item(KEY_CONTACT).ok().flatten().unwrap_or_default());
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
        waiting: 0,
        awake: false,
        lost: false,
        aura_shown: String::new(),
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
            let mut a = app.borrow_mut();
            a.game.pointer_up(e.pointer_id());
            match &a.audio {
                Some(audio) => audio.resume(),
                None => a.audio = Audio::new().ok(),
            }
        })?;
    }
    {
        let app = app.clone();
        listen(target, "wheel", false, move |e: WheelEvent| {
            e.prevent_default();
            app.borrow_mut().game.wheel(e.delta_y() as f32);
        })?;
    }
    {
        // Without preventDefault the browser never gives the context back.
        let app = app.clone();
        listen(target, "webglcontextlost", false, move |e: web_sys::Event| {
            e.prevent_default();
            app.borrow_mut().lost = true;
        })?;
    }
    {
        let app = app.clone();
        listen(target, "webglcontextrestored", true, move |_: web_sys::Event| {
            let mut a = app.borrow_mut();
            // Everything on the GPU is gone; build it again, and ask less of it this time.
            match Renderer::new(a.renderer.gl().clone()) {
                Ok(r) => {
                    a.renderer = r;
                    a.awake = false;
                    a.waiting = LOADER_FRAMES;
                    a.quality = (a.quality * 0.6).max(0.4);
                    a.lost = false;
                }
                Err(e) => fail(&e),
            }
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
