# How the object looks, moves, sounds and speaks

This is the design system. Everything in it is taken from what is already built; the source of each value is named so it can be checked. When the code and this file disagree, the code is right and this file should be fixed in the same pull request.

`CONTRIBUTING.md` says how the code is put together. This file says what it should feel like.

## 1. What it is about

First contact. The player meets something they do not understand, in a white room. The feeling to protect is curiosity: is it alive, is it aware, what is it. Over many visits the curiosity becomes mutual. The object learns the player, then answers, then once reaches back (`contact.rs`). After that it is no longer alone: something else comes into the room (`other.rs`), and the curiosity has a third side. Neither of them is ever named; they are told apart only by what they are like (the soft one, the hard one, the other one).

Three rules follow from that, and every other rule bends to them:

1. **Never say what it is.** Not on screen, not in a comment, not in the README, not in a commit message. Describe what it does and what it seems to do. Leave the noun empty.
2. **It is not a toy and not a pet.** It does not perform, reward, thank or cheer. It reacts, remembers, and sometimes decides.
3. **Nothing jumps.** Every change is a blend the eye can follow. A cut may flash; a form may never pop.

## 2. Color

The world is almost white, the object is almost white, and color is something the object earns or is given. Use color the way a wound uses red: rarely, and because something happened.

Color at rest lives in light, not in paint: the film on its skin, the room bent through it, the light it gathers on the floor. The room itself stays white. Saturated color over the whole screen belongs only to a moment (see *The moment* below), and fades.

### The room

| Token | Hex | Where |
| --- | --- | --- |
| `room` | `#f8f7f5` | page background, loader veil, `theme-color`, shader `bg` `vec3(0.972, 0.970, 0.962)` |
| `room-vignette` | room darkened by `0.015 × distance from center` | `shader.frag` `background()` |
| `room-sky-low` | `#b2b5bd` | lower half of the reflected environment, `env()` |
| `room-lights` | key `#ffffff` × 1.4, fill `#bfb3a8` × 1 | the two softboxes it reflects, hard-edged, `env()` |
| `room-gels` | cold `#4d6b9e` low behind it, warm `#8c5c4d` low to its right | the only color the room has, and only in reflections, `env()` |
| `night` | `#030308` | the night sky, `nightSky()` `vec3(0.012, 0.010, 0.032)`, with stars |

### The object (its own palettes)

| Palette | Hex | What it means | Source |
| --- | --- | --- | --- |
| pearl | `#f5f5fa` body, `#b2bdd6` deep, `#f7f7fc` lit | at rest; milky, light passing through | `sim.rs` `PEARL`, `shadeLiquid()` |
| pearl film | soap-film interference (film 0.2 – 0.7 µm, n 1.33): 5% across its face, up to 55% at the edges; thinner and brighter while it looks at you | it is never quite one color | `filmColor()`, `filmThickness()` |
| bent light | the room through it, red, green and blue bent at 0.735 / 0.75 / 0.77; 16% looking straight through | it is not quite solid | `dispersed()` |
| focus | its own color, a hot point in its shadow with a ring split red / green / blue outside it | light passes through it and gathers | `caustic()` |
| heat | `#ff294d` from inside, brightest where it is thickest; beats at 1.2 Hz while wounded | upset, something shows through | `shadeLiquid()` `hurt` |
| flush | `#c74c6b` (words), `#bf7380` multiplier (body) | upset: agitation, or the wounds before a burst | `aura.rs` `FLUSH`, `uAgit` |
| warmth | a slow cosine cycle: `#ff8e8e` `#d966da` `#8c8bff` `#66d7d7` `#8cff8b` `#d9da66` | held still, it changes color through these | `sim.rs` `palette()` |
| night lens | the sky seen through it, a rim of its own color | six drops free and the room goes dark | `shadeLiquid()` `uNight` |
| quicksilver | the environment × `#dbe0eb` | pulled out of a mirror | `uCoat.x` |
| ink | `#050508` plus a little of its tint | pulled out of the deep | `uCoat.y` |
| cell | `#fbecf1` body, `#b24580` rim, `#6b3d99` nucleus, `#52246b` nucleolus | divided; alive in a more familiar way | `shadeCell()` |
| seam | `#ff59a6` | the glow of the newest cut, fades in a fraction of a second | `shadeCell()` `uDiv.x` |
| wounded / dying cell | `#ffb2c2` multiplier / `#99999e` | hurt, then emptied to grey | `shadeCell()` `fate` |
| eye | `#1a0824` pupil to `#9442a8` iris | one cell stops agreeing and looks | `shadeCell()` |
| stone | `#040405`, reflection only | every seam closed; the 1 : 4 : 9 slab | `shadeStone()` |
| mere | mirror `#edf0f5`, ice `#c7dbed` to `#f7fcff`, cracks `#526680`, deep ink `#04050a` | the liquid floor's states | `mereSurface()`, `mere.rs` |
| blood | `#6b050d` river, `#9e0d17` at its banks, `#360306` in its depths, sheen `#ffdbdb` | the twelve, crowded together anywhere but the middle, melt and run as a river; brighter (×1.9) on each heartbeat | `river.rs` `BLOOD`, `shadeBlood()` |
| blooded body | `#87353c` (pearl 20%, blood 80%) | what comes back up out of the river, still keeping the beat | `game.rs` after the river drains |
| box light | `#fff0c7` | the inside of the box when the lid lifts | `uBox` |
| pollen | `#fac242` | the flower's heart | `formTint()` |
| dust | `#ccc7d4`, grainy, a few grains still glinting in the film's colors | the flower let go of; drifts on the floor, and the cloud they fall from | `shader.frag` `DUST`, `dustFloor()` |
| mound | `#b3abb3`, faint layers, the film back at its grazing edges; lit from inside while pressed | the dust heaped and settled; the only thing it is that stands on the ground | `MOUND`, `shadeHeap()` |
| fold | `#080412` the dark between, then a cosine run through `#e50839` `#6b00db` `#0091e0` `#6b9f3e` (`foldPal()`), squared, so mostly the violet and blue of the galaxy's arms come through, on an orbit-trap fractal | the galaxy's own colors let out into the room; each copy of it takes a different turn of the run | `shader.frag` fold block |
| fold seams | `#f2d9ff`, flaring on a new mirror | where the mirrors meet | `foldSky()` |
| backward | the fold's colors rotated a channel (`col.zxy`), cooler | its time running the other way | `foldSky()` `uFold.w` |
| the other one | `#05060a` body, the room in its faces, a full-spectrum cosine at its edges (`fire`, 0.35 → 1) | hard, cold, not from here; the colors split at its edges, more each time it is given back | `shadeOther()` |
| the other, struck | `#bdd1ff` | it rings; brightest at its edges | `shadeOther()` `uOther2.w` |
| the other, kept | the first's color at the time it was given back, 60% each time, glowing faintly at its grazing faces | what it took from inside the first | `other.rs` `kept` |
| cold | `#9ea3f5` | the first cools toward this while it holds the other inside | `other.rs` `COLD` |

