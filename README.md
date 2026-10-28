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
web/      the site: the page it lives on, plus the compiled object in web/pkg/
build.sh  compiles src/ into web/pkg/
```

It is hosted on Vercel, which runs `build.sh` and serves `web/` on every push to `main` (`vercel.json`, `scripts/vercel-install.sh`). Tests run on GitHub (`.github/workflows/test.yml`).

## Build it yourself

Requires Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.100, and Python (or any static file server).

```
./build.sh                         # compiles into web/pkg/
python3 -m http.server -d web      # then open http://localhost:8000
cargo test                         # runs the simulation tests
```
