use super::*;

impl AfterEffectsStudio {
    pub(crate) fn timeline_window(&self, duration_frames: u32) -> (f64, f64) {
        let total = f64::from(duration_frames.max(1));
        let visible = (total / self.timeline_zoom.clamp(1.0, 64.0)).clamp(1.0, total);
        let start = self
            .timeline_start_frame
            .clamp(0.0, (total - visible).max(0.0));
        (start, visible)
    }

    pub(crate) fn adjust_composition_zoom(&mut self, multiplier: f64) {
        let current = self
            .zoom
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or(100.0);
        let next = (current * multiplier).round().clamp(10.0, 800.0) as u32;
        self.zoom = format!("{next}%");
        self.zoom_manually_adjusted = true;
    }

    pub(crate) fn set_timeline_zoom(&mut self, multiplier: f64) {
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let (old_start, old_span) = self.timeline_window(duration);
        let anchor_fraction =
            ((f64::from(self.current_frame) - old_start) / old_span).clamp(0.0, 1.0);
        self.timeline_zoom = (self.timeline_zoom * multiplier).clamp(1.0, 64.0);
        let (_, new_span) = self.timeline_window(duration);
        let total = f64::from(duration);
        self.timeline_start_frame = (f64::from(self.current_frame) - anchor_fraction * new_span)
            .clamp(0.0, (total - new_span).max(0.0));
    }

    pub(crate) fn pan_timeline(&mut self, direction: f64) {
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let (_, visible) = self.timeline_window(duration);
        let total = f64::from(duration);
        self.timeline_start_frame = (self.timeline_start_frame + direction * visible * 0.5)
            .clamp(0.0, (total - visible).max(0.0));
    }

    pub(crate) fn fit_timeline(&mut self) {
        self.timeline_zoom = 1.0;
        self.timeline_start_frame = 0.0;
    }

