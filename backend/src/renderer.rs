use crate::math::{Mat4, Vec3, Vec4};
use crate::model::{Model, Texture};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ─── ASCII character ramps ────────────────────────────────────────────────────

const CHARSET_STANDARD: &[u8] =
    b" .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
const CHARSET_SIMPLE: &[u8] = b" .-:=+*#%@";
const CHARSET_BLOCKS: &[u8] = b" .:+#%@X";
const CHARSET_DOTS: &[u8] = b" .:oO0@";
const CHARSET_BINARY: &[u8] = b" 01";
const CHARSET_MATRIX: &[u8] = b" .,:;|()[]{}01234567890abcdefABCDEF";

fn charset_bytes(name: &str) -> &'static [u8] {
    match name {
        "simple" => CHARSET_SIMPLE,
        "blocks" => CHARSET_BLOCKS,
        "dots" => CHARSET_DOTS,
        "binary" => CHARSET_BINARY,
        "matrix" => CHARSET_MATRIX,
        _ => CHARSET_STANDARD,
    }
}

fn intensity_to_char(intensity: f32, charset: &[u8]) -> char {
    let idx = (intensity.clamp(0.0, 1.0) * (charset.len() - 1) as f32) as usize;
    charset[idx] as char
}

// ─── Render params ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export_to = "RenderParams.ts")]
pub struct RenderParams {
    pub width: usize,
    pub height: usize,
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    pub auto_rotate: bool,
    pub rotate_speed_x: f32,
    pub rotate_speed_y: f32,
    pub rotate_speed_z: f32,
    pub zoom: f32,
    pub charset: String,
    pub shading: String, // "normal" | "depth" | "flat" | "wireframe" | "normals"
    pub light_x: f32,
    pub light_y: f32,
    pub light_z: f32,
    pub ambient: f32,
    pub invert: bool,
    pub color_mode: String, // "green" | "white" | "amber" | "blue" | "rainbow"
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            width: 120,
            height: 50,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
            auto_rotate: true,
            rotate_speed_x: 0.0,
            rotate_speed_y: 1.0,
            rotate_speed_z: 0.0,
            zoom: 1.0,
            charset: "standard".into(),
            shading: "normal".into(),
            light_x: 0.5,
            light_y: 1.0,
            light_z: 0.8,
            ambient: 0.15,
            invert: false,
            color_mode: "green".into(),
        }
    }
}

// ─── Framebuffer ─────────────────────────────────────────────────────────────

struct Framebuffer {
    width: usize,
    height: usize,
    z_buf: Vec<f32>,
    char_buf: Vec<char>,
    color_buf: Vec<[u8; 3]>, // RGB per cell
}

impl Framebuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            z_buf: vec![f32::INFINITY; width * height],
            char_buf: vec![' '; width * height],
            color_buf: vec![[0, 0, 0]; width * height],
        }
    }

    fn set(&mut self, x: usize, y: usize, z: f32, c: char, color: [u8; 3]) {
        let idx = y * self.width + x;
        if z < self.z_buf[idx] {
            self.z_buf[idx] = z;
            self.char_buf[idx] = c;
            self.color_buf[idx] = color;
        }
    }

    fn to_string(&self) -> String {
        let mut s = String::with_capacity((self.width + 1) * self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                s.push(self.char_buf[y * self.width + x]);
            }
            s.push('\n');
        }
        s
    }

    /// Encodes every cell color as a flat hex string — 6 ASCII chars per cell, no separator.
    fn colors_hex(&self) -> String {
        let mut s = String::with_capacity(self.color_buf.len() * 6);
        for [r, g, b] in &self.color_buf {
            use std::fmt::Write;
            // INVARIANT: writing fixed-width hex into a String never fails.
            write!(s, "{:02x}{:02x}{:02x}", r, g, b).expect("write to String is infallible");
        }
        s
    }
}

// ─── Rasterizer helpers ───────────────────────────────────────────────────────

