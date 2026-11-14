use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_effect_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("effect_search_changed", Some(ActionValue::Text(value))) => {
                self.effect_search = value;
                true
            }
            ("animation_preset_search_changed", Some(ActionValue::Text(value))) => {
                self.animation_preset_search = value;
                true
            }
            ("apply_animation_preset", _) => {
                if let Some(preset_id) = selected_row_id.as_deref() {
                    self.apply_animation_preset(preset_id);
                };
                true
            }
            ("add_effect_catalog", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.add_effect(effect_id);
                };
                true
            }
            ("add_gaussian_blur", _) => {
                self.add_effect("blur");
                true
            }
            ("add_color_tint", _) => {
                self.add_effect("tint");
                true
            }
            ("add_drop_shadow", _) => {
                self.add_effect("shadow");
                true
            }
            ("add_glow", _) => {
                self.add_effect("glow");
                true
            }
            ("add_vignette", _) => {
                self.add_effect("vignette");
                true
            }
            ("add_hue_saturation", _) => {
                self.add_effect("hue_saturation");
                true
            }
            ("add_levels", _) => {
                self.add_effect("levels");
                true
            }
            ("add_motion_blur", _) => {
                self.add_effect("motion_blur");
                true
            }
            ("add_film_grain", _) => {
                self.add_effect("film_grain");
                true
            }
            ("add_posterize", _) => {
                self.add_effect("posterize");
                true
            }
            ("add_directional_blur", _) => {
                self.add_effect("directional_blur");
                true
            }
            ("add_radial_blur", _) => {
                self.add_effect("radial_blur");
                true
            }
            ("add_sharpen", _) => {
                self.add_effect("sharpen");
                true
            }
            ("select_effect", _) => {
                self.selected_effect_id = selected_row_id;
                true
            }
            ("toggle_effect", _) => {
                if let Some(effect_id) = selected_row_id {
                    let layer_id = self.selected_layer_id.clone();
                    let changed = layer_id
                        .as_deref()
                        .and_then(|layer_id| {
                            self.project_model
                                .active_composition_mut()
                                .layers
                                .iter_mut()
                                .find(|layer| layer.id == layer_id)
                        })
                        .and_then(|layer| {
                            layer
                                .effects
                                .iter_mut()
                                .find(|effect| effect.id == effect_id)
                        })
                        .map(|effect| effect.enabled = !effect.enabled)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("move_effect_up", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.move_effect(effect_id, -1);
                };
                true
            }
            ("move_effect_down", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() {
                    self.move_effect(effect_id, 1);
                };
                true
            }
            ("remove_effect", _) => {
                if let (Some(layer_id), Some(effect_id)) =
                    (self.selected_layer_id.clone(), selected_row_id)
                {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let old_len = layer.effects.len();
                        layer.effects.retain(|effect| effect.id != effect_id);
                        if layer.effects.len() != old_len {
                            if self.selected_effect_id.as_deref() == Some(effect_id.as_str()) {
                                self.selected_effect_id = None;
                            }
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("effect_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter_id) = selected_row_id {
                    self.update_selected_effect_parameter(&parameter_id, &value)
                };
                true
            }
            ("effect_parameter_keyframe", _) => {
                if let Some(parameter_id) = selected_row_id.as_deref() {
                    self.keyframe_selected_effect_parameter(parameter_id);
                };
                true
            }
            ("effect_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_effect_color(&value);
                true
            }
            _ => false,
        }
    }
}