### The moment

Every shock (whatever shakes the room: a cut, a slam, the burst, the lid, the slab, a heartbeat of the river, a jolt of the phone) throws a ring of color out from where it is on screen, kicks the light, and washes the room in the color of what happened. Strength is the shake over 0.09, so a burst and a slam are whole and a jolt is faint. `drama()` in `shader.frag`, fed by `camera.rs` `shock()`.

| What happened | Wash | Hex |
| --- | --- | --- |
| a blow, a fall, anything else | cold | `#80a8ff` |
| a cut | the seam's pink | `#ff4d9e` |
| the lid | the box's light | `#ffd173` |
| the river's beat | blood | `#bf0a17` |
| coming apart into dust | the dust | `#ccc7d4` |
| the burst | every color at once, around it | spectrum by angle |
| upset or wounded (no shock needed) | the edges bruise toward `#473859`, then `#8c1429` | at most 60%, less at night |

### The twelve

When it bursts, each piece is its own material. Index-matched in `burst.rs` `CONSISTENCIES`, `landscape.rs` `TINTS` and the shader's `TCOL`; change all three together.

| # | Material | Hex | | # | Material | Hex |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | milk | `#f2f0f7` | | 6 | velvet | `#29308f` |
| 1 | chrome | `#ccd1db` | | 7 | wax | `#f5e6c7` |
| 2 | glass | `#dbf5fa` | | 8 | ice | `#cce8ff` |
| 3 | jelly | `#ed3873` | | 9 | rubber | `#f25c29` |
| 4 | stone | `#9e9487` | | 10 | obsidian | `#0d0d12` |
| 5 | honey | `#f59924` | | 11 | moss | `#5c8c42` |

### The words

| Token | Hex | Where |
| --- | --- | --- |
| `--tint` | from the object every frame; at rest about `#787884` | glyphs in the middle of decoding, the alien reply (`aura.rs`) |
| `words` | `#7d7d86` | hint, toast line, count |
| `words-strong` | `#4a4a52` | the toast's name (the discovery's one word) |
| `words-faint` | `#9a9aa3` | the loader's whisper, errors, its thoughts (at 80%) |
| `words-night` | `#6d7088`, names `#c9cbe0` | the same, after dark (3 s transition) |
| loader film | `#f6c3d8` `#c6dcf6` `#d4f3e3` `#f3e6c2` | conic gradient over the loader's pearl, soft-light at 55% |

