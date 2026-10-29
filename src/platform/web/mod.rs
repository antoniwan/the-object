//! Browser glue: canvas, input events (keys go on to keys.rs), the frame loop, the lines of text
//! (hint, toast, count, now and then a thought, and the marks while it hears or sees), what is
//! saved between visits, the microphone and camera once the player has let it have them
//! (senses.rs, devices.rs), words made of what it hears (words.rs, speech.rs), and how a phone
//! is held and moved (motion.rs).

use crate::platform::audio::Audio;
use crate::mind::contact::{self, Contact};
use crate::platform::devices::{self, Answer, Ears, Eyes};
use crate::game::Game;
use crate::input::keys;
use crate::input::motion::Felt;
use crate::progress::{Discovery, Progress};
use crate::platform::render::Renderer;
use crate::input::senses::{self, Kind, Senses};
use crate::platform::speech::{self, Can, Note, Speech};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement, KeyboardEvent, PointerEvent, WebGl2RenderingContext, WheelEvent};

mod frame;
mod hud;
mod page;
mod sensing;
mod storage;

use hud::{Hud, Line};
use page::{body, fail, listen, send_answer, storage};
use storage::{load, save, KEY_CONTACT, KEY_FOUND, KEY_REMEMBRANCE, KEY_SENSES, KEY_VISITS};

/// Frames the loader gets to paint before anything that might block.
const LOADER_FRAMES: u32 = 2;

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
    /// How dark the room last was, as the browser's own bars were told (thirty-seconds).
    room_shown: i32,
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
    /// What this browser can do with a voice, the recognizer while it runs, what it said, and
    /// whether it stopped for good this visit.
    can_speech: Rc<Cell<Can>>,
    speech: Option<Speech>,
    notes: Rc<RefCell<Vec<Note>>>,
    speech_gone: bool,
    /// The languages words are listened for in (speech.rs).
    tongues: Vec<String>,
    /// Things said in a row with nothing in them it knows.
    unknown: u32,
}

