use super::*;

impl AfterEffectsStudio {
    pub(super) fn handle_secondary_action(
        &mut self,
        action_name: &str,
        action_val: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_val) {
            ("import_dropped_file", Some(ActionValue::Drop { payload: path, .. })) => {
                self.import_dropped_file(&path);
            }
            ("audio_volume_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_audio_volume(&value)
            }
            ("keyframe_audio_volume", _) => self.add_selected_audio_volume_keyframe(),
            ("audio_to_keyframes", _) => self.convert_selected_audio_to_keyframes(),
            ("audio_track_volume_changed", Some(ActionValue::Text(value))) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.update_audio_volume_for_layer(id, &value);
                }
            }
            ("audio_track_volume_keyframe", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.add_audio_volume_keyframe_for_layer(id);
                }
            }
            ("video_speed_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_video_speed(&value)
            }
            ("toggle_video_time_remap", _) => self.toggle_selected_video_time_remap(),
            ("toggle_video_posterize", _) => self.toggle_selected_video_posterize(),
            ("toggle_precomp_collapse_transforms", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let changed = self.project_model.active_composition_mut().layers
                        .iter_mut()
                        .find(|layer| {
                            layer.id == layer_id
                                && !layer.locked
                                && matches!(
                                    &layer.layer_type,
                                    kagari_vfx::core::timeline::LayerType::PreComp { .. }
                                )
                        })
                        .map(|layer| layer.is_collapsed = !layer.is_collapsed)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                }
            }
            ("video_posterize_fps_changed", Some(ActionValue::Text(value))) => self.update_selected_video_posterize_fps(&value),
            ("cycle_layer_proxy_resolution", _) => self.cycle_selected_layer_proxy_resolution(),
            ("select_layer_proxy_resolution", Some(ActionValue::Text(label))) => {
                self.set_selected_layer_proxy_resolution(&label);
            }
            ("toggle_layer_proxy", _) => self.toggle_selected_layer_proxy(),
            ("clear_layer_proxy", _) => self.clear_selected_layer_proxy(),
            ("generate_layer_proxy", _) => self.start_selected_layer_proxy_generation(),
            ("cancel_layer_proxy_generation", _) => self.cancel_layer_proxy_generation(),
            ("ease_selected_layer", _) => self.ease_selected_layer_keyframes(),
            ("open_recent_project", _) => {
                if let Some(path) = selected_row_id.and_then(|path| {
                    self.recent_projects
                        .iter()
                        .find(|item| item.path.as_os_str().to_string_lossy() == path)
                        .map(|item| item.path.clone())
                }) {
                    self.active_project_path = Some(path.display().to_string());
                    self.request_project_open(&path);
                }
            }
            ("project_sort_changed", Some(ActionValue::Text(sort))) => {
                self.project_sort = sort;
            }
            ("project_sort_toggle_dir", _) => {
                self.project_sort_desc = !self.project_sort_desc;
            }
            ("project_sort_by_name", _) => {
                if self.project_sort == "name" {
                    self.project_sort_desc = !self.project_sort_desc;
                } else {
                    self.project_sort = "name".to_owned();
                    self.project_sort_desc = false;
                }
            }
            ("project_sort_by_updated", _) => {
                if self.project_sort == "updated" {
                    self.project_sort_desc = !self.project_sort_desc;
                } else {
                    self.project_sort = "updated".to_owned();
                    self.project_sort_desc = true;
                }
            }
            ("project_view_toggle", _) => {
                self.project_view_grid = !self.project_view_grid;
            }
            ("project_toggle_select", _) => {
                if let Some(path) = selected_row_id {
                    let key = path.to_owned();
                    if !self.selected_project_paths.remove(&key) {
                        self.selected_project_paths.insert(key.clone());
                    }
                    self.active_project_path = Some(key);
                }
            }
            ("project_clear_selection", _) => {
                self.selected_project_paths.clear();
                self.active_project_path = None;
            }
            ("project_open_selected", _) => {
                let target_key = self
                    .active_project_path
                    .clone()
                    .or_else(|| self.selected_project_paths.iter().next().cloned())
                    .or_else(|| {
                        self.recent_projects
                            .first()
                            .map(|item| item.path.display().to_string())
                    });
                if let Some(path) = target_key.and_then(|key| {
                    self.recent_projects
                        .iter()
                        .find(|item| item.path.display().to_string() == key)
                        .map(|item| item.path.clone())
                }) {
                    self.request_project_open(&path);
                }
            }
            ("project_remove_selected", _) => {
                if let Some(path) = selected_row_id {
                    let key = path.to_owned();
                    if self.selected_project_paths.is_empty()
                        || !self.selected_project_paths.contains(&key)
                    {
                        self.recent_projects
                            .retain(|item| item.path.display().to_string() != key);
                        self.selected_project_paths.remove(&key);
                    } else {
                        self.recent_projects.retain(|item| {
                            !self
                                .selected_project_paths
                                .contains(&item.path.display().to_string())
                        });
                        self.selected_project_paths.clear();
                    }
                } else if !self.selected_project_paths.is_empty() {
                    self.recent_projects.retain(|item| {
                        !self
                            .selected_project_paths
                            .contains(&item.path.display().to_string())
                    });
                    self.selected_project_paths.clear();
                } else if let Some(key) = self.active_project_path.take() {
                    self.recent_projects
                        .retain(|item| item.path.display().to_string() != key);
                }
            }
            ("project_reveal_item", _) => {
                if let Some(path) = selected_row_id.and_then(|key| {
                    self.recent_projects
                        .iter()
                        .find(|item| item.path.display().to_string() == key)
                        .map(|item| item.path.clone())
                }) {
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg("-R").arg(&path).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        if let Some(parent) = path.parent() {
                            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                        }
                    }
                    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                    {
                        let _ = &path;
                    }
                }
            }
            ("project_duplicate_item", _) => {
                if let Some(item) = selected_row_id.and_then(|key| {
                    self.recent_projects
                        .iter()
                        .find(|item| item.path.display().to_string() == key)
                        .cloned()
                }) {
                    let stem = item
                        .path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&item.name);
                    let ext = item
                        .path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("json");
                    let copy_path = item.path.with_file_name(format!("{stem}_copy.{ext}"));
                    if item.path.exists() {
                        let _ = std::fs::copy(&item.path, &copy_path);
                    }
                    let mut dup = item.clone();
                    dup.name = format!("{} のコピー", item.name);
                    dup.path = copy_path;
                    dup.path_label = format!(".../projects/{}_copy.{ext}", stem);
                    self.recent_projects.insert(0, dup);
                }
            }
            ("request_remove_selected_layer", _) => self.request_remove_selected_layer(),
            ("confirm_remove_selected_layer", _) => {
                self.layer_remove_confirm_open = false;
                self.remove_selected_layer();
            }
            ("cancel_remove_selected_layer", _) => {
                self.layer_remove_confirm_open = false;
            }
            ("move_layer_up", _) => self.move_selected_layer(-1),
            ("move_layer_down", _) => self.move_selected_layer(1),
            ("move_layer_to_start", _) => self.move_selected_layer_to_edge(-1),
            ("move_layer_to_end", _) => self.move_selected_layer_to_edge(1),
            ("move_timeline_layer_up", _) => {
                if let Some(id) = selected_row_id.as_deref() { self.move_layer_by_id(id, -1); }
            }
            ("move_timeline_layer_down", _) => {
                if let Some(id) = selected_row_id.as_deref() { self.move_layer_by_id(id, 1); }
            }
            ("duplicate_selected_layer", _) => self.duplicate_selected_layer(),
            ("emitter_rate_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("rate", &value)
            }
            ("emitter_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("radius", &value)
            }
            ("emitter_spread_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("spread", &value)
            }
            ("emitter_speed_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("speed", &value)
            }
            ("emitter_speed_variance_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("speed_variance", &value)
            }
            ("emitter_lifetime_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("lifetime", &value)
            }
            ("emitter_size_start_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("size_start", &value)
            }
            ("emitter_size_end_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("size_end", &value)
            }
            ("emitter_opacity_start_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("opacity_start", &value)
            }
            ("emitter_opacity_end_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("opacity_end", &value)
            }
            ("emitter_gravity_y_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("gravity_y", &value)
            }
            ("emitter_wind_x_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("wind_x", &value)
            }
            ("emitter_turbulence_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("turbulence", &value)
            }
            ("emitter_drag_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_emitter("drag", &value)
            }
            ("emitter_shape_point", _) => self
                .set_selected_emitter_shape(kagari_vfx::core::particle_system::EmitterShape::Point),
            ("emitter_shape_circle_select", _) => self.set_selected_emitter_shape(
                kagari_vfx::core::particle_system::EmitterShape::Circle,
            ),
            ("add_motion_tracker", _) => {
                if let Some(layer_id) = self.selected_layer_id.clone() {
                    let comp = self.project_model.active_composition_mut();
                    if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == layer_id) {
                        let tracker_num = layer.trackers.len() + 1;
                        let tracker_id = format!("tracker_{}_{}", layer.id, tracker_num);
                        let tracker_name = format!("トラッカー {tracker_num}");
                        let initial_pos = layer.transform.position.evaluate(self.current_frame);
                        layer.trackers.push(kagari_vfx::core::timeline::TrackerPoint::new(
                            tracker_id,
                            tracker_name,
                            initial_pos,
                        ));
                        self.selected_tracker_idx = layer.trackers.len().saturating_sub(1);
                        self.tracker_status = format!("トラッカー {} を作成しました", tracker_num);
                        self.mark_project_changed();
                    }
                }
            }
            ("analyze_motion_track_forward", _) => {
                if let Some(layer_id) = self.selected_layer_id.clone() {
                    let comp = self.project_model.active_composition_mut();
                    if let Some(layer_idx) = comp.layers.iter().position(|l| l.id == layer_id) {
                        let duration = comp.duration_frames;
                        let tracker_idx = self.selected_tracker_idx;
                        let start_frame = self.current_frame;
                        let end_frame = (start_frame + 60).min(duration.saturating_sub(1));
                        kagari_vfx::core::tracker_engine::TrackerEngine::analyze_track(
                            comp,
                            layer_idx,
                            tracker_idx,
                            start_frame,
                            end_frame,
                        );
                        self.tracker_status = format!("フレーム {start_frame}〜{end_frame} を解析完了");
                        self.mark_project_changed();
                    }
                }
            }
            ("apply_motion_track_to_target", _) => {
                if let (Some(source_id), Some(target_id)) = (
                    self.selected_layer_id.clone(),
                    self.selected_tracker_target_id.clone(),
                ) {
                    let comp = self.project_model.active_composition_mut();
                    let source_idx = comp.layers.iter().position(|l| l.id == source_id);
                    let target_idx = comp.layers.iter().position(|l| l.id == target_id);
                    if let (Some(s_idx), Some(t_idx)) = (source_idx, target_idx) {
                        let tracker_idx = self.selected_tracker_idx;
                        kagari_vfx::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                            comp,
                            s_idx,
                            tracker_idx,
                            t_idx,
                            true,
                            true,
                        );
                        self.tracker_status = "ターゲットに位置・回転を適用しました".to_owned();
                        self.mark_project_changed();
                    }
                } else {
                    self.tracker_status = "適用先ターゲットレイヤーを選択してください".to_owned();
                }
            }
            ("apply_motion_track_stabilize", _) => {
                if let (Some(source_id), Some(target_id)) = (
                    self.selected_layer_id.clone(),
                    self.selected_tracker_target_id.clone(),
                ) {
                    let comp = self.project_model.active_composition_mut();
                    let source_idx = comp.layers.iter().position(|l| l.id == source_id);
                    let target_idx = comp.layers.iter().position(|l| l.id == target_id);
                    if let (Some(s_idx), Some(t_idx)) = (source_idx, target_idx) {
                        let tracker_idx = self.selected_tracker_idx;
                        kagari_vfx::core::tracker_engine::TrackerEngine::apply_stabilize_tracker_to_target(
                            comp,
                            s_idx,
                            tracker_idx,
                            t_idx,
                            true,
                            true,
                        );
                        self.tracker_status = "手ブレ補正（反転スタビライズ）を適用しました".to_owned();
                        self.mark_project_changed();
                    }
                } else if let Some(source_id) = self.selected_layer_id.clone() {
                    let comp = self.project_model.active_composition_mut();
                    if let Some(s_idx) = comp.layers.iter().position(|l| l.id == source_id) {
                        let tracker_idx = self.selected_tracker_idx;
                        kagari_vfx::core::tracker_engine::TrackerEngine::apply_stabilize_to_layer(
                            comp,
                            s_idx,
                            tracker_idx,
                            true,
                            true,
                        );
                        self.tracker_status = "自身に手ブレ補正を適用しました".to_owned();
                        self.mark_project_changed();
                    }
                }
            }
            ("apply_planar_track_corner_pin", _) => {
                if let (Some(source_id), Some(target_id)) = (
                    self.selected_layer_id.clone(),
                    self.selected_tracker_target_id.clone(),
                ) {
                    let comp = self.project_model.active_composition_mut();
                    let source_idx = comp.layers.iter().position(|l| l.id == source_id);
                    let target_idx = comp.layers.iter().position(|l| l.id == target_id);
                    if let (Some(s_idx), Some(t_idx)) = (source_idx, target_idx) {
                        let tracker_count = comp.layers[s_idx].trackers.len();
                        if tracker_count < 4 {
                            self.tracker_status = "4点コーナーピンには4つ以上のトラッカーが必要です".to_owned();
                        } else {
                            let current_frame = self.current_frame;
                            let duration = comp.duration_frames;
                            let end_frame = (current_frame + 60).min(duration.saturating_sub(1));
                            let src_layer = &comp.layers[s_idx];
                            let track = kagari_vfx::core::tracker_engine::QuadTrackData {
                                frames: (current_frame..=end_frame).collect(),
                                corners: (current_frame..=end_frame)
                                    .map(|frame| {
                                        [
                                            src_layer.trackers[0].position.evaluate(frame),
                                            src_layer.trackers[1].position.evaluate(frame),
                                            src_layer.trackers[2].position.evaluate(frame),
                                            src_layer.trackers[3].position.evaluate(frame),
                                        ]
                                    })
                                    .collect(),
                            };
                            let target_layer = &mut comp.layers[t_idx];
                            let applied = kagari_vfx::core::tracker_engine::apply_quad_track_to_corner_pin(
                                target_layer,
                                &track,
                            );
                            if applied {
                                self.tracker_status = "4点プラナー コーナーピンを適用しました".to_owned();
                                self.mark_project_changed();
                            } else {
                                self.tracker_status = "コーナーピンの適用に失敗しました".to_owned();
                            }
                        }
                    }
                } else {
                    self.tracker_status = "適用先ターゲットレイヤーを選択してください".to_owned();
                }
            }
            ("select_motion_tracker", ref val) => {
                let tracker_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(tracker_id) = tracker_opt.as_deref() {
                    if let Some(layer) = self.selected_layer_id.as_deref().and_then(|id| {
                        self.project_model.active_composition().layers.iter().find(|l| l.id == id)
                    }) {
                        if let Some(idx) = layer.trackers.iter().position(|t| t.id == tracker_id) {
                            self.selected_tracker_idx = idx;
                            self.tracker_status = format!("トラッカー「{}」を選択", layer.trackers[idx].name);
                        }
                    }
                }
            }
            ("select_motion_tracker_target", ref val) => {
                let target_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(target_id) = target_opt.as_deref() {
                    self.selected_tracker_target_id = Some(target_id.to_owned());
                    if let Some(layer) = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|l| l.id == target_id)
                    {
                        self.tracker_status = format!("適用先ターゲット「{}」を選択", layer.name);
                    }
                }
            }
            ("reload_ame_ui", _) => {
                let _ = self.reload_ame_ui();
            }
            _ => return false,
        }
        true
    }
}
