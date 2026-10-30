//! The microphone and the camera, once the player has tapped its word and the browser has let
//! it have them. Neither is ever heard or seen by anyone: each frame the microphone gives a short
//! window of samples and the camera a picture 32 x 24 pixels wide, senses/ turns them into a
//! few numbers, and they are dropped. Nothing is recorded, kept or sent. Closing stops every
//! track at once, so the browser's own indicator goes out with ours.
//!
//! Also how a phone is held and moved, for motion.rs: two angles and one push a frame, turned to
//! the screen, used and dropped.

use super::get;
use crate::input::motion::Felt;
use crate::input::senses::{self, Heard, Kind, Retina, Seen};
use crate::math::v3;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, MediaStream,
    MediaStreamConstraints,
};

/// How small it looks.
const SIGHT_W: u32 = 32;
const SIGHT_H: u32 = 24;
/// Looks per second; more says nothing new.
const SIGHT_RATE: f64 = 12.0;

/// What came back from the browser: the stream, or whether the player (or the page) said no.
pub type Answer = (Kind, Result<MediaStream, bool>);

fn obj(pairs: &[(&str, JsValue)]) -> JsValue {
    let o = js_sys::Object::new();
    for (k, v) in pairs {
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o.into()
}

/// Whether this page could ever be given it: a secure page with media devices, and not inside a
/// frame that forbids it (itch.io embeds the object in one that does).
pub fn possible(kind: Kind) -> bool {
    let Some(w) = web_sys::window() else { return false };
    if !w.is_secure_context() || get(&w.navigator(), "mediaDevices").is_undefined() {
        return false;
    }
    match kind {
        Kind::Hear => allowed(&w, &["microphone"]),
        Kind::See => allowed(&w, &["camera"]),
        Kind::Feel | Kind::Words => false,
    }
}

/// Whether the frame the page is in lets it use these.
fn allowed(w: &web_sys::Window, features: &[&str]) -> bool {
    let doc: JsValue = w.document().map(Into::into).unwrap_or(JsValue::UNDEFINED);
    let mut policy = get(&doc, "permissionsPolicy");
    if policy.is_undefined() || policy.is_null() {
        policy = get(&doc, "featurePolicy");
    }
    if let Ok(allows) = get(&policy, "allowsFeature").dyn_into::<js_sys::Function>() {
        return features
            .iter()
            .all(|f| allows.call1(&policy, &JsValue::from_str(f)).map(|v| v.is_truthy()).unwrap_or(false));
    }
    // No way to ask: only the top of the page is trusted to have it.
    w.top().ok().flatten().is_some_and(|top| JsValue::from(top) == JsValue::from(w.clone()))
}

/// How this page may feel how the phone is held.
#[derive(Clone, Copy, PartialEq)]
pub enum Feeling {
    /// Never here: an old browser, an insecure page, a frame that forbids it.
    Never,
    /// Whenever the phone says anything (most phones; a desk says nothing).
    Free,
    /// Only once the browser has asked, inside the player's tap (iOS).
    Ask,
}

pub fn feeling() -> Feeling {
    let Some(w) = web_sys::window() else { return Feeling::Never };
    let event = get(&w, "DeviceOrientationEvent");
    if !w.is_secure_context() || event.is_undefined() || !allowed(&w, &["accelerometer", "gyroscope"]) {
        return Feeling::Never;
    }
    // Desktop browsers have the question too, and nothing to ask about: only a hand-held
    // screen is worth asking for.
    if get(&event, "requestPermission").is_function() && coarse(&w) {
        Feeling::Ask
    } else {
        Feeling::Free
    }
}

/// Whether the page is mostly touched by a finger.
fn coarse(w: &web_sys::Window) -> bool {
    matches(w, "(pointer: coarse)")
}

/// Whether the player asked their system for less motion.
pub fn still() -> bool {
    web_sys::window().is_some_and(|w| matches(&w, "(prefers-reduced-motion: reduce)"))
}

/// Whether a media query holds; false where it cannot be asked.
fn matches(w: &web_sys::Window, query: &str) -> bool {
    let Ok(f) = get(w, "matchMedia").dyn_into::<js_sys::Function>() else { return false };
    f.call1(w, &JsValue::from_str(query)).map(|m| get(&m, "matches").is_truthy()).unwrap_or(false)
}

/// Asks the browser to let it feel the phone. Must be called inside the player's tap. Pushes
/// `Ok` when it may, `Err(true)` when the player said no, `Err(false)` when it simply could not.
pub fn ask_to_feel(answers: Rc<RefCell<Vec<Result<(), bool>>>>) {
    let Some(w) = web_sys::window() else { return };
    let ask = |name: &str| -> Option<js_sys::Promise> {
        let event = get(&w, name);
        let f = get(&event, "requestPermission").dyn_into::<js_sys::Function>().ok()?;
        f.call0(&event).ok()?.dyn_into().ok()
    };
    // One question covers both on the phones that ask; both are asked inside the same tap.
    let motion = ask("DeviceMotionEvent");
    let Some(promise) = ask("DeviceOrientationEvent").or(motion) else {
        answers.borrow_mut().push(Err(false));
        return;
    };
    let yes = answers.clone();
    let ok = Closure::once(move |s: JsValue| {
        yes.borrow_mut().push(if s.as_string().as_deref() == Some("granted") { Ok(()) } else { Err(true) })
    });
    let err = Closure::once(move |_: JsValue| answers.borrow_mut().push(Err(false)));
    let _ = promise.then2(&ok, &err);
    ok.forget();
    err.forget();
}

fn number(target: &JsValue, key: &str) -> Option<f32> {
    get(target, key).as_f64().map(|v| v as f32)
}

/// Which way the screen is turned, in degrees: 0, 90, 180 or 270.
fn turn(w: &web_sys::Window) -> f32 {
    let screen = get(w, "screen");
    number(&get(&screen, "orientation"), "angle").or_else(|| number(w, "orientation")).unwrap_or(0.0).rem_euclid(360.0)
}

/// Starts feeling how the phone is held and moved. Each reading lands in `felt`, which the
/// frame takes and empties; the harder push since the last frame is the one kept.
pub fn listen_to_feel(felt: Rc<RefCell<Felt>>) {
    let Some(w) = web_sys::window() else { return };
    let target: web_sys::EventTarget = w.clone().into();
    let held = felt.clone();
    let win = w.clone();
    let on_held = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        if let (Some(beta), Some(gamma)) = (number(&e, "beta"), number(&e, "gamma")) {
            held.borrow_mut().held = Some((beta, gamma, turn(&win)));
        }
    });
    // Where only the push with gravity in it is given, gravity is what stays: it is followed
    // slowly and taken away.
    let mut gravity: Option<(f32, f32, f32)> = None;
    let win = w.clone();
    let on_push = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        let bare = get(&e, "acceleration");
        let xyz = |a: &JsValue| Some((number(a, "x")?, number(a, "y")?, number(a, "z")?));
        let (x, y, z) = match xyz(&bare) {
            Some(a) => a,
            None => {
                let Some(a) = xyz(&get(&e, "accelerationIncludingGravity")) else { return };
                let g = gravity.get_or_insert(a);
                let k = 0.1;
                *g = (g.0 + (a.0 - g.0) * k, g.1 + (a.1 - g.1) * k, g.2 + (a.2 - g.2) * k);
                (a.0 - g.0, a.1 - g.1, a.2 - g.2)
            }
        };
        let (s, c) = turn(&win).to_radians().sin_cos();
        let p = v3(x * c - y * s, x * s + y * c, z);
        let mut f = felt.borrow_mut();
        if f.push.is_none_or(|q| p.len() > q.len()) {
            f.push = Some(p);
        }
    });
    let _ = target.add_event_listener_with_callback("deviceorientation", on_held.as_ref().unchecked_ref());
    let _ = target.add_event_listener_with_callback("devicemotion", on_push.as_ref().unchecked_ref());
    on_held.forget();
    on_push.forget();
}

