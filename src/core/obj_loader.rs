//! Wavefront OBJ 3D Model Mesh Parser & PBR Material System (AE Parity).
//!
//! Enables importing 3D models (.obj) into compositions with automatic triangulation,
//! vertex normal generation, UV texture mapping, and PBR material properties.

#![allow(dead_code)]

use std::collections::HashMap;

mod material_library;
mod triangulate;

pub use material_library::{obj_material_library_paths, parse_obj_file};
pub(crate) use material_library::{parse_obj_file_source, read_obj_file_source};
use triangulate::triangulate_obj_face;

const MAX_OBJ_EXPANDED_BYTES: usize = 256 * 1024 * 1024;
const MAX_OBJ_FACE_VERTICES: usize = 1_000_000;

fn validate_obj_expanded_size(
    vertices: usize,
    indices: usize,
    triangles: usize,
) -> Result<(), String> {
    // Includes position/normal/UV data, normal-generation keys and hash-table
    // overhead, plus emitted index and material-index arrays.
    let bytes = vertices
        .checked_mul(96)
        .and_then(|bytes| bytes.checked_add(indices.checked_mul(std::mem::size_of::<u32>())?))
        .and_then(|bytes| bytes.checked_add(triangles.checked_mul(std::mem::size_of::<usize>())?))
        .ok_or_else(|| "OBJ expanded mesh size overflows address space".to_string())?;
    if bytes > MAX_OBJ_EXPANDED_BYTES {
        return Err("Expanded OBJ mesh exceeds the 256 MiB import limit".into());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PbrMaterial3D {
    pub name: String,
    pub albedo_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emission: [f32; 3],
    #[serde(default = "default_alpha_mode")]
    pub alpha_mode: String,
    #[serde(default = "default_alpha_cutoff")]
    pub alpha_cutoff: f32,
    pub albedo_texture: Option<String>,
    #[serde(default)]
    pub albedo_texture_index: Option<usize>,
    #[serde(default = "default_texture_wrap")]
    pub texture_wrap_s: u32,
    #[serde(default = "default_texture_wrap")]
    pub texture_wrap_t: u32,
    #[serde(default)]
    pub texture_min_nearest: bool,
    #[serde(default)]
    pub texture_mag_nearest: bool,
    #[serde(default = "default_uv_scale")]
    pub texture_uv_scale: [f32; 2],
    #[serde(default)]
    pub texture_uv_offset: [f32; 2],
    #[serde(default)]
    pub texture_uv_rotation: f32,
    #[serde(default)]
    pub texture_tex_coord: u32,
    pub normal_map: Option<String>,
}

fn default_alpha_mode() -> String {
    "OPAQUE".to_string()
}

fn default_alpha_cutoff() -> f32 {
    0.5
}

fn default_texture_wrap() -> u32 {
    10497
}

fn default_uv_scale() -> [f32; 2] {
    [1.0, 1.0]
}

impl Default for PbrMaterial3D {
    fn default() -> Self {
        Self {
            name: "DefaultMaterial".to_string(),
            albedo_color: [0.8, 0.8, 0.8, 1.0],
            roughness: 0.5,
            metallic: 0.0,
            emission: [0.0, 0.0, 0.0],
            alpha_mode: default_alpha_mode(),
            alpha_cutoff: default_alpha_cutoff(),
            albedo_texture: None,
            albedo_texture_index: None,
            texture_wrap_s: default_texture_wrap(),
            texture_wrap_t: default_texture_wrap(),
            texture_min_nearest: false,
            texture_mag_nearest: false,
            texture_uv_scale: default_uv_scale(),
            texture_uv_offset: [0.0; 2],
            texture_uv_rotation: 0.0,
            texture_tex_coord: 0,
            normal_map: None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Mesh3D {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    #[serde(default)]
    pub uv_sets: Vec<Vec<[f32; 2]>>,
    #[serde(default)]
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub materials: Vec<PbrMaterial3D>,
    pub material_indices: Vec<usize>, // Material index per triangle
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    #[serde(skip)]
    pub embedded_textures: Vec<std::sync::Arc<crate::core::image_cache::CachedImage>>,
}

impl Mesh3D {
    pub fn new(name: String) -> Self {
        Self {
            name,
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            uv_sets: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
            materials: vec![PbrMaterial3D::default()],
            material_indices: Vec::new(),
            bbox_min: [f32::INFINITY; 3],
            bbox_max: [f32::NEG_INFINITY; 3],
            embedded_textures: Vec::new(),
        }
    }

    /// Computes bounding box dimensions and center.
    pub fn update_bounds(&mut self) {
        let mut bmin = [f32::INFINITY; 3];
        let mut bmax = [f32::NEG_INFINITY; 3];

        for v in &self.vertices {
            bmin[0] = bmin[0].min(v[0]);
            bmin[1] = bmin[1].min(v[1]);
            bmin[2] = bmin[2].min(v[2]);

            bmax[0] = bmax[0].max(v[0]);
            bmax[1] = bmax[1].max(v[1]);
            bmax[2] = bmax[2].max(v[2]);
        }

        self.bbox_min = bmin;
        self.bbox_max = bmax;
    }

    /// Calculates smooth per-vertex normals from triangle face normals.
    pub fn compute_smooth_normals(&mut self) {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return;
        }

        let mut normals = vec![[0.0f32; 3]; self.vertices.len()];

        for chunk in self.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len() {
                let v0 = self.vertices[i0];
                let v1 = self.vertices[i1];
                let v2 = self.vertices[i2];

                let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

                // Cross product
                let fnorm = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];

                for &idx in &[i0, i1, i2] {
                    normals[idx][0] += fnorm[0];
                    normals[idx][1] += fnorm[1];
                    normals[idx][2] += fnorm[2];
                }
            }
        }

        for n in &mut normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-5);
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        }

        self.normals = normals;
    }

    /// Möller-Trumbore Ray-Triangle intersection test.
    pub fn ray_intersect(&self, ray_orig: [f32; 3], ray_dir: [f32; 3]) -> Option<(f32, usize)> {
        let mut closest_t = f32::INFINITY;
        let mut hit_tri = None;

        for (tri_idx, chunk) in self.indices.as_chunks::<3>().0.iter().enumerate() {
            let (Some(&v0), Some(&v1), Some(&v2)) = (
                self.vertices.get(chunk[0] as usize),
                self.vertices.get(chunk[1] as usize),
                self.vertices.get(chunk[2] as usize),
            ) else {
                continue;
            };

            if let Some(t) = ray_triangle_intersect(ray_orig, ray_dir, v0, v1, v2) {
                if t > 0.001 && t < closest_t {
                    closest_t = t;
                    hit_tri = Some(tri_idx);
                }
            }
        }

        hit_tri.map(|tri| (closest_t, tri))
    }
}

