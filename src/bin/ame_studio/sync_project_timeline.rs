use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_project_timeline_bindings(
        &mut self,
    ) -> Result<Vec<kagari_vfx::core::timeline::Layer>, Box<dyn std::error::Error>> {
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
                    format_duration_seconds(composition.duration_frames, composition.fps,),
                    composition.layers.len(),
                )
            })
            .unwrap_or_else(|| "コンポジションなし".to_owned());
        self.ui
            .set_binding("project_summary", Value::String(project_summary))?;
        let active_composition = self.project_model.active_composition();
        let duration_text =
            format_duration_seconds(active_composition.duration_frames, active_composition.fps);
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
            Value::Bool(
                self.video_import
                    .as_ref()
                    .is_some_and(|job| job.cancel_supported),
            ),
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
        self.ui
            .set_binding("preview_badge", Value::String("CPU プレビュー".to_owned()))?;
        self.ui.set_binding(
            "preview_worker_failed",
            Value::Bool(self.viewport_preview_worker_failed),
        )?;
        self.ui.set_binding(
            "preview_fallback_reason",
            Value::String("現行のプレビュー生成はCPUレンダー経路を使用します".to_owned()),
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
            "video_export_quality",
            Value::Float(f64::from(self.video_export_quality)),
        )?;
        self.ui.set_binding(
            "video_export_audio_enabled",
            Value::Bool(self.video_export_audio_enabled),
        )?;
        self.ui.set_binding(
            "video_export_codec",
            Value::String(
                match self.video_export_codec_idx % 4 {
                    0 => "H.264 · MP4",
                    1 => "H.265 · MP4",
                    2 => "ProRes 422 · MOV",
                    _ => "ProRes 4444 · MOV",
                }
                .to_owned(),
            ),
        )?;
        self.ui.set_binding(
            "video_export_quality_supported",
            Value::Bool(self.video_export_codec_idx % 4 < 2),
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
                        self.project_model
                            .active_composition()
                            .start_timecode_frames,
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
        let (work_area_start, work_area_end) = self
            .playback_range
            .map(|(start, end)| {
                (
                    start.min(last_frame),
                    end.min(last_frame).max(start.min(last_frame)),
                )
            })
            .unwrap_or((0, last_frame));
        self.ui.set_binding(
            "work_area_start_frame",
            Value::String(work_area_start.to_string()),
        )?;
        self.ui.set_binding(
            "work_area_end_frame",
            Value::String(work_area_end.to_string()),
        )?;
        Ok(visible_timeline_layers)
    }
}
