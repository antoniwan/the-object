//! Gestures driven through `Game`, checked by what they do and what they discover.

use super::*;

fn game() -> Game {
    let mut g = Game::new();
    g.w = 800.0;
    g.h = 600.0;
    g
}

#[test]
fn tap_on_center_pokes() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    g.pointer_up(1);
    assert!(g.progress.has(Discovery::Poke));
    assert_eq!(g.world.ripples.len(), 1);
}

#[test]
fn a_tap_the_finger_slid_on_a_little_still_pokes() {
    let mut g = game();
    // a finger lands and slides a few pixels before it lifts: past the slop, far short of a pinch
    tug(&mut g, (400.0, 300.0), (0.6, 0.8), 6.0, 2, 1.0 / 60.0, 0.0);
    assert!(g.progress.has(Discovery::Poke));
    assert!(g.sounds.iter().any(|s| matches!(s, Sound::Poke { .. })));
    assert!(!g.progress.has(Discovery::Pinch) && g.world.forms.bloom.near.is_none());
    // the same slide held a while is not a tap
    let mut g = game();
    tug(&mut g, (400.0, 300.0), (0.6, 0.8), 6.0, 2, 1.0 / 60.0, 0.5);
    assert!(!g.progress.has(Discovery::Poke));
}

#[test]
fn drag_from_center_far_tears_and_keeps_holding_the_drop() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for i in 1..=60 {
        g.pointer_move(1, 400.0 + i as f32 * 8.0, 300.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Stretch));
    assert!(g.progress.has(Discovery::Tear));
    assert!(matches!(g.gesture, Gesture::Dragging { .. }));
    assert!(g.world.drops[0].target.is_some());
}

#[test]
fn a_fast_swipe_through_the_body_slices_it_into_two_cells() {
    let mut g = game();
    g.pointer_down(1, 400.0, 20.0);
    for i in 1..=6 {
        g.pointer_move(1, 400.0, 20.0 + i as f32 * 95.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(1);
    assert!(g.progress.has(Discovery::Divide));
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.division.phase, meiosis::Phase::Two);
    // the cut was vertical on screen, so the cells sit side by side
    assert!(g.world.division.axis.dot(g.cam.basis().right).abs() > 0.9);
    assert!(g.cam.dist < Camera::new().dist);
    // tapping a cell pokes it, and its twin feels it
    g.pointer_down(2, 577.0, 300.0);
    g.pointer_up(2);
    assert!(g.progress.has(Discovery::Twin));
}

fn swipe_down(g: &mut Game, id: i32, x: f32) {
    g.pointer_down(id, x, 20.0);
    for i in 1..=6 {
        g.pointer_move(id, x, 20.0 + i as f32 * 95.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(id);
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
}

#[test]
fn cut_again_and_again_it_multiplies_one_cell_dies_and_squeezed_it_turns_to_stone_until_held() {
    let mut g = game();
    swipe_down(&mut g, 1, 400.0);
    assert_eq!(g.world.division.phase, meiosis::Phase::Two);
    swipe_down(&mut g, 2, 577.0);
    swipe_down(&mut g, 3, 400.0);
    assert!(g.progress.has(Discovery::Cleave));
    let m = g.world.division.morula.as_ref().unwrap();
    assert_eq!(m.cells.len(), 8);
    assert!(g.progress.has(Discovery::Eye));

    for i in 0..3 {
        g.pointer_down(10 + i, 400.0, 300.0);
        g.pointer_up(10 + i);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Shed));
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.division.morula.as_ref().unwrap().cells.len(), 7);

    for _ in 0..120 {
        g.wheel(-200.0);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Stone));
    for _ in 0..(60 * 7) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Slab));
    assert!(g.cam.dist > 8.0);
    assert!(g.pick(400.0, 300.0).is_some(), "the slab can be touched");

    g.pointer_down(20, 400.0, 300.0);
    for _ in 0..(60 * 3) {
        g.update(1.0 / 60.0);
    }
    g.pointer_up(20);
    for _ in 0..(60 * 4) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Thaw));
    assert!(g.world.division.whole());
    assert!(g.world.division.morula.is_none());
}

#[test]
fn thawed_back_to_one_body_the_camera_comes_home_without_a_jump() {
    let mut g = game();
    swipe_down(&mut g, 1, 400.0);
    swipe_down(&mut g, 2, 577.0);
    for _ in 0..120 {
        g.wheel(-200.0);
        g.update(1.0 / 60.0);
    }
    wait(&mut g, 7.0);
    assert!(g.progress.has(Discovery::Slab));
    g.pointer_down(20, 400.0, 300.0);
    wait(&mut g, 3.0);
    g.pointer_up(20);
    let mut last = g.cam.dist;
    let mut worst = 0.0f32;
    for _ in 0..(60 * 5) {
        g.update(1.0 / 60.0);
        worst = worst.max((g.cam.dist - last).abs());
        last = g.cam.dist;
    }
    assert!(g.world.division.morula.is_none());
    assert!(worst < 0.2, "camera moved {worst} in one frame");
}

#[test]
fn squeezed_into_stone_it_flattens_and_rounds_out_smoothly() {
    let mut g = game();
    swipe_down(&mut g, 1, 400.0);
    swipe_down(&mut g, 2, 577.0);
    let mut last = g.world.body.squash;
    let mut worst = 0.0f32;
    for _ in 0..240 {
        g.wheel(-200.0);
        g.update(1.0 / 60.0);
        worst = worst.max((g.world.body.squash - last).abs());
        last = g.world.body.squash;
    }
    assert!(g.progress.has(Discovery::Stone));
    assert!(worst < 0.08, "squash moved {worst} in one frame");
}

#[test]
fn a_slow_drag_across_the_body_only_orbits() {
    let mut g = game();
    g.pointer_down(1, 400.0, 20.0);
    for i in 1..=60 {
        g.pointer_move(1, 400.0, 20.0 + i as f32 * 9.5, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(1);
    assert!(g.world.division.whole());
}

#[test]
fn far_enough_along_it_takes_a_turn_when_you_go_still() {
    let mut g = game();
    for d in crate::progress::ALL.iter().take(contact::TURN_AT) {
        g.progress.discover(*d);
    }
    for _ in 0..4 {
        g.pointer_down(1, 400.0, 300.0);
        g.pointer_up(1);
        g.update(1.0 / 60.0);
    }
    for _ in 0..(60 * 8) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Initiate));
    assert!(g.world.regard > 0.0, "it has started to regard the pointer");
}

#[test]
fn early_on_it_is_only_matter() {
    let mut g = game();
    for _ in 0..(60 * 20) {
        g.update(1.0 / 60.0);
    }
    assert!(!g.progress.has(Discovery::Initiate));
    assert_eq!(g.world.regard, 0.0);
}

