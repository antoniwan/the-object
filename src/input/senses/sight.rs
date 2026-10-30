//! Sight, as numbers: a tiny frame reduced to how much light, how much changed since the
//! last look, and where.

/// One look, reduced to what it acts on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seen {
    /// Mean light, 0..1.
    pub light: f32,
    /// How much of the picture changed since the last look, 0..1.
    pub motion: f32,
    /// Where it changed, -1..1 each way, as the player sees it (left is their left, up is up).
    pub at: Option<(f32, f32)>,
}

/// Keeps only the last look's light, to see what changed. No picture is kept past one look.
#[derive(Default)]
pub struct Retina {
    last: Vec<u8>,
    /// This look's light, written into the buffer the look before last used.
    luma: Vec<u8>,
}

impl Retina {
    /// `rgba` is a tiny frame, `w` x `h`, as the camera faces: it is mirrored here.
    pub fn look(&mut self, rgba: &[u8], w: usize, h: usize) -> Seen {
        if w == 0 || h == 0 || rgba.len() < w * h * 4 {
            return Seen::default();
        }
        let mut luma = std::mem::take(&mut self.luma);
        luma.clear();
        luma.extend(
            rgba.chunks_exact(4)
                .take(w * h)
                .map(|p| ((u32::from(p[0]) * 77 + u32::from(p[1]) * 150 + u32::from(p[2]) * 29) >> 8) as u8),
        );
        let light = luma.iter().map(|&l| f32::from(l)).sum::<f32>() / (w * h) as f32 / 255.0;
        let mut seen = Seen { light, motion: 0.0, at: None };
        if self.last.len() == luma.len() {
            let (mut sum, mut wx, mut wy, mut weight) = (0.0, 0.0, 0.0, 0.0);
            for (i, (&a, &b)) in luma.iter().zip(&self.last).enumerate() {
                let d = (f32::from(a) - f32::from(b)).abs() / 255.0;
                sum += d;
                // Small changes are the sensor's own noise.
                if d > 0.08 {
                    let (x, y) = ((i % w) as f32, (i / w) as f32);
                    wx += x * d;
                    wy += y * d;
                    weight += d;
                }
            }
            seen.motion = sum / luma.len() as f32;
            if weight > 0.5 {
                let x = wx / weight / (w - 1).max(1) as f32 * 2.0 - 1.0;
                let y = wy / weight / (h - 1).max(1) as f32 * 2.0 - 1.0;
                // The camera faces the player, so their left is its right; and rows run down.
                seen.at = Some((-x, -y));
            }
        }
        // The look before is let go; only its room is kept, for the next look.
        self.luma = std::mem::replace(&mut self.last, luma);
        self.luma.clear();
        seen
    }

    pub fn forget(&mut self) {
        self.last.clear();
    }
}
