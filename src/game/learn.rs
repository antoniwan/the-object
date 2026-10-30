//! Each frame, what it learns of the player: through the pointer, and through the senses.

use super::*;

/// How long after its one thing is said (seconds) the signs of it start to sound.
const CONTACT_SIGNS_AFTER: f32 = 0.3;

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
            note: self.remembrance.pitch().filter(|_| self.senses.is_open(senses::Kind::Hear)),
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
                        contact::Turn::Hum => {
                            if let Some(f) = self.remembrance.pitch() {
                                if self.senses.hum_first(f) {
                                    self.discover(Discovery::YourNote);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                C::Answered => {
                    self.sounds.push(Sound::Merge);
                    self.world.ripple_both(0.6);
                    self.discover(Discovery::Answer);
                }
                C::Reaching => self.sounds.push(Sound::Become),
                C::Said => {
                    // What it says is heard too, a note for each sign, as the signs appear.
                    self.sounds.extend(signs(&self.contact.reply(), CONTACT_SIGNS_AFTER));
                    self.discover(Discovery::Contact);
                }
                C::Asked => {
                    self.sounds.push(Sound::Call);
                    self.question.push(e);
                }
                C::Kept(n) => {
                    let pitch = f32::from(n) / f32::from(contact::KEEP_STEPS);
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
        let look = senses::Look {
            stage: self.stage(),
            still: self.still,
            busy: self.contact.acting(),
            fwd: b.fwd,
            right: b.right,
            up: b.up,
        };
        self.senses.learned = self.remembrance.pitch();
        self.senses.step(dt, &mut self.world, &look);
        for d in self.senses.drain_found() {
            self.discover(d);
        }
        // A note held and let go: an answer, if it was waiting for one, and a note to learn.
        if let Some(f) = self.senses.take_sung() {
            self.contact.sang(f);
            self.remembrance.sung(f);
        }
    }
}
