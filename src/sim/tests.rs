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
    let before: Vec<(u32, f32, f32)> = w.handles.iter().map(|h| (h.id, (h.tip - h.rest(&w.body)).len(), h.vel.len())).collect();
    w.begin_stretch(w.body.pos + dir(MAX_HANDLES) * r).unwrap();
    let (_, off, speed) = before.into_iter().find(|b| !w.handles.iter().any(|h| h.id == b.0)).unwrap();
    assert!(off < 0.03 && speed < 0.3, "a stump {off} out from rest at {speed} a second vanished in one frame");
}
