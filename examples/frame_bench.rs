//! Times the simulation natively, a frame at a time, and prints a fingerprint of where
//! each scene ends up, so a speed change can be checked for having changed nothing else.
//!
//!     cargo run --release --example frame_bench
//!
//! Each scene drives `Game` with the same pointer input every run, then steps it and
//! times only the steady frames after. Compare the fingerprints before and after a change:
//! they should match exactly.
//!
//!     cargo run --release --example frame_bench -- --check   # fails if any differs from fingerprints.txt
//!     cargo run --release --example frame_bench -- --bless   # writes the current ones there
//!
//! CI runs `--check`. A change that is meant to move the simulation blesses the new ones in
//! the same pull request and says why.

use chaos_object::game::Game;
use chaos_object::input::senses::{self, Leave, Retina};
use std::f32::consts::TAU;
use std::hint::black_box;
use std::time::Instant;

const DT: f32 = 1.0 / 60.0;
const FRAMES: usize = 6000;

fn game() -> Game {
    let mut g = Game::new();
    g.w = 800.0;
    g.h = 600.0;
    g
}

fn wait(g: &mut Game, secs: f32) {
    for _ in 0..(secs * 60.0) as usize {
        g.update(DT);
    }
}

fn stir(g: &mut Game, turns: f32) {
    g.pointer_down(1, 450.0, 300.0);
    for k in 1..=((60.0 * turns) as usize) {
        let a = k as f32 / 60.0 * TAU;
        g.pointer_move(1, 400.0 + 90.0 * a.cos(), 300.0 - 90.0 * a.sin(), DT);
        g.update(DT);
    }
    g.pointer_up(1);
}

fn swipe_down(g: &mut Game, id: i32, x: f32) {
    g.pointer_down(id, x, 20.0);
    for i in 1..=6 {
        g.pointer_move(id, x, 20.0 + i as f32 * 95.0, DT);
        g.update(DT);
    }
    g.pointer_up(id);
    wait(g, 4.0);
}

/// Tears drops off in every direction and leaves them loose.
fn torn(g: &mut Game) {
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU;
        g.pointer_down(1, 400.0, 300.0);
        for i in 1..=40 {
            g.pointer_move(1, 400.0 + a.cos() * i as f32 * 8.0, 300.0 + a.sin() * i as f32 * 8.0, DT);
            g.update(DT);
        }
        g.pointer_up(1);
        wait(g, 0.3);
    }
}

fn burst(g: &mut Game) {
    for i in 0..12 {
        g.pointer_down(i, 400.0, 300.0);
        g.pointer_up(i);
        wait(g, 0.2);
    }
    wait(g, 4.0);
}

