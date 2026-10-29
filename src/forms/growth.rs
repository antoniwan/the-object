//! What grows out of the mound (dust.rs). Once the mound has settled it takes root on its
//! own: a shoot comes up out of its top. From there it grows only as far as it is tended.
//! Every tap on the mound or on what is growing from it brings it on a little: a stem, then
//! a trunk, then branches, then leaves, and at the very end the flower it once was comes
//! back in blossom along them. Leave it alone and it slowly draws back down, never below
//! the shoot. Hold the mound and it all goes back in as the liquid rises (dust.rs).
//!
//! `shown` is what the shader draws, 0..4: 0 nothing, 1 a shoot, 2 a trunk, 3 branches,
//! 4 in leaf and blossom. It eases toward `tended`, so growing is something you watch.

use super::Event;

/// Where it gets to on its own: a shoot.
pub const ROOT: f32 = 0.6;
pub const FULL: f32 = 4.0;
/// How far one tap brings it on.
pub const TEND: f32 = 0.35;
/// How fast what is drawn follows what it has been given, per second.
const GROW_RATE: f32 = 0.3;
/// Untended this long, it starts to draw back, this much a second.
pub const WITHER_AFTER: f32 = 30.0;
const WITHER_RATE: f32 = 0.05;

#[derive(Default)]
pub struct Growth {
    pub shown: f32,
    pub tended: f32,
    /// Seconds since it was last tended.
    pub quiet: f32,
    peak: f32,
    rooted: bool,
    full: bool,
    withered: bool,
}

impl Growth {
    /// A tap: it grows a little further. Returns how far along it is now, 0..1.
    pub fn tend(&mut self) -> f32 {
        self.tended = (self.tended.max(ROOT) + TEND).min(FULL);
        self.quiet = 0.0;
        self.tended / FULL
    }

    /// Height of the whole of it above the top of the mound, in world units, for touching it.
    /// The shader's tree reaches about this far.
    pub fn reach(&self) -> f32 {
        let g = self.shown;
        0.15 + 2.3 * crate::math::smoothstep(0.0, 2.5, g) + 0.9 * crate::math::smoothstep(2.6, 3.6, g)
    }

    /// How wide its branches spread from the trunk.
    pub fn spread(&self) -> f32 {
        0.15 + 1.2 * crate::math::smoothstep(1.0, 3.0, self.shown)
    }

    /// While the mound stands: it takes root, follows the tending, and withers when left alone.
    pub fn step(&mut self, dt: f32, events: &mut Vec<Event>) {
        self.quiet += dt;
        self.tended = self.tended.max(ROOT);
        if self.quiet > WITHER_AFTER {
            self.tended = (self.tended - WITHER_RATE * dt).max(ROOT);
        }
        self.ease(dt);
        self.peak = self.peak.max(self.shown);
        if !self.rooted && self.shown > 0.5 {
            self.rooted = true;
            events.push(Event::Rooted);
        }
        if !self.full && self.shown > FULL - 0.1 {
            self.full = true;
            events.push(Event::Canopy);
        }
        if !self.withered && self.peak >= 2.0 && self.shown < self.peak - 1.0 {
            self.withered = true;
            events.push(Event::Withered);
        }
    }

    /// The mound going back in: what grew from it goes first.
    pub fn recede(&mut self, dt: f32) {
        self.tended = 0.0;
        self.shown = (self.shown - dt * 3.0).max(0.0);
    }

    fn ease(&mut self, dt: f32) {
        let d = self.tended - self.shown;
        self.shown += d.signum() * d.abs().min(GROW_RATE * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(g: &mut Growth, secs: f32, ev: &mut Vec<Event>) {
        for _ in 0..(secs * 30.0) as usize {
            g.step(1.0 / 30.0, ev);
        }
    }

    #[test]
    fn it_roots_by_itself_grows_as_far_as_it_is_tended_and_withers_back_to_a_shoot() {
        let (mut g, mut ev) = (Growth::default(), Vec::new());
        run(&mut g, 4.0, &mut ev);
        assert!(ev.contains(&Event::Rooted) && (g.shown - ROOT).abs() < 1e-3);
        for _ in 0..12 {
            g.tend();
            run(&mut g, 1.0, &mut ev);
        }
        run(&mut g, 4.0, &mut ev);
        assert!(ev.contains(&Event::Canopy), "{}", g.shown);
        run(&mut g, WITHER_AFTER + 80.0, &mut ev);
        assert!(ev.contains(&Event::Withered));
        assert!((g.shown - ROOT).abs() < 1e-3);
    }
}
