#!/usr/bin/env bash
# Compiles the object to WebAssembly into web/pkg/. Afterwards web/ is the whole site.
# ./build.sh --hash names the files in web/pkg/ by their contents and points web/index.html at
# them (Vercel does this, so they can be cached for good); a plain ./build.sh puts the names back.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
WASM_OPT=124  # binaryen release; scripts/vercel-install.sh installs the same one

cargo build --release --target wasm32-unknown-unknown
rm -rf web/pkg
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/chaos_object.wasm

# The feature flags are what rustc already emits for wasm32; wasm-opt must be told, or it
# refuses the module. Nothing here asks the browser for more than the compiled module did.
if wasm-opt --version 2>/dev/null | grep -q "version $WASM_OPT "; then
  wasm-opt -Oz --strip-producers \
    --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int \
    --enable-sign-ext --enable-mutable-globals --enable-multivalue --enable-reference-types \
    web/pkg/chaos_object_bg.wasm -o web/pkg/chaos_object_bg.wasm
else
  echo "build.sh: wasm-opt $WASM_OPT (binaryen) not found; web/pkg/ is left unoptimized, which is fine for local work" >&2
fi

node scripts/pkg-names.mjs "$@"
ls -lh web/pkg