**Rules.** Words take the object's color, never their own. Never put a saturated color on the page that the object did not produce. No pure black and no pure white on screen except where the shader computes them (the burst flash goes to white on purpose). There is no accent color, no button color, no link color, because there are no buttons and no links on the play surface.

## 3. Type

Three free fonts from Fontsource, self-hosted in `web/fonts/` with their licenses. No other fonts, and nothing loaded from a font CDN.

| Role | Font | Size / line | Tracking | Case | Used for |
| --- | --- | --- | --- | --- | --- |
| voice | Cormorant Garamond italic 400 | 17px / 1.4 | 0.04em | lowercase | hint, discovery lines |
| name | Cormorant Garamond upright 500 | 13px / 1.4 | 0.5em | lowercase | the discovery's name above its line |
| whisper | Cormorant Garamond italic 400 | 16px / 1.4 | 0.08em | lowercase | loader lines |
| thought | Cormorant Garamond italic 400 | 15px / 1.4 | 0.1em | lowercase | what it is thinking (`thoughts.rs`) |
| count | JetBrains Mono 400 | 10px / 1 | 0.3em | lowercase roman numerals | how many it has shown you (`aura.rs` `roman()`) |
| system | JetBrains Mono 400 | 10px / 1.4 | normal | as is | errors, and the one line under `say something.` that says where words go |
| alien | Noto Sans Math, subset ("Glyphs") | inherits | 0.4em | n/a | its reply at contact, the kept-steps answer |

Everything is lowercase. Numbers are spelled out or roman, never digits, except the slab's `1 : 4 : 9`, which is a ratio and looks like one.

### Glyph decode

Every visible word arrives as something else first (`web/glyph.js`). Each letter passes through four stages, smaller than the letter it becomes, then settles:

| Stage | Pool | Scale | Color |
| --- | --- | --- | --- |
| circles | `○◌◍◎●◐◑◒◓◯◦∘⊙⊚⊛⦾⦿◉` | 0.62 | tint at 40% |
| shapes | `△▽◇◆□■▲▼◢◣◤◥◊⬡⬢⬠▱▰⟁⧖` | 0.70 | tint at 70% |
| signs | `∴∵∷⊕⊗☉☽✶⟡⌬⍜⍟` | 0.82 | tint, glowing |
| markup | `<>/{}[]=#;&\|\_~^*+` | 0.86 | mono, tint mixed 45% with the text color |

The signs are the object's own alphabet: the same twelve appear in `contact.rs` `SIGNS` for its reply. Use them for anything it says in its own voice. Do not add new signs casually; each one added means re-subsetting the math font (`web/fonts/README.md`).

Every element that shows words carries `data-glyph`. Letters are thrown away from the object's screen position (`--ox`, `--oy`), harder the more upset it is (`--unrest`).

## 4. Space and layout

There is one screen, full bleed, no scroll, no chrome.

| Token | Value | Where |
| --- | --- | --- |
| `edge-v` | `9vh` | the hint sits 9vh from the bottom; the toast 9vh from the top |
| `thought` | `15vh` from the bottom, under the object's screen position (`--ox`, kept between 28% and 72%) | a thought; it drifts with the object over 3 s |
| `edge-corner` | `16px` right, `14px` bottom | the count |
| `edge-corner-left` | `16px` left, `14px` bottom | the marks while it hears or sees: `◍` hearing, `◉` seeing, in its tint at 70%, breathing; a mark it asked for and was not given stays at 28%, still |
| `safe-area` | `env(safe-area-inset-*)` added to both corners | so a phone's rounded edge and home bar never sit on the marks or the count |
| `reach` | `14px 22px` around its word, `12px` around the marks | how far past what is drawn a tap still lands; the look does not move |
| `toast-name-gap` | `6px` | between the name and its line |
| `seed` | `18px` | the loader's pearl, centered |
| `whisper-offset` | `50% + 88px` | the loader's line, under the seed |
| `max-pixels` | `1.6e6` | the canvas renders at most this many and scales up (`web.rs`) |

The object is always the center of attention. Words go above it (what was found) and below it (what to try, and what it is thinking), and nowhere else. Never cover the object with interface. Never add a menu, a settings icon, a close button or a progress bar. No key is announced: `m` mutes, `Escape` stops it hearing and seeing, and the rest of the keyboard is a second way to reach it (section 8), found by pressing. The one exception to "no interface" is the pair of marks while it hears or sees: privacy is never hidden, and the marks stop it when tapped. Once it has asked to hear or see you in a visit and was not given it (the word went by, or it was stopped since), that mark stays in the corner, faint and still, until the visit ends; tapped, it is the same as tapping the word. It never appears before it has asked, and never after a no.

