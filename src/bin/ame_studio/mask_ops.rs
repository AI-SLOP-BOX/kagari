use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_mask_value(&mut self, input: &str, channel: MaskValueChannel) {
        let Ok(mut value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        value = match channel {
            MaskValueChannel::Feather => value.clamp(0.0, 5000.0),
            MaskValueChannel::Opacity => value.clamp(0.0, 100.0),
            MaskValueChannel::Expansion => value.clamp(-5000.0, 5000.0),
        };
        let (Some(layer_id), Some(mask_id)) = (
            self.selected_layer_id.as_deref(),
            self.selected_mask_id.as_deref(),
        ) else {
            return;
        };
        let frame = self.current_frame;
        if let Some(mask) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
        {
            let track = match channel {
                MaskValueChannel::Feather => &mut mask.feather,
                MaskValueChannel::Opacity => &mut mask.opacity,
                MaskValueChannel::Expansion => &mut mask.expansion,
            };
            track.set_value_at_frame(frame, value);
            self.mark_project_changed();
        }
    }

    pub(crate) fn add_selected_mask_keyframe(&mut self, property: &str) {
        let (Some(layer_id), Some(mask_id)) = (
            self.selected_layer_id.as_deref(),
            self.selected_mask_id.as_deref(),
        ) else {
            return;
        };
        let frame = self.current_frame;
        let changed = self.project_model.active_composition_mut().layers.iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
            .is_some_and(|mask| {
                match property {
                    "path" => add_current_keyframe(&mut mask.path.vertices, frame),
                    "feather" => add_current_keyframe(&mut mask.feather, frame),
                    "opacity" => add_current_keyframe(&mut mask.opacity, frame),
                    "expansion" => add_current_keyframe(&mut mask.expansion, frame),
                    _ => return false,
                }
                true
            });
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn update_mask_vertex(&mut self, point: [f64; 2]) {
        let Some(drag) = self.mask_vertex_drag.clone() else {
            return;
        };
        let frame = self.current_frame;
        if let Some(mask) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == drag.layer_id && !layer.locked)
            .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == drag.mask_id))
        {
            let mut vertices = mask.path.get_vertices(frame);
            if let Some(vertex) = vertices.get_mut(drag.vertex_index) {
                vertex[0] = point[0] as f32;
                vertex[1] = point[1] as f32;
                mask.path.vertices.set_value_at_frame(frame, vertices);
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn add_mask_shape(&mut self, ellipse: bool) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let (width, height) = {
            let composition = self.project_model.active_composition();
            (composition.width as f32, composition.height as f32)
        };
        let layer = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked);
        let Some(layer) = layer else {
            return;
        };
        let mut index = layer.masks.len() + 1;
        let id = loop {
            let candidate = format!("mask_{index}");
            if layer.masks.iter().all(|mask| mask.id != candidate) {
                break candidate;
            }
            index += 1;
        };
        let name = if ellipse {
            format!("楕円マスク {index}")
        } else {
            format!("矩形マスク {index}")
        };
        let mask = if ellipse {
            kagari_vfx::core::mask::Mask::new_ellipse(
                id.clone(),
                name,
                width * 0.5,
                height * 0.5,
                width * 0.25,
                height * 0.25,
            )
        } else {
            kagari_vfx::core::mask::Mask::new_rect(
                id.clone(),
                name,
                width * 0.25,
                height * 0.25,
                width * 0.5,
                height * 0.5,
            )
        };
        layer.masks.push(mask);
        self.selected_mask_id = Some(id);
        self.mark_project_changed();
    }
}
