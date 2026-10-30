//! How much of the screen is drawn. The page's resolution follows how fast frames come: a frame
//! that stalls lowers it at once, a second of slow frames lowers it a step, and a long run of
//! quick ones raises it a step again, never back past a level it already failed at, so it settles
//! instead of flickering between two. A gap too long to be a stall (a hidden tab, a sleeping
//! laptop) says nothing about speed and is ignored. Frames that come steadily at thirty a second
//! are a screen that has been told to save power (a phone in low power mode), not a page that
//! cannot keep up, and are let be. The canvas follows the chosen resolution in small steps, so
//! the screen sharpens and softens gradually instead of jumping. The finest light (`uFine`) follows the
//! resolution, eased, so it fades out and back in rather than switching in one frame.
//! Plain Rust, tested natively; the frame loop in `platform/web/` feeds it each frame's length.

/// The lowest fraction of the screen it will draw.
pub const FLOOR: f64 = 0.4;
/// Below this fraction the bent and focused light are left out.
pub const FINE_AT: f64 = 0.75;
/// The longest step the simulation is given, however long the frame took.
pub const MAX_DT: f64 = 1.0 / 20.0;

/// One frame this long is a stall: quality drops at once.
const STALL: f64 = 0.25;
/// Longer than this, nobody was watching: the browser stopped sending frames.
const PAUSE: f64 = 1.0;
const STALL_DROP: f64 = 0.25;
/// Frames slower than this, for this long, drop a step.
const SLOW: f64 = 1.0 / 35.0;
const SLOW_FOR: f64 = 1.0;
const SLOW_DROP: f64 = 0.15;
/// Frames quicker than this, for this long in all, raise a step.
const FAST: f64 = 1.0 / 50.0;
const FAST_FOR: f64 = 8.0;
const RISE: f64 = 0.1;
/// A drop this soon after a rise means the rise was too much: that level becomes the ceiling.
const RISE_TRIAL: f64 = 10.0;
/// How fast the finest light follows (per second).
const FINE_RATE: f64 = 1.2;
/// A screen capped at this frame length, give or take CAP_NEAR, with frames this regular
/// (mean change from one frame to the next), is capped, not struggling.
const CAP: f64 = 1.0 / 30.0;
const CAP_NEAR: f64 = 0.0025;
const CAP_STEADY: f64 = 0.0015;
/// Over how long (seconds) the unevenness is averaged.
const CAP_WINDOW: f64 = 0.3;
/// Unevenness assumed before anything is known, so a cap must prove itself.
const CAP_DOUBT: f64 = 0.005;
/// How fast the drawn resolution follows the chosen one, down and up (per second), and the
/// step it moves the canvas by.
const SHOWN_DOWN: f64 = 0.4;
const SHOWN_UP: f64 = 0.15;
const SHOWN_STEP: f64 = 0.025;

#[derive(Clone, Debug)]
pub struct Pace {
    /// Fraction of device pixels drawn, FLOOR..1.
    pub quality: f64,
    /// How much of the finest light is drawn, 0..1, eased.
    pub fine: f32,
    ceiling: f64,
    slow: f64,
    fast: f64,
    /// The level it last rose from, and how long ago.
    rose_from: Option<(f64, f64)>,
    /// The resolution drawn, following `quality` gradually.
    shown: f64,
    /// The last frame's length, and how much it changes from frame to frame, averaged over the
    /// last CAP_WINDOW.
    last: Option<f64>,
    uneven: f64,
}

impl Default for Pace {
    fn default() -> Self {
        Pace {
            quality: 1.0,
            fine: 1.0,
            ceiling: 1.0,
            slow: 0.0,
            fast: 0.0,
            rose_from: None,
            shown: 1.0,
            last: None,
            uneven: CAP_DOUBT,
        }
    }
}

