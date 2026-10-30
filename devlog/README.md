# Devlog

What changed, told the way the object would want it told: plainly, without saying what it is or how to find anything. `CHANGELOG.md` is the exact record; these are for people.

Each post is ready to paste into an itch.io devlog (https://antoniwan.itch.io/the-object/devlog): the first line is the title, the italic line under it is the day and the versions, and the rest is the body. itch posts are made by hand; nothing here publishes itself.

## One a day

From 29 october 2026 there is at most one post a day, `YYYY-MM-DD.md`, covering every version released that day, and it is posted by the end of the day if anything shipped. A pull request no longer writes a post of its own; it bumps the version and writes its changelog entry as before.

- The day is the changelog's: the date `scripts/version.sh` wrote (UTC).
- The italic line names the day and the versions: `*29 october 2026 · versions 1.11.0 to 1.16.5*`, or `*30 october 2026 · version 1.17.0*` for one.
- Write it from the changelog entries since the last post. The first post of a day starts where the last one ended. If more versions land the same day after the post is written, the post is extended to reach them.
- `cargo test` fails once a day has gone by whose post is missing or stops short of that day's last version, and when a post names a day or a version the changelog does not have.

Posts 01 to 36 are from before: one per version from 1.2.0, ranges before that. 26 to 36 (1.11.0 to 1.16.4) were never posted on their own; `2026-10-29.md` carries them.

## How to write one

- Lowercase titles, sentence-case body. Short declarative sentences. No exclamation marks, no "excited to announce".
- Say what it does now, never how to make it do it. No gestures, no counts, no tips. "It can be cut" is fine; "swipe fast through it" is not.
- Never name it. Never explain the arc. It is allowed to sound slightly off.
- Lead with what a player would notice; quiet versions get a sentence or two at the end.
- Be honest about what broke. The crashes are part of the record.

## Pictures

A post carries a short looping GIF of each change a player can see, placed where the paragraph about it ends, as an image line of its own pointing at the GIF on GitHub, so nothing has to be uploaded by hand:

```
![](https://raw.githubusercontent.com/antoniwan/the-object/main/devlog/2026-10-29/dust.gif)
```

The GIFs are committed next to the post, in `devlog/YYYY-MM-DD/`, in the same pull request, and are only served once it is merged. They are filmed by `scripts/gif.mjs` from scenes in `scripts/gif-scenes.mjs`. `cargo test` fails when a post points at a GIF that is not in the repository, or at anything but `main`. A copy of the post and its GIFs also goes to the project's shared folder.

Each post has its HTML beside it, `devlog/YYYY-MM-DD.html`, made by `node scripts/itch-html.mjs devlog/YYYY-MM-DD.md` and remade whenever the post changes; `npm test` fails when it is missing or out of date. To post it: the first line of the `.md` goes in the itch.io devlog's title, and the whole `.html` goes in its body, in the editor's HTML view. The pictures come with it, served from GitHub.

```
./build.sh                                            # the page has to be built first
export NODE_PATH=$(npm root -g)                       # Playwright and its Chromium (npm i -g playwright), and ffmpeg
node scripts/gif.mjs --list                           # the scenes there are
node scripts/gif.mjs dust tree --out devlog/2026-10-30  # devlog/2026-10-30/dust.gif, .../tree.gif
```

It plays each scene in headless Chromium on a clock that only moves when the scene moves it: every finger, key and wait lands on the same frame every run, and a slow software GPU still gives smooth frames. Only the frames that are kept are drawn, so getting somewhere costs almost nothing. Every GIF stays in the repository for good, so keep them small: the defaults (400 wide, 12 frames a second, 112 colors) keep a take of five or six seconds around one to two MB; it warns over 2 MB. `--width`, `--fps` and `--colors` change them.

A new change to show gets a scene. Read the top of `scripts/gif-scenes.mjs`; in short:

- Places are measured from the middle of the view in view heights, as its camera is, so a scene films the same at any size.
- `t.wait(secs)` lets its time run; `t.drag(path, secs)`, `t.tap`, `t.hold`, `t.key` are a hand; `t.roll()` starts filming (`t.roll(8)` films eight times faster than it happens) and `t.cut()` stops.
- `saved` sets its memory before the page opens (`NOW` is the time in seconds), to start somewhere a player would take visits to reach: what was found, how it remembers being treated, a scar.
- `voice: true` gives it a quiet microphone and a stand-in for the browser's words: `t.asked('speak')` waits for it to ask, `t.click('toast')` takes its word, `t.say('hello')` is heard as if said aloud.
- `arrival: true` films the page arriving instead, before it wakes, as it happens on a slow network, slowed and sped back up (`arrive` in `scripts/gif.mjs`); it takes no fingers.
- `await t.snap('label')` saves a still to `<out>/<scene>-label.png` to see where things are while working one out.

Look at every GIF before it goes in a post: a scene that drifted shows nothing, or the wrong thing.
