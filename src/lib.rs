pub mod camera;
pub mod game;
pub mod math;
pub mod progress;
pub mod sim;

#[cfg(target_arch = "wasm32")]
mod render;
#[cfg(target_arch = "wasm32")]
mod web;
