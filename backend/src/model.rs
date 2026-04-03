use crate::math::{Vec2, Vec3};
use std::io::Cursor;

// ─── Texture ─────────────────────────────────────────────────────────────────

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA, 4 bytes per pixel, row-major
}

impl Texture {
    /// Sample texture at (u,v), returns luminance 0.0–1.0.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        if self.width == 0 || self.height == 0 { return 0.5; }
        let u = u.fract().abs();
        let v = 1.0 - v.fract().abs(); // flip V (OBJ/GLTF origin is bottom-left)
        let x = ((u * self.width as f32) as u32).min(self.width - 1);
        let y = ((v * self.height as f32) as u32).min(self.height - 1);
        let idx = (y * self.width + x) as usize * 4;
        if idx + 3 >= self.pixels.len() { return 0.5; }
        let r = self.pixels[idx]     as f32 / 255.0;
        let g = self.pixels[idx + 1] as f32 / 255.0;
        let b = self.pixels[idx + 2] as f32 / 255.0;
        // Rec. 709 luminance
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Sample texture at (u,v), returns raw `[R, G, B]` bytes (each 0–255).
    pub fn sample_rgb(&self, u: f32, v: f32) -> [u8; 3] {
        if self.width == 0 || self.height == 0 { return [128, 128, 128]; }
        let u = u.fract().abs();
        let v = 1.0 - v.fract().abs();
        let x = ((u * self.width as f32) as u32).min(self.width - 1);
        let y = ((v * self.height as f32) as u32).min(self.height - 1);
        let idx = (y * self.width + x) as usize * 4;
        if idx + 2 >= self.pixels.len() { return [128, 128, 128]; }
        [self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2]]
    }
}

// ─── Mesh / Model ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Mesh {
    /// Flat list of triangles: every 3 elements = one triangle
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,               // same length as vertices; (0,0) if no texture
    pub texture_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Model {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
    pub vertex_count: usize,
    pub face_count: usize,
}

// Texture does not derive Clone, so we provide a manual Clone for Model that
// deep-copies the pixel data.
impl Clone for Texture {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
        }
    }
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixels_len", &self.pixels.len())
            .finish()
    }
}

impl Model {
    /// Normalize the model so it fits in a unit sphere centered at origin.
    pub fn normalize(&mut self) {
        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for mesh in &self.meshes {
            for v in &mesh.vertices {
                min.x = min.x.min(v.x);
                min.y = min.y.min(v.y);
                min.z = min.z.min(v.z);
                max.x = max.x.max(v.x);
                max.y = max.y.max(v.y);
                max.z = max.z.max(v.z);
            }
        }

        let center = Vec3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        );

        let extent = (max.x - min.x)
            .max(max.y - min.y)
            .max(max.z - min.z);
        let scale = if extent > 1e-8 { 2.0 / extent } else { 1.0 };

        for mesh in &mut self.meshes {
            for v in &mut mesh.vertices {
                v.x = (v.x - center.x) * scale;
                v.y = (v.y - center.y) * scale;
                v.z = (v.z - center.z) * scale;
            }
            // UVs are already in [0,1] — no transform needed.
        }
    }
}

/// Compute smooth normals by averaging face normals at shared vertices.
fn compute_normals(vertices: &[Vec3], indices: &[usize]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::zero(); vertices.len()];

    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0], tri[1], tri[2]);
        let e1 = vertices[i1] - vertices[i0];
        let e2 = vertices[i2] - vertices[i0];
        let n = e1.cross(e2); // not normalized — magnitude weights by area
        normals[i0] = normals[i0] + n;
        normals[i1] = normals[i1] + n;
        normals[i2] = normals[i2] + n;
    }

    normals.iter().map(|n| n.normalize()).collect()
}

// ─── OBJ loader ─────────────────────────────────────────────────────────────

