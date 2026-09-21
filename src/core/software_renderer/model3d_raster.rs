//! Minimal, deterministic OBJ rasterization for imported 3D model layers.
//!
//! Model layers intentionally use the software renderer. This keeps the CPU
//! export path and the interactive preview on one implementation while the
//! GPU pipeline does not yet have a mesh vertex buffer.

use super::mask::compute_combined_mask_coverage;
use super::raster::RasterCtx;
use crate::core::obj_loader::Mesh3D;
use crate::core::timeline::{Layer, LightType};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

type MeshCache = HashMap<String, (Option<SystemTime>, Arc<Mesh3D>)>;

fn mesh_cache() -> &'static Mutex<MeshCache> {
    static CACHE: OnceLock<Mutex<MeshCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_mesh(path: &str) -> Option<Arc<Mesh3D>> {
    let source = Path::new(path);
    let modified = std::fs::metadata(source).ok().and_then(|m| m.modified().ok());
    if let Ok(cache) = mesh_cache().lock() {
        if let Some((cached_modified, mesh)) = cache.get(path) {
            if *cached_modified == modified {
                return Some(mesh.clone());
            }
        }
    }
    let text = crate::core::project_migration::read_bounded_text_file(source, 128 * 1024 * 1024).ok()?;
    let mesh = Arc::new(crate::core::obj_loader::parse_obj_str(&text).ok()?);
    if let Ok(mut cache) = mesh_cache().lock() {
        cache.insert(path.to_string(), (modified, mesh.clone()));
    }
    Some(mesh)
}

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
        .mul_add(vector[0], vector[1].mul_add(vector[1], vector[2] * vector[2]))
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

