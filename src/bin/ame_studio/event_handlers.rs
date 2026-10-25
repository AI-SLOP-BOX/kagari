use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_action(&mut self, action: HostAction) -> Result<(), Box<dyn std::error::Error>> {
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
            _ if self.handle_settings_action(action.name.as_str(), action_val.as_ref()) => {}

            ("open_project", _) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Kagari Project", &["json"])
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
                self.save_status.clear();
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
            _ if self.handle_composition_action(action.name.as_str(), action_val.as_ref(), selected_row_id.as_deref()) => {}
            ("back_to_editor", _) | ("timeline_tab", _) => {
                self.screen = Screen::Editor;
                self.render_queue = false;
            }
            ("tool_select", _) => self.selected_tool = "Selection",
            ("tool_hand", _) => self.selected_tool = "Hand",
            ("tool_zoom", _) => self.selected_tool = "Zoom",
            ("tool_text", _) => self.selected_tool = "Text",
            ("tool_shape", _) => self.selected_tool = "Shape",
            ("tool_pen", _) => self.selected_tool = "Pen",
            ("workspace_default", _) => {
                self.workspace = Workspace::Default;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_composition", _) => {
                self.workspace = Workspace::Composition;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_scene", _) => {
                self.workspace = Workspace::Scene;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_timeline", _) => {
                self.workspace = Workspace::Timeline;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_animation", _) => {
                self.workspace = Workspace::Animation;
                self.timeline_view = TimelineView::Animation;
            }
            ("workspace_graph", _) => {
                self.workspace = Workspace::Graph;
                self.timeline_view = TimelineView::Graph;
            }
            ("workspace_effects", _) => {
                self.workspace = Workspace::Effects;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_audio", _) => {
                self.workspace = Workspace::Audio;
                self.timeline_view = TimelineView::Audio;
            }
            ("workspace_time_remap", _) => {
                self.workspace = Workspace::TimeRemap;
                self.timeline_view = TimelineView::Media;
            }
            ("workspace_mask", _) => {
                self.workspace = Workspace::Mask;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_assets", _) => {
                self.workspace = Workspace::Assets;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_text", _) => {
                self.workspace = Workspace::Text;
                self.timeline_view = TimelineView::Text;
            }
            ("workspace_layer_style", _) => {
                self.workspace = Workspace::LayerStyle;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_color", _) => {
                self.workspace = Workspace::Color;
                self.timeline_view = TimelineView::All;
            }
            ("workspace_camera_light", _) => {
                self.workspace = Workspace::CameraLight;
                self.timeline_view = TimelineView::All;
            }
            ("time_remap_reverse", _) => self.apply_selected_time_remap_preset("reverse"),
            ("time_remap_freeze", _) => self.apply_selected_time_remap_preset("freeze"),
            ("time_remap_slow", _) => self.apply_selected_time_remap_preset("slow"),
            ("time_remap_fast", _) => self.apply_selected_time_remap_preset("fast"),
            ("time_remap_value_changed", Some(ActionValue::Text(value))) => self.set_selected_time_remap_keyframe(&value),
            ("graph_time_remap", _) => {
                if self.selected_layer_id.as_deref().is_some_and(|id| {
                    self.project_model.active_composition().layers.iter().any(|layer| {
                        layer.id == id
                            && !layer.locked
                            && layer.time_remap.is_some()
                            && matches!(&layer.layer_type, kagari_vfx::core::timeline::LayerType::Video { .. })
                    })
                }) {
                    self.graph_channel = GraphChannel::TimeRemap;
                    self.timeline_view = TimelineView::Graph;
                    self.workspace = Workspace::Graph;
                }
            }
            ("timeline_view_all", _) => self.timeline_view = TimelineView::All,
            ("timeline_view_animation", _) => self.timeline_view = TimelineView::Animation,
            ("timeline_view_current_keyframes", _) => {
                self.timeline_view = TimelineView::CurrentKeyframes
            }
            ("timeline_view_audio", _) => self.timeline_view = TimelineView::Audio,
            ("timeline_view_media", _) => self.timeline_view = TimelineView::Media,
            ("timeline_view_text", _) => self.timeline_view = TimelineView::Text,
            ("timeline_view_shapes", _) => self.timeline_view = TimelineView::Shapes,
            ("timeline_view_3d", _) => self.timeline_view = TimelineView::ThreeD,
            ("timeline_view_cameras", _) => self.timeline_view = TimelineView::Cameras,
            ("timeline_view_lights", _) => self.timeline_view = TimelineView::Lights,
            ("undo_project", _) => self.undo_project(),
            ("redo_project", _) => self.redo_project(),
            ("timeline_view_markers", _) => self.timeline_view = TimelineView::Markers,
            ("timeline_view_graph", _) => self.timeline_view = TimelineView::Graph,
            ("timeline_view_flowchart", _) => self.timeline_view = TimelineView::Flowchart,
            ("properties_section_transform", _) => {
                self.properties_section = PropertiesSection::Transform
            }
            ("properties_section_content", _) => {
                self.properties_section = PropertiesSection::Content
            }
            ("properties_section_composite", _) => {
                self.properties_section = PropertiesSection::Composite
            }
            ("properties_section_advanced", _) => {
                self.properties_section = PropertiesSection::Advanced
            }
            ("toggle_timeline_snap", _) => {
                self.timeline_snap_enabled = !self.timeline_snap_enabled;
                self.timeline_snap_frame = None;
            }
            ("split_selected_layer_at_playhead", _) => self.split_selected_layer_at_playhead(),
            ("set_selected_layer_in_to_playhead", _) => self.set_selected_layer_in_to_playhead(),
            ("set_selected_layer_out_to_playhead", _) => self.set_selected_layer_out_to_playhead(),
            ("graph_channel_next", _) => {
                let (channels, scalar_effect, scalar_params, color_effect, color_params) = self.selected_layer_id.as_deref()
                    .and_then(|id| self.project_model.active_composition().layers.iter().find(|layer| layer.id == id))
                    .map(|layer| {
                        let selected = self.selected_effect_id.as_deref()
                            .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                            .or_else(|| layer.effects.first());
                        let scalar = selected.filter(|effect| !scalar_effect_parameters(&effect.effect_type).is_empty())
                            .or_else(|| layer.effects.iter().find(|effect| !scalar_effect_parameters(&effect.effect_type).is_empty()));
                        let colors = selected.filter(|effect| !effect_color_parameters(&effect.effect_type).is_empty())
                            .or_else(|| layer.effects.iter().find(|effect| !effect_color_parameters(&effect.effect_type).is_empty()));
                        let params = scalar.map(|effect| scalar_effect_parameters(&effect.effect_type)).unwrap_or_default();
                        let color_params = colors.map(|effect| effect_color_parameters(&effect.effect_type)).unwrap_or_default();
                        let channels = GraphChannel::ALL.iter().copied()
                            .filter(|channel| *channel != GraphChannel::EffectParameter
                                && !matches!(channel, GraphChannel::EffectColorRed | GraphChannel::EffectColorGreen | GraphChannel::EffectColorBlue | GraphChannel::EffectColorAlpha)
                                && channel.supports(layer))
                            .chain((!params.is_empty()).then_some(GraphChannel::EffectParameter))
                            .chain((!color_params.is_empty()).then_some(GraphChannel::EffectColorRed))
                            .chain((!color_params.is_empty()).then_some(GraphChannel::EffectColorGreen))
                            .chain((!color_params.is_empty()).then_some(GraphChannel::EffectColorBlue))
                            .chain((!color_params.is_empty()).then_some(GraphChannel::EffectColorAlpha))
                            .collect::<Vec<_>>();
                        (channels, scalar.map(|effect| effect.id.clone()), params, colors.map(|effect| effect.id.clone()), color_params)
                    })
                    .unwrap_or_else(|| (GraphChannel::ALL[..6].to_vec(), None, Vec::new(), None, Vec::new()));
                if channels.is_empty() { return Ok(()); }
                let index = channels
                    .iter()
                    .position(|channel| *channel == self.graph_channel)
                    .unwrap_or(0);
                self.graph_channel = channels[(index + 1) % channels.len()];
                if self.graph_channel == GraphChannel::EffectParameter {
                    self.selected_effect_id = scalar_effect;
                    self.graph_effect_parameter_id = scalar_params.first().map(|(id, _)| id.clone());
                } else if effect_color_component(self.graph_channel).is_some() {
                    self.selected_effect_id = color_effect;
                    self.graph_effect_parameter_id = color_params.first().map(|(id, _)| id.clone());
                }
                self.timeline_view = TimelineView::Graph;
            }
            ("graph_channel_select", _) => {
                let Some(channel) = selected_row_id.as_deref()
                    .and_then(|id| id.parse::<usize>().ok())
                    .and_then(|index| GraphChannel::ALL.get(index).copied()) else { return Ok(()) };
                let Some(layer) = self.selected_layer_id.as_deref()
                    .and_then(|id| self.project_model.active_composition().layers.iter().find(|layer| layer.id == id)) else { return Ok(()) };
                if !channel.supports(layer) { return Ok(()); }
                if channel == GraphChannel::EffectParameter || effect_color_component(channel).is_some() {
                    let wants_color = effect_color_component(channel).is_some();
                    let Some(effect) = self.selected_effect_id.as_deref()
                        .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                        .filter(|effect| if wants_color { !effect_color_parameters(&effect.effect_type).is_empty() } else { !scalar_effect_parameters(&effect.effect_type).is_empty() })
                        .or_else(|| layer.effects.iter().find(|effect| if wants_color { !effect_color_parameters(&effect.effect_type).is_empty() } else { !scalar_effect_parameters(&effect.effect_type).is_empty() })) else { return Ok(()) };
                    let params = if wants_color { effect_color_parameters(&effect.effect_type) } else { scalar_effect_parameters(&effect.effect_type) };
                    let Some((parameter_id, _)) = params.first() else { return Ok(()) };
                    self.selected_effect_id = Some(effect.id.clone());
                    self.graph_effect_parameter_id = Some(parameter_id.clone());
                }
                if matches!(channel, GraphChannel::MaskFeather | GraphChannel::MaskOpacity | GraphChannel::MaskExpansion)
                    && self.selected_mask_id.as_deref().is_none_or(|id| layer.masks.iter().all(|mask| mask.id != id))
                {
                    self.selected_mask_id = layer.masks.first().map(|mask| mask.id.clone());
                }
                self.graph_channel = channel;
                self.timeline_view = TimelineView::Graph;
            }
            ("graph_effect_parameter_select", _) => {
                let Some(parameter_id) = selected_row_id.as_deref() else { return Ok(()) };
                let Some(effect) = self.selected_layer_id.as_deref()
                    .and_then(|id| self.project_model.active_composition().layers.iter().find(|layer| layer.id == id))
                    .and_then(|layer| self.selected_effect_id.as_deref().and_then(|effect_id| layer.effects.iter().find(|effect| effect.id == effect_id))) else { return Ok(()) };
                let params = if effect_color_component(self.graph_channel).is_some() {
                    effect_color_parameters(&effect.effect_type)
                } else if self.graph_channel == GraphChannel::EffectParameter {
                    scalar_effect_parameters(&effect.effect_type)
                } else { Vec::new() };
                if params.iter().any(|(id, _)| id == parameter_id) {
                    self.graph_effect_parameter_id = Some(parameter_id.to_owned());
                    self.timeline_view = TimelineView::Graph;
                }
            }
            ("graph_effect_parameter_next", _) => {
                if let Some(params) = self.selected_layer_id.as_deref()
                    .and_then(|id| self.project_model.active_composition().layers.iter().find(|layer| layer.id == id))
                    .and_then(|layer| self.selected_effect_id.as_deref().and_then(|effect_id| layer.effects.iter().find(|effect| effect.id == effect_id)))
                    .map(|effect| if effect_color_component(self.graph_channel).is_some() { effect_color_parameters(&effect.effect_type) } else { scalar_effect_parameters(&effect.effect_type) })
                {
                    if !params.is_empty() {
                        let index = params.iter().position(|(id, _)| Some(id.as_str()) == self.graph_effect_parameter_id.as_deref()).unwrap_or(0);
                        self.graph_effect_parameter_id = Some(params[(index + 1) % params.len()].0.clone());
                        self.timeline_view = TimelineView::Graph;
                    }
                }
            }
            ("graph_add_keyframe", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some((layer, camera, light)) = graph_targets_mut(
                        self.project_model.active_composition_mut(), layer_id,
                    )
                    {
                        add_graph_keyframe(layer, camera, light, self.selected_mask_id.as_deref(), self.selected_effect_id.as_deref(), self.graph_effect_parameter_id.as_deref(), self.graph_channel, frame);
                        self.mark_project_changed();
                    }
                    self.timeline_view = TimelineView::Graph;
                }
            }
            ("graph_remove_keyframe", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some((layer, camera, light)) = graph_targets_mut(
                        self.project_model.active_composition_mut(), layer_id,
                    )
                    {
                        if remove_graph_keyframe(layer, camera, light, self.selected_mask_id.as_deref(), self.selected_effect_id.as_deref(), self.graph_effect_parameter_id.as_deref(), self.graph_channel, frame) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                }
            }
            ("graph_cycle_interpolation", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    if let Some((layer, camera, light)) = graph_targets_mut(
                        self.project_model.active_composition_mut(), layer_id,
                    ) {
                        if cycle_graph_interpolation(layer, camera, light, self.selected_mask_id.as_deref(), self.selected_effect_id.as_deref(), self.graph_effect_parameter_id.as_deref(), self.graph_channel, self.current_frame) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                }
            }
            ("graph_set_easy_ease", _)
            | ("graph_set_ease_in", _)
            | ("graph_set_ease_out", _)
            | ("graph_set_overshoot", _)
            | ("graph_set_bounce", _)
            | ("graph_set_sine", _)
            | ("graph_set_linear", _)
            | ("graph_set_hold", _) => {
                let preset = match action.name.as_str() {
                    "graph_set_easy_ease" => "easy_ease",
                    "graph_set_ease_in" => "ease_in",
                    "graph_set_ease_out" => "ease_out",
                    "graph_set_overshoot" => "overshoot",
                    "graph_set_bounce" => "bounce",
                    "graph_set_sine" => "sine",
                    "graph_set_linear" => "linear",
                    "graph_set_hold" => "hold",
                    _ => "easy_ease",
                };
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    if let Some((layer, camera, light)) = graph_targets_mut(
                        self.project_model.active_composition_mut(), layer_id,
                    ) {
                        if set_graph_interpolation_preset(layer, camera, light, self.selected_mask_id.as_deref(), self.selected_effect_id.as_deref(), self.graph_effect_parameter_id.as_deref(), self.graph_channel, self.current_frame, preset) {
                            self.mark_project_changed();
                        }
                    }
                    self.timeline_view = TimelineView::Graph;
                }
            }
            ("timeline_zoom_in", _) => self.set_timeline_zoom(2.0),
            ("timeline_zoom_out", _) => self.set_timeline_zoom(0.5),
            ("timeline_zoom_fit", _) => self.fit_timeline(),
            ("timeline_pan_left", _) => self.pan_timeline(-1.0),
            ("timeline_pan_right", _) => self.pan_timeline(1.0),
            ("add_timeline_marker", _) => {
                let composition = self.project_model.active_composition_mut();
                if let Some(marker) = composition
                    .markers
                    .iter()
                    .find(|marker| marker.frame == self.current_frame)
                {
                    self.selected_marker_frame = Some(marker.frame);
                } else {
                    let label = format!("マーカー {}", composition.markers.len() + 1);
                    composition.markers.push(
                        kagari_vfx::core::timeline::TimelineMarker {
                            frame: self.current_frame,
                            label,
                            color: [1.0, 0.38, 0.08],
                        },
                    );
                    self.selected_marker_frame = Some(self.current_frame);
                    self.mark_project_changed();
                }
            }
            ("go_timeline_marker", _) => {
                if let Some(frame) = selected_row_id.and_then(|id| id.parse::<u32>().ok()) {
                    if self
                        .project_model
                        .active_composition()
                        .markers
                        .iter()
                        .any(|marker| marker.frame == frame)
                    {
                        self.selected_marker_frame = Some(frame);
                        self.current_frame = frame.min(
                            self.project_model
                                .active_composition()
                                .duration_frames
                                .saturating_sub(1),
                        );
                        self.current_time = format_timecode(self.current_frame, self.project_fps);
                    }
                }
            }
            ("remove_timeline_marker", _) => {
                if let Some(frame) = self.selected_marker_frame {
                    let composition = self.project_model.active_composition_mut();
                    let old_len = composition.markers.len();
                    composition.markers.retain(|marker| marker.frame != frame);
                    self.selected_marker_frame = None;
                    if composition.markers.len() != old_len {
                        self.mark_project_changed();
                    }
                }
            }
            ("timeline_marker_label_changed", Some(ActionValue::Text(value))) => {
                if let Some(frame) = self.selected_marker_frame {
                    let label = value.trim().chars().take(128).collect::<String>();
                    if !label.is_empty() {
                        if let Some(marker) = self
                            .project_model
                            .active_composition_mut()
                            .markers
                            .iter_mut()
                            .find(|marker| marker.frame == frame)
                        {
                            if marker.label != label {
                                marker.label = label;
                                self.mark_project_changed();
                            }
                        }
                    }
                }
            }
            ("timeline_marker_frame_changed", Some(ActionValue::Text(value))) => {
                if let (Some(old_frame), Ok(requested_frame)) = (self.selected_marker_frame, value.trim().parse::<u32>()) {
                    let duration = self.project_model.active_composition().duration_frames.max(1);
                    let new_frame = requested_frame.min(duration - 1);
                    let composition = self.project_model.active_composition_mut();
                    if !composition.markers.iter().any(|marker| marker.frame == new_frame && marker.frame != old_frame) {
                        if let Some(marker) = composition.markers.iter_mut().find(|marker| marker.frame == old_frame) {
                            if marker.frame != new_frame {
                                marker.frame = new_frame;
                                self.selected_marker_frame = Some(new_frame);
                                self.mark_project_changed();
                            }
                        }
                    }
                }
            }
            ("add_layer_marker", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some(layer) = self.project_model.active_composition_mut().layers.iter_mut().find(|layer| layer.id == layer_id && !layer.locked) {
                        if layer.markers.iter().any(|marker| marker.frame == frame) {
                            self.selected_layer_marker_frame = Some(frame);
                        } else {
                            let label = format!("マーカー {}", layer.markers.len() + 1);
                            layer.markers.push(kagari_vfx::core::timeline::TimelineMarker {
                                frame,
                                label,
                                color: [0.22, 0.72, 0.68],
                            });
                            self.selected_layer_marker_frame = Some(frame);
                            self.mark_project_changed();
                        }
                    }
                }
            }
            ("go_layer_marker", ref val) => {
                let frame_opt = match val {
                    Some(ActionValue::Text(s)) => s.parse::<u32>().ok(),
                    _ => selected_row_id.and_then(|id| id.parse::<u32>().ok()),
                };
                if let (Some(layer_id), Some(frame)) = (self.selected_layer_id.as_deref(), frame_opt) {
                    if self.project_model.active_composition().layers.iter()
                        .find(|layer| layer.id == layer_id)
                        .is_some_and(|layer| layer.markers.iter().any(|marker| marker.frame == frame))
                    {
                        self.selected_layer_marker_frame = Some(frame);
                        self.current_frame = frame.min(self.project_model.active_composition().duration_frames.saturating_sub(1));
                        self.current_time = format_timecode(self.current_frame, self.project_fps);
                    }
                }
            }
            ("remove_layer_marker", _) => {
                if let (Some(layer_id), Some(frame)) = (self.selected_layer_id.as_deref(), self.selected_layer_marker_frame) {
                    if let Some(layer) = self.project_model.active_composition_mut().layers.iter_mut().find(|layer| layer.id == layer_id && !layer.locked) {
                        let old_len = layer.markers.len();
                        layer.markers.retain(|marker| marker.frame != frame);
                        if layer.markers.len() != old_len {
                            self.selected_layer_marker_frame = None;
                            self.mark_project_changed();
                        }
                    }
                }
            }
            ("layer_marker_label_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(frame)) = (self.selected_layer_id.as_deref(), self.selected_layer_marker_frame) {
                    let label = value.trim().chars().take(128).collect::<String>();
                    if !label.is_empty() {
                        if let Some(marker) = self.project_model.active_composition_mut().layers.iter_mut()
                            .find(|layer| layer.id == layer_id && !layer.locked)
                            .and_then(|layer| layer.markers.iter_mut().find(|marker| marker.frame == frame))
                        {
                            if marker.label != label {
                                marker.label = label;
                                self.mark_project_changed();
                            }
                        }
                    }
                }
            }
            ("layer_marker_frame_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(old_frame), Ok(requested_frame)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_layer_marker_frame,
                    value.trim().parse::<u32>(),
                ) {
                    let duration = self.project_model.active_composition().duration_frames.max(1);
                    let new_frame = requested_frame.min(duration - 1);
                    if let Some(layer) = self.project_model.active_composition_mut().layers.iter_mut().find(|layer| layer.id == layer_id && !layer.locked) {
                        if !layer.markers.iter().any(|marker| marker.frame == new_frame && marker.frame != old_frame) {
                            if let Some(marker) = layer.markers.iter_mut().find(|marker| marker.frame == old_frame) {
                                if marker.frame != new_frame {
                                    marker.frame = new_frame;
                                    self.selected_layer_marker_frame = Some(new_frame);
                                    self.mark_project_changed();
                                }
                            }
                        }
                    }
                }
            }
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
                }
            }
            ("output_changed", Some(ActionValue::Text(value))) => {
                self.output_path = value;
                self.still_overwrite_pending = false;
                self.still_status.clear();
            }
            ("video_output_changed", Some(ActionValue::Text(value))) => {
                self.video_output_path = value;
                self.video_overwrite_pending = false;
            }
            ("choose_output_file", _) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG画像", &["png"])
                    .set_file_name("Particle_Study.png")
                    .save_file()
                {
                    self.output_path = path.display().to_string();
                    self.still_overwrite_pending = false;
                    self.still_status.clear();
                }
            }
            ("choose_video_output", _) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("動画", &["mp4", "mov", "m4v"])
                    .set_file_name("Kagari_export.mp4")
                    .save_file()
                {
                    self.video_output_path = path.display().to_string();
                    self.video_overwrite_pending = false;
                    self.render_status = "".to_owned();
                }
            }
            ("render_still", _) => self.export_current_frame_png(false),
            ("render_still_overwrite", _) => self.export_current_frame_png(true),
            ("render_still_cancel_overwrite", _) => {
                self.still_overwrite_pending = false;
                self.still_status = "PNGの上書きを取り消しました。".to_owned();
            }
            ("render_start", _) => self.start_video_export(false),
            ("cycle_video_export_codec", _) => self.cycle_video_export_codec(),
            ("render_overwrite", _) => self.start_video_export(true),
            ("render_cancel_overwrite", _) => {
                self.video_overwrite_pending = false;
                self.render_status = "上書きを取り消しました。".to_owned();
            }
            ("render_stop", _) => self.cancel_video_export(),
            ("save_project", _) => self.save_current_project(),
            ("search_changed", Some(ActionValue::Text(value))) => self.search = value,
            ("clear_scene_search", _) => self.search.clear(),
            ("project_search_changed", Some(ActionValue::Text(value))) => {
                self.project_search = value
            }
            ("time_changed", Some(ActionValue::Text(value))) => {
                self.stop_playback();
                if let Some(frame) = parse_timecode(&value, self.project_fps) {
                    self.current_frame = frame
                        .saturating_sub(
                            self.project_model.active_composition().start_timecode_frames,
                        )
                        .min(
                        self.project_model
                            .active_composition()
                            .duration_frames
                            .saturating_sub(1),
                        );
                }
                self.current_time = format_timecode(self.current_frame, self.project_fps);
            }
            ("zoom_changed", Some(ActionValue::Text(value))) => {
                let trimmed = value.trim().trim_end_matches('%').trim();
                if let Ok(pct) = trimmed.parse::<f64>() {
                    self.zoom = format!("{}%", pct.clamp(10.0, 800.0).round() as u32);
                } else {
                    self.zoom = "100%".to_owned();
                }
                self.zoom_manually_adjusted = true;
            }
            ("position_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let width = f64::from(self.project_model.active_composition().width);
                let min = (-width).min(self.position_x - width);
                let max = (width * 2.0).max(self.position_x + width);
                self.position_x = value.clamp(min, max);
                self.apply_selected_transform_controls();
            }
            ("position_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let height = f64::from(self.project_model.active_composition().height);
                let min = (-height).min(self.position_y - height);
                let max = (height * 2.0).max(self.position_y + height);
                self.position_y = value.clamp(min, max);
                self.apply_selected_transform_controls();
            }
            ("anchor_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let width = f64::from(self.project_model.active_composition().width);
                let min = (-width).min(self.anchor_x - width);
                let max = (width * 2.0).max(self.anchor_x + width);
                self.anchor_x = value.clamp(min, max);
                self.apply_selected_transform_controls();
            }
            ("anchor_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                let height = f64::from(self.project_model.active_composition().height);
                let min = (-height).min(self.anchor_y - height);
                let max = (height * 2.0).max(self.anchor_y + height);
                self.anchor_y = value.clamp(min, max);
                self.apply_selected_transform_controls();
            }
            ("scale_x_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.scale_x = value.clamp(0.1, 1000.0);
                self.apply_selected_transform_controls();
            }
            ("scale_y_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.scale_y = value.clamp(0.1, 1000.0);
                self.apply_selected_transform_controls();
            }
            ("rotation_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.rotation_degrees = value.clamp(-36_000.0, 36_000.0);
                self.apply_selected_transform_controls();
            }
            ("opacity_changed", Some(ActionValue::Number(value))) if value.is_finite() => {
                self.opacity = value.clamp(0.0, 100.0);
                self.apply_selected_transform_controls();
            }
            ("position_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_x", value)
            }
            ("position_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_y", value)
            }
            ("position_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("position_z", value)
            }
            ("rotation_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_x", value)
            }
            ("rotation_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_y", value)
            }
            ("rotation_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("rotation_z", value)
            }
            ("scale_3d_x_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_x", value)
            }
            ("scale_3d_y_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_y", value)
            }
            ("scale_3d_z_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_transform("scale_z", value)
            }
            ("material_ambient_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("ambient", value)
            }
            ("material_diffuse_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("diffuse", value)
            }
            ("material_specular_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("specular", value)
            }
            ("material_specular_exponent_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("specular_exponent", value)
            }
            ("material_emission_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("emission", value)
            }
            ("material_metalness_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("metalness", value)
            }
            ("material_light_transmission_changed", Some(ActionValue::Number(value))) => {
                self.update_selected_3d_material("light_transmission", value)
            }
            ("toggle_material_cast_shadows", _) => {
                self.toggle_selected_3d_material_flag("cast_shadows")
            }
            ("toggle_material_accepts_shadows", _) => {
                self.toggle_selected_3d_material_flag("accepts_shadows")
            }
            ("toggle_material_accepts_lights", _) => {
                self.toggle_selected_3d_material_flag("accepts_lights")
            }
            ("text_content_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("content", &value)
            }
            ("text_font_family_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("font_family", &value)
            }
            ("text_font_family_select", _) => {
                if let Some(family) = selected_row_id.as_deref() {
                    if self.text_font_families.iter().any(|installed| installed == family) {
                        self.update_selected_text_property("font_family", family);
                    }
                }
            }
            ("text_font_search_changed", Some(ActionValue::Text(value))) => {
                self.text_font_search = value.chars().take(100).collect();
            }
            ("text_font_size_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("font_size", &value)
            }
            ("text_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("color", &value)
            }
            ("text_tracking_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("tracking", &value)
            }
            ("text_leading_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("leading", &value)
            }
            ("text_align_left", _) => self.update_selected_text_property("align", "0"),
            ("text_align_center", _) => self.update_selected_text_property("align", "1"),
            ("text_align_right", _) => self.update_selected_text_property("align", "2"),
            ("text_stroke_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("stroke_width", &value)
            }
            ("text_stroke_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_property("stroke_color", &value)
            }
            ("toggle_text_faux_bold", _) => self.update_selected_text_formatting(|formatting| {
                formatting.faux_bold = !formatting.faux_bold;
                true
            }),
            ("toggle_text_faux_italic", _) => self.update_selected_text_formatting(|formatting| {
                formatting.faux_italic = !formatting.faux_italic;
                true
            }),
            ("toggle_text_all_caps", _) => self.update_selected_text_formatting(|formatting| {
                formatting.all_caps = !formatting.all_caps;
                if formatting.all_caps {
                    formatting.small_caps = false;
                }
                true
            }),
            ("toggle_text_small_caps", _) => self.update_selected_text_formatting(|formatting| {
                formatting.small_caps = !formatting.small_caps;
                if formatting.small_caps {
                    formatting.all_caps = false;
                }
                true
            }),
            ("text_box_width_changed", Some(ActionValue::Text(value))) => {
                if let Some(next) = value.trim().parse::<f32>().ok().filter(|value| value.is_finite()) {
                    self.update_selected_text_formatting(|formatting| {
                        let next = next.clamp(0.0, 10000.0);
                        if formatting.box_width == next { false } else { formatting.box_width = next; true }
                    });
                }
            }
            ("text_box_height_changed", Some(ActionValue::Text(value))) => {
                if let Some(next) = value.trim().parse::<f32>().ok().filter(|value| value.is_finite()) {
                    self.update_selected_text_formatting(|formatting| {
                        let next = next.clamp(0.0, 10000.0);
                        if formatting.box_height == next { false } else { formatting.box_height = next; true }
                    });
                }
            }
            ("toggle_text_animator", _) => self.toggle_selected_text_animator(),
            ("cycle_text_animator_shape", _) => self.cycle_selected_text_animator_shape(),
            ("toggle_text_animator_random_order", _) => {
                self.toggle_selected_text_animator_random_order()
            }
            ("text_animator_expression_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_text_animator_expression(&value)
            }
            ("text_animator_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter) = selected_row_id.as_deref() {
                    self.update_selected_text_animator_parameter(parameter, &value);
                }
            }
            ("shape_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("width", &value)
            }
            ("shape_width_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeWidth),
            ("shape_height_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("height", &value)
            }
            ("shape_height_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeHeight),
            ("shape_corner_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("corner_radius", &value)
            }
            ("shape_corner_radius_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeCornerRadius),
            ("shape_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("color", &value)
            }
            ("cycle_shape_fill_type", _) => self.cycle_selected_shape_fill_type(),
            ("shape_gradient_start_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_color(false, &value)
            }
            ("shape_gradient_end_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_color(true, &value)
            }
            ("shape_gradient_angle_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_geometry("angle", &value)
            }
            ("shape_gradient_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_gradient_geometry("radius", &value)
            }
            ("toggle_shape_trim_paths", _) => self.toggle_selected_shape_trim_paths(),
            ("shape_trim_start_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_trim_path("start", &value),
            ("shape_trim_end_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_trim_path("end", &value),
            ("shape_trim_offset_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_trim_path("offset", &value),
            ("shape_trim_start_keyframe", _) => self.add_selected_shape_trim_keyframe("start"),
            ("shape_trim_end_keyframe", _) => self.add_selected_shape_trim_keyframe("end"),
            ("shape_trim_offset_keyframe", _) => self.add_selected_shape_trim_keyframe("offset"),
            ("toggle_shape_repeater", _) => self.toggle_selected_shape_repeater(),
            ("toggle_shape_repeater_composite", _) => self.toggle_selected_shape_repeater_composite(),
            ("shape_repeater_copies_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("copies", &value),
            ("shape_repeater_x_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("position_x", &value),
            ("shape_repeater_y_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("position_y", &value),
            ("shape_repeater_scale_x_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("scale_x", &value),
            ("shape_repeater_scale_y_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("scale_y", &value),
            ("shape_repeater_rotation_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("rotation", &value),
            ("shape_repeater_start_opacity_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("start_opacity", &value),
            ("shape_repeater_end_opacity_changed", Some(ActionValue::Text(value))) => self.update_selected_shape_repeater("end_opacity", &value),
            ("shape_repeater_copies_keyframe", _) => self.add_selected_shape_repeater_keyframe("copies"),
            ("shape_repeater_x_keyframe", _) => self.add_selected_shape_repeater_keyframe("position_x"),
            ("shape_repeater_y_keyframe", _) => self.add_selected_shape_repeater_keyframe("position_y"),
            ("shape_repeater_scale_x_keyframe", _) => self.add_selected_shape_repeater_keyframe("scale_x"),
            ("shape_repeater_scale_y_keyframe", _) => self.add_selected_shape_repeater_keyframe("scale_y"),
            ("shape_repeater_rotation_keyframe", _) => self.add_selected_shape_repeater_keyframe("rotation"),
            ("shape_repeater_start_opacity_keyframe", _) => self.add_selected_shape_repeater_keyframe("start_opacity"),
            ("shape_repeater_end_opacity_keyframe", _) => self.add_selected_shape_repeater_keyframe("end_opacity"),
            ("shape_stroke_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("stroke_color", &value)
            }
            ("shape_stroke_width_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("stroke_width", &value)
            }
            ("shape_extrusion_depth_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("extrusion_depth", &value)
            }
            ("shape_bevel_depth_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("bevel_depth", &value)
            }
            ("shape_points_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("points", &value)
            }
            ("shape_points_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapePoints),
            ("shape_inner_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("inner_radius", &value)
            }
            ("shape_inner_radius_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeInnerRadius),
            ("shape_outer_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("outer_radius", &value)
            }
            ("shape_outer_radius_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeOuterRadius),
            ("shape_sides_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("sides", &value)
            }
            ("shape_sides_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeSides),
            ("shape_radius_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_shape_property("radius", &value)
            }
            ("shape_radius_keyframe", _) => self.add_selected_shape_keyframe(GraphChannel::ShapeRadius),
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
            ("effect_search_changed", Some(ActionValue::Text(value))) => {
                self.effect_search = value
            }
            ("add_effect_catalog", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() { self.add_effect(effect_id); }
            }
            ("add_gaussian_blur", _) => self.add_effect("blur"),
            ("add_color_tint", _) => self.add_effect("tint"),
            ("add_drop_shadow", _) => self.add_effect("shadow"),
            ("add_glow", _) => self.add_effect("glow"),
            ("add_vignette", _) => self.add_effect("vignette"),
            ("add_hue_saturation", _) => self.add_effect("hue_saturation"),
            ("add_levels", _) => self.add_effect("levels"),
            ("add_motion_blur", _) => self.add_effect("motion_blur"),
            ("add_film_grain", _) => self.add_effect("film_grain"),
            ("add_posterize", _) => self.add_effect("posterize"),
            ("add_directional_blur", _) => self.add_effect("directional_blur"),
            ("add_radial_blur", _) => self.add_effect("radial_blur"),
            ("add_sharpen", _) => self.add_effect("sharpen"),
            ("select_effect", _) => self.selected_effect_id = selected_row_id,
            ("toggle_effect", _) => {
                if let Some(effect_id) = selected_row_id {
                    let layer_id = self.selected_layer_id.clone();
                    let changed = layer_id
                        .as_deref()
                        .and_then(|layer_id| {
                            self.project_model
                                .active_composition_mut()
                                .layers
                                .iter_mut()
                                .find(|layer| layer.id == layer_id)
                        })
                        .and_then(|layer| {
                            layer
                                .effects
                                .iter_mut()
                                .find(|effect| effect.id == effect_id)
                        })
                        .map(|effect| effect.enabled = !effect.enabled)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                }
            }
            ("move_effect_up", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() { self.move_effect(effect_id, -1); }
            }
            ("move_effect_down", _) => {
                if let Some(effect_id) = selected_row_id.as_deref() { self.move_effect(effect_id, 1); }
            }
            ("remove_effect", _) => {
                if let (Some(layer_id), Some(effect_id)) =
                    (self.selected_layer_id.clone(), selected_row_id)
                {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let old_len = layer.effects.len();
                        layer.effects.retain(|effect| effect.id != effect_id);
                        if layer.effects.len() != old_len {
                            if self.selected_effect_id.as_deref() == Some(effect_id.as_str()) {
                                self.selected_effect_id = None;
                            }
                            self.mark_project_changed();
                        }
                    }
                }
            }
            ("effect_parameter_changed", Some(ActionValue::Text(value))) => {
                if let Some(parameter_id) = selected_row_id {
                    self.update_selected_effect_parameter(&parameter_id, &value)
                }
            }
            ("effect_parameter_keyframe", _) => {
                if let Some(parameter_id) = selected_row_id.as_deref() {
                    self.keyframe_selected_effect_parameter(parameter_id);
                }
            }
            ("effect_color_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_effect_color(&value)
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
            ("add_rect_mask", _) => self.add_mask_shape(false),
            ("add_ellipse_mask", _) => self.add_mask_shape(true),
            ("select_mask", _) => {
                if let Some(id) = selected_row_id {
                    self.selected_mask_id = Some(id);
                }
            }
            ("toggle_mask_enabled", _) => {
                if let (Some(layer_id), Some(mask_id)) = (self.selected_layer_id.as_deref(), selected_row_id.as_deref()) {
                    if let Some(mask) = self.project_model.active_composition_mut().layers.iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        mask.enabled = !mask.enabled;
                        self.mark_project_changed();
                    }
                }
            }
            ("cycle_mask_mode", _) => {
                if let (Some(layer_id), Some(mask_id)) = (self.selected_layer_id.as_deref(), self.selected_mask_id.as_deref()) {
                    if let Some(mask) = self.project_model.active_composition_mut().layers.iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        use kagari_vfx::core::mask::MaskMode;
                        const MODES: [MaskMode; 7] = [MaskMode::Add, MaskMode::Subtract, MaskMode::Intersect, MaskMode::Lighten, MaskMode::Darken, MaskMode::Difference, MaskMode::None];
                        let index = MODES.iter().position(|mode| *mode == mask.mode).unwrap_or(0);
                        mask.mode = MODES[(index + 1) % MODES.len()];
                        self.mark_project_changed();
                    }
                }
            }
            ("toggle_mask_inverted", _) => {
                if let (Some(layer_id), Some(mask_id)) = (self.selected_layer_id.as_deref(), self.selected_mask_id.as_deref()) {
                    if let Some(mask) = self.project_model.active_composition_mut().layers.iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        mask.inverted = !mask.inverted;
                        self.mark_project_changed();
                    }
                }
            }
            ("remove_mask", _) => {
                if let (Some(layer_id), Some(mask_id)) = (self.selected_layer_id.as_deref(), selected_row_id.as_deref()) {
                    if let Some(layer) = self.project_model.active_composition_mut().layers.iter_mut().find(|layer| layer.id == layer_id && !layer.locked) {
                        let before = layer.masks.len();
                        layer.masks.retain(|mask| mask.id != mask_id);
                        if layer.masks.len() != before {
                            self.selected_mask_id = layer.masks.first().map(|mask| mask.id.clone());
                            self.mark_project_changed();
                        }
                    }
                }
            }
            ("mask_name_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(mask_id)) = (self.selected_layer_id.as_deref(), self.selected_mask_id.as_deref()) {
                    if let Some(mask) = self.project_model.active_composition_mut().layers.iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter_mut().find(|mask| mask.id == mask_id))
                    {
                        if mask.name != value {
                            mask.name = value;
                            self.mark_project_changed();
                        }
                    }
                }
            }
            ("mask_feather_changed", Some(ActionValue::Text(value))) => self.update_selected_mask_value(&value, MaskValueChannel::Feather),
            ("mask_opacity_changed", Some(ActionValue::Text(value))) => self.update_selected_mask_value(&value, MaskValueChannel::Opacity),
            ("mask_expansion_changed", Some(ActionValue::Text(value))) => self.update_selected_mask_value(&value, MaskValueChannel::Expansion),
            ("mask_path_keyframe", _) => self.add_selected_mask_keyframe("path"),
            ("mask_feather_keyframe", _) => self.add_selected_mask_keyframe("feather"),
            ("mask_opacity_keyframe", _) => self.add_selected_mask_keyframe("opacity"),
            ("mask_expansion_keyframe", _) => self.add_selected_mask_keyframe("expansion"),
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
            ("toggle_shy_visibility", _) => self.hide_shy_layers = !self.hide_shy_layers,
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
                }
            }
            ("toggle_audio_mute", ref val) => {
                let id_opt = match val {
                    Some(ActionValue::Text(s)) => Some(s.clone()),
                    _ => selected_row_id,
                };
                if let Some(id) = id_opt {
                    let changed = self.project_model.active_composition_mut().layers.iter_mut()
                        .find(|layer| layer.id == id)
                        .map(|layer| layer.audio_muted = !layer.audio_muted)
                        .is_some();
                    if changed {
                        self.mark_project_changed();
                    }
                }
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
                }
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
                }
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
                        self.asset_status = format!(
                            "「{name}」はロック中のためモーションブラーを変更できません。"
                        );
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
                }
            }
            ("toggle_scene_group", _) => {
                if let Some(id) = selected_row_id {
                    self.toggle_scene_group(&id);
                }
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
            }
            ("expand_all_scene_groups", _) => self.collapsed_scene_layers.clear(),
            ("show_all_scene_layers", _) => self.set_all_scene_layer_visibility(true),
            ("hide_all_scene_layers", _) => self.set_all_scene_layer_visibility(false),
            ("toggle_alignment_layer_selection", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.toggle_alignment_layer_selection(id);
                }
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
            }
            ("clear_alignment_selection", _) => {
                self.align_selected_layer_ids.clear();
            }
            ("toggle_align_to_composition", _) => {
                self.align_to_composition = !self.align_to_composition;
            }
            ("align_scene_left", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::Left,
            ),
            ("align_scene_horizontal_center", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::HorizontalCenter,
            ),
            ("align_scene_right", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::Right,
            ),
            ("align_scene_top", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::Top,
            ),
            ("align_scene_vertical_center", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::VerticalCenter,
            ),
            ("align_scene_bottom", _) => self.align_scene_layers(
                kagari_vfx::core::layer_align::AlignMode::Bottom,
            ),
            ("distribute_scene_horizontal", _) => self.distribute_scene_layers(
                kagari_vfx::core::layer_align::DistributeMode::HorizontalCenters,
            ),
            ("distribute_scene_vertical", _) => self.distribute_scene_layers(
                kagari_vfx::core::layer_align::DistributeMode::VerticalCenters,
            ),
            ("parent_selected_layer", _) => self.parent_selected_layer(),
            ("unparent_selected_layer", _) => self.unparent_selected_layer(),
            ("set_layer_parent", _) => {
                if let Some(parent_id) = selected_row_id.as_deref() {
                    self.set_selected_layer_parent(Some(parent_id));
                }
            }
            ("add_keyframe", _) => self.add_transform_keyframes_at_current_frame(),
            ("keyframe_position_2d", _) => self.add_transform_property_keyframe("position_2d"),
            ("keyframe_anchor_point", _) => self.add_transform_property_keyframe("anchor_point"),
            ("keyframe_rotation_2d", _) => self.add_transform_property_keyframe("rotation_2d"),
            ("keyframe_scale_2d", _) => self.add_transform_property_keyframe("scale_2d"),
            ("keyframe_opacity", _) => self.add_transform_property_keyframe("opacity"),
            ("keyframe_position_3d", _) => self.add_transform_property_keyframe("position_3d"),
            ("keyframe_rotation_3d", _) => self.add_transform_property_keyframe("rotation_3d"),
            ("keyframe_scale_3d", _) => self.add_transform_property_keyframe("scale_3d"),
            ("previous_keyframe", _) => self.jump_to_selected_keyframe(false),
            ("next_keyframe", _) => self.jump_to_selected_keyframe(true),
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
            ("place_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.place_project_asset(id, false);
                }
            }
            ("place_project_asset_fit", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    self.place_project_asset(&id, false);
                }
            }
            ("place_project_asset_fill", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    self.place_project_asset(&id, true);
                }
            }
            ("open_project_composition", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.open_project_composition_asset(id);
                }
            }
            ("enter_project_asset_folder", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    let is_child_folder = self.project_model.assets.iter().find(|item| item.id == id)
                        .is_some_and(|item| matches!(&item.item_type, kagari_vfx::core::timeline::ProjectItemType::Folder { .. })
                            && item.parent_folder.as_deref() == self.asset_folder_id.as_deref());
                    if is_child_folder {
                        self.asset_folder_id = Some(id.to_owned());
                        self.project_asset_search.clear();
                    }
                }
            }
            ("leave_project_asset_folder", _) => {
                self.asset_folder_id = self.asset_folder_id.as_deref()
                    .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
                    .and_then(|item| item.parent_folder.clone());
                self.project_asset_search.clear();
            }
            ("project_asset_search_changed", Some(ActionValue::Text(value))) => {
                self.project_asset_search = value.chars().take(256).collect();
            }
            ("clear_project_asset_search", _) => self.project_asset_search.clear(),
            ("project_asset_search_all_folders", _) => {
                self.project_asset_search_all_folders = !self.project_asset_search_all_folders;
            }
            ("project_assets_sort_name", _) => self.project_assets_sort_by_type = false,
            ("project_assets_sort_type", _) => self.project_assets_sort_by_type = true,
            ("new_asset_folder_name_changed", Some(ActionValue::Text(value))) => {
                self.new_asset_folder_name = value.chars().take(80).collect();
            }
            ("select_project_asset", _) => {
                self.selected_project_asset_id = selected_row_id.clone().filter(|id| {
                    self.project_model.assets.iter().any(|item| {
                        item.id == *id && !matches!(
                            &item.item_type,
                            kagari_vfx::core::timeline::ProjectItemType::Folder { .. }
                        )
                    })
                });
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
            }
            ("fit_selected_image_to_composition", _) => {
                self.scale_selected_media_to_composition(false);
            }
            ("fill_selected_image_to_composition", _) => {
                self.scale_selected_media_to_composition(true);
            }
            ("selected_project_asset_name_changed", Some(ActionValue::Text(value))) => {
                let name = value.trim().chars().take(128).collect::<String>();
                if !name.is_empty() {
                    if let Some(id) = self.selected_project_asset_id.as_deref() {
                        if let Some(item) = self.project_model.assets.iter_mut().find(|item| item.id == id) {
                            if item.name != name {
                                item.name = name;
                                self.mark_project_changed();
                            }
                        }
                    }
                }
            }
            ("rename_project_asset", Some(ActionValue::Text(value))) => {
                let name = value.trim().chars().take(128).collect::<String>();
                if !name.is_empty() {
                    if let Some(id) = selected_row_id.as_deref() {
                        if let Some(item) = self.project_model.assets.iter_mut().find(|item| item.id == id) {
                            if matches!(&item.item_type, kagari_vfx::core::timeline::ProjectItemType::Folder { .. })
                                && item.name != name
                            {
                                item.name = name.clone();
                                if let kagari_vfx::core::timeline::ProjectItemType::Folder { name: folder_name } = &mut item.item_type {
                                    *folder_name = name;
                                }
                                self.mark_project_changed();
                            }
                        }
                    }
                }
            }
            ("move_selected_project_asset_here", _) => {
                if let Some(id) = self.selected_project_asset_id.clone() {
                    let destination = self.asset_folder_id.clone();
                    if project_asset_can_move_to_folder(
                        &self.project_model.assets,
                        &id,
                        destination.as_deref(),
                    ) {
                        if let Some(item) = self.project_model.assets.iter_mut().find(|item| item.id == id) {
                            item.parent_folder = destination;
                            self.asset_status = format!("{} を現在のフォルダーへ移動しました。", item.name);
                        }
                        self.mark_project_changed();
                    } else {
                        self.asset_status = "このフォルダーには移動できません。移動先を確認してください。".to_owned();
                    }
                }
            }
            ("remove_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    let removable = self.project_model.assets.iter().find(|item| item.id == id).is_some_and(|item| {
                        use kagari_vfx::core::timeline::ProjectItemType;
                        match &item.item_type {
                            ProjectItemType::Composition { .. } => false,
                            ProjectItemType::Folder { .. } => !self.project_model.assets.iter().any(|child| child.parent_folder.as_deref() == Some(item.id.as_str())),
                            _ => true,
                        }
                    });
                    if removable {
                        let removed_name = self.project_model.assets.iter().find(|item| item.id == id).map(|item| item.name.clone()).unwrap_or_default();
                        self.project_model.assets.retain(|item| item.id != id);
                        if self.selected_project_asset_id.as_deref() == Some(id) {
                            self.selected_project_asset_id = None;
                        }
                        self.asset_status = format!("素材をプロジェクトから登録解除しました（元ファイルは保持）: {removed_name}");
                        self.mark_project_changed();
                    }
                }
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
                }
            }
            ("relink_project_asset", _) => {
                if let Some(id) = selected_row_id.as_deref() {
                    self.relink_project_asset(id);
                }
            }
            ("relink_scene_asset", _) => {
                if let Some(id) = selected_row_id {
                    self.relink_scene_asset(&id);
                }
            }
            ("cancel_video_import", _) => self.cancel_video_import(),
            ("audio_volume_changed", Some(ActionValue::Text(value))) => {
                self.update_selected_audio_volume(&value)
            }
            ("keyframe_audio_volume", _) => self.add_selected_audio_volume_keyframe(),
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