pub fn load_obj(data: &[u8], name: &str) -> Result<Model, String> {
    let cursor = Cursor::new(data);
    let (models, _materials) = tobj::load_obj_buf(
        &mut std::io::BufReader::new(cursor),
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |_| Ok((vec![], ahash::AHashMap::new())),
    )
    .map_err(|e| format!("OBJ parse error: {e}"))?;

    let mut meshes = Vec::new();
    let mut total_verts = 0;
    let mut total_faces = 0;

    for m in &models {
        let mesh = &m.mesh;

        let verts: Vec<Vec3> = mesh.positions.chunks_exact(3)
            .map(|c| Vec3::new(c[0], c[1], c[2]))
            .collect();

        let indices: Vec<usize> = mesh.indices.iter().map(|&i| i as usize).collect();

        // Build per-vertex-usage list (one entry per index) for triangle soup
        let tri_verts: Vec<Vec3> = indices.iter().map(|&i| verts[i]).collect();

        // Normals: use provided or compute
        let tri_normals: Vec<Vec3> = if mesh.normals.len() == mesh.positions.len() {
            let raw: Vec<Vec3> = mesh.normals.chunks_exact(3)
                .map(|c| Vec3::new(c[0], c[1], c[2]).normalize())
                .collect();
            indices.iter().map(|&i| raw[i]).collect()
        } else {
            let vertex_normals = compute_normals(&verts, &indices);
            indices.iter().map(|&i| vertex_normals[i]).collect()
        };

        // UVs: with single_index:true, texcoords has 2 floats per unique vertex
        let has_uvs = !mesh.texcoords.is_empty()
            && mesh.texcoords.len() == (mesh.positions.len() / 3) * 2;

        let tri_uvs: Vec<Vec2> = if has_uvs {
            indices.iter().map(|&i| Vec2::new(
                mesh.texcoords[2 * i],
                mesh.texcoords[2 * i + 1],
            )).collect()
        } else {
            vec![Vec2::new(0.0, 0.0); indices.len()]
        };

        total_verts += tri_verts.len();
        total_faces += tri_verts.len() / 3;

        meshes.push(Mesh {
            vertices: tri_verts,
            normals: tri_normals,
            uvs: tri_uvs,
            texture_index: None,
        });
    }

    if meshes.is_empty() {
        return Err("OBJ file contains no meshes".into());
    }

    let mut model = Model {
        name: name.to_string(),
        meshes,
        textures: vec![],
        vertex_count: total_verts,
        face_count: total_faces,
    };
    model.normalize();
    Ok(model)
}

// ─── FBX loader ─────────────────────────────────────────────────────────────

