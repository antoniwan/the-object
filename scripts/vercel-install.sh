#!/usr/bin/env bash
# Vercel's build image has Node but no Rust. This installs what build.sh needs,
# and the one package the function in api/ uses.
set -euo pipefail
WASM_BINDGEN=0.2.100

if ! command -v cargo >/dev/null && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
fi
export PATH="$HOME/.cargo/bin:$PATH"
rustup target add wasm32-unknown-unknown

if ! wasm-bindgen --version 2>/dev/null | grep -q "$WASM_BINDGEN"; then
  url="https://github.com/rustwasm/wasm-bindgen/releases/download/$WASM_BINDGEN/wasm-bindgen-$WASM_BINDGEN-x86_64-unknown-linux-musl.tar.gz"
  curl -sSfL "$url" | tar -xz -C /tmp
  # Vercel's image may already ship Rust elsewhere, so this folder isn't guaranteed to exist.
  mkdir -p "$HOME/.cargo/bin"
  cp "/tmp/wasm-bindgen-$WASM_BINDGEN-x86_64-unknown-linux-musl/wasm-bindgen" "$HOME/.cargo/bin/"
fi
wasm-bindgen --version

npm ci --omit=dev --no-audit --no-fund
