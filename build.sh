#!/usr/bin/env bash
# Compiles the object to WebAssembly into web/pkg/. Afterwards web/ is the whole site.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/chaos_object.wasm
ls -lh web/pkg
