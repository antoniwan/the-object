//! Tests: when it asks, what a voice, a note, a whisper, a breath, a shout and a look do to it.

use super::*;

const RATE: f32 = 48_000.0;

fn tone(f: f32, amp: f32) -> Vec<f32> {
    (0..2048).map(|i| (i as f32 / RATE * f * std::f32::consts::TAU).sin() * amp).collect()
}

/// A hiss: the same every time.
fn hiss(amp: f32) -> Vec<f32> {
    let mut s: u32 = 0x1234_5678;
    (0..2048)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32 / u32::MAX as f32 * 2.0 - 1.0) * amp
        })
        .collect()
}

fn look(stage: Stage, still: f32) -> Look {
    Look { stage, still, busy: false, fwd: v3(0.0, 0.0, -1.0), right: v3(1.0, 0.0, 0.0), up: V3::UP }
}

fn run(s: &mut Senses, w: &mut World, l: &Look, seconds: f32, mut feed: impl FnMut(&mut Senses)) -> Vec<Discovery> {
    let mut found = Vec::new();
    for _ in 0..(seconds * 60.0) as usize {
        feed(s);
        s.step(1.0 / 60.0, w, l);
        w.step(1.0 / 60.0);
        found.extend(s.drain_found());
    }
    found
}

fn open(kind: Kind) -> Senses {
    let mut s = Senses { can_hear: true, can_see: true, ..Default::default() };
    *s.leave(kind) = Leave::Open;
    s
}

#[test]
fn a_voice_has_a_pitch_and_a_hiss_does_not() {
    for f in [110.0, 220.0, 440.0, 660.0] {
        let h = hear(&tone(f, 0.2), RATE);
        let p = h.pitch.expect("a pitch");
        assert!((p - f).abs() < f * 0.02, "{f} heard as {p}");
        assert!(h.rough < 0.05);
    }
    let h = hear(&hiss(0.05), RATE);
    assert_eq!(h.pitch, None);
    assert!(h.rough > WHISPER_ROUGH, "{}", h.rough);
    assert_eq!(hear(&[0.0; 2048], RATE).level, 0.0);
}

#[test]
fn it_asks_late_once_a_visit_and_only_when_left_alone() {
    let mut w = World::new();
    let mut s = Senses { can_hear: true, can_see: true, ..Default::default() };
    run(&mut s, &mut w, &look(Stage::Regard, 30.0), 40.0, |_| {});
    assert!(s.drain_events().is_empty(), "too early in the arc");
    run(&mut s, &mut w, &look(Stage::Echo, 0.5), 5.0, |_| {});
    assert!(s.drain_events().is_empty(), "not while being touched");
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), 1.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)]);
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_WINDOW + 1.0, |_| {});
    assert_eq!(s.hear, Leave::Closed, "the word went untaken");
    assert!(s.drain_events().is_empty(), "and it does not ask to see straight after");
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_GAP - OFFER_WINDOW, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::See)]);
    assert!(!s.accept(Kind::Hear), "the first word is gone");
    assert!(s.accept(Kind::See));
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 200.0, |_| {});
    assert!(s.drain_events().is_empty(), "once a visit");
}

#[test]
fn a_word_that_went_by_waits_as_a_mark() {
    let mut w = World::new();
    let mut s = Senses { can_hear: true, can_see: true, ..Default::default() };
    assert!(s.waiting(Kind::Hear), "hearing waits from the start");
    assert!(!s.waiting(Kind::See), "sight waits only once it has asked");
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 21.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)]);
    assert!(s.waiting(Kind::Hear), "and still while the word itself is up");
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_GAP, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::See)]);
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), OFFER_WINDOW + 1.0, |_| {});
    assert!(s.waiting(Kind::Hear) && s.waiting(Kind::See));
    assert!(s.accept_waiting(Kind::Hear));
    assert_eq!(s.hear, Leave::Opening);
    s.opened(Kind::Hear);
    assert!(!s.waiting(Kind::Hear), "not while it hears");
    s.close(Kind::Hear);
    assert!(s.waiting(Kind::Hear), "stopped, it can be let hear again");
    assert!(s.accept_waiting(Kind::Hear));
    s.refused(Kind::Hear, true);
    assert!(!s.waiting(Kind::Hear) && !s.accept_waiting(Kind::Hear), "a no is kept");
    let s = Senses { can_hear: false, ..Default::default() };
    assert!(!s.waiting(Kind::Hear), "where it cannot hear, nothing waits");
}

