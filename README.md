# the object

There is an object.

You can touch it.

**[Touch it.](https://the-object.antoniwan.online/)**

---

An art project. Something liquid, in a white room, in your browser.

It is written in Rust, compiled to WebAssembly, and drawn by a single shader. There is no engine, no physics library and no image of it anywhere: every shape is an equation, every sound is synthesized, every word arrives as something else first.

What it does is for you to find out. It keeps count.

It remembers you between visits. That memory lives in your browser and nowhere else. The site itself only counts visits, anonymously.

You are not the only one paying attention.

## What is here

```
src/             the object: how it moves, what touching it does, how it is drawn, how it sounds
web/             the page it lives on; the compiled object lands in web/pkg/
build.sh         compiles src/ into web/pkg/
CONTRIBUTING.md  how it is built, how it is put together, and the rules that keep it standing
```

## Build it yourself

You need Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.100, and any static file server.

```
./build.sh                         # compiles into web/pkg/
python3 -m http.server -d web      # then open http://localhost:8000
cargo test                         # the tests run natively, no browser needed
```
