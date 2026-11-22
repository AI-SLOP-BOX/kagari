use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_scene_layer_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("toggle_shy_visibility", _) => {
                self.hide_shy_layers = !self.hide_shy_layers;
                true
            }
            ("toggle_layer_visibility", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    let changed = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == id)
                        .map(|layer| layer.visible = !layer.visible)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("toggle_audio_mute", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    let changed = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == id)
                        .map(|layer| layer.audio_muted = !layer.audio_muted)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("toggle_layer_lock", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    let changed = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == id)
                        .map(|layer| layer.locked = !layer.locked)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("toggle_layer_solo", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    let changed = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == id)
                        .map(|layer| layer.solo = !layer.solo)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("toggle_layer_motion_blur", _) => {
                if let Some(id) = selected_row_id {
                    let locked_name = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == id && layer.locked)
                        .map(|layer| layer.name.clone());
                    if let Some(name) = locked_name {
                        self.asset_status =
                            format!("「{name}」はロック中のためモーションブラーを変更できません。");
                    } else {
                        let changed = self
                            .project_model
                            .active_composition_mut()
                            .layers
                            .iter_mut()
                            .find(|layer| layer.id == id)
                            .map(|layer| layer.motion_blur = !layer.motion_blur)
                            .is_some();
                        if changed {
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("toggle_scene_group", _) => {
                if let Some(id) = selected_row_id {
                    self.toggle_scene_group(&id);
                };
                true
            }
            ("collapse_all_scene_groups", _) => {
                let layers = &self.project_model.active_composition().layers;
                let parent_ids = layers
                    .iter()
                    .filter_map(|layer| layer.parent_id.as_deref())
                    .collect::<std::collections::HashSet<_>>();
                self.collapsed_scene_layers = layers
                    .iter()
                    .filter(|layer| parent_ids.contains(layer.id.as_str()))
                    .map(|layer| layer.id.clone())
                    .collect();
                true
            }
            ("expand_all_scene_groups", _) => {
                self.collapsed_scene_layers.clear();
                true
            }
            ("show_all_scene_layers", _) => {
                self.set_all_scene_layer_visibility(true);
                true
            }
            ("hide_all_scene_layers", _) => {
                self.set_all_scene_layer_visibility(false);
                true
            }
            ("toggle_alignment_layer_selection", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.toggle_alignment_layer_selection(id);
                };
                true
            }
            ("select_all_alignment_layers", _) => {
                self.align_selected_layer_ids = self
                    .project_model
                    .active_composition()
                    .layers
                    .iter()
                    .filter(|layer| !layer.locked)
                    .map(|layer| layer.id.clone())
                    .collect();
                true
            }
            ("clear_alignment_selection", _) => {
                self.align_selected_layer_ids.clear();
                true
            }
            ("toggle_align_to_composition", _) => {
                self.align_to_composition = !self.align_to_composition;
                true
            }
            ("align_scene_left", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::Left);
                true
            }
            ("align_scene_horizontal_center", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::HorizontalCenter);
                true
            }
            ("align_scene_right", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::Right);
                true
            }
            ("align_scene_top", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::Top);
                true
            }
            ("align_scene_vertical_center", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::VerticalCenter);
                true
            }
            ("align_scene_bottom", _) => {
                self.align_scene_layers(kagari_vfx::core::layer_align::AlignMode::Bottom);
                true
            }
            ("distribute_scene_horizontal", _) => {
                self.distribute_scene_layers(
                    kagari_vfx::core::layer_align::DistributeMode::HorizontalCenters,
                );
                true
            }
            ("distribute_scene_vertical", _) => {
                self.distribute_scene_layers(
                    kagari_vfx::core::layer_align::DistributeMode::VerticalCenters,
                );
                true
            }
            ("parent_selected_layer", _) => {
                self.parent_selected_layer();
                true
            }
            ("unparent_selected_layer", _) => {
                self.unparent_selected_layer();
                true
            }
            ("set_layer_parent", _) => {
                if let Some(parent_id) = selected_row_id.as_deref() {
                    self.set_selected_layer_parent(Some(parent_id));
                };
                true
            }
            _ => false,
        }
    }
}
