//! What the forms, the landscape and the cells report each frame, and what the game does about it.

use super::*;

/// How many beats it keeps after the river, each fainter than the last.
const HEART_BEATS: u32 = 40;

impl Game {
    /// What the forms ask of the game each frame: their news, the camera, and the box's contents.
    pub(super) fn update_forms(&mut self, dt: f32, orbiting: bool) {
        use forms::Event as F;
        self.world.forms.spin.glad = self.remembrance.kind();
        for e in self.world.forms.drain_events() {
            match e {
                F::Budded => {
                    self.sounds.push(Sound::Pinch);
                    self.discover(Discovery::Pinch);
                }
                F::Bloomed => {
                    self.sounds.push(Sound::Bloom);
                    self.discover(Discovery::Bloom);
                }
                F::Folded => {
                    self.world.add_ripple(V3::UP, 0.7);
                    self.discover(Discovery::Fold);
                }
                F::Threaded => self.discover(Discovery::Thread),
                F::Plucked { harmonic } => {
                    self.sounds.push(Sound::Pluck { harmonic });
                    self.discover(Discovery::Pluck);
                }
                F::Wound => {
                    self.world.add_ripple(self.world.forms.filament.axis, 0.8);
                    self.world.add_ripple(-self.world.forms.filament.axis, 0.8);
                    self.discover(Discovery::Wind);
                }
                F::Stirring => {
                    self.sounds.push(Sound::Stir);
                    self.discover(Discovery::Stir);
                }
                F::Cosmos => {
                    self.sounds.push(Sound::Become);
                    self.discover(Discovery::Cosmos);
                }
                F::Settled => {
                    self.world.add_ripple(V3::UP, 0.6);
                    self.discover(Discovery::Settle);
                }
                F::Cornered => {
                    self.sounds.push(Sound::Become);
                    self.discover(Discovery::Corners);
                }
                F::Knocked => {
                    self.sounds.push(Sound::Knock);
                    self.discover(Discovery::Knock);
                }
                F::Lid => {
                    self.sounds.push(Sound::Lid);
                    self.cam.shake = 0.04;
                    self.discover(Discovery::Lid);
                }
                F::Rounded => {
                    self.world.ripple_both(0.8);
                    self.discover(Discovery::Round);
                }
                F::Everywhere => {
                    self.sounds.push(Sound::Become);
                    self.cam.shake = 0.05;
                    self.discover(Discovery::Everywhere);
                }
                F::Facet => {
                    self.sounds.push(Sound::Chord);
                    self.discover(Discovery::Facet);
                }
                F::Backward => {
                    self.sounds.push(Sound::Stir);
                    self.discover(Discovery::Backward);
                }
                F::Crease => {
                    self.world.add_ripple(V3::UP, 0.7);
                    self.discover(Discovery::Crease);
                }
                F::Crumbled => {
                    self.sounds.push(Sound::Become);
                    self.cam.shake = 0.05;
                    self.discover(Discovery::Dust);
                }
                F::Swept => {}
                F::Heaped => self.sounds.push(Sound::Become),
                F::Grounded => {
                    self.cam.shake = 0.03;
                    self.discover(Discovery::Mound);
                }
                F::Unearthed => {
                    self.world.ripple_both(0.8);
                    self.discover(Discovery::Unearth);
                }
                F::Rooted => {
                    self.world.forms.dust.thud = 0.5;
                    self.discover(Discovery::Root);
                }
                F::Canopy => {
                    self.sounds.push(Sound::Bloom);
                    self.discover(Discovery::Canopy);
                }
                F::Withered => self.discover(Discovery::Wither),
                F::Swaying => self.discover(Discovery::Sway),
                F::Nearly => self.sounds.push(Sound::Almost),
                F::Learned { with } => {
                    self.world.add_ripple(V3::UP, 0.5);
                    self.discover(if with { Discovery::InStep } else { Discovery::Braced });
                }
                F::Winding | F::Whirling => self.sounds.push(Sound::Become),
                F::Flung => {
                    self.sounds.push(Sound::Chord);
                    self.cam.shake = 0.06;
                    self.discover(Discovery::Ropes);
                }
                F::LaidDown => {}
                F::Unwound => self.world.ripple_both(0.8),
                F::Aloft => {
                    self.sounds.push(Sound::Stir);
                    self.discover(Discovery::Aloft);
                }
                F::Landed => {
                    self.cam.shake = 0.02;
                    self.world.add_ripple(V3::UP, 0.6);
                }
            }
        }
        // Everything it was keeping gets out at once.
        if self.world.forms.pandora.spill() {
            let r = self.world.body.radius();
            let top = self.world.body.pos + V3::UP * (pandora::HALF_SIDE * r);
            for i in 0..7 {
                let a = i as f32 * 2.399 + self.world.time;
                let out = v3(a.cos(), 0.0, a.sin());
                let vel = V3::UP * (3.2 + 0.4 * (i % 3) as f32) + out * 1.6;
                self.world.spill(top + out * (0.2 * r), vel, 0.24 * r, crate::sim::palette(i as f32 / 7.0));
            }
        }
        // After a while the open box wants them back (at night, left alone, they would only circle),
        // and once it has all come home, it can be round again.
        if let pandora::Phase::Open { t } = self.world.forms.pandora.phase {
            let home = self.world.body.pos;
            let pull = 0.25 * ((t - 6.0) / 10.0).clamp(0.0, 1.0);
            for d in self.world.drops.iter_mut().filter(|d| d.target.is_none()) {
                d.vel += (home - d.pos).norm() * (pull * dt);
            }
            if self.world.loose() == 0 {
                self.world.forms.pandora.round();
            }
        }
        if self.look_back && !orbiting {
            let k = 1.0 - (-2.5 * dt).exp();
            self.cam.yaw += (self.look_from.0 - self.cam.yaw) * k;
            self.cam.pitch += (self.look_from.1 - self.cam.pitch) * k;
            // Back where it was, it lets go, so whatever frames it next is not pulled against;
            // and it gives way at once to the floor, the landscape or the cells, which frame it themselves.
            let home =
                (self.look_from.0 - self.cam.yaw).abs() < 1e-3 && (self.look_from.1 - self.cam.pitch).abs() < 1e-3;
            let w = &self.world;
            if home || !(w.fusion.whole() && w.burst.whole() && w.division.whole()) {
                self.look_back = false;
            }
        }
        // The camera rises to look into the flower, and stays up to look down on the dust.
        let open = self.world.forms.bloom.open().max(self.world.forms.dust.view());
        if open > 0.0 && !orbiting {
            // as what roots in the mound grows, the eye comes down to see it standing
            let level = 0.78 - 0.3 * self.world.forms.dust.growth.shown / forms::growth::FULL;
            // whirling up, it comes down low enough to see the floor let go
            let level = level - 0.55 * self.world.forms.spin.flat();
            self.cam.pitch += (level - self.cam.pitch) * (1.0 - (-1.5 * dt).exp()) * open;
        }
    }

