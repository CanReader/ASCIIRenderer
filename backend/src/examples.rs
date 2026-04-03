use crate::math::{Vec2, Vec3};
use crate::model::{Mesh, Model};

/// Returns a list of (display-name, Model) pairs for OBJ asset models embedded
/// at compile time via `include_bytes!`. Models that fail to parse are skipped
/// with a diagnostic message on stderr.
pub fn asset_models() -> Vec<(&'static str, Model)> {
    const ASSETS: &[(&str, &[u8])] = &[
        ("Teapot",    include_bytes!("../assets/teapot.obj")),
        ("Suzanne",   include_bytes!("../assets/suzanne.obj")),
        ("Spot",      include_bytes!("../assets/spot.obj")),
        ("Bunny",     include_bytes!("../assets/bunny.obj")),
        ("Armadillo", include_bytes!("../assets/armadillo.obj")),
    ];

    let mut models = Vec::with_capacity(ASSETS.len());
    for (name, bytes) in ASSETS {
        match crate::model::load_obj(bytes, name) {
            Ok(model) => models.push((*name, model)),
            Err(e) => eprintln!("asset_models: skipping {name}: {e}"),
        }
    }
    models
}

/// Returns a list of (display-name, Model) pairs for GLB asset models embedded
/// at compile time via `include_bytes!`. Models that fail to parse are skipped
/// with a diagnostic message on stderr.
pub fn asset_models_gltf() -> Vec<(&'static str, Model)> {
    const ASSETS: &[(&str, &[u8])] = &[
        ("Duck",           include_bytes!("../assets/Duck.glb")),
        ("Damaged Helmet", include_bytes!("../assets/DamagedHelmet.glb")),
        ("Avocado",        include_bytes!("../assets/Avocado.glb")),
    ];

    let mut out = Vec::with_capacity(ASSETS.len());
    for (name, bytes) in ASSETS {
        match crate::model::load_gltf(bytes, name) {
            Ok(model) => out.push((*name, model)),
            Err(e) => eprintln!("asset_models_gltf: skipping {name}: {e}"),
        }
    }
    out
}

/// Returns a list of (display-name, Model) pairs for built-in procedural models.
/// Each model has `normalize()` called so it fits the unit sphere the renderer
/// expects.
pub fn builtin_models() -> Vec<(&'static str, Model)> {
    vec![
        ("Cube", make_cube()),
        ("Sphere", make_sphere()),
        ("Torus", make_torus()),
    ]
}

// ─── Cube ─────────────────────────────────────────────────────────────────────

fn make_cube() -> Model {
    // 8 unique vertices of a unit cube [-1, 1]^3
    let v = [
        Vec3::new(-1.0, -1.0, -1.0), // 0
        Vec3::new( 1.0, -1.0, -1.0), // 1
        Vec3::new( 1.0,  1.0, -1.0), // 2
        Vec3::new(-1.0,  1.0, -1.0), // 3
        Vec3::new(-1.0, -1.0,  1.0), // 4
        Vec3::new( 1.0, -1.0,  1.0), // 5
        Vec3::new( 1.0,  1.0,  1.0), // 6
        Vec3::new(-1.0,  1.0,  1.0), // 7
    ];

    // Each face has an axis-aligned normal and 2 triangles.
    // (v_idx_0, v_idx_1, v_idx_2, normal)
    let faces: &[([usize; 6], Vec3)] = &[
        // Front  (+Z)
        ([4, 5, 6, 4, 6, 7], Vec3::new( 0.0,  0.0,  1.0)),
        // Back   (-Z)
        ([1, 0, 3, 1, 3, 2], Vec3::new( 0.0,  0.0, -1.0)),
        // Right  (+X)
        ([5, 1, 2, 5, 2, 6], Vec3::new( 1.0,  0.0,  0.0)),
        // Left   (-X)
        ([0, 4, 7, 0, 7, 3], Vec3::new(-1.0,  0.0,  0.0)),
        // Top    (+Y)
        ([7, 6, 2, 7, 2, 3], Vec3::new( 0.0,  1.0,  0.0)),
        // Bottom (-Y)
        ([0, 1, 5, 0, 5, 4], Vec3::new( 0.0, -1.0,  0.0)),
    ];

    let mut vertices = Vec::with_capacity(36);
    let mut normals = Vec::with_capacity(36);

    for (indices, normal) in faces {
        for &idx in indices {
            vertices.push(v[idx]);
            normals.push(*normal);
        }
    }

    let face_count = vertices.len() / 3;
    let vertex_count = vertices.len();
    let uvs = vec![Vec2::new(0.0, 0.0); vertex_count];

    let mut model = Model {
        name: "Cube".to_string(),
        meshes: vec![Mesh { vertices, normals, uvs, texture_index: None }],
        textures: vec![],
        vertex_count,
        face_count,
    };
    model.normalize();
    model
}

// ─── Sphere ───────────────────────────────────────────────────────────────────