#[test]
fn a_pinch_that_starts_on_empty_space_is_only_a_squeeze() {
    let mut g = game();
    g.pointer_down(1, 40.0, 300.0);
    g.pointer_down(2, 760.0, 300.0);
    g.pointer_move(2, 700.0, 300.0, 1.0 / 60.0);
    g.update(1.0 / 60.0);
    g.pointer_up(2);
    g.pointer_up(1);
    assert!(!g.progress.has(Discovery::Call), "the first finger was not a tap");
    assert_eq!(g.world.gaze, 0.0);
    assert!(g.world.forms.bloom.buds.is_empty(), "nor a pinch");
}

#[test]
fn a_second_finger_on_a_fresh_stretch_does_not_pinch() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    g.pointer_move(1, 440.0, 300.0, 1.0 / 60.0);
    g.update(1.0 / 60.0);
    g.pointer_down(2, 600.0, 300.0);
    assert!(g.world.handles.iter().all(|h| h.target.is_none()), "the tendril was let go");
    g.pointer_up(2);
    g.pointer_up(1);
    for _ in 0..30 {
        g.update(1.0 / 60.0);
    }
    assert!(!g.progress.has(Discovery::Pinch));
}

/// Two fingers down at `a` and `b`, moved to `a2` and `b2` over six frames, held `hold` seconds, lifted.
fn two_fingers(g: &mut Game, a: (f32, f32), b: (f32, f32), a2: (f32, f32), b2: (f32, f32), hold: f32) {
    g.pointer_down(1, a.0, a.1);
    g.pointer_down(2, b.0, b.1);
    for k in 1..=6 {
        let f = k as f32 / 6.0;
        g.pointer_move(1, a.0 + (a2.0 - a.0) * f, a.1 + (a2.1 - a.1) * f, 1.0 / 60.0);
        g.pointer_move(2, b.0 + (b2.0 - b.0) * f, b.1 + (b2.1 - b.1) * f, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    wait(g, hold);
    g.pointer_up(1);
    g.pointer_up(2);
    wait(g, 0.1);
}

#[test]
fn two_fingers_pinched_together_on_it_and_let_go_leave_a_bud_between_them() {
    let mut g = game();
    two_fingers(&mut g, (420.0, 260.0), (480.0, 260.0), (440.0, 260.0), (460.0, 260.0), 0.0);
    assert!(g.progress.has(Discovery::Pinch));
    assert!(!g.progress.has(Discovery::Squeeze), "a pinch of its skin does not flatten all of it");
    assert!(g.world.body.squash > 0.85, "{}", g.world.body.squash);
    let buds = &g.world.forms.bloom.buds;
    assert_eq!(buds.len(), 1);
    let b = g.cam.basis();
    assert!(
        buds[0].dir.dot(b.right) > 0.2 && buds[0].dir.dot(b.up) > 0.1,
        "it is up and to the right, where the fingers were"
    );
    assert!(!g.progress.has(Discovery::Poke) && !g.progress.has(Discovery::Stretch));

    // held a while, it is only a squeeze
    two_fingers(&mut g, (330.0, 340.0), (390.0, 340.0), (350.0, 340.0), (370.0, 340.0), 1.0);
    // spread apart, it is only a squeeze
    two_fingers(&mut g, (330.0, 340.0), (390.0, 340.0), (300.0, 340.0), (420.0, 340.0), 0.0);
    // barely closed, it is only a squeeze
    two_fingers(&mut g, (330.0, 340.0), (390.0, 340.0), (335.0, 340.0), (385.0, 340.0), 0.0);
    // one finger off it, it is only a squeeze
    two_fingers(&mut g, (330.0, 340.0), (700.0, 340.0), (450.0, 340.0), (580.0, 340.0), 0.0);
    assert_eq!(g.world.forms.bloom.buds.len(), 1);
}

#[test]
fn a_third_finger_lifting_does_not_end_the_squeeze() {
    let mut g = game();
    g.pointer_down(1, 300.0, 300.0);
    g.pointer_down(2, 500.0, 300.0);
    g.pointer_down(3, 400.0, 500.0);
    g.pointer_up(3);
    g.pointer_move(2, 700.0, 300.0, 1.0 / 60.0);
    assert!(g.world.body.squeeze > 0.3);
}

#[test]
fn two_fingers_pinched_on_a_trackpad_squeeze_it_as_far_as_on_a_screen() {
    let mut g = game();
    // told as a wheel with ctrl held, a little each frame, the way browsers tell it
    for _ in 0..20 {
        g.pad.wheel(4.0);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Squeeze));
    assert!(g.world.body.squeeze < -0.6, "{}", g.world.body.squeeze);
    // once the fingers stop it lets go
    wait(&mut g, 1.5);
    assert!(g.world.body.squeeze.abs() < 0.1);
}

#[test]
fn a_trackpad_pinch_held_still_in_safari_presses_two_cells_back_into_one() {
    let mut g = game();
    swipe_down(&mut g, 1, 400.0);
    assert_eq!(g.world.division.phase, meiosis::Phase::Two);
    g.pad.gesture(Some(0.5));
    wait(&mut g, 1.5);
    g.pad.gesture(None);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Rejoin));
}

#[test]
fn pulled_the_other_way_it_stands_taller_and_settles() {
    let mut g = game();
    for _ in 0..20 {
        g.wheel(-60.0);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Squeeze));
    assert!(g.world.body.squash > 1.15, "it stands taller: {}", g.world.body.squash);
    wait(&mut g, 3.0);
    assert!((g.world.body.squash - 1.0).abs() < 0.03, "and settles");
    // fingers spread apart on the screen pull it the same way (a moment after they lift, it is still tall)
    two_fingers(&mut g, (200.0, 300.0), (600.0, 300.0), (100.0, 300.0), (700.0, 300.0), 0.3);
    assert!(g.world.body.squash > 1.05, "{}", g.world.body.squash);
}

#[test]
fn holding_still_warms_instead_of_poking() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }
    assert!(g.world.warming);
    assert!(g.world.body.color != crate::sim::PEARL);
    g.pointer_up(1);
    assert!(!g.world.warming);
    assert!(g.world.ripples.is_empty());
    assert!(g.progress.has(Discovery::Warm));
}

#[test]
fn the_page_can_tell_a_finger_is_resting_so_the_canvas_keeps_its_size_until_it_lifts() {
    let mut g = game();
    assert!(!g.held());
    g.pointer_down(1, 400.0, 300.0);
    wait(&mut g, 0.5);
    assert!(g.held(), "a still finger is still a hand on it");
    g.pointer_up(1);
    assert!(!g.held());
}

#[test]
fn left_alone_it_watches() {
    let mut g = game();
    for _ in 0..(60 * 25) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Patience));
}

#[test]
fn poking_fast_provokes_it() {
    let mut g = game();
    for _ in 0..7 {
        g.pointer_down(1, 400.0, 300.0);
        g.pointer_up(1);
        g.update(0.1);
    }
    assert!(g.progress.has(Discovery::Provoke));
    assert!(g.world.agitation > 0.5);
}

