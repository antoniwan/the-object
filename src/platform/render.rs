//! WebGL2: one full-screen triangle and a raymarching fragment shader.

use crate::camera::{Camera, FOCAL};
use crate::math::V3;
use crate::sim::{World, K_DROP, K_HANDLE, MAX_DROPS, MAX_HANDLES, MAX_RIPPLES};
use crate::division::morula;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlShader, WebGlUniformLocation};

const VERT: &str = r#"#version 300 es
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}"#;

// The shader as built: src/shader.frag without its comments (see build.rs).
const FRAG: &str = include_str!(concat!(env!("OUT_DIR"), "/shader.frag"));

pub struct Renderer {
    gl: GL,
    program: WebGlProgram,
    shaders: [WebGlShader; 2],
    /// The browser can compile shaders in the background (KHR_parallel_shader_compile).
    parallel: bool,
    ready: bool,
    /// Uniform locations, looked up once: asking the driver by name every frame costs more than the draw's setup.
    locs: RefCell<HashMap<&'static str, Option<WebGlUniformLocation>>>,
}

/// GL_COMPLETION_STATUS_KHR.
const COMPLETION_STATUS: u32 = 0x91B1;

fn shader(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, JsValue> {
    let s = gl.create_shader(kind).ok_or("create_shader failed")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    Ok(s)
}

impl Renderer {
    /// Starts compiling. Nothing is checked here, because asking for the result
    /// would freeze the page until the driver is done; `poll` asks later.
    pub fn new(gl: GL) -> Result<Renderer, JsValue> {
        let parallel = gl.get_extension("KHR_parallel_shader_compile").ok().flatten().is_some();
        let program = gl.create_program().ok_or("create_program failed")?;
        let shaders = [shader(&gl, GL::VERTEX_SHADER, VERT)?, shader(&gl, GL::FRAGMENT_SHADER, FRAG)?];
        for s in &shaders {
            gl.attach_shader(&program, s);
        }
        gl.link_program(&program);
        Ok(Renderer { gl, program, shaders, parallel, ready: false, locs: RefCell::new(HashMap::new()) })
    }

    pub fn gl(&self) -> &GL {
        &self.gl
    }

    /// True once the shader can draw. Without the extension the check blocks, so
    /// the caller lets the loader paint at least once before asking.
    pub fn poll(&mut self) -> Result<bool, JsValue> {
        if self.ready {
            return Ok(true);
        }
        let gl = &self.gl;
        if self.parallel && gl.get_program_parameter(&self.program, COMPLETION_STATUS).as_bool() == Some(false) {
            return Ok(false);
        }
        if gl.get_program_parameter(&self.program, GL::LINK_STATUS).as_bool() != Some(true) {
            if gl.is_context_lost() {
                return Ok(false);
            }
            let logs: Vec<String> = self.shaders.iter().filter_map(|s| gl.get_shader_info_log(s)).collect();
            let link = gl.get_program_info_log(&self.program).unwrap_or_default();
            return Err(format!("{} {}", logs.join(" "), link).trim().to_string().into());
        }
        self.ready = true;
        Ok(true)
    }

    fn loc(&self, name: &'static str) -> Option<WebGlUniformLocation> {
        self.locs.borrow_mut().entry(name).or_insert_with(|| self.gl.get_uniform_location(&self.program, name)).clone()
    }

    fn f(&self, name: &'static str, x: f32) {
        let loc = self.loc(name);
        self.gl.uniform1f(loc.as_ref(), x);
    }
    fn i(&self, name: &'static str, x: usize) {
        let loc = self.loc(name);
        self.gl.uniform1i(loc.as_ref(), x as i32);
    }
    fn v2(&self, name: &'static str, x: f32, y: f32) {
        let loc = self.loc(name);
        self.gl.uniform2f(loc.as_ref(), x, y);
    }
    fn v3(&self, name: &'static str, v: V3) {
        let loc = self.loc(name);
        self.gl.uniform3f(loc.as_ref(), v.x, v.y, v.z);
    }
    fn v4(&self, name: &'static str, x: f32, y: f32, z: f32, w: f32) {
        let loc = self.loc(name);
        self.gl.uniform4f(loc.as_ref(), x, y, z, w);
    }
    // The array uploads skip an empty array: WebGL refuses a zero-length one.
    fn fv(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform1fv_with_f32_array(loc.as_ref(), data);
    }
    fn v2v(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform2fv_with_f32_array(loc.as_ref(), data);
    }
    fn v3v(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform3fv_with_f32_array(loc.as_ref(), data);
    }
    fn v4v(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform4fv_with_f32_array(loc.as_ref(), data);
    }

    /// `fine` is how much of the finest light is drawn, 0..1, eased by the page as frames slow (pace.rs).
    pub fn draw(&self, world: &World, cam: &Camera, w: u32, h: u32, fine: f32) {
        let gl = &self.gl;
        gl.viewport(0, 0, w as i32, h as i32);
        gl.use_program(Some(&self.program));

        self.upload_view(world, cam, w, h, fine);
        let body = &world.body;
        let r = body.radius();
        // It swells as the wounds add up, right before it bursts.
        let swell = 1.0 + 0.3 * world.burst.swell();
        self.upload_liquid(world, r, swell);
        self.upload_division(world, r);
        self.v3("uBodyCol", body.color);
        self.f("uAgit", world.agitation);
        self.f("uNight", world.night);
        let g = world.gaze_dir;
        self.v4v("uGaze", &[g.x, g.y, g.z, world.gaze]);
        self.upload_fusion(world);
        self.upload_mere(world);
        self.upload_burst(world);
        self.upload_forms(world, r * swell);
        self.upload_other(world);
        self.upload_wound(world);

        gl.draw_arrays(GL::TRIANGLES, 0, 3);
    }

    /// The canvas, the clock, the camera and its shake.
    fn upload_view(&self, world: &World, cam: &Camera, w: u32, h: u32, fine: f32) {
        self.v2("uRes", w as f32, h as f32);
        self.f("uTime", world.time);
        let b = cam.basis();
        self.v3("uCamPos", b.pos);
        self.v3("uCamRight", b.right);
        self.v3("uCamUp", b.up);
        self.v3("uCamFwd", b.fwd);
        self.f("uFocal", FOCAL);
        let (shock, since) = cam.shock();
        self.v2("uShock", shock, since);
        self.f("uFine", fine);
    }

    /// The body, its tendrils, its ripples and its drops.
    fn upload_liquid(&self, world: &World, r: f32, swell: f32) {
        let body = &world.body;
        self.v4v("uBody", &[body.pos.x, body.pos.y, body.pos.z, r * swell]);
        // gathering after a burst, it grows back inside the pieces (burst.rs `grown`)
        self.v3("uScale", body.scale() * world.burst.grown().max(0.02));
        self.v2("uK", K_HANDLE * r, K_DROP * r);

        let (mut ha, mut hb, mut bulb) = (Vec::new(), Vec::new(), Vec::new());
        for h in world.handles.iter().take(MAX_HANDLES) {
            let (base, neck, bulb_r) = h.radii(body);
            let a = h.anchor(body);
            ha.extend([a.x, a.y, a.z, base]);
            hb.extend([h.tip.x, h.tip.y, h.tip.z, neck]);
            bulb.push(bulb_r);
        }
        self.v4v("uHA", &ha);
        self.v4v("uHB", &hb);
        self.fv("uHBulb", &bulb);
        self.i("uNH", bulb.len());

        let (mut rip, mut age) = (Vec::new(), Vec::new());
        for rp in world.ripples.iter().take(MAX_RIPPLES) {
            rip.extend([rp.dir.x, rp.dir.y, rp.dir.z, rp.amp]);
            age.push(rp.age);
        }
        self.v4v("uRip", &rip);
        self.fv("uRipAge", &age);
        self.i("uNR", age.len());

        let (mut drops, mut colors) = (Vec::new(), Vec::new());
        for d in world.drops.iter().take(MAX_DROPS) {
            drops.extend([d.pos.x, d.pos.y, d.pos.z, d.r]);
            colors.extend([d.color.x, d.color.y, d.color.z]);
        }
        self.v4v("uDrop", &drops);
        self.v3v("uDropCol", &colors);
        self.i("uND", drops.len() / 4);
    }

    /// Slicing: see meiosis.rs. The shader rebuilds the cells from these.
    fn upload_division(&self, world: &World, r: f32) {
        let div = &world.division;
        let bl = div.blend();
        // past two, the newest cut's plane: only the seam's glow reads it then
        let axis = div.morula.as_ref().map_or(div.axis, |m| m.axis);
        self.v4("uCut", axis.x, axis.y, axis.z, bl.split);
        let flash = div.morula.as_ref().map_or(bl.flash, |m| m.look().flash);
        self.v3("uDiv", crate::math::v3(flash, bl.furrow, bl.cell));
        self.v2("uNerve", div.nerve.fear, div.nerve.power);
        self.upload_morula(world, r);
    }

    /// Past two cells: see morula.rs.
    fn upload_morula(&self, world: &World, r: f32) {
        let Some(m) = &world.division.morula else {
            self.i("uNC", 0);
            return;
        };
        let look = m.look();
        let (mut cells, mut fate) = (Vec::new(), Vec::new());
        for c in m.cells.iter().take(morula::MAX_CELLS) {
            cells.extend([c.off.x * r, c.off.y * r, c.off.z * r, c.r * r]);
            fate.extend([c.wounds as f32 / morula::WOUNDS_TO_DIE as f32, c.death.unwrap_or(0.0)]);
        }
        self.v4v("uCells", &cells);
        self.v2v("uCellF", &fate);
        self.i("uNC", (cells.len() / 4).max(1));
        self.v4v("uMor", &[look.grain, m.grain as f32, m.prev_grain() as f32, look.fine]);
        self.f("uGrainR", m.grain_radius());
        self.v4v("uStone", &[look.stone, look.slab, look.thaw, m.slab_yaw]);
        let c = morula::slab_center(world.body.pos, r);
        self.v4v("uSlab", &[c.x, c.y, c.z, m.join() * r]);
        // The eye sits on its cell until the cells have become grain, then on the grain body,
        // moving across as the shader blends one into the other.
        let e = m.eye_dir;
        let ec = m.eye.map(|i| {
            let k = crate::math::smoothstep(0.3, 0.7, look.grain);
            let c = &m.cells[i];
            let (off, cr) = (c.off * (1.0 - k), c.r + (m.grain_radius() - c.r) * k);
            [off.x * r, off.y * r, off.z * r, cr * r]
        });
        self.v4v("uEye", &[e.x, e.y, e.z, if ec.is_some() { m.eye_open } else { 0.0 }]);
        let ec = ec.unwrap_or([0.0, 0.0, 0.0, r]);
        self.v4v("uEyeC", &ec);
    }

    /// Slamming: see fusion.rs.
    fn upload_fusion(&self, world: &World) {
        let fu = &world.fusion;
        let radius = fu.blend().lake * crate::floor::fusion::LAKE_RADIUS;
        self.v4("uLake", fu.impact.0, fu.impact.1, radius, fu.since_impact);
        let mut lr = Vec::new();
        for r in fu.ripples.iter().take(crate::floor::fusion::MAX_LAKE_RIPPLES) {
            lr.extend([r.x, r.z, r.age, r.amp]);
        }
        self.v4v("uLakeRip", &lr);
        self.i("uNLR", lr.len() / 4);
    }

    /// What the lake became, and what the object came back out as: see mere.rs.
    fn upload_mere(&self, world: &World) {
        let m = &world.fusion.mere;
        self.v4("uMere", m.still, m.deep, m.ice, 0.0);
        self.v4("uIce", m.ice_at.0, m.ice_at.1, m.ice_reach, 0.0);
        let mut cr = Vec::new();
        for c in m.cracks.iter().take(crate::floor::mere::MAX_CRACKS) {
            cr.extend([c.x, c.z, c.age, c.seed]);
        }
        self.v4v("uCrack", &cr);
        self.i("uNCrack", cr.len() / 4);
        let (jx, jz, column, drop) = match m.fountain {
            Some(f) => {
                let (column, drop) = f.pose();
                (f.x, f.z, column, drop)
            }
            None => (0.0, 0.0, 0.0, -1.0),
        };
        self.v4("uJet", jx, jz, column, drop);
        let (quicksilver, ink) = m.coat_mix();
        self.v2("uCoat", quicksilver, ink);
    }

    /// Bursting: see burst.rs.
    fn upload_burst(&self, world: &World) {
        let bu = &world.burst;
        self.v4("uBurst", 1.0 - bu.grown(), bu.flash(), bu.wounds / crate::burst::FATAL_WOUNDS, bu.spread());
        let (mut ta, mut tb) = (Vec::new(), Vec::new());
        // landscape.rs: planted pieces grow, and the weather rounds every edge
        let round = bu.land.rounding();
        // gathered home, they are drops inside the body growing back, whatever their size (a drop's
        // size is the body's); their shadows go as its own comes (`grown`)
        let shown = 1.0 - bu.grown();
        for (i, s) in bu.shards.iter().enumerate() {
            let (pos, morph) = bu.shard_pose(i);
            let size = bu.shard_size(i);
            ta.extend([pos.x, pos.y, pos.z, if morph > 0.0 { size } else { size * shown }]);
            tb.extend([s.yaw, morph, s.wobble, round]);
        }
        self.v4v("uTri", &ta);
        self.v4v("uTriB", &tb);
        // river.rs: the band it runs in, and the columns standing out of it
        let rv = &bu.river;
        self.v4v("uRiver", &[rv.at.0, rv.at.1, rv.angle, if rv.active { rv.reach() } else { 0.0 }]);
        self.v4v("uRiver2", &[rv.amount, rv.flow, rv.beat(), rv.tallest()]);
        let pil: Vec<f32> = (0..crate::burst::river::SLOTS).map(|i| rv.height(i)).collect();
        self.v4v("uPil", &pil);
        let rows: Vec<f32> = (0..crate::burst::river::ROWS).map(|k| rv.rows_to_standing(k)).collect();
        self.v4v("uPilRow", &rows);
        let sp: Vec<f32> = rv.splashes.iter().flat_map(|&(x, z, age)| [x, z, age, 1.0]).collect();
        self.v4v("uSplash", &sp);
        self.i("uNSp", sp.len() / 4);
    }

    /// The other forms: see forms/. `radius` is the body's, swollen.
    fn upload_forms(&self, world: &World, radius: f32) {
        let fo = &world.forms;
        let bl = &fo.bloom;
        self.v4v("uBloom", &[bl.open(), bl.angle, 0.0, 0.0]);
        let mut buds = Vec::new();
        for b in bl.buds.iter().take(crate::forms::bloom::PETALS) {
            buds.extend([b.dir.x, b.dir.y, b.dir.z, bl.bud_size(b)]);
        }
        // The shadow of a bud only comes while there is room for one more.
        if let Some(b) = bl.near.filter(|_| buds.len() < 4 * crate::forms::bloom::PETALS) {
            buds.extend([b.dir.x, b.dir.y, b.dir.z, bl.near_size()]);
        }
        self.v4v("uBud", &buds);
        self.i("uNB", buds.len() / 4);
        let fi = &fo.filament;
        let ax = fi.axis;
        self.v4v("uFil", &[ax.x, ax.y, ax.z, fi.half_len()]);
        self.v4v("uFil2", &[fi.radius(radius), fi.morph(), fi.ringing(), fi.harmonic as f32]);
        self.v3("uFilSwing", fi.swing);
        let co = &fo.cosmos;
        self.v4v("uCosmos", &[co.amount(), co.spin, co.flare, co.sense]);
        let fd = &fo.fold;
        self.v4v("uFold", &[fd.amount(), fd.clock, fd.flash, fd.backward()]);
        self.v4v("uFold2", &[fd.mirrors_was as f32, fd.mirrors as f32, crate::math::smoothstep(0.0, 1.0, fd.mirror_t), 0.0]);
        let pa = &fo.pandora;
        self.v4v("uBox", &[pa.boxness(), pa.lid(), pa.rattle, pa.yaw]);
        let du = &fo.dust;
        let (cloud, fall) = du.cloud();
        self.v4v("uDust", &[du.settle(), du.crumble_amount(), cloud, fall]);
        let clumps: Vec<f32> = du.clumps.iter().flat_map(|c| [c.x, c.z, c.radius(), c.amount]).collect();
        self.v4v("uClump", &clumps);
        let (hx, hz, hr, hh) = du.heap();
        self.v4v("uHeap", &[hx, hz, hr, hh]);
        self.v4v("uHeapB", &[du.firm(), du.thud, du.press, du.growth.shown]);
    }

    /// The other one: see other.rs.
    fn upload_other(&self, world: &World) {
        let ot = &world.other;
        self.v4v("uOther", &[ot.pos.x, ot.pos.y, ot.pos.z, ot.size(world.body.pos)]);
        self.v4v("uOther2", &[ot.yaw, ot.tilt, ot.edge, ot.ring]);
        self.v4v("uOther3", &[ot.kept.x, ot.kept.y, ot.kept.z, ot.fire]);
        self.f("uOtherIn", ot.inside());
    }

    /// An old hurt: see wound.rs.
    fn upload_wound(&self, world: &World) {
        let wo = &world.wound;
        let shown = wo.shown;
        self.v4v("uScar", &[wo.dir.x, wo.dir.y, wo.dir.z, wo.scar * shown]);
        self.v4v("uWound", &[wo.open() * shown, wo.fill, wo.drip, 0.0]);
        self.v3("uWoundCol", wo.color);
        let mut spiked = 0usize;
        for (i, d) in world.drops.iter().take(MAX_DROPS).enumerate() {
            if wo.is_caltrop(d.id) {
                spiked |= 1 << i;
            }
        }
        self.i("uCaltrop", spiked);
    }
}
