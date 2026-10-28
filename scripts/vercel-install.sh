#!/usr/bin/env bash
# Vercel's build image has Node but no Rust. This installs what build.sh needs,
# and the one package the function in api/ uses.
set -euo pipefail
WASM_BINDGEN=0.2.100
BINARYEN=124  # must match WASM_OPT in build.sh

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

# wasm-opt, from binaryen's release tarball; only the one binary is kept (it is static).
if ! wasm-opt --version 2>/dev/null | grep -q "version $BINARYEN "; then
  url="https://github.com/WebAssembly/binaryen/releases/download/version_$BINARYEN/binaryen-version_$BINARYEN-x86_64-linux.tar.gz"
  curl -sSfL "$url" | tar -xz -C /tmp "binaryen-version_$BINARYEN/bin/wasm-opt"
  mkdir -p "$HOME/.cargo/bin"
  cp "/tmp/binaryen-version_$BINARYEN/bin/wasm-opt" "$HOME/.cargo/bin/"
fi
wasm-opt --version

npm ci --omit=dev --no-audit --no-fund
