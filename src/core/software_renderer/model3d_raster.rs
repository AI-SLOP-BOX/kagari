//! Minimal, deterministic OBJ rasterization for imported 3D model layers.
//!
//! Model layers intentionally use the software renderer. This keeps the CPU
//! export path and the interactive preview on one implementation while the
//! GPU pipeline does not yet have a mesh vertex buffer.

use super::mask::compute_combined_mask_coverage;
use super::raster::RasterCtx;
use crate::core::timeline::{Layer, LightType};
use std::collections::HashMap;

mod cache;
mod shading;
use cache::{clear_texture_load_error, load_mesh, report_texture_load_error_once};
use shading::{sample_texture, shade};

fn rotate_xyz(point: [f32; 3], rotation_deg: [f32; 3]) -> [f32; 3] {
    let [rx, ry, rz] = rotation_deg.map(f32::to_radians);
    let (sin_x, cos_x) = rx.sin_cos();
    let (sin_y, cos_y) = ry.sin_cos();
    let (sin_z, cos_z) = rz.sin_cos();
    let y1 = point[1] * cos_x - point[2] * sin_x;
    let z1 = point[1] * sin_x + point[2] * cos_x;
    let x2 = point[0] * cos_y + z1 * sin_y;
    let z2 = -point[0] * sin_y + z1 * cos_y;
    [x2 * cos_z - y1 * sin_z, x2 * sin_z + y1 * cos_z, z2]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = vector[0]
        .mul_add(
            vector[0],
            vector[1].mul_add(vector[1], vector[2] * vector[2]),
        )
        .sqrt()
        .max(1.0e-6);
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
}

#[derive(Clone, Copy, Default)]
struct ModelVertex {
    world: [f32; 3],
    view: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    has_normal: bool,
}