impl Pace {
    /// `raw` is the seconds since the last frame; `judge` is false while a frame says nothing
    /// about speed (the first ones after waking). Returns the step the simulation takes.
    pub fn frame(&mut self, raw: f64, judge: bool) -> f64 {
        let dt = raw.clamp(0.0, MAX_DT);
        if let Some((_, since)) = &mut self.rose_from {
            *since += dt;
        }
        if !judge || raw > PAUSE {
            // Nothing learned; and what was being counted no longer runs on.
            self.slow = 0.0;
            self.fast = 0.0;
        } else if raw > STALL {
            self.drop(STALL_DROP);
        } else if self.capped(raw) {
            // Held to thirty a second by the screen: as fast as it will go here.
            self.slow = (self.slow - dt).max(0.0);
        } else if dt > SLOW {
            self.slow += dt;
            self.fast = 0.0;
            if self.slow > SLOW_FOR {
                self.drop(SLOW_DROP);
            }
        } else {
            self.slow = (self.slow - dt).max(0.0);
            if dt < FAST {
                self.fast += dt;
                if self.fast > FAST_FOR && self.quality < self.ceiling {
                    self.rose_from = Some((self.quality, 0.0));
                    self.quality = round(self.quality + RISE).min(self.ceiling);
                    self.fast = 0.0;
                }
            }
        }
        let gap = self.quality - self.shown;
        let rate = if gap < 0.0 { SHOWN_DOWN } else { SHOWN_UP };
        self.shown += gap.clamp(-rate * dt, rate * dt);
        let goal = if self.quality >= FINE_AT - 1e-6 { 1.0 } else { 0.0 };
        self.fine += ((goal - self.fine as f64) * (1.0 - (-FINE_RATE * dt).exp())) as f32;
        dt
    }

    /// The fraction of the screen to draw now: `quality`, reached in small steps.
    pub fn scale(&self) -> f64 {
        (self.shown / SHOWN_STEP).round() * SHOWN_STEP
    }

    /// Learns this frame's length; true while frames come steadily at the screen's cap.
    fn capped(&mut self, raw: f64) -> bool {
        let change = self.last.map_or(CAP_DOUBT, |last| (raw - last).abs());
        self.last = Some(raw);
        self.uneven += (change - self.uneven) * (1.0 - (-raw / CAP_WINDOW).exp());
        (raw - CAP).abs() < CAP_NEAR && self.uneven < CAP_STEADY
    }

    /// The GPU gave up the context: ask less of it, and never as much again.
    pub fn lost(&mut self) {
        self.quality = round(self.quality * 0.6).max(FLOOR);
        // everything on the GPU is new anyway
        self.shown = self.quality;
        self.ceiling = self.ceiling.min(self.quality);
        self.rose_from = None;
    }

    fn drop(&mut self, by: f64) {
        let failed = self.rose_from.take().filter(|&(_, since)| since < RISE_TRIAL);
        self.quality = match failed {
            // Back to where it kept up, and no higher from now on.
            Some((from, _)) => {
                self.ceiling = from;
                from
            }
            None => round(self.quality - by).max(FLOOR),
        };
        self.slow = 0.0;
        self.fast = 0.0;
    }
}

