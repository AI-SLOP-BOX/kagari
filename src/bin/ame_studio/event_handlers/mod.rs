use super::*;

mod secondary_actions;


impl AfterEffectsStudio {
    pub(crate) fn create_project_with_settings(
        &mut self,
        name: String,
        width: u32,
        height: u32,
        fps: u32,
        duration_seconds: u32,
    ) {
        self.stop_playback();
        self.wait_for_autosave();
        self.cancel_video_import();
        self.invalidate_roto_operations();
        self.project_generation = self.project_generation.wrapping_add(1);
        self.setup_error.clear();
        self.setup_width = width.to_string();
        self.setup_height = height.to_string();
        self.setup_fps = fps.to_string();
        self.setup_duration_seconds = duration_seconds.to_string();
        self.project_name = if name.trim().is_empty() {
            "無題のプロジェクト".to_owned()
        } else {
            name.trim().to_owned()
        };
        self.project_model = kagari_vfx::core::timeline::Project::empty_for_new_project();
        self.history =
            kagari_vfx::core::history::ProjectHistory::new(self.project_model.clone());
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
            _ if self.handle_engine_hub_action(action.name.as_str()) => {}
            _ if self.handle_editor_workspace_action(action.name.as_str(), action_val.clone()) => {}
            _ if self.handle_settings_action(action.name.as_str(), action_val.as_ref()) => {}

            ("open_project", _) => {
                #[cfg(target_os = "macos")]
                if let Some(sender) = self.app_event_sender.clone() {
                    std::thread::spawn(move || {
                        if let Ok(output) = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(r#"POSIX path of (choose file with prompt "プロジェクトを開く" of type {"json", "kgtpl", "mogrt", "otio"})"#)
                            .output()
                        {
                            if output.status.success() {
                                let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                                if !path.is_empty() {
                                    let _ = sender.send(format!("open_project_path:{path}"));
                                }
                            }
                        }
                    });
                    return Ok(());
                }
                #[cfg(not(target_os = "macos"))]
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
                self.create_project_with_settings(
                    self.setup_name.clone(),
                    width,
                    height,
                    fps,
                    duration_seconds,
                );
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
                    "Luma" => "Zebra",
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
            ("select_light_type", Some(ActionValue::Text(label))) => {
                self.set_selected_scene_light_type(&label);
            }
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
            ("layer_style_parameter_slider_changed", Some(ActionValue::Number(norm))) => {
                if let Some(parameter_id) = selected_row_id.as_deref() {
                    let (min_v, max_v) = if parameter_id.ends_with(".angle") {
                        (-180.0_f32, 180.0_f32)
                    } else {
                        (0.0_f32, 100.0_f32)
                    };
                    let actual = min_v + (norm.clamp(0.0, 1.0) as f32) * (max_v - min_v);
                    self.update_selected_layer_style_parameter(
                        parameter_id,
                        &format!("{actual:.2}"),
                    );
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
            ("select_layer_blend_mode", Some(ActionValue::Text(key))) => {
                self.set_layer_blend_mode(&key);
            }
            ("cycle_layer_track_matte", _) => self.cycle_layer_track_matte(),
            ("select_layer_track_matte", Some(ActionValue::Text(key))) => {
                self.set_layer_track_matte(&key);
            }
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
            ("select_auto_orient", Some(ActionValue::Text(label))) => {
                self.set_selected_auto_orient(&label);
            }
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
            ("select_layer_label_color", Some(ActionValue::Text(label))) => {
                if let Some(layer_id) = selected_row_id
                    .as_deref()
                    .or(self.selected_layer_id.as_deref())
                    .map(str::to_owned)
                {
                    self.set_layer_label_color_for(&layer_id, &label);
                }
            }
            ("cycle_constraint_horizontal", _) => self.cycle_selected_layer_constraint(true),
            ("select_constraint_horizontal", Some(ActionValue::Text(label))) => {
                self.set_selected_layer_constraint(true, &label);
            }
            ("cycle_constraint_vertical", _) => self.cycle_selected_layer_constraint(false),
            ("select_constraint_vertical", Some(ActionValue::Text(label))) => {
                self.set_selected_layer_constraint(false, &label);
            }
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
            _ if self.handle_secondary_action(
                action.name.as_str(),
                action_val.clone(),
                selected_row_id.clone(),
            ) => {}
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