**Touch and access.** Nothing on the page can be selected, long-pressed or flashed by a tap, and the page does not pull or bounce. Whatever is tappable reaches further than it is drawn. While its word is up, Enter takes it. The toast and the thought are hidden from screen readers, which decode them as noise; a hidden `#said` line reads them out in plain letters instead (its signs are left unread), and adds, for its word only, that Enter allows it. The canvas is described as the README describes it.

## 5. Motion and time

The object is liquid, so motion is springy and continuous; the page is quiet, so its motion is slow and eased. Everything decays exponentially (`x += (goal - x) * (1 - exp(-rate * dt))`), never linearly.

### Gesture thresholds (`game.rs`)

| Token | Value | Meaning |
| --- | --- | --- |
| `TAP_SLOP` | 0.012 canvas heights | moved less than this, it was a tap |
| `HOLD` | 0.55 s | held still this long, it becomes warming |
| `IDLE_WATCH` | 12 s | left alone this long, it starts watching the pointer |
| `THROW_SPEED` | 2.2 canvas heights / s | a flick faster than this throws it |
| `SLICE_MAX_SECS` | 0.35 s | a swipe must cross it this fast to cut |
| `PINCH_MAX_SECS` | 0.4 s | a tug let go this fast is a pinch |
| `TAUT_SECS` | 0.9 s | held nearly tearing this long, it becomes thread |
| `TUG_MAX` | 0.12 canvas heights | on the flower, a pull shorter than this let go within `PINCH_MAX_SECS` is a pinch (`forms/bloom.rs`) |
| `HEAP_AT` | 0.9 of the dust | this much swept into one drift and it heaps itself (`forms/dust.rs`) |
| `RISE_HOLD_SECS` | 1.2 s | the mound pressed this long, it rises |
| `GATHER_SELF_AFTER` | 40 s | dust left untouched this long starts to drift together by itself |
| `HUM_AFTER` | 0.45 s | a letter held this long starts to hum (`keys.rs`) |
| `OFFER_WINDOW` | 9 s | its word (`speak.`, `show yourself.`) can be tapped this long (`senses.rs`) |
| `OFFER_NOT_BEFORE` / `OFFER_IDLE` / `OFFER_GAP` | 20 s / 5 s / 40 s | it asks no sooner into a visit, only when untouched this long, and this far apart |
| `GAP` (words) | 0.14 s | between one spoken verb and the next; at most eight wait (`words.rs`) |
| `GREET_GAP` | 6 s | a greeting is answered at most this often |
| `HELD_SECS` / `HUM_SECS` | 0.8 s / 2.4 s | a note held this steady comes back, for this long |
| `FOLD_TURNS` / `UNFOLD_TURNS` | 3 / 2 turns | stirred on past the universe this far, the room folds; this far the other way, it folds back up (`forms/fold.rs`) |
| `BACKWARD_SECS` | 0.6 s | held still this long in the fold, its time turns round |
| `BREATH_SLOW` | 1.2 s | space held this long is a breath; three in a row, none more than 9 s apart, and it follows |
| `FULL_TILT` / `DEAD` | 18° / 3° | a phone tipped this far tips the room all the way; less is only a hand (`motion.rs`) |
| `SETTLE` (tilt) | 20 s | however the phone is held slowly becomes level |
| `JOLT` | 13 m/s² beyond gravity | a push this hard is a jolt; walking is under it |
| `SHAKE_JOLTS` / `SHAKE_WINDOW` | 3 in 1.2 s | that many jolts is a shake |
| `SHED_AFTER` / `SHED_EVERY` | 1.2 s / 0.4 s | shaken this long without stopping, it comes loose, a drop each this often |

### Transformation durations

Forms take between two and four seconds to arrive and a little less to leave. A form you can watch happen is better than a fast one.

| Kind | Range | Examples |
| --- | --- | --- |
| hit-stop | 0.12 s | the freeze right after a cut (`meiosis.rs`) |
| quick change | 0.5 – 1.5 s | a bud growing, ice thawing, the floor taking it in |
| becoming | 2.2 – 4.2 s | dividing 3.0, rejoining 2.2, thread 2.6, cosmos 3.5, flower 3.2, lake spreading 2.6, the fold 4.2, coming apart into dust 3.0, heaping into a mound 3.2 |
| letting go | 1.6 – 3.0 s | folding 2.4, settling 2.8, gathering 2.8, the river draining 2.4, the room folding back up 3.0, rising out of the mound 2.6 |
| the river | rise 0.45, stand 5.5, fall 0.6 s per column; melt 2.2, spread 5.0 | seven beats in the rhythm the columns were raised in; the body then keeps it for forty more, each fainter |
| the other one | arrive 4.0, taken in 2.6, given back 2.2 s | pressed together 1.1 s before it is taken; squeezed 0.5 s to give it back; three hurts within 4 s and it stands between for 7 s; nobody doing anything for 10 s and they circle, a turn every 21 s. It comes no sooner than 20 s into a visit, after 6 s of stillness |
| kept together | follow 5 / s, lean 2 / s, turn 0.5 / s; a touch 0.12 s a piece; a breath 3.0 s + 0.35 s a piece | a cut steps each side back, fading `exp(-3 t)`; a drag off the ground faster than 1.5 canvas heights / s cuts instead of looking around (`kin.rs`) |
| slow life | 24 – 75 s | planted pieces growing 24, weathering 50, a coat wearing off 75 |

