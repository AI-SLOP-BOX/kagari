use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_project_export_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("layer_changed", Some(ActionValue::Text(value))) => {
                self.layer_name = value.clone();
                if let Some(id) = self.selected_layer_id.clone() {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == id && !layer.locked)
                    {
                        layer.name = value;
                        self.mark_project_changed();
                    }
                };
                true
            }
            ("output_changed", Some(ActionValue::Text(value))) => {
                self.output_path = value;
                self.still_overwrite_pending = false;
                self.still_overwrite_identity_pending = None;
                self.still_status.clear();
                true
            }
            ("video_output_changed", Some(ActionValue::Text(value))) => {
                if !self.export_in_progress() {
                    self.video_output_path = value;
                    self.video_overwrite_pending = false;
                    self.video_overwrite_identity_pending = None;
                }
                true
            }
            ("video_export_quality_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                if !self.export_in_progress() && self.video_export_codec_idx % 4 < 2 {
                    self.video_export_quality = value.round().clamp(0.0, 100.0) as u8;
                }
                true
            }
            ("video_export_audio_changed", Some(ActionValue::Checked(value))) => {
                if !self.export_in_progress() {
                    self.video_export_audio_enabled = value;
                }
                true
            }
            ("choose_output_file", _) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG画像", &["png"])
                    .set_file_name("Particle_Study.png")
                    .save_file()
                {
                    self.output_path = path.display().to_string();
                    self.still_overwrite_pending = false;
                    self.still_overwrite_identity_pending = None;
                    self.still_status.clear();
                };
                true
            }
            ("choose_video_output", _) => {
                if !self.export_in_progress() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("動画", &["mp4", "mov", "m4v"])
                        .set_file_name("Kagari_export.mp4")
                        .save_file()
                    {
                        self.video_output_path = path.display().to_string();
                        self.video_overwrite_pending = false;
                        self.video_overwrite_identity_pending = None;
                        self.render_status = "".to_owned();
                    }
                };
                true
            }
            ("render_still", _) => {
                self.export_current_frame_png(false);
                true
            }
            ("render_still_overwrite", _) => {
                self.export_current_frame_png(true);
                true
            }
            ("render_still_cancel_overwrite", _) => {
                self.still_overwrite_pending = false;
                self.still_overwrite_identity_pending = None;
                self.still_status = "PNGの上書きを取り消しました。".to_owned();
                true
            }
            ("render_start", _) => {
                self.start_video_export(false);
                true
            }
            ("cycle_video_export_codec", _) => {
                self.cycle_video_export_codec();
                true
            }
            ("select_video_export_codec", Some(ActionValue::Text(label))) => {
                self.set_video_export_codec(&label);
                true
            }
            ("render_overwrite", _) => {
                self.start_video_export(true);
                true
            }
            ("render_cancel_overwrite", _) => {
                self.video_overwrite_pending = false;
                self.video_overwrite_identity_pending = None;
                self.render_status = "上書きを取り消しました。".to_owned();
                true
            }
            ("render_stop", _) => {
                self.cancel_video_export();
                true
            }
            ("save_project", _) => {
                self.save_current_project();
                true
            }
            ("search_changed", Some(ActionValue::Text(value))) => {
                self.search = value;
                true
            }
            ("clear_scene_search", _) => {
                self.search.clear();
                true
            }
            ("project_search_changed", Some(ActionValue::Text(value))) => {
                self.project_search = value;
                true
            }
            _ => false,
        }
    }
}
