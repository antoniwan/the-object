//! Slice it and it divides. A fast swipe clean through the body cuts it; it
//! pinches along the cut, parts into two cells, and becomes something alive.
//! Squeeze the two cells long enough and they become one liquid again.
//!
//! `Phase` is a tiny state machine, but it is not where the smoothness comes
//! from: each phase only drives a few 0..1 values (`Blend`) that the shader
//! mixes continuously, so the shape and material never jump.

use crate::math::*;

/// 2^(-1/3): each of two cells holding half the volume.
pub const CELL_SCALE: f32 = 0.7937;
/// Each cell's center sits this many body radii from the middle once divided.
pub const SEP: f32 = 0.95;
/// Smoothing radius of the join while it is still one body, in body radii.
pub const K_JOIN: f32 = 0.9;
/// Seconds from the cut until the cells have settled.
pub const DIVIDE_SECS: f32 = 3.0;
/// Seconds for two cells to flow back into one.
pub const REJOIN_SECS: f32 = 2.2;
/// How long the time-freeze lasts right after the cut, in real seconds.
pub const HIT_STOP_SECS: f32 = 0.12;
/// A swipe must cross the body this fast to count as a slice.
pub const SLICE_MAX_SECS: f32 = 0.35;
/// Holding a squeeze this long presses two cells back into one.
const REJOIN_SQUEEZE_SECS: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Whole,
    Dividing { t: f32 },
    Two,
    Rejoining { t: f32 },
}

