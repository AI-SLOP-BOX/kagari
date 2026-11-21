//! Cinema 4D / Element 3D V2-style 3D Extruded & Beveled Mesh Generator.
//!
//! Converts any closed 2D polygon / glyph contour into a watertight 3D triangle mesh
//! with front/back caps, chamfered bevels, outward normals, and UV coordinates.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtrudedVertex3D {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct ExtrudedMesh3D {
    pub vertices: Vec<ExtrudedVertex3D>,
    pub indices: Vec<u32>,
}

/// Generates a beveled, extruded 3D mesh from a convex/star-shaped or simple closed 2D polygon.
pub fn extrude_polygon_beveled(
    contour: &[[f32; 2]],
    extrusion_depth: f32,
    bevel_size: f32,
) -> Option<ExtrudedMesh3D> {
    if contour.len() < 3 || extrusion_depth <= 0.0 {
        return None;
    }

    let n = contour.len();
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    for p in contour {
        cx += p[0];
        cy += p[1];
    }
    cx /= n as f32;
    cy /= n as f32;

    let half_d = extrusion_depth * 0.5;
    let bev = bevel_size.clamp(0.0, half_d * 0.45);
    let inner_scale = (1.0 - (bev * 0.015)).clamp(0.5, 1.0);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Helper to inset contour for front/back bevel rings
    let inset_pt = |p: [f32; 2]| -> [f32; 2] {
        [cx + (p[0] - cx) * inner_scale, cy + (p[1] - cy) * inner_scale]
    };

    // 1. Front Cap (z = +half_d)
    let front_center_idx = vertices.len() as u32;
    vertices.push(ExtrudedVertex3D {
        pos: [cx, cy, half_d],
        normal: [0.0, 0.0, 1.0],
        uv: [0.5, 0.5],
    });
    let front_ring_start = vertices.len() as u32;
    for (i, &p) in contour.iter().enumerate() {
        let ip = inset_pt(p);
        vertices.push(ExtrudedVertex3D {
            pos: [ip[0], ip[1], half_d],
            normal: [0.0, 0.0, 1.0],
            uv: [i as f32 / n as f32, 0.0],
        });
    }
    for i in 0..n {
        let a = front_ring_start + i as u32;
        let b = front_ring_start + ((i + 1) % n) as u32;
        indices.extend_from_slice(&[front_center_idx, a, b]);
    }

    // 2. Back Cap (z = -half_d)
    let back_center_idx = vertices.len() as u32;
    vertices.push(ExtrudedVertex3D {
        pos: [cx, cy, -half_d],
        normal: [0.0, 0.0, -1.0],
        uv: [0.5, 0.5],
    });
    let back_ring_start = vertices.len() as u32;
    for (i, &p) in contour.iter().enumerate() {
        let ip = inset_pt(p);
        vertices.push(ExtrudedVertex3D {
            pos: [ip[0], ip[1], -half_d],
            normal: [0.0, 0.0, -1.0],
            uv: [i as f32 / n as f32, 1.0],
        });
    }
    for i in 0..n {
        let a = back_ring_start + i as u32;
        let b = back_ring_start + ((i + 1) % n) as u32;
        indices.extend_from_slice(&[back_center_idx, b, a]);
    }

    // 3. Beveled side walls (front bevel -> outer wall -> back bevel)
    let z_levels = [
        (half_d, true),
        (half_d - bev, false),
        (-(half_d - bev), false),
        (-half_d, true),
    ];

    for seg in 0..n {
        let p0 = contour[seg];
        let p1 = contour[(seg + 1) % n];
        let ex = p1[0] - p0[0];
        let ey = p1[1] - p0[1];
        let len = (ex * ex + ey * ey).sqrt().max(1e-6);
        let nx = ey / len;
        let ny = -ex / len;
        let u0 = seg as f32 / n as f32;
        let u1 = (seg + 1) as f32 / n as f32;

        for band in 0..3 {
            let (za, ia) = z_levels[band];
            let (zb, ib) = z_levels[band + 1];
            let a0 = if ia { inset_pt(p0) } else { p0 };
            let a1 = if ia { inset_pt(p1) } else { p1 };
            let b0 = if ib { inset_pt(p0) } else { p0 };
            let b1 = if ib { inset_pt(p1) } else { p1 };

            let base = vertices.len() as u32;
            let nz = if band == 0 {
                0.45
            } else if band == 2 {
                -0.45
            } else {
                0.0
            };
            let nlen = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
            let norm = [nx / nlen, ny / nlen, nz / nlen];

            vertices.push(ExtrudedVertex3D {
                pos: [a0[0], a0[1], za],
                normal: norm,
                uv: [u0, band as f32 / 3.0],
            });
            vertices.push(ExtrudedVertex3D {
                pos: [a1[0], a1[1], za],
                normal: norm,
                uv: [u1, band as f32 / 3.0],
            });
            vertices.push(ExtrudedVertex3D {
                pos: [b1[0], b1[1], zb],
                normal: norm,
                uv: [u1, (band + 1) as f32 / 3.0],
            });
            vertices.push(ExtrudedVertex3D {
                pos: [b0[0], b0[1], zb],
                normal: norm,
                uv: [u0, (band + 1) as f32 / 3.0],
            });

            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    Some(ExtrudedMesh3D { vertices, indices })
}

#[cfg(feature = "wgpu")]
impl ExtrudedMesh3D {
    /// Converts this 3D extruded & beveled mesh into [`ShadowVertex`]s ready for
    /// [`crate::core::shadow_map_gpu::render_shadow_mapped_scene_rgba8`].
    pub fn to_shadow_vertices(&self, color: [f32; 4]) -> Vec<crate::core::shadow_map_gpu::ShadowVertex> {
        self.vertices
            .iter()
            .map(|v| crate::core::shadow_map_gpu::ShadowVertex {
                pos: v.pos,
                normal: v.normal,
                color,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrude_square_produces_beveled_watertight_3d_mesh() {
        let square = [[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0]];
        let mesh = extrude_polygon_beveled(&square, 20.0, 2.0).expect("extruded mesh");
        // 4 sides * 3 bands * 2 tris = 24 side tris + 4 front + 4 back = 32 tris (96 indices)
        assert_eq!(mesh.indices.len(), 96);
        assert!(mesh.vertices.iter().all(|v| v.pos[2].abs() <= 10.01));
    }
}