fn make_sphere() -> Model {
    const LON_SEGS: usize = 24; // longitude divisions
    const LAT_SEGS: usize = 16; // latitude divisions

    // Generate a grid of (LON_SEGS+1) × (LAT_SEGS+1) vertices so that seam
    // vertices are not shared — this keeps the triangle soup layout simple and
    // avoids index arithmetic on a wrapped grid.
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    for lat in 0..LAT_SEGS {
        let theta0 = std::f32::consts::PI * (lat as f32) / (LAT_SEGS as f32);
        let theta1 = std::f32::consts::PI * (lat as f32 + 1.0) / (LAT_SEGS as f32);

        for lon in 0..LON_SEGS {
            let phi0 = 2.0 * std::f32::consts::PI * (lon as f32) / (LON_SEGS as f32);
            let phi1 = 2.0 * std::f32::consts::PI * (lon as f32 + 1.0) / (LON_SEGS as f32);

            // Four corners of the quad
            let p = |theta: f32, phi: f32| -> Vec3 {
                Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.cos(),
                    theta.sin() * phi.sin(),
                )
            };

            let p00 = p(theta0, phi0);
            let p10 = p(theta1, phi0);
            let p11 = p(theta1, phi1);
            let p01 = p(theta0, phi1);

            // For a unit sphere the vertex position equals the outward normal.
            // Triangle 1: p00, p10, p11
            vertices.push(p00); normals.push(p00.normalize());
            vertices.push(p10); normals.push(p10.normalize());
            vertices.push(p11); normals.push(p11.normalize());

            // Triangle 2: p00, p11, p01
            vertices.push(p00); normals.push(p00.normalize());
            vertices.push(p11); normals.push(p11.normalize());
            vertices.push(p01); normals.push(p01.normalize());
        }
    }

    let face_count = vertices.len() / 3;
    let vertex_count = vertices.len();
    let uvs = vec![Vec2::new(0.0, 0.0); vertex_count];

    let mut model = Model {
        name: "Sphere".to_string(),
        meshes: vec![Mesh { vertices, normals, uvs, texture_index: None }],
        textures: vec![],
        vertex_count,
        face_count,
    };
    model.normalize();
    model
}

// ─── Torus ────────────────────────────────────────────────────────────────────

fn make_torus() -> Model {
    const MAJOR_R: f32 = 1.0;  // distance from torus center to tube center
    const MINOR_R: f32 = 0.35; // tube radius
    const RING_SEGS: usize = 32; // segments around the major circle
    const TUBE_SEGS: usize = 24; // segments around the tube

    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    for ring in 0..RING_SEGS {
        let phi0 = 2.0 * std::f32::consts::PI * (ring as f32) / (RING_SEGS as f32);
        let phi1 = 2.0 * std::f32::consts::PI * (ring as f32 + 1.0) / (RING_SEGS as f32);

        for tube in 0..TUBE_SEGS {
            let theta0 = 2.0 * std::f32::consts::PI * (tube as f32) / (TUBE_SEGS as f32);
            let theta1 = 2.0 * std::f32::consts::PI * (tube as f32 + 1.0) / (TUBE_SEGS as f32);

            // Surface point: (MAJOR_R + MINOR_R*cos(theta)) * [cos(phi), 0, sin(phi)]
            //                + MINOR_R * sin(theta) * [0, 1, 0]
            // Normal: direction from the ring-center point to the surface point.
            let point = |phi: f32, theta: f32| -> (Vec3, Vec3) {
                let ring_center = Vec3::new(MAJOR_R * phi.cos(), 0.0, MAJOR_R * phi.sin());
                let surface = Vec3::new(
                    (MAJOR_R + MINOR_R * theta.cos()) * phi.cos(),
                    MINOR_R * theta.sin(),
                    (MAJOR_R + MINOR_R * theta.cos()) * phi.sin(),
                );
                let normal = (surface - ring_center).normalize();
                (surface, normal)
            };

            let (p00, n00) = point(phi0, theta0);
            let (p10, n10) = point(phi1, theta0);
            let (p11, n11) = point(phi1, theta1);
            let (p01, n01) = point(phi0, theta1);

            // Triangle 1
            vertices.push(p00); normals.push(n00);
            vertices.push(p10); normals.push(n10);
            vertices.push(p11); normals.push(n11);

            // Triangle 2
            vertices.push(p00); normals.push(n00);
            vertices.push(p11); normals.push(n11);
            vertices.push(p01); normals.push(n01);
        }
    }

    let face_count = vertices.len() / 3;
    let vertex_count = vertices.len();
    let uvs = vec![Vec2::new(0.0, 0.0); vertex_count];

    let mut model = Model {
        name: "Torus".to_string(),
        meshes: vec![Mesh { vertices, normals, uvs, texture_index: None }],
        textures: vec![],
        vertex_count,
        face_count,
    };
    model.normalize();
    model
}
