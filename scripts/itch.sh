#!/usr/bin/env bash
# Packs the built site (web/, after ./build.sh) into dist/the-object-itch.zip for itch.io.
# itch serves the game from a CDN subfolder inside an iframe, so root-absolute links
# and Vercel-only parts (analytics, the ratings endpoint) are dropped; everything else is the same object.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f web/pkg/chaos_object_bg.wasm ] || ./build.sh

out=dist/itch
rm -rf "$out" dist/the-object-itch.zip
mkdir -p "$out"
cp -r web/pkg web/fonts web/glyph.js web/favicon.ico web/favicon-32.png "$out"/
sed -e '/_vercel\/insights/d' -e '/window\.va = /d' \
    -e '/rel="manifest"/d' -e '/rel="apple-touch-icon"/d' \
    -e 's|href="/favicon|href="favicon|g' \
    web/index.html \
  | perl -0pe 's|<script type="module">\n// After contact it asks one question.*?</script>\n||s' > "$out/index.html"
# The ratings script posts to the site's own /api/rating; itch has its own ratings.
grep -q 'api/rating' "$out/index.html" && { echo "itch.sh: rating script left in index.html" >&2; exit 1; }

if grep -nE '(src|href)="/[^/]' "$out/index.html"; then
  echo "itch.sh: root-absolute path left in index.html" >&2; exit 1
fi

(cd "$out" && zip -qr9 ../the-object-itch.zip .)
ls -lh dist/the-object-itch.zip