/// Where everything ended up, folded into one number.
fn fingerprint(g: &Game) -> u64 {
    let w = &g.world;
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |x: f32| {
        h ^= x.to_bits() as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    for v in [w.body.pos, w.body.vel, w.body.color, w.gaze_dir] {
        mix(v.x);
        mix(v.y);
        mix(v.z);
    }
    for x in [w.time, w.agitation, w.night, w.gaze, w.regard, w.dim, w.body.squash, g.cam.dist, g.cam.pitch, g.cam.yaw] {
        mix(x);
    }
    for d in &w.drops {
        mix(d.pos.x);
        mix(d.pos.y);
        mix(d.pos.z);
    }
    for r in &w.ripples {
        mix(r.amp);
    }
    mix(g.progress.count() as f32);
    mix(g.progress.save().len() as f32);
    h
}

/// A scene's name, and what is done to the object to reach it.
type Scene = (&'static str, fn(&mut Game));

/// The committed fingerprints, one `name hex` per line.
const EXPECTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/fingerprints.txt");

fn main() {
    let mut prints: Vec<(&str, u64)> = Vec::new();
    let scenes: [Scene; 6] = [
        ("still", |_| {}),
        ("torn", torn),
        ("stirred", |g| {
            stir(g, 3.0);
            wait(g, 4.0);
        }),
        ("folded", |g| {
            stir(g, 3.0);
            wait(g, 4.0);
            stir(g, 4.0);
            wait(g, 5.0);
        }),
        ("cells", |g| {
            swipe_down(g, 1, 400.0);
            swipe_down(g, 2, 577.0);
            swipe_down(g, 3, 400.0);
        }),
        ("landscape", burst),
    ];
    let mut total = 0.0;
    for (name, setup) in scenes {
        let mut g = game();
        setup(&mut g);
        let t = Instant::now();
        for _ in 0..FRAMES {
            g.update(black_box(DT));
            g.sounds.clear();
            g.announcements.clear();
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / FRAMES as f64;
        total += us;
        let f = fingerprint(&g);
        prints.push((name, f));
        println!("{name:>10}  {us:7.2} µs/frame  fingerprint {f:016x}");
    }
    println!("{:>10}  {total:7.2} µs", "sum");
    let (f, analysis) = senses_scene();
    prints.push(("senses", f));
    prints.push(("analysis", analysis));
    let voice: Vec<f32> = (0..2048).map(|i| (i as f32 / 48000.0 * 220.0 * TAU).sin() * 0.3).collect();
    let t = Instant::now();
    let mut p = 0.0;
    for _ in 0..2000 {
        p += senses::hear(black_box(&voice), 48000.0).pitch.unwrap_or(0.0);
    }
    println!("{:>10}  {:7.2} µs/call  pitch {p}", "hear", t.elapsed().as_secs_f64() * 1e6 / 2000.0);
    let now: String = prints.iter().map(|(n, f)| format!("{n} {f:016x}\n")).collect();
    match std::env::args().nth(1).as_deref() {
        Some("--bless") => {
            std::fs::write(EXPECTED, &now).unwrap();
            println!("wrote {EXPECTED}");
        }
        Some("--check") => {
            let want = std::fs::read_to_string(EXPECTED).unwrap();
            if want != now {
                eprintln!("fingerprints differ from examples/fingerprints.txt:\n--- expected\n{want}--- now\n{now}");
                std::process::exit(1);
            }
            println!("fingerprints match examples/fingerprints.txt");
        }
        _ => {}
    }
}

/// Hearing and sight open, a voice and a moving picture fed in every frame, as platform/web/ does.
/// Returns the world's fingerprint and one of everything the analysis produced.
fn senses_scene() -> (u64, u64) {
    let mut g = game();
    g.senses.hear = Leave::Open;
    g.senses.see = Leave::Open;
    let mut retina = Retina::default();
    let mut px = vec![0u8; 32 * 24 * 4];
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |x: f32| {
        h ^= x.to_bits() as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    let mut frame = |g: &mut Game, k: usize, mix: &mut dyn FnMut(f32)| {
        let t = k as f32 / 60.0;
        let f = 180.0 + 60.0 * (t * 0.7).sin();
        let amp = if (k / 90).is_multiple_of(2) { 0.3 } else { 0.005 };
        let samples: Vec<f32> = (0..2048).map(|i| (i as f32 / 48000.0 * f * TAU).sin() * amp).collect();
        let heard = senses::hear(&samples, 48000.0);
        for (i, p) in px.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i % 32, i / 32);
            let lit = ((x as f32 * 0.4 + t * 3.0).sin() * 60.0 + 120.0 + y as f32) as u8;
            p.copy_from_slice(&[lit, lit / 2, 255 - lit, 255]);
        }
        let seen = retina.look(&px, 32, 24);
        mix(heard.level);
        mix(heard.pitch.unwrap_or(-1.0));
        mix(heard.rough);
        mix(seen.light);
        mix(seen.motion);
        g.senses.heard(heard);
        g.senses.seen(seen);
        g.update(DT);
        g.sounds.clear();
        g.announcements.clear();
    };
    for k in 0..600 {
        frame(&mut g, k, &mut mix);
    }
    let start = Instant::now();
    for k in 600..(600 + FRAMES / 4) {
        frame(&mut g, k, &mut mix);
    }
    let us = start.elapsed().as_secs_f64() * 1e6 / (FRAMES / 4) as f64;
    let f = fingerprint(&g);
    mix(f as f32);
    println!("{:>10}  {us:7.2} µs/frame  fingerprint {:016x} (analysis {h:016x})", "senses", f);
    (f, h)
}