#[test]
fn given_hearing_early_it_asks_for_words_early() {
    let mut w = World::new();
    let mut s = Senses { can_hear: true, can_words: true, ..Default::default() };
    assert!(s.accept_waiting(Kind::Hear));
    s.opened(Kind::Hear);
    run(&mut s, &mut w, &look(Stage::Matter, 30.0), 21.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Words)]);
}

#[test]
fn it_asks_to_feel_first_and_earlier() {
    let mut w = World::new();
    let mut s = Senses { can_hear: true, can_feel: true, ..Default::default() };
    run(&mut s, &mut w, &look(Stage::Matter, 30.0), 40.0, |_| {});
    assert!(s.drain_events().is_empty(), "not before regard");
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), 1.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Feel)]);
    assert_eq!(Kind::Feel.word(), "hold the room.");
    assert!(s.accept(Kind::Feel));
    s.refused(Kind::Feel, true);
    assert_eq!(s.drain_events(), vec![Event::Refused(Kind::Feel)]);
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), OFFER_GAP, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)], "then, later, to hear");
    assert_eq!(Senses::load(&s.save()).feel, Leave::Refused);
    assert_eq!(Senses::load("hear=no;see=").feel, Leave::Closed, "an older save still loads");
}

#[test]
fn it_asks_for_words_only_where_they_must_leave_and_only_once_it_hears() {
    let mut w = World::new();
    let mut s = Senses { can_hear: true, can_words: true, ..Default::default() };
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), 25.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Hear)]);
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), 100.0, |_| {});
    assert!(s.drain_events().is_empty(), "not before it hears");
    s.hear = Leave::Open;
    run(&mut s, &mut w, &look(Stage::Echo, 30.0), 1.0, |_| {});
    assert_eq!(s.drain_events(), vec![Event::Offer(Kind::Words)]);
    assert_eq!(Kind::Words.word(), "say something.");
    assert!(s.accept(Kind::Words));
    s.refused(Kind::Words, true);
    assert_eq!(Senses::load(&s.save()).words, Leave::Refused);
    assert_eq!(Senses::load(&s.save()).hear, Leave::Closed);
    // where words never leave, it does not ask
    let mut s = Senses { can_hear: true, hear: Leave::Open, ..Default::default() };
    s.offered_hear = true;
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 200.0, |_| {});
    assert!(s.drain_events().is_empty());
}

#[test]
fn where_it_cannot_hear_it_never_asks() {
    let mut w = World::new();
    let mut s = Senses::default();
    run(&mut s, &mut w, &look(Stage::Initiative, 30.0), 200.0, |_| {});
    assert!(s.drain_events().is_empty());
}

#[test]
fn a_refusal_is_kept_and_never_asked_again() {
    let mut s = Senses { can_hear: true, ..Default::default() };
    s.hear = Leave::Offered(3.0);
    assert!(s.accept(Kind::Hear));
    s.refused(Kind::Hear, true);
    assert_eq!(s.drain_events(), vec![Event::Refused(Kind::Hear)]);
    let mut next = Senses::load(&s.save());
    assert_eq!(next.hear, Leave::Refused);
    assert_eq!(next.see, Leave::Closed);
    next.can_hear = true;
    let mut w = World::new();
    run(&mut next, &mut w, &look(Stage::Echo, 30.0), 200.0, |_| {});
    assert!(next.drain_events().is_empty());
    // A missing microphone is not a no.
    let mut s = Senses { hear: Leave::Opening, ..Default::default() };
    s.refused(Kind::Hear, false);
    assert_eq!((s.hear, s.save().as_str()), (Leave::Closed, "hear=;see=;feel=;words="));
}

#[test]
fn a_voice_makes_it_tremble_high_or_low() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(880.0, 0.1), RATE)));
    assert!(found.contains(&Discovery::Voice));
    assert!(w.ripples.len() > 5 && w.ripples.iter().all(|r| r.dir.y > 0.3), "high on it");
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(80.0, 0.1), RATE)));
    assert!(w.ripples.iter().all(|r| r.dir.y < -0.3), "low on it");
}

