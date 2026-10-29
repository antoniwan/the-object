//! What comes after two. Cut the twins and every cell divides at once, and
//! again, and again; the body never grows, it only gets finer. Four, eight,
//! sixteen, packed by their own pressure into a berry. Past sixteen they stop
//! reading as cells and the body becomes a material: a grain that keeps getting
//! finer with every cut until it is too fine to cut.
//!
//! Three things can happen to it along the way:
//! - at eight, one cell stops being like the others. It becomes an eye, and the
//!   whole cluster turns so the eye faces whatever it is watching;
//! - poke one cell three times and it dies: it greys, it is shed, and the rest
//!   close over the gap. What dies is gone; the body is smaller after;
//! - hold a squeeze and it compacts. The seams close, it goes dark and hard and
//!   rises into a slab, 1 : 4 : 9, holding exactly its volume. Two cells can be
//!   pressed back into one liquid; more than two cannot. Only holding the stone
//!   long enough brings the liquid back.
//!
//! Cells are a tiny particle system, in units of the body radius, centered on
//! the body. Like the rest of meiosis.rs, `Stage` only decides which continuous
//! values move, and the shader mixes them, so nothing jumps.

use crate::math::*;
use crate::division::meiosis::CELL_SCALE;

/// More cells than this and they become grain instead.
pub const MAX_CELLS: usize = 16;
/// Grain stops dividing here.
pub const MAX_GRAIN: u32 = 512;
pub const CLEAVE_SECS: f32 = 2.2;
pub const COMPACT_SECS: f32 = 6.0;
pub const THAW_SECS: f32 = 3.5;
/// Seconds of holding the stone that bring the liquid back.
pub const THAW_HOLD_SECS: f32 = 2.2;
/// Pokes on one cell that kill it.
pub const WOUNDS_TO_DIE: u32 = 3;
const DIE_SECS: f32 = 2.6;
/// The eye appears once there are this many living cells.
const EYE_AT: usize = 8;
/// Holding a squeeze on many cells this long compacts them.
const COMPACT_SQUEEZE_SECS: f32 = 1.2;
/// Cells rest this fraction of their radii apart: they press into each other.
const PACK: f32 = 0.84;
/// Nothing of it reaches further than this from its center, in body radii.
pub const BOUND: f32 = 1.9;
/// The grain body is a little larger than its volume, for the space between grains.
pub const GRAIN_LOOSE: f32 = 1.1;
/// Thickness of the slab in body radii: its sides are 1 : 4 : 9 and it holds the body's volume.
pub fn slab_unit() -> f32 {
    (4.0 / 3.0 * std::f32::consts::PI / 36.0).cbrt()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// Every living cell (or every grain) dividing in two.
    Cleaving { t: f32 },
    Cluster,
    Compacting { t: f32 },
    Stone,
    Thawing { t: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Cleaved,
    BecameGrain,
    Differentiated,
    Died,
    Compacting,
    Slab,
    Thawed,
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// Offset from the body center, in body radii.
    pub off: V3,
    vel: V3,
    pub r: f32,
    /// Radius before and after the division in progress.
    r0: f32,
    r1: f32,
    /// The cell it is dividing from, while it is.
    twin: Option<usize>,
    pub wounds: u32,
    /// 0..1 once it has started to die.
    pub death: Option<f32>,
}

impl Cell {
    fn new(off: V3, r: f32) -> Cell {
        Cell { off, vel: V3::ZERO, r, r0: r, r1: r, twin: None, wounds: 0, death: None }
    }
}

/// The continuous values the shader blends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    /// Glow of the newest cut.
    pub flash: f32,
    /// 0 = separate cells, 1 = grain.
    pub grain: f32,
    /// 0..1 from the previous grain count to the current one.
    pub fine: f32,
    /// 0 = living, 1 = stone.
    pub stone: f32,
    /// 0..1 how far the stone has risen into the slab.
    pub slab: f32,
    /// 0..1 from stone back to liquid.
    pub thaw: f32,
}

/// Rotate `v` about the unit `axis`.
fn rotate(v: V3, axis: V3, angle: f32) -> V3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// A small fixed scatter, so cells cut along the same plane don't stack.
fn jitter(i: usize, salt: u32) -> V3 {
    let f = |k: f32| ((i as f32 * 12.9898 + salt as f32 * 78.233 + k * 37.719).sin() * 43758.547).fract() - 0.5;
    v3(f(1.0), f(2.0), f(3.0))
}