    pub(super) fn on_land(&mut self, e: burst::landscape::Event) {
        use crate::burst::landscape::Event as L;
        match e {
            L::Chord | L::Planted => {}
            L::Grown => self.discover(Discovery::Grow),
            L::Weathered => self.discover(Discovery::Weather),
            L::Eroded => self.gesture = Gesture::Idle,
            L::Mended { last } => {
                // it comes back in the color of the last piece home
                let tint = burst::landscape::TINTS[last];
                self.world.body.color = crate::sim::PEARL.lerp(tint, 0.65);
                self.sounds.push(Sound::Chord);
                self.discover(Discovery::Mend);
                self.gesture = Gesture::Idle;
            }
        }
    }

    pub(super) fn on_kin(&mut self, e: burst::kin::Event) {
        use crate::burst::kin::Event as K;
        match e {
            K::Joined => {
                self.sounds.push(Sound::Merge);
                self.discover(Discovery::Kin);
            }
            K::Loosed => {
                self.sounds.push(Sound::Tear);
                self.discover(Discovery::Loose);
            }
            K::Passed => self.discover(Discovery::Pass),
            K::Felt { i } => self.sounds.push(Sound::Shard { i }),
            K::Many => self.discover(Discovery::Many),
        }
    }

    pub(super) fn on_river(&mut self, e: burst::river::Event) {
        use crate::burst::river::Event as R;
        match e {
            R::Bled => {
                self.sounds.push(Sound::Become);
                self.discover(Discovery::River);
                self.gesture = Gesture::Idle;
            }
            R::Raised => self.discover(Discovery::Column),
            R::Filled => {
                self.sounds.push(Sound::Beat);
                self.cam.shake = self.cam.shake.max(0.03);
                self.discover(Discovery::Pulse);
            }
            R::Beat => {
                self.sounds.push(Sound::Beat);
                self.cam.shake = self.cam.shake.max(0.03);
            }
            R::Pulsed => {
                // it comes up out of the river dark red, and keeps the beat a while
                self.world.body.color = crate::sim::PEARL.lerp(burst::river::BLOOD, 0.8);
                self.heart = (self.world.burst.river.rhythm(), 0.0, HEART_BEATS);
                self.sounds.push(Sound::Beat);
                self.gesture = Gesture::Idle;
            }
        }
    }

    /// The beat it kept from the river: a small pulse through the body, fainter each time.
    pub(super) fn update_heart(&mut self, dt: f32) {
        let (period, t, left) = &mut self.heart;
        if *left == 0 || !self.world.burst.whole() {
            return;
        }
        *t += dt;
        if *t >= *period {
            *t -= *period;
            *left -= 1;
            let amp = 0.12 + 0.3 * (*left as f32 / HEART_BEATS as f32);
            self.world.add_ripple(V3::UP, amp);
            self.world.add_ripple(-V3::UP, amp * 0.6);
        }
    }

    pub(super) fn morula_event(&mut self, e: morula::Event) {
        match e {
            morula::Event::Cleaved => self.discover(Discovery::Cleave),
            morula::Event::BecameGrain => self.discover(Discovery::Grain),
            morula::Event::Differentiated => self.discover(Discovery::Eye),
            morula::Event::Died => {
                self.discover(Discovery::Shed);
                self.cam.shake = self.cam.shake.max(0.02);
            }
            morula::Event::Compacting => {
                // The slab will stand facing whoever pressed it.
                if let Some(m) = &mut self.world.division.morula {
                    m.slab_yaw = self.cam.yaw;
                }
                self.discover(Discovery::Stone);
            }
            morula::Event::Slab => {
                self.discover(Discovery::Slab);
                self.cam.shake = 0.06;
                self.hit_stop = 0.1;
            }
            morula::Event::Thawed => {
                self.discover(Discovery::Thaw);
                self.world.ripple_both(0.9);
            }
        }
    }
}
