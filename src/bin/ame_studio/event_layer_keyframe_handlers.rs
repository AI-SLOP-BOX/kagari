use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_layer_keyframe_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("add_keyframe", _) => {
                self.add_transform_keyframes_at_current_frame();
                true
            }
            ("keyframe_position_2d", _) => {
                self.add_transform_property_keyframe("position_2d");
                true
            }
            ("keyframe_anchor_point", _) => {
                self.add_transform_property_keyframe("anchor_point");
                true
            }
            ("keyframe_rotation_2d", _) => {
                self.add_transform_property_keyframe("rotation_2d");
                true
            }
            ("keyframe_scale_2d", _) => {
                self.add_transform_property_keyframe("scale_2d");
                true
            }
            ("keyframe_opacity", _) => {
                self.add_transform_property_keyframe("opacity");
                true
            }
            ("keyframe_position_3d", _) => {
                self.add_transform_property_keyframe("position_3d");
                true
            }
            ("keyframe_rotation_3d", _) => {
                self.add_transform_property_keyframe("rotation_3d");
                true
            }
            ("keyframe_scale_3d", _) => {
                self.add_transform_property_keyframe("scale_3d");
                true
            }
            ("previous_keyframe", _) => {
                self.jump_to_selected_keyframe(false);
                true
            }
            ("next_keyframe", _) => {
                self.jump_to_selected_keyframe(true);
                true
            }
            _ => false,
        }
    }
}
