//! The object. Everything outside render.rs, web.rs and audio.rs is plain Rust with no
//! browser types, so it builds and tests natively; those three only exist in the wasm build.

pub mod aura;
pub mod burst;
pub mod camera;
pub mod contact;
pub mod forms;
pub mod fusion;
pub mod game;
pub mod landscape;
pub mod math;
pub mod meiosis;
pub mod mere;
pub mod morula;
pub mod progress;
pub mod sim;

#[cfg(target_arch = "wasm32")]
mod render;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod audio;