/// Parses Wavefront OBJ format text content.
pub fn parse_obj_str(content: &str) -> Result<Mesh3D, String> {
    parse_obj_str_with_materials(content, &HashMap::new())
}

pub(super) fn parse_obj_str_with_materials(
    content: &str,
    library_materials: &HashMap<String, PbrMaterial3D>,
) -> Result<Mesh3D, String> {
    let mut raw_positions: Vec<[f32; 3]> = Vec::new();
    let mut raw_uvs: Vec<[f32; 2]> = Vec::new();
    let mut raw_normals: Vec<[f32; 3]> = Vec::new();
    let mut mesh = Mesh3D::new("ImportedMesh".to_string());
    let mut vertex_lookup = HashMap::<(usize, Option<usize>, Option<usize>, u64), u32>::new();
    let mut missing_normals = Vec::<bool>::new();
    let mut missing_normal_keys = Vec::<(usize, u64)>::new();
    let mut smoothing_groups = HashMap::<String, u64>::new();
    let mut current_smoothing_group = None;
    let mut next_smoothing_group = 1u64;
    let mut next_flat_face = 1u64;
    mesh.materials.clear();
    mesh.materials.push(PbrMaterial3D {
        name: "DefaultMaterial".to_string(),
        albedo_color: [1.0; 4],
        ..Default::default()
    });
    let mut material_indices = HashMap::<String, usize>::new();
    let mut material_names = library_materials.keys().collect::<Vec<_>>();
    material_names.sort_unstable();
    for name in material_names {
        let material = &library_materials[name];
        let index = mesh.materials.len();
        mesh.materials.push(material.clone());
        material_indices.insert(name.clone(), index);
    }
    let mut current_material = 0usize;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else { continue };

        match tag {
            "v" => {
                let x: f32 = parts
                    .next()
                    .ok_or_else(|| "missing vertex x".to_string())?
                    .parse()
                    .map_err(|_| "invalid vertex coordinate".to_string())?;
                let y: f32 = parts
                    .next()
                    .ok_or_else(|| "missing vertex y".to_string())?
                    .parse()
                    .map_err(|_| "invalid vertex coordinate".to_string())?;
                let z: f32 = parts
                    .next()
                    .ok_or_else(|| "missing vertex z".to_string())?
                    .parse()
                    .map_err(|_| "invalid vertex coordinate".to_string())?;
                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    return Err("non-finite vertex coordinate".to_string());
                }
                raw_positions.push([x, y, z]);
            }
            "vt" => {
                let u: f32 = parts
                    .next()
                    .ok_or_else(|| "missing texture u".to_string())?
                    .parse()
                    .map_err(|_| "invalid texture coordinate".to_string())?;
                let v: f32 = parts
                    .next()
                    .ok_or_else(|| "missing texture v".to_string())?
                    .parse()
                    .map_err(|_| "invalid texture coordinate".to_string())?;
                if !u.is_finite() || !v.is_finite() {
                    return Err("non-finite texture coordinate".to_string());
                }
                raw_uvs.push([u, v]);
            }
            "vn" => {
                let nx: f32 = parts
                    .next()
                    .ok_or_else(|| "missing normal x".to_string())?
                    .parse()
                    .map_err(|_| "invalid normal coordinate".to_string())?;
                let ny: f32 = parts
                    .next()
                    .ok_or_else(|| "missing normal y".to_string())?
                    .parse()
                    .map_err(|_| "invalid normal coordinate".to_string())?;
                let nz: f32 = parts
                    .next()
                    .ok_or_else(|| "missing normal z".to_string())?
                    .parse()
                    .map_err(|_| "invalid normal coordinate".to_string())?;
                if !nx.is_finite() || !ny.is_finite() || !nz.is_finite() {
                    return Err("non-finite normal coordinate".to_string());
                }
                raw_normals.push([nx, ny, nz]);
            }
            "s" => {
                let group = parts.next().unwrap_or("off");
                current_smoothing_group = if matches!(group, "off" | "0") {
                    None
                } else {
                    let group = if group == "on" { "1" } else { group };
                    let id = *smoothing_groups
                        .entry(group.to_string())
                        .or_insert_with(|| {
                            let id = next_smoothing_group;
                            next_smoothing_group = next_smoothing_group.saturating_add(1);
                            id
                        });
                    Some(id)
                };
            }
            "usemtl" => {
                let name = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return Err("OBJ usemtl directive has no name".to_string());
                }
                current_material = if let Some(&index) = material_indices.get(&name) {
                    index
                } else {
                    let index = mesh.materials.len();
                    mesh.materials.push(PbrMaterial3D {
                        name: name.clone(),
                        albedo_color: [1.0; 4],
                        ..Default::default()
                    });
                    material_indices.insert(name, index);
                    index
                };
            }
            "f" => {
                let flat_face_key = (1u64 << 63) | next_flat_face;
                next_flat_face = next_flat_face.saturating_add(1);
                let mut face_indices = Vec::new();
                for token in parts {
                    if face_indices.len() >= MAX_OBJ_FACE_VERTICES {
                        return Err("OBJ face exceeds the 1,000,000 vertex face limit".into());
                    }
                    // Formats: v, v/vt, v/vt/vn, v//vn
                    let mut subparts = token.split('/');
                    let raw_position = subparts.next().unwrap_or_default();
                    let raw_uv = subparts.next();
                    let raw_normal = subparts.next();
                    if subparts.next().is_some() || raw_position.is_empty() {
                        return Err(format!("invalid face vertex reference: {token}"));
                    }
                    let position_index = resolve_obj_index(raw_position, raw_positions.len())
                        .map_err(|error| format!("{error} in face reference `{token}`"))?;
                    let uv_index = raw_uv
                        .filter(|raw| !raw.is_empty())
                        .map(|raw| resolve_obj_index(raw, raw_uvs.len()))
                        .transpose()
                        .map_err(|error| format!("{error} in face reference `{token}`"))?;
                    let normal_index = raw_normal
                        .filter(|raw| !raw.is_empty())
                        .map(|raw| resolve_obj_index(raw, raw_normals.len()))
                        .transpose()
                        .map_err(|error| format!("{error} in face reference `{token}`"))?;
                    let smoothing_key = if normal_index.is_some() {
                        0
                    } else {
                        current_smoothing_group.unwrap_or(flat_face_key)
                    };
                    let key = (position_index, uv_index, normal_index, smoothing_key);
                    let vertex_index = if let Some(&existing) = vertex_lookup.get(&key) {
                        existing
                    } else {
                        validate_obj_expanded_size(
                            mesh.vertices.len() + 1,
                            mesh.indices.len(),
                            mesh.material_indices.len(),
                        )?;
                        let vertex_index = u32::try_from(mesh.vertices.len()).map_err(|_| {
                            "OBJ mesh exceeds the 32-bit vertex index limit".to_string()
                        })?;
                        mesh.vertices.try_reserve(1).map_err(|error| {
                            format!("Could not reserve OBJ vertex data: {error}")
                        })?;
                        mesh.uvs
                            .try_reserve(1)
                            .map_err(|error| format!("Could not reserve OBJ UV data: {error}"))?;
                        mesh.normals.try_reserve(1).map_err(|error| {
                            format!("Could not reserve OBJ normal data: {error}")
                        })?;
                        missing_normals.try_reserve(1).map_err(|error| {
                            format!("Could not reserve OBJ normal flags: {error}")
                        })?;
                        missing_normal_keys.try_reserve(1).map_err(|error| {
                            format!("Could not reserve OBJ normal keys: {error}")
                        })?;
                        let position = raw_positions[position_index];
                        mesh.vertices.push(position);
                        mesh.uvs.push(uv_index.map_or([0.0; 2], |index| {
                            [raw_uvs[index][0], 1.0 - raw_uvs[index][1]]
                        }));
                        let has_normal = normal_index.is_some();
                        mesh.normals
                            .push(normal_index.map_or([0.0; 3], |index| raw_normals[index]));
                        missing_normals.push(!has_normal);
                        missing_normal_keys.push((position_index, smoothing_key));
                        vertex_lookup.insert(key, vertex_index);
                        vertex_index
                    };
                    face_indices
                        .try_reserve(1)
                        .map_err(|error| format!("Could not reserve OBJ face indices: {error}"))?;
                    face_indices.push(vertex_index);
                }

                // Triangulate n-gons while keeping concave faces inside their outline.
                if face_indices.len() >= 3 {
                    let triangles = triangulate_obj_face(&face_indices, &mesh.vertices)?;
                    let triangle_count = triangles.len();
                    let new_index_count = triangle_count
                        .checked_mul(3)
                        .ok_or("OBJ triangulated index count overflows address space")?;
                    validate_obj_expanded_size(
                        mesh.vertices.len(),
                        mesh.indices
                            .len()
                            .checked_add(new_index_count)
                            .ok_or("OBJ index count overflows address space")?,
                        mesh.material_indices
                            .len()
                            .checked_add(triangle_count)
                            .ok_or("OBJ material count overflows address space")?,
                    )?;
                    mesh.indices
                        .try_reserve(new_index_count)
                        .map_err(|error| format!("Could not reserve OBJ index data: {error}"))?;
                    mesh.material_indices
                        .try_reserve(triangle_count)
                        .map_err(|error| format!("Could not reserve OBJ material data: {error}"))?;
                    for triangle in triangles {
                        mesh.indices.extend_from_slice(&triangle);
                        mesh.material_indices.push(current_material);
                    }
                } else if !face_indices.is_empty() {
                    return Err("OBJ face must contain at least three vertices".to_string());
                }
            }
            _ => {}
        }
    }

    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err("OBJ file contains no renderable faces".to_string());
    }
    if missing_normals.iter().any(|&missing| missing) {
        let mut generated = HashMap::<(usize, u64), [f32; 3]>::new();
        generated
            .try_reserve(mesh.vertices.len())
            .map_err(|error| format!("Could not reserve OBJ smooth-normal data: {error}"))?;
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let a = mesh.vertices[triangle[0] as usize];
            let b = mesh.vertices[triangle[1] as usize];
            let c = mesh.vertices[triangle[2] as usize];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let face_normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            for &index in triangle {
                let index = index as usize;
                if missing_normals[index] {
                    let accumulated = generated
                        .entry(missing_normal_keys[index])
                        .or_insert([0.0; 3]);
                    for (axis, component) in face_normal.iter().copied().enumerate() {
                        accumulated[axis] += component;
                    }
                }
            }
        }
        for (index, &missing) in missing_normals.iter().enumerate() {
            if missing {
                let normal = generated
                    .get(&missing_normal_keys[index])
                    .copied()
                    .unwrap_or([0.0; 3]);
                let length = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
                mesh.normals[index] = if length.is_finite() && length > 1.0e-8 {
                    normal.map(|value| value / length)
                } else {
                    [0.0, 0.0, 1.0]
                };
            }
        }
    }
    mesh.update_bounds();

    Ok(mesh)
}