pub fn load_fbx(data: &[u8], name: &str) -> Result<Model, String> {
    use fbxcel_dom::{
        any::AnyDocument,
        v7400::object::{model::TypedModelHandle, TypedObjectHandle},
    };

    let doc = AnyDocument::from_seekable_reader(std::io::Cursor::new(data))
        .map_err(|e| format!("FBX parse error: {e}"))?;

    let doc = match doc {
        AnyDocument::V7400(_, doc) => doc,
        // The enum is #[non_exhaustive]; this arm silences the unreachable
        // warning while remaining future-proof.
        #[allow(unreachable_patterns)]
        _ => return Err("Unsupported FBX version (need 7.4+)".into()),
    };

    let mut meshes = Vec::new();
    let mut total_verts = 0usize;
    let mut total_faces = 0usize;

    for obj in doc.objects() {
        // Only process Model/Mesh objects.
        let mesh_model = match obj.get_typed() {
            TypedObjectHandle::Model(TypedModelHandle::Mesh(m)) => m,
            _ => continue,
        };

        // Every mesh model must have exactly one child geometry mesh.
        let geom = match mesh_model.geometry() {
            Ok(g) => g,
            Err(_) => continue,
        };

        // polygon_vertices() bundles control points + raw index array together.
        let poly_verts = match geom.polygon_vertices() {
            Ok(pv) => pv,
            Err(_) => continue,
        };

        // Collect control points (vertices) from the mint::Point3<f64> iterator.
        let ctrl_points: Vec<Vec3> = match poly_verts.raw_control_points() {
            Ok(iter) => iter.map(|p| Vec3::new(p.x as f32, p.y as f32, p.z as f32)).collect(),
            Err(_) => continue,
        };

        if ctrl_points.is_empty() {
            continue;
        }

        // raw_polygon_vertices() gives the raw i32 slice.
        // Negative values mark the last vertex of each polygon (bitwise NOT encodes
        // the true index: actual_index = !raw  when raw < 0).
        let raw_indices: &[i32] = poly_verts.raw_polygon_vertices();

        // Fan-triangulate each polygon.
        let mut tri_indices: Vec<usize> = Vec::new();
        let mut current_poly: Vec<usize> = Vec::new();

        for &raw in raw_indices {
            let (idx, is_end) = if raw < 0 {
                ((!raw) as usize, true)
            } else {
                (raw as usize, false)
            };
            current_poly.push(idx);
            if is_end {
                // Fan triangulation: anchor at poly[0], emit (0, i, i+1) for i in 1..len-1.
                if current_poly.len() >= 3 {
                    for i in 1..current_poly.len() - 1 {
                        tri_indices.push(current_poly[0]);
                        tri_indices.push(current_poly[i]);
                        tri_indices.push(current_poly[i + 1]);
                    }
                }
                current_poly.clear();
            }
        }

        if tri_indices.len() < 3 {
            continue;
        }

        // Clamp out-of-range indices defensively — FBX files can be malformed.
        let max_cp = ctrl_points.len();
        let tri_verts: Vec<Vec3> = tri_indices
            .iter()
            .map(|&i| ctrl_points.get(i.min(max_cp - 1)).copied().unwrap_or_default())
            .collect();

        // Compute smooth vertex normals from the triangulated index list.
        let vertex_normals = compute_normals(&ctrl_points, &tri_indices);
        let tri_normals: Vec<Vec3> = tri_indices
            .iter()
            .map(|&i| {
                vertex_normals
                    .get(i.min(max_cp - 1))
                    .copied()
                    .unwrap_or(Vec3::new(0.0, 1.0, 0.0))
            })
            .collect();

        // FBX UV extraction is complex (layer elements with mapping/reference
        // modes). Zero UVs are a safe, correct fallback for now.
        let tri_uvs: Vec<Vec2> = vec![Vec2::new(0.0, 0.0); tri_verts.len()];

        total_verts += tri_verts.len();
        total_faces += tri_verts.len() / 3;

        meshes.push(Mesh {
            vertices: tri_verts,
            normals: tri_normals,
            uvs: tri_uvs,
            texture_index: None,
        });
    }

    if meshes.is_empty() {
        return Err("FBX file contains no renderable meshes".into());
    }

    let mut model = Model {
        name: name.to_string(),
        meshes,
        textures: vec![],
        vertex_count: total_verts,
        face_count: total_faces,
    };
    model.normalize();
    Ok(model)
}

// ─── GLTF/GLB loader ────────────────────────────────────────────────────────