impl App {
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
            save(KEY_FOUND, || self.game.progress.save());
        }
        if self.game.contact.dirty {
            self.game.contact.dirty = false;
            save(KEY_CONTACT, || self.game.contact.save());
        }
        if self.game.senses.dirty {
            self.game.senses.dirty = false;
            save(KEY_SENSES, || self.game.senses.save());
        }
        if self.game.remembrance.dirty {
            self.game.remembrance.dirty = false;
            save(KEY_REMEMBRANCE, || self.game.remembrance.save());
        }
        for signs in self.game.words.said.drain(..) {
            self.hud.urgent(Line::Greeted(signs));
        }
        for e in self.game.senses.drain_events() {
            match e {
                senses::Event::Offer(k) => {
                    self.hud.urgent(Line::Offer(k));
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
                    self.hud.urgent(Line::Asked);
                }
                contact::Event::Answer(n) => {
                    self.hud.urgent(Line::Answer(n));
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
        let dark = self.game.world.night.max(self.game.world.forms.fold.amount());
        self.show_room(dark);
        let night = dark > 0.5;
        if night != self.night_shown {
            self.night_shown = night;
            if let Some(body) = body() {
                let _ = body.class_list().toggle_with_force("night", night);
            }
        }
        self.show_marks();
        let thought = self.game.thoughts.out.drain(..).next_back();
        self.hud.think(now / 1000.0, thought);
        self.hud.update(now / 1000.0, self.game.progress.count(), &self.game.contact);
        self.show_aura();
        self.renderer.draw(&self.game.world, &self.game.cam, self.canvas.width(), self.canvas.height(), self.quality as f32);
    }

    /// Sound can only start inside a gesture; the first one makes it.
    fn start_audio(&mut self) {
        if self.audio.is_none() {
            self.audio = Audio::new().ok();
        }
    }

    /// Inside a gesture: sound starts, or carries on if the browser held it.
    fn wake_audio(&mut self) {
        if let Some(audio) = &self.audio {
            audio.resume();
        } else {
            self.start_audio();
        }
    }

    fn set_cursor(&mut self, cursor: &'static str) {
        if cursor != self.cursor {
            self.cursor = cursor;
            let _ = self.canvas.style().set_property("cursor", cursor);
        }
    }
}

/// The frame loop's callback, kept alive by the loop itself.
type Frame = Closure<dyn FnMut(f64)>;

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
    if storage().is_some() {
        let found = load(KEY_FOUND);
        let visits = load(KEY_VISITS).parse::<u32>().unwrap_or(0) + 1;
        save(KEY_VISITS, || visits.to_string());
        game.thoughts.first_visit = visits == 1;
        game.progress = Progress::load(&found);
        game.contact = Contact::load(&load(KEY_CONTACT));
        game.senses = Senses::load(&load(KEY_SENSES));
        game.remembrance = crate::mind::remembrance::Remembrance::load(&load(KEY_REMEMBRANCE), js_sys::Date::now() / 1000.0);
        hud.show_count(game.progress.count());
        if visits > 1 && game.progress.count() > 0 {
            game.discover(Discovery::Return);
            game.sounds.clear();
        }
    }

    game.senses.can_hear = devices::possible(Kind::Hear);
    game.senses.can_see = devices::possible(Kind::See);
    let tongues = speech::tongues();
    let can_speech = Rc::new(Cell::new(if game.senses.can_hear { speech::can() } else { Can::Nothing }));
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
        room_shown: 0,
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
        can_speech,
        speech: None,
        notes: Rc::new(RefCell::new(Vec::new())),
        speech_gone: false,
        tongues,
        unknown: 0,
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
            a.start_audio();
            let (x, y) = a.pos(&e);
            a.moved(e.pointer_id(), e.time_stamp());
            let grab = a.game.hovering_liquid(x, y);
            a.game.pointer_down(e.pointer_id(), x, y);
            if grab {
                a.set_cursor("grabbing");
            }
        })?;
    }
    {
        let app = app.clone();
        listen(target, "pointermove", true, move |e: PointerEvent| {
            let mut a = app.borrow_mut();
            let (x, y) = a.pos(&e);
            let dt = a.moved(e.pointer_id(), e.time_stamp());
            a.game.pointer_move(e.pointer_id(), x, y, dt);
            // Whatever it was when the hand closed, it stays until the hand opens.
            if a.game.held() {
                return;
            }
            let cursor = if a.game.hovering_liquid(x, y) { "grab" } else { "default" };
            a.set_cursor(cursor);
        })?;
    }
    for name in ["pointerup", "pointercancel"] {
        let app = app.clone();
        listen(target, name, true, move |e: PointerEvent| {
            let mut a = app.borrow_mut();
            a.game.pointer_up(e.pointer_id());
            if a.cursor == "grabbing" && !a.game.held() {
                a.set_cursor("grab");
            }
            // A mouse keeps hovering after it lets go; a finger is gone.
            if e.pointer_type() != "mouse" {
                a.last_move.retain(|p| p.0 != e.pointer_id());
            }
            a.wake_audio();
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
            a.wake_audio();
            if e.key() == "m" {
                if let Some(audio) = &mut a.audio {
                    audio.toggle_mute();
                }
            }
            if e.key() == "Escape" {
                a.close_all();
            }
            // While its word is up, Enter is the same as tapping it.
            if e.key() == "Enter" && !e.repeat() && a.offer.is_some() {
                e.prevent_default();
                a.take_offer(e.time_stamp());
                return;
            }
            // A first press the keyboard took, or a held key that would scroll the page.
            if (!e.repeat() && keys::down(&mut a.game, &e.code()))
                || (e.repeat() && (e.code() == "Space" || e.code().starts_with("Arrow")))
            {
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
        listen(&marks, "click", true, move |e: web_sys::MouseEvent| {
            let waiting = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|el| el.closest("[data-k]").ok().flatten())
                .and_then(|el| el.get_attribute("data-k"));
            let mut a = app.borrow_mut();
            match waiting.as_deref() {
                Some("hear") => a.take_waiting(Kind::Hear),
                Some("see") => a.take_waiting(Kind::See),
                _ => a.close_all(),
            }
        })?;
    }
    {
        let app = app.clone();
        let mic: web_sys::EventTarget = app.borrow().hud.mic.clone().into();
        listen(&mic, "click", true, move |_: web_sys::MouseEvent| {
            app.borrow_mut().take_waiting(Kind::Hear);
        })?;
    }
    {
        // Nobody is looking at the page: it stops hearing and seeing.
        let app = app.clone();
        let d = doc.clone();
        listen(doc.as_ref(), "visibilitychange", true, move |_: web_sys::Event| {
            let mut a = app.borrow_mut();
            if d.hidden() {
                a.close_all();
                if let Some(audio) = &a.audio {
                    audio.suspend();
                }
            } else if let Some(audio) = &a.audio {
                audio.resume();
            }
        })?;
    }

    // requestAnimationFrame loop.
    let tick: Rc<RefCell<Option<Frame>>> = Rc::new(RefCell::new(None));
    let tick2 = tick.clone();
    let w = window.clone();
    *tick2.borrow_mut() = Some(Closure::new(move |now: f64| {
        app.borrow_mut().frame(now);
        let _ = w.request_animation_frame(tick.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    window.request_animation_frame(tick2.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