pub struct Morula {
    pub stage: Stage,
    pub cells: Vec<Cell>,
    /// Grain count, once it is grain; 0 while it is still separate cells.
    pub grain: u32,
    prev_grain: u32,
    /// The cell that became an eye, and where it looks from the body center.
    pub eye: Option<usize>,
    pub eye_dir: V3,
    pub eye_open: f32,
    /// The slab faces this way, set when it starts to compact.
    pub slab_yaw: f32,
    pub axis: V3,
    cuts: u32,
    since_cut: f32,
    squeezed_for: f32,
    events: Vec<Event>,
}

impl Morula {
    /// Two cells, exactly where meiosis.rs left them, already starting to divide along `axis`.
    pub fn from_two(two_axis: V3, sep: f32, axis: V3) -> Morula {
        let cells = vec![Cell::new(two_axis * -sep, CELL_SCALE), Cell::new(two_axis * sep, CELL_SCALE)];
        let mut m = Morula {
            stage: Stage::Cluster,
            cells,
            grain: 0,
            prev_grain: 0,
            eye: None,
            eye_dir: v3(0.0, 0.0, 1.0),
            eye_open: 0.0,
            slab_yaw: 0.0,
            axis,
            cuts: 0,
            since_cut: 10.0,
            squeezed_for: 0.0,
            events: Vec::new(),
        };
        m.cleave(axis);
        m
    }

    /// Grain count before the cut in progress.
    pub fn prev_grain(&self) -> u32 {
        self.prev_grain
    }

    pub fn living(&self) -> usize {
        self.cells.iter().filter(|c| c.death.is_none()).count()
    }

    /// Volume still alive, in body radii cubed.
    pub fn volume(&self) -> f32 {
        self.cells.iter().filter(|c| c.death.is_none()).map(|c| c.r1.powi(3)).sum()
    }

    /// Radius of the grain body.
    pub fn grain_radius(&self) -> f32 {
        self.volume().cbrt() * GRAIN_LOOSE
    }

    pub fn can_cut(&self) -> bool {
        self.stage == Stage::Cluster && self.grain < MAX_GRAIN && self.cells.iter().all(|c| c.death.is_none())
    }

    pub fn is_stone(&self) -> bool {
        matches!(self.stage, Stage::Stone | Stage::Thawing { .. } | Stage::Compacting { .. })
    }

    pub fn look(&self) -> Look {
        let mut l = Look { flash: (-self.since_cut * 5.0).exp(), grain: if self.grain > 0 { 1.0 } else { 0.0 }, fine: 1.0, ..Look::default() };
        match self.stage {
            Stage::Cleaving { t } => {
                if self.grain > 0 && self.prev_grain == 0 {
                    l.grain = smoothstep(0.3, CLEAVE_SECS, t);
                } else if self.grain > 0 {
                    l.fine = smoothstep(0.2, CLEAVE_SECS * 0.8, t);
                }
            }
            Stage::Compacting { t } => {
                l.stone = smoothstep(0.0, COMPACT_SECS * 0.45, t);
                l.slab = smoothstep(COMPACT_SECS * 0.4, COMPACT_SECS, t);
            }
            Stage::Stone => {
                l.stone = 1.0;
                l.slab = 1.0;
            }
            Stage::Thawing { t } => {
                l.thaw = smoothstep(0.0, THAW_SECS, t);
                l.stone = 1.0 - l.thaw;
                l.slab = 1.0;
            }
            Stage::Cluster => {}
        }
        l
    }

    /// Every living cell divides along `axis`; past sixteen, the grain gets finer.
    pub fn cleave(&mut self, axis: V3) -> bool {
        if !self.can_cut() {
            return false;
        }
        self.axis = axis.norm();
        self.cuts += 1;
        self.since_cut = 0.0;
        self.prev_grain = self.grain;
        if self.grain > 0 || self.cells.len() * 2 > MAX_CELLS {
            let n = if self.grain > 0 { self.grain } else { self.cells.len() as u32 };
            self.grain = (n * 2).min(MAX_GRAIN);
            if self.prev_grain == 0 {
                self.events.push(Event::BecameGrain);
            }
        } else {
            let mut next = Vec::with_capacity(self.cells.len() * 2);
            for (i, c) in self.cells.iter().enumerate() {
                // Cleavage planes wander a little from the cut, as they do.
                let a = (self.axis + jitter(i, self.cuts) * 0.5).norm();
                let r1 = c.r1 * CELL_SCALE;
                let k = next.len();
                for (side, twin) in [(-1.0, k + 1), (1.0, k)] {
                    let mut child = Cell::new(c.off + a * (0.01 * side), c.r1);
                    child.vel = c.vel + a * (side * 0.6);
                    child.r0 = c.r1;
                    child.r1 = r1;
                    child.twin = Some(twin);
                    next.push(child);
                }
            }
            // The eye passes to one of its daughters.
            self.eye = self.eye.map(|e| e * 2 + 1);
            self.cells = next;
        }
        self.stage = Stage::Cleaving { t: 0.0 };
        true
    }

