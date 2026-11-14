use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_project_asset_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("place_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.place_project_asset(id, false);
                };
                true
            }
            ("place_project_asset_fit", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    self.place_project_asset(&id, false);
                };
                true
            }
            ("place_project_asset_fill", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    self.place_project_asset(&id, true);
                };
                true
            }
            ("open_project_composition", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.open_project_composition_asset(id);
                };
                true
            }
            ("enter_project_asset_folder", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    let is_child_folder = self
                        .project_model
                        .assets
                        .iter()
                        .find(|item| item.id == id)
                        .is_some_and(|item| {
                            matches!(
                                &item.item_type,
                                kagari_vfx::core::timeline::ProjectItemType::Folder { .. }
                            ) && item.parent_folder.as_deref() == self.asset_folder_id.as_deref()
                        });
                    if is_child_folder {
                        self.asset_folder_id = Some(id.to_owned());
                        self.project_asset_search.clear();
                    }
                };
                true
            }
            ("leave_project_asset_folder", _) => {
                self.asset_folder_id = self
                    .asset_folder_id
                    .as_deref()
                    .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
                    .and_then(|item| item.parent_folder.clone());
                self.project_asset_search.clear();
                true
            }
            ("project_asset_search_changed", Some(ActionValue::Text(value))) => {
                self.project_asset_search = value.chars().take(256).collect();
                true
            }
            ("clear_project_asset_search", _) => {
                self.project_asset_search.clear();
                true
            }
            ("project_asset_search_all_folders", _) => {
                self.project_asset_search_all_folders = !self.project_asset_search_all_folders;
                true
            }
            ("project_assets_sort_name", _) => {
                self.project_assets_sort_by_type = false;
                true
            }
            ("project_assets_sort_type", _) => {
                self.project_assets_sort_by_type = true;
                true
            }
            ("new_asset_folder_name_changed", Some(ActionValue::Text(value))) => {
                self.new_asset_folder_name = value.chars().take(80).collect();
                true
            }
            ("select_project_asset", _) => {
                self.selected_project_asset_id = selected_row_id.clone().filter(|id| {
                    self.project_model.assets.iter().any(|item| {
                        item.id == *id
                            && !matches!(
                                &item.item_type,
                                kagari_vfx::core::timeline::ProjectItemType::Folder { .. }
                            )
                    })
                });
                true
            }
            ("select_project_asset_folder", _) => {
                self.selected_project_asset_id = selected_row_id.clone().filter(|id| {
                    self.project_model.assets.iter().any(|item| {
                        item.id == *id
                            && item.parent_folder.as_deref() == self.asset_folder_id.as_deref()
                            && matches!(
                                &item.item_type,
                                kagari_vfx::core::timeline::ProjectItemType::Folder { .. }
                            )
                    })
                });
                true
            }
            ("fit_selected_image_to_composition", _) => {
                self.scale_selected_media_to_composition(false);
                true
            }
            ("fill_selected_image_to_composition", _) => {
                self.scale_selected_media_to_composition(true);
                true
            }
            ("selected_project_asset_name_changed", Some(ActionValue::Text(value))) => {
                let name = value.trim().chars().take(128).collect::<String>();
                if !name.is_empty() {
                    if let Some(id) = self.selected_project_asset_id.as_deref() {
                        if let Some(item) = self
                            .project_model
                            .assets
                            .iter_mut()
                            .find(|item| item.id == id)
                        {
                            if item.name != name {
                                item.name = name;
                                self.mark_project_changed();
                            }
                        }
                    }
                };
                true
            }
            ("rename_project_asset", Some(ActionValue::Text(value))) => {
                let name = value.trim().chars().take(128).collect::<String>();
                if !name.is_empty() {
                    if let Some(id) = selected_row_id.as_deref() {
                        if let Some(item) = self
                            .project_model
                            .assets
                            .iter_mut()
                            .find(|item| item.id == id)
                        {
                            if matches!(
                                &item.item_type,
                                kagari_vfx::core::timeline::ProjectItemType::Folder { .. }
                            ) && item.name != name
                            {
                                item.name = name.clone();
                                if let kagari_vfx::core::timeline::ProjectItemType::Folder {
                                    name: folder_name,
                                } = &mut item.item_type
                                {
                                    *folder_name = name;
                                }
                                self.mark_project_changed();
                            }
                        }
                    }
                };
                true
            }
            ("move_selected_project_asset_here", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    let destination = self.asset_folder_id.clone();
                    if project_asset_can_move_to_folder(
                        &self.project_model.assets,
                        &id,
                        destination.as_deref(),
                    ) {
                        if let Some(item) = self
                            .project_model
                            .assets
                            .iter_mut()
                            .find(|item| item.id == id)
                        {
                            item.parent_folder = destination;
                            self.asset_status =
                                format!("{} を現在のフォルダーへ移動しました。", item.name);
                        }
                        self.mark_project_changed();
                    } else {
                        self.asset_status =
                            "このフォルダーには移動できません。移動先を確認してください。"
                                .to_owned();
                    }
                };
                true
            }
            ("remove_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    let removable = self
                        .project_model
                        .assets
                        .iter()
                        .find(|item| item.id == id)
                        .is_some_and(|item| {
                            use kagari_vfx::core::timeline::ProjectItemType;
                            match &item.item_type {
                                ProjectItemType::Composition { .. } => false,
                                ProjectItemType::Folder { .. } => {
                                    !self.project_model.assets.iter().any(|child| {
                                        child.parent_folder.as_deref() == Some(item.id.as_str())
                                    })
                                }
                                _ => true,
                            }
                        });
                    if removable {
                        let removed_name = self
                            .project_model
                            .assets
                            .iter()
                            .find(|item| item.id == id)
                            .map(|item| item.name.clone())
                            .unwrap_or_default();
                        self.project_model.assets.retain(|item| item.id != id);
                        if self.selected_project_asset_id.as_deref() == Some(id) {
                            self.selected_project_asset_id = None;
                        }
                        self.asset_status = format!("素材をプロジェクトから登録解除しました（元ファイルは保持）: {removed_name}");
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("create_project_asset_folder", _) => {
                let name = self.new_asset_folder_name.trim().to_owned();
                if !name.is_empty() {
                    let id = self.project_model.next_project_item_id("folder");
                    let mut folder = kagari_vfx::core::timeline::ProjectItem::new(
                        id,
                        name.clone(),
                        kagari_vfx::core::timeline::ProjectItemType::Folder { name },
                    );
                    folder.parent_folder = self.asset_folder_id.clone();
                    self.project_model.assets.push(folder);
                    self.new_asset_folder_name.clear();
                    self.mark_project_changed();
                };
                true
            }
            ("relink_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.relink_project_asset(id);
                };
                true
            }
            ("relink_scene_asset", _) => {
                if let Some(id) = selected_row_id {
                    self.relink_scene_asset(&id);
                };
                true
            }
            ("cancel_video_import", _) => {
                self.cancel_video_import();
                true
            }
            _ => false,
        }
    }
}
