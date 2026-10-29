//! Two fingers on a trackpad, pinched: a squeeze, as two fingers on a screen are. The browser
//! tells it either as a wheel turned with ctrl held (most browsers), a little at a time, or as
//! a gesture with a scale from where it began (Safari). Either way the fingers have come this
//! much closer or further apart, and it is squeezed as far as fingers on a screen would squeeze
//! it. While the fingers keep moving it stays squeezed; once they stop it lets go.
//!
//! A turned wheel is still a squeeze too, a little each notch (`Game::wheel`); this is only the pinch.

/// A pinch told as a wheel: how far, in wheel units, closes the fingers to 1/e of how far apart they were.
const WHEEL_PER_E: f32 = 100.0;
/// No more than this much of a pinch in one wheel event, so ctrl held on a mouse wheel is not a slam.
const WHEEL_MOST: f32 = 25.0;
/// A pinch told as a wheel has no end: this long with nothing more and it is over.
pub const REST_SECS: f32 = 0.35;

/// How far fingers this much closer (below 1) or further apart (above 1) squeeze it, -1..1.
/// The same for fingers on a screen (`game/gesture.rs`) and on a trackpad.
pub fn squeeze(ratio: f32) -> f32 {
    ((ratio - 1.0) * 1.6).clamp(-1.0, 1.0)
}

/// Fingers on a screen are measured by how far they move too, not only by how much closer they
/// come: fingers that begin close together (a pinch of its skin) halve their distance with a twitch,
/// and should not flatten all of it. Moving this far (canvas heights) is as far as a ratio of 0.5.
const FINGERS_HALF: f32 = 0.25;

/// How far two fingers on a screen squeeze it, `start` and `now` apart, on a canvas `h` high:
/// by how much closer they came, but never further than by how far they moved. Fingers that begin
/// half the screen apart or more squeeze it by the ratio alone.
pub fn fingers(start: f32, now: f32, h: f32) -> f32 {
    let by_ratio = squeeze(now / start.max(1e-3));
    let by_reach = squeeze(1.0 + (now - start) / h.max(1.0) * 0.5 / FINGERS_HALF);
    if by_ratio.abs() < by_reach.abs() { by_ratio } else { by_reach }
}

#[derive(Default)]
pub struct Pad {
    /// How far apart the fingers are now, as a share of how far apart they began; none between pinches.
    scale: Option<f32>,
    /// Seconds since the browser last told of it.
    quiet: f32,
    /// Safari tells when a pinch ends; until it does, fingers held still keep it squeezed.
    held: bool,
}

impl Pad {
    /// A pinch told as a wheel with ctrl held: `dy` above 0 closes the fingers.
    pub fn wheel(&mut self, dy: f32) {
        let s = self.scale.unwrap_or(1.0);
        self.scale = Some(s * (-dy.clamp(-WHEEL_MOST, WHEEL_MOST) / WHEEL_PER_E).exp());
        self.quiet = 0.0;
    }

    /// A pinch told as a gesture: `scale` from where it began, until `None` when it ends.
    pub fn gesture(&mut self, scale: Option<f32>) {
        self.held = scale.is_some();
        self.scale = scale.filter(|s| s.is_finite() && *s > 0.0);
        self.quiet = 0.0;
    }

    /// Each frame: how far the trackpad squeezes it, while it does.
    pub fn step(&mut self, dt: f32) -> Option<f32> {
        self.quiet += dt;
        if !self.held && self.quiet > REST_SECS {
            self.scale = None;
        }
        self.scale.map(squeeze)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pinch_told_as_a_wheel_squeezes_while_it_moves_and_lets_go_once_it_stops() {
        let mut p = Pad::default();
        assert_eq!(p.step(0.016), None);
        // fingers closing to about half as far apart, told a little at a time
        for _ in 0..14 {
            p.wheel(5.0);
            p.step(0.016);
        }
        let s = p.step(0.016).unwrap();
        assert!(s < -0.6, "squeezed as far as fingers on a screen would: {s}");
        // spread back out
        for _ in 0..14 {
            p.wheel(-5.0);
        }
        assert!(p.step(0.016).unwrap().abs() < 0.01);
        p.wheel(-30.0);
        assert!(p.step(0.016).unwrap() > 0.0, "spread apart, it is pulled");
        for _ in 0..30 {
            p.step(0.016);
        }
        assert_eq!(p.step(0.016), None, "stopped, it lets go");
    }

    #[test]
    fn fingers_that_begin_close_together_squeeze_it_by_how_far_they_move() {
        // half the screen apart, closed to half: the ratio alone
        assert!((fingers(300.0, 150.0, 600.0) - squeeze(0.5)).abs() < 1e-5);
        assert!((fingers(600.0, 300.0, 600.0) - squeeze(0.5)).abs() < 1e-5);
        // a pinch of its skin, 60 to 20 pixels: a third of the distance, but only a twitch
        let s = fingers(60.0, 20.0, 600.0);
        assert!(s < 0.0 && s > -0.25, "{s}");
        // spread from close together, the same
        let s = fingers(60.0, 120.0, 600.0);
        assert!(s > 0.0 && s < 0.4, "{s}");
    }

    #[test]
    fn ctrl_on_a_mouse_wheel_is_not_a_slam() {
        let mut p = Pad::default();
        p.wheel(100.0);
        assert!(p.step(0.016).unwrap() > -0.4);
    }

    #[test]
    fn a_pinch_told_as_a_gesture_holds_until_it_ends() {
        let mut p = Pad::default();
        p.gesture(Some(0.5));
        for _ in 0..120 {
            assert_eq!(p.step(0.016), Some(-0.8));
        }
        p.gesture(None);
        assert_eq!(p.step(0.016), None);
    }
}