/// The continuous values the shader blends. Every transition only moves these.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Blend {
    /// Glow of the fresh cut.
    pub flash: f32,
    /// Cleavage furrow pinching the equator.
    pub furrow: f32,
    /// 0 = one body, 1 = two separate cells.
    pub split: f32,
    /// Material: 0 = liquid, 1 = living cell.
    pub cell: f32,
    /// Camera push-in so the cells fill the screen.
    pub zoom: f32,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn dividing(t: f32) -> Blend {
    Blend {
        flash: (-t * 5.0).exp(),
        furrow: smoothstep(0.0, 0.55, t),
        split: smoothstep(0.45, 1.6, t),
        cell: smoothstep(0.3, 2.0, t),
        zoom: smoothstep(0.8, DIVIDE_SECS, t),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Divided,
    Rejoined,
}

pub struct Division {
    pub phase: Phase,
    /// Normal of the cut; the cells part along it.
    pub axis: V3,
    squeezed_for: f32,
    events: Vec<Event>,
}

impl Default for Division {
    fn default() -> Self {
        Division { phase: Phase::Whole, axis: v3(1.0, 0.0, 0.0), squeezed_for: 0.0, events: Vec::new() }
    }
}

impl Division {
    pub fn whole(&self) -> bool {
        self.phase == Phase::Whole
    }

    pub fn blend(&self) -> Blend {
        match self.phase {
            Phase::Whole => Blend::default(),
            Phase::Dividing { t } => dividing(t),
            Phase::Two => dividing(DIVIDE_SECS),
            // Run the division backwards, without replaying the cut.
            Phase::Rejoining { t } => {
                let back = DIVIDE_SECS * (1.0 - smoothstep(0.0, REJOIN_SECS, t));
                Blend { flash: 0.0, ..dividing(back) }
            }
        }
    }

    /// Cut along `axis`. Only a whole body can be cut.
    pub fn slice(&mut self, axis: V3) -> bool {
        if !self.whole() {
            return false;
        }
        self.axis = axis.norm();
        self.phase = Phase::Dividing { t: 0.0 };
        true
    }

    /// `squeeze` is the player's squeeze input (-1..1); held hard enough on two
    /// cells, it presses them back together.
    pub fn step(&mut self, dt: f32, squeeze: f32) {
        self.phase = match self.phase {
            Phase::Dividing { t } if t + dt >= DIVIDE_SECS => {
                self.events.push(Event::Divided);
                Phase::Two
            }
            Phase::Dividing { t } => Phase::Dividing { t: t + dt },
            Phase::Two => {
                self.squeezed_for = if squeeze.abs() > 0.4 { self.squeezed_for + dt } else { 0.0 };
                if self.squeezed_for > REJOIN_SQUEEZE_SECS {
                    self.squeezed_for = 0.0;
                    Phase::Rejoining { t: 0.0 }
                } else {
                    Phase::Two
                }
            }
            Phase::Rejoining { t } if t + dt >= REJOIN_SECS => {
                self.events.push(Event::Rejoined);
                Phase::Whole
            }
            Phase::Rejoining { t } => Phase::Rejoining { t: t + dt },
            Phase::Whole => Phase::Whole,
        };
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Centers of the two cells and each cell's radius, for a body at `pos` with radius `r`.
    /// While whole both centers coincide; the radius is shrunk by the join's bulge so the
    /// joined shape still has radius `r`.
    pub fn cells(&self, pos: V3, r: f32) -> ([V3; 2], f32) {
        let s = self.blend().split;
        let off = self.axis * (SEP * r * s);
        let rc = r * ((1.0 - K_JOIN * 0.25) + (CELL_SCALE - (1.0 - K_JOIN * 0.25)) * s);
        ([pos - off, pos + off], rc)
    }

    /// Distance to the body, given its center, radius, and squash scale.
    pub fn body_sdf(&self, p: V3, pos: V3, r: f32, scale: V3) -> f32 {
        let lip = scale.x.min(scale.y);
        let split = self.blend().split;
        let ([a, b], rc) = self.cells(pos, r);
        // Squash about the body center, not each cell's, so it stays one gesture.
        let q = (p - pos).div(scale) + pos;
        let da = (q - a).len() - rc;
        if split < 1e-4 {
            return (da - K_JOIN * r * 0.25) * lip;
        }
        let db = (q - b).len() - rc;
        smin(da, db, (K_JOIN * r * (1.0 - split)).max(1e-4)) * lip
    }

    /// Ripples live in the first cell's frame and are mirrored across the cut
    /// onto its twin. Returns the direction to store for a touch at `p`.
    pub fn ripple_dir(&self, p: V3, pos: V3, r: f32) -> V3 {
        let (_, center) = self.nearest_cell(p, pos, r);
        let dir = (p - center).norm();
        if self.whole() || (center - pos).dot(self.axis) <= 0.0 {
            dir
        } else {
            dir - self.axis * (2.0 * dir.dot(self.axis))
        }
    }

    /// Distance from `p` to the nearest cell's surface (including the join's bulge
    /// while still joined), and that cell's center.
    pub fn nearest_cell(&self, p: V3, pos: V3, r: f32) -> (f32, V3) {
        let ([a, b], rc) = self.cells(pos, r);
        let surface = rc + K_JOIN * r * 0.25 * (1.0 - self.blend().split);
        let (da, db) = ((p - a).len(), (p - b).len());
        if da <= db { (da - surface, a) } else { (db - surface, b) }
    }
}

/// Watches a drag that started off the body, and reports a slice when it
/// passes clean through the body fast enough.
#[derive(Default)]
pub struct Swipe {
    inside: bool,
    entry: Option<(f32, f32, f32)>,
}

impl Swipe {
    /// Feed one pointer sample: time, canvas position, and whether it is over
    /// the body. Returns the swipe's screen direction when it completes a slice.
    pub fn sample(&mut self, time: f32, x: f32, y: f32, over_body: bool, min_px: f32) -> Option<(f32, f32)> {
        let mut out = None;
        if over_body && !self.inside {
            self.entry = Some((time, x, y));
        } else if !over_body && self.inside {
            if let Some((t0, x0, y0)) = self.entry.take() {
                let (dx, dy) = (x - x0, y - y0);
                if time - t0 < SLICE_MAX_SECS && (dx * dx + dy * dy).sqrt() > min_px {
                    out = Some((dx, dy));
                }
            }
        }
        self.inside = over_body;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_body_keeps_its_radius() {
        let d = Division::default();
        let s = v3(1.0, 1.0, 1.0);
        assert!(d.body_sdf(v3(1.0, 0.0, 0.0), V3::ZERO, 1.0, s).abs() < 1e-4);
    }

    #[test]
    fn dividing_ends_with_two_separate_cells() {
        let mut d = Division::default();
        assert!(d.slice(v3(1.0, 0.0, 0.0)));
        for _ in 0..400 {
            d.step(1.0 / 120.0, 0.0);
        }
        assert_eq!(d.phase, Phase::Two);
        assert_eq!(d.drain_events(), vec![Event::Divided]);
        let s = v3(1.0, 1.0, 1.0);
        // the middle is now empty space, and each cell's far side sits where expected
        assert!(d.body_sdf(V3::ZERO, V3::ZERO, 1.0, s) > 0.1);
        let edge = SEP + CELL_SCALE;
        assert!(d.body_sdf(v3(edge, 0.0, 0.0), V3::ZERO, 1.0, s).abs() < 1e-3);
        // the two cells together hold the body's volume
        let (_, rc) = d.cells(V3::ZERO, 1.0);
        assert!((2.0 * rc.powi(3) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn a_held_squeeze_rejoins_them() {
        let mut d = Division::default();
        d.slice(v3(0.0, 0.0, 1.0));
        for _ in 0..400 {
            d.step(1.0 / 120.0, 0.0);
        }
        for _ in 0..500 {
            d.step(1.0 / 120.0, 0.8);
        }
        assert_eq!(d.phase, Phase::Whole);
        assert!(d.drain_events().contains(&Event::Rejoined));
        assert_eq!(d.blend(), Blend::default());
    }

    #[test]
    fn a_fast_swipe_through_slices_and_a_slow_one_does_not() {
        let mut s = Swipe::default();
        assert_eq!(s.sample(0.0, 0.0, 0.0, false, 30.0), None);
        assert_eq!(s.sample(0.05, 40.0, 0.0, true, 30.0), None);
        assert_eq!(s.sample(0.15, 120.0, 0.0, false, 30.0), Some((80.0, 0.0)));

        let mut s = Swipe::default();
        s.sample(0.0, 40.0, 0.0, true, 30.0);
        assert_eq!(s.sample(1.0, 120.0, 0.0, false, 30.0), None);
    }
}
