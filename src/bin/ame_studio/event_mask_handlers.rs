use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_mask_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("add_rect_mask", _) => {
                self.add_mask_shape(false);
                true
            }
            ("add_ellipse_mask", _) => {
                self.add_mask_shape(true);
                true
            }
            ("select_mask", _) => {
                if let Some(id) = selected_row_id {
                    self.selected_mask_id = Some(id);
                };
                true
            }
            ("toggle_mask_enabled", _) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    selected_row_id.as_deref(),
                ) {
                    if let Some(mask) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        mask.enabled = !mask.enabled;
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("cycle_mask_mode", _) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_mask_id.as_deref(),
                ) {
                    if let Some(mask) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        use kagari_vfx::core::mask::MaskMode;
                        const MODES: [MaskMode; 7] = [
                            MaskMode::Add,
                            MaskMode::Subtract,
                            MaskMode::Intersect,
                            MaskMode::Lighten,
                            MaskMode::Darken,
                            MaskMode::Difference,
                            MaskMode::None,
                        ];
                        let index = MODES
                            .iter()
                            .position(|mode| *mode == mask.mode)
                            .unwrap_or(0);
                        mask.mode = MODES[(index + 1) % MODES.len()];
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("select_mask_mode", Some(ActionValue::Text(label))) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_mask_id.as_deref(),
                ) {
                    use kagari_vfx::core::mask::MaskMode;
                    let next = match label.as_str() {
                        "加算" => Some(MaskMode::Add),
                        "減算" => Some(MaskMode::Subtract),
                        "交差" => Some(MaskMode::Intersect),
                        "明るく" => Some(MaskMode::Lighten),
                        "暗く" => Some(MaskMode::Darken),
                        "差分" => Some(MaskMode::Difference),
                        "なし" => Some(MaskMode::None),
                        _ => None,
                    };
                    if let Some(next) = next {
                        if let Some(mask) = self
                            .project_model
                            .active_composition_mut()
                            .layers
                            .iter_mut()
                            .find(|layer| layer.id == layer_id && !layer.locked)
                            .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                        {
                            if mask.mode != next {
                                mask.mode = next;
                                self.mark_project_changed();
                            }
                        }
                    }
                };
                true
            }
            ("toggle_mask_inverted", _) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_mask_id.as_deref(),
                ) {
                    if let Some(mask) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        mask.inverted = !mask.inverted;
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("remove_mask", _) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    selected_row_id.as_deref(),
                ) {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let before = layer.masks.len();
                        layer.masks.retain(|mask| mask.id != mask_id);
                        if layer.masks.len() != before {
                            self.selected_mask_id = layer.masks.first().map(|mask| mask.id.clone());
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("mask_name_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_mask_id.as_deref(),
                ) {
                    if let Some(mask) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        if mask.name != value {
                            mask.name = value;
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("mask_feather_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_mask_value(&value, MaskValueChannel::Feather);
                true
            }
            ("mask_opacity_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_mask_value(&value, MaskValueChannel::Opacity);
                true
            }
            ("mask_expansion_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_mask_value(&value, MaskValueChannel::Expansion);
                true
            }
            ("mask_path_keyframe", _) => {
                self.add_selected_mask_keyframe("path");
                true
            }
            ("mask_feather_keyframe", _) => {
                self.add_selected_mask_keyframe("feather");
                true
            }
            ("mask_opacity_keyframe", _) => {
                self.add_selected_mask_keyframe("opacity");
                true
            }
            ("mask_expansion_keyframe", _) => {
                self.add_selected_mask_keyframe("expansion");
                true
            }
            _ => false,
        }
    }
}
