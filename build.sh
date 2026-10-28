#!/usr/bin/env bash
# Builds the game into a single self-contained HTML file: dist/index.html
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir target/pkg \
  target/wasm32-unknown-unknown/release/chaos_object.wasm
mkdir -p dist
node web/bundle.mjs target/pkg web/index.html dist/index.html
ls -lh dist/index.html