/// To hundredths, so steps up and down land on the same levels.
fn round(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame length around `mean`, off by up to `by` either way, the same every run.
    fn uneven(k: usize, mean: f64, by: f64) -> f64 {
        let h = ((k as f64 * 12.9898).sin() * 43758.5453).fract().abs();
        mean + (h * 2.0 - 1.0) * by
    }

    fn run(p: &mut Pace, dt: f64, secs: f64) {
        for _ in 0..(secs / dt).round() as usize {
            p.frame(dt, true);
        }
    }

    #[test]
    fn a_hidden_tab_is_not_a_stall() {
        let mut p = Pace::default();
        for _ in 0..5 {
            run(&mut p, 1.0 / 60.0, 1.0);
            p.frame(4.0, true);
        }
        assert_eq!(p.quality, 1.0);
        assert_eq!(p.fine, 1.0);
    }

    #[test]
    fn a_stall_drops_at_once_and_quick_frames_bring_it_back() {
        let mut p = Pace::default();
        run(&mut p, 1.0 / 60.0, 1.0);
        p.frame(0.4, true);
        assert_eq!(p.quality, 0.75);
        run(&mut p, 1.0 / 60.0, 9.0);
        assert!(p.quality > 0.75 && p.quality <= 1.0, "{}", p.quality);
        run(&mut p, 1.0 / 60.0, 20.0);
        assert_eq!(p.quality, 1.0);
    }

    #[test]
    fn nothing_is_judged_while_waking() {
        let mut p = Pace::default();
        p.frame(0.6, false);
        assert_eq!(p.quality, 1.0);
    }

    #[test]
    fn slow_frames_step_it_down_to_the_floor() {
        let mut p = Pace::default();
        run(&mut p, 1.0 / 25.0, 30.0);
        assert_eq!(p.quality, FLOOR);
        assert!(p.fine < 0.01);
    }

    #[test]
    fn a_rise_that_fails_is_not_tried_again() {
        // A machine that keeps up at 0.6 but not at 0.7.
        let mut p = Pace::default();
        let mut levels = Vec::new();
        for k in 0..(120 * 60) {
            let dt = if p.quality > 0.62 { uneven(k, 1.0 / 30.0, 0.008) } else { 1.0 / 60.0 };
            p.frame(dt, true);
            if levels.last() != Some(&p.quality) {
                levels.push(p.quality);
            }
        }
        assert_eq!(p.quality, 0.55, "{levels:?}");
        // Down to where it keeps up, one try a step up, then it stays.
        assert!(levels.len() <= 6, "it keeps changing: {levels:?}");
    }

    #[test]
    fn the_finest_light_fades_rather_than_switches() {
        let mut p = Pace::default();
        p.frame(0.4, true);
        p.frame(1.0 / 60.0, true);
        run(&mut p, 1.0 / 25.0, 1.0);
        let mut last = p.fine;
        let mut most = 0.0f32;
        for _ in 0..240 {
            p.frame(1.0 / 25.0, true);
            most = most.max((last - p.fine).abs());
            last = p.fine;
        }
        assert!(p.quality < FINE_AT);
        assert!(p.fine < 0.05, "{}", p.fine);
        assert!(most < 0.06, "the finest light jumped {most} in one frame");
    }

    #[test]
    fn a_lost_context_lowers_the_ceiling() {
        let mut p = Pace::default();
        p.lost();
        assert_eq!(p.quality, 0.6);
        run(&mut p, 1.0 / 60.0, 60.0);
        assert_eq!(p.quality, 0.6);
    }

    #[test]
    fn a_screen_held_to_thirty_is_let_be() {
        // Low power mode: thirty a second, within a fraction of a millisecond.
        let mut p = Pace::default();
        for k in 0..(60 * 30) {
            p.frame(uneven(k, 1.0 / 30.0, 0.0004), true);
        }
        assert_eq!(p.quality, 1.0);
        assert_eq!(p.fine, 1.0);
    }

    #[test]
    fn a_screen_that_starts_saving_power_partway_is_let_be() {
        let mut p = Pace::default();
        run(&mut p, 1.0 / 60.0, 10.0);
        for k in 0..(60 * 30) {
            p.frame(uneven(k, 1.0 / 30.0, 0.0004), true);
        }
        assert_eq!(p.quality, 1.0);
    }

    #[test]
    fn a_page_that_cannot_keep_up_at_about_thirty_still_steps_down() {
        let mut p = Pace::default();
        for k in 0..(60 * 30) {
            p.frame(uneven(k, 1.0 / 30.0, 0.009), true);
        }
        assert_eq!(p.quality, FLOOR);
    }

    #[test]
    fn the_canvas_follows_in_small_steps() {
        let mut p = Pace::default();
        run(&mut p, 1.0 / 60.0, 1.0);
        p.frame(0.4, true);
        assert_eq!(p.quality, 0.75);
        let (mut last, mut changes) = (p.scale(), 0);
        for _ in 0..120 {
            p.frame(1.0 / 60.0, true);
            let s = p.scale();
            assert!((s - last).abs() <= SHOWN_STEP + 1e-9, "the canvas jumped {last} to {s}");
            changes += (s != last) as u32;
            last = s;
        }
        assert!((p.scale() - 0.75).abs() < 1e-9, "{}", p.scale());
        assert!(changes >= 8, "{changes}");
    }
}