/// Asks the browser. Must be called inside the player's tap. The answer is pushed to `answers`.
pub fn ask(kind: Kind, answers: Rc<RefCell<Vec<Answer>>>) {
    let c = MediaStreamConstraints::new();
    match kind {
        // Raw, so a whisper stays a whisper and a shout a shout (phones flatten both otherwise).
        // It never hears its own hum: it does not listen while it hums (senses/).
        Kind::Hear => {
            c.set_audio(&obj(&[
                ("echoCancellation", false.into()),
                ("noiseSuppression", false.into()),
                ("autoGainControl", false.into()),
            ]));
            c.set_video(&false.into());
        }
        Kind::Feel | Kind::Words => {
            answers.borrow_mut().push((kind, Err(false)));
            return;
        }
        Kind::See => {
            c.set_audio(&false.into());
            c.set_video(&obj(&[
                ("facingMode", "user".into()),
                ("width", obj(&[("ideal", 160.into())])),
                ("height", obj(&[("ideal", 120.into())])),
                ("frameRate", obj(&[("ideal", 15.into())])),
            ]));
        }
    }
    let promise = web_sys::window()
        .map(|w| w.navigator())
        .and_then(|n| n.media_devices().ok())
        .and_then(|d| d.get_user_media_with_constraints(&c).ok());
    let Some(promise) = promise else {
        answers.borrow_mut().push((kind, Err(false)));
        return;
    };
    let yes = answers.clone();
    let ok =
        Closure::once(move |s: JsValue| yes.borrow_mut().push((kind, s.dyn_into::<MediaStream>().map_err(|_| false))));
    let err = Closure::once(move |e: JsValue| {
        let name = get(&e, "name").as_string().unwrap_or_default();
        answers.borrow_mut().push((kind, Err(name == "NotAllowedError" || name == "SecurityError")));
    });
    let _ = promise.then2(&ok, &err);
    ok.forget();
    err.forget();
}