### Decay and recovery

| Token | Value | Meaning |
| --- | --- | --- |
| agitation | `exp(-dt / 3.0)` | an upset fades over about three seconds |
| wounds heal | after 2.5 s, over 5 s | stop poking and it recovers (`burst.rs`) |
| night | rate 0.7 / s | the room darkens and lightens slowly |
| warmth | rate 1.5 / s, cycle 0.22 / s | color drifts while held |
| remembered care | half-life 4 days | being held, sung to, answered, returned to (`remembrance.rs`) |
| remembered hurt | half-life 2 days | being upset or broken; it forgives faster than it forgets kindness |
| shock | flash `exp(-16 t)`, wash `exp(-2.2 t)`, ring 2.4 screen heights / s fading `exp(-1.8 t)`, gone at 1.5 s | the moment after anything shakes the room |

### The page

| Token | Value |
| --- | --- |
| glyph step | 85 ms per stage, 28 ms stagger, 650 ms spread, 60 ms flicker |
| toast | in over 0.6 s, stays 4.2 s; the question 7 s; the reply 10 s |
| thought | rises over 2.4 s, stays 7 s, sinks over 2.4 s; anything found or said sends it away. The first comes no sooner than 30 s into a visit, after 20 s without a discovery, 3.5 s with no hand on anything and 8 s in one place; then at most one every 45 s. On a first visit: 12 s, 10 s, 5 s in one place, then one every 25 s (`thoughts.rs` `FIRST_VISIT`) |
| hint | breathes 0.35 – 0.85 opacity every 3.5 s; fades over 1.2 s once something is found |
| night text | color eases over 3 s |
| loader | pearl pulses every 4.2 s, rings every 2.1 s, film turns every 11 s; on waking the pearl grows ninefold and blurs out over 2 s while the veil fades over 1.8 s |
| easing | `ease-in-out` for breathing, `cubic-bezier(.2, .7, 0, 1)` for arrivals, `cubic-bezier(.1, .6, .3, 1)` for rings |

`prefers-reduced-motion` stops the loader's animation and shows letters without decoding. Anything new on the page must respect it too.

## 6. Sound

Every sound is synthesized in `audio.rs`: sine and triangle oscillators, no samples, no noise, no files.

- **Envelope.** One shape for everything: an 8 ms attack, an exponential slide from one pitch to another over half the decay, an exponential fade. Chords are plucks staggered by 0.03 – 0.75 s.
- **Scale.** Major pentatonic across two octaves (`SCALE`). Anything pitched by where it was touched picks from it, so nothing the player does can sound wrong.
- **Register.** Its own body is low and soft: merging at 110 Hz, becoming at 65 Hz + 98 Hz, the night drone at 55 Hz + 82.6 Hz. Discoveries are high and quiet: E5, B5, E6, each fainter. The river's heartbeat is the lowest thing it plays: two sine thumps sliding 62 → 44 Hz and 58 → 42 Hz, the second softer; its columns climb the pentatonic scale from G3, a row raised in one drag rising with it.
- **Away.** Left for another tab, it goes silent at once (the audio context is suspended), and comes back as it was when the page is looked at again.
- **Volume.** Master 0.7. Single tones 0.02 – 0.3. Discoveries are the quietest thing it plays.
- **Timbre as meaning.** Sine for the liquid, for living and for finding. Triangle for what is hard or hostile: provocation, knocks, the ringing pieces, the thread, and the other one: its ring is a triangle on the pentatonic scale from E4 with a bell's partial (×2.76) over it, 2.8 s, low as it arrives and high when struck.
- **No music.** The continuous sounds are a hum while it is warmed, a held letter sung back an octave under the pokes (C3 pentatonic, 0.06), a note the player sang held and let go, sung back at the same pitch folded under C4 (sine, 0.06, 2.4 s), and a drone at night. While it plays anything it does not listen, so a microphone never hears it as the player. Silence is the default.

A new form gets a sound only if it changes what the object is; use `Sound::Become` before inventing one.

## 7. Voice

### On-screen lines

Each discovery has a **name** (one or two words) and a **line** (`progress.rs`). The line is what the object's world says about what just happened. It is never the game talking to the player.

**Do**

