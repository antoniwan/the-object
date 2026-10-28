#!/usr/bin/env bash
# Compiles the object to WebAssembly into web/pkg/. Afterwards web/ is the whole site.
# ./build.sh --hash names the files in web/pkg/ by their contents and points web/index.html at
# them (Vercel does this, so they can be cached for good); a plain ./build.sh puts the names back.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --target wasm32-unknown-unknown
rm -rf web/pkg
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/chaos_object.wasm

node scripts/pkg-names.mjs "$@"
ls -lh web/pkg