fn shade(layer: &Layer, comp: &crate::core::timeline::Composition, world: [f32; 3], normal: [f32; 3], base: [f32; 4], frame: u32) -> [u8; 4] {
    let material = &layer.material;
    let mut light = material.ambient.clamp(0.0, 1.0) + material.emission.clamp(0.0, 1.0);
    if material.accepts_lights {
        for source in &comp.lights {
            let contribution = match source.light_type {
                LightType::Ambient => source.intensity / 100.0,
                LightType::Point | LightType::Spot { .. } | LightType::Parallel => {
                    let direction = normalize(sub(comp.light_position_at(source, frame), world));
                    (dot(normal, direction).max(0.0) * source.intensity / 100.0)
                        .min(1.0)
                }
            };
            light += contribution * material.diffuse.clamp(0.0, 1.0);
        }
    }
    let brightness = light.clamp(0.0, 2.0);
    [
        (base[0] * brightness * 255.0).clamp(0.0, 255.0) as u8,
        (base[1] * brightness * 255.0).clamp(0.0, 255.0) as u8,
        (base[2] * brightness * 255.0).clamp(0.0, 255.0) as u8,
        (base[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

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
    let model_scale = layer.transform_3d.scale.evaluate(frame).map(|value| value / 100.0);
    let camera = comp.resolve_camera_at();
    let mut projected = Vec::with_capacity(mesh.vertices.len());
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
        let screen = crate::core::timeline::project_point_to_screen_at_frame(
            &camera,
            world_point,
            width as f32,
            height as f32,
            frame,
        );
        projected.push(screen.map(|point| (point[0] * width as f32, point[1] * height as f32, world_point)));
    }
    let mut depth = vec![f32::INFINITY; (bw * bh) as usize];
    for triangle in mesh.indices.chunks_exact(3) {
        let (Some(a), Some(b), Some(c)) = (
            projected.get(triangle[0] as usize).copied().flatten(),
            projected.get(triangle[1] as usize).copied().flatten(),
            projected.get(triangle[2] as usize).copied().flatten(),
        ) else {
            continue;
        };
        let area = edge([a.0, a.1], [b.0, b.1], [c.0, c.1]);
        if area.abs() < 0.001 {
            continue;
        }
        let min_px = a.0.min(b.0).min(c.0).floor().max(min_x as f32) as u32;
        let max_px = a.0.max(b.0).max(c.0).ceil().min(max_x.saturating_sub(1) as f32) as u32;
        let min_py = a.1.min(b.1).min(c.1).floor().max(min_y as f32) as u32;
        let max_py = a.1.max(b.1).max(c.1).ceil().min(max_y.saturating_sub(1) as f32) as u32;
        if min_px > max_px || min_py > max_py {
            continue;
        }
        let normal = normalize(cross(sub([b.2[0], b.2[1], b.2[2]], a.2), sub([c.2[0], c.2[1], c.2[2]], a.2)));
        let color = shade(layer, comp, a.2, normal, base_color, frame);
        for py in min_py..=max_py {
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
                    compute_combined_mask_coverage(px as f32, py as f32, masks)
                };
                if mask_alpha <= 0.001 {
                    continue;
                }
                let world_z = weights[0] * a.2[2] + weights[1] * b.2[2] + weights[2] * c.2[2];
                let depth_index = ((py - min_y) * bw + (px - min_x)) as usize;
                if world_z >= depth[depth_index] {
                    continue;
                }
                depth[depth_index] = world_z;
                let pixel_index = depth_index * 4;
                layer_buf[pixel_index] = color[0];
                layer_buf[pixel_index + 1] = color[1];
                layer_buf[pixel_index + 2] = color[2];
                layer_buf[pixel_index + 3] = (f32::from(color[3]) * mask_alpha) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::render_frame_to_pixels;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Layer, LayerType};

    #[test]
    fn model_layer_renders_an_obj_triangle() {
        let path = std::env::temp_dir().join(format!(
            "kagari-model3d-test-{}.obj",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n",
        )
        .expect("write test mesh");

        let mut comp = Composition::new("comp".into(), "Comp".into(), 64, 64, 24, 24);
        let mut layer = Layer::new(
            "mesh".into(),
            "Mesh".into(),
            LayerType::Model3D {
                path: path.to_string_lossy().into_owned(),
            },
            comp.duration_frames,
        );
        layer.is_3d = true;
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        layer.transform_3d.position = Animatable::new_constant([0.0, 0.0, 600.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let model_pixels = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 25 || pixel[1] > 25 || pixel[2] > 25)
            .count();
        assert!(model_pixels > 0, "OBJ triangle did not reach the compositor");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn model_layer_renders_inside_a_precomposition() {
        let path = std::env::temp_dir().join(format!(
            "kagari-model3d-precomp-test-{}.obj",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n",
        )
        .expect("write test mesh");

        let mut nested = Composition::new("nested".into(), "Nested".into(), 64, 64, 24, 24);
        let mut model = Layer::new(
            "mesh".into(),
            "Mesh".into(),
            LayerType::Model3D {
                path: path.to_string_lossy().into_owned(),
            },
            nested.duration_frames,
        );
        model.is_3d = true;
        model.transform.position = Animatable::new_constant([32.0, 32.0]);
        model.transform_3d.position = Animatable::new_constant([0.0, 0.0, 600.0]);
        nested.layers.push(model);

        let mut root = Composition::new("root".into(), "Root".into(), 64, 64, 24, 24);
        root.sub_compositions.push(nested);
        let mut precomp = Layer::new(
            "precomp".into(),
            "Nested".into(),
            LayerType::PreComp {
                comp_id: "nested".into(),
            },
            root.duration_frames,
        );
        precomp.transform.position = Animatable::new_constant([32.0, 32.0]);
        root.layers.push(precomp);

        let pixels = render_frame_to_pixels(&root, 0, 64, 64, 0.0, 0);
        let model_pixels = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 25 || pixel[1] > 25 || pixel[2] > 25)
            .count();
        assert!(model_pixels > 0, "nested OBJ triangle did not reach the compositor");

        let _ = std::fs::remove_file(path);
    }
}
