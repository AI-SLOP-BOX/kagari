use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_settings_action(
        &mut self,
        action_name: &str,
        action_value: Option<&ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("go_settings", _) => {
                self.settings_open = true;
                self.settings_category = 0;
                self.settings_feedback.clear();
                self.settings_snapshot = Some(self.settings_draft());
                true
            }
            ("close_settings", _) | ("settings_cancel", _) => {
                if let Some(draft) = self.settings_snapshot.take() {
                    self.restore_settings_draft(draft);
                    self.sync_bindings_logged();
                }
                self.settings_feedback.clear();
                self.settings_open = false;
                true
            }
            ("settings_apply", _) => {
                match self.commit_settings() {
                    Ok(()) => {
                        self.settings_feedback.clear();
                        self.settings_snapshot = None;
                        self.settings_open = false;
                    }
                    Err(error) => {
                        self.settings_feedback = format!("設定を保存できませんでした: {error}");
                    }
                }
                true
            }
            ("settings_general_select", _) => {
                self.settings_category = 0;
                true
            }
            ("settings_performance_select", _) => {
                self.settings_category = 1;
                true
            }
            ("settings_media_select", _) => {
                self.settings_category = 2;
                true
            }
            ("settings_display_select", _) => {
                self.settings_category = 3;
                true
            }
            ("settings_playback_select", _) => {
                self.settings_category = 4;
                true
            }
            ("settings_shortcuts_select", _) => {
                self.settings_category = 5;
                true
            }
            ("settings_filepath_select", _) => {
                self.settings_category = 6;
                true
            }
            ("settings_external_select", _) => {
                self.settings_category = 7;
                self.refresh_openfx_plugins();
                true
            }
            ("settings_experimental_select", _) => {
                self.settings_category = 8;
                true
            }
            ("settings_plugins_select", _) => {
                self.settings_category = 7;
                self.refresh_openfx_plugins();
                true
            }
            ("settings_plugins_refresh", _) => {
                self.refresh_openfx_plugins();
                true
            }
            ("settings_projects_select", _) => {
                self.settings_category = 6;
                true
            }
            ("settings_cache_select", _) => {
                self.settings_category = 2;
                true
            }
            ("settings_interface_select", _) => {
                self.settings_category = 3;
                true
            }
            ("settings_license_select", _) => {
                self.settings_category = 8;
                true
            }
            ("settings_defaults", _) => {
                self.reset_settings_draft();
                self.settings_feedback = "デフォルトに戻しました。「OK」で保存します。".to_owned();
                true
            }
            ("__ame_binding_update_accent_color", Some(ActionValue::Text(value))) => {
                if parse_hex_color(value).is_some() {
                    self.accent_color = value.clone();
                }
                true
            }
            ("setting_accent_changed", Some(ActionValue::Text(value))) => {
                if parse_hex_color(value).is_some() {
                    self.accent_color = value.clone();
                }
                true
            }
            ("setting_renderer_changed", Some(ActionValue::Text(value))) => {
                self.renderer = value.clone();
                self.gpu_enabled = value != "cpu";
                true
            }
            ("setting_background_render_changed", Some(ActionValue::Checked(value))) => {
                self.background_render = *value;
                true
            }
            ("setting_preview_quality_changed", Some(ActionValue::Text(value))) => {
                self.preview_quality = value.clone();
                true
            }
            ("setting_high_quality_changed", Some(ActionValue::Checked(value))) => {
                self.high_quality_preview = *value;
                true
            }
            ("setting_preview_resolution_changed", Some(ActionValue::Text(value))) => {
                self.preview_resolution = value.clone();
                true
            }
            ("setting_still_duration_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.still_duration_secs = value.clamp(0.1, 600.0);
                true
            }
            ("setting_image_sequence_changed", Some(ActionValue::Checked(value))) => {
                self.image_sequence_detect = *value;
                true
            }
            ("setting_proxy_auto_changed", Some(ActionValue::Checked(value))) => {
                self.proxy_auto_generate = *value;
                true
            }
            ("setting_hardware_decode_changed", Some(ActionValue::Checked(value))) => {
                self.hardware_decode = *value;
                true
            }
            ("setting_preferred_decoder_changed", Some(ActionValue::Text(value))) => {
                self.preferred_decoder = value.clone();
                true
            }
            ("setting_media_color_space_changed", Some(ActionValue::Text(value))) => {
                self.media_color_space = value.clone();
                true
            }
            ("setting_grid_visible_changed", Some(ActionValue::Checked(value))) => {
                self.grid_visible = *value;
                true
            }
            ("setting_grid_spacing_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.grid_spacing = value.clamp(8.0, 512.0);
                true
            }
            ("setting_safe_area_changed", Some(ActionValue::Checked(value))) => {
                self.safe_area_visible = *value;
                true
            }
            ("setting_color_management_changed", Some(ActionValue::Text(value))) => {
                self.color_management = value.clone();
                true
            }
            ("setting_gamma_changed", Some(ActionValue::Text(value))) => {
                self.gamma = value.clone();
                true
            }
            ("setting_hdr_changed", Some(ActionValue::Checked(value))) => {
                self.hdr_enabled = *value;
                true
            }
            ("setting_playback_quality_changed", Some(ActionValue::Text(value))) => {
                self.playback_quality = value.clone();
                true
            }
            ("setting_playback_start_changed", Some(ActionValue::Text(value))) => {
                self.playback_start_mode = value.clone();
                true
            }
            ("setting_pre_roll_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.pre_roll_secs = value.clamp(0.0, 10.0);
                true
            }
            ("setting_post_roll_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.post_roll_secs = value.clamp(0.0, 10.0);
                true
            }
            ("setting_volume_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.playback_volume = value.clamp(0.0, 100.0);
                true
            }
            ("setting_playback_cache_changed", Some(ActionValue::Checked(value))) => {
                self.playback_cache = *value;
                true
            }
            ("setting_scrub_audio_changed", Some(ActionValue::Checked(value))) => {
                self.scrub_audio = *value;
                true
            }
            ("setting_show_peaks_changed", Some(ActionValue::Checked(value))) => {
                self.show_audio_peaks = *value;
                true
            }
            ("setting_loop_changed", Some(ActionValue::Checked(value))) => {
                self.playback_loop = *value;
                true
            }
            ("setting_auto_project_names_changed", Some(ActionValue::Checked(value))) => {
                self.auto_project_names = *value;
                true
            }
            ("setting_shortcut_filter_changed", Some(ActionValue::Text(value))) => {
                self.shortcut_filter = value.clone();
                true
            }
            ("setting_project_dir_changed", Some(ActionValue::Text(value))) => {
                self.project_dir = value.clone();
                true
            }
            ("setting_project_dir_browse", _) => {
                if let Some(path) = pick_directory(&self.project_dir) {
                    self.project_dir = path;
                }
                true
            }
            ("setting_preview_cache_changed", Some(ActionValue::Text(value))) => {
                self.preview_cache_dir = value.clone();
                true
            }
            ("setting_preview_cache_browse", _) => {
                if let Some(path) = pick_directory(&self.preview_cache_dir) {
                    self.preview_cache_dir = path;
                }
                true
            }
            ("setting_temp_dir_changed", Some(ActionValue::Text(value))) => {
                self.temp_dir = value.clone();
                true
            }
            ("setting_temp_dir_browse", _) => {
                if let Some(path) = pick_directory(&self.temp_dir) {
                    self.temp_dir = path;
                }
                true
            }
            ("setting_autosave_dir_changed", Some(ActionValue::Text(value))) => {
                self.autosave_dir = value.clone();
                true
            }
            ("setting_autosave_dir_browse", _) => {
                if let Some(path) = pick_directory(&self.autosave_dir) {
                    self.autosave_dir = path;
                }
                true
            }
            ("setting_backup_dir_changed", Some(ActionValue::Text(value))) => {
                self.backup_dir = value.clone();
                true
            }
            ("setting_backup_dir_browse", _) => {
                if let Some(path) = pick_directory(&self.backup_dir) {
                    self.backup_dir = path;
                }
                true
            }
            ("setting_backup_generations_changed", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.backup_generations = (value.round() as u64).clamp(1, 100);
                true
            }
            ("setting_asset_library_changed", Some(ActionValue::Text(value))) => {
                self.asset_library_dir = value.clone();
                true
            }
            ("setting_asset_library_browse", _) => {
                if let Some(path) = pick_directory(&self.asset_library_dir) {
                    self.asset_library_dir = path;
                }
                true
            }
            ("setting_restore_changed", Some(ActionValue::Checked(value))) => {
                self.restore_previous_state = *value;
                true
            }
            ("setting_confirm_close_changed", Some(ActionValue::Checked(value))) => {
                self.confirm_close_project = *value;
                true
            }
            ("setting_confirm_exit_changed", Some(ActionValue::Checked(value))) => {
                self.confirm_exit = *value;
                true
            }
            ("setting_adaptive_preview_changed", Some(ActionValue::Checked(value))) => {
                self.adaptive_preview = *value;
                true
            }
            ("setting_audio_preview_changed", Some(ActionValue::Checked(value))) => {
                self.audio_preview = *value;
                true
            }
            ("setting_home_changed", Some(ActionValue::Checked(value))) => {
                self.show_home_on_startup = *value;
                true
            }
            ("setting_autosave_interval_changed", Some(ActionValue::Text(value))) => {
                if let Ok(seconds) = value.parse::<u64>() {
                    self.autosave_secs = seconds;
                }
                true
            }
            ("setting_memory_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.memory_limit_gb = value.round().clamp(1.0, 64.0);
                true
            }
            ("setting_undo_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.undo_steps = (value.round() as usize).clamp(10, 500);
                true
            }
            ("setting_recovery_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.recovery_slots = (value.round() as usize).clamp(1, 50);
                true
            }
            ("setting_gpu_changed", Some(ActionValue::Checked(value))) => {
                self.gpu_enabled = *value;
                true
            }
            ("setting_cache_path_changed", Some(ActionValue::Text(value))) => {
                self.cache_path = value.clone();
                self.cache_path_status.clear();
                true
            }
            ("setting_cache_path_browse", _) => {
                let initial = expand_home_path(self.cache_path.trim());
                let initial = if initial.is_dir() {
                    initial
                } else {
                    std::env::var_os("HOME")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(std::env::temp_dir)
                };
                if let Some(path) = rfd::FileDialog::new().set_directory(initial).pick_folder() {
                    self.cache_path = path.display().to_string();
                    self.cache_path_status = "変更は「適用」で保存されます。".to_owned();
                }
                true
            }
            ("setting_cache_limit_changed", Some(ActionValue::Text(value))) => {
                self.cache_limit = value.clone();
                true
            }
            ("clear_cache", _) => {
                self.clear_cache_confirm = true;
                true
            }
            ("dismiss_clear_cache", _) => {
                self.clear_cache_confirm = false;
                true
            }
            ("confirm_clear_cache", _) => {
                self.clear_cache_confirm = false;
                if self.proxy_generation.is_some() {
                    self.cache_path_status =
                        "プロキシ生成中はキャッシュを削除できません。生成完了後に再試行してください。"
                            .to_owned();
                    return true;
                }
                kagari_vfx::core::frame_cache::disk_cache::clear_all();
                let preview_cache_dir = if self.preview_cache_dir.trim().is_empty() {
                    default_preview_cache_dir()
                } else {
                    self.preview_cache_dir.clone()
                };
                let preview_cache = expand_home_path(preview_cache_dir.trim()).join("LayerProxies");
                self.cache_path_status = match std::fs::remove_dir_all(&preview_cache) {
                    Ok(()) => "プレビューキャッシュを削除しました。".to_owned(),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        "プレビューキャッシュを削除しました。".to_owned()
                    }
                    Err(error) => {
                        format!("一部のプレビューキャッシュを削除できませんでした: {error}")
                    }
                };
                kagari_vfx::core::frame_cache::bump_version();
                self.cache_usage_bytes =
                    kagari_vfx::core::frame_cache::disk_cache::current_disk_usage();
                true
            }
            ("exit_app", _) => {
                if self.confirm_exit {
                    self.exit_confirm_open = true;
                } else {
                    self.exit_requested = true;
                }
                true
            }
            ("confirm_exit_app", _) => {
                self.exit_confirm_open = false;
                self.exit_requested = true;
                true
            }
            ("save_and_exit_app", _) => {
                self.save_current_project();
                if !self.project_is_dirty {
                    self.exit_confirm_open = false;
                    self.exit_requested = true;
                }
                true
            }
            ("cancel_exit_app", _) => {
                self.exit_confirm_open = false;
                true
            }
            ("confirm_project_switch", _) => {
                self.complete_pending_project_switch();
                true
            }
            ("save_and_confirm_project_switch", _) => {
                self.save_current_project();
                if self.save_status == "保存しました" {
                    self.complete_pending_project_switch();
                }
                true
            }
            ("cancel_project_switch", _) => {
                self.project_switch_confirm_open = false;
                self.pending_project_path = None;
                self.pending_new_project = false;
                true
            }
            _ => false,
        }
    }
}