#[test]
fn tearing_many_pieces_opens_it_and_they_all_come_home() {
    let mut g = game();
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let (dx, dy) = (a.cos(), a.sin());
        g.pointer_down(1, 400.0 + dx * 60.0, 300.0 + dy * 60.0);
        for k in 1..=60 {
            let s = 60.0 + k as f32 * 8.0;
            g.pointer_move(1, 400.0 + dx * s, 300.0 + dy * s, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        for _ in 0..20 {
            g.update(1.0 / 60.0);
        }
    }
    assert!(g.progress.has(Discovery::Open), "{} drops", g.world.drops.len());
    for _ in 0..(60 * 120) {
        g.update(1.0 / 60.0);
    }
    assert!(
        g.world.drops.is_empty(),
        "{:?} drops left",
        g.world.drops.iter().map(|d| (d.pos.len(), d.vel.len(), d.target.is_some())).collect::<Vec<_>>()
    );
    assert!(g.progress.has(Discovery::Close));
}

#[test]
fn tap_on_empty_space_makes_it_lean_toward_the_tap() {
    let mut g = game();
    g.pointer_down(1, 40.0, 300.0);
    g.pointer_up(1);
    assert!(g.progress.has(Discovery::Call));
    assert!(g.world.gaze > 0.5 && g.world.gaze_dir.x < -0.5);
}

#[test]
fn a_hard_downward_throw_slams_it_into_the_floor_and_pulling_up_brings_it_back() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for i in 1..=4 {
        g.pointer_move(1, 400.0, 300.0 + i as f32 * 40.0, 1.0 / 120.0);
        g.update(1.0 / 120.0);
    }
    g.pointer_up(1);
    assert!(g.progress.has(Discovery::Throw));
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Slam));
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
    assert!(g.world.body.pos.y < fusion::FLOOR_Y);

    // tap the liquid floor, then pull up from it
    g.pointer_down(2, 400.0, 560.0);
    g.pointer_up(2);
    assert!(g.progress.has(Discovery::Lake));
    g.pointer_down(3, 400.0, 560.0);
    g.pointer_move(3, 400.0, 400.0, 1.0 / 60.0);
    g.pointer_up(3);
    assert!(g.progress.has(Discovery::Rise));
    for _ in 0..360 {
        g.update(1.0 / 60.0);
    }
    assert!(g.world.fusion.whole());
    assert!(g.world.body.pos.len() < 0.2, "back home in the middle");
}

fn slammed() -> Game {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for i in 1..=4 {
        g.pointer_move(1, 400.0, 300.0 + i as f32 * 40.0, 1.0 / 120.0);
        g.update(1.0 / 120.0);
    }
    g.pointer_up(1);
    for _ in 0..360 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
    g
}

#[test]
fn held_still_the_lake_freezes_and_breaks_under_taps_and_pushed_down_it_comes_up_ink() {
    let mut g = slammed();
    g.pointer_down(2, 400.0, 560.0);
    for _ in 0..70 {
        g.update(1.0 / 60.0);
    }
    g.pointer_up(2);
    assert!(g.progress.has(Discovery::Freeze));
    assert!(g.world.fusion.mere.frozen());
    // it can't be pulled up through the ice
    g.pointer_down(3, 400.0, 560.0);
    g.pointer_move(3, 400.0, 400.0, 1.0 / 60.0);
    g.pointer_up(3);
    assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
    for i in 0..mere::CRACKS_TO_BREAK {
        g.pointer_down(10 + i as i32, 380.0 + i as f32 * 10.0, 560.0);
        g.pointer_up(10 + i as i32);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Shatter));
    for _ in 0..90 {
        g.update(1.0 / 60.0);
    }
    assert!(!g.world.fusion.mere.frozen());

    // push down: it goes deep, and comes back out ink
    g.pointer_down(4, 400.0, 500.0);
    g.pointer_move(4, 400.0, 590.0, 1.0 / 60.0);
    g.pointer_up(4);
    assert!(g.progress.has(Discovery::Deep));
    for _ in 0..180 {
        g.update(1.0 / 60.0);
    }
    assert!(g.world.body.pos.y < fusion::FLOOR_Y - 2.0, "it went further down");
    g.pointer_down(5, 400.0, 560.0);
    g.pointer_move(5, 400.0, 400.0, 1.0 / 60.0);
    g.pointer_up(5);
    assert!(g.progress.has(Discovery::Ink));
    for _ in 0..420 {
        g.update(1.0 / 60.0);
    }
    assert!(g.world.fusion.whole());
    assert!(g.world.body.pos.len() < 0.2);
    assert!(g.world.fusion.mere.coat_mix().1 > 0.9);
}

#[test]
fn a_finger_slid_along_the_lake_and_back_does_not_freeze_it() {
    let mut g = slammed();
    g.pointer_down(2, 400.0, 560.0);
    // well off along the floor and back, never holding still
    for i in 0..40 {
        let x = 400.0 + 120.0 * ((i as f32 + 1.0) / 40.0 * std::f32::consts::PI).sin();
        g.pointer_move(2, x, 560.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    wait(&mut g, 0.34);
    assert!(!g.progress.has(Discovery::Freeze), "froze though the finger never held still");
}

#[test]
fn left_alone_the_lake_becomes_a_mirror_and_it_comes_up_quicksilver() {
    let mut g = slammed();
    for _ in 0..(60 * 10) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Mirror));
    g.pointer_down(2, 400.0, 560.0);
    g.pointer_move(2, 400.0, 400.0, 1.0 / 60.0);
    g.pointer_up(2);
    assert!(g.progress.has(Discovery::Quicksilver));
}

#[test]
fn three_taps_in_one_place_on_the_lake_throw_up_a_fountain() {
    let mut g = slammed();
    for i in 0..3 {
        g.pointer_down(2 + i, 400.0, 560.0);
        g.pointer_up(2 + i);
        for _ in 0..10 {
            g.update(1.0 / 60.0);
        }
    }
    assert!(g.progress.has(Discovery::Fountain));
    assert!(g.world.fusion.mere.fountain.is_some());
}

#[test]
fn poked_to_death_it_bursts_into_a_landscape_and_holding_still_gathers_it() {
    let mut g = game();
    for i in 0..12 {
        g.pointer_down(i, 400.0, 300.0);
        g.pointer_up(i);
        for _ in 0..12 {
            g.update(1.0 / 60.0);
        }
    }
    assert!(g.progress.has(Discovery::Burst), "wounds {}", g.world.burst.wounds);
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.burst.phase, burst::Phase::Landscape);
    assert!(g.cam.dist > 15.0);
    assert!(g.pick(400.0, 300.0).is_none(), "the body is gone");
    // tap the sky (nothing there), hold still on the ground, and the pieces come home
    g.pointer_down(20, 400.0, 590.0);
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }
    g.pointer_up(20);
    assert!(g.progress.has(Discovery::Gather));
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert!(g.world.burst.whole());
}

