# Working on the object

This is the plain-language side of the repository: how to build it, how it is put together, and the few rules that keep it from breaking. The README stays short and says nothing about what the object does. Keep it that way; what it does is mapped elsewhere, not here.

## Build, run, test

You need Rust with the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at exactly 0.2.100 (it must match the `wasm-bindgen` version pinned in `Cargo.toml`).

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100

./build.sh                         # compiles to web/pkg/; after this, web/ is the whole site
python3 -m http.server -d web      # open http://localhost:8000
cargo test                         # runs natively
npm ci && npm test                 # the serverless function's helpers (api/)
```

Everything except the browser glue is plain Rust with no browser types, so the tests run on your machine without a browser. Gestures are tested by feeding pointer positions into `Game` and checking what was discovered.

Never commit build output. `web/pkg/` and `target/` are ignored on purpose.

## Where it runs

- **GitHub** runs `.github/workflows/test.yml` on every push to `main` and every pull request: `cargo test`, `npm test`, then a full `./build.sh`. Both must pass before a PR merges.
- **Vercel** hosts the site at <https://the-object.antoniwan.online>. It runs `scripts/vercel-install.sh` (installs Rust and wasm-bindgen, which its build image lacks), then `./build.sh`, and serves `web/`. Settings are in `vercel.json`.
- **Only `main` deploys.** Preview deployments are turned off in the Vercel project settings, so branches and pull requests build nothing there. Merging is deploying.
- **itch.io** gets the same object as a zip. `./scripts/itch.sh` packs `web/` into `dist/the-object-itch.zip` with `index.html` at its root, minus the Vercel analytics and the root-absolute links that break when itch serves the game from a subfolder in an iframe. `.github/workflows/itch.yml` does this on a `v*` tag (or by hand from the Actions tab) and attaches the zip to a GitHub release. With an `ITCH_TARGET` repository variable (`user/game`) and a `BUTLER_API_KEY` secret it also pushes the zip to the itch page.
- **One serverless function**, `api/rating.js`, keeps the answers to the question the object asks after contact (`contact.rs`, `Ask`): the player is handed a drop and the steps they kept it for, 1 to 5, are posted to `/api/rating`. They are stored in a private Vercel Blob store connected to the project (`BLOB_READ_WRITE_TOKEN`), as counts in `ratings/summary.json` and one small file per visitor under `ratings/visitors/`, named by a salted hash of their address (salt: `RATING_SALT` if set, else the store token), never the address itself. A visitor who answers again replaces their answer, and no sooner than a minute later. `GET /api/rating` returns the mean and count; `web/index.html` adds them to the page's JSON-LD as an `AggregateRating` once there is at least one. Nothing is invented: with no answers there is no rating.
- Vercel Web Analytics is loaded by `web/index.html` and counts page views. The object's own memory of a player stays in their browser (see Saved state).

## How a frame happens

1. `web.rs` turns browser events (pointer, wheel, the `m` key that mutes) into calls on `Game`.
2. `game.rs` turns raw pointer movement into verbs: poke, stretch, slice, stir, and so on. It owns the camera, the player's progress, and what the object has learned about them.
3. The verbs act on `World` in `sim.rs`, which owns the body, its tendrils, ripples and free drops, plus one state machine per transformation.
4. `render.rs` copies the world into shader uniforms, and `shader.frag` raymarches the scene.
5. `web.rs` writes the object's color, agitation and screen position onto the page as CSS variables (computed in `aura.rs`); `web/glyph.js` uses them to tint and scatter the words.
6. `audio.rs` plays whatever sounds the frame asked for.

## Module map

| File | What it holds |
| --- | --- |
| `src/web.rs` | Browser glue: canvas, input, the frame loop, the hint, the toast and the count, saving to `localStorage`. |
| `src/game.rs` | Pointer input to verbs. The header comment lists every gesture. |
| `src/sim.rs` | The liquid: one body, tendrils, ripples, drops, tearing, merging, color, night. `World::sdf` is mirrored by the shader. |
| `src/meiosis.rs` | Slice it and it divides into two cells; squeeze them back into one. |
| `src/morula.rs` | Past two: every cell divides, then grain, the eye, cells dying, the stone. |
| `src/fusion.rs` | Slam it into the floor and the floor turns liquid. |
| `src/mere.rs` | What the liquid floor can become: mirror, ice, the deep, a fountain. |
| `src/burst.rs` | Poke it to death and it bursts into twelve pieces. |
| `src/landscape.rs` | What the twelve pieces do once they have landed. |
| `src/forms/` | Four verbs that each make it something else: `bloom` (flower), `filament` (thread), `cosmos` (universe), `pandora` (box). |
| `src/contact.rs` | It learns the player, in stages unlocked by how much has been discovered: regard (6), echo (14), its own turn (24), contact (36). Saved between visits. |
| `src/progress.rs` | The discoveries: the enum, their names, and the line shown for each. 66 so far. |
| `src/aura.rs` | The object's state as the page's words need it: color, unrest, screen position. |
| `src/camera.rs` | The orbit camera and screen shake. |
| `src/render.rs` | WebGL2 setup, compiling the shader in the background, uniforms. |
| `src/shader.frag` | The one fragment shader that draws everything. |
| `src/audio.rs` | Synthesized sounds, no samples. |
| `src/math.rs` | Small vector math, so the simulation has no dependencies. |
| `web/index.html` | The page: canvas, loader, the three lines of text, styles. |
| `web/glyph.js` | Makes every word on the page decode letter by letter. |
| `web/fonts/` | Self-hosted fonts and their licenses; see its README. |
| `web/llms.txt`, `robots.txt`, `sitemap.xml`, `og.png` | For crawlers and link previews. |

## Adding a verb or a transformation

- Give it its own module (or file in `src/forms/`) and keep `game.rs` and `sim.rs` changes small. Several people may be working at once, and separate files keep merges clean.
- Write the module's header comment the way the others are written: what the player does and what happens, in a few plain sentences. Then add the gesture to the list at the top of `game.rs`.
- Nothing jumps. A state machine only decides which 0..1 values are moving; the shader blends between them. Follow the `Phase`/`Blend` pattern in `meiosis.rs`.
- New discoveries go at the end of the `Discovery` enum in `progress.rs`, and also in `ALL`, `name()` and `line()`. The test `all_lists_every_discovery` catches a missed one. The line is shown on screen: cryptic, serious, short, lowercase, never cute, never an explanation.
- Add a test in the module that drives the gesture through `Game` and checks the discovery.
- Never say what the object is: not in code comments, not on screen, not in the README.

## The shader compile budget

On 2026-10-28 the live site crashed a desktop browser. The shader had grown to take about 11 seconds to compile, and some GPU drivers hang on that. The cause was that the compiler copies `scene()` into every place it is called, and unrolls every loop it can; a 12-piece loop inside `scene()` multiplied across dozens of copies. It now compiles in under a second. To keep it that way:

- **Every loop starts from `ZERO`**, not `0`. `ZERO` is a zero the compiler can't see through, so the loop stays a loop instead of being unrolled.
- **Don't add new calls to `scene()`, `shadeLiquid()` or `calcNormal()`.** Add your shape inside `scene()` or your material inside the existing call instead.
- **Measure before and after** any shader change: load the page in headless Chromium at a tiny viewport (48×32) and time the first frame. If it gets noticeably slower, find out why before merging.
- On-screen effects that don't need the shader belong in the page (CSS, `glyph.js`), not in `shader.frag`.

The page also guards against slow machines: the loader stays up while the shader compiles in the background (`KHR_parallel_shader_compile`), large screens render at most about 1.6 million pixels and scale up, and a lost WebGL context is recovered.

## Words on screen

Every visible line (`#hint`, `#toast`, `#count`) carries `data-glyph`. The Rust side only sets text and classes; `web/glyph.js` watches those elements and decodes each letter through circles, shapes, signs and markup before it settles. New on-screen text should carry `data-glyph` too.

Fonts are self-hosted from Fontsource. The math font is cut down to just the glyphs `glyph.js` uses, so if you add a glyph there, subset the font again (`web/fonts/README.md` says how).

## Saved state

Three `localStorage` keys, all in the player's browser (the one answer that leaves it is described under Where it runs):

- `the-object.found`: which discoveries they have made.
- `the-object.visits`: how many times they have come back.
- `the-object.contact`: what the object has learned about them (`contact.rs`).

Clearing site data starts the object over.
