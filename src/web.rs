//! Browser glue: canvas, input events (keys go on to keys.rs), the frame loop, the lines of text
//! (hint, toast, count, now and then a thought, and the marks while it hears or sees), what is
//! saved between visits, the microphone and camera once the player has let it have them
//! (senses.rs, devices.rs), and how a phone is held and moved (motion.rs).

use crate::audio::Audio;
use crate::aura::{aura, roman};
use crate::contact::{self, Contact, KEEP_STEPS};
use crate::devices::{self, Answer, Ears, Eyes};
use crate::game::Game;
use crate::keys;
use crate::motion::Felt;
use crate::progress::{Discovery, Progress};
use crate::render::Renderer;
use crate::senses::{self, Kind, Senses};
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
/// Whether the player ever told the browser no to the microphone or the camera (senses.rs).
const KEY_SENSES: &str = "the-object.senses";
/// What it remembers of what was done to it, and when you were last here (remembrance.rs).
const KEY_REMEMBRANCE: &str = "the-object.remembrance";
/// The question stays up while the drop is held out.
const ASK_SECONDS: f64 = 7.0;
/// Its reply stays up longer than anything else it says.
const REPLY_SECONDS: f64 = 10.0;
/// A thought surfaces, stays this long, then sinks (thoughts.rs).
const THOUGHT_SECONDS: f64 = 7.0;
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

/// The answer to its question goes to the page (`the-object:answer` on window, detail 1..=5),
/// which sends it on; see web/index.html.
fn send_answer(n: u8) {
    let Some(w) = web_sys::window() else { return };
    let init = web_sys::CustomEventInit::new();
    init.set_detail(&JsValue::from(n));
    if let Ok(e) = web_sys::CustomEvent::new_with_event_init_dict("the-object:answer", &init) {
        let _ = w.dispatch_event(&e);
    }
}

/// Browser storage can be missing or throw (private windows); every use tolerates that.
fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Something for the toast to say.
enum Line {
    Found(Discovery),
    /// It is holding out a drop after contact (contact.rs, `Ask`).
    Asked,
    /// The drop was let go after this many steps, of KEEP_STEPS.
    Answer(u8),
    /// It asks to hear or to see; the word can be tapped while it is up.
    Offer(Kind),
    /// The browser was told no.
    Refused(Kind),
}

struct Hud {
    hint: HtmlElement,
    toast: HtmlElement,
    count: HtmlElement,
    thought: HtmlElement,
    /// The mark that shows it is hearing or seeing; tapped, it stops.
    marks: HtmlElement,
    queue: Vec<Line>,
    toast_until: f64,
    thought_until: f64,
    /// Whether the toast and the thought carry `show`, so the class is only touched when that changes.
    toast_shown: bool,
    thought_shown: bool,
}

impl Hud {
    fn new(doc: &Document) -> Result<Hud, JsValue> {
        let get = |id: &str| -> Result<HtmlElement, JsValue> {
            Ok(doc.get_element_by_id(id).ok_or(format!("#{id} missing"))?.dyn_into()?)
        };
        Ok(Hud {
            hint: get("hint")?,
            toast: get("toast")?,
            count: get("count")?,
            thought: get("thought")?,
            marks: get("senses")?,
            queue: Vec::new(),
            toast_until: 0.0,
            thought_until: 0.0,
            toast_shown: false,
            thought_shown: false,
        })
    }

