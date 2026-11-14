use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_time_transform_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("time_changed", Some(ActionValue::Text(value))) => {
                self.stop_playback();
                if let Some(frame) = parse_timecode(&value, self.project_fps) {
                    self.current_frame = frame
                        .saturating_sub(
                            self.project_model
                                .active_composition()
                                .start_timecode_frames,
                        )
                        .min(
                            self.project_model
                                .active_composition()
                                .duration_frames
                                .saturating_sub(1),
                        );
                }
                self.current_time = format_timecode(self.current_frame, self.project_fps);
                true
            }
            ("zoom_changed", Some(ActionValue::Text(value))) => {
                let trimmed = value.trim().trim_end_matches('%').trim();
                if let Ok(pct) = trimmed.parse::<f64>() {
                    self.zoom = format!("{}%", pct.clamp(10.0, 800.0).round() as u32);
                } else {
                    self.zoom = "100%".to_owned();
                }
                self.zoom_manually_adjusted = true;
                true
            }
            ("position_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let width = f64::from(self.project_model.active_composition().width);
                let min = (-width).min(self.position_x - width);
                let max = (width * 2.0).max(self.position_x + width);
                self.position_x = value.clamp(min, max);
                self.apply_selected_transform_controls();
                true
            }
            ("position_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let height = f64::from(self.project_model.active_composition().height);
                let min = (-height).min(self.position_y - height);
                let max = (height * 2.0).max(self.position_y + height);
                self.position_y = value.clamp(min, max);
                self.apply_selected_transform_controls();
                true
            }
            ("anchor_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let width = f64::from(self.project_model.active_composition().width);
                let min = (-width).min(self.anchor_x - width);
                let max = (width * 2.0).max(self.anchor_x + width);
                self.anchor_x = value.clamp(min, max);
                self.apply_selected_transform_controls();
                true
            }
            ("anchor_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let height = f64::from(self.project_model.active_composition().height);
                let min = (-height).min(self.anchor_y - height);
                let max = (height * 2.0).max(self.anchor_y + height);
                self.anchor_y = value.clamp(min, max);
                self.apply_selected_transform_controls();
                true
            }
            ("scale_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.scale_x = value.clamp(0.1, 1000.0);
                self.apply_selected_transform_controls();
                true
            }
            ("scale_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.scale_y = value.clamp(0.1, 1000.0);
                self.apply_selected_transform_controls();
                true
            }
            ("rotation_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.rotation_degrees = value.clamp(-36_000.0, 36_000.0);
                self.apply_selected_transform_controls();
                true
            }
            ("opacity_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.opacity = value.clamp(0.0, 100.0);
                self.apply_selected_transform_controls();
                true
            }
            ("position_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_x", value);
                true
            }
            ("position_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_y", value);
                true
            }
            ("position_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_z", value);
                true
            }
            ("rotation_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_x", value);
                true
            }
            ("rotation_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_y", value);
                true
            }
            ("rotation_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_z", value);
                true
            }
            ("scale_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_x", value);
                true
            }
            ("scale_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_y", value);
                true
            }
            ("scale_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_z", value);
                true
            }
            ("material_ambient_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("ambient", value);
                true
            }
            ("material_diffuse_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("diffuse", value);
                true
            }
            ("material_specular_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("specular", value);
                true
            }
            ("material_specular_exponent_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("specular_exponent", value);
                true
            }
            ("material_emission_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("emission", value);
                true
            }
            ("material_metalness_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("metalness", value);
                true
            }
            ("material_light_transmission_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("light_transmission", value);
                true
            }
            ("toggle_material_cast_shadows", _) => {
                self.toggle_selected_3d_material_flag("cast_shadows");
                true
            }
            ("toggle_material_accepts_shadows", _) => {
                self.toggle_selected_3d_material_flag("accepts_shadows");
                true
            }
            ("toggle_material_accepts_lights", _) => {
                self.toggle_selected_3d_material_flag("accepts_lights");
                true
            }
            _ => false,
        }
    }
}
