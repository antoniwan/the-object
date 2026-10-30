//! Tests: rays find the body, a long pull tears off a drop, drops come home, nothing is lost.

use super::*;

fn run(w: &mut World, seconds: f32) {
    for _ in 0..(seconds * 120.0) as usize {
        w.step(1.0 / 120.0);
    }
}

#[test]
fn camera_ray_hits_the_body() {
    let w = World::new();
    let hit = w.raycast(v3(0.0, 0.0, 4.0), v3(0.0, 0.0, -1.0)).unwrap();
    assert_eq!(hit.target, Target::Body);
    assert!((hit.pos.z - 1.0).abs() < 0.01);
}

#[test]
fn pulling_far_tears_a_drop_and_volume_is_conserved() {
    let mut w = World::new();
    let v0 = w.total_volume();
    let h = w.begin_stretch(v3(1.0, 0.0, 0.0)).unwrap();
    w.pull_handle(h, v3(4.0, 0.0, 0.0));
    run(&mut w, 1.0);
    let events = w.drain_events();
    assert!(matches!(events[0], Event::Tore { .. }), "{events:?}");
    assert_eq!(w.drops.len(), 1);
    assert!((w.total_volume() - v0).abs() < 1e-4);
}

#[test]
fn a_released_drop_drifts_home_and_merges() {
    let mut w = World::new();
    let v0 = w.total_volume();
    let h = w.begin_stretch(v3(1.0, 0.0, 0.0)).unwrap();
    w.pull_handle(h, v3(4.0, 0.0, 0.0));
    run(&mut w, 1.0);
    let id = w.drops[0].id;
    w.release_drop(id);
    run(&mut w, 15.0);
    assert!(w.drops.is_empty());
    assert!(w.drain_events().contains(&Event::MergedIntoBody { drop: id }));
    assert!((w.total_volume() - v0).abs() < 1e-4);
    assert!(w.handles.is_empty(), "stump should settle and disappear");
}

#[test]
fn a_short_pull_springs_back() {
    let mut w = World::new();
    let h = w.begin_stretch(v3(0.0, 1.0, 0.0)).unwrap();
    w.pull_handle(h, v3(0.0, 1.5, 0.0));
    run(&mut w, 0.5);
    assert!(w.drops.is_empty());
    w.release_handle(h);
    run(&mut w, 4.0);
    assert!(w.handles.is_empty());
}

#[test]
fn tearing_never_leaves_less_than_the_smallest_body() {
    let mut w = World::new();
    // just over the smallest it may be: one more drop would take it under
    w.body.volume = (MIN_BODY_RADIUS + 0.002).powi(3);
    let h = w.begin_stretch(w.body.pos + v3(w.body.radius(), 0.0, 0.0)).unwrap();
    w.pull_handle(h, w.body.pos + v3(4.0, 0.0, 0.0));
    run(&mut w, 1.0);
    assert!(w.body.radius() >= MIN_BODY_RADIUS, "{}", w.body.radius());
}

#[test]
fn a_pull_past_the_most_stumps_lets_go_of_the_one_nearest_settled() {
    let mut w = World::new();
    let r = w.body.radius();
    let dir = |k: usize| v3((k as f32 * 1.4).cos(), (k as f32 * 1.4).sin(), 0.3).norm();
    // one pulled until it tears, still springing back
    let h = w.begin_stretch(w.body.pos + dir(0) * r).unwrap();
    w.pull_handle(h, w.body.pos + dir(0) * (r * 2.5));
    run(&mut w, 0.3);
    w.release_handle(h);
    run(&mut w, 0.05);
    // and the rest only drawn out a little
    for k in 1..MAX_HANDLES {
        let h = w.begin_stretch(w.body.pos + dir(k) * (r * 0.82)).unwrap();
        w.pull_handle(h, w.body.pos + dir(k) * (r * 0.86));
        run(&mut w, 0.1);
        w.release_handle(h);
        run(&mut w, 0.05);
    }
    run(&mut w, 0.15);
    assert_eq!(w.handles.len(), MAX_HANDLES);
    let before: Vec<(u32, f32, f32)> =
        w.handles.iter().map(|h| (h.id, (h.tip - h.rest(&w.body)).len(), h.vel.len())).collect();
    w.begin_stretch(w.body.pos + dir(MAX_HANDLES) * r).unwrap();
    assert!(before.iter().all(|b| w.handles.iter().any(|h| h.id == b.0)), "a stump vanished in one frame");
    let going: Vec<u32> = w.handles.iter().filter(|h| h.fade > 0.0).map(|h| h.id).collect();
    assert_eq!(going.len(), 1, "one stump makes way");
    let (_, off, speed) = before.into_iter().find(|b| b.0 == going[0]).unwrap();
    assert!(off < 0.03 && speed < 0.3, "a stump {off} out from rest at {speed} a second made way");
}

