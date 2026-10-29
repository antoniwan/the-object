//! Each frame, what it learns of the player: through the pointer, and through the senses.

use super::*;

impl Game {
    /// It learns the player back, and now and then does something about it.
    pub(super) fn update_contact(&mut self, dt: f32, b: &Basis) {
        use contact::Event as C;
        let sense = Sense {
            stage: self.stage(),
            found: self.progress.count(),
            idle: self.still,
            fwd: b.fwd,
            right: b.right,
            up: b.up,
            pointer: self.pointer_at,
        };
        self.contact.step(dt, &mut self.world, &sense);
        for e in self.contact.drain_events() {
            match e {
                C::Regarded => self.discover(Discovery::Regard),
                C::Flinched => self.discover(Discovery::Flinch),
                C::Tap { pitch } => self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() }),
                C::Echoed => self.discover(Discovery::Echo),
                C::Mimicked => self.discover(Discovery::Mimic),
                C::Turned(turn) => {
                    self.discover(Discovery::Initiate);
                    match turn {
                        contact::Turn::Offer => self.discover(Discovery::Offer),
                        contact::Turn::Beckon => self.sounds.push(Sound::Call),
                        _ => {}
                    }
                }
                C::Answered => {
                    self.sounds.push(Sound::Merge);
                    self.world.add_ripple(V3::UP, 0.6);
                    self.world.add_ripple(-V3::UP, 0.6);
                    self.discover(Discovery::Answer);
                }
                C::Reaching => self.sounds.push(Sound::Become),
                C::Said => self.discover(Discovery::Contact),
                C::Asked => {
                    self.sounds.push(Sound::Call);
                    self.question.push(e);
                }
                C::Kept(n) => {
                    let pitch = n as f32 / contact::KEEP_STEPS as f32;
                    self.sounds.push(Sound::Poke { pitch, size: self.world.body.radius() * 0.3 });
                }
                C::Answer(_) => {
                    self.sounds.push(Sound::Merge);
                    self.world.add_ripple(-b.fwd, 0.5);
                    self.question.push(e);
                }
            }
        }
    }

    /// Hearing and sight, once the player has let it have them.
    pub(super) fn update_senses(&mut self, dt: f32, b: &Basis) {
        let look = senses::Look { stage: self.stage(), still: self.still, busy: self.contact.acting(), fwd: b.fwd, right: b.right, up: b.up };
        self.senses.step(dt, &mut self.world, &look);
        for d in self.senses.drain_found() {
            self.discover(d);
        }
    }
}
