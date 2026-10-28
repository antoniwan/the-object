# For Claude

Read these before changing anything, in this order:

1. **`DESIGN.md`**: the design system. Colors, type, timing, sound, voice, the interaction grammar, and the checklist for adding a form. New work should be built from it, not around it.
2. **`CONTRIBUTING.md`**: how to build and test, the module map, and the rules that keep the site standing.

## Rules that are never broken

- **Never say what the object is.** Not on screen, in code comments, in the README, in commit messages or in pull request text. Describe what it does.
- **Shader compile budget.** Every loop in `src/shader.frag` starts from `ZERO`. Never add a call to `scene()`, `shadeLiquid()` or `calcNormal()`. Time the first frame in headless Chromium at 48×32 before and after any shader change, and say the numbers in the pull request.
- **One module per verb or form**, in `src/forms/` or its own file. Keep changes to `game.rs` and `sim.rs` small.
- **Forms start only from the ordinary liquid** (`World::ordinary()`), blend through `Phase`/`Blend`, and have a way back.
- **Discoveries are appended** to the end of `Discovery`, `ALL`, `name()` and `line()` in `progress.rs`. Lines are lowercase, short, serious, never cute, never an explanation. Check them against `DESIGN.md` section 7.
- **New on-screen text carries `data-glyph`.** No new fonts; if you add a glyph, re-subset the math font.
- **Never commit build output** (`web/pkg/`, `target/`, `dist/`).
- **The README stays short and cryptic.** Developer detail goes in `CONTRIBUTING.md`; design detail goes in `DESIGN.md`.

## Before opening a pull request

```
cargo test
npm ci && npm test
./build.sh
```

If you add a color, a timing, a sound or a row to the gesture table, update `DESIGN.md` in the same pull request.