fn wait(g: &mut Game, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        g.update(1.0 / 60.0);
    }
}

#[test]
fn five_pinches_make_a_flower_and_a_tap_folds_it() {
    let mut g = game();
    for i in 0..5 {
        let a = i as f32 * std::f32::consts::TAU / 5.0;
        let (dx, dy) = (a.cos(), a.sin());
        let (x0, y0) = (400.0 + dx * 50.0, 300.0 + dy * 50.0);
        g.pointer_down(1, x0, y0);
        for k in 1..=3 {
            g.pointer_move(1, x0 + dx * 14.0 * k as f32, y0 + dy * 14.0 * k as f32, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        assert!(g.world.drops.is_empty() && !g.progress.has(Discovery::Throw));
        wait(&mut g, 0.6);
    }
    assert!(g.progress.has(Discovery::Pinch));
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Bloom), "{} buds", g.world.forms.bloom.buds.len());
    assert!(g.cam.pitch > 0.5, "it looks down into the flower");
    g.pointer_down(2, 400.0, 300.0);
    g.pointer_up(2);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Fold));
    assert!(g.world.ordinary());
}

/// A tug outward from a point on it: `steps` moves of `px` canvas pixels, `dt` apart, then held for `hold` seconds.
fn tug(g: &mut Game, (x0, y0): (f32, f32), (dx, dy): (f32, f32), px: f32, steps: usize, dt: f32, hold: f32) {
    g.pointer_down(1, x0, y0);
    for k in 1..=steps {
        g.pointer_move(1, x0 + dx * px * k as f32, y0 + dy * px * k as f32, dt);
        g.update(dt);
    }
    wait(g, hold);
    g.pointer_up(1);
}

#[test]
fn a_tug_let_go_a_little_too_late_nearly_leaves_a_bud() {
    let mut g = game();
    tug(&mut g, (450.0, 300.0), (1.0, 0.0), 14.0, 3, 1.0 / 60.0, 0.5);
    assert!(!g.progress.has(Discovery::Pinch), "too late to be a pinch");
    assert!(g.world.forms.bloom.buds.is_empty());
    assert!(g.world.forms.bloom.near.is_some(), "but a bud nearly rose");
    assert!(g.sounds.contains(&Sound::Almost));
    wait(&mut g, 0.3);
    assert!(g.world.forms.bloom.near_size() > 0.2);
    wait(&mut g, 1.5);
    assert!(g.world.forms.bloom.near.is_none(), "and sank back");
    // held much longer, it was only a stretch
    g.sounds.clear();
    tug(&mut g, (350.0, 300.0), (-1.0, 0.0), 14.0, 3, 1.0 / 60.0, 1.5);
    assert!(g.world.forms.bloom.near.is_none() && !g.sounds.contains(&Sound::Almost));
}

#[test]
fn a_tug_flicked_off_fast_is_still_a_pinch_not_a_throw() {
    let mut g = game();
    // a finger leaves a short tug fast enough to be a flick
    tug(&mut g, (450.0, 300.0), (1.0, 0.0), 24.0, 3, 1.0 / 120.0, 0.0);
    wait(&mut g, 0.1);
    assert!(g.progress.has(Discovery::Pinch));
    assert!(!g.progress.has(Discovery::Throw));
    assert_eq!(g.world.forms.bloom.buds.len(), 1);
    // pulled further, the same flick throws it
    tug(&mut g, (400.0, 300.0), (0.0, 1.0), 40.0, 4, 1.0 / 120.0, 0.0);
    assert!(g.progress.has(Discovery::Throw));
}

/// `tug`, but the browser takes the touch away instead of the finger lifting.
fn tug_cancelled(g: &mut Game, (x0, y0): (f32, f32), (dx, dy): (f32, f32), px: f32, steps: usize, dt: f32) {
    g.pointer_down(1, x0, y0);
    for k in 1..=steps {
        g.pointer_move(1, x0 + dx * px * k as f32, y0 + dy * px * k as f32, dt);
        g.update(dt);
    }
    g.drop_hand(1);
}

#[test]
fn a_cancelled_touch_finishes_nothing() {
    // pressed and taken away at once: not a poke
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    wait(&mut g, 0.2);
    g.drop_hand(1);
    wait(&mut g, 0.6);
    assert!(!g.progress.has(Discovery::Poke) && g.world.ripples.is_empty());
    assert!(g.pointers.is_empty() && matches!(g.gesture, Gesture::Idle));
    // a short tug taken away: not a pinch
    tug_cancelled(&mut g, (450.0, 300.0), (1.0, 0.0), 24.0, 3, 1.0 / 120.0);
    wait(&mut g, 0.1);
    assert!(!g.progress.has(Discovery::Pinch) && g.world.forms.bloom.buds.is_empty());
    // a long pull flicked as the touch is taken away: not a throw
    tug_cancelled(&mut g, (400.0, 300.0), (0.0, 1.0), 40.0, 4, 1.0 / 120.0);
    wait(&mut g, 0.1);
    assert!(!g.progress.has(Discovery::Throw));
    assert!(g.world.handles.iter().all(|h| h.target.is_none()), "the tendril was let go");
    // on empty space: not a call
    g.pointer_down(1, 60.0, 60.0);
    g.drop_hand(1);
    assert!(!g.progress.has(Discovery::Call));
    assert!(!g.progress.has(Discovery::Poke));
}