/// How far out the surface stands along `dir` from the body's center.
fn surface_along(w: &World, dir: V3) -> f32 {
    let mut s = 0.5;
    while w.sdf(w.body.pos + dir * s).0 < 0.0 {
        s += 0.0005;
    }
    s
}

/// The most the surface along `dir` moves in one frame, over `frames` frames at 120 a second.
fn largest_step(w: &mut World, dir: V3, frames: usize) -> f32 {
    let mut was = surface_along(w, dir);
    let mut most = 0.0f32;
    for _ in 0..frames {
        w.step(1.0 / 120.0);
        let now = surface_along(w, dir);
        most = most.max((now - was).abs());
        was = now;
    }
    most
}

/// No more in one frame than the tendril's own motion moves it (pulled half a radius
/// further, its surface goes about 0.033 a frame; let go from 0.3 out, it springs back 0.028),
/// and well under the old pops: 0.12 as a settled stump went, 0.28 as one came.
const STEP_MOST: f32 = 0.03;

#[test]
fn a_settled_stump_sinks_back_without_a_jump() {
    let mut w = World::new();
    let r = w.body.radius();
    let dir = v3(1.0, 0.0, 0.0);
    let h = w.begin_stretch(w.body.pos + dir * r).unwrap();
    w.pull_handle(h, w.body.pos + dir * (r * 1.3));
    run(&mut w, 0.2);
    w.release_handle(h);
    let most = largest_step(&mut w, dir, 600);
    assert!(w.handles.is_empty(), "the stump went");
    assert!((surface_along(&w, dir) - r).abs() < 0.002, "and the surface is round again");
    assert!(most < STEP_MOST * r, "the surface moved {most} in one frame");
}

#[test]
fn a_new_stump_grows_out_without_a_jump() {
    let mut w = World::new();
    let r = w.body.radius();
    let dir = v3(0.0, 0.0, 1.0);
    let hit = w.raycast(v3(0.0, 0.0, 4.0), v3(0.0, 0.0, -1.0)).unwrap();
    let before = surface_along(&w, dir);
    w.begin_stretch(hit.pos).unwrap();
    let jump = (surface_along(&w, dir) - before).abs();
    let most = largest_step(&mut w, dir, 120).max(jump);
    assert!(surface_along(&w, dir) > 1.2 * r, "it still grew out");
    assert!(most < STEP_MOST * r, "the surface moved {most} in one frame");
}

#[test]
fn a_pull_past_the_most_stumps_never_jumps() {
    let mut w = World::new();
    let r = w.body.radius();
    let dir = |k: usize| v3((k as f32 * 1.4).cos(), (k as f32 * 1.4).sin(), 0.3).norm();
    for k in 0..MAX_HANDLES {
        let h = w.begin_stretch(w.body.pos + dir(k) * r).unwrap();
        w.pull_handle(h, w.body.pos + dir(k) * (r * 1.2));
        run(&mut w, 0.1);
        w.release_handle(h);
    }
    run(&mut w, 0.05);
    assert_eq!(w.handles.len(), MAX_HANDLES);
    let mut was: Vec<f32> = (0..=MAX_HANDLES).map(|k| surface_along(&w, dir(k))).collect();
    let h = w.begin_stretch(w.body.pos + dir(MAX_HANDLES) * r).unwrap();
    let mut most = 0.0f32;
    for f in 0..120 {
        if f > 0 {
            w.step(1.0 / 120.0);
        }
        for (k, was) in was.iter_mut().enumerate() {
            let now = surface_along(&w, dir(k));
            most = most.max((now - *was).abs());
            *was = now;
        }
    }
    assert!(w.handles.iter().any(|x| x.id == h), "the new one is there");
    assert!(surface_along(&w, dir(MAX_HANDLES)) > 1.1 * r, "and has grown out");
    assert!(most < STEP_MOST * r, "the surface moved {most} in one frame");
}

#[test]
fn a_stump_with_nothing_out_leaves_it_round() {
    let mut w = World::new();
    let r = w.body.radius();
    let dir = v3(0.0, 1.0, 0.0);
    // wherever its tip is, near the surface or pulled far out
    for out in [0.8, 1.0, 2.0] {
        w.handles = vec![Handle::new(1, dir, w.body.pos + dir * (r * out))];
        w.handles[0].weight = 0.0;
        for p in [dir, v3(0.6, 0.8, 0.0), v3(0.0, 0.8, 0.6)] {
            let d = w.sdf(w.body.pos + p * r).0;
            assert!(d.abs() < 1e-6, "{d} off the surface at {p:?}, the tip {out} out");
        }
    }
}
