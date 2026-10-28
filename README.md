# the object

There is an object.

You can touch it.

**[Touch it.](https://antoniwan.github.io/the-object/)**

---

An art project. A liquid object rendered in the browser, written in Rust and compiled to WebAssembly. No libraries for the physics or rendering: a small simulation in Rust, and one shader that draws the liquid.

What it does is for you to find out. It keeps count.

## Build

Requires Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.100, and Node.

```
./build.sh
```

This writes a single self-contained file to `docs/index.html`, which is what is served.

```
cargo test
```

runs the simulation tests natively.
