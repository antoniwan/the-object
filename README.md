# the object

There is an object.

You can touch it.

**[Touch it.](https://antoniwan.github.io/the-object/)**

---

An art project. A liquid object rendered in the browser, written in Rust and compiled to WebAssembly. No libraries for the physics or rendering: a small simulation in Rust, and one shader that draws the liquid.

What it does is for you to find out. It keeps count.

## What is here

```
src/      the object: its physics, what touching it does, how it is drawn, how it sounds
web/      the page it lives on
build.sh  turns both into one file, dist/index.html
```

Every push to `main` is tested, built and published by `.github/workflows/pages.yml`.

## Build it yourself

Requires Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.100, and Node.

```
./build.sh       # writes dist/index.html; open it in a browser
cargo test       # runs the simulation tests
```
