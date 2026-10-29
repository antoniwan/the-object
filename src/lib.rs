//! The object. Everything outside `platform/` is plain Rust with no browser types, so it builds
//! and tests natively; `platform/` only exists in the wasm build.
//!
//! - `game`: pointer input to verbs; `sim`: the liquid itself.
//! - `division`, `floor`, `burst`, `forms`, `wound`, `other`: what it can become, one module each.
//! - `input`: keyboard, phone, words, hearing and sight. `mind`: what it learns of the player.
//! - `progress`, `aura`, `camera`, `math`, `version`: small shared pieces; `pace`: how much of
//!   the screen is drawn.

pub mod aura;
pub mod burst;
pub mod camera;
pub mod division;
pub mod floor;
pub mod forms;
pub mod game;
pub mod input;
pub mod math;
pub mod mind;
pub mod other;
pub mod pace;
pub mod progress;
pub mod sim;
pub mod version;
pub mod wound;

#[cfg(target_arch = "wasm32")]
mod platform;
