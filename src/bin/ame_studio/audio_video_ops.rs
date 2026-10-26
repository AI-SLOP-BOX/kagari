use super::*;

impl AfterEffectsStudio {
    pub(crate) fn update_selected_audio_volume(&mut self, input: &str) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        self.update_audio_volume_for_layer(&id, input);
    }

    pub(crate) fn add_selected_audio_volume_keyframe(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        self.add_audio_volume_keyframe_for_layer(&id);
    }

    pub(crate) fn add_audio_volume_keyframe_for_layer(&mut self, id: &str) {
        let id = id.to_owned();
        let frame = self.current_frame;
        let value = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == id && !layer.locked)
            .and_then(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                    Some(volume.evaluate(frame))
                }
                kagari_vfx::core::timeline::LayerType::Video {
                    audio_wav: Some(_),
                    audio_volume,
                    ..
                } => Some(audio_volume.evaluate(frame)),
                _ => None,
            });
        if let Some(value) = value {
            self.update_audio_volume_for_layer(&id, &value.to_string());
        }
    }

    pub(crate) fn update_audio_volume_for_layer(&mut self, id: &str, input: &str) {
        let Ok(value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        if let Some(layer) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|layer| layer.id == id && layer.locked)
        {
            self.asset_status = format!("「{}」はロック中のため音量を変更できません。", layer.name);
            return;
        }
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let volume = match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume,
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume,
            _ => return,
        };
        let value = value.clamp(-48.0, 12.0);
        volume.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
            self.current_frame,
            value,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_video_speed(&mut self, input: &str) {
        let Ok(value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let kagari_vfx::core::timeline::LayerType::Video { speed, .. } = &mut layer.layer_type
        else {
            return;
        };
        let next = value.clamp(0.05, 10.0);
        if (*speed - next).abs() > f32::EPSILON {
            *speed = next;
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_selected_video_posterize(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Video { .. }
                    )
            })
        else {
            return;
        };
        if let Some(settings) = layer.posterize_time.as_mut() {
            settings.enabled = !settings.enabled;
        } else {
            layer.posterize_time =
                Some(kagari_vfx::core::posterize_time::PosterizeTimeSettings::default());
        }
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_video_posterize_fps(&mut self, input: &str) {
        let Some(value) = input
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| {
                layer.id == id
                    && matches!(
                        &layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Video { .. }
                    )
            })
        else {
            return;
        };
        let Some(settings) = layer.posterize_time.as_mut() else {
            return;
        };
        let next = value.clamp(0.1, 240.0);
        if (settings.target_fps - next).abs() > f32::EPSILON {
            settings.target_fps = next;
            self.mark_project_changed();
        }
    }

    pub(crate) fn toggle_selected_video_time_remap(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        if !matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Video { .. }
        ) {
            return;
        }
        if layer.time_remap.is_some() {
            layer.clear_time_remap();
        } else {
            layer.enable_time_remapping();
        }
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn set_selected_time_remap_keyframe(&mut self, value: &str) {
        let Ok(source_frame) = value.trim().parse::<f32>() else {
            return;
        };
        if !source_frame.is_finite() || source_frame < 0.0 {
            return;
        }
        let Some(id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        let Some(track) = layer.time_remap.as_mut() else {
            return;
        };
        track.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
            self.current_frame,
            source_frame,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
        self.mark_project_changed();
    }

    pub(crate) fn apply_selected_time_remap_preset(&mut self, preset: &str) {
        let Some(id) = self.selected_layer_id.as_deref() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        if !matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Video { .. }
        ) {
            return;
        }
        let source_frame_at_playhead = layer.remap_frame(self.current_frame);
        match preset {
            "reverse" => layer.time_reverse(),
            "freeze" => layer.freeze_at(source_frame_at_playhead),
            "slow" => layer.time_stretch(2.0),
            "fast" => layer.time_stretch(0.5),
            _ => return,
        }
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }
}
