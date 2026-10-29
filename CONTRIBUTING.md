# Working on the object

This is the plain-language side of the repository: how to build it, how it is put together, and the few rules that keep it from breaking. The README stays short and says nothing about what the object does. Keep it that way; what it does is mapped elsewhere, not here. How it should look, move, sound and speak is in `DESIGN.md`.

## Build, run, test

You need Rust with the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at exactly 0.2.100 (it must match the `wasm-bindgen` version pinned in `Cargo.toml`).

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100

./build.sh                         # compiles to web/pkg/; after this, web/ is the whole site
python3 -m http.server -d web      # open http://localhost:8000
cargo test                         # runs natively
cargo run --release --example frame_bench   # times a frame of each scene, natively
cargo run --release --example frame_bench -- --check   # fails if a scene no longer ends where examples/fingerprints.txt says
npm ci && npm test                 # the serverless function's helpers (api/)
```

Everything except the browser glue is plain Rust with no browser types, so the tests run on your machine without a browser. Gestures are tested by feeding pointer positions into `Game` and checking what was discovered.

Never commit build output. `web/pkg/` and `target/` are ignored on purpose.

Vercel builds with `./build.sh --hash`, which names the two files in `web/pkg/` by a hash of their contents and rewrites the `pkg/` links in `web/index.html` to match (`scripts/pkg-names.mjs`), so `vercel.json` can cache `/pkg/` for a year and a deploy is never served stale code. A plain `./build.sh` puts the plain names back, so don't commit `index.html` after a `--hash` build; run `./build.sh` first. Font files are cached the same way, so a changed font gets a new file name.

## Versions and the changelog

`Cargo.toml` holds the version; it follows semantic versioning, read for a piece you touch:

- **major**: the arc changes (how it learns the player, what it does at the end), or something a player kept between visits is lost (a `localStorage` key renamed or reset).
- **minor**: a new verb, a new form, or anything else new a player can do or see.
- **patch**: fixes, speed, words, metadata, hosting and documentation.

Every merge to `main` deploys, so **every pull request is a release**: it bumps the version and adds its entry to `CHANGELOG.md` ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/): Added, Changed, Fixed, Removed; one line each, ending with the PR number).

```
./scripts/version.sh               # prints the current version
./scripts/version.sh 1.3.0         # moves it in Cargo.toml, Cargo.lock, web/index.html, web/llms.txt and CHANGELOG.md
```

Then write what changed under the new heading. `cargo test` fails if the page, `llms.txt` or the changelog disagree with `Cargo.toml` (`src/version.rs`), and CI fails a pull request whose version is not higher than its base. If another PR takes your number first, merge main and bump again.

After the merge, `.github/workflows/release.yml` tags `v<version>` on main and publishes a GitHub release with the changelog entry as its notes. The itch.io push names the build `<version>+<short commit>`.

The devlog is one post a day, not one a version: `devlog/YYYY-MM-DD.md`, written from that day's changelog entries, with GIFs of what a player can see filmed by `scripts/gif.mjs`, ready to paste into the itch.io devlog by the end of the day. `devlog/README.md` says how they are written and filmed. `cargo test` fails once a day has gone by without its post.

## Where it runs

- **GitHub** runs `.github/workflows/test.yml` on every push to `main` and every pull request: `cargo test`, `npm test`, then a full `./build.sh`. Both must pass before a PR merges. On pull requests it also checks that the version went up. `.github/workflows/release.yml` tags each new version on `main` and publishes its release notes.
- **Vercel** hosts the site at <https://the-object.antoniwan.online>. It runs `scripts/vercel-install.sh` (installs Rust and wasm-bindgen, which its build image lacks), then `./build.sh --hash`, and serves `web/`. Settings are in `vercel.json`.
- **Only `main` deploys.** Preview deployments are turned off in the Vercel project settings, so branches and pull requests build nothing there. Merging is deploying.
- **itch.io** gets the same object as a zip. `./scripts/itch.sh` packs `web/` into `dist/the-object-itch.zip` with `index.html` at its root; `scripts/itch-page.mjs` takes out of the page the Vercel analytics, Stats, the ratings script (`web/rating.js`) and the root-absolute links, which break when itch serves the game from a subfolder in an iframe. Every production deploy on Vercel then runs `scripts/itch-push.sh`, which pushes that build to <https://antoniwan.itch.io/the-object> with butler, using the `BUTLER_API_KEY` and `ITCH_TARGET` (`user/game`) environment variables of the Vercel project; so merging to main updates both. A failed push is logged and never fails the deploy. `.github/workflows/itch.yml` is the manual path: on a `v*` tag (or by hand from the Actions tab) it attaches the zip to a GitHub release.
- **One serverless function**, `api/rating.js`, keeps the answers to the question the object asks after contact (`contact.rs`, `Ask`): the player is handed a drop and the steps they kept it for, 1 to 5, are posted to `/api/rating`. They are stored in a private Vercel Blob store connected to the project (`BLOB_READ_WRITE_TOKEN`), as counts in `ratings/summary.json` and one small file per visitor under `ratings/visitors/`, named by a salted hash of their address (salt: `RATING_SALT` if set, else the store token), never the address itself. A visitor who answers again replaces their answer, and no sooner than a minute later. `GET /api/rating` returns the mean and count; `web/index.html` adds them to the page's JSON-LD as an `AggregateRating` once there is at least one. Nothing is invented: with no answers there is no rating.
- Vercel Web Analytics is loaded by `web/index.html` and counts page views. Beside it, `web/index.html` loads Stats, Antonio's self-hosted Umami at `antoniwan-stats.vercel.app` (no cookies, Do Not Track respected, counted only on `the-object.antoniwan.online`), and counts a click on a link to another site as an `outbound` event. The itch.io page leaves both out. The object's own memory of a player stays in their browser (see Saved state).

## How a frame happens

1. `platform/web/` turns browser events (pointer, wheel, the `m` key that mutes) into calls on `Game`, and every other key into calls on `keys.rs`.
2. `game/` turns raw pointer movement into verbs: poke, stretch, slice, stir, and so on. It owns the camera, the player's progress, and what the object has learned about them.
3. The verbs act on `World` in `sim/`, which owns the body, its tendrils, ripples and free drops, plus one state machine per transformation.
4. `platform/render.rs` copies the world into shader uniforms, and `shader.frag` raymarches the scene.
5. `platform/web/` writes the object's color, agitation and screen position onto the page as CSS variables (computed in `aura.rs`); `web/glyph.js` uses them to tint and scatter the words.
6. `platform/audio.rs` plays whatever sounds the frame asked for.

## Module map

Each folder is a family of modules that use each other. A new verb or form goes into the folder of its family, or `src/forms/`, as its own file.

### The core

Pointer to verb, the liquid, and the small pieces everything shares.

| File | What it holds |
| --- | --- |
| `src/game/` | Pointer input to verbs. `mod.rs` holds `Game`, its frame step and, in its header comment, every gesture; `gesture.rs` turns pointer movement into verbs, `news.rs` routes what each transformation reports back, `view.rs` projects between screen and room, `learn.rs` feeds contact and the senses, `sound.rs` names the sounds a frame asks for. |
| `src/sim/` | The liquid: one body, tendrils, ripples, drops, tearing, merging, color, night. `mod.rs` holds `World` and its frame step; `parts.rs` the pieces it is made of, `verbs.rs` what a hand does to it, `liquid.rs` tendrils and drops, `moods.rs` how it feels, `geometry.rs` its distance function (`World::sdf`, mirrored by the shader). |
| `src/progress.rs` | The discoveries: the enum, their names, and the line shown for each. 118 so far. |
| `src/aura.rs` | The object's state as the page's words need it: color, unrest, screen position. |
| `src/camera.rs` | The orbit camera and screen shake. |
| `src/math.rs` | Small vector math, so the simulation has no dependencies. |
| `src/version.rs` | The version from `Cargo.toml`, and the tests that keep the page, `llms.txt` and the changelog saying the same. |

### What it becomes

One folder per family of transformations; the files in a family use each other, and nothing outside it reaches in past its state machine.

| File | What it holds |
| --- | --- |
| `src/division/meiosis.rs` | Slice it and it divides into two cells; squeeze them back into one. |
| `src/division/nerve.rs` | What a cut does to it besides the cut: afraid at once, then, left alone, sure it is still there. Two eased values the shader draws the cells from (`uNerve`). |
| `src/division/morula.rs` | Past two: every cell divides, then grain, the eye, cells dying, the stone. |
| `src/floor/fusion.rs` | Slam it into the floor and the floor turns liquid. |
| `src/floor/mere.rs` | What the liquid floor can become: mirror, ice, the deep, a fountain. |
| `src/burst/mod.rs` | Poke it to death and it bursts into twelve pieces. |
| `src/burst/landscape.rs` | What the twelve pieces do once they have landed. |
| `src/burst/kin.rs` | Which of the landed pieces hold on to which: groups made by setting them down close, followed when carried, loosed, cut apart, breathing together. |
| `src/burst/river.rs` | The twelve crowded together anywhere but the middle melt into the ground and run as a river: touched, columns stand out of it; raise enough and it beats, then drains, and the body gathers itself out of it. |
| `src/forms/` | Four verbs that each make it something else: `bloom` (flower), `filament` (thread), `cosmos` (universe), `pandora` (box); `fold`, where the universe stirred on gets out into the room; and `dust`, where the flower pinched comes apart over the floor, is swept and heaps into a mound; and `growth`, what roots in the mound and grows as far as it is tended. |
| `src/wound.rs` | An old hurt: hurt enough, ever, it scars where it is struck most (kept in `the-object.remembrance`); hurt again, the scar opens and four spiked pieces, drops it holds in place, lie around it turning through every color. Carried in green, one is taken and fills the wound; any other color is spat back out. Three and it closes, leaving the scar fainter. The seam and wound are on the body's surface in the shader (`scarAt()`), the spikes in the drops' loop (`caltrop()`, `uCaltrop` bits). |
| `src/other.rs` | Late, something else in the room: hard, dark, many-sided. It comes after contact (or fifty discoveries), and everything it does is between it and the first: rung, carried, thrown, taken in and given back, standing between, circling. It is not part of the body, so it never changes `World::ordinary()`; when the body becomes anything else, it rises out of the room until it is itself again. |

### What reaches it (`src/input/`)

Everything besides the pointer, as plain Rust tested natively. Their browser halves are in `src/platform/`.

| File | What it holds |
| --- | --- |
| `src/input/keys.rs` | What the keyboard does that a hand cannot: letters land where their keys sit, a held letter hums, space breathes, the arrows tip the room. |
| `src/input/motion.rs` | A phone in the hand: tipped, it tips the room the way the arrows do; shaken, it sloshes, is upset, and comes loose. Plain Rust, tested natively; `devices.rs` feeds it two angles and one push a frame, turned to the screen, and asks iOS for them inside the player's tap. |
| `src/input/pad.rs` | Two fingers pinched on a trackpad: told as a ctrl-held wheel or as Safari's gesture, it squeezes as far as two fingers on a screen do, held while they move. |
| `src/input/words.rs` | Words, once it hears: a verb said is done by the same hand the pointer is, once for each time it is said; a greeting in any language is answered in signs, or not, by what it remembers (`remembrance.rs`). Plain Rust, tested natively. |
| `src/input/senses/` | Hearing and sight, once given: when it asks (`speak.`, `show yourself.`), what a voice, a note, a whisper, a breath, a shout, a movement or a covered lens does to it, and the analysis that reduces sound and picture to a few numbers (`hearing.rs`, `sight.rs`). Plain Rust, tested natively. |

### What it learns (`src/mind/`)

| File | What it holds |
| --- | --- |
| `src/mind/contact.rs` | It learns the player, in stages unlocked by how much has been discovered: regard (6), echo (14), its own turn (24), contact (36). Saved between visits. |
| `src/mind/remembrance.rs` | What it remembers of how it was treated, after Lifesworn's Remembrance: a ledger of deeds (held, sung to, answered, returned to; hurt, broken) it notices by itself each frame, and how warm or wary it is, worked out from that ledger by a formula and never saved. Hurt fades in half the time care does. Once a visit, from regard on, it meets the first hand that comes near accordingly. Other code reads `kind()`, `wary()`, `warmth()` and `bond()`, and adds deeds with `remember()`. |
| `src/mind/thoughts.rs` | What it is thinking: one quiet line under it when the player has stopped finding things, chosen by where it is, what is not yet found and how far it has learned them. The whole table of thoughts and when each one comes is in this file. |

### The browser (`src/platform/`, wasm only)

| File | What it holds |
| --- | --- |
| `src/platform/web/` | Browser glue: canvas, input, the frame loop (`mod.rs`), the hint, the toast, the count and its thoughts (`hud.rs`), the page's colors and marks (`page.rs`), asking for and feeding the senses (`sensing.rs`), canvas size (`frame.rs`), `localStorage` keys (`storage.rs`). |
| `src/platform/render.rs` | WebGL2 setup, compiling the shader in the background, uniforms. |
| `src/platform/audio.rs` | Synthesized sounds, no samples. |
| `src/platform/devices.rs` | Browser side of the same (wasm only): `getUserMedia` inside the player's tap, an `AnalyserNode` for the microphone, a 32 × 24 canvas for the camera, and stopping every track; and the phone's `deviceorientation` / `devicemotion`, asked for with `requestPermission` only where the browser has it and the screen is touched. |
| `src/platform/speech.rs` | Browser side of the same (wasm only): the browser's speech recognition, on the device (`processLocally`) as soon as it hears where the browser can, otherwise only after the player taps `say something.`; the text goes to `words.rs` and is dropped. |

### The shader, the build and the page

| File | What it holds |
| --- | --- |
| `src/shader.frag` | The one fragment shader that draws everything. |
| `build.rs` | Copies `shader.frag` into the build without its comments or indentation, keeping every line where it was, so a driver's error line still points at the source. |
| `examples/frame_bench.rs` | Times the simulation scene by scene and prints a fingerprint of where each one ends up. A change made for speed should leave every fingerprint as it was; CI checks them against `examples/fingerprints.txt` (`--check`). A change meant to move the simulation writes the new ones with `--bless` in the same pull request. |
| `web/index.html` | The page: canvas, loader, the four lines of text, styles. |
| `web/glyph.js` | Makes every word on the page decode letter by letter. |
| `web/rating.js` | Sends the answer to the question asked after contact to `/api/rating`, and adds what all the answers come to to the page's structured data. Left out of the itch build. |
| `scripts/gif.mjs`, `scripts/gif-scenes.mjs` | Films short looping GIFs of the built page for the daily devlog, from scripted scenes on a clock it moves itself (`devlog/README.md`). |
| `scripts/shader-time.mjs` | Times how long the built page takes to be ready in headless Chromium at 48×32: the number to compare before and after a shader change. |
| `scripts/load-check.mjs` | Loads the built page in headless Chromium dressed as Chromium, Safari and Firefox, a first visit and a return, and fails if any throws or never wakes. CI runs it. |
| `web/fonts/` | Self-hosted fonts and their licenses; see its README. |
| `web/llms.txt`, `robots.txt`, `sitemap.xml`, `og.png` | For crawlers and link previews. |

## Adding a verb or a transformation

- Give it its own module (or file in `src/forms/`) and keep `game/` and `sim/` changes small. Several people may be working at once, and separate files keep merges clean.
- Write the module's header comment the way the others are written: what the player does and what happens, in a few plain sentences. Then add the gesture to the list at the top of `game/mod.rs`.
- Nothing jumps. A state machine only decides which 0..1 values are moving; the shader blends between them. Follow the `Phase`/`Blend` pattern in `meiosis.rs`.
- New discoveries go at the end of the `Discovery` enum in `progress.rs`, and also in `ALL`, `name()` and `line()`. The test `all_lists_every_discovery` catches a missed one. The line is shown on screen: cryptic, serious, short, lowercase, never cute, never an explanation.
- Add a test in the module that drives the gesture through `Game` and checks the discovery.
- Never say what the object is: not in code comments, not on screen, not in the README.

## The shader compile budget

On 2026-10-28 the live site crashed a desktop browser. The shader had grown to take about 11 seconds to compile, and some GPU drivers hang on that. The cause was that the compiler copies `scene()` into every place it is called, and unrolls every loop it can; a 12-piece loop inside `scene()` multiplied across dozens of copies. It now compiles in under a second. To keep it that way:

- **Every loop starts from `ZERO`**, not `0`. `ZERO` is a zero the compiler can't see through, so the loop stays a loop instead of being unrolled.
- **Don't add new calls to `scene()`, `shadeLiquid()` or `calcNormal()`.** Add your shape inside `scene()` or your material inside the existing call instead.
- **Grow `marchSpan()` with the scene.** Rays are marched only between the bounds it builds (spheres around the body, its drops, tendrils and the other one, and a layer over the floor). A new shape that reaches past them is cut off; give it its own bound there. On real GPUs a branch on a uniform skips its code; SwiftShader runs it anyway, so fewer `scene()` calls is the only speedup that shows there.
- **Measure before and after** any shader change: `./build.sh && node scripts/shader-time.mjs` loads the page in headless Chromium at a tiny viewport (48×32), a fresh browser each run, and times how long until it is ready. If it gets noticeably slower, find out why before merging.
- On-screen effects that don't need the shader belong in the page (CSS, `glyph.js`), not in `shader.frag`.

The page also guards against slow machines: the loader stays up while the shader compiles in the background (`KHR_parallel_shader_compile`), large screens render at most about 1.6 million pixels and scale up, and a lost WebGL context is recovered.

## Words on screen

Every visible line (`#hint`, `#toast`, `#count`, `#thought`, `#senses`) carries `data-glyph`. `#toast` and `#thought` are `aria-hidden`; `platform/web/` writes the same words in plain letters into `#said`, a visually hidden live region, so a screen reader never reads the decoding. The Rust side only sets text and classes; `web/glyph.js` watches those elements and decodes each letter through circles, shapes, signs and markup before it settles. New on-screen text should carry `data-glyph` too.

