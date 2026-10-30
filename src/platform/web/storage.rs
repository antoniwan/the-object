//! What is kept between visits: the localStorage keys, and reading and writing them. They are
//! read as the page opens and written after each frame, in mod.rs.

use super::page::storage;

pub(super) const KEY_FOUND: &str = "the-object.found";
pub(super) const KEY_VISITS: &str = "the-object.visits";
/// What it has learned about whoever keeps visiting (see contact.rs).
pub(super) const KEY_CONTACT: &str = "the-object.contact";
/// Whether the player ever told the browser no to the microphone or the camera (input/senses/).
pub(super) const KEY_SENSES: &str = "the-object.senses";
/// What it remembers of what was done to it, and when you were last here (remembrance.rs).
pub(super) const KEY_REMEMBRANCE: &str = "the-object.remembrance";
/// How the meetings with the other one ended, every visit (other.rs).
pub(super) const KEY_MET: &str = "the-object.met";

/// What was kept under `key`, or nothing (no storage, or never kept).
pub(super) fn load(key: &str) -> String {
    storage().and_then(|s| s.get_item(key).ok().flatten()).unwrap_or_default()
}

/// Keeps `value` under `key`. The value is only made when there is somewhere to keep it.
pub(super) fn save(key: &str, value: impl FnOnce() -> String) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, &value());
    }
}