- Lowercase. One or two short sentences, ending with a full stop.
- Say what happened from the side of the thing, or what it implies. Leave the cause unspoken.
- Use plain words: hand, seam, floor, inside, room, wound, breath.
- Let it feel slightly off: a fact that should not be true, said calmly.
- Keep it under about 60 characters. The longest one today is 61.
- Address the player as "you" sparingly, and only when it is about what they did.
- **Never say how; rarely say what it wants.** No gesture, speed or tip, ever. But late in the contact arc it may ask, in its own voice: one bare imperative, a word or two, once a visit at most (`stay.` after its turn, `again.` after an echo; `hold the room.`, `speak.`, `show yourself.` and `say something.` when it asks to feel the phone, to hear, to see and to take words). Its asking is part of the curiosity becoming mutual, so it never asks before regard.

**Examples from the game**

> poke · a pressure, and then the memory of a pressure.
> merge · the seam forgets it was ever a seam.
> slam · the ground was the softer thing.
> eye · one of them stopped agreeing. it looks.
> pluck · it was always a note. it only needed a length.
> its turn · you stopped. it did not.

**Don't**

| Instead of | Because |
| --- | --- |
| "Nice! You discovered Poke!" | cheering, title case, exclamation, game voice |
| "The alien creature splits in two." | says what it is |
| "Try swiping faster to slice it." | says how; it may ask for something (`stay.`), never explain how to do it |
| "it's so squishy and cute :)" | cute |
| "Mitosis: the cell divides into two daughter cells." | explanation, biology lesson |
| "you have unlocked 14 of 66 secrets" | progress talk, digits; the count is roman and unlabeled |
| "the ineffable vessel of eternity weeps" | purple; the voice is plain, the strangeness is in the fact |

### Other text

- **The hint** is one invitation, then gone: `lay a hand on it`.
- **Thoughts** (`thoughts.rs`) are what can be seen of its mind when the player has stopped finding things. They are its hints, so they follow the strictest version of the voice: never a gesture, a verb from the grammar table, a count or a tip; suggest what it has not been yet, from its side (`it has never had a corner. it is curious about them.`). Always "it", never "i": its own words are still the signs. Early thoughts are about itself; once it has started to learn the player, the same thought turns toward them (`it does not know yet how far it goes.` becomes `it wonders how far you would take it.`). Never the same thought twice in a visit. A test in `thoughts.rs` checks the words.
- **The loader** whispers in the object's uncertainty: `something is choosing a surface`, `it has not yet agreed to a shape`, `it is shy of being seen`. On failure: `it stayed on the other side`. Never "loading" or a percentage.
- **The object's own words** are the twelve signs, never letters, until the one word `you`.
- **README and metadata** use the same register in sentence case: short declarative sentences, no adjectives of praise, nothing that names it. ("There is an object. You can touch it.")
- **Code comments** describe what the player does and what happens, plainly, in the same quiet tone. They may be precise about mechanics; they may not name it either.

## 8. Interaction grammar

The player has a few raw gestures. The gesture, where it lands and what state the world is in decide the verb; the verb, repeated or held, decides the form. `game.rs`'s header comment is the full list.

