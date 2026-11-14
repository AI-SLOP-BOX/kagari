use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_action(
        &mut self,
        action: HostAction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let prior_frame = self.current_frame;
        if action.name.starts_with("setting_") {
            self.settings_feedback.clear();
        }
        let selected_row_id = action.target.as_ref().and_then(|target| {
            target.segments.iter().find_map(|segment| match segment {
                NodePathSegment::Key(StableKey::String(id)) => Some(id.clone()),
                _ => None,
            })
        });
        let selected_layer_locked = self.selected_layer_id.as_deref().is_some_and(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .any(|layer| layer.id == id && layer.locked)
        });
        if selected_layer_locked && action_edits_selected_layer(&action.name) {
            self.asset_status =
                "選択中のレイヤーはロックされています。解除してから編集してください。".to_owned();
            self.sync_bindings()?;
            return Ok(());
        }
        let action_val = action.value;
        let graph_action = self.handle_graph_timeline_action(
            action.name.as_str(),
            action_val.clone(),
            selected_row_id.clone(),
        )?;
        if graph_action == Some(true) {
            return Ok(());
        }
        match (action.name.as_str(), action_val.clone()) {
            ("go_home", _) => {
                self.screen = Screen::Home;
                self.render_queue = false;
                self.docs_detail_page = false;
            }
            ("go_projects", _) => {
                self.screen = Screen::Projects;
                self.render_queue = false;
            }
            ("go_activity", _) => {
                self.screen = Screen::Activity;
                self.render_queue = false;
            }
            ("go_documents", _) => {
                self.screen = Screen::Documents;
                self.render_queue = false;
                self.docs_detail_page = false;
            }
            ("go_settings", _) => {
                self.settings_open = true;
                self.settings_category = 0;
                self.settings_feedback.clear();
                self.settings_snapshot = Some(self.settings_draft());
            }
            ("docs_project_basics", _) => self.set_document(
                "プロジェクトの基本",
                "新規プロジェクトを選び、名前、幅と高さ、フレームレート、長さを指定します。\n\nHD、Full HD、QHD、4K、縦型のプリセットを選んだ後も、幅や高さは個別に変更できます。作成すると編集画面に移動します。\n\n既存のプロジェクトはホームまたはプロジェクト一覧の「開く」から選びます。",
            ),
            ("docs_editor_workflow", _) => self.set_document(
                "編集画面",
                "左のシーン欄では粒子、文字、長方形、楕円レイヤーを追加できます。選択レイヤーは複製、重なり順の変更、名前変更、削除ができます。レイヤーを選ぶと、右のプロパティ欄に位置や種類に応じた設定が表示されます。\n\n素材欄では画像、音声、動画を読み込めます。タイムライン上のクリップをドラッグすると位置を移動でき、端をドラッグすると長さを調整できます。キーのひし形をクリックするとそのフレームへ移動し、上部の前後ボタンでもキー間を移動できます。\n\n編集内容は保存ボタンまたは ⌘ / Ctrl + S で保存します。保存先がない新規プロジェクトでは、最初の保存時にファイル名を指定します。",
            ),
            ("docs_import", _) => self.set_document(
                "素材の読み込み",
                "画像は PNG、JPEG、WebP、BMP、TIFF、SVG、音声は WAV、MP3、M4A、AAC、FLAC、OGG、AIFF、CAF を読み込めます。PNG・JPEG・WebPの連番は「連番」から先頭画像を選択してください。連番のファイル名は末尾が連続したフレーム番号で、画像サイズもそろっている必要があります。連番と動画はフレーム展開をバックグラウンドで行い、素材欄から中止できます。SVG内の外部ファイル参照は安全のため読み込みません。動画は MP4、MOV、MKV、AVI、WebM、M4V、MPEG、TS などに対応し、読み込みには FFmpeg が必要です。\n\n素材欄の画像・連番・音声・動画ボタンからファイルを指定するか、画像・音声・動画・3Dモデルを素材欄へドラッグ＆ドロップしてください。読み込み後は選択中のコンポジションにレイヤーとして追加されます。",
            ),
            ("docs_export", _) => self.set_document(
                "書き出し",
                "編集画面から「書き出し」を開きます。「現在のフレームを画像保存」では表示中の1フレームをPNGで保存できます。\n\n「動画を書き出す」ではコンポジション全体をH.264のMP4として保存します。動画出力にはFFmpegが必要です。保存先を選び、既存ファイルを置き換える場合は確認画面で上書きを選んでください。進行中の動画出力はキャンセルできます。",
            ),
            ("docs_shortcuts", _) => self.set_document(
                "キーボードショートカット",
                "⌘ / Ctrl + S  編集中のプロジェクトを保存\n⌘ / Ctrl + N  新規プロジェクト設定を開く\n⌘ / Ctrl + M  書き出し画面を開く（編集画面）",
            ),
            _ if self.handle_navigation_setup_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_editor_workspace_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_settings_action(action.name.as_str(), action_val.as_ref()) => {}

            ("open_project", _) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Kagari プロジェクト / テンプレート / OTIO",
                        &["json", "kgtpl", "mogrt", "otio"],
                    )
                    .pick_file()
                {
                    self.request_project_open(&path);
                }
            }
            ("new_project", _) => {
                self.request_new_project();
            }
            ("setup_cancel", _) => {
                self.screen = self.setup_return_screen;
            }
            ("setup_name_changed", Some(ActionValue::Text(value))) => {
                self.setup_name = value;
            }
            ("setup_width_changed", Some(ActionValue::Text(value))) => {
                self.setup_width = value;
                self.setup_error.clear();
            }
            ("setup_height_changed", Some(ActionValue::Text(value))) => {
                self.setup_height = value;
                self.setup_error.clear();
            }
            ("setup_fps_changed", Some(ActionValue::Text(value))) => {
                self.setup_fps = value;
                self.setup_error.clear();
            }
            ("setup_duration_changed", Some(ActionValue::Text(value))) => {
                self.setup_duration_seconds = value;
                self.setup_error.clear();
            }
            ("setup_preset_hd", _) => {
                self.setup_width = "1280".to_owned();
                self.setup_height = "720".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_full_hd", _) => {
                self.setup_width = "1920".to_owned();
                self.setup_height = "1080".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_uhd", _) => {
                self.setup_width = "3840".to_owned();
                self.setup_height = "2160".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_qhd", _) => {
                self.setup_width = "2560".to_owned();
                self.setup_height = "1440".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_vertical", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1920".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_square", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1080".to_owned();
                self.setup_error.clear();
            }
            ("setup_preset_portrait_4_5", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1350".to_owned();
                self.setup_error.clear();
            }
            ("setup_fps_24", _) => {
                self.setup_fps = "24".to_owned();
                self.setup_error.clear();
            }
            ("setup_fps_25", _) => {
                self.setup_fps = "25".to_owned();
                self.setup_error.clear();
            }
            ("setup_fps_30", _) => {
                self.setup_fps = "30".to_owned();
                self.setup_error.clear();
            }
            ("setup_fps_50", _) => {
                self.setup_fps = "50".to_owned();
                self.setup_error.clear();
            }
            ("setup_fps_60", _) => {
                self.setup_fps = "60".to_owned();
                self.setup_error.clear();
            }
            ("setup_create", _) => {
                self.stop_playback();
                let Ok(width) = self.setup_width.trim().parse::<u32>() else {
                    self.setup_error = "幅は1〜16384の整数で入力してください。".to_owned();
                    return self.sync_bindings();
                };
                let Ok(height) = self.setup_height.trim().parse::<u32>() else {
                    self.setup_error = "高さは1〜16384の整数で入力してください。".to_owned();
                    return self.sync_bindings();
                };
                let Ok(fps) = self.setup_fps.trim().parse::<u32>() else {
                    self.setup_error =
                        "フレームレートは1〜240の整数で入力してください。".to_owned();
                    return self.sync_bindings();
                };
                let Ok(duration_seconds) = self.setup_duration_seconds.trim().parse::<u32>() else {
                    self.setup_error = "長さは1〜3600秒の整数で入力してください。".to_owned();
                    return self.sync_bindings();
                };
                if !(1..=16384).contains(&width) {
                    self.setup_error = "幅は1〜16384の範囲で入力してください。".to_owned();
                    return self.sync_bindings();
                }
                if !(1..=16384).contains(&height) {
                    self.setup_error = "高さは1〜16384の範囲で入力してください。".to_owned();
                    return self.sync_bindings();
                }
                if !(1..=240).contains(&fps) {
                    self.setup_error =
                        "フレームレートは1〜240の範囲で入力してください。".to_owned();
                    return self.sync_bindings();
                }
                if !(1..=3600).contains(&duration_seconds) {
                    self.setup_error = "長さは1〜3600秒の範囲で入力してください。".to_owned();
                    return self.sync_bindings();
                }
                self.setup_error.clear();
                self.project_name = if self.setup_name.trim().is_empty() {
                    "無題のプロジェクト".to_owned()
                } else {
                    self.setup_name.trim().to_owned()
                };
                self.project_model = kagari_vfx::core::timeline::Project::empty_for_new_project();
                self.history = kagari_vfx::core::history::ProjectHistory::new(
                    self.project_model.clone(),
                );
                self.history.set_max_history_entries(self.undo_steps);
                self.history_interaction_snapshot = None;
                self.reset_project_asset_browser();
                self.collapsed_scene_layers.clear();
                self.align_selected_layer_ids.clear();
                self.preview_revision = self.preview_revision.wrapping_add(1);
                self.project_file_path = None;
                self.project_is_dirty = true;
                self.save_status = "未保存の変更があります".to_owned();
                self.selected_layer_id = None;
                let composition = self.project_model.active_composition_mut();
                composition.width = width;
                composition.height = height;
                composition.fps = fps;
                composition.duration_frames = fps.saturating_mul(duration_seconds);
                composition.name = self.project_name.clone();
                self.history
                    .restore_current_without_history(self.project_model.clone());
                self.project_fps = fps;
                self.current_frame = 0;
                self.current_time = format_timecode(0, fps);
                self.position_x = f64::from(width) * 0.5;
                self.position_y = f64::from(height) * 0.5;
                self.anchor_x = 0.0;
                self.anchor_y = 0.0;
                self.scale_x = 100.0;
                self.scale_y = 100.0;
                self.opacity = 100.0;
                self.selected_layer = false;
                self.screen = Screen::Editor;
                self.render_queue = false;
            }
            ("open_recent_1", _) => {
                if let Some(item) = self.recent_projects.first() {
                    let path = item.path.clone();
                    self.request_project_open(&path);
                } else {
                    self.request_new_project();
                }
            }
            ("go_export", _) | ("queue_tab", _) => {
                self.screen = Screen::Export;
                self.render_queue = true;
            }
            ("export_mlt", _) => self.export_mlt(),
            ("export_mogrt", _) => self.export_mogrt(),
            ("export_lottie", _) => self.export_lottie(),
            ("export_subtitles", _) => self.export_subtitles(),
            ("export_otio", _) => self.export_otio(),
            ("export_gif", _) => self.export_gif(),
            ("export_png_sequence", _) => self.export_png_sequence(),
            ("export_exr_sequence", _) => self.export_exr_sequence(),
            ("workspace_left_resized", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.workspace_left_ratios[self.workspace.index()] = value.clamp(0.16, 0.36);
            }
            ("workspace_center_resized", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.workspace_center_ratios[self.workspace.index()] = value.clamp(0.54, 0.84);
            }
            ("workspace_vertical_resized", Some(ActionValue::Number(value)))
                if value.is_finite() =>
            {
                self.workspace_vertical_ratios[self.workspace.index()] = value.clamp(0.20, 0.80);
            }
            ("workspace_reset_layout", _) => {
                let index = self.workspace.index();
                self.workspace_left_ratios[index] = WORKSPACE_LEFT_DEFAULTS[index];
                self.workspace_center_ratios[index] = WORKSPACE_CENTER_DEFAULTS[index];
                self.workspace_vertical_ratios[index] = WORKSPACE_VERTICAL_DEFAULT;
            }
            ("sidebar_vertical_resized", _) => {}
            ("viewport_cycle_channel", _) => {
                self.viewport_channel_mode = match self.viewport_channel_mode.as_str() {
                    "RGBA" => "R",
                    "R" => "G",
                    "G" => "B",
                    "B" => "A",
                    "A" => "Luma",
                    _ => "RGBA",
                }
                .to_string();
            }
            ("viewport_cycle_exposure", _) => {
                self.viewport_exposure_ev = match self.viewport_exposure_ev as i32 {
                    0 => 1.0,
                    1 => 2.0,
                    2 => -1.0,
                    -1 => -2.0,
                    _ => 0.0,
                };
            }
            ("viewport_toggle_wipe", _) => {
                self.viewport_wipe_enabled = !self.viewport_wipe_enabled;
            }
            _ if self.handle_composition_action(action.name.as_str(), action_val.as_ref(), selected_row_id.as_deref()) => {}
            ("add_solid_layer", _) => self.add_solid_layer(),
            ("add_scene_camera", _) => self.add_scene_camera(),
            ("add_scene_light", _) => self.add_scene_light(),
            ("activate_selected_camera", _) => self.activate_selected_scene_camera(),
            ("selected_camera_fov_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_camera("fov", value),
            ("selected_camera_focus_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_camera("focus", value),
            ("selected_camera_aperture_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_camera("aperture", value),
            ("toggle_selected_camera_dof", _) => self.toggle_selected_scene_camera_dof(),
            ("selected_camera_dof_max_blur_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_camera("dof_blur", value),
            ("keyframe_camera_fov", _) => self.add_selected_scene_camera_keyframe(GraphChannel::CameraFov),
            ("keyframe_camera_focus", _) => self.add_selected_scene_camera_keyframe(GraphChannel::CameraFocus),
            ("keyframe_camera_aperture", _) => self.add_selected_scene_camera_keyframe(GraphChannel::CameraAperture),
            ("keyframe_camera_dof_blur", _) => self.add_selected_scene_camera_keyframe(GraphChannel::CameraDofBlur),
            ("selected_light_intensity_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_light_intensity(value),
            ("keyframe_light_intensity", _) => self.keyframe_selected_scene_light_intensity(),
            ("toggle_selected_light_shadows", _) => self.toggle_selected_scene_light_shadows(),
            ("cycle_selected_light_type", _) => self.cycle_selected_scene_light_type(),
            ("selected_light_color_changed", Some(ActionValue::Text(value))) => self.update_selected_scene_light_color(&value),
            ("selected_light_cone_angle_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_light_property("cone_angle", value),
            ("selected_light_cone_feather_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_light_property("cone_feather", value),
            ("selected_light_falloff_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_light_property("falloff", value),
            ("selected_light_shadow_darkness_changed", Some(ActionValue::Number(value))) => self.update_selected_scene_light_property("shadow_darkness", value),
            ("solid_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_solid_color(&value)
            }
            ("toggle_layer_style", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.as_str()),
                    _ => selected_row_id.as_deref(),
                };
                if let Some(style_id) = id_opt {
                    self.toggle_selected_layer_style(style_id);
                }
            }
            ("layer_style_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter_id) = selected_row_id.as_deref() {
                    self.update_selected_layer_style_parameter(parameter_id, &value);
                }
            }
            ("select_layer", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    if self.selected_layer_id.as_deref() != Some(id.as_str()) {
                        self.selected_layer_marker_frame = None;
                    }
                    self.selected_layer_id = Some(id);
                    self.sync_selected_layer_controls();
                }
            }
            ("layer_in_frame_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_layer_timing(&value, true);
            }
            ("layer_out_frame_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_layer_timing(&value, false);
            }
            ("layer_in_to_playhead", _) => {
                let frame = self.current_frame.to_string();
                self.update_selected_layer_timing(&frame, true);
            }
            ("layer_out_to_playhead", _) => {
                let frame = self.current_frame.to_string();
                self.update_selected_layer_timing(&frame, false);
            }
            ("cycle_layer_blend_mode", _) => self.cycle_layer_blend_mode(),
            ("cycle_layer_track_matte", _) => self.cycle_layer_track_matte(),
            ("select_track_matte_source", ref val) => {
                let source_id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.as_str()),
                    _ => selected_row_id.as_deref(),
                };
                self.select_track_matte_source(source_id_opt);
            }
            ("clear_track_matte_source", _) => self.clear_track_matte_source(),
            ("toggle_frame_blending", _) => self.toggle_selected_layer_flag(LayerToggle::FrameBlending),
            ("toggle_selected_layer_motion_blur", _) => self.toggle_selected_layer_flag(LayerToggle::MotionBlur),
            ("toggle_preserve_transparency", _) => self.toggle_selected_layer_flag(LayerToggle::PreserveTransparency),
            ("toggle_guide_layer", _) => self.toggle_selected_layer_flag(LayerToggle::Guide),
            ("toggle_shy_layer", _) => self.toggle_selected_layer_flag(LayerToggle::Shy),
            ("toggle_layer_effects", _) => self.toggle_selected_layer_flag(LayerToggle::Effects),
            ("cycle_auto_orient", _) => self.cycle_selected_auto_orient(),
            ("auto_orient_point_x_changed", Some(ActionValue::Text(value))) => self.update_auto_orient_target(&value, 0),
            ("auto_orient_point_y_changed", Some(ActionValue::Text(value))) => self.update_auto_orient_target(&value, 1),
            ("cycle_layer_label_color", _) => {
                if let Some(layer_id) = selected_row_id
                    .as_deref()
                    .or(self.selected_layer_id.as_deref())
                    .map(str::to_owned)
                {
                    self.cycle_layer_label_color_for(&layer_id);
                }
            }
            ("cycle_constraint_horizontal", _) => self.cycle_selected_layer_constraint(true),
            ("cycle_constraint_vertical", _) => self.cycle_selected_layer_constraint(false),
            ("add_particle_layer", _) => self.add_particle_layer(),
            ("add_adjustment_layer", _) => self.add_utility_layer(
                "調整レイヤー",
                kagari_vfx::core::timeline::LayerType::AdjustmentLayer,
            ),
            ("add_null_layer", _) => self.add_utility_layer(
                "ヌル",
                kagari_vfx::core::timeline::LayerType::Null,
            ),
            ("add_text_layer", _) => self.add_text_layer(),
            ("import_subtitles", _) => self.import_subtitles(),
            ("toggle_text_on_path", _) => self.toggle_selected_text_on_path(),
            ("add_shape_layer", _) => self.add_shape_layer(),
            ("add_ellipse_layer", _) => self.add_ellipse_layer(),
            ("add_star_layer", _) => self.add_star_layer(),
            ("add_polygon_layer", _) => self.add_polygon_layer(),
            ("import_image_layer", _) => self.import_image_layer(),
            ("import_image_sequence_layer", _) => self.import_image_sequence_layer(),
            ("import_model_layer", _) => self.import_model_layer(),
            ("import_audio_layer", _) => self.import_audio_layer(),
            ("import_video_layer", _) => self.import_video_layer(),
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
                    self.request_project_open(&path);
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
            _ if graph_action == Some(false) => {},
            _ if self.handle_timeline_marker_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ if self.handle_mask_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ if self.handle_scene_layer_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ if self.handle_layer_keyframe_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_project_asset_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ if self.handle_project_export_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_time_transform_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_effect_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ if self.handle_text_shape_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
            _ => {
                if !self.handle_playback_action(action.name.as_str(), action_val.as_ref()) {
                    log::warn!("[kagari-studio] unhandled action: `{}` with payload {:?}", action.name, action_val);
                    #[cfg(debug_assertions)]
                    {
                        // In debug builds, warn loudly about missing action handlers
                        eprintln!("[kagari-studio:debug] unhandled action: `{}` with payload {:?}", action.name, action_val);
                    }
                }
            }
        }

        if self.current_frame != prior_frame {
            self.sync_selected_layer_controls();
        }
        self.sync_bindings()
    }
}
