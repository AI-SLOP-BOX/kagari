use super::*;

#[derive(Default)]
pub(super) struct LayerRenderData {
    pub(super) source_frame: u32,
    pub(super) pos: [f32; 2],
    pub(super) scale: [f32; 2],
    pub(super) rotation: f32,
    pub(super) l_opacity: f32,
    pub(super) masks: Vec<CpuMaskEntry>,
    pub(super) skip: bool,
    pub(super) dof_blur: f32,
}

pub(super) fn prepare_layer_render_data(
    comp: &Composition,
    frame: u32,
    render_camera: &crate::core::timeline::Camera3D,
    has_solo: bool,
) -> Vec<LayerRenderData> {
    use rayon::prelude::*;
    comp.layers
        .par_iter()
        .map(|layer| {
            if !layer.is_active(frame) || (has_solo && !layer.solo) || !layer.visible {
                return LayerRenderData::default(); // skip=true
            }

            let source_frame = layer.effective_render_frame(frame, comp.fps);
            let (pos, scale, rotation, opacity) = comp.resolve_world_transform(layer, frame);
            let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
            if l_opacity < 0.001 {
                return LayerRenderData::default();
            }

            // ── Depth of field: circle-of-confusion for 3D layers ──
            let dof_blur = if layer.is_3d && render_camera.dof_enabled_at(frame) {
                let z = layer.transform_3d.position.evaluate(frame)[2];
                let dof = crate::core::camera_dof::CameraDofSettings {
                    focus_distance: render_camera.focus_distance_at(frame),
                    aperture: render_camera.aperture_at(frame),
                    f_stop: render_camera.aperture_at(frame),
                    blur_level: 100.0,
                    iris_sides: render_camera.dof_iris_sides,
                    anamorphic_ratio: 1.0,
                    optical_vignetting: 0.0,
                };
                crate::core::camera_dof::calculate_circle_of_confusion(z, &dof)
                    .clamp(0.0, render_camera.dof_max_blur_at(frame))
            } else {
                0.0
            };

            let mut masks = Vec::new();
            for mask in &layer.masks {
                if mask.enabled && mask.mode != MaskMode::None {
                    let vertices = wiggle_polygon(
                        mask,
                        mask.path.to_polygon(frame, 16),
                        frame as f32 / comp.fps.max(1) as f32,
                    );
                    if vertices.len() >= 3 {
                        // Expand once per layer (not per pixel): the offset
                        // used to be recomputed inside the per-pixel
                        // coverage test, costing seconds per masked layer.
                        let expansion = mask.expansion.evaluate(frame);
                        masks.push(CpuMaskEntry {
                            vertices: offset_polygon_vertices(&vertices, expansion),
                            feather: mask.feather.evaluate(frame),
                            opacity: (mask.opacity.evaluate(frame) / 100.0).clamp(0.0, 1.0),
                            inverted: mask.inverted,
                            mode: mask.mode,
                        });
                    }
                }
            }

            LayerRenderData {
                source_frame,
                pos,
                scale,
                rotation,
                l_opacity,
                masks,
                skip: false,
                dof_blur,
            }
        })
        .collect()
}