pub fn load_gltf(data: &[u8], name: &str) -> Result<Model, String> {
    let (doc, buffers, images) = gltf::import_slice(data)
        .map_err(|e| format!("GLTF parse error: {e}"))?;

    let mut meshes = Vec::new();
    let mut total_verts = 0;
    let mut total_faces = 0;

    for gltf_mesh in doc.meshes() {
        for prim in gltf_mesh.primitives() {
            let reader = prim.reader(|buf| buffers.get(buf.index()).map(|d| d.0.as_ref()));

            // Read positions
            let positions: Vec<Vec3> = match reader.read_positions() {
                Some(iter) => iter.map(|p| Vec3::new(p[0], p[1], p[2])).collect(),
                None => continue,
            };

            // Read indices or generate sequential
            let indices: Vec<usize> = match reader.read_indices() {
                Some(iter) => iter.into_u32().map(|i| i as usize).collect(),
                None => (0..positions.len()).collect(),
            };

            // Triangulate: only handle TRIANGLES and TRIANGLE_STRIP for now
            let tri_indices: Vec<usize> = match prim.mode() {
                gltf::mesh::Mode::Triangles => indices,
                gltf::mesh::Mode::TriangleStrip => {
                    let mut tris = Vec::new();
                    for i in 0..indices.len().saturating_sub(2) {
                        if i % 2 == 0 {
                            tris.extend_from_slice(&[indices[i], indices[i+1], indices[i+2]]);
                        } else {
                            tris.extend_from_slice(&[indices[i+1], indices[i], indices[i+2]]);
                        }
                    }
                    tris
                }
                gltf::mesh::Mode::TriangleFan => {
                    let mut tris = Vec::new();
                    for i in 1..indices.len().saturating_sub(1) {
                        tris.extend_from_slice(&[indices[0], indices[i], indices[i+1]]);
                    }
                    tris
                }
                _ => continue, // Skip points/lines
            };

            if tri_indices.len() < 3 { continue; }

            let tri_verts: Vec<Vec3> = tri_indices.iter().map(|&i| positions[i]).collect();

            // Read normals or compute them
            let tri_normals: Vec<Vec3> = match reader.read_normals() {
                Some(iter) => {
                    let raw: Vec<Vec3> = iter.map(|n| Vec3::new(n[0], n[1], n[2]).normalize()).collect();
                    tri_indices.iter().map(|&i| raw.get(i).copied().unwrap_or(Vec3::new(0.0, 1.0, 0.0))).collect()
                }
                None => {
                    let vertex_normals = compute_normals(&positions, &tri_indices);
                    tri_indices.iter().map(|&i| vertex_normals.get(i).copied().unwrap_or(Vec3::new(0.0, 1.0, 0.0))).collect()
                }
            };

            // UVs
            let raw_uvs: Vec<[f32; 2]> = reader.read_tex_coords(0)
                .map(|c| c.into_f32().collect())
                .unwrap_or_default();
            let tri_uvs: Vec<Vec2> = tri_indices.iter().map(|&i| {
                let uv = raw_uvs.get(i).copied().unwrap_or([0.0, 0.0]);
                Vec2::new(uv[0], uv[1])
            }).collect();

            // Texture index (base color from PBR material)
            let texture_index = prim.material()
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|info| info.texture().source().index());

            total_verts += tri_verts.len();
            total_faces += tri_verts.len() / 3;

            meshes.push(Mesh {
                vertices: tri_verts,
                normals: tri_normals,
                uvs: tri_uvs,
                texture_index,
            });
        }
    }

    if meshes.is_empty() {
        return Err("GLTF file contains no renderable meshes".into());
    }

    // Extract textures from images
    let textures: Vec<Texture> = images.iter().map(|img| {
        use gltf::image::Format;
        let pixels: Vec<u8> = match img.format {
            Format::R8G8B8 => img.pixels.chunks(3)
                .flat_map(|p| [p[0], p[1], p[2], 255u8])
                .collect(),
            Format::R8G8B8A8 => img.pixels.clone(),
            Format::R8 => img.pixels.iter()
                .flat_map(|&v| [v, v, v, 255u8])
                .collect(),
            Format::R8G8 => img.pixels.chunks(2)
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            _ => vec![128u8, 128, 128, 255], // fallback grey pixel
        };
        Texture { width: img.width, height: img.height, pixels }
    }).collect();

    let mut model = Model {
        name: name.to_string(),
        meshes,
        textures,
        vertex_count: total_verts,
        face_count: total_faces,
    };
    model.normalize();
    Ok(model)
}
