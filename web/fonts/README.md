Self-hosted from [Fontsource](https://fontsource.org), all under the SIL Open Font License 1.1 (texts beside each file).

- `cormorant-garamond-latin-*` from `@fontsource/cormorant-garamond` 5.x: the words.
- `jetbrains-mono-latin-400-normal-*` from `@fontsource/jetbrains-mono` 5.x: the count, and the markup the words pass through.
- `noto-sans-math-glyphs` from `@fontsource/noto-sans-math` 5.x, subset with `pyftsubset` to the circles, shapes and signs in `web/glyph.js`. Add a glyph there, subset again.

Every file name ends in the first 8 hex digits of its SHA-256, because `/fonts/*` is cached for a year as immutable. Change a file, rename it, and update its `@font-face` and `preload` lines in `index.html`.

## The text faces: core and ext

Each text face is cut in two, joined again by `unicode-range` in `index.html`:

- **core**: printable ASCII, no-break space, `– — ‘ ’ “ ” …`. Everything the page says today is in here (every string in `src/` and `index.html`; the count is lowercase roman numerals; the markup stage of `glyph.js` is `<>/{}[]=#;&|\_~^*+`). Only core is ever fetched.
- **ext**: the rest of Fontsource's latin range (Latin-1 letters, other punctuation, € ™ and so on), so any other text, such as an error message, still renders in the right face. It is fetched only if such a character is shown.

If new text needs a character outside core, add it to `CORE` (and take it out of `EXT`) and cut both again; kerning does not cross from one file to the other.

Layout features kept: only those a browser turns on by itself (`kern liga clig calt ccmp locl mark mkmk rlig`). Hinting is kept (these faces carry only a `prep` table; dropping it saves nothing).

```
pip install fonttools brotli
# SRC: the Fontsource latin woff2 files (node_modules/@fontsource/<family>/files/<family>-latin-<weight>-<style>.woff2)
FEATURES=kern,liga,clig,calt,ccmp,locl,mark,mkmk,rlig
CORE=U+0020-007E,U+00A0,U+2013-2014,U+2018-2019,U+201C-201D,U+2026
EXT=U+00A1-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-2012,U+2015-2017,U+201A-201B,U+201E-2025,U+2027-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD
for f in cormorant-garamond-latin-400-italic cormorant-garamond-latin-500-normal jetbrains-mono-latin-400-normal; do
  pyftsubset "$SRC/$f.woff2" --unicodes="$CORE" --layout-features="$FEATURES" --flavor=woff2 --output-file="$f-core.woff2"
  pyftsubset "$SRC/$f.woff2" --unicodes="$EXT"  --layout-features="$FEATURES" --flavor=woff2 --output-file="$f-ext.woff2"
done
# then name each by its content:
for x in *-core.woff2 *-ext.woff2; do mv "$x" "${x%.woff2}.$(sha256sum "$x" | cut -c1-8).woff2"; done
```

## Loading

Preloaded: the italic core (the loader's words and the first hint) and the glyphs (the loader's words pass through them, and their `font-display: block` must not wait). The mono core is not preloaded: it is first needed a moment into the loader, and preloading it took bandwidth from the wasm and made the first frame later.