Fonts are self-hosted from Fontsource. The math font is cut down to just the glyphs `glyph.js` uses, so if you add a glyph there, subset the font again (`web/fonts/README.md` says how).

## Saved state

Five `localStorage` keys, all in the player's browser (the one answer that leaves it is described under Where it runs):

- `the-object.found`: which discoveries they have made.
- `the-object.visits`: how many times they have come back.
- `the-object.contact`: what the object has learned about them (`contact.rs`).
- `the-object.remembrance`: what it remembers of how it was treated (`remembrance.rs`): when it first met them and when they were last here, for each kind of deed how many times ever and a fading recent sum, and the scar, if there is one (which way it faces and how plain it is). Never how it feels about them; that is worked out from these each time.
- `the-object.senses`: only whether they ever told the browser no to the microphone, the camera, feeling the phone or words leaving the page (`hear=no;see=no;feel=no;words=no`), so it never asks again (`senses/`).

Words are made by the browser's own speech recognition: on the device where it can (then nothing leaves the page), and otherwise only once the player has tapped `say something.`, whose line says the voice goes to the browser's maker. Either way the text that comes back is read for a few words and dropped. The microphone and camera are never recorded, kept or sent: each frame `devices.rs` hands `senses/` one window of samples or one 32 × 24 picture, which become a few numbers and are dropped. Nothing is played back, nothing is drawn. Inside a frame that forbids them (itch.io's embed), it never asks.

Clearing site data starts the object over.