    /// A poke at `at` (body radii, from the body center). Three on one cell kill it.
    /// Returns true if that poke killed it.
    pub fn wound(&mut self, at: V3) -> bool {
        if self.stage != Stage::Cluster || self.grain > 0 || self.living() <= 2 {
            return false;
        }
        let nearest = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.death.is_none())
            .min_by(|a, b| {
                let da = (at - a.1.off).len() - a.1.r;
                let db = (at - b.1.off).len() - b.1.r;
                da.total_cmp(&db)
            })
            .map(|(i, _)| i);
        let Some(i) = nearest else { return false };
        let c = &mut self.cells[i];
        c.wounds += 1;
        c.vel -= (at - c.off).norm() * 0.6;
        if c.wounds >= WOUNDS_TO_DIE {
            c.death = Some(0.0);
            if self.eye == Some(i) {
                self.eye_open = 0.0;
            }
            self.events.push(Event::Died);
            return true;
        }
        false
    }

    pub fn thaw(&mut self) -> bool {
        if self.stage != Stage::Stone {
            return false;
        }
        self.stage = Stage::Thawing { t: 0.0 };
        true
    }

    /// `squeeze` is the player's squeeze input; `watch` is where the body is looking.
    pub fn step(&mut self, dt: f32, squeeze: f32, watch: V3) {
        self.since_cut += dt;
        self.stage = match self.stage {
            Stage::Cleaving { t } if t + dt >= CLEAVE_SECS => {
                for c in &mut self.cells {
                    c.r = c.r1;
                    c.r0 = c.r1;
                    c.twin = None;
                }
                self.prev_grain = self.grain;
                self.events.push(Event::Cleaved);
                Stage::Cluster
            }
            Stage::Cleaving { t } => Stage::Cleaving { t: t + dt },
            Stage::Cluster => {
                self.squeezed_for = if squeeze.abs() > 0.4 { self.squeezed_for + dt } else { 0.0 };
                if self.squeezed_for > COMPACT_SQUEEZE_SECS {
                    self.squeezed_for = 0.0;
                    self.events.push(Event::Compacting);
                    Stage::Compacting { t: 0.0 }
                } else {
                    Stage::Cluster
                }
            }
            Stage::Compacting { t } if t + dt >= COMPACT_SECS => {
                self.events.push(Event::Slab);
                Stage::Stone
            }
            Stage::Compacting { t } => Stage::Compacting { t: t + dt },
            Stage::Thawing { t } if t + dt >= THAW_SECS => {
                self.events.push(Event::Thawed);
                Stage::Thawing { t: THAW_SECS }
            }
            Stage::Thawing { t } => Stage::Thawing { t: t + dt },
            Stage::Stone => Stage::Stone,
        };
        self.differentiate(watch);
        self.step_cells(dt);
        self.turn_eye(dt, watch);
    }

    /// At eight, the cell facing what it watches becomes different.
    fn differentiate(&mut self, watch: V3) {
        if self.eye.is_some() || self.stage != Stage::Cluster || self.living() < EYE_AT || self.grain > 0 {
            return;
        }
        let best = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.death.is_none())
            .max_by(|a, b| a.1.off.norm().dot(watch).total_cmp(&b.1.off.norm().dot(watch)))
            .map(|(i, _)| i);
        if let Some(i) = best {
            self.eye = Some(i);
            self.eye_dir = self.cells[i].off.norm();
            self.events.push(Event::Differentiated);
        }
    }

    fn step_cells(&mut self, dt: f32) {
        let split = match self.stage {
            Stage::Cleaving { t } => smoothstep(0.25, 1.5, t),
            _ => 1.0,
        };
        let stone = self.look().stone;
        let pack = PACK * (1.0 - 0.4 * stone);
        let pull = 6.0 + 30.0 * stone;
        let n = self.cells.len();
        for c in &mut self.cells {
            if c.twin.is_some() {
                c.r = c.r0 + (c.r1 - c.r0) * split;
            }
        }
        // Neighbors press apart; daughters part as they divide.
        // Never more than MAX_CELLS (cleave turns to grain first), so this lives on the stack.
        let mut force = [V3::ZERO; MAX_CELLS];
        for i in 0..n {
            for j in (i + 1)..n {
                let (a, b) = (&self.cells[i], &self.cells[j]);
                if a.death.is_some_and(|d| d > 0.3) || b.death.is_some_and(|d| d > 0.3) {
                    continue;
                }
                let twins = a.twin == Some(j);
                let rest = (a.r + b.r) * pack * if twins { split } else { 1.0 };
                let d = a.off - b.off;
                let len = d.len();
                if len < rest {
                    let dir = if len > 1e-4 { d * (1.0 / len) } else { jitter(i, j as u32).norm() };
                    let f = dir * ((rest - len) * 70.0);
                    force[i] += f;
                    force[j] -= f;
                }
            }
        }
        let mut dead = Vec::new();
        for (i, c) in self.cells.iter_mut().enumerate() {
            let mut f = force[i] - c.vel * 7.0;
            match c.death {
                Some(d) => {
                    // Shed: it greys, shrinks, and drifts off the cluster.
                    let d = d + dt / DIE_SECS;
                    c.death = Some(d);
                    c.r = c.r1 * (1.0 - smoothstep(0.35, 1.0, d)).sqrt();
                    f += c.off.norm() * 2.5 + v3(0.0, -1.5, 0.0) * smoothstep(0.3, 1.0, d);
                    if d >= 1.0 {
                        dead.push(i);
                    }
                }
                None => f -= c.off * pull,
            }
            c.vel += f * dt;
            c.off += c.vel * dt;
            // Keep it within reach of the shader's bounds.
            let reach = c.off.len() + c.r;
            if reach > BOUND {
                c.off = c.off * ((BOUND - c.r).max(0.0) / c.off.len().max(1e-4));
            }
        }
        for &i in dead.iter().rev() {
            self.cells.remove(i);
            self.eye = match self.eye {
                Some(e) if e == i => None,
                Some(e) if e > i => Some(e - 1),
                e => e,
            };
        }
    }

    /// The eye turns the whole body toward what it watches.
    fn turn_eye(&mut self, dt: f32, watch: V3) {
        let alive = match self.eye {
            Some(e) => self.cells[e].death.is_none(),
            None => false,
        };
        let goal = if alive && !self.is_stone() { 1.0 } else { 0.0 };
        self.eye_open = ease(self.eye_open, goal, -0.8 * dt);
        if !alive || self.is_stone() {
            return;
        }
        if let (Some(e), 0) = (self.eye, self.grain) {
            self.eye_dir = self.cells[e].off.norm();
        }
        let axis = self.eye_dir.cross(watch);
        let s = axis.len();
        if s < 1e-4 {
            return;
        }
        let angle = s.asin().min(0.9 * dt);
        let axis = axis * (1.0 / s);
        self.eye_dir = rotate(self.eye_dir, axis, angle);
        for c in &mut self.cells {
            c.off = rotate(c.off, axis, angle);
            c.vel = rotate(c.vel, axis, angle);
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Smoothing between cells, in body radii: tight while alive, melting as it compacts.
    pub fn join(&self) -> f32 {
        0.08 + 0.5 * self.look().stone
    }

    /// Half extents of the slab (width, height, depth), in body radii.
    pub fn slab_half() -> V3 {
        let s = slab_unit();
        v3(2.0 * s, 4.5 * s, 0.5 * s)
    }

    /// Distance to the cluster, in body radii, for a point `q` relative to its center.
    /// Grain is a plain sphere here; its surface only exists in the shader.
    pub fn sdf_local(&self, q: V3) -> f32 {
        let look = self.look();
        let k = self.join();
        let mut d = 1e3;
        for c in &self.cells {
            d = smin(d, (q - c.off).len() - c.r, k);
        }
        let ball = q.len() - self.grain_radius();
        d + (ball - d) * look.grain.max(look.stone * 0.8)
    }
}

/// Where the slab stands: on the floor, under the body.
pub fn slab_center(pos: V3, r: f32) -> V3 {
    v3(pos.x, crate::floor::fusion::FLOOR_Y + Morula::slab_half().y * r, pos.z)
}

/// Distance to an upright box centered at `c`, turned by `yaw`, with half extents `h`.
pub fn sd_slab(p: V3, c: V3, yaw: f32, h: V3) -> f32 {
    let q = p - c;
    let (s, co) = yaw.sin_cos();
    let l = v3(co * q.x - s * q.z, q.y, s * q.x + co * q.z);
    // its edges are only just softened, enough to catch the light
    let bevel = 0.06 * h.z;
    let o = v3(l.x.abs() - h.x + bevel, l.y.abs() - h.y + bevel, l.z.abs() - h.z + bevel);
    let out = v3(o.x.max(0.0), o.y.max(0.0), o.z.max(0.0)).len();
    out + o.x.max(o.y).max(o.z).min(0.0) - bevel
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(m: &mut Morula, secs: f32) {
        for _ in 0..(secs * 120.0) as usize {
            m.step(1.0 / 120.0, 0.0, v3(0.0, 0.0, 1.0));
        }
    }

    #[test]
    fn it_never_grows_it_only_gets_finer() {
        let mut m = Morula::from_two(v3(1.0, 0.0, 0.0), 0.95, v3(0.0, 1.0, 0.0));
        settle(&mut m, 3.0);
        assert_eq!(m.cells.len(), 4);
        for n in [8, 16] {
            assert!(m.cleave(v3(0.0, 0.0, 1.0)));
            settle(&mut m, 3.0);
            assert_eq!(m.cells.len(), n);
        }
        // the volume of one body, split sixteen ways
        assert!((m.volume() - 1.0).abs() < 1e-3, "{}", m.volume());
        // packed by their own pressure, within the shader's reach
        for c in &m.cells {
            assert!(c.off.len() + c.r <= BOUND + 1e-3);
        }
        assert!(m.sdf_local(V3::ZERO) < 0.0, "the middle is full");
        // past sixteen it becomes grain
        assert!(m.cleave(v3(1.0, 0.0, 0.0)));
        settle(&mut m, 3.0);
        assert_eq!(m.grain, 32);
        assert!(m.drain_events().contains(&Event::BecameGrain));
    }

    #[test]
    fn at_eight_one_becomes_an_eye_and_turns_toward_what_it_watches() {
        let mut m = Morula::from_two(v3(1.0, 0.0, 0.0), 0.95, v3(0.0, 1.0, 0.0));
        settle(&mut m, 3.0);
        assert!(m.eye.is_none());
        m.cleave(v3(0.0, 0.0, 1.0));
        settle(&mut m, 3.0);
        assert!(m.eye.is_some());
        assert!(m.drain_events().contains(&Event::Differentiated));
        let watch = v3(-1.0, 0.0, 0.0);
        for _ in 0..1200 {
            m.step(1.0 / 120.0, 0.0, watch);
        }
        assert!(m.eye_dir.dot(watch) > 0.95, "{:?}", m.eye_dir);
    }

    #[test]
    fn three_pokes_kill_one_cell_and_it_is_shed() {
        let mut m = Morula::from_two(v3(1.0, 0.0, 0.0), 0.95, v3(0.0, 1.0, 0.0));
        settle(&mut m, 3.0);
        let at = m.cells[0].off * 1.5;
        assert!(!m.wound(at));
        assert!(!m.wound(at));
        assert!(m.wound(at));
        assert!(!m.can_cut(), "not while it is dying");
        settle(&mut m, 4.0);
        assert_eq!(m.cells.len(), 3);
        assert!((m.volume() - 0.75).abs() < 1e-3);
        assert!(m.can_cut());
    }

    #[test]
    fn a_held_squeeze_turns_it_to_stone_and_only_holding_it_thaws_it() {
        let mut m = Morula::from_two(v3(1.0, 0.0, 0.0), 0.95, v3(0.0, 1.0, 0.0));
        settle(&mut m, 3.0);
        assert!(!m.thaw());
        for _ in 0..(1.5 * 120.0) as usize {
            m.step(1.0 / 120.0, 0.8, v3(0.0, 0.0, 1.0));
        }
        settle(&mut m, COMPACT_SECS);
        assert_eq!(m.stage, Stage::Stone);
        assert!(!m.cleave(v3(1.0, 0.0, 0.0)), "stone can't be cut");
        assert!(m.thaw());
        settle(&mut m, THAW_SECS + 0.1);
        assert!(m.drain_events().contains(&Event::Thawed));
    }

    #[test]
    fn the_slab_holds_the_volume_of_the_body() {
        let h = Morula::slab_half();
        let v = 8.0 * h.x * h.y * h.z;
        assert!((v - 4.0 / 3.0 * std::f32::consts::PI).abs() < 1e-3);
        assert!((h.y / h.z - 9.0).abs() < 1e-4 && (h.x / h.z - 4.0).abs() < 1e-4);
        assert!(sd_slab(v3(0.0, 0.0, 0.0), V3::ZERO, 0.3, h) < 0.0);
        assert!((sd_slab(v3(0.0, h.y + 0.5, 0.0), V3::ZERO, 0.3, h) - 0.5).abs() < 1e-4);
    }
}
