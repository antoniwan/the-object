//! Moods each frame: upset and calming, color drifting while held still, leaning toward
//! whoever watches, and night when enough of it is free.

use super::{palette, Event, World, OPEN_AT};
use crate::math::*;

impl World {
    pub(super) fn step_moods(&mut self, dt: f32) {
        // Agitation: it shivers, then calms down.
        self.agitation = fade(self.agitation, -dt / 3.0);
        if self.agitation > 0.01 {
            let t = self.time;
            let shake = v3((t * 37.0).sin(), (t * 41.0).cos(), (t * 29.0).sin());
            self.body.vel += shake * (9.0 * self.agitation * dt);
        }

        // Warmth: held still, it drifts through colors.
        if self.warming {
            self.warm_phase += dt * 0.22;
            let goal = palette(self.warm_phase);
            self.body.color = self.body.color.lerp(goal, 1.0 - (-1.5 * dt).exp());
        }

        // Watching: it slowly grows toward whoever is looking.
        let goal = if self.watching { 1.0 } else { self.regard };
        let rate = if self.watching { 0.25 } else { 3.0 };
        self.gaze = ease(self.gaze, goal, -rate * dt);

        // Opening: enough of it free at once and the world turns to night.
        let loose = self.loose();
        if !self.opened && loose >= OPEN_AT {
            self.opened = true;
            self.events.push(Event::Opened);
        } else if self.opened && loose == 0 {
            self.opened = false;
            self.events.push(Event::Closed);
        }
        let goal = if self.opened { 1.0 } else { self.dim };
        self.night = ease(self.night, goal, -0.7 * dt);
    }
}
