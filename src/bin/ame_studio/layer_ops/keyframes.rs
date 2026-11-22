use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_transform_property_keyframe(&mut self, property: &str) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let frame = self.current_frame;
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        match property {
            "position_2d" => add_current_keyframe(&mut layer.transform.position, frame),
            "anchor_point" => add_current_keyframe(&mut layer.transform.anchor_point, frame),
            "rotation_2d" => add_current_keyframe(&mut layer.transform.rotation, frame),
            "scale_2d" => add_current_keyframe(&mut layer.transform.scale, frame),
            "opacity" => add_current_keyframe(&mut layer.transform.opacity, frame),
            "position_3d" => add_current_keyframe(&mut layer.transform_3d.position, frame),
            "rotation_3d" => add_current_keyframe(&mut layer.transform_3d.rotation, frame),
            "scale_3d" => add_current_keyframe(&mut layer.transform_3d.scale, frame),
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn add_selected_shape_keyframe(&mut self, channel: GraphChannel) {
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let frame = self.current_frame;
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .and_then(|layer| shape_parameter_track_mut(layer, channel))
            .is_some_and(|track| {
                add_current_keyframe(track, frame);
                true
            });
        if changed {
            self.mark_project_changed();
        }
    }
}
