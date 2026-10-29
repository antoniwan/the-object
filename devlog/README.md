# Devlog

What changed, told the way the object would want it told: plainly, without saying what it is or how to find anything. One post per version, oldest first: every pull request that bumps the version adds one, named `NN-title.md` with the next number. `CHANGELOG.md` is the exact record; these are for people.

Each post is ready to paste into an itch.io devlog (https://antoniwan.itch.io/the-object/devlog): the first line is the title, the italic line under it is the version and date, and the rest is the body. itch posts are made by hand; nothing here publishes itself.

How to write the next one:

- Lowercase titles, sentence-case body. Short declarative sentences. No exclamation marks, no "excited to announce".
- Say what it does now, never how to make it do it. No gestures, no counts, no tips. "It can be cut" is fine; "swipe fast through it" is not.
- Never name it. Never explain the arc. It is allowed to sound slightly off.
- Be honest about what broke. The crashes are part of the record.
- The italic line names the version exactly (`*version 1.8.4 · 29 october 2026*`), so it lines up with the changelog. `cargo test` fails when the current version has no post.
- Posts 01 to 09 cover ranges of versions, written before this rule; every version from 1.2.0 on has its own.
- A patch with nothing to find still gets a post. Two or three sentences is enough.
