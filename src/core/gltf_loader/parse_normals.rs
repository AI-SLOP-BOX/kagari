use super::*;
use std::collections::HashMap;

pub(super) fn generate_missing_normals(
    mesh: &mut Mesh3D,
    missing_normal_flags: &[bool],
    normal_smoothing_keys: &[(usize, [u32; 3])],
) -> Result<(), String> {
    if missing_normal_flags.iter().any(|&missing| missing) {
        let mut accumulated_normals: HashMap<(usize, [u32; 3]), [f32; 3]> = HashMap::new();
        let missing_count = missing_normal_flags
            .iter()
            .filter(|&&missing| missing)
            .count();
        accumulated_normals
            .try_reserve(missing_count)
            .map_err(|error| format!("Could not reserve generated glTF normals: {error}"))?;
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
            for &vertex_index in triangle {
                let index = vertex_index as usize;
                if missing_normal_flags[index] {
                    let key = normal_smoothing_keys[index];
                    let normal = accumulated_normals.entry(key).or_insert([0.0; 3]);
                    for (axis, component) in face_normal.iter().copied().enumerate() {
                        normal[axis] += component;
                    }
                }
            }
        }
        for (index, is_missing) in missing_normal_flags.iter().copied().enumerate() {
            if is_missing {
                let normal = accumulated_normals
                    .get(&normal_smoothing_keys[index])
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
    Ok(())
}
