//! WebGL2: one full-screen triangle and a raymarching fragment shader.

use crate::camera::{Camera, FOCAL};
use crate::math::V3;
use crate::sim::{World, K_DROP, K_HANDLE, MAX_DROPS, MAX_HANDLES, MAX_RIPPLES};
use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlShader};

const VERT: &str = r#"#version 300 es
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}"#;

const FRAG: &str = include_str!("shader.frag");

pub struct Renderer {
    gl: GL,
    program: WebGlProgram,
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, JsValue> {
    let s = gl.create_shader(kind).ok_or("create_shader failed")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool() == Some(true) {
        Ok(s)
    } else {
        Err(gl.get_shader_info_log(&s).unwrap_or_default().into())
    }
}

impl Renderer {
    pub fn new(gl: GL) -> Result<Renderer, JsValue> {
        let program = gl.create_program().ok_or("create_program failed")?;
        gl.attach_shader(&program, &compile(&gl, GL::VERTEX_SHADER, VERT)?);
        gl.attach_shader(&program, &compile(&gl, GL::FRAGMENT_SHADER, FRAG)?);
        gl.link_program(&program);
        if gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool() != Some(true) {
            return Err(gl.get_program_info_log(&program).unwrap_or_default().into());
        }
        Ok(Renderer { gl, program })
    }

    fn v3(&self, name: &str, v: V3) {
        let loc = self.gl.get_uniform_location(&self.program, name);
        self.gl.uniform3f(loc.as_ref(), v.x, v.y, v.z);
    }
    fn f(&self, name: &str, x: f32) {
        let loc = self.gl.get_uniform_location(&self.program, name);
        self.gl.uniform1f(loc.as_ref(), x);
    }
    fn i(&self, name: &str, x: usize) {
        let loc = self.gl.get_uniform_location(&self.program, name);
        self.gl.uniform1i(loc.as_ref(), x as i32);
    }
    fn fv(&self, name: &str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.gl.get_uniform_location(&self.program, name);
        self.gl.uniform1fv_with_f32_array(loc.as_ref(), data);
    }
    fn v4v(&self, name: &str, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        let loc = self.gl.get_uniform_location(&self.program, name);
        self.gl.uniform4fv_with_f32_array(loc.as_ref(), data);
    }

    pub fn draw(&self, world: &World, cam: &Camera, w: u32, h: u32) {
        let gl = &self.gl;
        gl.viewport(0, 0, w as i32, h as i32);
        gl.use_program(Some(&self.program));

        let loc = gl.get_uniform_location(&self.program, "uRes");
        gl.uniform2f(loc.as_ref(), w as f32, h as f32);
        self.f("uTime", world.time);
        let b = cam.basis();
        self.v3("uCamPos", b.pos);
        self.v3("uCamRight", b.right);
        self.v3("uCamUp", b.up);
        self.v3("uCamFwd", b.fwd);
        self.f("uFocal", FOCAL);

        let body = &world.body;
        let r = body.radius();
        self.v4v("uBody", &[body.pos.x, body.pos.y, body.pos.z, r]);
        self.v3("uScale", body.scale());
        let loc = gl.get_uniform_location(&self.program, "uK");
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
            let loc = gl.get_uniform_location(&self.program, "uDropCol");
            gl.uniform3fv_with_f32_array(loc.as_ref(), &colors);
        }
        self.i("uND", drops.len() / 4);

        // Slicing: see meiosis.rs. The shader rebuilds the cells from these.
        let div = &world.division;
        let bl = div.blend();
        let loc = gl.get_uniform_location(&self.program, "uCut");
        gl.uniform4f(loc.as_ref(), div.axis.x, div.axis.y, div.axis.z, bl.split);
        let loc = gl.get_uniform_location(&self.program, "uDiv");
        gl.uniform3f(loc.as_ref(), bl.flash, bl.furrow, bl.cell);
        self.v3("uBodyCol", body.color);
        self.f("uAgit", world.agitation);
        self.f("uNight", world.night);
        let g = world.gaze_dir;
        self.v4v("uGaze", &[g.x, g.y, g.z, world.gaze]);

        // Slamming: see fusion.rs.
        let fu = &world.fusion;
        let loc = gl.get_uniform_location(&self.program, "uLake");
        let radius = fu.blend().lake * crate::fusion::LAKE_RADIUS;
        gl.uniform4f(loc.as_ref(), fu.impact.0, fu.impact.1, radius, fu.since_impact);
        let mut lr = Vec::new();
        for r in fu.ripples.iter().take(crate::fusion::MAX_LAKE_RIPPLES) {
            lr.extend([r.x, r.z, r.age, r.amp]);
        }
        self.v4v("uLakeRip", &lr);
        self.i("uNLR", lr.len() / 4);

        gl.draw_arrays(GL::TRIANGLES, 0, 3);
    }
}
