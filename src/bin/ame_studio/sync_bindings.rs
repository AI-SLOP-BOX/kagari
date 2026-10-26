use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_bindings_logged(&mut self) {
        if let Err(error) = self.sync_bindings() {
            log::error!("[kagari-studio] sync_bindings failed: {error}");
            #[cfg(debug_assertions)]
            {
                eprintln!("[kagari-studio:debug] sync_bindings failed: {error}");
            }
        }
    }

    pub(crate) fn sync_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_composition_project_items();
        self.sync_environment_bindings()?;
        let project_summary = self
            .project_model
            .compositions
            .get(self.project_model.active_composition_idx)
            .map(|composition| {
                format!(
                    "{}  ·  {} × {}  ·  {} fps  ·  {} 秒  ·  {} レイヤー",
                    composition.name,
                    composition.width,
                    composition.height,
                    composition.fps,
                    format_duration_seconds(
                        composition.duration_frames,
                        composition.fps,
                    ),
                    composition.layers.len(),
                )
            })
            .unwrap_or_else(|| "コンポジションなし".to_owned());
        self.ui
            .set_binding("project_summary", Value::String(project_summary))?;
        let active_composition = self.project_model.active_composition();
        let duration_text = format_duration_seconds(
            active_composition.duration_frames,
            active_composition.fps,
        );
        for (name, value) in [
            ("composition_width", active_composition.width.to_string()),
            ("composition_height", active_composition.height.to_string()),
            ("composition_fps", active_composition.fps.to_string()),
            ("composition_duration", duration_text),
        ] {
            self.ui.set_binding(name, Value::String(value))?;
        }
        self.ui.set_binding(
            "position_x_min",
            Value::Float(
                (-f64::from(active_composition.width))
                    .min(self.position_x - f64::from(active_composition.width)),
            ),
        )?;
        self.ui.set_binding(
            "position_x_max",
            Value::Float(
                (f64::from(active_composition.width) * 2.0)
                    .max(self.position_x + f64::from(active_composition.width)),
            ),
        )?;
        self.ui.set_binding(
            "position_y_min",
            Value::Float(
                (-f64::from(active_composition.height))
                    .min(self.position_y - f64::from(active_composition.height)),
            ),
        )?;
        self.ui.set_binding(
            "position_y_max",
            Value::Float(
                (f64::from(active_composition.height) * 2.0)
                    .max(self.position_y + f64::from(active_composition.height)),
            ),
        )?;
        self.ui.set_binding(
            "anchor_x_min",
            Value::Float(
                (-f64::from(active_composition.width))
                    .min(self.anchor_x - f64::from(active_composition.width)),
            ),
        )?;
        self.ui.set_binding(
            "anchor_x_max",
            Value::Float(
                (f64::from(active_composition.width) * 2.0)
                    .max(self.anchor_x + f64::from(active_composition.width)),
            ),
        )?;
        self.ui.set_binding(
            "anchor_y_min",
            Value::Float(
                (-f64::from(active_composition.height))
                    .min(self.anchor_y - f64::from(active_composition.height)),
            ),
        )?;
        self.ui.set_binding(
            "anchor_y_max",
            Value::Float(
                (f64::from(active_composition.height) * 2.0)
                    .max(self.anchor_y + f64::from(active_composition.height)),
            ),
        )?;
        self.ui
            .set_binding("asset_importing", Value::Bool(self.video_import.is_some()))?;
        self.ui.set_binding(
            "asset_import_cancellable",
            Value::Bool(self.video_import.as_ref().is_some_and(|job| job.cancel_supported)),
        )?;
        self.ui.set_binding(
            "composition_name",
            Value::String(active_composition.name.clone()),
        )?;
        self.ui.set_binding(
            "composition_start_timecode",
            Value::String(format_timecode(
                active_composition.start_timecode_frames,
                active_composition.fps,
            )),
        )?;
        self.ui.set_binding(
            "composition_settings_status",
            Value::String(self.composition_settings_status.clone()),
        )?;
        use kagari_vfx::core::proxy::ProxyResolution;
        for (name, resolution) in [
            ("composition_proxy_full", ProxyResolution::Full),
            ("composition_proxy_half", ProxyResolution::Half),
            ("composition_proxy_quarter", ProxyResolution::Quarter),
            ("composition_proxy_eighth", ProxyResolution::Eighth),
        ] {
            self.ui.set_binding(
                name,
                Value::Bool(active_composition.comp_proxy.global_resolution == resolution),
            )?;
        }
        self.ui.set_binding(
            "composition_proxy_preview",
            Value::Bool(active_composition.comp_proxy.active_in_preview),
        )?;
        self.ui.set_binding(
            "composition_blend_linear",
            Value::Bool(active_composition.blend_linear),
        )?;
        self.ui.set_binding(
            "composition_dither_output",
            Value::Bool(active_composition.dither_output),
        )?;
        self.ui.set_binding(
            "composition_pixel_aspect",
            Value::String(format!("{:.3}", active_composition.pixel_aspect)),
        )?;
        self.ui.set_binding(
            "composition_shutter_angle",
            Value::String(format!(
                "{:.0}",
                active_composition.motion_blur_shutter_angle
            )),
        )?;
        self.ui.set_binding(
            "composition_shutter_phase",
            Value::String(format!(
                "{:.0}",
                active_composition.motion_blur_shutter_phase
            )),
        )?;
        self.ui.set_binding(
            "composition_background",
            Value::String(rgba_to_hex(&active_composition.background_color)),
        )?;
        self.ui.set_binding(
            "composition_bit_depth",
            Value::String(active_composition.bit_depth.label().to_owned()),
        )?;
        self.ui.set_binding(
            "composition_working_space",
            Value::String(active_composition.working_color_space.label().to_owned()),
        )?;
        self.ui.set_binding(
            "composition_position",
            Value::String(format!(
                "{} / {}",
                self.project_model.active_composition_idx + 1,
                self.project_model.compositions.len()
            )),
        )?;
        self.ui.set_binding(
            "has_multiple_compositions",
            Value::Bool(self.project_model.compositions.len() > 1),
        )?;
        let composition_items = self
            .project_model
            .compositions
            .iter()
            .enumerate()
            .map(|(index, composition)| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(index.to_string())),
                    ("name".to_owned(), Value::String(composition.name.clone())),
                    (
                        "selected".to_owned(),
                        Value::Bool(index == self.project_model.active_composition_idx),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui
            .set_binding("composition_items", Value::List(composition_items))?;
        self.ui.set_binding(
            "status_frame",
            Value::String(format!(
                "{} / {} f",
                self.current_frame.min(active_composition.duration_frames),
                active_composition.duration_frames,
            )),
        )?;
        self.ui.set_binding(
            "status_resolution",
            Value::String(format!(
                "{} × {}",
                active_composition.width, active_composition.height,
            )),
        )?;
        let sw_reason = kagari_vfx::core::render_capability::software_preview_reason(
            active_composition,
            self.current_frame,
        );
        let preview_badge = if sw_reason.is_none() { "GPU プレビュー" } else { "CPU プレビュー" };
        self.ui.set_binding(
            "preview_badge",
            Value::String(preview_badge.to_owned()),
        )?;
        self.ui.set_binding(
            "preview_fallback_reason",
            Value::String(sw_reason.unwrap_or("").to_owned()),
        )?;
        self.ui.set_binding(
            "fps_label",
            Value::String(format!("{} FPS", active_composition.fps)),
        )?;
        self.ui.set_binding(
            "duration_label",
            Value::String(format!(
                "{} 秒",
                format_duration_seconds(
                    active_composition.duration_frames,
                    active_composition.fps,
                ),
            )),
        )?;
        let visible_timeline_layers = self.timeline_layers();
        self.ui.set_binding(
            "timeline_canvas_height",
            Value::Float(22.0 + visible_timeline_layers.len().max(1) as f64 * 28.0),
        )?;
        self.ui.set_binding(
            "timeline_zoom_label",
            Value::String(format!("{:.0}%", self.timeline_zoom * 100.0)),
        )?;
        let total_frames = active_composition.duration_frames.max(1) as f64;
        let visible_frames = (total_frames / self.timeline_zoom).clamp(1.0, total_frames);
        let timeline_start = self
            .timeline_start_frame
            .clamp(0.0, (total_frames - visible_frames).max(0.0));
        let ruler_labels = (0..=5)
            .map(|tick| {
                let frame = (timeline_start + visible_frames * tick as f64 / 5.0).round() as u64;
                format_timeline_seconds(
                    u64::from(active_composition.start_timecode_frames).saturating_add(frame),
                    active_composition.fps,
                )
            })
            .collect::<Vec<_>>();
        for (index, label) in ruler_labels.into_iter().enumerate() {
            self.ui
                .set_binding(&format!("timeline_time_{index}"), Value::String(label))?;
        }
        self.ui.set_binding(
            "workspace_left_ratio",
            Value::Float(self.workspace_left_ratios[self.workspace.index()]),
        )?;
        self.ui.set_binding(
            "workspace_center_ratio",
            Value::Float(self.workspace_center_ratios[self.workspace.index()]),
        )?;
        self.ui.set_binding(
            "workspace_vertical_ratio",
            Value::Float(self.workspace_vertical_ratios[self.workspace.index()]),
        )?;
        self.ui
            .set_binding("render_queue", Value::Bool(self.render_queue))?;
        self.ui
            .set_binding("render_progress", Value::Int(self.render_progress))?;
        self.ui
            .set_binding("render_running", Value::Bool(self.render_events.is_some()))?;
        self.ui
            .set_binding("render_status", Value::String(self.render_status.clone()))?;
        self.ui.set_binding(
            "video_output_path",
            Value::String(self.video_output_path.clone()),
        )?;
        self.ui.set_binding(
            "video_export_codec",
            Value::String(match self.video_export_codec_idx % 4 {
                0 => "H.264 · MP4",
                1 => "H.265 · MP4",
                2 => "ProRes 422 · MOV",
                _ => "ProRes 4444 · MOV",
            }.to_owned()),
        )?;
        self.ui.set_binding(
            "video_overwrite_pending",
            Value::Bool(self.video_overwrite_pending),
        )?;
        let selected_layer_kind = self
            .selected_layer_id
            .as_deref()
            .and_then(|id| {
                self.project_model
                    .active_composition()
                    .layers
                    .iter()
                    .find(|layer| layer.id == id)
            })
            .map(scene_layer_kind_label)
            .unwrap_or("未選択");
        self.ui.set_binding(
            "selected_layer_kind",
            Value::String(selected_layer_kind.to_owned()),
        )?;
        for (name, value) in [
            ("layer_name", self.layer_name.clone()),
            ("search", self.search.clone()),
            (
                "current_time",
                format_timecode(
                    self.current_frame.saturating_add(
                        self.project_model.active_composition().start_timecode_frames,
                    ),
                    self.project_model.active_composition().fps,
                ),
            ),
            ("zoom", self.zoom.clone()),
            ("effect_search", self.effect_search.clone()),
            ("asset_status", self.asset_status.clone()),
        ] {
            self.ui.set_binding(name, Value::String(value))?;
        }
        for (name, value) in [
            ("position_x", self.position_x),
            ("position_y", self.position_y),
            ("anchor_x", self.anchor_x),
            ("anchor_y", self.anchor_y),
            ("rotation_degrees", self.rotation_degrees),
            ("scale_x", self.scale_x),
            ("scale_y", self.scale_y),
            ("opacity", self.opacity),
        ] {
            self.ui.set_binding(name, Value::Float(value))?;
        }
        self.ui
            .set_binding("selected_layer", Value::Bool(self.selected_layer))?;
        self.ui.set_binding(
            "selected_layer_locked",
            Value::Bool(self.selected_layer_id.as_deref().is_some_and(|id| {
                self.project_model
                    .active_composition()
                    .layers
                    .iter()
                    .any(|layer| layer.id == id && layer.locked)
            })),
        )?;
        self.ui
            .set_binding("is_playing", Value::Bool(self.is_playing))?;
        self.ui
            .set_binding("playback_loop", Value::Bool(self.playback_loop))?;
        let work_area_label = self
            .playback_range
            .map(|(start, end)| {
                let last = active_composition.duration_frames.saturating_sub(1);
                let start = start.min(last);
                let end = end.min(last).max(start);
                format!(
                    "範囲 {}–{}",
                    format_timecode(
                        start.saturating_add(active_composition.start_timecode_frames),
                        active_composition.fps,
                    ),
                    format_timecode(
                        end.saturating_add(active_composition.start_timecode_frames),
                        active_composition.fps,
                    )
                )
            })
            .unwrap_or_else(|| "全体".to_owned());
        self.ui
            .set_binding("work_area_label", Value::String(work_area_label))?;
        let last_frame = active_composition.duration_frames.saturating_sub(1);
        let (work_area_start, work_area_end) = self.playback_range
            .map(|(start, end)| (start.min(last_frame), end.min(last_frame).max(start.min(last_frame))))
            .unwrap_or((0, last_frame));
        self.ui.set_binding(
            "work_area_start_frame",
            Value::String(work_area_start.to_string()),
        )?;
        self.ui.set_binding(
            "work_area_end_frame",
            Value::String(work_area_end.to_string()),
        )?;

        let query = self.search.trim().to_lowercase();
        let composition_layers = &self.project_model.active_composition().layers;
        let scene_layer_by_id = composition_layers
            .iter()
            .map(|layer| (layer.id.as_str(), layer))
            .collect::<std::collections::HashMap<_, _>>();
        let scene_search_visible =
            scene_search_visible_layer_ids(composition_layers, &scene_layer_by_id, &query);
        let scene_parent_ids = composition_layers
            .iter()
            .filter_map(|layer| layer.parent_id.as_deref())
            .collect::<std::collections::HashSet<_>>();
        let scene_layer_depths = scene_layer_depths(composition_layers, &scene_layer_by_id);
        let scene_layers = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .filter(|layer| scene_search_visible.contains(&layer.id))
            .filter(|layer| {
                !query.is_empty()
                    || !scene_layer_hidden_by_collapse(
                        &scene_layer_by_id,
                        layer,
                        &self.collapsed_scene_layers,
                    )
            })
            .map(|layer| {
                let kind = scene_layer_kind_label(layer);
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(layer.id.clone())),
                    ("name".to_owned(), Value::String(layer.name.clone())),
                    ("kind".to_owned(), Value::String(kind.to_owned())),
                    (
                        "label".to_owned(),
                        Value::String(layer_label_color_label(layer.label).to_owned()),
                    ),
                    (
                        "indent".to_owned(),
                        Value::String(
                            "　".repeat(scene_layer_depths.get(layer.id.as_str()).copied().unwrap_or(0)),
                        ),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    ),
                    (
                        "align_selected".to_owned(),
                        Value::Bool(self.align_selected_layer_ids.contains(&layer.id)),
                    ),
                    (
                        "has_children".to_owned(),
                        Value::Bool(scene_parent_ids.contains(layer.id.as_str())),
                    ),
                    (
                        "collapsed".to_owned(),
                        Value::Bool(self.collapsed_scene_layers.contains(&layer.id)),
                    ),
                    ("visible".to_owned(), Value::Bool(layer.visible)),
                    ("locked".to_owned(), Value::Bool(layer.locked)),
                    ("solo".to_owned(), Value::Bool(layer.solo)),
                    ("motion_blur".to_owned(), Value::Bool(layer.motion_blur)),
                    ("has_audio".to_owned(), Value::Bool(matches!(&layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Audio { .. }
                            | kagari_vfx::core::timeline::LayerType::Video { audio_wav: Some(_), .. }
                    ))),
                    ("audio_muted".to_owned(), Value::Bool(layer.audio_muted)),
                    ("blend_mode".to_owned(), Value::String(blend_mode_label(layer.blend_mode).to_owned())),
                    (
                        "track_matte".to_owned(),
                        Value::String(match layer.track_matte {
                            kagari_vfx::core::timeline::TrackMatteMode::None => "なし".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte => "α".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatteInverted => "α反転".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatte => "輝度".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatteInverted => "輝度反転".to_owned(),
                        }),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_scene_layers",
            Value::Bool(!self.project_model.active_composition().layers.is_empty()),
        )?;
        self.ui.set_binding(
            "has_scene_layer_matches",
            Value::Bool(!scene_layers.is_empty()),
        )?;
        self.ui
            .set_binding("scene_layers", Value::List(scene_layers))?;
        self.ui.set_binding(
            "align_to_composition",
            Value::Bool(self.align_to_composition),
        )?;
        let active_layers = &self.project_model.active_composition().layers;
        let mut align_selected_count = active_layers
            .iter()
            .filter(|layer| {
                !layer.locked && self.align_selected_layer_ids.contains(&layer.id)
            })
            .count();
        if self.align_selected_layer_ids.is_empty()
            && self.selected_layer_id.as_deref().is_some_and(|id| {
                active_layers
                    .iter()
                    .any(|layer| layer.id == id && !layer.locked)
            })
        {
            align_selected_count = 1;
        }
        self.ui.set_binding(
            "align_selected_count",
            Value::Int(align_selected_count as i64),
        )?;
        self.ui.set_binding(
            "can_align_layers",
            Value::Bool(align_selected_count >= 1),
        )?;
        self.ui.set_binding(
            "can_distribute_layers",
            Value::Bool(align_selected_count >= 3),
        )?;

        let all_layers = &self.project_model.active_composition().layers;
        let current_parent_id = self
            .selected_layer_id
            .as_deref()
            .and_then(|id| all_layers.iter().find(|layer| layer.id == id))
            .and_then(|layer| layer.parent_id.as_deref());
        let parent_candidates = self
            .selected_layer_id
            .as_deref()
            .map(|selected_id| {
                all_layers
                    .iter()
                    .filter_map(|candidate| {
                        if candidate.id == selected_id {
                            return None;
                        }
                        let mut ancestor = candidate.parent_id.as_deref();
                        let mut steps = 0usize;
                        while let Some(parent_id) = ancestor {
                            if parent_id == selected_id {
                                return None;
                            }
                            steps += 1;
                            if steps > all_layers.len() {
                                return None;
                            }
                            ancestor = all_layers
                                .iter()
                                .find(|layer| layer.id == parent_id)
                                .and_then(|layer| layer.parent_id.as_deref());
                        }
                        Some(Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(candidate.id.clone())),
                            ("name".to_owned(), Value::String(candidate.name.clone())),
                            (
                                "selected".to_owned(),
                                Value::Bool(current_parent_id == Some(candidate.id.as_str())),
                            ),
                        ])))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ui.set_binding(
            "has_parent_candidates",
            Value::Bool(!parent_candidates.is_empty()),
        )?;
        self.ui
            .set_binding("parent_candidates", Value::List(parent_candidates))?;
        self.ui.set_binding(
            "selected_layer_has_parent",
            Value::Bool(current_parent_id.is_some()),
        )?;
        let current_parent_name = current_parent_id
            .and_then(|id| all_layers.iter().find(|layer| layer.id == id))
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| "なし".to_owned());
        self.ui.set_binding(
            "selected_layer_parent_name",
            Value::String(current_parent_name),
        )?;

        let audio_tracks: Vec<Value> = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .filter_map(|layer| {
                let source_kind = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::Audio { .. } => Some("音声素材"),
                    kagari_vfx::core::timeline::LayerType::Video {
                        audio_wav: Some(_), ..
                    } => Some("動画内音声"),
                    _ => None,
                }?;
                let volume_track = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => Some(volume),
                    kagari_vfx::core::timeline::LayerType::Video {
                        audio_wav: Some(_),
                        audio_volume,
                        ..
                    } => Some(audio_volume),
                    _ => None,
                };
                let volume = volume_track.map(|track| track.evaluate(self.current_frame));
                let keyframe_here = volume_track.is_some_and(|track| {
                    track.keyframes().is_some_and(|keyframes| {
                        keyframes.iter().any(|key| key.frame == self.current_frame)
                    })
                });
                Some(Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(layer.id.clone())),
                    ("name".to_owned(), Value::String(layer.name.clone())),
                    (
                        "source_kind".to_owned(),
                        Value::String(source_kind.to_owned()),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    ),
                    ("locked".to_owned(), Value::Bool(layer.locked)),
                    ("solo".to_owned(), Value::Bool(layer.solo)),
                    ("muted".to_owned(), Value::Bool(layer.audio_muted)),
                    ("has_volume".to_owned(), Value::Bool(volume.is_some())),
                    ("keyframe_here".to_owned(), Value::Bool(keyframe_here)),
                    (
                        "volume".to_owned(),
                        Value::String(
                            volume
                                .map(|value| format!("{value:.2}"))
                                .unwrap_or_default(),
                        ),
                    ),
                ])))
            })
            .collect();
        self.ui
            .set_binding("has_audio_tracks", Value::Bool(!audio_tracks.is_empty()))?;
        self.ui
            .set_binding("audio_tracks", Value::List(audio_tracks))?;

        let timeline_layers: Vec<Value> = visible_timeline_layers
            .into_iter()
            .map(|layer| {
                let kind = scene_layer_kind_label(&layer);
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(layer.id.clone())),
                    ("name".to_owned(), Value::String(layer.name.clone())),
                    ("kind".to_owned(), Value::String(kind.to_owned())),
                    (
                        "label".to_owned(),
                        Value::String(layer_label_color_badge(layer.label).to_owned()),
                    ),
                    (
                        "indent".to_owned(),
                        Value::String(
                            "　".repeat(scene_layer_depths.get(layer.id.as_str()).copied().unwrap_or(0)),
                        ),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    ),
                    (
                        "align_selected".to_owned(),
                        Value::Bool(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    ),
                    (
                        "has_children".to_owned(),
                        Value::Bool(scene_parent_ids.contains(layer.id.as_str())),
                    ),
                    (
                        "collapsed".to_owned(),
                        Value::Bool(self.collapsed_scene_layers.contains(&layer.id)),
                    ),
                    ("visible".to_owned(), Value::Bool(layer.visible)),
                    ("locked".to_owned(), Value::Bool(layer.locked)),
                    ("solo".to_owned(), Value::Bool(layer.solo)),
                    ("motion_blur".to_owned(), Value::Bool(layer.motion_blur)),
                    ("has_audio".to_owned(), Value::Bool(matches!(&layer.layer_type,
                        kagari_vfx::core::timeline::LayerType::Audio { .. }
                            | kagari_vfx::core::timeline::LayerType::Video { audio_wav: Some(_), .. }
                    ))),
                    ("audio_muted".to_owned(), Value::Bool(layer.audio_muted)),
                    ("blend_mode".to_owned(), Value::String(blend_mode_label(layer.blend_mode).to_owned())),
                    (
                        "track_matte".to_owned(),
                        Value::String(match layer.track_matte {
                            kagari_vfx::core::timeline::TrackMatteMode::None => "なし".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte => "α".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatteInverted => "α反転".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatte => "輝度".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatteInverted => "輝度反転".to_owned(),
                        }),
                    ),
                ]))
            })
            .collect();
        self.ui.set_binding(
            "has_timeline_layers",
            Value::Bool(!timeline_layers.is_empty()),
        )?;
        self.ui
            .set_binding("timeline_layers", Value::List(timeline_layers))?;

        let composition = self.project_model.active_composition();
        let mut markers = composition.markers.iter().collect::<Vec<_>>();
        markers.sort_by_key(|marker| marker.frame);
        let timeline_markers = markers
            .into_iter()
            .map(|marker| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(marker.frame.to_string())),
                    ("label".to_owned(), Value::String(marker.label.clone())),
                    (
                        "time".to_owned(),
                        Value::String(format_timecode(
                            marker.frame.saturating_add(composition.start_timecode_frames),
                            composition.fps,
                        )),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_marker_frame == Some(marker.frame)),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_timeline_markers",
            Value::Bool(!timeline_markers.is_empty()),
        )?;
        self.ui.set_binding(
            "has_selected_timeline_marker",
            Value::Bool(self.selected_marker_frame.is_some_and(|frame| {
                composition.markers.iter().any(|marker| marker.frame == frame)
            })),
        )?;
        self.ui
            .set_binding("timeline_markers", Value::List(timeline_markers))?;
        let selected_marker_label = self
            .selected_marker_frame
            .and_then(|frame| {
                composition
                    .markers
                    .iter()
                    .find(|marker| marker.frame == frame)
            })
            .map(|marker| marker.label.clone())
            .unwrap_or_default();
        self.ui.set_binding(
            "timeline_marker_label",
            Value::String(selected_marker_label),
        )?;
        self.ui.set_binding(
            "timeline_marker_frame",
            Value::String(self.selected_marker_frame.map(|frame| frame.to_string()).unwrap_or_default()),
        )?;
        let selected_layer_markers = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });
        let mut layer_markers = selected_layer_markers
            .map(|layer| layer.markers.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        layer_markers.sort_by_key(|marker| marker.frame);
        let layer_marker_items = layer_markers
            .iter()
            .map(|marker| {
                Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(marker.frame.to_string())),
                    ("label".to_owned(), Value::String(marker.label.clone())),
                    (
                        "time".to_owned(),
                        Value::String(format_timecode(
                            marker.frame.saturating_add(composition.start_timecode_frames),
                            composition.fps,
                        )),
                    ),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_layer_marker_frame == Some(marker.frame)),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_layer_markers",
            Value::Bool(!layer_marker_items.is_empty()),
        )?;
        self.ui.set_binding(
            "has_selected_layer_marker",
            Value::Bool(self.selected_layer_marker_frame.is_some_and(|frame| {
                selected_layer_markers.is_some_and(|layer| {
                    layer.markers.iter().any(|marker| marker.frame == frame)
                })
            })),
        )?;
        self.ui
            .set_binding("layer_markers", Value::List(layer_marker_items))?;
        let selected_layer_marker_label = self
            .selected_layer_marker_frame
            .and_then(|frame| {
                selected_layer_markers
                    .and_then(|layer| layer.markers.iter().find(|marker| marker.frame == frame))
            })
            .map(|marker| marker.label.clone())
            .unwrap_or_default();
        self.ui.set_binding(
            "layer_marker_label",
            Value::String(selected_layer_marker_label),
        )?;
        self.ui.set_binding(
            "layer_marker_frame",
            Value::String(self.selected_layer_marker_frame.map(|frame| frame.to_string()).unwrap_or_default()),
        )?;

        let scene_assets = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .filter_map(|layer| {
                let (kind, path, missing) = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::Image { path } => {
                        ("画像", path, !std::path::Path::new(path).is_file())
                    }
                    kagari_vfx::core::timeline::LayerType::Video {
                        source, frames_dir, ..
                    } => (
                        "動画",
                        source,
                        !std::path::Path::new(source).is_file()
                            || !std::path::Path::new(frames_dir).is_dir(),
                    ),
                    kagari_vfx::core::timeline::LayerType::Audio { path, .. } => {
                        ("音声", path, !std::path::Path::new(path).is_file())
                    }
                    kagari_vfx::core::timeline::LayerType::Model3D { path } => {
                        ("3Dモデル", path, !std::path::Path::new(path).is_file())
                    }
                    _ => return None,
                };
                let source = std::path::Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                Some(Value::Record(std::collections::BTreeMap::from([
                    ("id".to_owned(), Value::String(layer.id.clone())),
                    ("name".to_owned(), Value::String(layer.name.clone())),
                    ("kind".to_owned(), Value::String(kind.to_owned())),
                    ("source".to_owned(), Value::String(source)),
                    ("source_path".to_owned(), Value::String(path.clone())),
                    (
                        "selected".to_owned(),
                        Value::Bool(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    ),
                    ("locked".to_owned(), Value::Bool(layer.locked)),
                    ("missing".to_owned(), Value::Bool(missing)),
                ])))
            })
            .collect::<Vec<_>>();
        self.ui
            .set_binding("has_scene_assets", Value::Bool(!scene_assets.is_empty()))?;
        self.ui
            .set_binding("scene_assets", Value::List(scene_assets))?;
        self.sync_project_asset_bindings()?;

        let selected_particle = self
            .selected_layer_id
            .as_deref()
            .and_then(|id| {
                self.project_model
                    .active_composition()
                    .layers
                    .iter()
                    .find(|layer| layer.id == id)
            })
            .and_then(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Particle { emitter } => Some(emitter),
                _ => None,
            });
        self.ui.set_binding(
            "selected_is_particle",
            Value::Bool(selected_particle.is_some()),
        )?;
        self.ui.set_binding(
            "emitter_shape_circle",
            Value::Bool(selected_particle.is_some_and(|emitter| {
                emitter.shape == kagari_vfx::core::particle_system::EmitterShape::Circle
            })),
        )?;
        for (name, value) in [
            ("emitter_rate", selected_particle.map(|e| e.rate)),
            (
                "emitter_radius",
                selected_particle.map(|e| e.emitter_size[0] * 0.5),
            ),
            ("emitter_spread", selected_particle.map(|e| e.spread_degrees)),
            ("velocity", selected_particle.map(|e| e.speed)),
            (
                "velocity_random",
                selected_particle.map(|e| e.speed_variance),
            ),
            ("lifetime", selected_particle.map(|e| e.lifetime)),
            ("particle_size", selected_particle.map(|e| e.size_start)),
            ("size_random", selected_particle.map(|e| e.size_end)),
            ("emitter_gravity_y", selected_particle.map(|e| e.gravity[1])),
            ("emitter_wind_x", selected_particle.map(|e| e.wind[0])),
            ("emitter_turbulence", selected_particle.map(|e| e.turbulence)),
            ("emitter_drag", selected_particle.map(|e| e.drag)),
        ] {
            self.ui.set_binding(
                name,
                Value::String(value.map(|n| format!("{n:.2}")).unwrap_or_default()),
            )?;
        }

        self.sync_layer_styles_bindings()?;
        self.sync_composite_bindings()?;

        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });
        self.ui.set_binding(
            "layer_in_frame",
            Value::String(selected_layer.map_or(0, |layer| layer.in_frame).to_string()),
        )?;
        self.ui.set_binding(
            "layer_out_frame",
            Value::String(
                selected_layer
                    .map_or(0, |layer| layer.out_frame)
                    .to_string(),
            ),
        )?;
        let horizontal_constraint = selected_layer
            .map(|layer| match layer.constraints.horizontal {
                kagari_vfx::core::layer_constraints::HorizontalPin::Left => "左固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Center => "中央固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Right => "右固定",
                kagari_vfx::core::layer_constraints::HorizontalPin::Scale => "幅に比例",
            })
            .unwrap_or("左固定");
        let vertical_constraint = selected_layer
            .map(|layer| match layer.constraints.vertical {
                kagari_vfx::core::layer_constraints::VerticalPin::Top => "上固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Center => "中央固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Bottom => "下固定",
                kagari_vfx::core::layer_constraints::VerticalPin::Scale => "高さに比例",
            })
            .unwrap_or("上固定");
        self.ui.set_binding(
            "layer_constraint_horizontal",
            Value::String(horizontal_constraint.to_owned()),
        )?;
        self.ui.set_binding(
            "layer_constraint_vertical",
            Value::String(vertical_constraint.to_owned()),
        )?;
        let active_mask_id = selected_layer.and_then(|layer| {
            layer
                .masks
                .iter()
                .find(|mask| self.selected_mask_id.as_deref() == Some(mask.id.as_str()))
                .or_else(|| layer.masks.first())
                .map(|mask| mask.id.clone())
        });
        self.selected_mask_id = active_mask_id.clone();
        let selected_mask = selected_layer.and_then(|layer| {
            active_mask_id
                .as_deref()
                .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
        });
        let layer_masks = selected_layer
            .map(|layer| {
                layer
                    .masks
                    .iter()
                    .map(|mask| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(mask.id.clone())),
                            ("name".to_owned(), Value::String(mask.name.clone())),
                            (
                                "selected".to_owned(),
                                Value::Bool(active_mask_id.as_deref() == Some(mask.id.as_str())),
                            ),
                            ("enabled".to_owned(), Value::Bool(mask.enabled)),
                            (
                                "mode".to_owned(),
                                Value::String(mask_mode_label(mask.mode).to_owned()),
                            ),
                            ("inverted".to_owned(), Value::Bool(mask.inverted)),
                        ]))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ui
            .set_binding("has_layer_masks", Value::Bool(!layer_masks.is_empty()))?;
        self.ui
            .set_binding("layer_masks", Value::List(layer_masks))?;
        self.ui
            .set_binding("hide_shy_layers", Value::Bool(self.hide_shy_layers))?;
        self.ui.set_binding(
            "layer_frame_blending",
            Value::Bool(selected_layer.is_some_and(|layer| layer.frame_blending)),
        )?;
        self.ui.set_binding(
            "layer_motion_blur",
            Value::Bool(selected_layer.is_some_and(|layer| layer.motion_blur)),
        )?;
        self.ui.set_binding(
            "layer_preserve_transparency",
            Value::Bool(selected_layer.is_some_and(|layer| layer.preserve_transparency)),
        )?;
        self.ui.set_binding(
            "layer_guide",
            Value::Bool(selected_layer.is_some_and(|layer| layer.is_guide_layer)),
        )?;
        self.ui.set_binding(
            "layer_shy",
            Value::Bool(selected_layer.is_some_and(|layer| layer.is_shy)),
        )?;
        self.ui.set_binding(
            "layer_effects_enabled",
            Value::Bool(selected_layer.is_some_and(|layer| layer.effects_enabled)),
        )?;
        let auto_orient = selected_layer.map(|layer| layer.auto_orient);
        let auto_orient_label = auto_orient.map(auto_orient_label).unwrap_or("なし");
        self.ui.set_binding(
            "layer_auto_orient_label",
            Value::String(auto_orient_label.to_owned()),
        )?;
        let auto_orient_point = auto_orient.and_then(|mode| match mode {
            kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint { target_point } => {
                Some(target_point)
            }
            _ => None,
        });
        self.ui.set_binding(
            "layer_auto_orient_point_x",
            Value::String(
                auto_orient_point
                    .map(|point| format!("{:.1}", point[0]))
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "layer_auto_orient_point_y",
            Value::String(
                auto_orient_point
                    .map(|point| format!("{:.1}", point[1]))
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "layer_label_color_label",
            Value::String(
                selected_layer
                    .map(|layer| layer_label_color_label(layer.label).to_owned())
                    .unwrap_or_else(|| "なし".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "selected_mask_name",
            Value::String(
                selected_mask
                    .map(|mask| mask.name.clone())
                    .unwrap_or_default(),
            ),
        )?;
        self.ui.set_binding(
            "mask_mode_label",
            Value::String(
                selected_mask
                    .map(|mask| mask_mode_label(mask.mode))
                    .unwrap_or("加算")
                    .to_owned(),
            ),
        )?;
        self.ui.set_binding(
            "mask_inverted",
            Value::Bool(selected_mask.is_some_and(|mask| mask.inverted)),
        )?;
        for (name, value) in [
            (
                "mask_feather",
                selected_mask.map(|mask| mask.feather.evaluate(self.current_frame)),
            ),
            (
                "mask_opacity",
                selected_mask.map(|mask| mask.opacity.evaluate(self.current_frame)),
            ),
            (
                "mask_expansion",
                selected_mask.map(|mask| mask.expansion.evaluate(self.current_frame)),
            ),
        ] {
            self.ui.set_binding(
                name,
                Value::String(value.map(|value| format!("{value:.2}")).unwrap_or_default()),
            )?;
        }
        let selected_keyframes = selected_layer
            .map(|layer| {
                layer_keyframe_frames_for_composition(
                    self.project_model.active_composition(),
                    layer,
                )
            })
            .unwrap_or_default();
        self.ui.set_binding(
            "keyframe_status",
            Value::String(if selected_keyframes.is_empty() {
                "選択レイヤーにキーなし".to_owned()
            } else {
                format!("{} キー", selected_keyframes.len())
            }),
        )?;
        let graph_effect = selected_layer.and_then(|layer| {
            self.selected_effect_id
                .as_deref()
                .and_then(|id| layer.effects.iter().find(|effect| effect.id == id))
                .filter(|effect| {
                    !scalar_effect_parameters(&effect.effect_type).is_empty()
                        || !effect_color_parameters(&effect.effect_type).is_empty()
                })
                .or_else(|| {
                    layer.effects.iter().find(|effect| {
                        !scalar_effect_parameters(&effect.effect_type).is_empty()
                            || !effect_color_parameters(&effect.effect_type).is_empty()
                    })
                })
        });
        let graph_effect_parameters = graph_effect
            .map(|effect| scalar_effect_parameters(&effect.effect_type))
            .unwrap_or_default();
        let graph_effect_colors = graph_effect
            .map(|effect| effect_color_parameters(&effect.effect_type))
            .unwrap_or_default();
        let mut graph_color_channel = matches!(
            self.graph_channel,
            GraphChannel::EffectColorRed
                | GraphChannel::EffectColorGreen
                | GraphChannel::EffectColorBlue
                | GraphChannel::EffectColorAlpha
        );
        if selected_layer.is_some_and(|layer| !self.graph_channel.supports(layer))
            || (self.graph_channel == GraphChannel::EffectParameter
                && graph_effect_parameters.is_empty())
            || (graph_color_channel && graph_effect_colors.is_empty())
        {
            self.graph_channel = GraphChannel::PositionX;
            self.graph_effect_parameter_id = None;
            graph_color_channel = false;
        }
        if (self.graph_channel == GraphChannel::EffectParameter || graph_color_channel)
            && graph_effect.is_some_and(|effect| {
                Some(effect.id.as_str()) != self.selected_effect_id.as_deref()
            })
        {
            self.selected_effect_id = graph_effect.map(|effect| effect.id.clone());
        }
        if self.graph_channel == GraphChannel::EffectParameter || graph_color_channel {
            let params = if self.graph_channel == GraphChannel::EffectParameter {
                &graph_effect_parameters
            } else {
                &graph_effect_colors
            };
            let current_exists = params
                .iter()
                .any(|(id, _)| Some(id.as_str()) == self.graph_effect_parameter_id.as_deref());
            if !current_exists {
                self.graph_effect_parameter_id = params.first().map(|(id, _)| id.clone());
            }
        }
        let graph_channel_label = if self.graph_channel == GraphChannel::EffectParameter {
            selected_layer
                .and_then(|layer| {
                    graph_effect_parameter_label(
                        layer,
                        graph_effect.map(|effect| effect.id.as_str()),
                        self.graph_effect_parameter_id.as_deref(),
                    )
                })
                .unwrap_or_else(|| self.graph_channel.label().to_owned())
        } else if graph_color_channel {
            graph_effect
                .and_then(|effect| {
                    effect_color_parameters(&effect.effect_type)
                        .into_iter()
                        .find(|(id, _)| {
                            Some(id.as_str()) == self.graph_effect_parameter_id.as_deref()
                        })
                        .map(|(_, label)| {
                            format!(
                                "{} · {} · {}",
                                effect.name,
                                label,
                                graph_color_component_name(self.graph_channel)
                            )
                        })
                })
                .unwrap_or_else(|| self.graph_channel.label().to_owned())
        } else {
            self.graph_channel.label().to_owned()
        };
        let effect_value_channel =
            self.graph_channel == GraphChannel::EffectParameter || graph_color_channel;
        self.ui
            .set_binding("graph_effect_channel", Value::Bool(effect_value_channel))?;
        self.ui
            .set_binding("graph_channel_label", Value::String(graph_channel_label))?;
        let graph_channels = selected_layer
            .map(|layer| {
                GraphChannel::ALL
                    .iter()
                    .enumerate()
                    .filter(|(_, channel)| {
                        channel.supports(layer)
                            && (**channel != GraphChannel::EffectParameter
                                || !graph_effect_parameters.is_empty())
                            && (!matches!(
                                **channel,
                                GraphChannel::EffectColorRed
                                    | GraphChannel::EffectColorGreen
                                    | GraphChannel::EffectColorBlue
                                    | GraphChannel::EffectColorAlpha
                            ) || !graph_effect_colors.is_empty())
                    })
                    .map(|(index, channel)| {
                        let label = if *channel == GraphChannel::EffectParameter {
                            selected_layer
                                .and_then(|layer| {
                                    graph_effect_parameter_label(
                                        layer,
                                        graph_effect.map(|effect| effect.id.as_str()),
                                        self.graph_effect_parameter_id.as_deref(),
                                    )
                                })
                                .unwrap_or_else(|| channel.label().to_owned())
                        } else if matches!(
                            *channel,
                            GraphChannel::EffectColorRed
                                | GraphChannel::EffectColorGreen
                                | GraphChannel::EffectColorBlue
                                | GraphChannel::EffectColorAlpha
                        ) {
                            graph_effect
                                .and_then(|effect| {
                                    graph_effect_colors
                                        .iter()
                                        .find(|(id, _)| {
                                            Some(id.as_str())
                                                == self.graph_effect_parameter_id.as_deref()
                                        })
                                        .map(|(_, label)| {
                                            format!(
                                                "{} · {} · {}",
                                                effect.name,
                                                label,
                                                graph_color_component_name(*channel)
                                            )
                                        })
                                })
                                .unwrap_or_else(|| channel.label().to_owned())
                        } else {
                            channel.label().to_owned()
                        };
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(index.to_string())),
                            ("label".to_owned(), Value::String(label)),
                            (
                                "selected".to_owned(),
                                Value::Bool(*channel == self.graph_channel),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                GraphChannel::ALL[..6]
                    .iter()
                    .enumerate()
                    .map(|(index, channel)| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String(index.to_string())),
                            (
                                "label".to_owned(),
                                Value::String(channel.label().to_owned()),
                            ),
                            (
                                "selected".to_owned(),
                                Value::Bool(*channel == self.graph_channel),
                            ),
                        ]))
                    })
                    .collect()
            });
        self.ui.set_binding(
            "has_graph_channels",
            Value::Bool(!graph_channels.is_empty()),
        )?;
        self.ui
            .set_binding("graph_channels", Value::List(graph_channels))?;
        let graph_parameters = if graph_color_channel {
            &graph_effect_colors
        } else {
            &graph_effect_parameters
        };
        let graph_parameter_items = if effect_value_channel {
            graph_parameters
                .iter()
                .map(|(id, label)| {
                    let label = if graph_color_channel {
                        format!(
                            "{} · {}",
                            label,
                            graph_color_component_name(self.graph_channel)
                        )
                    } else {
                        label.clone()
                    };
                    Value::Record(std::collections::BTreeMap::from([
                        ("id".to_owned(), Value::String(id.clone())),
                        ("label".to_owned(), Value::String(label)),
                        (
                            "selected".to_owned(),
                            Value::Bool(
                                Some(id.as_str()) == self.graph_effect_parameter_id.as_deref(),
                            ),
                        ),
                    ]))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.ui
            .set_binding("graph_parameter_items", Value::List(graph_parameter_items))?;
        let graph_interpolation = selected_layer
            .and_then(|layer| {
                graph_interpolation_label(
                    layer,
                    camera_for_layer(self.project_model.active_composition(), layer),
                    light_for_layer(self.project_model.active_composition(), layer),
                    self.selected_mask_id.as_deref(),
                    self.selected_effect_id.as_deref(),
                    self.graph_effect_parameter_id.as_deref(),
                    self.graph_channel,
                    self.current_frame,
                )
            })
            .unwrap_or_else(|| "キー未選択".to_owned());
        self.ui.set_binding(
            "graph_interpolation_label",
            Value::String(graph_interpolation),
        )?;
        self.ui.set_binding(
            "graph_has_keyframes",
            Value::Bool(selected_layer.is_some_and(|layer| {
                !graph_channel_keys(
                    layer,
                    camera_for_layer(self.project_model.active_composition(), layer),
                    light_for_layer(self.project_model.active_composition(), layer),
                    self.selected_mask_id.as_deref(),
                    self.selected_effect_id.as_deref(),
                    self.graph_effect_parameter_id.as_deref(),
                    self.graph_channel,
                )
                .is_empty()
            })),
        )?;
        self.ui.set_binding(
            "selected_time_remap",
            Value::Bool(selected_layer.is_some_and(|layer| layer.time_remap.is_some())),
        )?;
        self.ui.set_binding(
            "selected_is_video_layer",
            Value::Bool(selected_layer.is_some_and(|layer| {
                matches!(
                    &layer.layer_type,
                    kagari_vfx::core::timeline::LayerType::Video { .. }
                )
            })),
        )?;
        self.ui.set_binding(
            "selected_time_remap_source_frame",
            Value::String(
                selected_layer
                    .and_then(|layer| {
                        layer
                            .time_remap
                            .as_ref()
                            .map(|track| track.evaluate(self.current_frame))
                    })
                    .map(|frame| format!("{frame:.2}"))
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        )?;
        let posterize_time = selected_layer.and_then(|layer| {
            layer
                .posterize_time
                .map(|settings| (settings.enabled, settings.target_fps))
        });
        self.ui.set_binding(
            "video_posterize_present",
            Value::Bool(posterize_time.is_some()),
        )?;
        self.ui.set_binding(
            "video_posterize_enabled",
            Value::Bool(posterize_time.is_some_and(|settings| settings.0)),
        )?;
        self.ui.set_binding(
            "video_posterize_fps",
            Value::String(
                posterize_time
                    .map(|settings| format!("{:.2}", settings.1))
                    .unwrap_or_else(|| "12".to_owned()),
            ),
        )?;
        let layer_proxy = selected_layer.map(|layer| {
            use kagari_vfx::core::proxy::ProxyResolution;
            let resolution_label = match layer.proxy.resolution {
                ProxyResolution::Full => "Full",
                ProxyResolution::Half => "1/2",
                ProxyResolution::Quarter => "1/4",
                ProxyResolution::Eighth => "1/8",
            };
            let can_generate = matches!(
                &layer.layer_type,
                kagari_vfx::core::timeline::LayerType::Image { .. }
                    | kagari_vfx::core::timeline::LayerType::Video { .. }
            ) && layer.proxy.resolution != ProxyResolution::Full;
            let has_file = layer.proxy.proxy_path.as_deref().is_some_and(|path| {
                let path = std::path::Path::new(path);
                path.is_file() || path.is_dir()
            });
            (
                layer.proxy.enabled,
                resolution_label,
                can_generate,
                has_file,
                layer.proxy.proxy_path.is_some(),
            )
        });
        self.ui.set_binding(
            "layer_proxy_enabled",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.0)),
        )?;
        self.ui.set_binding(
            "layer_proxy_resolution_label",
            Value::String(
                layer_proxy
                    .map(|proxy| proxy.1.to_owned())
                    .unwrap_or_else(|| "Full".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "layer_proxy_can_generate",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.2)),
        )?;
        self.ui.set_binding(
            "layer_proxy_has_file",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.3)),
        )?;
        self.ui.set_binding(
            "layer_proxy_path_set",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.4)),
        )?;
        self.ui.set_binding(
            "proxy_generation_active",
            Value::Bool(self.proxy_generation.is_some()),
        )?;
        self.ui
            .set_binding("proxy_status", Value::String(self.proxy_status.clone()))?;
        self.ui
            .set_binding("proxy_progress", Value::Int(self.proxy_progress))?;
        let audio_volume_track = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => Some(volume),
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => Some(audio_volume),
            _ => None,
        });
        let audio_volume = audio_volume_track.map(|track| track.evaluate(self.current_frame));
        let audio_volume_keyframe_here = audio_volume_track.is_some_and(|track| {
            track
                .keyframes()
                .is_some_and(|keyframes| keyframes.iter().any(|key| key.frame == self.current_frame))
        });
        self.ui
            .set_binding("selected_is_audio", Value::Bool(audio_volume.is_some()))?;
        self.ui.set_binding(
            "audio_volume_keyframe_here",
            Value::Bool(audio_volume_keyframe_here),
        )?;
        self.ui.set_binding(
            "audio_volume",
            Value::String(
                audio_volume
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "0.0".to_owned()),
            ),
        )?;
        let transform_3d = selected_layer.filter(|layer| layer.is_3d).map(|layer| {
            (
                layer.transform_3d.position.evaluate(self.current_frame),
                layer.transform_3d.rotation.evaluate(self.current_frame),
                layer.transform_3d.scale.evaluate(self.current_frame),
            )
        });
        self.ui
            .set_binding("selected_is_3d", Value::Bool(transform_3d.is_some()))?;
        let selected_scene_ref = selected_layer.and_then(|layer| layer.scene_object.as_ref());
        let camera = selected_scene_ref.and_then(|scene_ref| match scene_ref {
            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => self
                .project_model
                .active_composition()
                .cameras
                .iter()
                .find(|camera| camera.id == *id),
            _ => None,
        });
        let light = selected_scene_ref.and_then(|scene_ref| match scene_ref {
            kagari_vfx::core::timeline::SceneObjectRef::Light { id } => self
                .project_model
                .active_composition()
                .lights
                .iter()
                .find(|light| light.id == *id),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_camera", Value::Bool(camera.is_some()))?;
        self.ui.set_binding(
            "selected_is_active_camera",
            Value::Bool(camera.is_some_and(|camera| camera.active)),
        )?;
        self.ui.set_binding(
            "selected_camera_fov",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.fov_at(self.current_frame)))
                    .unwrap_or(50.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_focus",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.focus_distance_at(self.current_frame)))
                    .unwrap_or(1000.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_aperture",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.aperture_at(self.current_frame)))
                    .unwrap_or(2.8),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_dof_enabled",
            Value::Bool(camera.is_some_and(|camera| camera.dof_enabled_at(self.current_frame))),
        )?;
        self.ui.set_binding(
            "selected_camera_dof_max_blur",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.dof_max_blur_at(self.current_frame)))
                    .unwrap_or(16.0),
            ),
        )?;
        self.ui
            .set_binding("selected_is_light", Value::Bool(light.is_some()))?;
        self.ui.set_binding(
            "selected_light_intensity",
            Value::Float(
                light
                    .map(|light| f64::from(light.intensity_at(self.current_frame)))
                    .unwrap_or(100.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_light_casts_shadows",
            Value::Bool(light.is_none_or(|light| light.casts_shadows)),
        )?;
        let (
            light_type_label,
            light_color,
            light_is_spot,
            cone_angle,
            cone_feather,
            falloff,
            shadow_darkness,
        ) = light
            .map(|light| {
                use kagari_vfx::core::timeline::LightType;
                let (label, cone_angle, cone_feather) = match &light.light_type {
                    LightType::Ambient => ("環境光", 90.0, 50.0),
                    LightType::Point => ("点光源", 90.0, 50.0),
                    LightType::Spot {
                        cone_angle_deg,
                        cone_feather_pct,
                    } => ("スポット", *cone_angle_deg, *cone_feather_pct),
                    LightType::Parallel => ("平行光", 90.0, 50.0),
                };
                let [r, g, b, _] = light.color;
                (
                    label,
                    format!(
                        "#{:02X}{:02X}{:02X}",
                        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (b.clamp(0.0, 1.0) * 255.0).round() as u8
                    ),
                    matches!(&light.light_type, LightType::Spot { .. }),
                    cone_angle,
                    cone_feather,
                    light.falloff,
                    light.shadow_darkness,
                )
            })
            .unwrap_or(("点光源", "#FFFFFF".to_owned(), false, 90.0, 50.0, 1.0, 60.0));
        self.ui.set_binding(
            "selected_light_type_label",
            Value::String(light_type_label.to_owned()),
        )?;
        self.ui
            .set_binding("selected_light_color", Value::String(light_color))?;
        self.ui
            .set_binding("selected_light_is_spot", Value::Bool(light_is_spot))?;
        self.ui.set_binding(
            "selected_light_cone_angle",
            Value::Float(f64::from(cone_angle)),
        )?;
        self.ui.set_binding(
            "selected_light_cone_feather",
            Value::Float(f64::from(cone_feather)),
        )?;
        self.ui
            .set_binding("selected_light_falloff", Value::Float(f64::from(falloff)))?;
        self.ui.set_binding(
            "selected_light_shadow_darkness",
            Value::Float(f64::from(shadow_darkness)),
        )?;
        for (name, value) in [
            ("position_3d_x", transform_3d.map(|values| values.0[0])),
            ("position_3d_y", transform_3d.map(|values| values.0[1])),
            ("position_3d_z", transform_3d.map(|values| values.0[2])),
            ("rotation_3d_x", transform_3d.map(|values| values.1[0])),
            ("rotation_3d_y", transform_3d.map(|values| values.1[1])),
            ("rotation_3d_z", transform_3d.map(|values| values.1[2])),
            ("scale_3d_x", transform_3d.map(|values| values.2[0])),
            ("scale_3d_y", transform_3d.map(|values| values.2[1])),
            ("scale_3d_z", transform_3d.map(|values| values.2[2])),
        ] {
            self.ui
                .set_binding(name, Value::Float(value.map(f64::from).unwrap_or(0.0)))?;
        }
        let material = selected_layer
            .filter(|layer| layer.is_3d)
            .map(|layer| &layer.material);
        for (name, value) in [
            ("material_ambient", material.map(|value| value.ambient)),
            ("material_diffuse", material.map(|value| value.diffuse)),
            ("material_specular", material.map(|value| value.specular)),
            (
                "material_specular_exponent",
                material.map(|value| value.specular_exponent),
            ),
            ("material_emission", material.map(|value| value.emission)),
            ("material_metalness", material.map(|value| value.metalness)),
            (
                "material_light_transmission",
                material.map(|value| value.light_transmission),
            ),
        ] {
            self.ui
                .set_binding(name, Value::Float(value.map(f64::from).unwrap_or_default()))?;
        }
        for (name, value) in [
            (
                "material_cast_shadows",
                material.is_some_and(|value| value.cast_shadows),
            ),
            (
                "material_accepts_shadows",
                material.is_some_and(|value| value.accepts_shadows),
            ),
            (
                "material_accepts_lights",
                material.is_some_and(|value| value.accepts_lights),
            ),
        ] {
            self.ui.set_binding(name, Value::Bool(value))?;
        }
        let video_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Video {
                source,
                frame_count,
                audio_wav,
                speed,
                ..
            } => Some((
                *speed,
                format!(
                    "{} f · {:.1}s · 音声{} · {}",
                    frame_count,
                    *frame_count as f32 / self.project_fps.max(1) as f32 / speed.max(0.05),
                    if audio_wav.is_some() {
                        "あり"
                    } else {
                        "なし"
                    },
                    media_file_status(source),
                ),
            )),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_video", Value::Bool(video_properties.is_some()))?;
        let selected_precomp = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::PreComp { .. } => Some(layer.is_collapsed),
            _ => None,
        });
        self.ui.set_binding(
            "selected_is_precomp",
            Value::Bool(selected_precomp.is_some()),
        )?;
        self.ui.set_binding(
            "precomp_collapse_transforms",
            Value::Bool(selected_precomp.unwrap_or(false)),
        )?;
        let precomp_source_summary = selected_layer
            .and_then(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::PreComp { comp_id } => self
                    .project_model
                    .active_composition()
                    .find_sub_comp(comp_id)
                    .map(|composition| {
                        format!(
                            "{} · {} × {} px · {} fps · {} 秒 · {} レイヤー",
                            composition.name,
                            composition.width,
                            composition.height,
                            composition.fps,
                            format_duration_seconds(
                                composition.duration_frames,
                                composition.fps,
                            ),
                            composition.layers.len(),
                        )
                    }),
                _ => None,
            })
            .unwrap_or_else(|| "参照先コンポジションが見つかりません。".to_owned());
        self.ui.set_binding(
            "precomp_source_summary",
            Value::String(precomp_source_summary),
        )?;
        self.ui.set_binding(
            "video_speed",
            Value::String(
                video_properties
                    .as_ref()
                    .map(|properties| format!("{:.2}", properties.0))
                    .unwrap_or_else(|| "1.0".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "video_info",
            Value::String(
                video_properties
                    .as_ref()
                    .map(|properties| properties.1.clone())
                    .unwrap_or_default(),
            ),
        )?;
        let image_info = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Image { path } => Some(media_file_status(path)),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_image", Value::Bool(image_info.is_some()))?;
        self.ui
            .set_binding("image_info", Value::String(image_info.unwrap_or_default()))?;
        let text_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text {
                text,
                font_size,
                color,
                font_family,
                tracking,
                leading,
                align,
                stroke_color,
                stroke_width,
                ..
            } => {
                let formatting = layer.text_formatting.as_ref();
                let effective_stroke_color = formatting
                    .and_then(|formatting| formatting.stroke_color)
                    .unwrap_or(*stroke_color);
                Some((
                    text.clone(),
                    font_size.to_string(),
                    rgba_to_hex(color),
                    formatting
                        .map(|formatting| formatting.font_family.clone())
                        .unwrap_or_else(|| font_family.clone()),
                    formatting
                        .map(|formatting| formatting.tracking)
                        .unwrap_or(*tracking)
                        .to_string(),
                    formatting
                        .map(|formatting| formatting.leading)
                        .unwrap_or(*leading)
                        .to_string(),
                    formatting
                        .map(|formatting| formatting.alignment as usize)
                        .unwrap_or(*align),
                    rgba_to_hex(&effective_stroke_color),
                    formatting
                        .map(|formatting| {
                            if formatting.stroke_color.is_some() {
                                formatting.stroke_width
                            } else {
                                0.0
                            }
                        })
                        .unwrap_or(*stroke_width)
                        .to_string(),
                ))
            }
            _ => None,
        });
        self.ui
            .set_binding("selected_is_text", Value::Bool(text_properties.is_some()))?;
        let active_text_family = text_properties.as_ref().map(|properties| properties.3.as_str());
        let font_query = self.text_font_search.trim().to_lowercase();
        let mut visible_font_families = self
            .text_font_families
            .iter()
            .filter(|family| {
                font_query.is_empty() || family.to_lowercase().contains(&font_query)
            })
            .take(if font_query.is_empty() { 16 } else { 40 })
            .collect::<Vec<_>>();
        if font_query.is_empty() {
            if let Some(selected_family) = active_text_family {
                if !visible_font_families.iter().any(|family| family.as_str() == selected_family) {
                    if let Some(family) = self.text_font_families.iter().find(|family| family.as_str() == selected_family) {
                        visible_font_families.push(family);
                    }
                }
            }
        }
        self.ui.set_binding(
            "text_font_families",
            Value::List(
                visible_font_families
                    .iter()
                    .map(|family| {
                        Value::Record(std::collections::BTreeMap::from([
                            ("id".to_owned(), Value::String((*family).clone())),
                            ("name".to_owned(), Value::String((*family).clone())),
                            (
                                "selected".to_owned(),
                                Value::Bool(active_text_family == Some(family.as_str())),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        )?;
        self.ui.set_binding(
            "has_text_font_families",
            Value::Bool(!visible_font_families.is_empty()),
        )?;
        self.ui.set_binding(
            "text_font_search",
            Value::String(self.text_font_search.clone()),
        )?;
        let text_formatting = selected_layer.and_then(|layer| layer.text_formatting.as_ref());
        for (name, value) in [
            ("text_faux_bold", text_formatting.is_some_and(|formatting| formatting.faux_bold)),
            ("text_faux_italic", text_formatting.is_some_and(|formatting| formatting.faux_italic)),
            ("text_all_caps", text_formatting.is_some_and(|formatting| formatting.all_caps)),
            ("text_small_caps", text_formatting.is_some_and(|formatting| formatting.small_caps)),
        ] {
            self.ui.set_binding(name, Value::Bool(value))?;
        }
        self.ui.set_binding(
            "text_box_width",
            Value::String(text_formatting.map(|formatting| formatting.box_width.to_string()).unwrap_or_else(|| "0".to_owned())),
        )?;
        self.ui.set_binding(
            "text_box_height",
            Value::String(text_formatting.map(|formatting| formatting.box_height.to_string()).unwrap_or_else(|| "0".to_owned())),
        )?;
        for (name, value) in [
            (
                "text_content",
                text_properties
                    .as_ref()
                    .map(|properties| properties.0.clone())
                    .unwrap_or_default(),
            ),
            (
                "text_font_size",
                text_properties
                    .as_ref()
                    .map(|properties| properties.1.clone())
                    .unwrap_or_else(|| "48".to_owned()),
            ),
            (
                "text_color",
                text_properties
                    .as_ref()
                    .map(|properties| properties.2.clone())
                    .unwrap_or_else(|| "#FFFFFF".to_owned()),
            ),
            (
                "text_font_family",
                text_properties
                    .as_ref()
                    .map(|properties| properties.3.clone())
                    .unwrap_or_else(|| "Inter".to_owned()),
            ),
            (
                "text_tracking",
                text_properties
                    .as_ref()
                    .map(|properties| properties.4.clone())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
            (
                "text_leading",
                text_properties
                    .as_ref()
                    .map(|properties| properties.5.clone())
                    .unwrap_or_else(|| "1.2".to_owned()),
            ),
            (
                "text_stroke_color",
                text_properties
                    .as_ref()
                    .map(|properties| properties.7.clone())
                    .unwrap_or_else(|| "#000000".to_owned()),
            ),
            (
                "text_stroke_width",
                text_properties
                    .as_ref()
                    .map(|properties| properties.8.clone())
                    .unwrap_or_else(|| "0".to_owned()),
            ),
        ] {
            self.ui.set_binding(name, Value::String(value))?;
        }
        for (name, alignment) in [
            ("text_align_left", 0),
            ("text_align_center", 1),
            ("text_align_right", 2),
        ] {
            self.ui.set_binding(
                name,
                Value::Bool(
                    text_properties
                        .as_ref()
                        .is_some_and(|properties| properties.6 == alignment),
                ),
            )?;
        }
        let text_animator = selected_layer.and_then(|layer| layer.text_animator.as_ref());
        self.ui.set_binding(
            "text_animator_present",
            Value::Bool(text_animator.is_some()),
        )?;
        self.ui.set_binding(
            "text_animator_enabled",
            Value::Bool(text_animator.is_some_and(|animator| animator.enabled)),
        )?;
        let shape_label = text_animator
            .map(|animator| match animator.selector.shape {
                kagari_vfx::core::text_animator::SelectorShape::Square => "Square",
                kagari_vfx::core::text_animator::SelectorShape::RampUp => "Ramp Up",
                kagari_vfx::core::text_animator::SelectorShape::RampDown => "Ramp Down",
                kagari_vfx::core::text_animator::SelectorShape::Triangle => "Triangle",
                kagari_vfx::core::text_animator::SelectorShape::Round => "Round",
                kagari_vfx::core::text_animator::SelectorShape::Smooth => "Smooth",
                kagari_vfx::core::text_animator::SelectorShape::Wobble => "Wobble",
                kagari_vfx::core::text_animator::SelectorShape::Random => "Random",
                kagari_vfx::core::text_animator::SelectorShape::Expression => "Expression",
            })
            .unwrap_or("Square");
        self.ui.set_binding(
            "text_animator_shape_label",
            Value::String(shape_label.to_owned()),
        )?;
        self.ui.set_binding(
            "text_animator_random_order",
            Value::Bool(text_animator.is_some_and(|animator| animator.selector.random_order)),
        )?;
        self.ui.set_binding(
            "text_animator_expression",
            Value::String(
                text_animator
                    .and_then(|animator| animator.selector.expression.clone())
                    .unwrap_or_default(),
            ),
        )?;
        let text_animator_parameters = text_animator
            .filter(|animator| animator.enabled)
            .map(|animator| {
                [
                    ("selector.start", "範囲開始", animator.selector.start, "%"),
                    ("selector.end", "範囲終了", animator.selector.end, "%"),
                    (
                        "selector.offset",
                        "範囲オフセット",
                        animator.selector.offset,
                        "%",
                    ),
                    (
                        "selector.ease_high",
                        "上側イージング",
                        animator.selector.ease_high,
                        "%",
                    ),
                    (
                        "selector.ease_low",
                        "下側イージング",
                        animator.selector.ease_low,
                        "%",
                    ),
                    (
                        "selector.char_start",
                        "文字番号開始",
                        animator.selector.char_start as f32,
                        "(-1 = 全て)",
                    ),
                    (
                        "selector.char_end",
                        "文字番号終了",
                        animator.selector.char_end as f32,
                        "(-1 = 全て)",
                    ),
                    (
                        "position_offset.x",
                        "位置 X",
                        animator.position_offset[0],
                        "px",
                    ),
                    (
                        "position_offset.y",
                        "位置 Y",
                        animator.position_offset[1],
                        "px",
                    ),
                    ("scale.x", "倍率 X", animator.scale[0], "×"),
                    ("scale.y", "倍率 Y", animator.scale[1], "×"),
                    ("opacity", "不透明度変化", animator.opacity, "%"),
                    ("tracking", "文字間隔変化", animator.tracking, "px"),
                    ("rotation", "回転", animator.rotation, "°"),
                    ("blur_amount", "ぼかし", animator.blur_amount, "px"),
                ]
                .into_iter()
                .map(|(id, label, value, unit)| {
                    Value::Record(std::collections::BTreeMap::from([
                        ("id".to_owned(), Value::String(id.to_owned())),
                        ("label".to_owned(), Value::String(label.to_owned())),
                        ("value".to_owned(), Value::String(format!("{value:.2}"))),
                        ("unit".to_owned(), Value::String(unit.to_owned())),
                    ]))
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.ui.set_binding(
            "has_text_animator_parameters",
            Value::Bool(!text_animator_parameters.is_empty()),
        )?;
        self.ui.set_binding(
            "text_animator_parameters",
            Value::List(text_animator_parameters),
        )?;

        self.sync_shape_bindings()?;
        self.sync_effect_bindings()?;
        self.sync_recent_projects_bindings()?;
        Ok(())
    }


}