#[test]
fn a_cancelled_squeeze_finishes_nothing() {
    let mut g = game();
    g.pointer_down(1, 420.0, 260.0);
    g.pointer_down(2, 480.0, 260.0);
    g.pointer_down(3, 400.0, 500.0);
    for k in 1..=6 {
        let f = k as f32 / 6.0;
        g.pointer_move(1, 420.0 + 20.0 * f, 260.0, 1.0 / 60.0);
        g.pointer_move(2, 480.0 - 20.0 * f, 260.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    // a third finger taken away changes nothing
    g.drop_hand(3);
    assert!(matches!(g.gesture, Gesture::Squeezing { .. }));
    // one of the two taken away: the squeeze is dropped, not a pinch
    g.drop_hand(1);
    assert!(matches!(g.gesture, Gesture::Idle));
    g.pointer_up(2);
    wait(&mut g, 0.1);
    assert!(!g.progress.has(Discovery::Pinch) && g.world.forms.bloom.buds.is_empty());
    assert!(g.pointers.is_empty());
}

fn flower(g: &mut Game) {
    for i in 0..5 {
        let a = i as f32 * std::f32::consts::TAU / 5.0;
        let (dx, dy) = (a.cos(), a.sin());
        let (x0, y0) = (400.0 + dx * 50.0, 300.0 + dy * 50.0);
        g.pointer_down(1, x0, y0);
        for k in 1..=3 {
            g.pointer_move(1, x0 + dx * 14.0 * k as f32, y0 + dy * 14.0 * k as f32, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(1);
        wait(g, 0.6);
    }
    wait(g, 4.0);
    assert!(g.progress.has(Discovery::Bloom));
}

#[test]
fn two_fingers_pinched_together_on_the_flower_bring_it_to_dust() {
    let mut g = game();
    flower(&mut g);
    let (x, y) = g.screen_of(g.world.body.pos);
    two_fingers(&mut g, (x - 40.0, y), (x + 40.0, y), (x - 10.0, y), (x + 10.0, y), 0.0);
    assert!(!g.world.forms.dust.whole(), "it comes apart");
}

#[test]
fn the_flower_tugged_a_little_too_slowly_only_flinches() {
    let mut g = game();
    flower(&mut g);
    g.sounds.clear();
    tug(&mut g, (400.0, 300.0), (0.0, 1.0), 9.0, 3, 1.0 / 60.0, 0.5);
    assert!(g.world.forms.dust.whole(), "it did not come apart");
    assert!(g.sounds.contains(&Sound::Almost));
    wait(&mut g, 0.3);
    assert!(g.world.forms.bloom.open() < 0.9, "its petals drew in");
    wait(&mut g, 1.5);
    assert_eq!(g.world.forms.bloom.open(), 1.0);
    // let go at once, the same tug brings it to dust
    tug(&mut g, (400.0, 300.0), (0.0, 1.0), 9.0, 3, 1.0 / 60.0, 0.0);
    assert!(!g.world.forms.dust.whole());
}

/// A flower pinched to dust and swept into a mound; where the mound is on screen.
fn mounded() -> (Game, (f32, f32)) {
    let mut g = game();
    flower(&mut g);
    // pulled a long way, the flower only looks around
    g.pointer_down(1, 400.0, 300.0);
    g.pointer_move(1, 400.0, 420.0, 1.0 / 60.0);
    g.pointer_up(1);
    wait(&mut g, 0.5);
    assert!(g.world.forms.dust.whole());
    // a short pull let go of at once: it comes apart
    g.pointer_down(1, 400.0, 300.0);
    g.update(1.0 / 60.0);
    g.pointer_move(1, 400.0, 325.0, 1.0 / 60.0);
    g.update(1.0 / 60.0);
    g.pointer_up(1);
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Dust));
    assert!(g.world.forms.bloom.whole() && !g.world.ordinary());
    assert!(
        g.world.raycast(g.cam.basis().pos, (g.world.body.pos - g.cam.basis().pos).norm()).is_none(),
        "nothing left to touch"
    );
    // sweep each drift into the fullest one, with strokes across the floor
    let floor = |x: f32, z: f32| v3(x, fusion::FLOOR_Y, z);
    for _ in 0..12 {
        if g.world.forms.dust.phase != forms::dust::Phase::Dust {
            break;
        }
        let k = g.world.forms.dust.biggest();
        let to = g.world.forms.dust.clumps[k];
        for i in 0..forms::dust::CLUMPS {
            let c = g.world.forms.dust.clumps[i];
            if i == k || c.amount <= 0.0 {
                continue;
            }
            let (ox, oz) = (c.x - to.x, c.z - to.z);
            let from = g.screen_of(floor(c.x + ox * 0.15, c.z + oz * 0.15));
            let end = g.screen_of(floor(to.x, to.z));
            g.pointer_down(3, from.0, from.1);
            for s in 1..=8 {
                let f = s as f32 / 8.0;
                g.pointer_move(3, from.0 + (end.0 - from.0) * f, from.1 + (end.1 - from.1) * f, 1.0 / 60.0);
            }
            g.pointer_up(3);
        }
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Sweep));
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Mound), "{:?}", g.world.forms.dust.clumps);
    let (hx, hz, _, h) = g.world.forms.dust.heap();
    let on = g.screen_of(floor(hx, hz) + V3::UP * (h * 0.5));
    (g, on)
}

#[test]
fn a_pinched_flower_goes_to_dust_that_sweeps_into_a_mound_and_held_it_rises() {
    let (mut g, on) = mounded();
    g.pointer_down(4, on.0, on.1);
    g.pointer_up(4);
    assert!(g.world.forms.dust.thud > 0.5, "it answers a tap");
    g.pointer_down(5, on.0, on.1);
    wait(&mut g, 1.5);
    g.pointer_up(5);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Unearth));
    assert!(g.world.ordinary());
    assert!(g.world.body.pos.len() < 0.3, "it floats where it did");
}

#[test]
fn held_taut_it_becomes_a_thread_that_plucks_and_winds_back() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for k in 1..=20 {
        g.pointer_move(1, 400.0 + 7.5 * k as f32, 300.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    wait(&mut g, 1.5);
    assert!(g.world.drops.is_empty(), "it did not tear");
    assert!(
        !g.world.forms.filament.whole(),
        "stretch {:?}",
        g.world.handles.iter().map(|h| h.stretch(&g.world.body)).collect::<Vec<_>>()
    );
    g.pointer_up(1);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Thread));
    // the thread runs across the screen, far past where the sphere was
    let ax = g.world.forms.filament.axis;
    assert!(ax.dot(g.cam.basis().fwd).abs() < 0.05);
    g.pointer_down(2, 400.0, 300.0);
    g.pointer_up(2);
    wait(&mut g, 0.1);
    assert!(g.progress.has(Discovery::Pluck));
    g.pointer_down(3, 400.0, 300.0);
    wait(&mut g, 1.0);
    g.pointer_up(3);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Wind));
    assert!(g.world.ordinary());
}

#[test]
fn stirring_round_and_round_makes_a_universe() {
    let mut g = game();
    g.pointer_down(1, 400.0 + 60.0, 300.0);
    for k in 1..=(60 * 3) {
        let a = k as f32 / 60.0 * std::f32::consts::TAU;
        g.pointer_move(1, 400.0 + 95.0 * a.cos(), 300.0 - 95.0 * a.sin(), 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(1);
    assert!(g.progress.has(Discovery::Stir));
    assert!(g.world.drops.is_empty(), "stirring did not tear it");
    assert!(g.world.forms.filament.whole(), "stirring is not holding it taut");
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Cosmos));
    assert_eq!(g.world.forms.cosmos.sense, 1.0, "counterclockwise");
}

