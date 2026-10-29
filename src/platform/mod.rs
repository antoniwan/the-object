//! The browser: everything here only exists in the wasm build. `web` is the page and the frame
//! loop, `render` WebGL2 and the shader, `audio` the synthesized sound, `devices` the microphone,
//! camera and phone sensors, `speech` the browser's speech recognition.

pub(crate) mod audio;
pub(crate) mod devices;
pub(crate) mod render;
pub(crate) mod speech;
pub(crate) mod web;