    /// A thought surfaces under it. Anything found, or anything it says, sends the thought away.
    fn think(&mut self, now: f64, line: Option<&str>) {
        if let Some(line) = line {
            self.thought.set_text_content(Some(line));
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
        if self.toast_shown {
            self.toast_shown = false;
            let _ = self.toast.class_list().remove_1("show");
        }
        if self.queue.is_empty() {
            return;
        }
        let alien = " class=\"alien\"";
        let (name, line, class, linger) = match self.queue.remove(0) {
            Line::Found(d @ Discovery::Contact) => (d.name(), contact.reply(), alien, REPLY_SECONDS),
            Line::Found(d) => (d.name(), d.line().to_string(), "", TOAST_SECONDS),
            Line::Asked => ("a question", "keep it for as long as this was worth".to_string(), "", ASK_SECONDS),
            Line::Answer(n) => {
                let kept = "⟡".repeat(n as usize) + &"∴".repeat((KEEP_STEPS - n.min(KEEP_STEPS)) as usize);
                (if n > 1 { "kept" } else { "let go" }, kept, alien, TOAST_SECONDS)
            }
            Line::Offer(k) => ("it asks", k.word().to_string(), " class=\"offer\"", senses::OFFER_WINDOW as f64),
            Line::Refused(k) => ("no", k.refused().to_string(), "", TOAST_SECONDS),
        };
        self.toast.set_inner_html(&format!("<b>{}</b><span{}>{}</span>", name, class, line));
        let _ = self.toast.class_list().add_1("show");
        self.toast_shown = true;
        self.toast_until = now + linger;
        self.show_count(found);
        let _ = self.hint.class_list().add_1("gone");
    }
}

struct App {
    window: web_sys::Window,
    game: Game,
    renderer: Renderer,
    canvas: HtmlCanvasElement,
    hud: Hud,
    audio: Option<Audio>,
    night_shown: bool,
    last_frame: Option<f64>,
    /// When each pointer last moved, so two fingers don't time each other's moves.
    last_move: Vec<(i32, f64)>,
    /// Fraction of device pixels rendered; lowered automatically on slow devices.
    quality: f64,
    slow_time: f64,
    /// Frames since the page opened or the context came back; the loader covers the first ones.
    waiting: u32,
    awake: bool,
    /// The GPU dropped the context (driver reset, too much work); we wait for it to come back.
    lost: bool,
    /// The aura last handed to the page, in hundredths (tint r g b, unrest, x, y), so the CSS
    /// only changes when the object does, and only the variables that moved are written.
    aura_shown: [i32; 6],
    /// The page root's style, where the aura is written.
    root_style: Option<web_sys::CssStyleDeclaration>,
    /// Canvas pixels per CSS pixel, measured once a frame in `resize`, so pointer events never ask for layout.
    scale: (f64, f64),
    /// The canvas cursor last set.
    cursor: &'static str,
    /// The microphone and the camera, while it is let have them.
    ears: Option<Ears>,
    eyes: Option<Eyes>,
    /// What the browser answered, waiting for the next frame.
    answers: Rc<RefCell<Vec<Answer>>>,
    /// The word it asked with, and until when (seconds) it can be tapped.
    offer: Option<(Kind, f64)>,
    /// Made inside the tap that let it hear, waiting for the microphone.
    ear_ctx: Option<web_sys::AudioContext>,
    /// How the phone was held and pushed since the last frame, and whether it is being felt.
    felt: Rc<RefCell<Felt>>,
    feeling: bool,
    /// What the browser answered when asked to let it feel.
    feel_answers: Rc<RefCell<Vec<Result<(), bool>>>>,
}

impl App {
    /// Pointer position in canvas pixels.
    fn pos(&self, e: &PointerEvent) -> (f32, f32) {
        let (sx, sy) = self.scale;
        ((e.offset_x() as f64 * sx) as f32, (e.offset_y() as f64 * sy) as f32)
    }

    /// Seconds since this pointer last moved (a frame, if it hasn't yet), and remember now.
    fn moved(&mut self, id: i32, now: f64) -> f32 {
        match self.last_move.iter_mut().find(|p| p.0 == id) {
            Some(p) => ((now - std::mem::replace(&mut p.1, now)) / 1000.0).max(0.001) as f32,
            None => {
                self.last_move.push((id, now));
                1.0 / 60.0
            }
        }
    }

