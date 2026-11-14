use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_scene_asset_bindings(
        &mut self,
        visible_timeline_layers: Vec<kagari_vfx::core::timeline::Layer>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        let needs_scene_children = self.selected_layer_id.is_some()
            || (query.is_empty() && !self.collapsed_scene_layers.is_empty());
        let scene_children_by_parent = if needs_scene_children {
            scene_layer_children_by_parent(composition_layers)
        } else {
            std::collections::HashMap::new()
        };
        let collapsed_hidden_layers = if query.is_empty() && !self.collapsed_scene_layers.is_empty()
        {
            scene_layer_hidden_ids(&scene_children_by_parent, &self.collapsed_scene_layers)
        } else {
            std::collections::HashSet::new()
        };
        let scene_layers = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .filter(|layer| scene_search_visible.contains(&layer.id))
            .filter(|layer| !collapsed_hidden_layers.contains(layer.id.as_str()))
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
                            "　".repeat(
                                scene_layer_depths
                                    .get(layer.id.as_str())
                                    .copied()
                                    .unwrap_or(0),
                            ),
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
                    (
                        "has_audio".to_owned(),
                        Value::Bool(matches!(
                            &layer.layer_type,
                            kagari_vfx::core::timeline::LayerType::Audio { .. }
                                | kagari_vfx::core::timeline::LayerType::Video {
                                    audio_wav: Some(_),
                                    ..
                                }
                        )),
                    ),
                    ("audio_muted".to_owned(), Value::Bool(layer.audio_muted)),
                    (
                        "blend_mode".to_owned(),
                        Value::String(blend_mode_label(layer.blend_mode).to_owned()),
                    ),
                    (
                        "track_matte".to_owned(),
                        Value::String(match layer.track_matte {
                            kagari_vfx::core::timeline::TrackMatteMode::None => "なし".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte => {
                                "α".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatteInverted => {
                                "α反転".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatte => {
                                "輝度".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatteInverted => {
                                "輝度反転".to_owned()
                            }
                        }),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui.set_binding(
            "has_scene_layers",
            Value::Bool(!composition_layers.is_empty()),
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
        let mut align_selected_count = composition_layers
            .iter()
            .filter(|layer| !layer.locked && self.align_selected_layer_ids.contains(&layer.id))
            .count();
        if self.align_selected_layer_ids.is_empty()
            && self.selected_layer_id.as_deref().is_some_and(|id| {
                composition_layers
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
        self.ui
            .set_binding("can_align_layers", Value::Bool(align_selected_count >= 1))?;
        self.ui.set_binding(
            "can_distribute_layers",
            Value::Bool(align_selected_count >= 3),
        )?;

        let all_layers = composition_layers;
        let current_parent_id = self
            .selected_layer_id
            .as_deref()
            .and_then(|id| scene_layer_by_id.get(id).copied())
            .and_then(|layer| layer.parent_id.as_deref());
        let parent_candidates = self
            .selected_layer_id
            .as_deref()
            .map(|selected_id| {
                let mut descendants = std::collections::HashSet::new();
                let mut pending = vec![selected_id];
                while let Some(parent_id) = pending.pop() {
                    if let Some(children) = scene_children_by_parent.get(parent_id) {
                        for child_id in children {
                            if descendants.insert(*child_id) {
                                pending.push(*child_id);
                            }
                        }
                    }
                }
                all_layers
                    .iter()
                    .filter_map(|candidate| {
                        if candidate.id == selected_id
                            || descendants.contains(candidate.id.as_str())
                        {
                            return None;
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
            .and_then(|id| scene_layer_by_id.get(id).copied())
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| "なし".to_owned());
        self.ui.set_binding(
            "selected_layer_parent_name",
            Value::String(current_parent_name),
        )?;

        let audio_tracks: Vec<Value> = if self.workspace == Workspace::Audio {
            self.project_model
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
                            Value::Bool(
                                self.selected_layer_id.as_deref() == Some(layer.id.as_str()),
                            ),
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
                .collect()
        } else {
            Vec::new()
        };
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
                            "　".repeat(
                                scene_layer_depths
                                    .get(layer.id.as_str())
                                    .copied()
                                    .unwrap_or(0),
                            ),
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
                    (
                        "has_audio".to_owned(),
                        Value::Bool(matches!(
                            &layer.layer_type,
                            kagari_vfx::core::timeline::LayerType::Audio { .. }
                                | kagari_vfx::core::timeline::LayerType::Video {
                                    audio_wav: Some(_),
                                    ..
                                }
                        )),
                    ),
                    ("audio_muted".to_owned(), Value::Bool(layer.audio_muted)),
                    (
                        "blend_mode".to_owned(),
                        Value::String(blend_mode_label(layer.blend_mode).to_owned()),
                    ),
                    (
                        "track_matte".to_owned(),
                        Value::String(match layer.track_matte {
                            kagari_vfx::core::timeline::TrackMatteMode::None => "なし".to_owned(),
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte => {
                                "α".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::AlphaMatteInverted => {
                                "α反転".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatte => {
                                "輝度".to_owned()
                            }
                            kagari_vfx::core::timeline::TrackMatteMode::LumaMatteInverted => {
                                "輝度反転".to_owned()
                            }
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
                            marker
                                .frame
                                .saturating_add(composition.start_timecode_frames),
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
                composition
                    .markers
                    .iter()
                    .any(|marker| marker.frame == frame)
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
            Value::String(
                self.selected_marker_frame
                    .map(|frame| frame.to_string())
                    .unwrap_or_default(),
            ),
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
                            marker
                                .frame
                                .saturating_add(composition.start_timecode_frames),
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
                selected_layer_markers
                    .is_some_and(|layer| layer.markers.iter().any(|marker| marker.frame == frame))
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
            Value::String(
                self.selected_layer_marker_frame
                    .map(|frame| frame.to_string())
                    .unwrap_or_default(),
            ),
        )?;

        let show_scene_assets = !matches!(
            self.workspace,
            Workspace::Composition
                | Workspace::CameraLight
                | Workspace::Color
                | Workspace::LayerStyle
                | Workspace::Text
        );
        let scene_assets: Vec<Value> = if show_scene_assets {
            self.project_model
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
                            Value::Bool(
                                self.selected_layer_id.as_deref() == Some(layer.id.as_str()),
                            ),
                        ),
                        ("locked".to_owned(), Value::Bool(layer.locked)),
                        ("missing".to_owned(), Value::Bool(missing)),
                    ])))
                })
                .collect()
        } else {
            Vec::new()
        };
        self.ui
            .set_binding("has_scene_assets", Value::Bool(!scene_assets.is_empty()))?;
        self.ui
            .set_binding("scene_assets", Value::List(scene_assets))?;
        self.sync_project_asset_bindings()?;
        Ok(())
    }
}
