//! The microphone and the camera, once the player has tapped its word and the browser has let
//! it have them. Neither is ever heard or seen by anyone: each frame the microphone gives a short
//! window of samples and the camera a picture 32 x 24 pixels wide, senses.rs turns them into a
//! few numbers, and they are dropped. Nothing is recorded, kept or sent. Closing stops every
//! track at once, so the browser's own indicator goes out with ours.

use crate::senses::{self, Heard, Kind, Retina, Seen};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, MediaStream, MediaStreamConstraints};

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

fn get(target: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

/// Whether this page could ever be given it: a secure page with media devices, and not inside a
/// frame that forbids it (itch.io embeds the object in one that does).
pub fn possible(kind: Kind) -> bool {
    let Some(w) = web_sys::window() else { return false };
    if !w.is_secure_context() || get(&w.navigator(), "mediaDevices").is_undefined() {
        return false;
    }
    let feature = match kind {
        Kind::Hear => "microphone",
        Kind::See => "camera",
    };
    let doc: JsValue = w.document().map(Into::into).unwrap_or(JsValue::UNDEFINED);
    let mut policy = get(&doc, "permissionsPolicy");
    if policy.is_undefined() || policy.is_null() {
        policy = get(&doc, "featurePolicy");
    }
    if let Ok(allows) = get(&policy, "allowsFeature").dyn_into::<js_sys::Function>() {
        return allows.call1(&policy, &JsValue::from_str(feature)).map(|v| v.is_truthy()).unwrap_or(false);
    }
    // No way to ask: only the top of the page is trusted to have it.
    w.top().ok().flatten().is_some_and(|top| JsValue::from(top) == JsValue::from(w.clone()))
}

/// Asks the browser. Must be called inside the player's tap. The answer is pushed to `answers`.
pub fn ask(kind: Kind, answers: Rc<RefCell<Vec<Answer>>>) {
    let c = MediaStreamConstraints::new();
    match kind {
        // Raw, so a whisper stays a whisper and a shout a shout (phones flatten both otherwise).
        // It never hears its own hum: it does not listen while it hums (senses.rs).
        Kind::Hear => {
            c.set_audio(&obj(&[
                ("echoCancellation", false.into()),
                ("noiseSuppression", false.into()),
                ("autoGainControl", false.into()),
            ]));
            c.set_video(&false.into());
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
    let ok = Closure::once(move |s: JsValue| yes.borrow_mut().push((kind, s.dyn_into::<MediaStream>().map_err(|_| false))));
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
        Ok(Ears { ctx, analyser, stream, buf: vec![0.0; 2048] })
    }

    /// Touch screens only grant audio inside a gesture, so this is retried on every release.
    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn listen(&mut self) -> Heard {
        self.analyser.get_float_time_domain_data(&mut self.buf);
        senses::hear(&self.buf, self.ctx.sample_rate())
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
        Ok(Eyes { video, sight, stream, retina: Retina::default(), last: 0.0 })
    }

    /// A look, at most SIGHT_RATE times a second, once the camera has a picture.
    pub fn look(&mut self, now: f64) -> Option<Seen> {
        if now - self.last < 1000.0 / SIGHT_RATE || self.video.ready_state() < 2 {
            return None;
        }
        self.last = now;
        let (w, h) = (SIGHT_W as f64, SIGHT_H as f64);
        self.sight.draw_image_with_html_video_element_and_dw_and_dh(&self.video, 0.0, 0.0, w, h).ok()?;
        let px = self.sight.get_image_data(0.0, 0.0, w, h).ok()?.data();
        Some(self.retina.look(&px, SIGHT_W as usize, SIGHT_H as usize))
    }

    pub fn close(mut self) {
        stop(&self.stream);
        self.video.set_src_object(None);
        self.retina.forget();
    }
}
