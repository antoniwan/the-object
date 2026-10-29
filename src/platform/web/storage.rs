//! What is kept between visits: the localStorage keys. They are read as the page opens and
//! written after each frame, in mod.rs.

pub(super) const KEY_FOUND: &str = "the-object.found";
pub(super) const KEY_VISITS: &str = "the-object.visits";
/// What it has learned about whoever keeps visiting (see contact.rs).
pub(super) const KEY_CONTACT: &str = "the-object.contact";
/// Whether the player ever told the browser no to the microphone or the camera (senses.rs).
pub(super) const KEY_SENSES: &str = "the-object.senses";
/// What it remembers of what was done to it, and when you were last here (remembrance.rs).
pub(super) const KEY_REMEMBRANCE: &str = "the-object.remembrance";
