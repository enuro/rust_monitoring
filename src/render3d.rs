use std::collections::HashSet;
use std::f64::consts::PI;
use std::path::PathBuf;

pub const MODEL_DIR: &str = "src/model";

pub type Rgb = (u8, u8, u8);

const DEFAULT_COLOR: Rgb = (180, 220, 240);

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
    pub edge_colors: Vec<Rgb>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, other: Mesh) {
        let offset = self.vertices.len();
        self.vertices.extend(other.vertices);
        for (i, (a, b)) in other.edges.iter().enumerate() {
            self.edges.push((a + offset, b + offset));
            self.edge_colors
                .push(other.edge_colors.get(i).copied().unwrap_or(DEFAULT_COLOR));
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
    color: Rgb,
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
            mesh.edge_colors.push(color);
            if lat + 1 < rings {
                let next_lat = (lat + 1) * lon_segs + lon;
                mesh.edges.push((idx, next_lat));
                mesh.edge_colors.push(color);
            }
        }
    }

    mesh
}

pub fn plushie() -> Mesh {
    let mut m = Mesh::new();
    let body: Rgb = (250, 220, 230);
    let limb: Rgb = (235, 190, 205);
    let hair: Rgb = (170, 215, 255);
    let topknot: Rgb = (200, 230, 255);
    let eye: Rgb = (40, 30, 30);

    m.append(ellipsoid(Vec3::new(0.0, 0.55, 0.0), 0.42, 0.42, 0.42, 8, 14, body));
    m.append(ellipsoid(Vec3::new(0.0, -0.15, 0.0), 0.36, 0.42, 0.36, 8, 14, body));
    m.append(ellipsoid(Vec3::new(-0.46, 0.0, 0.0), 0.11, 0.22, 0.11, 5, 8, limb));
    m.append(ellipsoid(Vec3::new(0.46, 0.0, 0.0), 0.11, 0.22, 0.11, 5, 8, limb));
    m.append(ellipsoid(Vec3::new(-0.18, -0.72, 0.0), 0.13, 0.18, 0.13, 5, 8, limb));
    m.append(ellipsoid(Vec3::new(0.18, -0.72, 0.0), 0.13, 0.18, 0.13, 5, 8, limb));
    m.append(ellipsoid(Vec3::new(-0.36, 0.78, 0.0), 0.1, 0.15, 0.1, 5, 8, hair));
    m.append(ellipsoid(Vec3::new(0.36, 0.78, 0.0), 0.1, 0.15, 0.1, 5, 8, hair));
    m.append(ellipsoid(Vec3::new(0.0, 0.95, 0.0), 0.14, 0.08, 0.14, 4, 8, topknot));
    m.append(ellipsoid(Vec3::new(-0.14, 0.6, 0.4), 0.04, 0.04, 0.04, 3, 6, eye));
    m.append(ellipsoid(Vec3::new(0.14, 0.6, 0.4), 0.04, 0.04, 0.04, 3, 6, eye));

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
    let mut vert_colors: Vec<Rgb> = Vec::new();

    for model in models {
        let m = &model.mesh;
        let offset = mesh.vertices.len();
        for v in m.positions.chunks_exact(3) {
            mesh.vertices
                .push(Vec3::new(v[0] as f64, v[1] as f64, v[2] as f64));
        }
        let vert_count = m.positions.len() / 3;
        if m.vertex_color.len() == vert_count * 3 {
            for c in m.vertex_color.chunks_exact(3) {
                vert_colors.push((
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                ));
            }
        } else {
            for _ in 0..vert_count {
                vert_colors.push(DEFAULT_COLOR);
            }
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
                    let ca = vert_colors[a];
                    let cb = vert_colors[b];
                    let avg = (
                        ((ca.0 as u16 + cb.0 as u16) / 2) as u8,
                        ((ca.1 as u16 + cb.1 as u16) / 2) as u8,
                        ((ca.2 as u16 + cb.2 as u16) / 2) as u8,
                    );
                    mesh.edges.push((a, b));
                    mesh.edge_colors.push(avg);
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

pub fn project(v: Vec3, focal: f64) -> (f64, f64) {
    let denom = v.z + focal;
    let factor = if denom.abs() < 1e-6 {
        1.0
    } else {
        focal / denom
    };
    (v.x * factor, v.y * factor)
}

pub fn ensure_default_models() {
    let dir = std::path::Path::new(MODEL_DIR);
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("cube.obj"), cube_obj());
    let _ = std::fs::write(dir.join("rose.obj"), rose_obj());
}

pub fn list_model_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(MODEL_DIR) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension()
                .map(|x| x.eq_ignore_ascii_case("obj"))
                .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn cube_obj() -> String {
    let palette = [
        (0.95, 0.40, 0.35),
        (0.95, 0.75, 0.30),
        (0.40, 0.85, 0.45),
        (0.30, 0.75, 0.95),
        (0.55, 0.45, 0.95),
        (0.95, 0.45, 0.85),
        (0.40, 0.95, 0.85),
        (0.95, 0.95, 0.45),
    ];
    let verts = [
        (-1.0_f64, -1.0, -1.0),
        (1.0, -1.0, -1.0),
        (1.0, 1.0, -1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, 1.0),
        (-1.0, 1.0, 1.0),
    ];
    let faces = [
        (1, 2, 3),
        (1, 3, 4),
        (5, 7, 6),
        (5, 8, 7),
        (1, 4, 8),
        (1, 8, 5),
        (2, 6, 7),
        (2, 7, 3),
        (4, 3, 7),
        (4, 7, 8),
        (1, 5, 6),
        (1, 6, 2),
    ];
    let mut out = String::from("# cube\n");
    for (i, (x, y, z)) in verts.iter().enumerate() {
        let (r, g, b) = palette[i];
        out.push_str(&format!("v {x} {y} {z} {r:.3} {g:.3} {b:.3}\n"));
    }
    for (a, b, c) in faces {
        out.push_str(&format!("f {a} {b} {c}\n"));
    }
    out
}

fn rose_obj() -> String {
    let mut verts: Vec<(f64, f64, f64)> = Vec::new();
    let mut colors: Vec<(f32, f32, f32)> = Vec::new();
    let mut faces: Vec<(usize, usize, usize)> = Vec::new();

    let red_outer = (0.95, 0.12, 0.20);
    let red_mid = (0.85, 0.10, 0.18);
    let red_inner = (0.65, 0.06, 0.14);
    let green_stem = (0.20, 0.65, 0.25);

    add_petal_layer(
        &mut verts, &mut colors, &mut faces, 8, 1.0, 0.30, 0.00, 0.0, red_outer,
    );
    add_petal_layer(
        &mut verts,
        &mut colors,
        &mut faces,
        6,
        0.60,
        0.32,
        0.15,
        PI / 6.0,
        red_mid,
    );
    add_petal_layer(
        &mut verts,
        &mut colors,
        &mut faces,
        5,
        0.35,
        0.28,
        0.28,
        PI / 3.0,
        red_inner,
    );
    add_stem(&mut verts, &mut colors, &mut faces, green_stem);

    let mut out = String::from("# rose\n");
    for (i, (x, y, z)) in verts.iter().enumerate() {
        let (r, g, b) = colors[i];
        out.push_str(&format!("v {x:.4} {y:.4} {z:.4} {r:.3} {g:.3} {b:.3}\n"));
    }
    for (a, b, c) in &faces {
        out.push_str(&format!("f {} {} {}\n", a + 1, b + 1, c + 1));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn add_petal_layer(
    verts: &mut Vec<(f64, f64, f64)>,
    colors: &mut Vec<(f32, f32, f32)>,
    faces: &mut Vec<(usize, usize, usize)>,
    petal_count: usize,
    radius: f64,
    height_factor: f64,
    y_offset: f64,
    angle_offset: f64,
    color: (f32, f32, f32),
) {
    const RES_U: usize = 5;
    const RES_V: usize = 4;
    for p in 0..petal_count {
        let base = angle_offset + (p as f64 / petal_count as f64) * 2.0 * PI;
        let mut grid: Vec<Vec<usize>> = vec![vec![0; RES_V + 1]; RES_U + 1];
        for ui in 0..=RES_U {
            let u = ui as f64 / RES_U as f64;
            for vi in 0..=RES_V {
                let v = vi as f64 / RES_V as f64 * 2.0 - 1.0;
                let env = (u * PI).sin();
                let r = u * radius;
                let half_width = 0.40 * env;
                let theta = base + v * half_width;
                let curl = v * v * 0.35 * env;
                let droop = u * 0.08;
                let y = u * height_factor + curl - droop + y_offset;
                grid[ui][vi] = verts.len();
                verts.push((r * theta.cos(), y, r * theta.sin()));
                colors.push(color);
            }
        }
        for ui in 0..RES_U {
            for vi in 0..RES_V {
                let a = grid[ui][vi];
                let b = grid[ui][vi + 1];
                let c = grid[ui + 1][vi + 1];
                let d = grid[ui + 1][vi];
                faces.push((a, b, c));
                faces.push((a, c, d));
            }
        }
    }
}

fn add_stem(
    verts: &mut Vec<(f64, f64, f64)>,
    colors: &mut Vec<(f32, f32, f32)>,
    faces: &mut Vec<(usize, usize, usize)>,
    color: (f32, f32, f32),
) {
    let segs = 6;
    let layers = 4;
    let top = 0.0;
    let bot = -1.2;
    let radius = 0.04;
    let start = verts.len();
    for h in 0..=layers {
        let y = top + (bot - top) * h as f64 / layers as f64;
        for s in 0..segs {
            let theta = s as f64 / segs as f64 * 2.0 * PI;
            verts.push((radius * theta.cos(), y, radius * theta.sin()));
            colors.push(color);
        }
    }
    for h in 0..layers {
        for s in 0..segs {
            let a = start + h * segs + s;
            let b = start + h * segs + (s + 1) % segs;
            let c = start + (h + 1) * segs + (s + 1) % segs;
            let d = start + (h + 1) * segs + s;
            faces.push((a, b, c));
            faces.push((a, c, d));
        }
    }
}