    pub(crate) fn timeline_layers(&self) -> Vec<kagari_vfx::core::timeline::Layer> {
        use kagari_vfx::core::timeline::LayerType;
        let composition = self.project_model.active_composition();
        let marker_view = self.timeline_view == TimelineView::Markers;
        let mut hidden_layers = std::collections::HashSet::new();
        if !marker_view && !self.collapsed_scene_layers.is_empty() {
            let mut children_by_parent: std::collections::HashMap<&str, Vec<&str>> =
                std::collections::HashMap::with_capacity(composition.layers.len());
            for layer in &composition.layers {
                if let Some(parent_id) = layer.parent_id.as_deref() {
                    children_by_parent
                        .entry(parent_id)
                        .or_default()
                        .push(layer.id.as_str());
                }
            }
            let mut pending: Vec<&str> = composition
                .layers
                .iter()
                .filter(|layer| {
                    layer
                        .parent_id
                        .as_deref()
                        .is_some_and(|parent_id| self.collapsed_scene_layers.contains(parent_id))
                })
                .map(|layer| layer.id.as_str())
                .collect();
            while let Some(layer_id) = pending.pop() {
                if !hidden_layers.insert(layer_id) {
                    continue;
                }
                if let Some(children) = children_by_parent.get(layer_id) {
                    pending.extend(children.iter().copied());
                }
            }
        }

        composition
            .layers
            .iter()
            .filter(|layer| {
                !hidden_layers.contains(layer.id.as_str())
                    && (marker_view || !self.hide_shy_layers || !layer.is_shy)
                    && match self.timeline_view {
                        TimelineView::All => true,
                        TimelineView::Animation => {
                            !layer_keyframe_frames_for_composition(composition, layer).is_empty()
                        }
                        TimelineView::CurrentKeyframes => {
                            layer_keyframe_frames_for_composition(composition, layer)
                                .contains(&self.current_frame.min(composition.duration_frames))
                        }
                        TimelineView::Audio => match &layer.layer_type {
                            LayerType::Audio { .. } => true,
                            LayerType::Video { audio_wav, .. } => audio_wav.is_some(),
                            _ => false,
                        },
                        TimelineView::Media => matches!(
                            &layer.layer_type,
                            LayerType::Image { .. }
                                | LayerType::Video { .. }
                                | LayerType::Model3D { .. }
                                | LayerType::PreComp { .. }
                        ),
                        TimelineView::Text => matches!(&layer.layer_type, LayerType::Text { .. }),
                        TimelineView::Shapes => matches!(
                            &layer.layer_type,
                            LayerType::Shape { .. }
                                | LayerType::Solid { .. }
                                | LayerType::Particle { .. }
                        ),
                        TimelineView::ThreeD => layer.is_3d,
                        TimelineView::Cameras => matches!(
                            layer.scene_object.as_ref(),
                            Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { .. })
                        ),
                        TimelineView::Lights => matches!(
                            layer.scene_object.as_ref(),
                            Some(kagari_vfx::core::timeline::SceneObjectRef::Light { .. })
                        ),
                        TimelineView::Markers => true,
                        TimelineView::Graph | TimelineView::Flowchart => false,
                    }
            })
            .cloned()
            .collect()
    }

    pub(crate) fn update_timeline_clip_drag(&mut self, pointer_x: f64) {
        if !pointer_x.is_finite() || self.timeline_width <= 0.0 {
            return;
        }
        let Some(drag) = self.timeline_clip_drag.clone() else {
            return;
        };
        self.timeline_snap_frame = None;
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let (_, visible_frames) = self.timeline_window(duration);
        let delta =
            ((pointer_x - drag.start_x) * visible_frames / self.timeline_width).round() as i64;
        if matches!(drag.mode, TimelineClipDragMode::Keyframe) {
            let (Some(source), Some(current)) = (drag.keyframe_source, drag.keyframe_current)
            else {
                return;
            };
            let minimum = drag.initial_in.min(duration.saturating_sub(1));
            let maximum = drag
                .initial_out
                .min(duration)
                .max(minimum.saturating_add(1));
            let mut target =
                (i64::from(source) + delta).clamp(i64::from(minimum), i64::from(maximum)) as u32;
            if self.timeline_snap_enabled && self.timeline_width > 0.0 {
                let snap_distance =
                    (8.0 * visible_frames / self.timeline_width).ceil().max(1.0) as u32;
                let composition = self.project_model.active_composition();
                let mut candidates = composition.markers.iter().map(|marker| marker.frame);
                let mut snap_points = candidates.by_ref().collect::<Vec<_>>();
                snap_points.push(self.current_frame);
                for layer in composition
                    .layers
                    .iter()
                    .filter(|layer| layer.id != drag.layer_id)
                {
                    snap_points.push(layer.in_frame);
                    snap_points.push(layer.out_frame);
                    snap_points.extend(layer_keyframe_frames_for_composition(composition, layer));
                }
                for camera in &composition.cameras {
                    snap_points.extend(camera_keyframe_frames(camera));
                }
                if let Some((snap_frame, _)) = snap_points
                    .into_iter()
                    .filter(|candidate| *candidate >= minimum && *candidate <= maximum)
                    .map(|candidate| (candidate, target.abs_diff(candidate)))
                    .filter(|(_, distance)| *distance <= snap_distance)
                    .min_by_key(|(_, distance)| *distance)
                {
                    target = snap_frame;
                    self.timeline_snap_frame = Some(snap_frame);
                }
            }
            if target == current {
                return;
            }
            let composition = self.project_model.active_composition_mut();
            let camera_id = composition
                .layers
                .iter()
                .find(|layer| layer.id == drag.layer_id && !layer.locked)
                .and_then(|layer| match layer.scene_object.as_ref() {
                    Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { id }) => {
                        Some(id.clone())
                    }
                    _ => None,
                });
            let (layers, cameras) = (&mut composition.layers, &mut composition.cameras);
            let mut changed = layers
                .iter_mut()
                .find(|layer| layer.id == drag.layer_id && !layer.locked)
                .is_some_and(|layer| move_layer_keyframes_at_frame(layer, current, target));
            if let Some(camera_id) = camera_id {
                if let Some(camera) = cameras.iter_mut().find(|camera| camera.id == camera_id) {
                    changed |= move_camera_keyframes_at_frame(camera, current, target);
                }
            }
            if changed {
                if let Some(active_drag) = self.timeline_clip_drag.as_mut() {
                    active_drag.keyframe_current = Some(target);
                }
                self.current_frame = target;
                self.current_time = format_timecode(target, self.project_fps);
                self.sync_selected_layer_controls();
                self.mark_project_changed();
                self.sync_bindings_logged();
            }
            return;
        }
        let original_in = drag.initial_in.min(duration.saturating_sub(1));
        let original_out = drag
            .initial_out
            .min(duration)
            .max(original_in.saturating_add(1));
        let clip_length = original_out.saturating_sub(original_in).max(1);
        let (mut next_in, mut next_out) = match drag.mode {
            TimelineClipDragMode::Move => {
                let max_start = duration.saturating_sub(clip_length);
                let next_in =
                    (i64::from(original_in) + delta).clamp(0, i64::from(max_start)) as u32;
                (next_in, next_in.saturating_add(clip_length).min(duration))
            }
            TimelineClipDragMode::TrimStart => {
                let next_in = (i64::from(original_in) + delta)
                    .clamp(0, i64::from(original_out.saturating_sub(1)))
                    as u32;
                (next_in, original_out)
            }
            TimelineClipDragMode::TrimEnd => {
                let next_out = (i64::from(original_out) + delta).clamp(
                    i64::from(original_in.saturating_add(1)),
                    i64::from(duration),
                ) as u32;
                (original_in, next_out)
            }
            TimelineClipDragMode::Keyframe => return,
        };
        if self.timeline_snap_enabled && self.timeline_width > 0.0 {
            let snap_distance = (8.0 * visible_frames / self.timeline_width).ceil().max(1.0) as u32;
            let composition = self.project_model.active_composition();
            let mut candidates = composition
                .markers
                .iter()
                .map(|marker| marker.frame.min(duration))
                .chain(std::iter::once(self.current_frame.min(duration)))
                .collect::<Vec<_>>();
            for layer in composition
                .layers
                .iter()
                .filter(|layer| layer.id != drag.layer_id)
            {
                candidates.extend([layer.in_frame.min(duration), layer.out_frame.min(duration)]);
                candidates.extend(
                    layer_keyframe_frames_for_composition(composition, layer)
                        .into_iter()
                        .map(|frame| frame.min(duration)),
                );
            }
            let nearest_snap = |frame: u32| {
                candidates
                    .iter()
                    .map(|candidate| (*candidate, frame.abs_diff(*candidate)))
                    .filter(|(_, distance)| *distance <= snap_distance)
                    .min_by_key(|(_, distance)| *distance)
                    .map(|(candidate, _)| candidate)
            };
            match drag.mode {
                TimelineClipDragMode::Move => {
                    let start_snap =
                        nearest_snap(next_in).map(|target| (target, next_in.abs_diff(target)));
                    let end_snap =
                        nearest_snap(next_out).map(|target| (target, next_out.abs_diff(target)));
                    let (snap_delta, snap_frame) = match (start_snap, end_snap) {
                        (Some((_target, distance)), Some((end_target, end_distance)))
                            if end_distance < distance =>
                        {
                            (
                                Some(i64::from(end_target) - i64::from(next_out)),
                                Some(end_target),
                            )
                        }
                        (Some((target, _)), _) => {
                            (Some(i64::from(target) - i64::from(next_in)), Some(target))
                        }
                        (_, Some((target, _))) => {
                            (Some(i64::from(target) - i64::from(next_out)), Some(target))
                        }
                        _ => (None, None),
                    };
                    if let Some(delta) = snap_delta {
                        self.timeline_snap_frame = snap_frame;
                        let clip_length = next_out.saturating_sub(next_in).max(1);
                        next_in = (i64::from(next_in) + delta)
                            .clamp(0, i64::from(duration.saturating_sub(clip_length)))
                            as u32;
                        next_out = next_in.saturating_add(clip_length).min(duration);
                    }
                }
                TimelineClipDragMode::TrimStart => {
                    if let Some(target) = nearest_snap(next_in) {
                        self.timeline_snap_frame = Some(target);
                        next_in = target.min(next_out.saturating_sub(1));
                    }
                }
                TimelineClipDragMode::TrimEnd => {
                    if let Some(target) = nearest_snap(next_out) {
                        self.timeline_snap_frame = Some(target);
                        next_out = target.clamp(next_in.saturating_add(1), duration);
                    }
                }
                TimelineClipDragMode::Keyframe => {}
            }
        }
        let composition = self.project_model.active_composition_mut();
        let layer_index = composition
            .layers
            .iter()
            .position(|layer| layer.id == drag.layer_id && !layer.locked);
        let scene_object =
            layer_index.and_then(|index| composition.layers[index].scene_object.clone());
        let changed = layer_index
            .map(|index| {
                let layer = &mut composition.layers[index];
                if matches!(drag.mode, TimelineClipDragMode::Move) {
                    layer.move_timeline_range(next_in, next_out)
                } else if layer.in_frame == next_in && layer.out_frame == next_out {
                    false
                } else {
                    layer.in_frame = next_in;
                    layer.out_frame = next_out;
                    true
                }
            })
            .unwrap_or(false);
        if changed && matches!(drag.mode, TimelineClipDragMode::Move) {
            if let Some(scene_object) = scene_object {
                let shift = i64::from(next_in) - i64::from(drag.initial_in);
                let ref_count = composition
                    .layers
                    .iter()
                    .filter(|layer| layer.scene_object.as_ref() == Some(&scene_object))
                    .count();
                // A scene object can be shared by multiple layers. Its own tracks
                // belong to the object, so only move them with an unambiguous owner.
                if ref_count == 1 {
                    match scene_object {
                        kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => {
                            if let Some(camera) =
                                composition.cameras.iter_mut().find(|c| c.id == id)
                            {
                                for track in [
                                    &mut camera.fov_animation,
                                    &mut camera.focus_distance_animation,
                                    &mut camera.aperture_animation,
                                    &mut camera.dof_max_blur_animation,
                                    &mut camera.dof_enabled_animation,
                                ] {
                                    shift_optional_track_frames(track, shift);
                                }
                            }
                        }
                        kagari_vfx::core::timeline::SceneObjectRef::Light { id } => {
                            if let Some(light) = composition.lights.iter_mut().find(|l| l.id == id)
                            {
                                shift_optional_track_frames(&mut light.intensity_animation, shift);
                            }
                        }
                    }
                }
            }
        }
        if changed {
            self.mark_project_changed();
            self.sync_bindings_logged();
        }
    }
}

fn shift_optional_track_frames<T: Clone>(
    track: &mut Option<kagari_vfx::core::property::Animatable<T>>,
    shift: i64,
) {
    if let Some(keyframes) = track.as_mut().and_then(|track| track.keyframes_mut()) {
        for keyframe in keyframes.iter_mut() {
            keyframe.frame =
                (i64::from(keyframe.frame) + shift).clamp(0, i64::from(u32::MAX)) as u32;
        }
        keyframes.sort_by_key(|keyframe| keyframe.frame);
    }
}
