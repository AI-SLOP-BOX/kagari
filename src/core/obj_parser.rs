//! Wavefront OBJ 3D Mesh Parser & Importer.
//!
//! Loads 3D meshes with vertices, normals, and texture coordinates for 3D compositing layers.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh3DVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh3DTriangle {
    pub vertices: [Mesh3DVertex; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Mesh3DModel {
    pub name: String,
    pub triangles: Vec<Mesh3DTriangle>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

fn resolve_obj_index(raw: &str, item_count: usize, kind: &str) -> Result<usize, String> {
    let parsed: i32 = raw
        .parse()
        .map_err(|error| format!("{kind} index: {error}"))?;
    if parsed == 0 {
        return Err(format!("{kind} index cannot be zero"));
    }
    let index = if parsed > 0 {
        parsed as usize - 1
    } else {
        item_count
            .checked_sub(parsed.unsigned_abs() as usize)
            .ok_or_else(|| format!("{kind} index out of range: {raw}"))?
    };
    if index >= item_count {
        return Err(format!("{kind} index out of range: {raw}"));
    }
    Ok(index)
}

impl Mesh3DModel {
    /// Parses a Wavefront OBJ string format.
    pub fn parse_obj(obj_str: &str) -> Result<Self, String> {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut triangles: Vec<Mesh3DTriangle> = Vec::new();

        let mut bounds_min = [f32::INFINITY; 3];
        let mut bounds_max = [f32::NEG_INFINITY; 3];

        for line in obj_str.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().map_err(|e| format!("Invalid v.x: {e}"))?;
                        let y: f32 = parts[2].parse().map_err(|e| format!("Invalid v.y: {e}"))?;
                        let z: f32 = parts[3].parse().map_err(|e| format!("Invalid v.z: {e}"))?;
                        positions.push([x, y, z]);

                        bounds_min[0] = bounds_min[0].min(x);
                        bounds_min[1] = bounds_min[1].min(y);
                        bounds_min[2] = bounds_min[2].min(z);

                        bounds_max[0] = bounds_max[0].max(x);
                        bounds_max[1] = bounds_max[1].max(y);
                        bounds_max[2] = bounds_max[2].max(z);
                    }
                }
                "vn" => {
                    if parts.len() >= 4 {
                        let nx: f32 = parts[1].parse().map_err(|e| format!("Invalid vn.x: {e}"))?;
                        let ny: f32 = parts[2].parse().map_err(|e| format!("Invalid vn.y: {e}"))?;
                        let nz: f32 = parts[3].parse().map_err(|e| format!("Invalid vn.z: {e}"))?;
                        normals.push([nx, ny, nz]);
                    }
                }
                "vt" => {
                    if parts.len() >= 3 {
                        let u: f32 = parts[1].parse().map_err(|e| format!("Invalid vt.u: {e}"))?;
                        let v: f32 = parts[2].parse().map_err(|e| format!("Invalid vt.v: {e}"))?;
                        uvs.push([u, v]);
                    }
                }
                "f" => {
                    if parts.len() >= 4 {
                        let parse_vertex = |spec: &str| -> Result<Mesh3DVertex, String> {
                            let tokens: Vec<&str> = spec.split('/').collect();
                            let pos_idx = resolve_obj_index(tokens[0], positions.len(), "Vertex")?;
                            let pos = positions[pos_idx];

                            let uv = if tokens.len() > 1 && !tokens[1].is_empty() {
                                let uv_idx = resolve_obj_index(tokens[1], uvs.len(), "UV")?;
                                uvs[uv_idx]
                            } else {
                                [0.0, 0.0]
                            };

                            let normal = if tokens.len() > 2 && !tokens[2].is_empty() {
                                let norm_idx =
                                    resolve_obj_index(tokens[2], normals.len(), "Normal")?;
                                normals[norm_idx]
                            } else {
                                [0.0, 0.0, 1.0]
                            };

                            Ok(Mesh3DVertex {
                                position: pos,
                                normal,
                                uv,
                            })
                        };

                        let face_vertices: Vec<Mesh3DVertex> = parts[1..]
                            .iter()
                            .map(|spec| parse_vertex(spec))
                            .collect::<Result<_, _>>()?;
                        for triangle_index in 1..face_vertices.len().saturating_sub(1) {
                            triangles.push(Mesh3DTriangle {
                                vertices: [
                                    face_vertices[0].clone(),
                                    face_vertices[triangle_index].clone(),
                                    face_vertices[triangle_index + 1].clone(),
                                ],
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if positions.is_empty() {
            bounds_min = [0.0, 0.0, 0.0];
            bounds_max = [0.0, 0.0, 0.0];
        }

        Ok(Self {
            name: "Imported Mesh".into(),
            triangles,
            bounds_min,
            bounds_max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_obj_triangle() {
        let obj_data = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
";
        let mesh = Mesh3DModel::parse_obj(obj_data).expect("Parsing OBJ succeeds");
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.triangles[0].vertices[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(mesh.bounds_max[0], 1.0);
    }

    #[test]
    fn test_parse_negative_indices_and_ngon_fan() {
        let obj_data = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v -0.5 0.5 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
vt 0.5 0.5
vn 0.0 0.0 1.0
f -5/-5/-1 -4/-4/-1 -3/-3/-1 -2/-2/-1 -1/-1/-1
";
        let mesh = Mesh3DModel::parse_obj(obj_data).expect("negative OBJ indices should parse");
        assert_eq!(mesh.triangles.len(), 3);
        assert_eq!(mesh.triangles[0].vertices[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.triangles[2].vertices[2].position, [-0.5, 0.5, 0.0]);
        assert_eq!(mesh.triangles[1].vertices[2].uv, [0.0, 1.0]);
    }

    #[test]
    fn test_parse_rejects_zero_or_out_of_range_indices() {
        let zero = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 2 3\n";
        let out_of_range = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 -4\n";
        assert!(Mesh3DModel::parse_obj(zero).is_err());
        assert!(Mesh3DModel::parse_obj(out_of_range).is_err());
    }
}
