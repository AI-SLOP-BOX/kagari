use super::*;

impl AfterEffectsStudio {
    pub(crate) fn remove_selected_layer(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == id)
        else {
            self.selected_layer_id = None;
            return;
        };
        if layer.locked {
            self.asset_status = "ロック中のレイヤーは削除できません。".to_owned();
            return;
        }
        self.align_selected_layer_ids.remove(&id);
        self.selected_layer_id = None;
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition.layers.iter().position(|layer| layer.id == id) else {
            self.selected_layer_id = None;
            return;
        };
        composition.remove_layer_at(index);
        for layer in &mut composition.layers {
            if layer.parent_id.as_deref() == Some(id.as_str()) {
                layer.parent_id = None;
            }
            if layer.track_matte_layer_id.as_deref() == Some(id.as_str()) {
                layer.track_matte_layer_id = None;
                layer.track_matte = kagari_vfx::core::timeline::TrackMatteMode::None;
            }
        }
        self.collapsed_scene_layers.remove(&id);
        self.selected_layer_id = self
            .project_model
            .active_composition()
            .layers
            .last()
            .map(|layer| layer.id.clone());
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn request_remove_selected_layer(&mut self) {
        if self.selected_layer_id.as_deref().is_some_and(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .any(|layer| layer.id == id && !layer.locked)
        }) {
            self.layer_remove_confirm_open = true;
        }
    }

    pub(crate) fn move_selected_layer(&mut self, direction: isize) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        self.move_layer_by_id(&id, direction);
    }

    pub(crate) fn move_selected_layer_to_edge(&mut self, direction: isize) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは並べ替えできません。".to_owned();
            return;
        }

        let moved = {
            let layers = &mut self.project_model.active_composition_mut().layers;
            let Some(mut index) = layers.iter().position(|layer| layer.id == id) else {
                return;
            };
            let mut moved = false;
            if direction < 0 {
                while index > 0 && !layers[index - 1].locked {
                    layers.swap(index, index - 1);
                    index -= 1;
                    moved = true;
                }
            } else {
                while index + 1 < layers.len() && !layers[index + 1].locked {
                    layers.swap(index, index + 1);
                    index += 1;
                    moved = true;
                }
            }
            moved
        };

        if moved {
            self.selected_layer_id = Some(id);
            self.sync_selected_layer_controls();
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn move_layer_by_id(&mut self, id: &str, direction: isize) {
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは並べ替えできません。".to_owned();
            return;
        }
        let layers = &mut self.project_model.active_composition_mut().layers;
        let Some(index) = layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let next = (index as isize + direction).clamp(0, layers.len() as isize - 1) as usize;
        if index != next && layers[next].locked {
            self.asset_status = "ロック中のレイヤーをまたいで並べ替えできません。".to_owned();
            return;
        }
        if index != next {
            layers.swap(index, next);
            self.selected_layer_id = Some(id.to_owned());
            self.sync_selected_layer_controls();
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn duplicate_selected_layer(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = "ロック中のレイヤーは複製できません。".to_owned();
            return;
        }
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition.layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let new_id = composition.next_layer_id("layer_copy");
        let mut duplicate = composition.layers[index].clone();
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} copy", duplicate.name);
        duplicate.parent_id = None;
        duplicate.track_matte_layer_id = None;
        duplicate.track_matte = kagari_vfx::core::timeline::TrackMatteMode::None;
        composition.layers.insert(index + 1, duplicate);
        self.selected_layer_id = Some(new_id);
        self.selected_effect_id = None;
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn split_selected_layer_at_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.asset_status = "分割するレイヤーを選択してください。".to_owned();
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(index) = composition
            .layers
            .iter()
            .position(|layer| layer.id == layer_id)
        else {
            return;
        };
        let original = composition.layers[index].clone();
        if original.locked {
            self.asset_status = "ロック中のレイヤーは分割できません。".to_owned();
            return;
        }
        if original.scene_object.is_some() {
            self.asset_status = "カメラ・ライトレイヤーは分割できません。".to_owned();
            return;
        }
        if frame <= original.in_frame || frame > original.out_frame {
            self.asset_status = "再生位置をレイヤーの範囲内に置いてください。".to_owned();
            return;
        }
        let new_id = composition.next_layer_id("split");
        let mut second = original.clone();
        let original_out = original.out_frame;
        let original_name = original.name.clone();
        composition.layers[index].out_frame = frame - 1;
        second.id = new_id.clone();
        second.name = format!("{} · 分割", original_name);
        second.in_frame = frame;
        second.out_frame = original_out;
        composition.layers.insert(index + 1, second);
        self.selected_layer_id = Some(new_id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("{original_name} を {frame} フレームで分割しました。");
        self.mark_project_changed();
        self.sync_bindings_logged();
    }

    pub(crate) fn set_selected_layer_in_to_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition
            .layers
            .iter_mut()
            .find(|l| l.id == layer_id && !l.locked)
        else {
            return;
        };
        if frame < layer.out_frame {
            layer.in_frame = frame;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn set_selected_layer_out_to_playhead(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let frame = self.current_frame;
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition
            .layers
            .iter_mut()
            .find(|l| l.id == layer_id && !l.locked)
        else {
            return;
        };
        if frame > layer.in_frame {
            layer.out_frame = frame;
            self.preview_revision = self.preview_revision.wrapping_add(1);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn toggle_scene_group(&mut self, layer_id: &str) {
        let has_children = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .any(|layer| layer.parent_id.as_deref() == Some(layer_id));
        if !has_children {
            return;
        }
        if !self.collapsed_scene_layers.remove(layer_id) {
            self.collapsed_scene_layers.insert(layer_id.to_owned());
        }
        self.sync_bindings_logged();
    }

    pub(crate) fn set_all_scene_layer_visibility(&mut self, visible: bool) {
        let changed = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .fold(false, |changed, layer| {
                let layer_changed = layer.visible != visible;
                layer.visible = visible;
                changed || layer_changed
            });
        if changed {
            self.mark_project_changed();
        }
    }

    pub(crate) fn parent_selected_layer(&mut self) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let layers = &self.project_model.active_composition().layers;
        let query = self.search.trim().to_lowercase();
        let layer_by_id = layers
            .iter()
            .map(|layer| (layer.id.as_str(), layer))
            .collect::<std::collections::HashMap<_, _>>();
        let search_visible = scene_search_visible_layer_ids(layers, &layer_by_id, &query);
        let visible_ids = layers
            .iter()
            .filter(|layer| search_visible.contains(&layer.id))
            .filter(|layer| {
                !query.is_empty()
                    || !scene_layer_hidden_by_collapse(
                        &layer_by_id,
                        layer,
                        &self.collapsed_scene_layers,
                    )
            })
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>();
        let Some(index) = visible_ids.iter().position(|id| *id == selected_id) else {
            return;
        };
        let Some(parent_id) = index
            .checked_sub(1)
            .and_then(|previous| visible_ids.get(previous))
            .map(|layer_id| (*layer_id).to_owned())
        else {
            return;
        };
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, Some(parent_id.clone()));
        if changed {
            self.collapsed_scene_layers.remove(&parent_id);
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn set_selected_layer_parent(&mut self, parent_id: Option<&str>) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let parent_id = parent_id.map(str::to_owned);
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, parent_id.clone());
        if changed {
            if let Some(parent_id) = parent_id {
                self.collapsed_scene_layers.remove(&parent_id);
            }
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn unparent_selected_layer(&mut self) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            return;
        };
        if self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == selected_id)
            .is_none_or(|layer| layer.locked)
        {
            return;
        }
        let changed = self
            .project_model
            .active_composition_mut()
            .set_layer_parent(&selected_id, None);
        if changed {
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }
}
