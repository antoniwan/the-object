//! WebGL2: one full-screen triangle and a raymarching fragment shader.

use crate::camera::{Camera, FOCAL};
use crate::math::V3;
use crate::sim::{World, K_DROP, K_HANDLE, MAX_DROPS, MAX_HANDLES, MAX_RIPPLES};
use crate::morula;
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

    fn v3(&self, name: &'static str, v: V3) {
        let loc = self.loc(name);
        self.gl.uniform3f(loc.as_ref(), v.x, v.y, v.z);
    }
    fn f(&self, name: &'static str, x: f32) {
        let loc = self.loc(name);
        self.gl.uniform1f(loc.as_ref(), x);
    }
    fn i(&self, name: &'static str, x: usize) {
        let loc = self.loc(name);
        self.gl.uniform1i(loc.as_ref(), x as i32);
    }
    fn fv(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform1fv_with_f32_array(loc.as_ref(), data);
    }
    fn v4v(&self, name: &'static str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.loc(name);
        self.gl.uniform4fv_with_f32_array(loc.as_ref(), data);
    }

    /// Past two cells: see morula.rs.
    fn draw_morula(&self, world: &World, r: f32) {
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
        if !fate.is_empty() {
            let loc = self.loc("uCellF");
            self.gl.uniform2fv_with_f32_array(loc.as_ref(), &fate);
        }
        self.i("uNC", (cells.len() / 4).max(1));
        self.v4v("uMor", &[look.grain, m.grain as f32, m.prev_grain() as f32, look.fine]);
        self.f("uGrainR", m.grain_radius());
        self.v4v("uStone", &[look.stone, look.slab, look.thaw, m.slab_yaw]);
        let c = morula::slab_center(world.body.pos, r);
        self.v4v("uSlab", &[c.x, c.y, c.z, m.join() * r]);
        // The eye sits on its cell until the cells have become grain, then on the grain body.
        let e = m.eye_dir;
        let ec = match m.eye {
            Some(i) if look.grain < 0.5 => Some([m.cells[i].off.x * r, m.cells[i].off.y * r, m.cells[i].off.z * r, m.cells[i].r * r]),
            Some(_) => Some([0.0, 0.0, 0.0, m.grain_radius() * r]),
            None => None,
        };
        self.v4v("uEye", &[e.x, e.y, e.z, if ec.is_some() { m.eye_open } else { 0.0 }]);
        let ec = ec.unwrap_or([0.0, 0.0, 0.0, r]);
        self.v4v("uEyeC", &ec);
    }

    /// `quality` is the page's adaptive resolution, 0.4..1; below 0.75 the finest light is left out.
    pub fn draw(&self, world: &World, cam: &Camera, w: u32, h: u32, quality: f32) {
        let gl = &self.gl;
        gl.viewport(0, 0, w as i32, h as i32);
        gl.use_program(Some(&self.program));

        let loc = self.loc("uRes");
        gl.uniform2f(loc.as_ref(), w as f32, h as f32);
        self.f("uTime", world.time);
        let b = cam.basis();
        self.v3("uCamPos", b.pos);
        self.v3("uCamRight", b.right);
        self.v3("uCamUp", b.up);
        self.v3("uCamFwd", b.fwd);
        self.f("uFocal", FOCAL);
        let (shock, since) = cam.shock();
        let loc = self.loc("uShock");
        gl.uniform2f(loc.as_ref(), shock, since);
        self.f("uFine", if quality < 0.75 { 0.0 } else { 1.0 });

        let body = &world.body;
        let r = body.radius();
        // It swells as the wounds add up, right before it bursts.
        let swell = 1.0 + 0.3 * world.burst.swell();
        self.v4v("uBody", &[body.pos.x, body.pos.y, body.pos.z, r * swell]);
        self.v3("uScale", body.scale());
        let loc = self.loc("uK");
        gl.uniform2f(loc.as_ref(), K_HANDLE * r, K_DROP * r);

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
        if !colors.is_empty() {
            let loc = self.loc("uDropCol");
            gl.uniform3fv_with_f32_array(loc.as_ref(), &colors);
        }
        self.i("uND", drops.len() / 4);

        // Slicing: see meiosis.rs. The shader rebuilds the cells from these.
        let div = &world.division;
        let bl = div.blend();
        let loc = self.loc("uCut");
        gl.uniform4f(loc.as_ref(), div.axis.x, div.axis.y, div.axis.z, bl.split);
        let loc = self.loc("uDiv");
        let flash = div.morula.as_ref().map_or(bl.flash, |m| m.look().flash);
        gl.uniform3f(loc.as_ref(), flash, bl.furrow, bl.cell);
        self.draw_morula(world, r);
        self.v3("uBodyCol", body.color);
        self.f("uAgit", world.agitation);
        self.f("uNight", world.night);
        let g = world.gaze_dir;
        self.v4v("uGaze", &[g.x, g.y, g.z, world.gaze]);

        // Slamming: see fusion.rs.
        let fu = &world.fusion;
        let loc = self.loc("uLake");
        let radius = fu.blend().lake * crate::fusion::LAKE_RADIUS;
        gl.uniform4f(loc.as_ref(), fu.impact.0, fu.impact.1, radius, fu.since_impact);
        let mut lr = Vec::new();
        for r in fu.ripples.iter().take(crate::fusion::MAX_LAKE_RIPPLES) {
            lr.extend([r.x, r.z, r.age, r.amp]);
        }
        self.v4v("uLakeRip", &lr);
        self.i("uNLR", lr.len() / 4);

        // What the lake became, and what the object came back out as: see mere.rs.
        let m = &fu.mere;
        let loc = self.loc("uMere");
        gl.uniform4f(loc.as_ref(), m.still, m.deep, m.ice, 0.0);
        let loc = self.loc("uIce");
        gl.uniform4f(loc.as_ref(), m.ice_at.0, m.ice_at.1, m.ice_reach, 0.0);
        let mut cr = Vec::new();
        for c in m.cracks.iter().take(crate::mere::MAX_CRACKS) {
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
        let loc = self.loc("uJet");
        gl.uniform4f(loc.as_ref(), jx, jz, column, drop);
        let (quicksilver, ink) = m.coat_mix();
        let loc = self.loc("uCoat");
        gl.uniform2f(loc.as_ref(), quicksilver, ink);

        // Bursting: see burst.rs.
        let bu = &world.burst;
        let loc = self.loc("uBurst");
        let gone = if bu.body_gone() { 1.0 } else { 0.0 };
        gl.uniform4f(loc.as_ref(), gone, bu.flash(), bu.wounds / crate::burst::FATAL_WOUNDS, bu.spread());
        let (mut ta, mut tb) = (Vec::new(), Vec::new());
        // landscape.rs: planted pieces grow, and the weather rounds every edge
        let round = bu.land.rounding();
        for (i, s) in bu.shards.iter().enumerate() {
            let (pos, morph) = bu.shard_pose(i);
            ta.extend([pos.x, pos.y, pos.z, bu.shard_size(i)]);
            tb.extend([s.yaw, morph, s.wobble, round]);
        }
        self.v4v("uTri", &ta);
        self.v4v("uTriB", &tb);
        // river.rs: the band it runs in, and the columns standing out of it
        let rv = &bu.river;
        self.v4v("uRiver", &[rv.at.0, rv.at.1, rv.angle, if rv.active { rv.reach() } else { 0.0 }]);
        self.v4v("uRiver2", &[rv.amount, rv.flow, rv.beat(), rv.tallest()]);
        let pil: Vec<f32> = (0..crate::river::SLOTS).map(|i| rv.height(i)).collect();
        self.v4v("uPil", &pil);
        let rows: Vec<f32> = (0..crate::river::ROWS).map(|k| rv.rows_to_standing(k)).collect();
        self.v4v("uPilRow", &rows);
        let sp: Vec<f32> = rv.splashes.iter().flat_map(|&(x, z, age)| [x, z, age, 1.0]).collect();
        self.v4v("uSplash", &sp);
        self.i("uNSp", sp.len() / 4);

        // The other forms: see forms/.
        let fo = &world.forms;
        let bl = &fo.bloom;
        self.v4v("uBloom", &[bl.open(), bl.angle, 0.0, 0.0]);
        let mut buds = Vec::new();
        for b in bl.buds.iter().take(crate::forms::bloom::PETALS) {
            buds.extend([b.dir.x, b.dir.y, b.dir.z, bl.bud_size(b)]);
        }
        self.v4v("uBud", &buds);
        self.i("uNB", buds.len() / 4);
        let fi = &fo.filament;
        let ax = fi.axis;
        self.v4v("uFil", &[ax.x, ax.y, ax.z, fi.half_len()]);
        self.v4v("uFil2", &[fi.radius(r * swell), fi.morph(), fi.ringing(), fi.harmonic as f32]);
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

        // The other one: see other.rs.
        let ot = &world.other;
        self.v4v("uOther", &[ot.pos.x, ot.pos.y, ot.pos.z, ot.size(body.pos)]);
        self.v4v("uOther2", &[ot.yaw, ot.tilt, ot.edge, ot.ring]);
        self.v4v("uOther3", &[ot.kept.x, ot.kept.y, ot.kept.z, ot.fire]);
        self.f("uOtherIn", ot.inside());

        // An old hurt: see wound.rs.
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

        gl.draw_arrays(GL::TRIANGLES, 0, 3);
    }
}
