# Changelog

Every version that has been live, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html), read for a piece you touch rather than an API:

- **major**: the arc changes (how it learns you, what it does at the end), or something a player kept between visits is lost.
- **minor**: a new verb, a new form, or anything new a player can do or see.
- **patch**: fixes, speed, words, metadata, hosting and documentation.

Before 1.0.0, the first release outside its own site, everything new was a minor version, the arc included.

Every merge to `main` is deployed, so every pull request is a release and carries its own entry. `CONTRIBUTING.md` says how.

## [Unreleased]

## [1.5.0] - 2026-10-28

### Added
- A phone in the hand is the room: tipped, the room tips with it, as far as it is tipped; level is however it was held. (#43)
- Shaken, it sloshes against the far wall from each jolt and is upset; keep on and it comes loose, drops flung the way it was thrown, until the room goes dark. (#43)
- Where the browser must ask before the page can feel the phone, it asks once a visit, from early in the arc: `hold the room.` A no is answered and never asked again. On a desk nothing changes. (#43)
- Three thoughts for a room that has never moved. (#43)

## [1.4.1] - 2026-10-28

### Changed
- On a first visit its thoughts come sooner and closer together; after that it waits as long as before. (#42)

## [1.4.0] - 2026-10-28

### Added
- Late in the arc it asks to hear you (`speak.`) and later to see you (`show yourself.`), once a visit; only a tap on that word lets the browser ask. A no is answered in its voice and never asked again. (#41)
- Heard: a voice makes it tremble where the pitch lands, a note held and let go is hummed back, a whisper stills it, a breath blows a drop off it, a shout upsets it, a sudden sound makes it flinch. (#41)
- Seen: it turns toward where you move, flinches from fast movement, the room goes dark while the lens is covered, and the eye follows you. (#41)
- A quiet mark in the corner while it hears or sees; tapping it, pressing Escape or leaving the tab stops both. Sound and picture become a few numbers in the browser and are dropped; nothing is recorded or sent. (#41)

## [1.3.1] - 2026-10-28

### Added
- Thoughts for the keyboard's verbs: letters that have not landed, a note it would sing back, a breath it would match, a room that could tip, and its word not yet said back. (#40)

## [1.3.0] - 2026-10-28

### Added
- When nothing has been found for a while and no hand is on it, a line of what it is thinking sometimes surfaces under it: about where it is, what it has not yet been, and later about you. Never twice in a visit, never how. (#39)

## [1.2.0] - 2026-10-28

### Added
- The keyboard reaches it too, in ways a hand cannot: each letter lands where its key sits, a held letter hums, space breathes with it, the arrows tip the room. (#38)
- Five discoveries for these, the last for whoever answers its word. (#38)

## [1.1.2] - 2026-10-28

### Added
- A version number, kept in `Cargo.toml` and repeated in the page's metadata, its structured data and `llms.txt`; tests fail when they disagree.
- This changelog, rebuilt from every pull request so far, and a tag for each version.
- Devlogs, from the first afternoon to today, in `devlog/`.
- Each merge to `main` tags its version and publishes a GitHub release with that version's notes.

### Changed
- itch.io builds carry the version and the short commit (`1.1.2+abc1234`) instead of the commit alone.

## [1.1.1] - 2026-10-28

### Added
- `DESIGN.md`: the palettes, type, timing, sound, voice and gesture grammar already in the code, and a checklist for adding a form. `CLAUDE.md` points every session at it first. (#34)
- The river's colors in the design system. (#35)

### Changed
- The voice guide: late in the contact arc it may ask, in a word or two; it still never explains how. (#36)

## [1.1.0] - 2026-10-28

### Added
- Crowded together anywhere but the middle, the twelve melt and run as a dark red river. Tapped, it stands up in columns; enough of them at once and it beats in their rhythm, drains, and the body gathers out of it red, keeping the beat a while. Three discoveries. (#33)

## [1.0.2] - 2026-10-28

### Added
- Every production deploy also pushes the build to itch.io, so one merge updates both. A failed push never stops the site. (#29)
- The itch.io page is named as the same work in the structured data, `llms.txt` and the README. (#31, #32)

### Changed
- itch.io builds are versioned by short commit. (#30)

## [1.0.1] - 2026-10-28

### Fixed
- The page crashed Edge on Windows again: the scene had crept back to several call sites in the shader. It is compiled once now, about 1 s instead of about 6 s. (#24)

## [1.0.0] - 2026-10-28

The first release outside the site.

### Added
- An itch.io build: the site packed with its page at the root, without the analytics, the rating call or root-absolute links. (#28)

## [0.10.0] - 2026-10-28

### Added
- After contact, it holds out one drop. How long it is kept is the only answer that ever leaves the browser, counted anonymously and shown as a rating once there is one. (#27)

## [0.9.2] - 2026-10-28

### Changed
- Moved to https://the-object.antoniwan.online; the old address redirects, front page included. (#25, #26)
- Only `main` deploys; branches build nothing. (#16)

## [0.9.1] - 2026-10-28

### Added
- A face for strangers: Open Graph and Twitter cards, a share image, structured data, robots and sitemap. (#18)
- `llms.txt`. (#19)
- `CONTRIBUTING.md`, and a README that says less. (#20)
- Icons and a web manifest. (#21)

### Fixed
- A second finger now drops what the first began; a third lifting no longer ends a squeeze; its rhythm, answered, is no longer echoed back. (#22)

### Changed
- Faster start: uniforms looked up once, a stripped wasm fetched early, cached fonts. (#23)

## [0.9.0] - 2026-10-28

The arc arrives: what it was always about.

### Added
- It learns you back, in stages unlocked by how much you have found: regard, echo, its turn, and once, contact. What it learns is kept between visits. (#17)

## [0.8.0] - 2026-10-28

### Added
- Four verbs that each make it something else: a flower, a thread longer than the room, a universe inside it, a box that opens. Each has a way back. Thirteen discoveries. (#15)

## [0.7.0] - 2026-10-28

### Added
- The twelve pieces have a life of their own once they land: they ring, can be carried and planted, weather, and mend. (#14)

## [0.6.0] - 2026-10-28

### Added
- The liquid floor becomes a mirror, ice, a deep and a fountain, and what rises out of it keeps something of what the floor was. Seven discoveries. (#13)

## [0.5.0] - 2026-10-28

### Added
- After two: cleavage into many, grain, an eye, cells that die, and a stone that thaws. (#12)

## [0.4.0] - 2026-10-28

### Added
- Every word arrives as glyphs first and leaves the same way, colored by it and thrown from where it is. Self-hosted fonts; the count is roman. (#10)
- Anonymous visit counting with Vercel Web Analytics. (#11)

### Changed
- Discovery lines, the hint and the loader say less plainly. (#10)

## [0.3.1] - 2026-10-28

### Fixed
- The shader took about 11 s to compile and could hang the browser. Loops no longer unroll and the liquid is shaded in one place: about 0.6 s. The page now waits behind a veil, renders at most 1.6 million pixels, and recovers a lost context. (#8)
- Discoveries newer than the first fifteen were forgotten between visits. (#9)

## [0.3.0] - 2026-10-28

### Added
- Three ways to change it for good: cut it and it divides, throw it down hard and the floor takes it, wound it enough and it bursts into twelve. (#1, #3)

### Changed
- Hosted on Vercel; `web/` is the site, nothing built is committed. (#4, #5, #6, #7)

## [0.2.0] - 2026-10-28

### Added
- Held still, it changes color; pieces carry their color and mix. Hurried, it boils. Left alone, it reaches. Free enough of it and the room goes dark. A tap in empty space calls it. (#2)
- Sound, all synthesized.
- Discoveries are kept between visits.

## [0.1.0] - 2026-10-28

### Added
- A liquid in a white room, drawn by one raymarching shader from Rust compiled to WebAssembly. It can be poked, stretched, torn, merged, squeezed and flung, and looked around.

[Unreleased]: https://github.com/antoniwan/the-object/compare/v1.5.0...HEAD
[1.5.0]: https://github.com/antoniwan/the-object/compare/v1.4.1...v1.5.0
[1.4.1]: https://github.com/antoniwan/the-object/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/antoniwan/the-object/compare/v1.3.1...v1.4.0
[1.3.1]: https://github.com/antoniwan/the-object/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/antoniwan/the-object/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/antoniwan/the-object/compare/v1.1.2...v1.2.0
[1.1.2]: https://github.com/antoniwan/the-object/compare/v1.1.1...v1.1.2
[1.1.1]: https://github.com/antoniwan/the-object/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/antoniwan/the-object/compare/v1.0.2...v1.1.0
[1.0.2]: https://github.com/antoniwan/the-object/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/antoniwan/the-object/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/antoniwan/the-object/compare/v0.10.0...v1.0.0
[0.10.0]: https://github.com/antoniwan/the-object/compare/v0.9.2...v0.10.0
[0.9.2]: https://github.com/antoniwan/the-object/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/antoniwan/the-object/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/antoniwan/the-object/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/antoniwan/the-object/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/antoniwan/the-object/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/antoniwan/the-object/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/antoniwan/the-object/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/antoniwan/the-object/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/antoniwan/the-object/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/antoniwan/the-object/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/antoniwan/the-object/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/antoniwan/the-object/releases/tag/v0.1.0
