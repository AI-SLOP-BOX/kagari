use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_selected_layer_controls(&mut self) {
        let selected_effect_exists = self.selected_layer_id.as_deref().is_some_and(|layer_id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .is_some_and(|layer| {
                    self.selected_effect_id.as_deref().is_some_and(|effect_id| {
                        layer.effects.iter().any(|effect| effect.id == effect_id)
                    })
                })
        });
        if !selected_effect_exists {
            self.selected_effect_id = None;
        }
        let selected = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
                .map(|layer| {
                    (
                        layer.name.clone(),
                        layer.transform.position.evaluate(self.current_frame),
                        layer.transform.anchor_point.evaluate(self.current_frame),
                        layer.transform.rotation.evaluate(self.current_frame),
                        layer.transform.scale.evaluate(self.current_frame),
                        layer.transform.opacity.evaluate(self.current_frame),
                    )
                })
        });
        if let Some((name, position, anchor, rotation, scale, opacity)) = selected {
            self.layer_name = name;
            self.position_x = f64::from(position[0]);
            self.position_y = f64::from(position[1]);
            self.anchor_x = f64::from(anchor[0]);
            self.anchor_y = f64::from(anchor[1]);
            self.rotation_degrees = f64::from(rotation).to_degrees();
            self.scale_x = f64::from(scale[0]);
            self.scale_y = f64::from(scale[1]);
            self.opacity = f64::from(opacity);
            self.selected_layer = true;
        } else {
            self.selected_layer = false;
        }
    }

    pub(crate) fn mark_project_changed(&mut self) {
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.save_status = "未保存の変更があります".to_owned();
        let interaction_active = self.timeline_clip_drag.is_some()
            || self.graph_key_drag.is_some()
            || self.viewport_drag.is_some()
            || self.mask_vertex_drag.is_some();
        if interaction_active && self.history_interaction_snapshot.is_none() {
            self.history_interaction_snapshot = Some(self.history.current().clone());
        } else if !interaction_active && self.history_interaction_snapshot.is_none() {
            self.history
                .commit_action(self.project_model.clone(), "編集");
        }
        let _ = self
            .ui
            .set_binding("can_undo", Value::Bool(self.history.can_undo()));
        let _ = self
            .ui
            .set_binding("can_redo", Value::Bool(self.history.can_redo()));
    }

    pub(crate) fn finish_history_interaction(&mut self, action_name: &str) {
        if let Some(before) = self.history_interaction_snapshot.take() {
            self.history
                .commit_drag_action(before, self.project_model.clone(), action_name);
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn restore_history_project(&mut self, project: kagari_vfx::core::timeline::Project) {
        self.history_interaction_snapshot = None;
        self.timeline_clip_drag = None;
        self.graph_key_drag = None;
        self.viewport_drag = None;
        self.mask_vertex_drag = None;
        self.project_model = project;
        let composition = self.project_model.active_composition();
        self.project_fps = composition.fps.max(1);
        self.current_frame = self.current_frame.min(composition.duration_frames.saturating_sub(1));
        self.current_time = format_timecode(self.current_frame, self.project_fps);
        let layer_ids = composition
            .layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        if self
            .selected_layer_id
            .as_deref()
            .is_some_and(|id| !layer_ids.contains(id))
        {
            self.selected_layer_id = None;
            self.selected_effect_id = None;
        }
        if self.selected_effect_id.as_deref().is_some_and(|effect_id| {
            self.selected_layer_id
                .as_deref()
                .and_then(|layer_id| {
                    self.project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == layer_id)
                })
                .is_none_or(|layer| !layer.effects.iter().any(|effect| effect.id == effect_id))
        }) {
            self.selected_effect_id = None;
        }
        self.align_selected_layer_ids
            .retain(|id| layer_ids.contains(id.as_str()));
        self.collapsed_scene_layers
            .retain(|id| layer_ids.contains(id.as_str()));
        if self
            .selected_project_asset_id
            .as_deref()
            .is_some_and(|id| !self.project_model.assets.iter().any(|asset| asset.id == id))
        {
            self.selected_project_asset_id = None;
        }
        self.preview_revision = self.preview_revision.wrapping_add(1);
        self.save_status = "未保存の変更があります".to_owned();
        self.sync_selected_layer_controls();
        self.sync_bindings_logged();
    }

    pub(crate) fn undo_project(&mut self) {
        if let Some(project) = self.history.undo().cloned() {
            self.restore_history_project(project);
        }
    }

    pub(crate) fn redo_project(&mut self) {
        if let Some(project) = self.history.redo().cloned() {
            self.restore_history_project(project);
        }
    }

    pub(crate) fn scale_selected_media_to_composition(&mut self, fill_frame: bool) {
        use kagari_vfx::core::timeline::{Animatable, LayerType};

        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        let composition = self.project_model.active_composition();
        let Some(layer_for_fit) = composition
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
        else {
            return;
        };
        if layer_for_fit.locked {
            self.asset_status = format!(
                "「{}」はロック中のためサイズを変更できません。",
                layer_for_fit.name
            );
            return;
        }
        let Some(path) = (match &layer_for_fit.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Video { frames_dir, .. } => Some(
                    kagari_vfx::core::video_import::frame_path_in_dir(frames_dir, 0)
                        .to_string_lossy()
                        .into_owned(),
                ),
                _ => None,
            })
        else {
            return;
        };
        let Some((image_width, image_height)) =
            kagari_vfx::core::image_cache::image_dimensions(&path)
        else {
            self.asset_status = "素材の寸法を取得できませんでした。ファイルを確認してください。".to_owned();
            return;
        };
        let (comp_width, comp_height, frame) = (
            composition.width.max(1),
            composition.height.max(1),
            self.current_frame,
        );
        let width_scale = comp_width as f32 / image_width.max(1) as f32;
        let height_scale = comp_height as f32 / image_height.max(1) as f32;
        let media_scale = if fill_frame {
            width_scale.max(height_scale)
        } else {
            width_scale.min(height_scale)
        } * 100.0;
        let center = [comp_width as f32 * 0.5, comp_height as f32 * 0.5];
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        if matches!(&layer.transform.position, Animatable::Animated(_)) {
            layer.transform.position.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
                frame,
                center,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        } else {
            layer.transform.position = Animatable::new_constant(center);
        }
        let fitted_scale = [media_scale, media_scale];
        if matches!(&layer.transform.scale, Animatable::Animated(_)) {
            layer.transform.scale.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
                frame,
                fitted_scale,
                kagari_vfx::core::keyframe::InterpolationType::Linear,
            ));
        } else {
            layer.transform.scale = Animatable::new_constant(fitted_scale);
        }
        self.mark_project_changed();
        self.sync_selected_layer_controls();
    }

}
