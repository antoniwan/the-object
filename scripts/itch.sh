#!/usr/bin/env bash
# Packs the built site (web/, after ./build.sh) into dist/the-object-itch.zip for itch.io.
# With --dir it stops at the unzipped folder, dist/itch/.
# The page itself is rewritten by itch-page.mjs; everything else is the same object.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f web/pkg/chaos_object_bg.wasm ] || ./build.sh

out=dist/itch
rm -rf "$out" dist/the-object-itch.zip
mkdir -p "$out"
cp -r web/pkg web/fonts web/glyph.js web/favicon.ico web/favicon-32.png "$out"/
node scripts/itch-page.mjs web/index.html "$out/index.html"

[ "${1:-}" = "--dir" ] && exit 0  # butler can push the folder as is
(cd "$out" && zip -qr9 ../the-object-itch.zip .)
ls -lh dist/the-object-itch.zip