| Gesture | On | Verb | If pushed further, it becomes | Module |
| --- | --- | --- | --- | --- |
| tap | the liquid | poke | twelve pokes unhealed: burst into the twelve | `burst.rs` |
| tap | empty space | call | it reaches toward the place | `sim.rs` |
| press and drag | the liquid | stretch | far enough: tear off a drop | `sim.rs` |
| stretch and hold taut | the liquid | (taut) | a thread longer than the room | `forms/filament.rs` |
| short tug, let go at once | the liquid | pinch | five buds: a flower | `forms/bloom.rs` |
| short tug, let go at once | the flower | pinch | it comes apart into dust on the floor | `forms/dust.rs` |
| drag | the dust | sweep | drifts pushed together become one; nearly all of it in one place heaps into a mound; tap: it answers; hold: it rises back out | `forms/dust.rs` |
| drag round and round | a tendril | stir | a universe inside it | `forms/cosmos.rs` |
| keep stirring the same way | the universe | (stir on) | the room folds: it is everywhere in mirrors; tap: another mirror; hold still: its time runs backward; stir back: the room folds up | `forms/fold.rs` |
| draw a square | around it | (corners) | a box; knock three times | `forms/pandora.rs` |
| press and hold still | the liquid | warm | color shift | `sim.rs` |
| fast swipe through | the liquid | slice | two cells; again: many, then grain, an eye, stone | `meiosis.rs`, `morula.rs` |
| flick hard downward | the liquid | throw / slam | it fuses with the floor | `fusion.rs`, `mere.rs` |
| pinch or wheel | anywhere | squeeze | on cells: rejoin, or stone | `meiosis.rs`, `morula.rs` |
| drag | a drop | carry / fling | six drops free: night | `sim.rs` |
| set pieces down close together | the twelve | (kin) | as many groups as you like: carry one and its group follows; pull it away fast and it comes loose; a fast sweep across the ground between them parts them; touch one and the rest feel it after | `kin.rs` |
| crowd the twelve together | anywhere but the middle | (river) | tap: a column; drag: a row; raise thirty at once and it beats | `river.rs` |
| drag | empty space | look around | | `camera.rs` |
| nothing | | patience | it watches; later it takes its turn | `sim.rs`, `contact.rs` |
| tap | the other one, after contact | ring | it rings; the first shivers on the side facing it | `other.rs` |
| drag | the other one | carry | brought near, the first reaches a tendril toward it | `other.rs` |
| flick it at the first | the other one | clash | both recoil and the first is upset; neither gives | `other.rs` |
| press it into the first and keep pressing | the other one | (inside) | the first takes it in; squeeze and it is given back, each keeping some of the other's color | `other.rs` |
| poke the first, again and again | the liquid, while the other is there | (between) | the other places itself between you and the first | `other.rs` |
| nothing, while both are there | | (distance) | they circle each other; the first watches it instead of you | `other.rs` |
| type a letter | (keyboard) | letters | a ripple where the key sits, q high left, n low right; each its own note | `keys.rs` |
| hold a letter | (keyboard) | hum | the note sung back, the surface trembling in time, that letter's color seeping in | `keys.rs` |
| hold space, let go | (keyboard) | breath | it swells a tenth and sighs out; breathed with slowly, it breathes four more on its own, at your pace | `keys.rs` |
| arrow keys | (keyboard) | lean | the room tips; it and the loose drops slide downhill, and come home when level | `keys.rs` |
| type its word | (keyboard), after contact | said | it reaches toward you | `keys.rs` |
| tip the phone | (phone) | lean | the same as the arrows, by as much as the phone is tipped; level is however it was held | `motion.rs` |
| shake the phone | (phone) | shaken | it sloshes against the far wall from each jolt, and is upset; keep on and drops come loose, flung the way it was thrown, until the room goes dark | `motion.rs` |

**Hearing and sight** (`senses.rs`, `devices.rs`) are never taken, only given. Once it echoes, it asks `speak.`; once it takes turns, `show yourself.` Each once a visit, only when untouched and ordinary, and only the player's tap on that word lets the browser ask. A no to the browser is answered in its voice (`silence is also something it can learn.`, `it looks away. it will not ask again.`) and kept: it never asks again. What it hears is three numbers (loudness, pitch, roughness), what it sees three more (light, how much moved, where); there is no picture and no recording, and nothing leaves the page. `Escape`, a tap on the marks, or leaving the tab stops both at once. A word that went by untaken, or a sense stopped, leaves its mark faint in the corner for the rest of the visit (section 4); a tap there lets the browser ask, as the word would have.

| What the player does | Verb | What it does | Module |
| --- | --- | --- | --- |
| speak | voice | trembles where the pitch lands, low notes low on it | `senses.rs` |
| hold a note, then stop | sing | hums the note back, the same pitch, lower | `senses.rs` |
| whisper | whisper | goes still and leans in | `senses.rs` |
| blow | blow | a drop comes off the far side | `senses.rs` |
| shout, keep shouting | shout | upset, shaking harder the longer it goes | `senses.rs` |
| a sudden loud sound, or a fast movement | startle | flinches away | `senses.rs` |
| move in front of it | seen | turns toward where you moved | `senses.rs` |
| cover the camera | blind | the room goes dark until you uncover it | `senses.rs`, `sim.rs` `dim` |
| be watched by the eye at eight cells | watched | the eye follows you | `senses.rs`, `morula.rs` |
| say a verb (poke, cut, squeeze, hold, pull, tear, pinch, throw, come; English or Spanish) | word | the same hand the pointer is does it, once for each time it is said | `words.rs` |
| greet it, in any language, having been gentle | greeting | turns to you, two rising notes, and answers in signs: the same signs for the same greeting | `words.rs` |
| greet it, having hurt it lately | unanswered | draws back and says nothing; not sure of you yet, it only turns to listen | `words.rs` |

**Words** come out of hearing and need nothing more where the browser can make them on the device: then they simply work once it hears you, and nothing leaves the page. Where the browser can only make words by sending the voice to its maker (Safari on the iPhone, some others), it asks for that separately, once it already hears you, with `say something.` and one plain line under it saying where the voice goes; only a tap on that allows it, and while it lasts the marks say so too. A no is answered (`it will listen to the sound of you instead.`) and kept. The text that comes back is read for its verbs and greetings and dropped. Whether it answers a greeting is decided by what it remembers (`remembrance.rs`), and being greeted is remembered as a small kindness.

