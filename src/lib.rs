//! The object. Everything outside render.rs, web.rs, audio.rs and devices.rs is plain Rust with no
//! browser types, so it builds and tests natively; those four only exist in the wasm build.

pub mod aura;
pub mod burst;
pub mod camera;
pub mod contact;
pub mod forms;
pub mod fusion;
pub mod game;
pub mod keys;
pub mod kin;
pub mod landscape;
pub mod math;
pub mod meiosis;
pub mod mere;
pub mod morula;
pub mod motion;
pub mod other;
pub mod progress;
pub mod remembrance;
pub mod river;
pub mod senses;
pub mod sim;
pub mod thoughts;
pub mod version;
pub mod wound;

#[cfg(target_arch = "wasm32")]
mod render;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod audio;
#[cfg(target_arch = "wasm32")]
mod devices;
