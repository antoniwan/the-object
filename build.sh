#!/usr/bin/env bash
# Builds the game into a single self-contained HTML file: docs/index.html
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir target/pkg \
  target/wasm32-unknown-unknown/release/chaos_object.wasm
mkdir -p docs
node web/bundle.mjs target/pkg web/index.html docs/index.html
ls -lh docs/index.html
