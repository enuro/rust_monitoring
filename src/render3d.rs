use std::collections::HashSet;
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn rotate_y(self, angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x: self.x * c + self.z * s,
            y: self.y,
            z: -self.x * s + self.z * c,
        }
    }

    pub fn rotate_x(self, angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x: self.x,
            y: self.y * c - self.z * s,
            z: self.y * s + self.z * c,
        }
    }
}

#[derive(Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub edges: Vec<(usize, usize)>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, other: Mesh) {
        let offset = self.vertices.len();
        self.vertices.extend(other.vertices);
        for (a, b) in other.edges {
            self.edges.push((a + offset, b + offset));
        }
    }
}

pub fn ellipsoid(
    center: Vec3,
    rx: f64,
    ry: f64,
    rz: f64,
    lat_segs: usize,
    lon_segs: usize,
) -> Mesh {
    let mut mesh = Mesh::new();

    for lat in 0..=lat_segs {
        let theta = PI * (lat as f64) / (lat_segs as f64);
        let sin_t = theta.sin();
        let cos_t = theta.cos();
        for lon in 0..lon_segs {
            let phi = 2.0 * PI * (lon as f64) / (lon_segs as f64);
            mesh.vertices.push(Vec3::new(
                center.x + rx * sin_t * phi.cos(),
                center.y + ry * cos_t,
                center.z + rz * sin_t * phi.sin(),
            ));
        }
    }

    let rings = lat_segs + 1;
    for lat in 0..rings {
        for lon in 0..lon_segs {
            let idx = lat * lon_segs + lon;
            let next_lon = lat * lon_segs + (lon + 1) % lon_segs;
            mesh.edges.push((idx, next_lon));
            if lat + 1 < rings {
                let next_lat = (lat + 1) * lon_segs + lon;
                mesh.edges.push((idx, next_lat));
            }
        }
    }

    mesh
}

/// Procedural stylised "plushie" — generic chibi silhouette with twin hair tufts.
/// Not a real Ayanami Rei model; supply a proper .obj via CLI arg for that.
pub fn plushie() -> Mesh {
    let mut m = Mesh::new();

    // Head
    m.append(ellipsoid(Vec3::new(0.0, 0.55, 0.0), 0.42, 0.42, 0.42, 8, 14));
    // Body
    m.append(ellipsoid(Vec3::new(0.0, -0.15, 0.0), 0.36, 0.42, 0.36, 8, 14));
    // Arms
    m.append(ellipsoid(Vec3::new(-0.46, 0.0, 0.0), 0.11, 0.22, 0.11, 5, 8));
    m.append(ellipsoid(Vec3::new(0.46, 0.0, 0.0), 0.11, 0.22, 0.11, 5, 8));
    // Legs
    m.append(ellipsoid(Vec3::new(-0.18, -0.72, 0.0), 0.13, 0.18, 0.13, 5, 8));
    m.append(ellipsoid(Vec3::new(0.18, -0.72, 0.0), 0.13, 0.18, 0.13, 5, 8));
    // Twin side hair tufts (Rei-ish silhouette)
    m.append(ellipsoid(Vec3::new(-0.36, 0.78, 0.0), 0.1, 0.15, 0.1, 5, 8));
    m.append(ellipsoid(Vec3::new(0.36, 0.78, 0.0), 0.1, 0.15, 0.1, 5, 8));
    // Top hair tuft
    m.append(ellipsoid(Vec3::new(0.0, 0.95, 0.0), 0.14, 0.08, 0.14, 4, 8));
    // Tiny "eye dots" — front-facing markers
    m.append(ellipsoid(Vec3::new(-0.14, 0.6, 0.4), 0.04, 0.04, 0.04, 3, 6));
    m.append(ellipsoid(Vec3::new(0.14, 0.6, 0.4), 0.04, 0.04, 0.04, 3, 6));

    m
}

pub fn load_obj(path: &str) -> Result<Mesh, String> {
    let (models, _) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
    )
    .map_err(|e| format!("OBJ load error: {e}"))?;

    let mut mesh = Mesh::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for model in models {
        let m = &model.mesh;
        let offset = mesh.vertices.len();
        for v in m.positions.chunks_exact(3) {
            mesh.vertices
                .push(Vec3::new(v[0] as f64, v[1] as f64, v[2] as f64));
        }
        for tri in m.indices.chunks_exact(3) {
            let verts = [
                tri[0] as usize + offset,
                tri[1] as usize + offset,
                tri[2] as usize + offset,
            ];
            for k in 0..3 {
                let (a, b) = (verts[k], verts[(k + 1) % 3]);
                let key = if a < b { (a, b) } else { (b, a) };
                if seen.insert(key) {
                    mesh.edges.push((a, b));
                }
            }
        }
    }

    normalize(&mut mesh);
    Ok(mesh)
}

fn normalize(mesh: &mut Mesh) {
    if mesh.vertices.is_empty() {
        return;
    }
    let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in &mesh.vertices {
        if v.x < min.x {
            min.x = v.x;
        }
        if v.x > max.x {
            max.x = v.x;
        }
        if v.y < min.y {
            min.y = v.y;
        }
        if v.y > max.y {
            max.y = v.y;
        }
        if v.z < min.z {
            min.z = v.z;
        }
        if v.z > max.z {
            max.z = v.z;
        }
    }
    let cx = (min.x + max.x) / 2.0;
    let cy = (min.y + max.y) / 2.0;
    let cz = (min.z + max.z) / 2.0;
    let extent = (max.x - min.x).max(max.y - min.y).max(max.z - min.z).max(1e-6);
    let scale = 2.0 / extent;
    for v in &mut mesh.vertices {
        v.x = (v.x - cx) * scale;
        v.y = (v.y - cy) * scale;
        v.z = (v.z - cz) * scale;
    }
}

/// Simple perspective projection. Camera at z = -focal looking towards +z.
pub fn project(v: Vec3, focal: f64) -> (f64, f64) {
    let denom = v.z + focal;
    let factor = if denom.abs() < 1e-6 {
        1.0
    } else {
        focal / denom
    };
    (v.x * factor, v.y * factor)
}