/// Stir round and round, `turns` times, counterclockwise if positive.
fn stir(g: &mut Game, turns: f32) {
    g.pointer_down(1, 400.0 + 50.0, 300.0);
    for k in 1..=((60.0 * turns.abs()) as usize) {
        let a = k as f32 / 60.0 * std::f32::consts::TAU * turns.signum();
        g.pointer_move(1, 400.0 + 90.0 * a.cos(), 300.0 - 90.0 * a.sin(), 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(1);
}

#[test]
fn a_universe_stirred_on_folds_the_room() {
    let mut g = game();
    stir(&mut g, 3.0);
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Cosmos));
    stir(&mut g, 4.0);
    wait(&mut g, 5.0);
    assert!(g.progress.has(Discovery::Everywhere));
    assert_eq!(g.world.forms.fold.phase, forms::fold::Phase::Folded);
    // a poke: another mirror
    g.pointer_down(2, 400.0, 300.0);
    g.pointer_up(2);
    wait(&mut g, 0.1);
    assert!(g.progress.has(Discovery::Facet));
    // held: its time runs backward
    g.pointer_down(3, 400.0, 300.0);
    wait(&mut g, 2.5);
    g.pointer_up(3);
    assert!(g.progress.has(Discovery::Backward));
    // the other way folds the room back up around the universe
    stir(&mut g, -3.0);
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Crease));
    assert!(g.world.forms.fold.whole());
    assert_eq!(g.world.forms.cosmos.phase, forms::cosmos::Phase::Cosmos);
    // and once more the other way, it is liquid again
    stir(&mut g, -3.0);
    wait(&mut g, 4.0);
    assert!(g.world.ordinary());
}

