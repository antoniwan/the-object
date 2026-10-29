# For Claude

Read these before changing anything, in this order:

1. **`DESIGN.md`**: the design system. Colors, type, timing, sound, voice, the interaction grammar, and the checklist for adding a form. New work should be built from it, not around it.
2. **`CONTRIBUTING.md`**: how to build and test, the module map, and the rules that keep the site standing.

## Rules that are never broken

- **Never say what the object is.** Not on screen, in code comments, in the README, in commit messages or in pull request text. Describe what it does.
- **Shader compile budget.** Every loop in `src/shader.frag` starts from `ZERO`. Never add a call to `scene()`, `shadeLiquid()` or `calcNormal()`. Time the first frame in headless Chromium at 48×32 before and after any shader change (`node scripts/shader-time.mjs`), and say the numbers in the pull request.
- **One module per verb or form**, in the folder of its family (`src/forms/`, `src/division/`, `src/floor/`, `src/burst/`, `src/input/`, `src/mind/`) or its own file. Keep changes to `game/` and `sim/` small.
- **Forms start only from the ordinary liquid** (`World::ordinary()`), blend through `Phase`/`Blend`, and have a way back.
- **Discoveries are appended** to the end of `Discovery`, `ALL`, `name()` and `line()` in `progress.rs`. Lines are lowercase, short, serious, never cute, never an explanation. Check them against `DESIGN.md` section 7.
- **New on-screen text carries `data-glyph`.** No new fonts; if you add a glyph, re-subset the math font.
- **Every pull request bumps the version and adds a `CHANGELOG.md` entry**: `./scripts/version.sh <next>`, by the rule in `CONTRIBUTING.md` (major: the arc or saved progress; minor: a new verb or form; patch: everything else). The entry follows the same voice rules: describe what it does, never what it is. It does not write a devlog post of its own: the devlog is one post a day (`devlog/YYYY-MM-DD.md`, covering every version that day, with GIFs from `scripts/gif.mjs`), paste-ready for itch.io by the end of the day; `devlog/README.md` says how. `cargo test` fails once a day has gone by without its post.
- **Never commit build output** (`web/pkg/`, `target/`, `dist/`).
- **The README stays short and cryptic.** Developer detail goes in `CONTRIBUTING.md`; design detail goes in `DESIGN.md`.

## Before opening a pull request

```
./scripts/version.sh <next>   # then write its CHANGELOG.md entry (and today's devlog/ post, if you are the one writing it)
cargo test
npm ci && npm test
./build.sh
```

If you add a color, a timing, a sound or a row to the gesture table, update `DESIGN.md` in the same pull request.