#[test]
fn a_held_note_comes_back_as_a_hum_and_it_does_not_hear_itself() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let l = look(Stage::Echo, 0.0);
    run(&mut s, &mut w, &l, 1.2, |s| s.heard(hear(&tone(440.0, 0.1), RATE)));
    assert_eq!(s.hum(), None, "not while you are still singing");
    let found = run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
    assert!(found.contains(&Discovery::Sing));
    let (f, _) = s.hum().expect("humming");
    assert!((f - 220.0).abs() < 5.0, "the same note, lower: {f}");
    // its own hum, picked up by the microphone, is not a new note
    run(&mut s, &mut w, &l, HUM_SECS, |s| s.heard(hear(&tone(220.0, 0.1), RATE)));
    let found = run(&mut s, &mut w, &l, 0.6, |s| s.heard(Heard::default()));
    assert!(!found.contains(&Discovery::Sing) && s.hum().is_none());
    // a note too short is not held
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    run(&mut s, &mut w, &l, 0.4, |s| s.heard(hear(&tone(300.0, 0.1), RATE)));
    run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
    assert_eq!(s.hum(), None);
}

#[test]
fn a_whisper_stills_it_and_a_breath_takes_a_drop() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    w.agitation = 1.0;
    let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&hiss(0.03), RATE)));
    assert!(found.contains(&Discovery::Whisper));
    assert!(w.agitation < 0.1 && w.gaze > 0.5, "still, and leaning in");
    assert!(w.drops.is_empty());

    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 0.6, |s| s.heard(hear(&hiss(0.4), RATE)));
    assert!(found.contains(&Discovery::Blow));
    assert_eq!(w.drops.len(), 1);
    assert!(w.drops[0].pos.z < -0.5, "off the far side, away from you");
}

#[test]
fn a_whisper_at_arms_length_is_heard_and_a_steady_room_is_not() {
    // As quiet as a phone's raw microphone hears a whisper, in breaths with gaps between.
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let (soft, l) = (hear(&hiss(0.006), RATE), look(Stage::Echo, 0.0));
    let mut t = 0;
    let found = run(&mut s, &mut w, &l, 3.0, |s| {
        t += 1;
        s.heard(if t % 50 < 40 { soft } else { Heard::default() })
    });
    assert!(found.contains(&Discovery::Whisper));

    // The same hiss, never stopping, is a fan: after a while it is only the room.
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    run(&mut s, &mut w, &l, 40.0, |s| s.heard(soft));
    let found = run(&mut s, &mut w, &l, 2.0, |s| s.heard(soft));
    assert!(!found.contains(&Discovery::Whisper));
    // and a voice over it still is someone
    let found = run(&mut s, &mut w, &l, 1.0, |s| s.heard(hear(&tone(220.0, 0.02), RATE)));
    assert!(found.contains(&Discovery::Voice));
}

/// Air across a microphone: loud, and nearly all low rumble.
fn wind(amp: f32) -> Vec<f32> {
    let k = 1.0 - (-std::f32::consts::TAU * 80.0 / RATE).exp();
    let mut y = 0.0;
    hiss(1.0).into_iter().map(|x| {
        y += (x - y) * k;
        y * amp * 3.0
    }).collect()
}

#[test]
fn wind_is_a_breath_not_a_voice() {
    let h = hear(&wind(1.0), RATE);
    assert!(h.low > BLOW_LOW && h.level > BLOW, "{h:?}");
    assert!(hear(&tone(220.0, 0.1), RATE).clarity > BLOW_CLARITY);
    // A low voice, with its overtones, is still a voice, however low.
    let voice: Vec<f32> = (0..2048)
        .map(|i| (1..8).map(|k| (i as f32 / RATE * 110.0 * k as f32 * std::f32::consts::TAU).sin() * 0.08 / k as f32).sum())
        .collect();
    let v = hear(&voice, RATE);
    assert!(v.pitch.is_some() && !(v.low > BLOW_LOW && v.clarity < BLOW_CLARITY), "{v:?}");

    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let l = look(Stage::Echo, 0.0);
    let found = run(&mut s, &mut w, &l, 1.0, |s| s.heard(hear(&wind(1.0), RATE)));
    assert!(found.contains(&Discovery::Blow), "{found:?}");
    assert!(!found.iter().any(|d| matches!(d, Discovery::Voice | Discovery::Shout | Discovery::Startle)), "{found:?}");
    assert!(w.body.pos.z < -0.08, "pushed away from you: {:?}", w.body.pos);
    // even a soft one is felt at once, as ripples on the near side
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    run(&mut s, &mut w, &l, 0.2, |s| s.heard(hear(&wind(0.25), RATE)));
    assert!(!w.ripples.is_empty());
}

#[test]
fn its_own_sounds_are_not_yours() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let l = look(Stage::Echo, 0.0);
    s.deafen(1.5);
    let found = run(&mut s, &mut w, &l, 1.4, |s| s.heard(hear(&tone(659.0, 0.1), RATE)));
    run(&mut s, &mut w, &l, 0.5, |s| s.heard(Heard::default()));
    assert!(found.is_empty() && s.hum().is_none());
}