    fn resize(&mut self) {
        let (cw, ch) = (self.canvas.client_width(), self.canvas.client_height());
        let (cwi, chi) = (cw.max(1) as f64, ch.max(1) as f64);
        let (cw, ch) = (cw as f64, ch as f64);
        let mut dpr = self.window.device_pixel_ratio().min(1.5);
        dpr = dpr.min((MAX_PIXELS / (cw * ch).max(1.0)).sqrt()) * self.quality;
        let w = (cw * dpr).max(1.0) as u32;
        let h = (ch * dpr).max(1.0) as u32;
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        self.scale = (w as f64 / cwi, h as f64 / chi);
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

    /// Stops hearing (or seeing) at once: the device is let go before anything else happens.
    fn close(&mut self, kind: Kind) {
        self.game.senses.close(kind);
        match kind {
            Kind::Hear => self.ears.take().map(Ears::close),
            Kind::See => self.eyes.take().map(Eyes::close),
            Kind::Feel => None,
        };
        if self.offer.is_some_and(|o| o.0 == kind) {
            self.offer = None;
        }
        self.show_marks();
    }

    fn close_all(&mut self) {
        self.close(Kind::Hear);
        self.close(Kind::See);
    }

    /// One quiet mark for each thing it is let have, in the corner opposite the count.
    fn show_marks(&self) {
        let (ear, eye) = (self.ears.is_some(), self.eyes.is_some());
        let marks: Vec<&str> = [(ear, "◍"), (eye, "◉")].iter().filter(|m| m.0).map(|m| m.1).collect();
        self.hud.marks.set_text_content(Some(&marks.join(" ")));
        let what = match (ear, eye) {
            (true, true) => "it is listening and watching. tap here or press escape to stop.",
            (true, false) => "it is listening. tap here or press escape to stop.",
            (false, true) => "it is watching. tap here or press escape to stop.",
            _ => "",
        };
        let _ = self.hud.marks.set_attribute("aria-label", what);
        let _ = self.hud.marks.set_attribute("title", what);
        let _ = self.hud.marks.class_list().toggle_with_force("on", ear || eye);
    }

    /// The word was tapped while it was up: now, inside the tap, the browser may ask.
    fn take_offer(&mut self, now: f64) {
        let Some((kind, until)) = self.offer.take() else { return };
        if now / 1000.0 > until || !self.game.senses.accept(kind) {
            return;
        }
        match kind {
            Kind::Feel => devices::ask_to_feel(self.feel_answers.clone()),
            Kind::Hear => {
                self.ear_ctx = devices::ear_context();
                devices::ask(kind, self.answers.clone());
            }
            Kind::See => devices::ask(kind, self.answers.clone()),
        }
        self.hud.toast_until = 0.0;
    }

    /// Starts feeling the phone, once.
    fn feel(&mut self) {
        if !self.feeling {
            self.feeling = true;
            devices::listen_to_feel(self.felt.clone());
        }
    }

    /// What the browser answered; and, while it is let, what it hears, sees and feels this frame.
    fn sense(&mut self, now: f64) {
        let answers = std::mem::take(&mut *self.answers.borrow_mut());
        for (kind, answer) in answers {
            let waiting = match kind {
                Kind::Hear => self.game.senses.hear == senses::Leave::Opening,
                Kind::See => self.game.senses.see == senses::Leave::Opening,
                Kind::Feel => false,
            };
            match answer {
                // Stopped while the browser was still asking: let it go unused.
                Ok(stream) if !waiting => devices::stop(&stream),
                Ok(stream) => {
                    let opened = match kind {
                        Kind::Hear => match self.ear_ctx.take() {
                            Some(ctx) => Ears::new(ctx, stream.clone()).map(|e| self.ears = Some(e)),
                            None => Err(JsValue::from_str("no audio context")),
                        },
                        Kind::See => Eyes::new(stream.clone()).map(|e| self.eyes = Some(e)),
                        Kind::Feel => Err(JsValue::from_str("nothing to open")),
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
                Err(no) => self.game.senses.refused(kind, no),
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
        self.sense(now);
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
        if self.game.senses.dirty {
            self.game.senses.dirty = false;
            if let Some(s) = storage() {
                let _ = s.set_item(KEY_SENSES, &self.game.senses.save());
            }
        }
        if self.game.remembrance.dirty {
            self.game.remembrance.dirty = false;
            if let Some(s) = storage() {
                let _ = s.set_item(KEY_REMEMBRANCE, &self.game.remembrance.save());
            }
        }
        for e in self.game.senses.drain_events() {
            match e {
                senses::Event::Offer(k) => {
                    self.hud.toast_until = 0.0;
                    self.hud.queue.insert(0, Line::Offer(k));
                    self.offer = Some((k, now / 1000.0 + senses::OFFER_WINDOW as f64));
                }
                senses::Event::Refused(k) => self.hud.queue.insert(0, Line::Refused(k)),
                senses::Event::Closed(_) => {}
            }
        }
        self.hud.queue.extend(self.game.announcements.drain(..).map(Line::Found));
        for e in std::mem::take(&mut self.game.question) {
            match e {
                contact::Event::Asked => {
                    // Whatever else it was saying can wait; this is asked now.
                    self.hud.toast_until = 0.0;
                    self.hud.queue.insert(0, Line::Asked);
                }
                contact::Event::Answer(n) => {
                    self.hud.toast_until = 0.0;
                    self.hud.queue.insert(0, Line::Answer(n));
                    send_answer(n);
                }
                _ => {}
            }
        }
        if self.audio.as_ref().is_some_and(|a| !a.muted) {
            // A microphone near the speaker would hear these; it is not to take them for the player.
            for s in &self.game.sounds {
                self.game.senses.deafen(if *s == crate::game::Sound::Discover { 3.0 } else { 1.2 });
            }
        }
        if let Some(a) = &self.audio {
            for s in self.game.sounds.drain(..) {
                a.play(s);
            }
            a.follow(&self.game.world);
            a.hum(self.game.keys.hum());
            a.echo(self.game.senses.hum());
        }
        // Emptied in place, so the list keeps its room for the next frame.
        self.game.sounds.clear();
        // the fold is dark too (forms/fold.rs)
        let night = self.game.world.night.max(self.game.world.forms.fold.amount()) > 0.5;
        if night != self.night_shown {
            self.night_shown = night;
            if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
                let _ = body.class_list().toggle_with_force("night", night);
            }
        }
        let thought = self.game.thoughts.out.drain(..).last();
        self.hud.think(now / 1000.0, thought);
        self.hud.update(now / 1000.0, self.game.progress.count(), &self.game.contact);
        self.show_aura();
        self.renderer.draw(&self.game.world, &self.game.cam, self.canvas.width(), self.canvas.height(), self.quality as f32);
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
    // Without storage every visit is a first one.
    game.thoughts.first_visit = true;
    let hud = Hud::new(&doc)?;
    if let Some(s) = storage() {
        let found = s.get_item(KEY_FOUND).ok().flatten().unwrap_or_default();
        let visits = s.get_item(KEY_VISITS).ok().flatten().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) + 1;
        let _ = s.set_item(KEY_VISITS, &visits.to_string());
        game.thoughts.first_visit = visits == 1;
        game.progress = Progress::load(&found);
        game.contact = Contact::load(&s.get_item(KEY_CONTACT).ok().flatten().unwrap_or_default());
        game.senses = Senses::load(&s.get_item(KEY_SENSES).ok().flatten().unwrap_or_default());
        game.remembrance = crate::remembrance::Remembrance::load(
            &s.get_item(KEY_REMEMBRANCE).ok().flatten().unwrap_or_default(),
            js_sys::Date::now() / 1000.0,
        );
        hud.show_count(game.progress.count());
        if visits > 1 && game.progress.count() > 0 {
            game.discover(Discovery::Return);
            game.sounds.clear();
        }
    }

    game.senses.can_hear = devices::possible(Kind::Hear);
    game.senses.can_see = devices::possible(Kind::See);
    let feeling = devices::feeling();
    match feeling {
        devices::Feeling::Free => game.senses.feel = senses::Leave::Open,
        devices::Feeling::Ask => game.senses.can_feel = true,
        devices::Feeling::Never => {}
    }

    let app = Rc::new(RefCell::new(App {
        window: window.clone(),
        game,
        renderer,
        canvas: canvas.clone(),
        hud,
        audio: None,
        night_shown: false,
        last_frame: None,
        last_move: Vec::new(),
        quality: 1.0,
        slow_time: 0.0,
        waiting: 0,
        awake: false,
        lost: false,
        aura_shown: [i32::MIN; 6],
        root_style: doc.document_element().and_then(|r| r.dyn_into::<HtmlElement>().ok()).map(|r| r.style()),
        scale: (1.0, 1.0),
        cursor: "",
        ears: None,
        eyes: None,
        answers: Rc::new(RefCell::new(Vec::new())),
        offer: None,
        ear_ctx: None,
        felt: Rc::new(RefCell::new(Felt::default())),
        feeling: false,
        feel_answers: Rc::new(RefCell::new(Vec::new())),
    }));
    app.borrow_mut().resize();
    if feeling == devices::Feeling::Free {
        app.borrow_mut().feel();
    }

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
            a.moved(e.pointer_id(), e.time_stamp());
            a.game.pointer_down(e.pointer_id(), x, y);
        })?;
    }
    {
        let app = app.clone();
        let c = canvas.clone();
        listen(target, "pointermove", true, move |e: PointerEvent| {
            let mut a = app.borrow_mut();
            let (x, y) = a.pos(&e);
            let dt = a.moved(e.pointer_id(), e.time_stamp());
            a.game.pointer_move(e.pointer_id(), x, y, dt);
            let cursor = if a.game.hovering_liquid(x, y) { "grab" } else { "default" };
            if cursor != a.cursor {
                a.cursor = cursor;
                let _ = c.style().set_property("cursor", cursor);
            }
        })?;
    }
    for name in ["pointerup", "pointercancel"] {
        let app = app.clone();
        listen(target, name, true, move |e: PointerEvent| {
            let mut a = app.borrow_mut();
            a.game.pointer_up(e.pointer_id());
            // A mouse keeps hovering after it lets go; a finger is gone.
            if e.pointer_type() != "mouse" {
                a.last_move.retain(|p| p.0 != e.pointer_id());
            }
            match &a.audio {
                Some(audio) => audio.resume(),
                None => a.audio = Audio::new().ok(),
            }
            if let Some(ears) = &a.ears {
                ears.resume();
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
        listen(window.as_ref(), "keydown", false, move |e: KeyboardEvent| {
            // The browser's own shortcuts stay the browser's.
            if e.ctrl_key() || e.meta_key() || e.alt_key() {
                return;
            }
            let mut a = app.borrow_mut();
            if a.audio.is_none() {
                a.audio = Audio::new().ok();
            } else if let Some(audio) = &a.audio {
                audio.resume();
            }
            if e.key() == "m" {
                if let Some(audio) = &mut a.audio {
                    audio.toggle_mute();
                }
            }
            if e.key() == "Escape" {
                a.close_all();
            }
            if !e.repeat() && keys::down(&mut a.game, &e.code()) {
                e.prevent_default();
            } else if e.repeat() && (e.code() == "Space" || e.code().starts_with("Arrow")) {
                e.prevent_default();
            }
        })?;
    }
    {
        let app = app.clone();
        listen(window.as_ref(), "keyup", true, move |e: KeyboardEvent| {
            keys::up(&mut app.borrow_mut().game, &e.code());
        })?;
    }
    {
        // Its word, tapped while it is up, is the only thing that lets the browser ask.
        let app = app.clone();
        let toast: web_sys::EventTarget = app.borrow().hud.toast.clone().into();
        listen(&toast, "click", true, move |e: web_sys::MouseEvent| {
            app.borrow_mut().take_offer(e.time_stamp());
        })?;
    }
    {
        let app = app.clone();
        listen(window.as_ref(), "blur", true, move |_: web_sys::Event| {
            keys::release(&mut app.borrow_mut().game);
        })?;
    }
    {
        let app = app.clone();
        let marks: web_sys::EventTarget = app.borrow().hud.marks.clone().into();
        listen(&marks, "click", true, move |_: web_sys::MouseEvent| app.borrow_mut().close_all())?;
    }
    {
        // Nobody is looking at the page: it stops hearing and seeing.
        let app = app.clone();
        let d = doc.clone();
        listen(doc.as_ref(), "visibilitychange", true, move |_: web_sys::Event| {
            if d.hidden() {
                app.borrow_mut().close_all();
            }
        })?;
    }

    // requestAnimationFrame loop.
    let tick: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let tick2 = tick.clone();
    let w = window.clone();
    *tick2.borrow_mut() = Some(Closure::new(move |now: f64| {
        app.borrow_mut().frame(now);
        let _ = w.request_animation_frame(tick.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    window.request_animation_frame(tick2.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
