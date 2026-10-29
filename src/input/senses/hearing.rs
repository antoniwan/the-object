//! Hearing, as numbers: a window of samples reduced to how loud, how rough and what pitch,
//! and where a pitch lands on the body.

use super::{PITCH_HI, PITCH_LO, QUIET};

/// One short window of sound, reduced to what it acts on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Heard {
    /// Root mean square, 0..1.
    pub level: f32,
    /// The fundamental, in Hz, when there is one.
    pub pitch: Option<f32>,
    /// Zero crossings per sample, 0..1: high for breath and hiss, low for a voice.
    pub rough: f32,
}

/// Samples in, three numbers out. Pitch is found by autocorrelation on every other sample.
pub fn hear(samples: &[f32], rate: f32) -> Heard {
    let n = samples.len();
    if n < 64 || rate <= 0.0 {
        return Heard::default();
    }
    let level = (samples.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
    let crossings = samples.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
    let rough = crossings as f32 / n as f32;
    let pitch = if level < QUIET { None } else { fundamental(samples, rate) };
    Heard { level, pitch, rough }
}

fn fundamental(samples: &[f32], rate: f32) -> Option<f32> {
    // Every other sample is plenty for a voice, and a quarter of the work.
    // Heard every frame, so both buffers live on the stack.
    let mut halved = [0.0f32; 1024];
    let mut n = 0;
    for p in samples.chunks_exact(2).take(1024) {
        halved[n] = (p[0] + p[1]) * 0.5;
        n += 1;
    }
    let x = &halved[..n];
    let rate = rate * 0.5;
    let lo = (rate / PITCH_HI) as usize;
    let hi = ((rate / PITCH_LO) as usize).min(n / 2);
    if lo < 2 || hi <= lo + 2 {
        return None;
    }
    let corr = |lag: usize| {
        let (mut xy, mut xx, mut yy) = (0.0, 0.0, 0.0);
        for i in 0..n - lag {
            xy += x[i] * x[i + lag];
            xx += x[i] * x[i];
            yy += x[i + lag] * x[i + lag];
        }
        xy / (xx * yy).sqrt().max(1e-9)
    };
    // hi is at most 512, so there are fewer lags than that.
    let mut corrs = [0.0f32; 512];
    let lags = hi - lo + 1;
    let fours = lags - lags % 4;
    for (k, four) in corrs[..fours].chunks_exact_mut(4).enumerate() {
        four.copy_from_slice(&corr4(x, lo + 4 * k));
    }
    for (k, c) in corrs[..lags].iter_mut().enumerate().skip(fours) {
        *c = corr(lo + k);
    }
    let c = &corrs[..lags];
    let best = c.iter().cloned().fold(f32::MIN, f32::max);
    if best < 0.82 {
        return None;
    }
    // The first peak nearly as good as the best, so a note is not heard an octave low.
    let i = (1..c.len() - 1).find(|&i| c[i] >= best * 0.95 && c[i] >= c[i - 1] && c[i] >= c[i + 1])?;
    // Between samples: fit a parabola through the peak.
    let (a, b, d) = (c[i - 1], c[i], c[i + 1]);
    let shift = 0.5 * (a - d) / (a - 2.0 * b + d).min(-1e-6);
    Some(rate / (lo as f32 + i as f32 + shift))
}

/// The correlation at `lag` .. `lag + 4`, in one pass. Each sum adds the same products in
/// the same order as one lag at a time, so the numbers are the same; there are just four
/// of them to work on while each waits for the last.
fn corr4(x: &[f32], lag: usize) -> [f32; 4] {
    let n = x.len();
    let (mut xy0, mut xy1, mut xy2, mut xy3) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut yy0, mut yy1, mut yy2, mut yy3) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    // Until the shorter lags run on, all four add up the same squares in the same order.
    let mut xx = 0.0f32;
    let common = n - lag - 3;
    for (i, &a) in x[..common].iter().enumerate() {
        let y = &x[i + lag..i + lag + 4];
        xx += a * a;
        xy0 += a * y[0];
        yy0 += y[0] * y[0];
        xy1 += a * y[1];
        yy1 += y[1] * y[1];
        xy2 += a * y[2];
        yy2 += y[2] * y[2];
        xy3 += a * y[3];
        yy3 += y[3] * y[3];
    }
    let mut s = [(xy0, xx, yy0), (xy1, xx, yy1), (xy2, xx, yy2), (xy3, xx, yy3)];
    // The shorter lags run on a little further.
    for (j, s) in s.iter_mut().enumerate() {
        for i in common..n - lag - j {
            let (a, b) = (x[i], x[i + lag + j]);
            s.0 += a * b;
            s.1 += a * a;
            s.2 += b * b;
        }
    }
    s.map(|(xy, xx, yy)| xy / (xx * yy).sqrt().max(1e-9))
}

/// Where a pitch lands on the body, 0 (low) to 1 (high), by octaves.
pub(super) fn pitch_height(f: f32) -> f32 {
    ((f / PITCH_LO).log2() / (PITCH_HI / PITCH_LO).log2()).clamp(0.0, 1.0)
}

/// The same note, brought down into its own low register.
pub(super) fn hum_pitch(mut f: f32) -> f32 {
    while f > 262.0 {
        f *= 0.5;
    }
    f
}