/// Stops every track, so the browser lets go of the device.
pub fn stop(stream: &MediaStream) {
    for t in stream.get_tracks().iter() {
        if let Ok(t) = t.dyn_into::<web_sys::MediaStreamTrack>() {
            t.stop();
        }
    }
}

/// The microphone, through an analyser and nowhere else: it is never played or kept.
pub struct Ears {
    ctx: AudioContext,
    analyser: AnalyserNode,
    stream: MediaStream,
    buf: Vec<f32>,
    /// The context's rate never changes, so it is asked once.
    rate: f32,
}

/// The context the microphone will be heard through, made inside the player's tap: phones
/// (iOS above all) keep a context made any later silent.
pub fn ear_context() -> Option<AudioContext> {
    let ctx = AudioContext::new().ok()?;
    let _ = ctx.resume();
    Some(ctx)
}

impl Ears {
    pub fn new(ctx: AudioContext, stream: MediaStream) -> Result<Ears, JsValue> {
        let source = ctx.create_media_stream_source(&stream)?;
        let analyser = ctx.create_analyser()?;
        analyser.set_fft_size(2048);
        source.connect_with_audio_node(&analyser)?;
        let _ = ctx.resume();
        let rate = ctx.sample_rate();
        Ok(Ears { ctx, analyser, stream, buf: vec![0.0; 2048], rate })
    }

    /// Touch screens only grant audio inside a gesture, so this is retried on every release.
    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn listen(&mut self) -> Heard {
        self.analyser.get_float_time_domain_data(&mut self.buf);
        senses::hear(&self.buf, self.rate)
    }

    pub fn close(self) {
        stop(&self.stream);
        let _ = self.ctx.close();
    }
}

/// The camera, into a video element that is never shown and a canvas that is never shown.
pub struct Eyes {
    video: HtmlVideoElement,
    sight: CanvasRenderingContext2d,
    stream: MediaStream,
    retina: Retina,
    last: f64,
    /// The picture's pixels, copied into the same place every look.
    px: Vec<u8>,
}

impl Eyes {
    pub fn new(stream: MediaStream) -> Result<Eyes, JsValue> {
        let doc = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
        let video: HtmlVideoElement = doc.create_element("video")?.dyn_into()?;
        video.set_muted(true);
        let _ = video.set_attribute("playsinline", "");
        video.set_src_object(Some(&stream));
        let _ = video.play();
        let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
        canvas.set_width(SIGHT_W);
        canvas.set_height(SIGHT_H);
        let sight = canvas
            .get_context_with_context_options("2d", &obj(&[("willReadFrequently", true.into())]))?
            .ok_or("no 2d")?
            .dyn_into()?;
        let px = vec![0; (SIGHT_W * SIGHT_H * 4) as usize];
        Ok(Eyes { video, sight, stream, retina: Retina::default(), last: 0.0, px })
    }

    /// A look, at most SIGHT_RATE times a second, once the camera has a picture.
    pub fn look(&mut self, now: f64) -> Option<Seen> {
        if now - self.last < 1000.0 / SIGHT_RATE || self.video.ready_state() < 2 {
            return None;
        }
        self.last = now;
        let (w, h) = (f64::from(SIGHT_W), f64::from(SIGHT_H));
        self.sight.draw_image_with_html_video_element_and_dw_and_dh(&self.video, 0.0, 0.0, w, h).ok()?;
        let image: JsValue = self.sight.get_image_data(0.0, 0.0, w, h).ok()?.into();
        let data: js_sys::Uint8ClampedArray = get(&image, "data").dyn_into().ok()?;
        self.px.resize(data.length() as usize, 0);
        data.copy_to(&mut self.px);
        Some(self.retina.look(&self.px, SIGHT_W as usize, SIGHT_H as usize))
    }

    pub fn close(mut self) {
        stop(&self.stream);
        self.video.set_src_object(None);
        self.retina.forget();
    }
}
