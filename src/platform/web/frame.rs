//! What each frame starts from: the canvas size, waiting behind the loader, and where and how
//! fast a pointer moved.

use super::page::{body, fail};
use super::{App, LOADER_FRAMES};
use web_sys::PointerEvent;

/// A big screen at full density is millions of raymarched pixels; past this it renders smaller and scales up.
const MAX_PIXELS: f64 = 1.6e6;

impl App {
    /// Pointer position in canvas pixels.
    pub(super) fn pos(&self, e: &PointerEvent) -> (f32, f32) {
        let (sx, sy) = self.scale;
        ((e.offset_x() as f64 * sx) as f32, (e.offset_y() as f64 * sy) as f32)
    }

    /// Seconds since this pointer last moved (a frame, if it hasn't yet), and remember now.
    pub(super) fn moved(&mut self, id: i32, now: f64) -> f32 {
        match self.last_move.iter_mut().find(|p| p.0 == id) {
            Some(p) => ((now - std::mem::replace(&mut p.1, now)) / 1000.0).max(0.001) as f32,
            None => {
                self.last_move.push((id, now));
                1.0 / 60.0
            }
        }
    }

    pub(super) fn resize(&mut self) {
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
    pub(super) fn wake(&mut self) -> bool {
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
}