fn clip_triangle_to_near_plane(
    triangle: [ModelVertex; 3],
    near_z: f32,
) -> ([[ModelVertex; 3]; 2], usize) {
    let mut polygon = [ModelVertex::default(); 4];
    let mut polygon_len = 0;
    let mut previous = triangle[2];
    let mut previous_inside = previous.view[2] >= near_z;
    for current in triangle {
        let current_inside = current.view[2] >= near_z;
        if current_inside != previous_inside {
            let t = ((near_z - previous.view[2]) / (current.view[2] - previous.view[2]))
                .clamp(0.0, 1.0);
            let mut intersection = ModelVertex {
                world: std::array::from_fn(|axis| {
                    previous.world[axis] + (current.world[axis] - previous.world[axis]) * t
                }),
                view: [
                    previous.view[0] + (current.view[0] - previous.view[0]) * t,
                    previous.view[1] + (current.view[1] - previous.view[1]) * t,
                    near_z,
                ],
                normal: [
                    previous.normal[0] + (current.normal[0] - previous.normal[0]) * t,
                    previous.normal[1] + (current.normal[1] - previous.normal[1]) * t,
                    previous.normal[2] + (current.normal[2] - previous.normal[2]) * t,
                ],
                uv: [
                    previous.uv[0] + (current.uv[0] - previous.uv[0]) * t,
                    previous.uv[1] + (current.uv[1] - previous.uv[1]) * t,
                ],
                color: std::array::from_fn(|channel| {
                    previous.color[channel] + (current.color[channel] - previous.color[channel]) * t
                }),
                has_normal: previous.has_normal && current.has_normal,
            };
            if intersection.has_normal {
                intersection.normal = normalize(intersection.normal);
            }
            polygon[polygon_len] = intersection;
            polygon_len += 1;
        }
        if current_inside {
            polygon[polygon_len] = current;
            polygon_len += 1;
        }
        previous = current;
        previous_inside = current_inside;
    }

    let mut triangles = [[ModelVertex::default(); 3]; 2];
    if polygon_len >= 3 {
        triangles[0] = [polygon[0], polygon[1], polygon[2]];
        if polygon_len == 4 {
            triangles[1] = [polygon[0], polygon[2], polygon[3]];
            (triangles, 2)
        } else {
            (triangles, 1)
        }
    } else {
        (triangles, 0)
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
mod lighting_tests;

pub(crate) fn rasterize_model3d_layer(ctx: RasterCtx<'_>) {
    let RasterCtx {
        comp,
        layer,
        frame,
        masks,
        min_x,
        min_y,
        max_x,
        max_y,
        bw,
        bh,
        width,
        height,
        base_color,
        layer_buf,
        ..
    } = ctx;
    let crate::core::timeline::LayerType::Model3D { path } = &layer.layer_type else {
        return;
    };
    let Some(mesh) = load_mesh(path) else {
        return;
    };
    if mesh.vertices.is_empty() || mesh.indices.len() < 3 {
        return;
    }

    let center = [
        (mesh.bbox_min[0] + mesh.bbox_max[0]) * 0.5,
        (mesh.bbox_min[1] + mesh.bbox_max[1]) * 0.5,
        (mesh.bbox_min[2] + mesh.bbox_max[2]) * 0.5,
    ];
    let extent = (mesh.bbox_max[0] - mesh.bbox_min[0])
        .max(mesh.bbox_max[1] - mesh.bbox_min[1])
        .max(mesh.bbox_max[2] - mesh.bbox_min[2])
        .max(1.0e-4);
    let scale = (width.min(height) as f32 * 0.72 / extent).max(0.001);
    let position = layer.transform_3d.position.evaluate(frame);
    let rotation = layer.transform_3d.rotation.evaluate(frame);
    let model_scale = layer
        .transform_3d
        .scale
        .evaluate(frame)
        .map(|value| value / 100.0);
    let camera = comp.resolve_camera_at();
    let camera_view = crate::core::timeline::camera_view_transform(&camera, frame, 1.0);
    if camera_view.iter().flatten().any(|value| !value.is_finite()) {
        return;
    }
    let Some(mut world_positions) = super::try_vec_with_capacity(mesh.vertices.len()) else {
        log::error!(
            "[Renderer] Could not allocate world vertices for model {}",
            layer.id
        );
        return;
    };
    let Some(mut view_positions) = super::try_vec_with_capacity(mesh.vertices.len()) else {
        log::error!(
            "[Renderer] Could not allocate view vertices for model {}",
            layer.id
        );
        return;
    };
    for vertex in &mesh.vertices {
        let local = [
            (vertex[0] - center[0]) * scale * model_scale[0],
            (vertex[1] - center[1]) * scale * model_scale[1],
            (vertex[2] - center[2]) * scale * model_scale[2],
        ];
        let rotated = rotate_xyz(local, rotation);
        let world_point = [
            position[0] + rotated[0],
            position[1] + rotated[1],
            position[2] + rotated[2],
        ];
        if world_point.iter().any(|value| !value.is_finite()) {
            world_positions.push(None);
            view_positions.push(None);
            continue;
        }
        let view_point: [f32; 3] = std::array::from_fn(|row| {
            camera_view[0][row] * world_point[0]
                + camera_view[1][row] * world_point[1]
                + camera_view[2][row] * world_point[2]
                + camera_view[3][row]
        });
        if view_point.iter().any(|value| !value.is_finite()) {
            world_positions.push(None);
            view_positions.push(None);
            continue;
        }
        world_positions.push(Some(world_point));
        view_positions.push(Some(view_point));
    }
    // Imported vertex normals are in mesh-local space. Apply the inverse
    // non-uniform scale before rotation so lighting follows the transformed
    // surface instead of the untransformed mesh.
    let Some(mut transformed_normals) = super::try_vec_with_capacity(mesh.normals.len()) else {
        log::error!(
            "[Renderer] Could not allocate transformed normals for model {}",
            layer.id
        );
        return;
    };
    for normal in &mesh.normals {
        let inverse_scaled = [
            normal[0] / model_scale[0].abs().max(1.0e-6) * model_scale[0].signum(),
            normal[1] / model_scale[1].abs().max(1.0e-6) * model_scale[1].signum(),
            normal[2] / model_scale[2].abs().max(1.0e-6) * model_scale[2].signum(),
        ];
        transformed_normals.push(normalize(rotate_xyz(inverse_scaled, rotation)));
    }
    let mut depth = vec![f32::INFINITY; (bw * bh) as usize];
    let mut material_textures = HashMap::new();
    for (material_index, material) in mesh.materials.iter().enumerate() {
        if let Some(image) = material
            .albedo_texture_index
            .and_then(|index| mesh.embedded_textures.get(index))
            .filter(|image| image.width > 0)
        {
            material_textures.insert(material_index, image.clone());
            continue;
        }
        if let Some(path) = material.albedo_texture.as_deref() {
            if let Some(texture) = crate::core::image_cache::load_image_shared(path) {
                material_textures.insert(material_index, texture);
                clear_texture_load_error(path);
            } else {
                report_texture_load_error_once(path);
            }
        }
    }
    let source_triangles = mesh.indices.as_chunks::<3>().0;
    let has_blend_material = mesh.material_indices.iter().any(|&material_index| {
        mesh.materials
            .get(material_index)
            .is_some_and(|material| material.alpha_mode == "BLEND")
    });
    let mut triangle_order = (0..source_triangles.len()).collect::<Vec<_>>();
    if has_blend_material {
        triangle_order.sort_by(|&a, &b| {
            let depth_of = |index: usize| {
                let triangle = source_triangles[index];
                let (Some(a), Some(b), Some(c)) = (
                    view_positions.get(triangle[0] as usize).copied().flatten(),
                    view_positions.get(triangle[1] as usize).copied().flatten(),
                    view_positions.get(triangle[2] as usize).copied().flatten(),
                ) else {
                    return f32::NEG_INFINITY;
                };
                (a[2] + b[2] + c[2]) / 3.0
            };
            depth_of(b).total_cmp(&depth_of(a))
        });
    }
    let pass_count = if has_blend_material { 2 } else { 1 };
    for pass in 0..pass_count {
        if super::render_cancelled() {
            return;
        }
        let blend_pass = pass == 1;
        for &triangle_index in &triangle_order {
            if super::render_cancelled() {
                return;
            }
            let triangle = &source_triangles[triangle_index];
            let triangle_material = mesh
                .material_indices
                .get(triangle_index)
                .and_then(|&material_index| mesh.materials.get(material_index));
            let uv_set = triangle_material
                .filter(|material| material.texture_tex_coord > 0)
                .and_then(|material| mesh.uv_sets.get(material.texture_tex_coord as usize))
                .unwrap_or(&mesh.uvs);
            let is_blend = mesh
                .material_indices
                .get(triangle_index)
                .and_then(|&material_index| mesh.materials.get(material_index))
                .is_some_and(|material| material.alpha_mode == "BLEND");
            if has_blend_material && is_blend != blend_pass {
                continue;
            }
            let (Some(wa), Some(wb), Some(wc), Some(va), Some(vb), Some(vc)) = (
                world_positions.get(triangle[0] as usize).copied().flatten(),
                world_positions.get(triangle[1] as usize).copied().flatten(),
                world_positions.get(triangle[2] as usize).copied().flatten(),
                view_positions.get(triangle[0] as usize).copied().flatten(),
                view_positions.get(triangle[1] as usize).copied().flatten(),
                view_positions.get(triangle[2] as usize).copied().flatten(),
            ) else {
                continue;
            };
            let vertex_normals = [
                transformed_normals.get(triangle[0] as usize),
                transformed_normals.get(triangle[1] as usize),
                transformed_normals.get(triangle[2] as usize),
            ];
            let has_normals = vertex_normals.iter().all(Option::is_some);
            let triangle_colors = [
                mesh.colors
                    .get(triangle[0] as usize)
                    .copied()
                    .unwrap_or([1.0; 4]),
                mesh.colors
                    .get(triangle[1] as usize)
                    .copied()
                    .unwrap_or([1.0; 4]),
                mesh.colors
                    .get(triangle[2] as usize)
                    .copied()
                    .unwrap_or([1.0; 4]),
            ];
            let model_triangle = [
                ModelVertex {
                    world: wa,
                    view: va,
                    normal: vertex_normals[0].copied().unwrap_or_default(),
                    uv: uv_set
                        .get(triangle[0] as usize)
                        .copied()
                        .unwrap_or_default(),
                    color: triangle_colors[0],
                    has_normal: has_normals,
                },
                ModelVertex {
                    world: wb,
                    view: vb,
                    normal: vertex_normals[1].copied().unwrap_or_default(),
                    uv: uv_set
                        .get(triangle[1] as usize)
                        .copied()
                        .unwrap_or_default(),
                    color: triangle_colors[1],
                    has_normal: has_normals,
                },
                ModelVertex {
                    world: wc,
                    view: vc,
                    normal: vertex_normals[2].copied().unwrap_or_default(),
                    uv: uv_set
                        .get(triangle[2] as usize)
                        .copied()
                        .unwrap_or_default(),
                    color: triangle_colors[2],
                    has_normal: has_normals,
                },
            ];
            let near_depth = (height as f32 * 0.05 + 0.101).max(0.101);
            let (clipped_triangles, clipped_count) =
                clip_triangle_to_near_plane(model_triangle, near_depth);
            for clipped_triangle in clipped_triangles.into_iter().take(clipped_count) {
                let projected_triangle = clipped_triangle.map(|vertex| {
                    crate::core::timeline::project_point_to_screen_at_frame(
                        &camera,
                        vertex.world,
                        width as f32,
                        height as f32,
                        frame,
                    )
                    .filter(|point| point.iter().all(|value| value.is_finite()))
                    .map(|point| {
                        (
                            point[0] * width as f32,
                            point[1] * height as f32,
                            vertex.world,
                        )
                    })
                });
                let [Some(a), Some(b), Some(c)] = projected_triangle else {
                    continue;
                };
                let area = edge([a.0, a.1], [b.0, b.1], [c.0, c.1]);
                if !area.is_finite() || area.abs() < 0.001 {
                    continue;
                }
                let min_px = a.0.min(b.0).min(c.0).floor().max(min_x as f32) as u32;
                let max_px =
                    a.0.max(b.0)
                        .max(c.0)
                        .ceil()
                        .min(max_x.saturating_sub(1) as f32) as u32;
                let min_py = a.1.min(b.1).min(c.1).floor().max(min_y as f32) as u32;
                let max_py =
                    a.1.max(b.1)
                        .max(c.1)
                        .ceil()
                        .min(max_y.saturating_sub(1) as f32) as u32;
                if min_px > max_px || min_py > max_py {
                    continue;
                }
                let normal = normalize(cross(
                    sub([b.2[0], b.2[1], b.2[2]], a.2),
                    sub([c.2[0], c.2[1], c.2[2]], a.2),
                ));
                let imported_material = mesh
                    .material_indices
                    .get(triangle_index)
                    .and_then(|&material_index| mesh.materials.get(material_index));
                let mut material_color = base_color;
                let mut imported_emission = [0.0; 3];
                let mut alpha_mode = "OPAQUE";
                let mut alpha_cutoff = 0.5;
                if let Some(imported_material) = imported_material {
                    for (channel, factor) in material_color
                        .iter_mut()
                        .zip(imported_material.albedo_color)
                    {
                        *channel *= factor.clamp(0.0, 1.0);
                    }
                    imported_emission = imported_material.emission;
                    alpha_mode = imported_material.alpha_mode.as_str();
                    alpha_cutoff = imported_material.alpha_cutoff;
                }
                if alpha_mode == "MASK" && material_color[3] < alpha_cutoff {
                    continue;
                }
                if blend_pass && material_color[3] <= 0.0 {
                    continue;
                }
                let clipped_normals =
                    clipped_triangle.map(|vertex| vertex.has_normal.then_some(vertex.normal));
                for py in min_py..=max_py {
                    if super::render_cancelled() {
                        return;
                    }
                    for px in min_px..=max_px {
                        let sample = [px as f32 + 0.5, py as f32 + 0.5];
                        let weights = [
                            edge([b.0, b.1], [c.0, c.1], sample) / area,
                            edge([c.0, c.1], [a.0, a.1], sample) / area,
                            edge([a.0, a.1], [b.0, b.1], sample) / area,
                        ];
                        if weights.iter().any(|weight| *weight < -0.001) {
                            continue;
                        }
                        let mask_alpha = if masks.is_empty() {
                            1.0
                        } else {
                            compute_combined_mask_coverage(px as f32 + 0.5, py as f32 + 0.5, masks)
                        };
                        if mask_alpha <= 0.001 {
                            continue;
                        }
                        let camera_depths = [
                            clipped_triangle[0].view[2],
                            clipped_triangle[1].view[2],
                            clipped_triangle[2].view[2],
                        ];
                        if camera_depths.iter().any(|depth| *depth <= 0.1) {
                            continue;
                        }
                        let inverse_depth = weights[0] / camera_depths[0]
                            + weights[1] / camera_depths[1]
                            + weights[2] / camera_depths[2];
                        if !inverse_depth.is_finite() || inverse_depth <= 0.0 {
                            continue;
                        }
                        let perspective_weights = [
                            weights[0] / camera_depths[0] / inverse_depth,
                            weights[1] / camera_depths[1] / inverse_depth,
                            weights[2] / camera_depths[2] / inverse_depth,
                        ];
                        let view_depth = 1.0 / inverse_depth;
                        let depth_index = ((py - min_y) * bw + (px - min_x)) as usize;
                        if view_depth >= depth[depth_index] {
                            continue;
                        }
                        if !blend_pass {
                            depth[depth_index] = view_depth;
                        }
                        let world_position = [
                            perspective_weights[0] * a.2[0]
                                + perspective_weights[1] * b.2[0]
                                + perspective_weights[2] * c.2[0],
                            perspective_weights[0] * a.2[1]
                                + perspective_weights[1] * b.2[1]
                                + perspective_weights[2] * c.2[1],
                            perspective_weights[0] * a.2[2]
                                + perspective_weights[1] * b.2[2]
                                + perspective_weights[2] * c.2[2],
                        ];
                        let mut uv = [
                            perspective_weights[0] * clipped_triangle[0].uv[0]
                                + perspective_weights[1] * clipped_triangle[1].uv[0]
                                + perspective_weights[2] * clipped_triangle[2].uv[0],
                            perspective_weights[0] * clipped_triangle[0].uv[1]
                                + perspective_weights[1] * clipped_triangle[1].uv[1]
                                + perspective_weights[2] * clipped_triangle[2].uv[1],
                        ];
                        if let Some(material) = imported_material {
                            let scaled = [
                                uv[0] * material.texture_uv_scale[0],
                                uv[1] * material.texture_uv_scale[1],
                            ];
                            let (sin_rotation, cos_rotation) =
                                material.texture_uv_rotation.sin_cos();
                            uv = [
                                cos_rotation * scaled[0] - sin_rotation * scaled[1]
                                    + material.texture_uv_offset[0],
                                sin_rotation * scaled[0]
                                    + cos_rotation * scaled[1]
                                    + material.texture_uv_offset[1],
                            ];
                        }
                        let texture = mesh
                            .material_indices
                            .get(triangle_index)
                            .and_then(|&material_index| material_textures.get(&material_index));
                        let texel = texture.map_or([1.0; 4], |image| {
                            let uv_at = |sample: [f32; 2]| {
                                let sample_weights = [
                                    edge([b.0, b.1], [c.0, c.1], sample) / area,
                                    edge([c.0, c.1], [a.0, a.1], sample) / area,
                                    edge([a.0, a.1], [b.0, b.1], sample) / area,
                                ];
                                let inverse = sample_weights[0] / camera_depths[0]
                                    + sample_weights[1] / camera_depths[1]
                                    + sample_weights[2] / camera_depths[2];
                                if !inverse.is_finite() || inverse.abs() <= 1.0e-8 {
                                    return uv;
                                }
                                let weights = [
                                    sample_weights[0] / camera_depths[0] / inverse,
                                    sample_weights[1] / camera_depths[1] / inverse,
                                    sample_weights[2] / camera_depths[2] / inverse,
                                ];
                                let mut sample_uv = [
                                    weights[0] * clipped_triangle[0].uv[0]
                                        + weights[1] * clipped_triangle[1].uv[0]
                                        + weights[2] * clipped_triangle[2].uv[0],
                                    weights[0] * clipped_triangle[0].uv[1]
                                        + weights[1] * clipped_triangle[1].uv[1]
                                        + weights[2] * clipped_triangle[2].uv[1],
                                ];
                                if let Some(material) = imported_material {
                                    let scaled = [
                                        sample_uv[0] * material.texture_uv_scale[0],
                                        sample_uv[1] * material.texture_uv_scale[1],
                                    ];
                                    let (sin_rotation, cos_rotation) =
                                        material.texture_uv_rotation.sin_cos();
                                    sample_uv = [
                                        cos_rotation * scaled[0] - sin_rotation * scaled[1]
                                            + material.texture_uv_offset[0],
                                        sin_rotation * scaled[0]
                                            + cos_rotation * scaled[1]
                                            + material.texture_uv_offset[1],
                                    ];
                                }
                                sample_uv
                            };
                            let uv_dx = uv_at([px as f32 + 1.5, py as f32 + 0.5]);
                            let uv_dy = uv_at([px as f32 + 0.5, py as f32 + 1.5]);
                            let texel_dx = (((uv_dx[0] - uv[0]) * image.width as f32).powi(2)
                                + ((uv_dx[1] - uv[1]) * image.height as f32).powi(2))
                            .sqrt();
                            let texel_dy = (((uv_dy[0] - uv[0]) * image.width as f32).powi(2)
                                + ((uv_dy[1] - uv[1]) * image.height as f32).powi(2))
                            .sqrt();
                            let minifying = texel_dx.max(texel_dy) > 1.0;
                            let nearest = imported_material.is_some_and(|material| {
                                if minifying {
                                    material.texture_min_nearest
                                } else {
                                    material.texture_mag_nearest
                                }
                            });
                            sample_texture(
                                image,
                                uv,
                                imported_material.map_or(10497, |material| material.texture_wrap_s),
                                imported_material.map_or(10497, |material| material.texture_wrap_t),
                                nearest,
                            )
                        });
                        let vertex_color: [f32; 4] = std::array::from_fn(|channel| {
                            perspective_weights[0] * clipped_triangle[0].color[channel]
                                + perspective_weights[1] * clipped_triangle[1].color[channel]
                                + perspective_weights[2] * clipped_triangle[2].color[channel]
                        });
                        let mut fragment_color = material_color;
                        for channel in 0..3 {
                            fragment_color[channel] *= vertex_color[channel] * texel[channel];
                        }
                        fragment_color[3] *= vertex_color[3] * texel[3];
                        match alpha_mode {
                            "OPAQUE" => fragment_color[3] = 1.0,
                            "MASK" => {
                                if fragment_color[3] < alpha_cutoff {
                                    continue;
                                }
                                fragment_color[3] = 1.0;
                            }
                            "BLEND" => {}
                            _ => unreachable!("material alpha mode is validated during import"),
                        }
                        if blend_pass && fragment_color[3] <= 0.0 {
                            continue;
                        }
                        let surface_normal = if let [Some(na), Some(nb), Some(nc)] = clipped_normals
                        {
                            normalize([
                                perspective_weights[0] * na[0]
                                    + perspective_weights[1] * nb[0]
                                    + perspective_weights[2] * nc[0],
                                perspective_weights[0] * na[1]
                                    + perspective_weights[1] * nb[1]
                                    + perspective_weights[2] * nc[1],
                                perspective_weights[0] * na[2]
                                    + perspective_weights[1] * nb[2]
                                    + perspective_weights[2] * nc[2],
                            ])
                        } else {
                            normal
                        };
                        let color = shade(
                            layer,
                            comp,
                            world_position,
                            surface_normal,
                            fragment_color,
                            imported_material,
                            imported_emission,
                            frame,
                        );
                        let pixel_index = depth_index * 4;
                        if blend_pass {
                            let source_alpha = f32::from(color[3]) / 255.0 * mask_alpha;
                            let destination_alpha = f32::from(layer_buf[pixel_index + 3]) / 255.0;
                            let output_alpha =
                                source_alpha + destination_alpha * (1.0 - source_alpha);
                            if output_alpha > 0.0 {
                                for channel in 0..3 {
                                    let source = f32::from(color[channel]) * source_alpha;
                                    let destination = f32::from(layer_buf[pixel_index + channel])
                                        * destination_alpha
                                        * (1.0 - source_alpha);
                                    layer_buf[pixel_index + channel] =
                                        ((source + destination) / output_alpha).clamp(0.0, 255.0)
                                            as u8;
                                }
                                layer_buf[pixel_index + 3] = (output_alpha * 255.0) as u8;
                            }
                        } else {
                            layer_buf[pixel_index] = color[0];
                            layer_buf[pixel_index + 1] = color[1];
                            layer_buf[pixel_index + 2] = color[2];
                            layer_buf[pixel_index + 3] = (f32::from(color[3]) * mask_alpha) as u8;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