#[test]
fn shouted_at_it_is_upset() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&tone(300.0, 0.4), RATE)));
    assert!(found.contains(&Discovery::Shout));
    assert!(w.agitation > 0.5, "{}", w.agitation);
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.5, |s| s.heard(hear(&tone(300.0, 0.1), RATE)));
    assert!(w.agitation < 0.1, "a voice is not a shout");
}

#[test]
fn closed_it_hears_nothing() {
    let (mut w, mut s) = (World::new(), open(Kind::Hear));
    assert!(s.close(Kind::Hear));
    assert_eq!(s.drain_events(), vec![Event::Closed(Kind::Hear)]);
    let found = run(&mut s, &mut w, &look(Stage::Echo, 0.0), 1.0, |s| s.heard(hear(&tone(440.0, 0.2), RATE)));
    assert!(found.is_empty() && w.ripples.is_empty());
    assert!(!s.close(Kind::Hear));
}

/// A tiny frame: grey, with a bright square at (x, y).
fn frame(light: u8, square: Option<(usize, usize)>) -> Vec<u8> {
    let (w, h) = (32, 24);
    let mut px = vec![light; w * h * 4];
    if let Some((sx, sy)) = square {
        for y in sy..(sy + 5).min(h) {
            for x in sx..(sx + 5).min(w) {
                let i = (y * w + x) * 4;
                px[i..i + 3].copy_from_slice(&[250, 250, 250]);
            }
        }
    }
    px
}

#[test]
fn it_sees_light_and_where_things_moved_as_you_see_them() {
    let mut r = Retina::default();
    let first = r.look(&frame(100, None), 32, 24);
    assert!((first.light - 100.0 / 255.0).abs() < 0.02);
    assert_eq!(first.at, None, "nothing to compare with yet");
    // something appears on the camera's right, near the top: the player's left, up
    let s = r.look(&frame(100, Some((26, 1))), 32, 24);
    let (x, y) = s.at.expect("it moved");
    assert!(x < -0.5 && y > 0.5, "{x} {y}");
    assert!(s.motion > 0.0);
    let s = r.look(&frame(100, Some((26, 1))), 32, 24);
    assert_eq!((s.motion, s.at), (0.0, None), "nothing moved");
}

#[test]
fn it_turns_toward_you_and_flinches_from_fast_movement() {
    let (mut w, mut s) = (World::new(), open(Kind::See));
    let l = look(Stage::Initiative, 5.0);
    let mut r = Retina::default();
    let mut k = 0;
    let found = run(&mut s, &mut w, &l, 2.0, |s| {
        k += 1;
        // a small movement, to the player's right
        s.seen(r.look(&frame(120, Some((2 + k % 2, 10))), 32, 24));
    });
    assert!(found.contains(&Discovery::Seen));
    assert!(w.gaze_dir.x > 0.2 && w.gaze_dir.z > 0.5, "toward you, on your right: {:?}", w.gaze_dir);
    assert!(!found.contains(&Discovery::Startle));
    let found = run(&mut s, &mut w, &l, 0.2, |s| s.seen(Seen { light: 0.5, motion: 0.4, at: Some((0.0, 0.0)) }));
    assert!(found.contains(&Discovery::Startle));
    assert!(w.body.vel.z < 0.0, "away from you");
}

#[test]
fn covered_the_room_goes_dark_and_comes_back() {
    let (mut w, mut s) = (World::new(), open(Kind::See));
    let l = look(Stage::Initiative, 5.0);
    let found = run(&mut s, &mut w, &l, 0.5, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
    assert!(found.is_empty(), "a camera still waking up is not a covered one");
    run(&mut s, &mut w, &l, 1.5, |s| s.seen(Seen { light: 0.5, motion: 0.0, at: None }));
    let found = run(&mut s, &mut w, &l, 3.0, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
    assert!(found.contains(&Discovery::Blind));
    assert!(w.night > 0.5, "{}", w.night);
    run(&mut s, &mut w, &l, 8.0, |s| s.seen(Seen { light: 0.5, motion: 0.0, at: None }));
    assert!(w.night < 0.1, "{}", w.night);
    // and closed, it lets the room go
    run(&mut s, &mut w, &l, 3.0, |s| s.seen(Seen { light: 0.01, motion: 0.0, at: None }));
    s.close(Kind::See);
    run(&mut s, &mut w, &l, 8.0, |_| {});
    assert!(w.night < 0.1);
}