fn edge_function(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

fn rasterize_triangle(
    fb: &mut Framebuffer,
    // Screen-space positions (x, y, z_ndc)
    s0: (f32, f32, f32),
    s1: (f32, f32, f32),
    s2: (f32, f32, f32),
    // Intensities at each vertex for smooth shading
    i0: f32,
    i1: f32,
    i2: f32,
    uv0: (f32, f32),
    uv1: (f32, f32),
    uv2: (f32, f32),
    texture: Option<&Texture>,
    texture_lit: bool,
    charset: &[u8],
    invert: bool,
) {
    let w = fb.width as f32;
    let h = fb.height as f32;

    let min_x = s0.0.min(s1.0).min(s2.0).max(0.0) as usize;
    let max_x = (s0.0.max(s1.0).max(s2.0).min(w - 1.0) as usize).min(fb.width - 1);
    let min_y = s0.1.min(s1.1).min(s2.1).max(0.0) as usize;
    let max_y = (s0.1.max(s1.1).max(s2.1).min(h - 1.0) as usize).min(fb.height - 1);

    let area = edge_function((s0.0, s0.1), (s1.0, s1.1), (s2.0, s2.1));
    if area.abs() < 1e-6 { return; }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let w0 = edge_function((s1.0, s1.1), (s2.0, s2.1), (px, py));
            let w1 = edge_function((s2.0, s2.1), (s0.0, s0.1), (px, py));
            let w2 = edge_function((s0.0, s0.1), (s1.0, s1.1), (px, py));

            // Check winding (support both CW and CCW)
            let inside = if area > 0.0 {
                w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
            } else {
                w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
            };

            if !inside { continue; }

            let b0 = w0 / area;
            let b1 = w1 / area;
            let b2 = w2 / area;

            let z = b0 * s0.2 + b1 * s1.2 + b2 * s2.2;

            let u_interp = b0 * uv0.0 + b1 * uv1.0 + b2 * uv2.0;
            let v_interp = b0 * uv0.1 + b1 * uv1.1 + b2 * uv2.1;

            let (tex_color, tex_lum) = if let Some(tex) = texture {
                let rgb = tex.sample_rgb(u_interp, v_interp);
                let lum = 0.2126 * rgb[0] as f32 / 255.0
                        + 0.7152 * rgb[1] as f32 / 255.0
                        + 0.0722 * rgb[2] as f32 / 255.0;
                (Some(rgb), Some(lum))
            } else {
                (None, None)
            };

            let base = b0 * i0 + b1 * i1 + b2 * i2;
            let raw = match (tex_lum, texture_lit) {
                (Some(lum), false) => lum,
                (Some(lum), true)  => lum * base,
                _                  => base,
            };
            let final_intensity = if invert { 1.0 - raw } else { raw };
            let c = intensity_to_char(final_intensity, charset);
            let color = tex_color.unwrap_or([0, 0, 0]);
            fb.set(x, y, z, c, color);
        }
    }
}

fn draw_line(
    fb: &mut Framebuffer,
    x0: f32, y0: f32, z0: f32,
    x1: f32, y1: f32, z1: f32,
    c: char,
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let steps = dx.max(dy) as usize;
    if steps == 0 { return; }

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = (x0 + (x1 - x0) * t) as usize;
        let y = (y0 + (y1 - y0) * t) as usize;
        let z = z0 + (z1 - z0) * t;
        if x < fb.width && y < fb.height {
            fb.set(x, y, z, c, [0, 0, 0]);
        }
    }
}

// ─── Main render function ─────────────────────────────────────────────────────