fn resolve_obj_index(raw: &str, element_count: usize) -> Result<usize, String> {
    let parsed: i32 = raw
        .parse()
        .map_err(|_| format!("invalid OBJ index `{raw}`"))?;
    let resolved = if parsed < 0 {
        element_count.checked_sub(parsed.unsigned_abs() as usize)
    } else if parsed > 0 {
        Some(parsed as usize - 1)
    } else {
        None
    };
    resolved
        .filter(|&index| index < element_count)
        .ok_or_else(|| format!("OBJ index `{raw}` is out of range"))
}

fn ray_triangle_intersect(
    orig: [f32; 3],
    dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<f32> {
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let pvec = [
        dir[1] * e2[2] - dir[2] * e2[1],
        dir[2] * e2[0] - dir[0] * e2[2],
        dir[0] * e2[1] - dir[1] * e2[0],
    ];

    let det = e1[0] * pvec[0] + e1[1] * pvec[1] + e1[2] * pvec[2];
    if det.abs() < 1e-6 {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = [orig[0] - v0[0], orig[1] - v0[1], orig[2] - v0[2]];
    let u = (tvec[0] * pvec[0] + tvec[1] * pvec[1] + tvec[2] * pvec[2]) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = [
        tvec[1] * e1[2] - tvec[2] * e1[1],
        tvec[2] * e1[0] - tvec[0] * e1[2],
        tvec[0] * e1[1] - tvec[1] * e1[0],
    ];

    let v = (dir[0] * qvec[0] + dir[1] * qvec[1] + dir[2] * qvec[2]) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = (e2[0] * qvec[0] + e2[1] * qvec[1] + e2[2] * qvec[2]) * inv_det;
    if t > 1e-5 {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_obj_cube() {
        let obj_data = r#"
# Simple unit quad
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
f 1 2 3 4
"#;
        let mesh = parse_obj_str(obj_data).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6); // 2 triangles for quad
        assert_eq!(mesh.normals.len(), 4);
        assert_eq!(mesh.bbox_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bbox_max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn test_ray_mesh_intersection() {
        let obj_data = r#"
v -1.0 -1.0 0.0
v  1.0 -1.0 0.0
v  0.0  1.0 0.0
f 1 2 3
"#;
        let mesh = parse_obj_str(obj_data).unwrap();
        let hit = mesh.ray_intersect([0.0, 0.0, -5.0], [0.0, 0.0, 1.0]);
        assert!(hit.is_some());
        let (t, tri) = hit.unwrap();
        assert!((t - 5.0).abs() < 1e-3);
        assert_eq!(tri, 0);
    }

    #[test]
    fn test_obj_negative_indices_reference_previous_vertices() {
        let mesh = parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_obj_invalid_face_index_is_rejected() {
        assert!(parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 9\n").is_err());
    }
}