#[test]
fn a_square_drawn_around_it_makes_a_box_that_opens_on_the_third_knock() {
    let mut g = game();
    let corners = [(150.0, 80.0), (650.0, 80.0), (650.0, 520.0), (150.0, 520.0), (150.0, 80.0)];
    g.pointer_down(1, corners[0].0, corners[0].1);
    for w in corners.windows(2) {
        for k in 1..=25 {
            let f = k as f32 / 25.0;
            g.pointer_move(1, w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
    }
    g.pointer_up(1);
    wait(&mut g, 0.1);
    assert!(g.progress.has(Discovery::Corners));
    assert!(g.world.division.whole(), "drawing around it does not cut it");
    wait(&mut g, 2.5);
    for i in 0..3 {
        g.pointer_down(10 + i, 400.0, 300.0);
        g.pointer_up(10 + i);
        wait(&mut g, 0.3);
    }
    assert!(g.progress.has(Discovery::Knock) && g.progress.has(Discovery::Lid));
    assert!(g.world.burst.wounds < 1.0, "knocking is not wounding");
    wait(&mut g, 3.0);
    assert!(g.world.drops.len() >= crate::sim::OPEN_AT, "{} drops", g.world.drops.len());
    assert!(g.progress.has(Discovery::Open));
    wait(&mut g, 150.0);
    assert!(
        g.progress.has(Discovery::Close),
        "{:?}",
        g.world.drops.iter().map(|d| (d.pos, d.vel.len())).collect::<Vec<_>>()
    );
    assert!(g.progress.has(Discovery::Round));
    assert!(g.world.ordinary());
}

#[test]
fn once_the_camera_has_drifted_back_from_the_box_it_lets_the_floor_tilt_it() {
    let mut g = game();
    let corners = [(150.0, 80.0), (650.0, 80.0), (650.0, 520.0), (150.0, 520.0), (150.0, 80.0)];
    g.pointer_down(1, corners[0].0, corners[0].1);
    for w in corners.windows(2) {
        for k in 1..=25 {
            let f = k as f32 / 25.0;
            g.pointer_move(1, w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
    }
    g.pointer_up(1);
    wait(&mut g, 2.6);
    for i in 0..3 {
        g.pointer_down(10 + i, 400.0, 300.0);
        g.pointer_up(10 + i);
        wait(&mut g, 0.3);
    }
    wait(&mut g, 153.0);
    assert!(g.world.ordinary());
    // thrown into the floor, the camera tilts down to the lake as it would have without the box
    g.pointer_down(1, 400.0, 300.0);
    for i in 1..=4 {
        g.pointer_move(1, 400.0, 300.0 + i as f32 * 40.0, 1.0 / 120.0);
        g.update(1.0 / 120.0);
    }
    g.pointer_up(1);
    wait(&mut g, 10.0);
    assert_eq!(g.world.fusion.phase, fusion::Phase::Fused);
    let mut h = slammed();
    wait(&mut h, 4.0);
    assert!(
        (g.cam.pitch - h.cam.pitch).abs() < 0.05,
        "pitch {} with the box first, {} without",
        g.cam.pitch,
        h.cam.pitch
    );
}

fn burst_it(g: &mut Game) {
    for i in 0..12 {
        g.pointer_down(i, 400.0, 300.0);
        g.pointer_up(i);
        for _ in 0..12 {
            g.update(1.0 / 60.0);
        }
    }
    for _ in 0..240 {
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.burst.phase, burst::Phase::Landscape);
}

/// The screen point of piece `i`, partway up so the press lands on it.
fn shard_at(g: &Game, i: usize) -> (f32, f32) {
    let (pos, _) = g.world.burst.shard_pose(i);
    g.screen_of(pos + V3::UP * (g.world.burst.shard_size(i) * 0.3))
}

#[test]
fn in_the_landscape_pieces_sing_root_and_can_be_carried_home() {
    let mut g = game();
    burst_it(&mut g);
    // three quick taps on different pieces: a chord
    for (n, i) in [0usize, 1, 2].into_iter().enumerate() {
        let (x, y) = shard_at(&g, i);
        g.pointer_down(30 + n as i32, x, y);
        g.pointer_up(30 + n as i32);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Chord));
    assert!(g.sounds.contains(&Sound::Chord));

    // hold one still and it roots
    let (x, y) = shard_at(&g, 5);
    g.pointer_down(40, x, y);
    for _ in 0..70 {
        g.update(1.0 / 60.0);
    }
    g.pointer_up(40);
    assert!(g.progress.has(Discovery::Plant));
    assert!(g.world.burst.land.roots[5].rooted);
    // and holding it again lets it go
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }
    let (x, y) = shard_at(&g, 5);
    g.pointer_down(41, x, y);
    for _ in 0..70 {
        g.update(1.0 / 60.0);
    }
    g.pointer_up(41);
    assert!(!g.world.burst.land.roots[5].rooted);
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }

    // carry every piece to the middle and it is whole again, in the last one's color
    let middle = v3(0.0, fusion::FLOOR_Y, 0.6);
    let mut id = 50;
    for _ in 0..60 {
        let Some(i) = (0..burst::SHARDS).find(|&i| !g.world.burst.land.returned(i)) else { break };
        let (x0, y0) = shard_at(&g, i);
        let (x1, y1) = g.screen_of(middle);
        id += 1;
        g.pointer_down(id, x0, y0);
        for k in 1..=30 {
            let f = k as f32 / 30.0;
            g.pointer_move(id, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(id);
        for _ in 0..30 {
            g.update(1.0 / 60.0);
        }
    }
    assert!(g.progress.has(Discovery::Carry));
    for _ in 0..300 {
        g.update(1.0 / 60.0);
    }
    assert!(
        g.progress.has(Discovery::Mend),
        "{:?}",
        (0..12).map(|i| g.world.burst.land.returned(i)).collect::<Vec<_>>()
    );
    assert!(g.world.burst.whole());
    assert!(g.world.body.color != crate::sim::PEARL);
}

#[test]
fn a_drop_out_when_it_bursts_waits_for_the_body_to_gather_before_falling_back_in() {
    let mut g = game();
    assert!(g.world.spill(v3(2.5, 0.5, 0.0), V3::ZERO, 0.2, crate::sim::PEARL));
    for d in &mut g.world.drops {
        d.age = 0.0;
    }
    burst_it(&mut g);
    g.sounds.clear();
    let before = g.world.drops.len();
    assert!(before > 0);
    let mut merged = 0;
    for _ in 0..(60 * 30) {
        g.update(1.0 / 60.0);
        merged += g.sounds.drain(..).filter(|s| matches!(s, Sound::Merge)).count();
    }
    assert!(g.world.burst.body_gone());
    assert_eq!(merged, 0, "a drop fell into a body that is not there");
    assert_eq!(g.world.drops.len(), before);
}

/// Drag piece `i` slowly over the ground to `to`, and let go.
fn carry_to(g: &mut Game, id: i32, i: usize, to: V3) {
    let (x0, y0) = shard_at(g, i);
    let (x1, y1) = g.screen_of(to);
    g.pointer_down(id, x0, y0);
    for k in 1..=60 {
        let f = k as f32 / 60.0;
        g.pointer_move(id, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(id);
    for _ in 0..30 {
        g.update(1.0 / 60.0);
    }
}

#[test]
fn set_down_together_they_keep_together_in_as_many_groups_as_you_like_and_a_cut_parts_them() {
    let mut g = game();
    burst_it(&mut g);
    // three pairs, each in its own place
    for (n, (a, b)) in [(0usize, 1usize), (2, 3), (4, 5)].into_iter().enumerate() {
        let turn = n as f32 * 2.1 + 0.4;
        let at = v3(turn.cos() * 22.0, fusion::FLOOR_Y, turn.sin() * 22.0);
        let side = v3(-at.z, 0.0, at.x).norm();
        carry_to(&mut g, 70 + 2 * n as i32, a, at);
        carry_to(&mut g, 71 + 2 * n as i32, b, at + side * 3.6);
        assert_eq!(
            g.world.burst.kin.group(a),
            (1 << a) | (1 << b),
            "{a} and {b} hold on, only to each other: {:?} {:?} {:?}",
            g.world.burst.shards[a].home,
            g.world.burst.shards[b].home,
            g.cam.basis().pos
        );
    }
    assert!(g.progress.has(Discovery::Kin));
    assert!(g.progress.has(Discovery::Many));
    assert!(!g.world.burst.river.active, "small groups do not run");

    // touch one and the other feels it after
    let (x, y) = shard_at(&g, 0);
    g.pointer_down(80, x, y);
    g.pointer_up(80);
    g.sounds.clear();
    for _ in 0..30 {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Pass));
    assert!(g.sounds.iter().any(|s| matches!(s, Sound::Shard { i: 0 | 1 })), "the other one of the pair answers after");

    // a fast cut across the ground between them, and they part
    let (pa, pb) = (shard_at(&g, 4), shard_at(&g, 5));
    let mid = ((pa.0 + pb.0) * 0.5, (pa.1 + pb.1) * 0.5);
    let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
    let l = (dx * dx + dy * dy).sqrt();
    let across = (-dy / l, dx / l);
    let from = (mid.0 - across.0 * 80.0, mid.1 - across.1 * 80.0);
    g.pointer_down(81, from.0, from.1);
    assert!(matches!(g.gesture, Gesture::Ground { .. }), "the cut starts on the ground");
    for k in 1..=4 {
        let f = k as f32 * 40.0;
        g.pointer_move(81, from.0 + across.0 * f, from.1 + across.1 * f, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    g.pointer_up(81);
    assert!(g.progress.has(Discovery::Part));
    assert!(!g.world.burst.kin.bonded(4, 5));
    assert!(g.world.burst.kin.bonded(0, 1) && g.world.burst.kin.bonded(2, 3), "the other groups were not touched");
    // and each side steps back from the cut
    let gap = (g.world.burst.shards[4].home - g.world.burst.shards[5].home).len();
    for _ in 0..120 {
        g.update(1.0 / 60.0);
    }
    assert!((g.world.burst.shards[4].home - g.world.burst.shards[5].home).len() > gap + 1.0);
}

#[test]
fn a_group_carried_to_the_middle_goes_home_together() {
    let mut g = game();
    burst_it(&mut g);
    let home = g.world.burst.shards[0].home;
    let side = v3(-home.z, 0.0, home.x).norm();
    carry_to(&mut g, 90, 1, home + side * 3.6);
    let group = g.world.burst.kin.group(0);
    assert!(group & 0b10 != 0, "they hold on");
    // slowly, so the others keep up
    let (x0, y0) = shard_at(&g, 0);
    let (x1, y1) = g.screen_of(v3(0.0, fusion::FLOOR_Y, 0.6));
    g.pointer_down(91, x0, y0);
    for k in 1..=400 {
        let f = k as f32 / 400.0;
        g.pointer_move(91, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    assert_eq!(g.world.burst.kin.group(0), group, "they kept up");
    g.pointer_up(91);
    for i in 0..burst::SHARDS {
        assert_eq!(g.world.burst.land.returned(i), group & (1 << i) != 0, "piece {i}");
    }
}

#[test]
fn crowded_together_they_run_red_and_a_view_full_of_columns_gives_it_a_pulse() {
    let mut g = game();
    burst_it(&mut g);
    // crowd every piece together off to one side
    let c = v3(9.0, fusion::FLOOR_Y, -5.0);
    for i in 0..burst::SHARDS {
        let a = i as f32 * 2.399_963;
        let r = 1.9 * (i as f32).sqrt();
        assert!(g.world.burst.carry(i, c + v3(a.cos() * r, 0.0, a.sin() * r)));
        g.world.burst.release(i, g.world.body.radius());
    }
    wait(&mut g, 6.0);
    assert!(g.progress.has(Discovery::River));
    assert!(!g.progress.has(Discovery::Mend));

    // a tap on the river stands a column up out of it
    let river = &g.world.burst.river;
    let slot = burst::river::LANES * 12 + 1;
    let (x, z) = river.slot_pos(slot);
    let (px, py) = g.screen_of(v3(x, fusion::FLOOR_Y, z));
    g.pointer_down(60, px, py);
    g.pointer_up(60);
    assert!(g.sounds.contains(&Sound::Column { slot }));
    wait(&mut g, 0.6);
    assert!(g.progress.has(Discovery::Column));
    assert!(g.world.burst.river.height(slot) > 2.0);
    assert!(!g.world.burst.whole());

    // drag along it, lane by lane, until the view is full of them
    for lane in 0..burst::river::LANES {
        let id = 61 + lane as i32;
        let at = |g: &Game, row: usize| {
            let (x, z) = g.world.burst.river.slot_pos(row * burst::river::LANES + lane);
            g.screen_of(v3(x, fusion::FLOOR_Y, z))
        };
        let (x0, y0) = at(&g, 0);
        g.pointer_down(id, x0, y0);
        for row in 1..burst::river::ROWS {
            let (x, y) = at(&g, row);
            g.pointer_move(id, x, y, 1.0 / 60.0);
            g.update(1.0 / 60.0);
        }
        g.pointer_up(id);
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Pulse), "{} standing", g.world.burst.river.standing_count());
    assert!(g.sounds.contains(&Sound::Beat));

    // a few beats, then it comes back up out of the river: red, and still beating
    wait(&mut g, 14.0);
    assert!(g.world.burst.whole());
    let col = g.world.body.color;
    assert!(col.x > 0.4 && col.y < 0.4, "{col:?}");
    assert!(g.heart.2 > 0);
}

#[test]
fn left_alone_the_landscape_weathers_and_rolls_home() {
    let mut g = game();
    burst_it(&mut g);
    for _ in 0..(60 * 60) {
        g.update(1.0 / 60.0);
    }
    assert!(g.progress.has(Discovery::Weather));
    assert!(g.world.burst.whole());
}

#[test]
fn drag_on_empty_space_orbits() {
    let mut g = game();
    let yaw = g.cam.yaw;
    g.pointer_down(1, 20.0, 20.0);
    g.pointer_move(1, 120.0, 20.0, 1.0 / 60.0);
    g.pointer_up(1);
    assert!(g.cam.yaw != yaw);
    assert!(g.world.ripples.is_empty());
}

#[test]
fn the_flower_spun_left_past_dizzy_flies_apart_in_ropes_and_comes_back_liquid() {
    let mut g = game();
    flower(&mut g);
    stir(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Sway), "it sways");
    assert!(g.world.forms.dust.whole() && g.world.forms.cosmos.whole(), "spinning is neither a pinch nor a stir");
    stir(&mut g, 4.0);
    wait(&mut g, 4.0);
    assert!(g.progress.has(Discovery::Ropes));
    wait(&mut g, 8.0);
    assert!(g.world.ordinary(), "it is the liquid again");
    assert!(!g.progress.has(Discovery::Fold), "it was not folded");
}

#[test]
fn the_flower_spun_right_past_dizzy_rises_and_lands_a_flower() {
    let mut g = game();
    flower(&mut g);
    stir(&mut g, -6.0);
    wait(&mut g, 3.0);
    assert!(g.progress.has(Discovery::Aloft));
    assert!(g.world.forms.spin.offset().y > 0.9, "off the floor");
    // a tap while it is up does not fold it
    let (x, y) = g.screen_of(g.world.body.pos + g.world.forms.spin.offset() * g.world.body.radius());
    g.pointer_down(1, x, y);
    g.pointer_up(1);
    wait(&mut g, 12.0);
    assert_eq!(g.world.forms.bloom.phase, bloom::Phase::Flower);
    assert!(g.world.forms.spin.whole());
}

#[test]
fn brought_near_dizzy_again_and_again_over_several_visits_it_learns_which_way_you_turn_it() {
    let mut kept = String::new();
    for visit in 0..3 {
        let mut g = game();
        g.remembrance = crate::mind::remembrance::Remembrance::load(&kept, 0.0);
        g.update(1.0 / 60.0);
        flower(&mut g);
        for _ in 0..3 {
            stir(&mut g, 2.7);
            wait(&mut g, 6.0);
        }
        assert!(g.world.forms.spin.whole());
        if visit < 2 {
            assert!(g.world.forms.spin.learned.is_none(), "not in one visit, nor two");
        } else {
            assert_eq!(g.world.forms.spin.learned, Some(1.0));
            stir(&mut g, 0.6);
            wait(&mut g, 1.0);
            assert!(g.progress.has(Discovery::Braced) || g.progress.has(Discovery::InStep));
        }
        kept = g.remembrance.save();
        assert!(kept.contains("spun="), "{kept}");
    }
}

#[test]
fn the_mound_let_go_of_early_fades_its_light() {
    let (mut g, on) = mounded();
    g.pointer_down(5, on.0, on.1);
    wait(&mut g, 1.0);
    let held = g.world.forms.dust.press;
    assert!(held > 0.5, "lit while pressed: {held}");
    g.pointer_up(5);
    g.update(1.0 / 60.0);
    let after = g.world.forms.dust.press;
    assert!(held - after < 0.1, "pressed light {held} -> {after} in one frame");
    wait(&mut g, 0.5);
    assert_eq!(g.world.forms.dust.press, 0.0, "and it goes out");
}

#[test]
fn the_mound_slid_off_or_joined_by_a_second_finger_fades_its_light() {
    for second in [false, true] {
        let (mut g, on) = mounded();
        g.pointer_down(5, on.0, on.1);
        wait(&mut g, 0.8);
        let held = g.world.forms.dust.press;
        assert!(held > 0.4, "lit while pressed: {held}");
        if second {
            g.pointer_down(6, on.0 + 60.0, on.1);
        } else {
            g.pointer_move(5, on.0 + 40.0, on.1, 1.0 / 60.0);
        }
        g.update(1.0 / 60.0);
        let after = g.world.forms.dust.press;
        assert!(held - after < 0.1, "pressed light {held} -> {after} in one frame");
        wait(&mut g, 0.5);
        assert_eq!(g.world.forms.dust.press, 0.0, "and it goes out");
        assert!(!g.world.forms.dust.whole(), "it did not rise");
    }
}

#[test]
fn held_taut_the_tendril_sinks_back_as_it_becomes_the_thread() {
    let mut g = game();
    g.pointer_down(1, 400.0, 300.0);
    for k in 1..=20 {
        g.pointer_move(1, 400.0 + 7.5 * k as f32, 300.0, 1.0 / 60.0);
        g.update(1.0 / 60.0);
    }
    let mut last: Option<f32> = None;
    for f in 0..120 {
        g.update(1.0 / 60.0);
        let b = &g.world.body;
        let shown = g.world.handles.iter().map(|h| (h.shown_tip(b) - h.anchor(b)).len()).fold(0.0, f32::max);
        if let Some(l) = last {
            assert!(l - shown < 0.15, "frame {f}: tendril {l} -> {shown}, morph {}", g.world.forms.filament.morph());
        }
        last = Some(shown);
    }
}