pub fn render_frame(model: &Model, params: &RenderParams) -> (String, Option<String>) {
    let w = params.width.max(10).min(2400);
    let h = params.height.max(5).min(1200);

    let charset = charset_bytes(&params.charset);
    let mut fb = Framebuffer::new(w, h);

    // Aspect ratio: terminal chars are ~2x taller than wide
    let char_aspect = 0.5_f32;
    let screen_aspect = (w as f32 * char_aspect) / h as f32;

    // Build MVP matrix
    let model_mat = Mat4::rotation_x(params.rot_x)
        * Mat4::rotation_y(params.rot_y)
        * Mat4::rotation_z(params.rot_z)
        * Mat4::scale(params.zoom, params.zoom, params.zoom);

    let camera_dist = 3.5;
    let view_mat = Mat4::look_at(
        Vec3::new(0.0, 0.0, camera_dist),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let proj_mat = Mat4::perspective(
        std::f32::consts::FRAC_PI_4, // 45 degrees FOV
        screen_aspect,
        0.1,
        100.0,
    );

    let mv = view_mat * model_mat;
    let mvp = proj_mat * mv;

    // Normal matrix (upper-left 3x3 of MV, transposed inverse — for uniform scale, MV is fine)
    let light_dir = Vec3::new(params.light_x, params.light_y, params.light_z).normalize();

    let is_wireframe = params.shading == "wireframe";
    let mut any_texture = false;

    for mesh in &model.meshes {
        let verts = &mesh.vertices;
        let norms = &mesh.normals;
        let uvs = &mesh.uvs;

        let use_texture = matches!(params.shading.as_str(), "texture" | "texture_lit");
        let texture_lit_mode = params.shading == "texture_lit";
        let texture: Option<&Texture> = if use_texture {
            mesh.texture_index.and_then(|i| model.textures.get(i))
        } else {
            None
        };

        if texture.is_some() {
            any_texture = true;
        }

        for tri_idx in (0..verts.len().saturating_sub(2)).step_by(3) {
            let v0 = verts[tri_idx];
            let v1 = verts[tri_idx + 1];
            let v2 = verts[tri_idx + 2];

            // Transform to clip space
            let c0 = mvp.mul_vec4(Vec4::from_vec3(v0, 1.0));
            let c1 = mvp.mul_vec4(Vec4::from_vec3(v1, 1.0));
            let c2 = mvp.mul_vec4(Vec4::from_vec3(v2, 1.0));

            // Simple near/far clip — skip triangles fully behind camera
            if c0.w <= 0.01 || c1.w <= 0.01 || c2.w <= 0.01 { continue; }

            // Perspective divide → NDC
            let n0 = c0.perspective_divide();
            let n1 = c1.perspective_divide();
            let n2 = c2.perspective_divide();

            // Frustum cull — skip if any vertex is outside NDC cube
            let out_of_bounds = |n: Vec3| {
                n.x < -1.1 || n.x > 1.1 || n.y < -1.1 || n.y > 1.1
            };
            if out_of_bounds(n0) && out_of_bounds(n1) && out_of_bounds(n2) { continue; }

            // NDC → screen space
            let to_screen = |n: Vec3| -> (f32, f32, f32) {
                let sx = (n.x + 1.0) * 0.5 * (w as f32 - 1.0);
                let sy = (1.0 - (n.y + 1.0) * 0.5) * (h as f32 - 1.0);
                (sx, sy, n.z)
            };

            let s0 = to_screen(n0);
            let s1 = to_screen(n1);
            let s2 = to_screen(n2);

            if is_wireframe {
                let wc = '#';
                draw_line(&mut fb, s0.0, s0.1, s0.2, s1.0, s1.1, s1.2, wc);
                draw_line(&mut fb, s1.0, s1.1, s1.2, s2.0, s2.1, s2.2, wc);
                draw_line(&mut fb, s2.0, s2.1, s2.2, s0.0, s0.1, s0.2, wc);
                continue;
            }

            // Compute vertex intensities
            let compute_intensity = |vert_idx: usize| -> f32 {
                match params.shading.as_str() {
                    "depth" => {
                        let z_ndc = mvp.mul_vec4(Vec4::from_vec3(verts[vert_idx], 1.0))
                            .perspective_divide().z;
                        // Map z from [-1,1] to [1,0] (closer = brighter)
                        (1.0 - z_ndc) * 0.5
                    }
                    "flat" => {
                        let e1 = verts[tri_idx + 1] - verts[tri_idx];
                        let e2 = verts[tri_idx + 2] - verts[tri_idx];
                        let face_normal_world = model_mat.mul_vec3_dir(e1.cross(e2).normalize());
                        let diff = face_normal_world.dot(light_dir).max(0.0);
                        diff * (1.0 - params.ambient) + params.ambient
                    }
                    "normals" => {
                        // Visualize normals as rainbow-ish intensity using the dominant axis
                        let n = mv.mul_vec3_dir(norms[vert_idx]).normalize();
                        (n.x * 0.5 + 0.5) * 0.33
                            + (n.y * 0.5 + 0.5) * 0.33
                            + (n.z * 0.5 + 0.5) * 0.34
                    }
                    _ => {
                        // "normal" — Phong-like shading
                        let n = mv.mul_vec3_dir(norms[vert_idx]).normalize();
                        let diff = n.dot(light_dir).max(0.0);
                        // Specular: quick Blinn-Phong
                        let view_dir = Vec3::new(0.0, 0.0, 1.0);
                        let half = (light_dir + view_dir).normalize();
                        let spec = n.dot(half).max(0.0).powi(32) * 0.3;
                        (diff * (1.0 - params.ambient) + params.ambient + spec).min(1.0)
                    }
                }
            };

            let i0 = compute_intensity(tri_idx);
            let i1 = compute_intensity(tri_idx + 1);
            let i2 = compute_intensity(tri_idx + 2);

            let uv0 = (uvs[tri_idx].x,     uvs[tri_idx].y);
            let uv1 = (uvs[tri_idx + 1].x, uvs[tri_idx + 1].y);
            let uv2 = (uvs[tri_idx + 2].x, uvs[tri_idx + 2].y);

            rasterize_triangle(
                &mut fb, s0, s1, s2, i0, i1, i2,
                uv0, uv1, uv2, texture, texture_lit_mode,
                charset, params.invert,
            );
        }
    }

    let ascii = fb.to_string();
    let colors = if any_texture { Some(fb.colors_hex()) } else { None };
    (ascii, colors)
}