**A phone in the hand** is the room. Tipping it tips the room and shaking it shakes the room; the object is what stays behind. Most phones let the page feel this without asking, and then it simply works; where the browser must ask first (iOS), it asks once a visit from regard on, before anything else, in its voice: `hold the room.` A no is answered (`it will stay where it is put.`) and kept, like the others. Two angles and one push a frame, used and dropped. On a desk nothing arrives and it never asks.

**The keyboard** is not a second mouse. Nothing on the pointer side is mapped to keys; each key reaches it from somewhere of its own (a letter from its place on the keyboard, space as breath, the arrows as the floor). Keys are read by place (`KeyboardEvent.code`), so the places hold on any layout. Like everything else, they act only on the ordinary liquid, and nothing explains them.

**The grammar's rules**

- A verb is a gesture the player already knows from the physical world. Prefer verbs a hand does to a soft thing: press, pull, pinch, stir, hold, cut, throw, knock.
- A form is earned by doing a verb more, longer or harder than needed, never by finding a hidden button.
- Every form has a way back, and the way back is the verb's opposite or a gentle version of it (stir back, hold to wind, tap to fold, squeeze to rejoin, hold to thaw).
- Forms start only from the ordinary liquid: gate the gesture on `World::ordinary()`. One form at a time.
- The contact arc (`contact.rs`) watches all of this and unlocks by count: regard at 6 discoveries, echo at 14, its turn at 24, contact at 36. It acts only while the world is ordinary.
- Remembrance (`remembrance.rs`) watches how it is treated, not what is found: a ledger of deeds kept between visits (held, hummed to, sung to, answered, greeted, returned to, stayed with; hurt, broken), and how glad or wary of you it is, worked out from that ledger and never stored. It shows only once a visit, from regard on, when a hand first comes near: it leans toward a hand that was kind to it (`remembered`) and pulls away from one that was not (`wary`). No number of it is ever shown. Other verbs may ask it whether it is glad of you (`kind()`), never tell the player.

## 9. Adding a new form

A checklist. Most of it is enforced by review, some of it by tests.

1. **Find the verb first.** Write the one sentence: "do X to it and it becomes Y". If X is not something a hand does, or Y needs a noun that says what the object is, stop.
2. **Give it its own file** in `src/forms/` (or its own module for something larger), with a header comment in the house tone. Add it to `forms/mod.rs`'s list and `game.rs`'s gesture list.
3. **Gate it** on `World::ordinary()` so it cannot start inside another form, and add it to `Forms::whole()` so other forms cannot start inside it.
4. **Phase and Blend.** A `Phase` enum decides which 0..1 values move; a `Blend` struct carries them to the shader. The shader only mixes. Arrival 2 – 4 s, departure a little shorter, both smoothstepped. Follow `meiosis.rs`.
5. **Shader, inside the budget.** Add the shape inside `scene()` and the material inside the existing `shadeLiquid()` call. No new calls to `scene()`, `shadeLiquid()` or `calcNormal()`. Every loop starts from `ZERO`. Measure the first frame at 48×32 before and after. Pick colors from section 2, or add the new one to section 2 in the same pull request.
6. **A way back.** The opposite or gentle verb returns it to the ordinary liquid, with its own blend.
7. **Discoveries.** Usually three: becoming, an interaction inside the form, and leaving it. Append to the end of `Discovery`, `ALL`, `name()` and `line()`. Write the lines against section 7.
8. **Sound.** `Sound::Become` on arrival; a new `Sound` only if the form has an action of its own (pluck, knock), built from the same envelope.
9. **Test** in the module: drive the gesture through `Game`, check the discovery, check it returns to `ordinary()`.
10. **Update** this file if you added a color, a timing range or a row in the grammar table, and the Atlas if you keep one.

## 10. Engineering rules that shape the design

These are in `CONTRIBUTING.md` in full. They are here because they limit what a design may ask for.

- **Shader compile budget.** One shader draws everything, and it must compile in under a second on a slow Windows/Edge machine. Loops from `ZERO`, no new `scene()`/`shadeLiquid()`/`calcNormal()` call sites, measure before and after. An effect that does not need the shader goes in the page.
- **Quality tier.** When frames stall and the page drops below 0.75 of its resolution (`web.rs` `quality`), the bent light and the focus are left out (`uFine`); the film and the moment stay. A design may not depend on them being there.
- **Module per form.** Many threads work at once; separate files keep merges clean. `game.rs` and `sim.rs` changes stay small.
- **No engine, no images, no samples.** Every shape is an equation, every sound is synthesized, every word decodes. Keep it that way.
- **Nothing leaves the browser** except the one answer described in `CONTRIBUTING.md`, and words where the player has tapped `say something.` under the line that says they go to the browser's maker. Microphone and camera are reduced to a few numbers each frame and dropped. The object's memory of the player is `localStorage`.
- **The site is the build.** `web/` is everything that is served; build output is never committed.
