use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_3d_transform(&mut self, field: &str, input: f64) {
        if !input.is_finite() {
            return;
        }
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && layer.is_3d)
        else {
            return;
        };
        let (track, axis, value) = match field {
            "position_x" => (
                &mut layer.transform_3d.position,
                0,
                input.clamp(-100_000.0, 100_000.0),
            ),
            "position_y" => (
                &mut layer.transform_3d.position,
                1,
                input.clamp(-100_000.0, 100_000.0),
            ),
            "position_z" => (
                &mut layer.transform_3d.position,
                2,
                input.clamp(-100_000.0, 100_000.0),
            ),
            "rotation_x" => (
                &mut layer.transform_3d.rotation,
                0,
                input.clamp(-36_000.0, 36_000.0),
            ),
            "rotation_y" => (
                &mut layer.transform_3d.rotation,
                1,
                input.clamp(-36_000.0, 36_000.0),
            ),
            "rotation_z" => (
                &mut layer.transform_3d.rotation,
                2,
                input.clamp(-36_000.0, 36_000.0),
            ),
            "scale_x" => (&mut layer.transform_3d.scale, 0, input.clamp(0.1, 1000.0)),
            "scale_y" => (&mut layer.transform_3d.scale, 1, input.clamp(0.1, 1000.0)),
            "scale_z" => (&mut layer.transform_3d.scale, 2, input.clamp(0.1, 1000.0)),
            _ => return,
        };
        let mut next = track.evaluate(frame);
        let next_value = value as f32;
        if (next[axis] - next_value).abs() <= f32::EPSILON {
            return;
        }
        next[axis] = next_value;
        set_animatable_at_frame(track, frame, next);
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_3d_material(&mut self, field: &str, input: f64) {
        if !input.is_finite() {
            return;
        }
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && layer.is_3d)
        else {
            return;
        };
        let material = &mut layer.material;
        let (target, value) = match field {
            "ambient" => (&mut material.ambient, input.clamp(0.0, 1.0)),
            "diffuse" => (&mut material.diffuse, input.clamp(0.0, 1.0)),
            "specular" => (&mut material.specular, input.clamp(0.0, 1.0)),
            "specular_exponent" => (&mut material.specular_exponent, input.clamp(1.0, 256.0)),
            "emission" => (&mut material.emission, input.clamp(0.0, 1.0)),
            "metalness" => (&mut material.metalness, input.clamp(0.0, 1.0)),
            "light_transmission" => (&mut material.light_transmission, input.clamp(0.0, 100.0)),
            _ => return,
        };
        let value = value as f32;
        if (*target - value).abs() <= f32::EPSILON {
            return;
        }
        *target = value;
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_3d_material_flag(&mut self, field: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && layer.is_3d)
        else {
            return;
        };
        let target = match field {
            "cast_shadows" => &mut layer.material.cast_shadows,
            "accepts_shadows" => &mut layer.material.accepts_shadows,
            "accepts_lights" => &mut layer.material.accepts_lights,
            _ => return,
        };
        *target = !*target;
        self.mark_project_changed();
    }
}
