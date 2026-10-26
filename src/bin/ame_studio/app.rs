use super::*;

impl App for AfterEffectsStudio {
    fn asset_store(&self) -> AssetStore {
        self.assets.clone()
    }

    fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    fn on_canvas_pointer_event(&mut self, canvas_id: &str, event: CanvasPointerEvent) -> bool {
        if canvas_id == "kagari.graph" {
            let duration = self
                .project_model
                .active_composition()
                .duration_frames
                .max(1);
            let (timeline_start, visible_frames) = self.timeline_window(duration);
            let plot_left = 14.0;
            let plot_right = (self.timeline_width - 14.0).max(plot_left + 1.0);
            let plot_top = 12.0;
            let plot_bottom = (self.graph_canvas_height - 12.0).max(plot_top + 1.0);
            match event.phase {
                CanvasPointerPhase::Down => {
                    self.timeline_snap_frame = None;
                    let Some(layer_id) = self.selected_layer_id.as_deref() else {
                        return true;
                    };
                    let (layer_locked, keys) = {
                        let composition = self.project_model.active_composition();
                        let Some(layer) = composition.layers.iter().find(|layer| layer.id == layer_id)
                        else {
                            return true;
                        };
                        let camera = camera_for_layer(composition, layer);
                        let light = light_for_layer(composition, layer);
                        let keys = graph_channel_keys(
                            layer,
                            camera,
                            light,
                            self.selected_mask_id.as_deref(),
                            self.selected_effect_id.as_deref(),
                            self.graph_effect_parameter_id.as_deref(),
                            self.graph_channel,
                        );
                        (layer.locked, keys)
                    };
                    let (min_value, max_value) = graph_value_range(&keys);
                    let plot_height = plot_bottom - plot_top;
                    let nearest = keys.iter().min_by(|a, b| {
                        let distance = |key: &(u32, f32)| {
                            let px = plot_left
                                + (f64::from(key.0) - timeline_start) / visible_frames
                                    * (plot_right - plot_left);
                            let py = plot_bottom
                                - (f64::from(key.1) - f64::from(min_value))
                                    / f64::from((max_value - min_value).max(f32::EPSILON))
                                    * plot_height;
                            (event.x - px).hypot(event.y - py)
                        };
                        distance(a).total_cmp(&distance(b))
                    });
                    if let Some((frame, _)) = nearest.filter(|key| {
                        let px = plot_left
                            + (f64::from(key.0) - timeline_start) / visible_frames
                                * (plot_right - plot_left);
                        let py = plot_bottom
                            - (f64::from(key.1) - f64::from(min_value))
                                / f64::from((max_value - min_value).max(f32::EPSILON))
                                * plot_height;
                        (event.x - px).hypot(event.y - py) <= 12.0
                    }) {
                        self.current_frame = (*frame).min(duration.saturating_sub(1));
                        self.current_time = format_timecode(self.current_frame, self.project_fps);

                        // If right-clicked on keyframe, cycle interpolation (Linear -> Bezier Easing -> Hold)
                        if event.button == Some(ame_ui::event::PointerButton::Secondary) && !layer_locked {
                            let comp = self.project_model.active_composition_mut();
                            if let Some((target_layer, target_camera, target_light)) = graph_targets_mut(comp, layer_id) {
                                if cycle_graph_interpolation(
                                    target_layer,
                                    target_camera,
                                    target_light,
                                    self.selected_mask_id.as_deref(),
                                    self.selected_effect_id.as_deref(),
                                    self.graph_effect_parameter_id.as_deref(),
                                    self.graph_channel,
                                    *frame,
                                ) {
                                    self.mark_project_changed();
                                    self.sync_selected_layer_controls();
                                    self.sync_bindings_logged();
                                    return true;
                                }
                            }
                        }

                        let mut snap_points = Vec::new();
                        let composition = self.project_model.active_composition();
                        snap_points.extend(composition.markers.iter().map(|marker| marker.frame.min(duration.saturating_sub(1))));
                        snap_points.push(self.current_frame.min(duration.saturating_sub(1)));
                        for l in &composition.layers {
                            if l.id != layer_id {
                                snap_points.push(l.in_frame.min(duration.saturating_sub(1)));
                                snap_points.push(l.out_frame.min(duration.saturating_sub(1)));
                            }
                            snap_points.extend(layer_keyframe_frames_for_composition(composition, l));
                        }
                        for camera in &composition.cameras {
                            snap_points.extend(camera_keyframe_frames(camera));
                        }
                        snap_points.sort_unstable();
                        snap_points.dedup();
                        self.graph_key_drag = (!layer_locked).then(|| GraphKeyDrag {
                            layer_id: layer_id.to_owned(),
                            channel: self.graph_channel,
                            current_frame: *frame,
                            min_value,
                            max_value,
                            mask_id: self.selected_mask_id.clone(),
                            effect_id: self.selected_effect_id.clone(),
                            effect_parameter_id: self.graph_effect_parameter_id.clone(),
                            snap_points,
                        });
                    } else {
                        let fraction =
                            ((event.x - plot_left) / (plot_right - plot_left)).clamp(0.0, 1.0);
                        self.current_frame = ((timeline_start + fraction * visible_frames).round()
                            as u32)
                            .min(duration.saturating_sub(1));
                        self.current_time = format_timecode(self.current_frame, self.project_fps);
                    }
                    self.sync_selected_layer_controls();
                    self.sync_bindings_logged();
                    true
                }
                CanvasPointerPhase::Moved if self.graph_key_drag.is_some() => {
                    let drag = self.graph_key_drag.clone().unwrap();
                    let plot_height = plot_bottom - plot_top;
                    let value_fraction = ((plot_bottom - event.y) / plot_height).clamp(0.0, 1.0);
                    let value =
                        drag.min_value + (drag.max_value - drag.min_value) * value_fraction as f32;
                    let time_fraction =
                        ((event.x - plot_left) / (plot_right - plot_left)).clamp(0.0, 1.0);
                    let mut target_frame = ((timeline_start + time_fraction * visible_frames).round()
                        as u32)
                        .min(duration.saturating_sub(1));
                    self.timeline_snap_frame = None;
                    if self.timeline_snap_enabled {
                        let snap_distance =
                            (8.0 * visible_frames / self.timeline_width.max(1.0)).ceil().max(1.0)
                                as u32;
                        if let Some((snap_frame, _)) = drag
                            .snap_points
                            .iter()
                            .copied()
                            .filter(|candidate| *candidate != drag.current_frame)
                            .map(|candidate| (candidate, target_frame.abs_diff(candidate)))
                            .filter(|(_, distance)| *distance <= snap_distance)
                            .min_by_key(|(_, distance)| *distance)
                        {
                            target_frame = snap_frame;
                            self.timeline_snap_frame = Some(snap_frame);
                        }
                    }
                    let target_is_unlocked = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == drag.layer_id)
                        .is_some_and(|layer| !layer.locked);
                    if target_is_unlocked {
                        if let Some((layer, camera, light)) =
                            graph_targets_mut(
                                self.project_model.active_composition_mut(),
                                &drag.layer_id,
                            )
                        {
                            if update_graph_keyframe(
                                layer,
                                camera,
                                light,
                                drag.mask_id.as_deref(),
                                drag.effect_id.as_deref(),
                                drag.effect_parameter_id.as_deref(),
                                drag.channel,
                                drag.current_frame,
                                target_frame,
                                value,
                            ) {
                                self.graph_key_drag = Some(GraphKeyDrag {
                                    current_frame: target_frame,
                                    ..drag
                                });
                                self.current_frame = target_frame;
                                self.current_time = format_timecode(target_frame, self.project_fps);
                                self.mark_project_changed();
                                self.sync_selected_layer_controls();
                                self.sync_bindings_logged();
                            }
                        }
                    }
                    true
                }
                CanvasPointerPhase::Moved => false,
                CanvasPointerPhase::Up | CanvasPointerPhase::Cancelled => {
                    self.graph_key_drag = None;
                    self.timeline_snap_frame = None;
                    self.finish_history_interaction("グラフキーを移動");
                    true
                }
            }
        } else if canvas_id == "kagari.viewport" {
            let Some((left, top, scale_x, scale_y)) = self.viewport_mapping else {
                return false;
            };
            if scale_x <= 0.0 || scale_y <= 0.0 {
                return false;
            }
            let point = [(event.x - left) / scale_x, (event.y - top) / scale_y];

            // 3D Camera interactive navigation: Alt + Primary (Orbit), Alt + Secondary (Pan), Alt + Shift / Tertiary (Dolly)
            if event.phase == CanvasPointerPhase::Down && event.modifiers.alt {
                let comp = self.project_model.active_composition();
                let cam = comp.resolve_camera();
                let cam_pos = cam.transform.position.evaluate(self.current_frame);
                let cam_rot = cam.transform.rotation.evaluate(self.current_frame);
                let mode = if event.modifiers.shift || event.button == Some(ame_ui::event::PointerButton::Middle) {
                    CameraNavMode::Dolly
                } else if event.button == Some(ame_ui::event::PointerButton::Secondary) {
                    CameraNavMode::Pan
                } else {
                    CameraNavMode::Orbit
                };
                self.camera_nav_drag = Some(CameraNavDrag {
                    mode,
                    start_pos: cam_pos,
                    start_rot: cam_rot,
                    start_screen_point: [event.x, event.y],
                });
                return true;
            }

            if event.phase == CanvasPointerPhase::Down
                && event
                    .button
                    .is_none_or(|button| button == ame_ui::event::PointerButton::Primary)
                && !event.space_pressed
                && self.selected_tool == "Selection"
            {
                if let (Some(layer_id), Some(mask_id)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_mask_id.as_deref(),
                ) {
                    let hit = self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                        .and_then(|layer| layer.masks.iter().find(|mask| mask.id == mask_id))
                        .filter(|mask| mask.enabled)
                        .and_then(|mask| {
                            mask.path
                                .vertices_at_frame(self.current_frame)
                                .iter()
                                .enumerate()
                                .map(|(index, vertex)| {
                                    let distance = ((f64::from(vertex[0]) - point[0]) * scale_x)
                                        .hypot((f64::from(vertex[1]) - point[1]) * scale_y);
                                    (index, distance)
                                })
                                .filter(|(_, distance)| {
                                    *distance <= 12.0
                                })
                                .min_by(|a, b| a.1.total_cmp(&b.1))
                        });
                    if let Some((vertex_index, _)) = hit {
                        self.mask_vertex_drag = Some(MaskVertexDrag {
                            layer_id: layer_id.to_owned(),
                            mask_id: mask_id.to_owned(),
                            vertex_index,
                        });
                        return true;
                    }
                }
            }
            if event.phase == CanvasPointerPhase::Down
                && (event.button.is_none()
                    || event.button == Some(ame_ui::event::PointerButton::Primary)
                    || (self.selected_tool == "Zoom"
                        && event.button == Some(ame_ui::event::PointerButton::Secondary)))
            {
                match self.selected_tool {
                    "Hand" if !event.space_pressed => {
                        self.viewport_pan_drag = Some((self.viewport_pan, [event.x, event.y]));
                        return true;
                    }
                    "Zoom" => {
                        let current = self
                            .zoom
                            .trim_end_matches('%')
                            .parse::<f64>()
                            .unwrap_or(100.0);
                        let factor =
                            if event.button == Some(ame_ui::event::PointerButton::Secondary) {
                                0.8
                            } else {
                                1.25
                            };
                        self.zoom = format!("{}%", (current * factor).round().clamp(10.0, 800.0));
                        self.zoom_manually_adjusted = true;
                        self.sync_bindings_logged();
                        return true;
                    }
                    "Text" => {
                        self.add_text_at(point);
                        return true;
                    }
                    "Shape" => {
                        self.add_shape_at(point);
                        return true;
                    }
                    "Pen" => {
                        self.viewport_pen_start = Some(point);
                        return true;
                    }
                    _ if event.space_pressed => {
                        self.viewport_pan_drag = Some((self.viewport_pan, [event.x, event.y]));
                        return true;
                    }
                    _ => {}
                }
            }
            match event.phase {
                CanvasPointerPhase::Down
                    if event.button.is_none()
                        || event.button == Some(ame_ui::event::PointerButton::Primary) =>
                {
                    self.stop_playback();

                    // Check if clicked on a 3D Light Gizmo (hit radius ~14px on screen)
                    let composition = self.project_model.active_composition();
                    let active_cam = composition.resolve_camera();
                    let src_w = composition.width.max(1) as f32;
                    let src_h = composition.height.max(1) as f32;
                    let mut hit_light = None;
                    for light in &composition.lights {
                        let l_pos = light.position.evaluate(self.current_frame);
                        let (lx, ly) = if let Some(norm) = kagari_vfx::core::timeline::project_point_to_screen_at_frame(
                            active_cam,
                            l_pos,
                            src_w,
                            src_h,
                            self.current_frame,
                        ) {
                            (f64::from(norm[0]) * f64::from(src_w), f64::from(norm[1]) * f64::from(src_h))
                        } else {
                            (f64::from(l_pos[0]), f64::from(l_pos[1]))
                        };
                        let dist = (point[0] - lx).hypot(point[1] - ly);
                        if dist * scale_x.min(scale_y) <= 14.0 {
                            hit_light = Some((light.id.clone(), l_pos));
                            break;
                        }
                    }

                    if let Some((light_id, start_pos_3d)) = hit_light {
                        self.light_gizmo_drag = Some(LightGizmoDrag {
                            light_id,
                            start_pos_3d,
                            start_point: point,
                        });
                        return true;
                    }

                    let hit_padding = (8.0 / scale_x.min(scale_y)).clamp(2.0, 48.0);
                    let selected = {
                        self.project_model
                            .active_composition()
                            .layers
                            .iter()
                            .filter(|layer| layer.visible && layer.is_active(self.current_frame))
                            .filter_map(|layer| {
                                let position =
                                    layer.transform.position.evaluate(self.current_frame);
                                let dx = f64::from(position[0]) - point[0];
                                let dy = f64::from(position[1]) - point[1];
                                let rotation = f64::from(
                                    layer.transform.rotation.evaluate(self.current_frame),
                                )
                                .to_radians();
                                let (sin, cos) = rotation.sin_cos();
                                let local_x = dx * cos + dy * sin;
                                let local_y = -dx * sin + dy * cos;
                                let size = layer.bounding_size();
                                let transform_scale =
                                    layer.transform.scale.evaluate(self.current_frame);
                                let half_width = (f64::from(size[0])
                                    * f64::from(transform_scale[0].abs())
                                    / 200.0)
                                    .max(1.0)
                                    + hit_padding;
                                let half_height = (f64::from(size[1])
                                    * f64::from(transform_scale[1].abs())
                                    / 200.0)
                                    .max(1.0)
                                    + hit_padding;
                                let distance = dx.hypot(dy);
                                (local_x.abs() <= half_width && local_y.abs() <= half_height)
                                    .then(|| (distance, layer.id.clone(), position, layer.locked))
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                    };
                    if let Some((_, id, position, locked)) = selected {
                        self.selected_layer_id = Some(id.clone());
                        self.viewport_drag = (!locked).then_some((id, position, point));
                    } else {
                        self.selected_layer_id = None;
                        self.viewport_drag = None;
                    }
                    self.sync_selected_layer_controls();
                    self.sync_bindings_logged();
                    true
                }
                CanvasPointerPhase::Moved => {
                    if let Some(drag) = self.camera_nav_drag.clone() {
                        let dx = (event.x - drag.start_screen_point[0]) as f32;
                        let dy = (event.y - drag.start_screen_point[1]) as f32;
                        let comp = self.project_model.active_composition_mut();
                        let cam = comp.resolve_camera_mut();
                        match drag.mode {
                            CameraNavMode::Orbit => {
                                // Rotate yaw (Y axis) and pitch (X axis)
                                let new_rot = [
                                    (drag.start_rot[0] + dy * 0.4).clamp(-89.0, 89.0),
                                    drag.start_rot[1] + dx * 0.4,
                                    drag.start_rot[2],
                                ];
                                cam.transform.rotation.set_value_at_frame(self.current_frame, new_rot);
                            }
                            CameraNavMode::Pan => {
                                // Pan camera along local screen X and Y
                                let new_pos = [
                                    drag.start_pos[0] - dx * 1.5,
                                    drag.start_pos[1] - dy * 1.5,
                                    drag.start_pos[2],
                                ];
                                cam.transform.position.set_value_at_frame(self.current_frame, new_pos);
                            }
                            CameraNavMode::Dolly => {
                                // Dolly camera along depth (Z axis)
                                let new_pos = [
                                    drag.start_pos[0],
                                    drag.start_pos[1],
                                    drag.start_pos[2] + dy * 3.0,
                                ];
                                cam.transform.position.set_value_at_frame(self.current_frame, new_pos);
                            }
                        }
                        self.mark_project_changed();
                        self.sync_bindings_logged();
                        return true;
                    }
                    if self.mask_vertex_drag.is_some() {
                        self.update_mask_vertex(point);
                        return true;
                    }
                    if let Some((start_offset, start_point)) = self.viewport_pan_drag {
                        self.viewport_pan = [
                            start_offset[0] + event.x - start_point[0],
                            start_offset[1] + event.y - start_point[1],
                        ];
                        return true;
                    }
                    if let Some(drag) = self.light_gizmo_drag.clone() {
                        let dx = (point[0] - drag.start_point[0]) as f32;
                        let dy = (point[1] - drag.start_point[1]) as f32;
                        let new_pos_3d = [
                            drag.start_pos_3d[0] + dx,
                            drag.start_pos_3d[1] + dy,
                            drag.start_pos_3d[2],
                        ];
                        let comp = self.project_model.active_composition_mut();
                        if let Some(light) = comp.lights.iter_mut().find(|l| l.id == drag.light_id) {
                            light.position.set_value_at_frame(self.current_frame, new_pos_3d);
                            self.mark_project_changed();
                            self.sync_bindings_logged();
                        }
                        return true;
                    }
                    if let Some((id, start_position, start_point)) = self.viewport_drag.clone() {
                        let position = [
                            start_position[0] + (point[0] - start_point[0]) as f32,
                            start_position[1] + (point[1] - start_point[1]) as f32,
                        ];
                        if let Some(layer) = self
                            .project_model
                            .active_composition_mut()
                            .layers
                            .iter_mut()
                            .find(|layer| layer.id == id && !layer.locked)
                        {
                            layer
                                .transform
                                .position
                                .set_value_at_frame(self.current_frame, position);
                            self.mark_project_changed();
                            self.sync_selected_layer_controls();
                            self.sync_bindings_logged();
                        }
                    }
                    if self.viewport_pen_start.is_some() {
                        self.viewport_pen_current = Some(point);
                    }
                    true
                }
                CanvasPointerPhase::Up | CanvasPointerPhase::Cancelled => {
                    self.viewport_drag = None;
                    self.light_gizmo_drag = None;
                    self.camera_nav_drag = None;
                    self.mask_vertex_drag = None;
                    self.viewport_pan_drag = None;
                    self.finish_history_interaction("キャンバス上で編集");
                    match event.phase {
                        CanvasPointerPhase::Up => {
                            let _ = self.viewport_pen_current.take();
                            if let Some(start) = self.viewport_pen_start.take() {
                                self.add_pen_line(start, point);
                            }
                        }
                        CanvasPointerPhase::Cancelled => {
                            self.viewport_pen_start = None;
                            self.viewport_pen_current = None;
                        }
                        _ => {}
                    }
                    true
                }
                CanvasPointerPhase::Down => true,
            }
        } else if canvas_id == "kagari.timeline" {
            if self.timeline_width <= 0.0 {
                return false;
            }
            match event.phase {
                CanvasPointerPhase::Down => {
                    self.stop_playback();
                    self.timeline_clip_drag = None;
                    self.timeline_work_area_drag = None;
                    self.timeline_snap_frame = None;
                    if event.y <= 12.0 {
                        if let Some((start_frame, end_frame)) = self.playback_range {
                            let duration = self
                                .project_model
                                .active_composition()
                                .duration_frames
                                .max(1);
                            let (timeline_start, visible_frames) = self.timeline_window(duration);
                            let start_x = self.timeline_width
                                * (f64::from(start_frame) - timeline_start)
                                / visible_frames;
                            let end_x = self.timeline_width
                                * (f64::from(end_frame.saturating_add(1)) - timeline_start)
                                / visible_frames;
                            let start_distance = (event.x - start_x).abs();
                            let end_distance = (event.x - end_x).abs();
                            if start_distance.min(end_distance) <= 10.0 {
                                self.timeline_work_area_drag = Some(if start_distance <= end_distance {
                                    TimelineWorkAreaDragMode::Start
                                } else {
                                    TimelineWorkAreaDragMode::End
                                });
                                self.timeline_scrubbing = false;
                                return true;
                            }
                        }
                    }
                    // Select from the same row geometry used by the layer labels.
                    let track_index = ((event.y - 22.0) / 28.0).floor();
                    if track_index >= 0.0 {
                        let layer_data =
                            self.timeline_layers()
                                .get(track_index as usize)
                                .map(|layer| {
                                    (
                                        layer.id.clone(),
                                        layer.in_frame,
                                        layer.out_frame,
                                        layer.locked,
                                        self.project_model
                                            .active_composition()
                                            .layers
                                            .iter()
                                            .find(|source| source.id == layer.id)
                                            .map(|source| {
                                                layer_keyframe_frames_for_composition(
                                                    self.project_model.active_composition(),
                                                    source,
                                                )
                                            })
                                            .unwrap_or_else(|| layer_keyframe_frames(layer)),
                                        layer
                                            .markers
                                            .iter()
                                            .map(|marker| marker.frame)
                                            .collect::<Vec<_>>(),
                                    )
                                });
                        if let Some((
                            layer_id,
                            in_frame,
                            out_frame,
                            locked,
                            keyframes,
                            marker_frames,
                        )) = layer_data
                        {
                            self.selected_layer_id = Some(layer_id.clone());
                            self.sync_selected_layer_controls();
                            let duration = self
                                .project_model
                                .active_composition()
                                .duration_frames
                                .max(1);
                            let (timeline_start, visible_frames) = self.timeline_window(duration);
                            let clip_start = self.timeline_width
                                * (f64::from(in_frame) - timeline_start)
                                / visible_frames;
                            let clip_end = self.timeline_width
                                * (f64::from(out_frame) - timeline_start)
                                / visible_frames;
                            let row_top = 22.0 + track_index * 28.0;
                            let clicked_layer_marker =
                                if event.y >= row_top + 1.0 && event.y <= row_top + 12.0 {
                                    marker_frames.into_iter().find(|marker_frame| {
                                        let marker_x = self.timeline_width
                                            * (f64::from(*marker_frame) - timeline_start)
                                            / visible_frames;
                                        (event.x - marker_x).abs() <= 7.0
                                    })
                                } else {
                                    None
                                };
                            if let Some(marker_frame) = clicked_layer_marker {
                                self.timeline_scrubbing = false;
                                self.selected_layer_marker_frame = Some(marker_frame);
                                self.current_frame = marker_frame.min(duration.saturating_sub(1));
                                self.current_time =
                                    format_timecode(self.current_frame, self.project_fps);
                                self.sync_selected_layer_controls();
                                self.sync_bindings_logged();
                                return true;
                            }
                            let row_center_y = 22.0 + track_index * 28.0 + 14.0;
                            let clicked_keyframe = if (event.y - row_center_y).abs() <= 9.0 {
                                keyframes.into_iter().find(|keyframe| {
                                    let keyframe_x = self.timeline_width
                                        * (f64::from(*keyframe) - timeline_start)
                                        / visible_frames;
                                    (event.x - keyframe_x).abs() <= 7.0
                                })
                            } else {
                                None
                            };
                            if let Some(keyframe) = clicked_keyframe {
                                self.timeline_scrubbing = false;
                                self.current_frame = keyframe.min(duration.saturating_sub(1));
                                self.current_time =
                                    format_timecode(self.current_frame, self.project_fps);
                                if !locked {
                                    self.timeline_clip_drag = Some(TimelineClipDrag {
                                        layer_id,
                                        start_x: event.x,
                                        initial_in: in_frame,
                                        initial_out: out_frame,
                                        keyframe_source: Some(keyframe),
                                        keyframe_current: Some(keyframe),
                                        mode: TimelineClipDragMode::Keyframe,
                                    });
                                }
                                self.sync_selected_layer_controls();
                                self.sync_bindings_logged();
                                return true;
                            }
                            if event.x >= clip_start && event.x <= clip_end && !locked {
                                let edge_tolerance = 8.0;
                                let mode = if (event.x - clip_start).abs() <= edge_tolerance {
                                    TimelineClipDragMode::TrimStart
                                } else if (event.x - clip_end).abs() <= edge_tolerance {
                                    TimelineClipDragMode::TrimEnd
                                } else {
                                    TimelineClipDragMode::Move
                                };
                                self.timeline_clip_drag = Some(TimelineClipDrag {
                                    layer_id,
                                    start_x: event.x,
                                    initial_in: in_frame,
                                    initial_out: out_frame,
                                    keyframe_source: None,
                                    keyframe_current: None,
                                    mode,
                                });
                                self.timeline_scrubbing = false;
                                self.sync_bindings_logged();
                                return true;
                            }
                        }
                    }
                    self.timeline_scrubbing = true;
                    let duration = self
                        .project_model
                        .active_composition()
                        .duration_frames
                        .max(1);
                    let (timeline_start, visible_frames) = self.timeline_window(duration);
                    let fraction = (event.x / self.timeline_width).clamp(0.0, 1.0);
                    self.current_frame = ((timeline_start + fraction * visible_frames).round()
                        as u32)
                        .min(duration.saturating_sub(1));
                    self.current_time = format_timecode(self.current_frame, self.project_fps);
                    self.sync_selected_layer_controls();
                    self.sync_bindings_logged();
                    true
                }
                CanvasPointerPhase::Moved if self.timeline_work_area_drag.is_some() => {
                    self.update_playback_range_drag(event.x);
                    true
                }
                CanvasPointerPhase::Moved if self.timeline_clip_drag.is_some() => {
                    self.update_timeline_clip_drag(event.x);
                    true
                }
                CanvasPointerPhase::Moved if self.timeline_scrubbing => {
                    self.stop_playback();
                    let duration = self
                        .project_model
                        .active_composition()
                        .duration_frames
                        .max(1);
                    let (timeline_start, visible_frames) = self.timeline_window(duration);
                    let fraction = (event.x / self.timeline_width).clamp(0.0, 1.0);
                    self.current_frame = ((timeline_start + fraction * visible_frames).round()
                        as u32)
                        .min(duration.saturating_sub(1));
                    self.current_time = format_timecode(self.current_frame, self.project_fps);
                    self.sync_selected_layer_controls();
                    self.sync_bindings_logged();
                    true
                }
                CanvasPointerPhase::Moved => false,
                CanvasPointerPhase::Up => {
                    self.timeline_scrubbing = false;
                    self.timeline_clip_drag = None;
                    self.timeline_work_area_drag = None;
                    self.timeline_snap_frame = None;
                    self.finish_history_interaction("タイムラインで編集");
                    true
                }
                CanvasPointerPhase::Cancelled => {
                    self.timeline_scrubbing = false;
                    self.timeline_clip_drag = None;
                    self.timeline_work_area_drag = None;
                    self.timeline_snap_frame = None;
                    self.finish_history_interaction("タイムラインで編集");
                    false
                }
            }
        } else if canvas_id == "kagari.flowchart" {
            let composition = self.project_model.active_composition();
            let graph = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(composition);
            let node_w = 150.0;
            let node_h = 38.0;
            let col_spacing = 180.0;
            let row_spacing = 52.0;
            let origin_x = 24.0;
            let origin_y = 20.0;
            let positions = graph.positions_f64(origin_x, origin_y, col_spacing, row_spacing);

            match event.phase {
                CanvasPointerPhase::Down => {
                    // Check if clicked near output pin (top: Parent, bottom: Matte)
                    for (idx, (nx, ny)) in positions.iter().enumerate() {
                        let parent_pin_center = (nx + node_w, ny + 12.0);
                        let matte_pin_center = (nx + node_w, ny + node_h - 12.0);

                        let p_dist_sq = (event.x - parent_pin_center.0).powi(2) + (event.y - parent_pin_center.1).powi(2);
                        let m_dist_sq = (event.x - matte_pin_center.0).powi(2) + (event.y - matte_pin_center.1).powi(2);

                        if p_dist_sq <= 100.0 {
                            if let Some(layer) = composition.layers.get(idx) {
                                self.flowchart_wire_drag = Some(FlowchartWireDrag {
                                    source_layer_id: layer.id.clone(),
                                    kind: FlowchartWireKind::Parent,
                                    start_pos: parent_pin_center,
                                    current_pos: (event.x, event.y),
                                });
                                return true;
                            }
                        } else if m_dist_sq <= 100.0 {
                            if let Some(layer) = composition.layers.get(idx) {
                                self.flowchart_wire_drag = Some(FlowchartWireDrag {
                                    source_layer_id: layer.id.clone(),
                                    kind: FlowchartWireKind::Matte,
                                    start_pos: matte_pin_center,
                                    current_pos: (event.x, event.y),
                                });
                                return true;
                            }
                        }
                    }

                    // Otherwise, hit-test nodes to select layer
                    for (idx, (nx, ny)) in positions.into_iter().enumerate() {
                        if event.x >= nx && event.x <= nx + node_w && event.y >= ny && event.y <= ny + node_h {
                            if let Some(layer) = composition.layers.get(idx) {
                                if self.selected_layer_id.as_deref() != Some(layer.id.as_str()) {
                                    self.selected_layer_marker_frame = None;
                                }
                                self.selected_layer_id = Some(layer.id.clone());
                                self.sync_selected_layer_controls();
                                self.sync_bindings_logged();
                                return true;
                            }
                        }
                    }
                }
                CanvasPointerPhase::Moved => {
                    if let Some(ref mut drag) = self.flowchart_wire_drag {
                        drag.current_pos = (event.x, event.y);
                        return true;
                    }
                }
                CanvasPointerPhase::Up => {
                    if let Some(drag) = self.flowchart_wire_drag.take() {
                        // Check if dropped onto an input pin of another node (or inside the node)
                        for (idx, (nx, ny)) in positions.iter().enumerate() {
                            let in_pin_center = (*nx, ny + node_h * 0.5);
                            let dist_sq = (event.x - in_pin_center.0).powi(2) + (event.y - in_pin_center.1).powi(2);
                            let inside_node = event.x >= *nx && event.x <= nx + node_w && event.y >= *ny && event.y <= ny + node_h;
                            if dist_sq <= 144.0 || inside_node {
                                if let Some(target_layer) = composition.layers.get(idx) {
                                    if target_layer.id != drag.source_layer_id {
                                        let target_id = target_layer.id.clone();
                                        let source_id = drag.source_layer_id.clone();
                                        let comp = self.project_model.active_composition_mut();
                                        if let Some(target) = comp.layers.iter_mut().find(|l| l.id == target_id) {
                                            match drag.kind {
                                                FlowchartWireKind::Parent => {
                                                    target.parent_id = Some(source_id);
                                                }
                                                FlowchartWireKind::Matte => {
                                                    target.track_matte_layer_id = Some(source_id);
                                                    if target.track_matte == kagari_vfx::core::timeline::TrackMatteMode::None {
                                                        target.track_matte = kagari_vfx::core::timeline::TrackMatteMode::AlphaMatte;
                                                    }
                                                }
                                            }
                                        }
                                        self.mark_project_changed();
                                        self.sync_bindings_logged();
                                        return true;
                                    }
                                }
                            }
                        }
                        return true;
                    }
                }
                CanvasPointerPhase::Cancelled => {
                    self.flowchart_wire_drag = None;
                }
            }
            true
        } else {
            false
        }
    }

    fn draw_canvas(
        &mut self,
        canvas_id: &str,
        scene: &mut Scene,
        bounds: CanvasBounds,
        _context: &mut CanvasContext<'_>,
        _viewport: Viewport,
        _tree: &ViewTree,
    ) {
        self.advance_playback();
        self.maybe_autosave();
        let x = f64::from(bounds.x);
        let y = f64::from(bounds.y);
        let width = f64::from(bounds.width.max(1.0));
        let height = f64::from(bounds.height.max(1.0));
        if canvas_id == "kagari.graph" {
            self.timeline_width = width;
            self.graph_canvas_height = height;
            let duration = self
                .project_model
                .active_composition()
                .duration_frames
                .max(1);
            let (timeline_start, visible_frames) = self.timeline_window(duration);
            let composition = self.project_model.active_composition();
            let selected = self
                .selected_layer_id
                .as_deref()
                .and_then(|id| composition.layers.iter().find(|layer| layer.id == id));
            let camera = selected.and_then(|layer| camera_for_layer(composition, layer));
            let light = selected.and_then(|layer| light_for_layer(composition, layer));
            let keys = selected
                .map(|layer| {
                    graph_channel_keys(
                        layer,
                        camera,
                        light,
                        self.selected_mask_id.as_deref(),
                        self.selected_effect_id.as_deref(),
                        self.graph_effect_parameter_id.as_deref(),
                        self.graph_channel,
                    )
                })
                .unwrap_or_default();
            draw_value_graph(
                scene,
                x,
                y,
                width,
                height,
                self.current_frame,
                duration,
                timeline_start,
                visible_frames,
                self.graph_channel,
                selected,
                camera,
                light,
                self.selected_mask_id.as_deref(),
                self.selected_effect_id.as_deref(),
                self.graph_effect_parameter_id.as_deref(),
                &keys,
            );
            if let Some(snap_frame) = self.timeline_snap_frame {
                let guide_x = x
                    + 14.0
                    + (f64::from(snap_frame) - timeline_start) / visible_frames.max(1.0)
                        * (width - 28.0).max(1.0);
                if guide_x >= x + 14.0 && guide_x <= x + width - 14.0 {
                    scene.stroke(
                        &Stroke::new(1.5),
                        Affine::IDENTITY,
                        Color::from_rgb8(100, 238, 208),
                        None,
                        &Line::new((guide_x, y + 12.0), (guide_x, y + height - 12.0)),
                    );
                }
            }
            return;
        }
        if canvas_id == "kagari.flowchart" {
            let composition = self.project_model.active_composition();
            let drag_wire = self.flowchart_wire_drag.as_ref().map(|d| {
                FlowchartDragWireView {
                    start: d.start_pos,
                    current: d.current_pos,
                    is_matte: d.kind == FlowchartWireKind::Matte,
                }
            });
            draw_flowchart(
                scene,
                x,
                y,
                width,
                height,
                composition,
                self.selected_layer_id.as_deref(),
                drag_wire,
            );
            return;
        }
        if canvas_id == "kagari.timeline" {
            self.timeline_width = width;
            let layers = self.timeline_layers();
            self.update_audio_waveforms(&layers);
            let composition = self.project_model.active_composition();
            let fps = composition.fps.max(1);
            let (timeline_start, visible_frames) =
                self.timeline_window(composition.duration_frames);
            draw_timeline(
                scene,
                x,
                y,
                width,
                height,
                self.current_frame,
                self.playback_range,
                composition.duration_frames,
                timeline_start,
                visible_frames,
                &layers,
                self.selected_layer_id.as_deref(),
                self.selected_layer_marker_frame,
                &self.waveform_cache,
                fps,
            );
            for (row, layer) in layers.iter().enumerate() {
                let camera_id =
                    layer
                        .scene_object
                        .as_ref()
                        .and_then(|scene_object| match scene_object {
                            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => {
                                Some(id.as_str())
                            }
                            _ => None,
                        });
                let Some(camera) = camera_id
                    .and_then(|id| composition.cameras.iter().find(|camera| camera.id == id))
                else {
                    continue;
                };
                for keyframe in camera_keyframe_frames(camera) {
                    let marker_x = x + width * (f64::from(keyframe) - timeline_start)
                        / visible_frames.max(1.0);
                    if marker_x < x || marker_x > x + width {
                        continue;
                    }
                    let center_y = y + 22.0 + row as f64 * 28.0 + 14.0;
                    let mut diamond = BezPath::new();
                    diamond.move_to((marker_x, center_y - 4.0));
                    diamond.line_to((marker_x + 4.0, center_y));
                    diamond.line_to((marker_x, center_y + 4.0));
                    diamond.line_to((marker_x - 4.0, center_y));
                    diamond.close_path();
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        Color::from_rgb8(255, 218, 174),
                        None,
                        &diamond,
                    );
                }
            }
            if let Some(snap_frame) = self.timeline_snap_frame {
                let guide_x =
                    x + width * (f64::from(snap_frame) - timeline_start) / visible_frames.max(1.0);
                if guide_x >= x && guide_x <= x + width {
                    scene.stroke(
                        &Stroke::new(1.5),
                        Affine::IDENTITY,
                        Color::from_rgb8(100, 238, 208),
                        None,
                        &Line::new((guide_x, y), (guide_x, y + height)),
                    );
                }
            }
            return;
        }
        if canvas_id == "kagari.asset_preview" {
            self.update_audio_waveforms(&[]);
            let bounds_rect = Rect::new(x, y, x + width, y + height);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(10, 11, 13),
                None,
                &bounds_rect,
            );
            let selected = self.selected_project_asset_id.as_deref().and_then(|id| {
                self.project_model
                    .assets
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| (item.id.clone(), item.item_type.clone()))
            });
            let selected_render_key = selected.as_ref().and_then(|(asset_id, item_type)| {
                use kagari_vfx::core::timeline::ProjectItemType;
                match item_type {
                    ProjectItemType::Image { path, .. } => {
                        Some(asset_image_preview_cache_key(asset_id, path))
                    }
                    ProjectItemType::Video { path, .. } => {
                        find_video_preview_frame(&self.project_model.compositions, path)
                            .map(|frame| asset_image_preview_cache_key(
                                asset_id,
                                &frame.to_string_lossy(),
                            ))
                    }
                    ProjectItemType::Model3D { path } => Some(model_preview_cache_key(path)),
                    ProjectItemType::Composition { .. } => Some(format!(
                        "composition:{asset_id}:{}",
                        self.preview_revision
                    )),
                    _ => None,
                }
            });
            self.asset_render_preview_pending
                .retain(|key, _| selected_render_key.as_ref() == Some(key));
            if self.update_asset_preview_results() {
                self.sync_bindings_logged();
            }
            if let Some((asset_id, item_type)) = selected {
                use kagari_vfx::core::timeline::ProjectItemType;
                match item_type {
                    ProjectItemType::Audio { path, .. } => {
                        self.asset_preview_image = None;
                        self.asset_preview_failed_asset_id = None;
                        if let Some(waveform) = self.waveform_cache.get(&path) {
                            draw_asset_waveform(scene, x, y, width, height, waveform);
                        }
                    }
                    ProjectItemType::Model3D { path } => {
                        let cache_key = model_preview_cache_key(&path);
                        match self.asset_render_preview_cache.get(&cache_key) {
                            Some(Some(preview)) => {
                                let image = vello::peniko::ImageData {
                                    data: vello::peniko::Blob::new(preview.pixels.clone()),
                                    format: vello::peniko::ImageFormat::Rgba8,
                                    alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                    width: preview.width,
                                    height: preview.height,
                                };
                                self.asset_preview_image = Some((cache_key, Some(image)));
                                self.asset_preview_failed_asset_id = None;
                            }
                            Some(None) => {
                                self.asset_preview_image = Some((cache_key, None));
                                self.asset_preview_failed_asset_id = Some(asset_id.clone());
                            }
                            None => {
                                self.asset_preview_image = Some((cache_key.clone(), None));
                                self.asset_preview_failed_asset_id = None;
                                if !self.asset_render_preview_requested.contains(&cache_key)
                                    && !self.asset_render_preview_pending.contains_key(&cache_key)
                                {
                                    let job = AssetPreviewJob::Model3D(path);
                                    if self
                                        .asset_render_preview_request_tx
                                        .try_send((cache_key.clone(), job.clone()))
                                        .is_ok()
                                    {
                                        self.asset_render_preview_requested.insert(cache_key);
                                    } else {
                                        self.asset_render_preview_pending.insert(cache_key, job);
                                    }
                                }
                            }
                        }
                    }
                    ProjectItemType::Solid { color } => {
                        let rgba = color.map(|component| {
                            (component.clamp(0.0, 1.0) * 255.0).round() as u8
                        });
                        let cache_key = format!(
                            "solid:{asset_id}:{}:{}:{}:{}",
                            rgba[0], rgba[1], rgba[2], rgba[3]
                        );
                        let needs_refresh = self
                            .asset_preview_image
                            .as_ref()
                            .is_none_or(|(key, _)| key != &cache_key);
                        if needs_refresh {
                            let pixels = rgba.repeat(160 * 90);
                            let image = vello::peniko::ImageData {
                                data: vello::peniko::Blob::new(std::sync::Arc::new(pixels)),
                                format: vello::peniko::ImageFormat::Rgba8,
                                alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                width: 160,
                                height: 90,
                            };
                            self.asset_preview_image = Some((cache_key, Some(image)));
                            self.asset_preview_failed_asset_id = None;
                        }
                    }
                    ProjectItemType::Composition { comp_idx } => {
                        let cache_key = format!("composition:{asset_id}:{}", self.preview_revision);
                        match self.asset_render_preview_cache.get(&cache_key) {
                            Some(Some(preview)) => {
                                let image = vello::peniko::ImageData {
                                    data: vello::peniko::Blob::new(preview.pixels.clone()),
                                    format: vello::peniko::ImageFormat::Rgba8,
                                    alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                    width: preview.width,
                                    height: preview.height,
                                };
                                self.asset_preview_image = Some((cache_key, Some(image)));
                                self.asset_preview_failed_asset_id = None;
                            }
                            Some(None) => {
                                self.asset_preview_image = Some((cache_key, None));
                                self.asset_preview_failed_asset_id = Some(asset_id.clone());
                            }
                            None => {
                                self.asset_preview_image = Some((cache_key.clone(), None));
                                self.asset_preview_failed_asset_id = None;
                                if !self.asset_render_preview_requested.contains(&cache_key)
                                    && !self.asset_render_preview_pending.contains_key(&cache_key)
                                {
                                    if let Some(composition) = self
                                        .project_model
                                        .compositions
                                        .get(comp_idx)
                                        .cloned()
                                    {
                                        let job = AssetPreviewJob::Composition(Box::new(composition));
                                        if self
                                            .asset_render_preview_request_tx
                                            .try_send((cache_key.clone(), job.clone()))
                                            .is_ok()
                                        {
                                            self.asset_render_preview_requested.insert(cache_key);
                                        } else {
                                            self.asset_render_preview_pending.insert(cache_key, job);
                                        }
                                    } else {
                                        self.asset_preview_failed_asset_id = Some(asset_id.clone());
                                    }
                                }
                            }
                        }
                    }
                    item_type => {
                        let path = match item_type {
                            ProjectItemType::Image { path, .. } => Some(std::path::PathBuf::from(path)),
                            ProjectItemType::Video { path, .. } => {
                                find_video_preview_frame(&self.project_model.compositions, &path)
                            }
                            _ => None,
                        };
                        if let Some(path) = path {
                            let path = path.to_string_lossy().into_owned();
                            let cache_key = asset_image_preview_cache_key(&asset_id, &path);
                            match self.asset_render_preview_cache.get(&cache_key) {
                                Some(Some(preview)) => {
                                    let image = vello::peniko::ImageData {
                                        data: vello::peniko::Blob::new(preview.pixels.clone()),
                                        format: vello::peniko::ImageFormat::Rgba8,
                                        alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                        width: preview.width,
                                        height: preview.height,
                                    };
                                    self.asset_preview_image = Some((cache_key, Some(image)));
                                    self.asset_preview_failed_asset_id = None;
                                }
                                Some(None) => {
                                    self.asset_preview_image = Some((cache_key, None));
                                    self.asset_preview_failed_asset_id = Some(asset_id.clone());
                                }
                                None => {
                                    self.asset_preview_image = Some((cache_key.clone(), None));
                                    self.asset_preview_failed_asset_id = None;
                                    if !self.asset_render_preview_requested.contains(&cache_key)
                                        && !self.asset_render_preview_pending.contains_key(&cache_key)
                                    {
                                        let job = AssetPreviewJob::Image(path);
                                        if self
                                            .asset_render_preview_request_tx
                                            .try_send((cache_key.clone(), job.clone()))
                                            .is_ok()
                                        {
                                            self.asset_render_preview_requested.insert(cache_key);
                                        } else {
                                            self.asset_render_preview_pending.insert(cache_key, job);
                                        }
                                    }
                                }
                            }
                        } else {
                            self.asset_preview_image = None;
                            self.asset_preview_failed_asset_id = None;
                        }
                    }
                }
            } else {
                self.asset_preview_image = None;
                self.asset_preview_failed_asset_id = None;
            }
            if let Some((_, Some(image))) = &self.asset_preview_image {
                let fit = ((width - 24.0).max(1.0) / f64::from(image.width.max(1)))
                    .min((height - 24.0).max(1.0) / f64::from(image.height.max(1)));
                let draw_width = f64::from(image.width) * fit;
                let draw_height = f64::from(image.height) * fit;
                let left = x + (width - draw_width) * 0.5;
                let top = y + (height - draw_height) * 0.5;
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &bounds_rect);
                scene.draw_image(image, Affine::translate((left, top)) * Affine::scale(fit));
                scene.pop_layer();
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(56, 62, 69),
                    None,
                    &Rect::new(left, top, left + draw_width, top + draw_height),
                );
            }
            return;
        }
        if canvas_id != "kagari.viewport" {
            return;
        }
        let composition = self.project_model.active_composition();
        let preview_limit = if self.adaptive_preview && self.is_playing {
            320.0
        } else {
            640.0
        };
        let source_width = f64::from(composition.width.max(1));
        let source_height = f64::from(composition.height.max(1));
        let pixel_aspect = f64::from(composition.pixel_aspect).clamp(0.1, 10.0);
        let proxy_scale = f64::from(kagari_vfx::core::proxy::composition_preview_scale(composition));
        let render_scale = (preview_limit / (source_width * pixel_aspect))
            .min(preview_limit / source_height)
            .min(1.0)
            * proxy_scale;
        let render_width = source_width * render_scale;
        let render_height = source_height * render_scale;
        let display_width = render_width * pixel_aspect;
        let zoom_factor = self
            .zoom
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or(100.0)
            .clamp(10.0, 800.0)
            / 100.0;
        let fit = (width - 32.0)
            .max(1.0)
            .min((height - 32.0).max(1.0) * display_width / render_height)
            / display_width
            * zoom_factor;
        let draw_width = display_width * fit;
        let draw_height = render_height * fit;
        self.viewport_mapping = Some((
            (width - draw_width) * 0.5 + self.viewport_pan[0],
            (height - draw_height) * 0.5 + self.viewport_pan[1],
            render_scale * fit * pixel_aspect,
            render_scale * fit,
        ));
        self.draw_viewport(scene, x, y, width, height);
    }

    fn content_max_width(&self) -> Option<f32> {
        None
    }

    fn canvas_frame_interval(&self) -> Option<std::time::Duration> {
        if self.is_playing
            || self.render_events.is_some()
            || self.video_import.is_some()
            || self.proxy_generation.is_some()
            || self.render_refresh_ticks > 0
        {
            Some(std::time::Duration::from_millis(16))
        } else if self.screen == Screen::Editor
            && (!self.waveform_requested.is_empty()
                                || !self.asset_render_preview_requested.is_empty()
                                || !self.asset_render_preview_pending.is_empty()
                                || self.viewport_preview_inflight)
        {
            Some(std::time::Duration::from_millis(100))
        } else if self.screen == Screen::Editor
            && self.autosave_secs > 0
            && self.project_file_path.is_some()
        {
            Some(std::time::Duration::from_secs(
                self.autosave_secs.clamp(1, 60),
            ))
        } else {
            None
        }
    }

    fn on_frame_metrics(&mut self, _metrics: ame_ui::app::FrameMetrics) {
        let was_rendering = self.render_events.is_some();
        let events_changed = self.receive_video_export_events();
        let import_changed = self.receive_video_import_result();
        let proxy_changed = self.receive_layer_proxy_generation();
        if !was_rendering && self.render_refresh_ticks > 0 {
            self.render_refresh_ticks -= 1;
        }
        if (was_rendering && events_changed) || import_changed || proxy_changed {
            self.sync_bindings_logged();
        }
    }

    fn theme(&self) -> Theme {
        Theme {
            // Use soft panel corners throughout the studio. Square cards made
            // the new interface read like a default utility app.
            panel_radius: 14.0,
            panel_padding_x: 0.0,
            panel_padding_y: 0.0,
            control_radius: 10.0,
            input_radius: 9.0,
            background: Color::from_rgb8(11, 12, 14),
            surface: Color::from_rgb8(17, 22, 26),
            surface_border: Color::from_rgb8(42, 49, 55),
            text: Color::from_rgb8(232, 235, 239),
            secondary_text: Color::from_rgb8(158, 165, 173),
            placeholder_text: Color::from_rgb8(112, 120, 128),
            accent: Color::from_rgb8(245, 140, 75),
            accent_hover: Color::from_rgb8(255, 160, 99),
            accent_pressed: Color::from_rgb8(222, 113, 54),
            accent_text: Color::from_rgb8(255, 255, 255),
            // Navigation uses text-only secondary buttons, matching the original
            // sidebar where only the selected row owns a background surface.
            secondary_button: Color::from_rgb8(17, 22, 26),
            secondary_button_hover: Color::from_rgb8(27, 34, 39),
            secondary_button_pressed: Color::from_rgb8(36, 43, 49),
            secondary_button_text: Color::from_rgb8(220, 222, 228),
            destructive_button: Color::from_rgb8(69, 43, 45),
            destructive_button_hover: Color::from_rgb8(84, 49, 52),
            destructive_button_pressed: Color::from_rgb8(98, 52, 55),
            destructive_text: Color::from_rgb8(255, 166, 166),
            disabled_button: Color::from_rgb8(43, 47, 51),
            disabled_text: Color::from_rgb8(112, 114, 120),
            input_background: Color::from_rgb8(13, 13, 13),
            input_border: Color::from_rgb8(64, 70, 76),
            selection: Color::from_rgba8(245, 126, 53, 108),
            focus_ring: Color::from_rgb8(255, 151, 88),
        }
    }

    fn window_config(&self) -> WindowConfig {
        self.ui.window.clone()
    }

    fn shared_view_tree(&mut self) -> Option<ViewTree> {
        match self.ui.view_tree() {
            Ok(tree) => Some(tree),
            Err(error) => {
                eprintln!("[kagari-studio] .ame evaluation error: {error}");
                None
            }
        }
    }

    fn on_event(&mut self, event: &AmeEvent) {
        if let AmeEvent::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
        } = event
        {
            let action = if modifiers.is_primary() {
                match key.to_ascii_lowercase().as_str() {
                    "z" if modifiers.shift => Some("redo_project"),
                    "z" => Some("undo_project"),
                    "y" => Some("redo_project"),
                    "s" if self.screen == Screen::Editor => Some("save_project"),
                    "n" => Some("new_project"),
                    "m" if self.screen == Screen::Editor => Some("go_export"),
                    _ => None,
                }
            } else if self.screen == Screen::Editor && !modifiers.alt && !modifiers.shift {
                match key.to_ascii_lowercase().as_str() {
                    "f5" => Some("reload_ame_ui"),
                    "f9" => Some("ease_selected_layer"),
                    " " | "space" => Some("play_toggle"),
                    "j" => Some("previous_keyframe"),
                    "k" => Some("next_keyframe"),
                    "b" => Some("set_playback_range_start"),
                    "n" => Some("set_playback_range_end"),
                    "[" => Some("layer_in_to_playhead"),
                    "]" => Some("layer_out_to_playhead"),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(name) = action {
                let _ = self.handle_action(HostAction {
                    name: name.to_owned(),
                    value: None,
                    target: None,
                });
            }
        }
        if let AmeEvent::Resized { width, height } = event {
            let compact = *width < 900 || *height < 620;
            let narrow = *width < 760;
            let wide = *width >= 1300 && *height >= 700;
            let mut changed = false;
            if narrow != self.narrow {
                self.narrow = narrow;
                changed = true;
            }
            if compact != self.compact {
                self.compact = compact;
                if !self.zoom_manually_adjusted {
                    self.zoom = if compact { "20%" } else { "33%" }.to_owned();
                }
                changed = true;
            }
            if wide != self.wide {
                self.wide = wide;
                changed = true;
            }
            if changed {
                self.sync_bindings_logged();
            }
        }
        App::on_event(&mut self.ui, event);
        while let Some(action) = self.ui.take_pending_event() {
            if let Err(error) = self.handle_action(action) {
                eprintln!("Ame VFX action failed: {error}");
            }
        }
    }

    fn on_close_requested(&mut self) -> bool {
        if self.confirm_exit {
            self.exit_confirm_open = true;
            self.sync_bindings_logged();
            false
        } else {
            true
        }
    }
}

pub(crate) fn rgba_to_hex(color: &[f32; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(crate) fn effect_type_label(effect: &kagari_vfx::core::timeline::EffectType) -> &'static str {
    use kagari_vfx::core::timeline::EffectType;
    match effect {
        EffectType::GaussianBlur { .. } => "ぼかし",
        EffectType::ColorTint { .. } => "色調",
        EffectType::DropShadow { .. } => "影",
        EffectType::Vignette { .. } => "ビネット",
        EffectType::Levels { .. } => "Levels",
        EffectType::HueSaturation { .. } => "HSL",
        EffectType::Glow { .. } => "Glow",
        EffectType::MotionBlur { .. } => "モーションブラー",
        EffectType::FilmGrain { .. } => "フィルムグレイン",
        EffectType::Posterize { .. } => "ポスタライズ",
        EffectType::DirectionalBlur { .. } => "方向ぼかし",
        EffectType::RadialBlur { .. } => "放射状ぼかし",
        EffectType::Sharpen { .. } => "シャープ",
        EffectType::Twirl { .. } => "渦巻き",
        EffectType::Bulge { .. } => "膨張・収縮",
        EffectType::Invert { .. } => "反転",
        EffectType::Offset { .. } => "オフセット",
        EffectType::Threshold { .. } => "しきい値",
        EffectType::LinearWipe { .. } => "リニアワイプ",
        EffectType::Spherize { .. } => "球面化",
        EffectType::Vibrance { .. } => "自然な彩度",
        EffectType::WhiteBalance { .. } => "ホワイトバランス",
        _ => "その他",
    }
}

pub(crate) fn effect_parameter_specs(
    effect: &kagari_vfx::core::timeline::EffectType,
    frame: u32,
) -> Vec<(&'static str, &'static str, f32, &'static str)> {
    use kagari_vfx::core::timeline::EffectType;
    let mut specs = match effect {
        EffectType::GaussianBlur { blur_radius } => {
            vec![("blur_radius", "半径", blur_radius.evaluate(frame), "px")]
        }
        EffectType::ColorTint { intensity, .. } => {
            vec![("intensity", "強度", intensity.evaluate(frame), "%")]
        }
        EffectType::DropShadow {
            opacity,
            direction,
            distance,
            softness,
            ..
        } => vec![
            ("opacity", "不透明度", opacity.evaluate(frame), "%"),
            ("direction", "角度", direction.evaluate(frame), "°"),
            ("distance", "距離", distance.evaluate(frame), "px"),
            ("softness", "ぼかし", softness.evaluate(frame), "px"),
        ],
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            ..
        } => vec![
            ("intensity", "強度", intensity.evaluate(frame), "%"),
            ("roundness", "丸み", roundness.evaluate(frame), ""),
            ("feather", "ぼかし", feather.evaluate(frame), "%"),
        ],
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => vec![
            ("input_black", "入力黒", input_black.evaluate(frame), ""),
            ("input_white", "入力白", input_white.evaluate(frame), ""),
            ("gamma", "ガンマ", gamma.evaluate(frame), ""),
            ("output_black", "出力黒", output_black.evaluate(frame), ""),
            ("output_white", "出力白", output_white.evaluate(frame), ""),
        ],
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => vec![
            ("hue_shift", "色相", hue_shift.evaluate(frame), "°"),
            ("saturation", "彩度", saturation.evaluate(frame), "%"),
            ("lightness", "明度", lightness.evaluate(frame), "%"),
        ],
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            ..
        } => vec![
            ("threshold", "しきい値", threshold.evaluate(frame), "%"),
            ("radius", "半径", radius.evaluate(frame), "px"),
            ("intensity", "強度", intensity.evaluate(frame), "%"),
        ],
        EffectType::MotionBlur { shutter_angle, .. } => vec![(
            "shutter_angle",
            "シャッター角",
            shutter_angle.evaluate(frame),
            "°",
        )],
        EffectType::FilmGrain { intensity, .. } => {
            vec![("intensity", "強度", intensity.evaluate(frame), "%")]
        }
        EffectType::Posterize { levels } => {
            vec![("levels", "階調数", levels.evaluate(frame), "段")]
        }
        EffectType::DirectionalBlur { angle, length } => vec![
            ("angle", "角度", angle.evaluate(frame), "°"),
            ("length", "長さ", length.evaluate(frame), "px"),
        ],
        EffectType::RadialBlur { amount } => {
            vec![("amount", "強度", amount.evaluate(frame), "×")]
        }
        EffectType::Sharpen { amount } => {
            vec![("amount", "量", amount.evaluate(frame), "×")]
        }
        _ => Vec::new(),
    };
    use kagari_vfx::core::effect_params::ParamRefRef;
    let known = specs
        .iter()
        .map(|(id, _, _, _)| *id)
        .collect::<std::collections::HashSet<_>>();
    for (id, parameter) in effect.animatable_params_ref() {
        if known.contains(id) {
            continue;
        }
        if let ParamRefRef::Scalar(track) = parameter {
            // Expose reflected scalar tracks for newly wired effects without
            // requiring a hand-written property row for every effect type.
            specs.push((id, id, track.evaluate(frame), ""));
        }
    }
    specs
}

pub(crate) fn scalar_effect_parameters(
    effect: &kagari_vfx::core::timeline::EffectType,
) -> Vec<(String, String)> {
    use kagari_vfx::core::effect_params::ParamRefRef;
    let labels = effect_parameter_specs(effect, 0)
        .into_iter()
        .map(|(id, label, _, _)| (id, label))
        .collect::<std::collections::HashMap<_, _>>();
    effect
        .animatable_params_ref()
        .into_iter()
        .filter_map(|(id, parameter)| {
            matches!(parameter, ParamRefRef::Scalar(_)).then(|| {
                (
                    id.to_owned(),
                    labels.get(id).copied().unwrap_or(id).to_owned(),
                )
            })
        })
        .collect()
}

pub(crate) fn effect_color_parameters(
    effect: &kagari_vfx::core::timeline::EffectType,
) -> Vec<(String, String)> {
    use kagari_vfx::core::effect_params::ParamRefRef;
    effect
        .animatable_params_ref()
        .into_iter()
        .filter_map(|(id, parameter)| {
            matches!(parameter, ParamRefRef::Vec4Color(_)).then(|| (id.to_owned(), id.to_owned()))
        })
        .collect()
}

pub(crate) fn graph_effect_parameter_label(
    layer: &kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<String> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    scalar_effect_parameters(&effect.effect_type)
        .into_iter()
        .find(|(id, _)| Some(id.as_str()) == parameter_id)
        .map(|(_, label)| format!("{} · {}", effect.name, label))
}

pub(crate) fn scalar_effect_track<'a>(
    layer: &'a kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a kagari_vfx::core::timeline::Animatable<f32>> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params_ref()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Scalar(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn scalar_effect_track_mut<'a>(
    layer: &'a mut kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a mut kagari_vfx::core::timeline::Animatable<f32>> {
    let effect = layer
        .effects
        .iter_mut()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRef::Scalar(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_track<'a>(
    layer: &'a kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a kagari_vfx::core::timeline::Animatable<[f32; 4]>> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params_ref()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Vec4Color(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_track_mut<'a>(
    layer: &'a mut kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a mut kagari_vfx::core::timeline::Animatable<[f32; 4]>> {
    let effect = layer
        .effects
        .iter_mut()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRef::Vec4Color(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_component(channel: GraphChannel) -> Option<usize> {
    match channel {
        GraphChannel::EffectColorRed => Some(0),
        GraphChannel::EffectColorGreen => Some(1),
        GraphChannel::EffectColorBlue => Some(2),
        GraphChannel::EffectColorAlpha => Some(3),
        _ => None,
    }
}

pub(crate) fn graph_color_component_name(channel: GraphChannel) -> &'static str {
    match channel {
        GraphChannel::EffectColorRed => "R",
        GraphChannel::EffectColorGreen => "G",
        GraphChannel::EffectColorBlue => "B",
        GraphChannel::EffectColorAlpha => "A",
        _ => "",
    }
}

pub(crate) fn update_color_component_graph_track(
    track: &mut kagari_vfx::core::timeline::Animatable<[f32; 4]>,
    component: usize,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> bool {
    if track.keyframes().is_none() {
        return false;
    }
    if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
        return false;
    }
    let mut color = track.evaluate(target_frame);
    color[component] = value;
    track.set_value_at_frame(target_frame, color);
    true
}

pub(crate) fn shape_parameter_track(
    layer: &kagari_vfx::core::timeline::Layer,
    channel: GraphChannel,
) -> Option<&kagari_vfx::core::timeline::Animatable<f32>> {
    use kagari_vfx::core::timeline::{LayerType, ShapeType};
    let LayerType::Shape { shape_type, .. } = &layer.layer_type else {
        return None;
    };
    match (shape_type, channel) {
        (
            ShapeType::Rectangle { width, .. } | ShapeType::Ellipse { width, .. },
            GraphChannel::ShapeWidth,
        ) => Some(width),
        (
            ShapeType::Rectangle { height, .. } | ShapeType::Ellipse { height, .. },
            GraphChannel::ShapeHeight,
        ) => Some(height),
        (ShapeType::Rectangle { corner_radius, .. }, GraphChannel::ShapeCornerRadius) => {
            Some(corner_radius)
        }
        (ShapeType::Star { points, .. }, GraphChannel::ShapePoints) => Some(points),
        (ShapeType::Star { inner_radius, .. }, GraphChannel::ShapeInnerRadius) => {
            Some(inner_radius)
        }
        (ShapeType::Star { outer_radius, .. }, GraphChannel::ShapeOuterRadius) => {
            Some(outer_radius)
        }
        (ShapeType::Polygon { sides, .. }, GraphChannel::ShapeSides) => Some(sides),
        (ShapeType::Polygon { radius, .. }, GraphChannel::ShapeRadius) => Some(radius),
        _ => None,
    }
}

pub(crate) fn shape_parameter_track_mut(
    layer: &mut kagari_vfx::core::timeline::Layer,
    channel: GraphChannel,
) -> Option<&mut kagari_vfx::core::timeline::Animatable<f32>> {
    use kagari_vfx::core::timeline::{LayerType, ShapeType};
    let LayerType::Shape { shape_type, .. } = &mut layer.layer_type else {
        return None;
    };
    match (shape_type, channel) {
        (
            ShapeType::Rectangle { width, .. } | ShapeType::Ellipse { width, .. },
            GraphChannel::ShapeWidth,
        ) => Some(width),
        (
            ShapeType::Rectangle { height, .. } | ShapeType::Ellipse { height, .. },
            GraphChannel::ShapeHeight,
        ) => Some(height),
        (ShapeType::Rectangle { corner_radius, .. }, GraphChannel::ShapeCornerRadius) => {
            Some(corner_radius)
        }
        (ShapeType::Star { points, .. }, GraphChannel::ShapePoints) => Some(points),
        (ShapeType::Star { inner_radius, .. }, GraphChannel::ShapeInnerRadius) => {
            Some(inner_radius)
        }
        (ShapeType::Star { outer_radius, .. }, GraphChannel::ShapeOuterRadius) => {
            Some(outer_radius)
        }
        (ShapeType::Polygon { sides, .. }, GraphChannel::ShapeSides) => Some(sides),
        (ShapeType::Polygon { radius, .. }, GraphChannel::ShapeRadius) => Some(radius),
        _ => None,
    }
}

pub(crate) fn set_f32_if_changed(target: &mut f32, next: f32) -> bool {
    if !next.is_finite() || (*target - next).abs() <= f32::EPSILON {
        false
    } else {
        *target = next;
        true
    }
}

pub(crate) fn parse_hex_rgb(input: &str, alpha: f32) -> Option<[f32; 4]> {
    let value = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some([
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha.clamp(0.0, 1.0),
    ])
}

pub(crate) fn add_current_keyframe<T: kagari_vfx::core::property::Interpolate>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
) {
    let value = track.evaluate(frame);
    track.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
        frame,
        value,
        kagari_vfx::core::keyframe::InterpolationType::Linear,
    ));
}

pub(crate) fn set_animatable_at_frame<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
    value: T,
) -> bool {
    if track.keyframes().is_some() {
        track.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
            frame,
            value,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
    } else {
        *track = kagari_vfx::core::timeline::Animatable::new_constant(value);
    }
    true
}

pub(crate) fn media_file_status(path: &str) -> String {
    let media_path = std::path::Path::new(path);
    let name = media_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned());
    if media_path.is_file() {
        name
    } else {
        format!("{name} · ファイルが見つかりません")
    }
}

pub(crate) fn scene_layer_kind_label(layer: &kagari_vfx::core::timeline::Layer) -> &'static str {
    use kagari_vfx::core::timeline::{LayerType, SceneObjectRef};

    match layer.scene_object.as_ref() {
        Some(SceneObjectRef::Camera { .. }) => "カメラ",
        Some(SceneObjectRef::Light { .. }) => "ライト",
        None => match &layer.layer_type {
            LayerType::Particle { .. } => "パーティクル",
            LayerType::Image { .. } => "画像",
            LayerType::Video { .. } => "動画",
            LayerType::Audio { .. } => "音声",
            LayerType::Text { .. } => "テキスト",
            LayerType::Shape { .. } => "シェイプ",
            LayerType::Solid { .. } => "平面",
            LayerType::Model3D { .. } => "3Dモデル",
            LayerType::PreComp { .. } => "プリコンポーズ",
            LayerType::Null => "ヌル",
            LayerType::AdjustmentLayer => "調整レイヤー",
        },
    }
}

pub(crate) fn scene_layer_depths(
    layers: &[kagari_vfx::core::timeline::Layer],
    layer_by_id: &std::collections::HashMap<
        &str,
        &kagari_vfx::core::timeline::Layer,
    >,
) -> std::collections::HashMap<String, usize> {
    let mut depths = std::collections::HashMap::with_capacity(layers.len());
    for layer in layers {
        if depths.contains_key(&layer.id) {
            continue;
        }
        let mut path = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current = Some(layer);
        let mut base_depth = 0usize;
        while let Some(candidate) = current {
            if let Some(depth) = depths.get(candidate.id.as_str()) {
                base_depth = *depth;
                break;
            }
            if !visited.insert(candidate.id.as_str()) {
                break;
            }
            path.push(candidate);
            current = candidate
                .parent_id
                .as_deref()
                .and_then(|parent_id| layer_by_id.get(parent_id).copied());
        }
        for candidate in path.into_iter().rev() {
            let has_parent = candidate
                .parent_id
                .as_deref()
                .is_some_and(|parent_id| layer_by_id.contains_key(parent_id));
            base_depth = if has_parent {
                base_depth.saturating_add(1)
            } else {
                0
            };
            depths.insert(candidate.id.clone(), base_depth);
        }
    }
    depths
}

pub(crate) fn replace_project_media_path(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    old_path: &str,
    new_path: &str,
) {
    use kagari_vfx::core::timeline::LayerType;
    for composition in compositions {
        for layer in &mut composition.layers {
            match &mut layer.layer_type {
                LayerType::Image { path }
                | LayerType::Audio { path, .. }
                | LayerType::Model3D { path }
                    if path == old_path =>
                {
                    *path = new_path.to_owned();
                }
                _ => {}
            }
        }
        replace_project_media_path(&mut composition.sub_compositions, old_path, new_path);
    }
}

pub(crate) fn remove_precomp_references(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    removed_comp_id: &str,
) -> usize {
    use kagari_vfx::core::timeline::LayerType;
    let mut removed = 0;
    for composition in compositions {
        let previous_count = composition.layers.len();
        composition.layers.retain(|layer| {
            !matches!(
                &layer.layer_type,
                LayerType::PreComp { comp_id } if comp_id == removed_comp_id
            )
        });
        removed += previous_count - composition.layers.len();
        removed += remove_precomp_references(&mut composition.sub_compositions, removed_comp_id);
    }
    removed
}

pub(crate) fn clone_composition_asset_tree(
    project_compositions: &[kagari_vfx::core::timeline::Composition],
    source_index: usize,
    root_id: &str,
) -> Result<Vec<kagari_vfx::core::timeline::Composition>, &'static str> {
    use kagari_vfx::core::timeline::LayerType;
    fn collect_existing_ids(
        composition: &kagari_vfx::core::timeline::Composition,
        ids: &mut std::collections::HashSet<String>,
    ) {
        ids.insert(composition.id.clone());
        for child in &composition.sub_compositions {
            collect_existing_ids(child, ids);
        }
    }
    fn assign_tree_ids(
        composition: &kagari_vfx::core::timeline::Composition,
        root_id: &str,
        next: &mut usize,
        ids: &mut std::collections::HashMap<String, String>,
        reserved_ids: &mut std::collections::HashSet<String>,
    ) {
        let id = loop {
            let candidate = if *next == 0 {
                root_id.to_owned()
            } else {
                format!("{root_id}_nested_{}", *next)
            };
            *next += 1;
            if reserved_ids.insert(candidate.clone()) {
                break candidate;
            }
        };
        ids.entry(composition.id.clone()).or_insert(id);
        for child in &composition.sub_compositions {
            assign_tree_ids(child, root_id, next, ids, reserved_ids);
        }
    }
    fn collect_references(
        composition: &kagari_vfx::core::timeline::Composition,
        references: &mut Vec<String>,
    ) {
        references.extend(composition.layers.iter().filter_map(|layer| {
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                Some(comp_id.clone())
            } else {
                None
            }
        }));
        for child in &composition.sub_compositions {
            collect_references(child, references);
        }
    }
    fn apply_ids(
        composition: &mut kagari_vfx::core::timeline::Composition,
        ids: &std::collections::HashMap<String, String>,
    ) {
        if let Some(id) = ids.get(&composition.id) {
            composition.id = id.clone();
        }
        for layer in &mut composition.layers {
            if let LayerType::PreComp { comp_id } = &mut layer.layer_type {
                if let Some(id) = ids.get(comp_id) {
                    *comp_id = id.clone();
                }
            }
        }
        for child in &mut composition.sub_compositions {
            apply_ids(child, ids);
        }
    }

    let Some(source) = project_compositions.get(source_index) else {
        return Err("配置するコンポジションが見つかりません。");
    };
    let mut cloned_roots = vec![source.clone()];
    let mut reserved_ids = std::collections::HashSet::new();
    for composition in project_compositions {
        collect_existing_ids(composition, &mut reserved_ids);
    }
    let mut ids = std::collections::HashMap::new();
    let mut next_id = 0;
    assign_tree_ids(
        &cloned_roots[0],
        root_id,
        &mut next_id,
        &mut ids,
        &mut reserved_ids,
    );
    let mut next_dependency = 0;
    let mut index = 0;
    while index < cloned_roots.len() {
        let mut references = Vec::new();
        collect_references(&cloned_roots[index], &mut references);
        for reference in references {
            if ids.contains_key(&reference) {
                continue;
            }
            let Some(dependency) = project_compositions
                .iter()
                .find(|composition| composition.id == reference)
            else {
                return Err("参照先コンポジションが見つからないため配置できません。");
            };
            let dependency_id = format!("{root_id}_dependency_{next_dependency}");
            next_dependency += 1;
            assign_tree_ids(
                dependency,
                &dependency_id,
                &mut next_id,
                &mut ids,
                &mut reserved_ids,
            );
            cloned_roots.push(dependency.clone());
        }
        index += 1;
    }
    for composition in &mut cloned_roots {
        apply_ids(composition, &ids);
    }
    Ok(cloned_roots)
}

pub(crate) fn replace_project_video_references(
    compositions: &mut [kagari_vfx::core::timeline::Composition],
    old_path: &str,
    asset: &kagari_vfx::core::video_import::VideoAsset,
) {
    use kagari_vfx::core::timeline::LayerType;
    for composition in compositions {
        for layer in &mut composition.layers {
            let settings = match &layer.layer_type {
                LayerType::Video {
                    source,
                    speed,
                    audio_volume,
                    ..
                } if source == old_path => Some((*speed, audio_volume.clone())),
                _ => None,
            };
            if let Some((speed, audio_volume)) = settings {
                layer.layer_type = LayerType::Video {
                    source: asset.source_path.clone(),
                    frames_dir: asset.frames_dir.clone(),
                    frame_count: asset.frame_count,
                    audio_wav: asset.audio_wav.clone(),
                    audio_volume,
                    speed,
                };
            }
        }
        replace_project_video_references(&mut composition.sub_compositions, old_path, asset);
    }
}

pub(crate) fn scene_layer_hidden_by_collapse(
    layer_by_id: &std::collections::HashMap<
        &str,
        &kagari_vfx::core::timeline::Layer,
    >,
    layer: &kagari_vfx::core::timeline::Layer,
    collapsed: &std::collections::HashSet<String>,
) -> bool {
    let mut parent_id = layer.parent_id.as_deref();
    let mut visited = std::collections::HashSet::new();
    while let Some(id) = parent_id {
        if collapsed.contains(id) || !visited.insert(id) {
            return collapsed.contains(id);
        }
        let Some(parent) = layer_by_id.get(id).copied() else {
            return false;
        };
        parent_id = parent.parent_id.as_deref();
    }
    false
}

pub(crate) fn scene_search_visible_layer_ids(
    layers: &[kagari_vfx::core::timeline::Layer],
    layer_by_id: &std::collections::HashMap<
        &str,
        &kagari_vfx::core::timeline::Layer,
    >,
    query: &str,
) -> std::collections::HashSet<String> {
    if query.is_empty() {
        return layers.iter().map(|layer| layer.id.clone()).collect();
    }

    let mut visible = std::collections::HashSet::new();
    for layer in layers.iter().filter(|layer| scene_layer_matches_query(layer, query)) {
        visible.insert(layer.id.clone());
        let mut parent_id = layer.parent_id.as_deref();
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = parent_id {
            if !visited.insert(id) {
                break;
            }
            let Some(parent) = layer_by_id.get(id).copied() else {
                break;
            };
            visible.insert(parent.id.clone());
            parent_id = parent.parent_id.as_deref();
        }
    }
    visible
}

pub(crate) fn scene_layer_matches_query(layer: &kagari_vfx::core::timeline::Layer, query: &str) -> bool {
    if layer.name.to_lowercase().contains(query) {
        return true;
    }
    let matches_text = |value: &str| value.to_lowercase().contains(query);
    let matches_kind_or_source = match layer.scene_object.as_ref() {
        Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { .. }) => matches_text("カメラ"),
        Some(kagari_vfx::core::timeline::SceneObjectRef::Light { .. }) => matches_text("ライト"),
        None => match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Solid { .. } => matches_text("平面"),
            kagari_vfx::core::timeline::LayerType::Image { path } => {
                matches_text("画像") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::Model3D { path } => {
                matches_text("3dモデル") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::Video { source, frames_dir, .. } => {
                matches_text("動画") || matches_text(source) || matches_text(frames_dir)
            }
            kagari_vfx::core::timeline::LayerType::Text { text, font_family, .. } => {
                matches_text("テキスト") || matches_text(text) || matches_text(font_family)
            }
            kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => {
                matches_text("シェイプ") || matches_text(&format!("{shape_type:?}").to_lowercase())
            }
            kagari_vfx::core::timeline::LayerType::Null => matches_text("ヌル"),
            kagari_vfx::core::timeline::LayerType::PreComp { comp_id } => {
                matches_text("プリコンポーズ") || matches_text(comp_id)
            }
            kagari_vfx::core::timeline::LayerType::Audio { path, .. } => {
                matches_text("音声") || matches_text(path)
            }
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer => {
                matches_text("調整レイヤー")
            }
            kagari_vfx::core::timeline::LayerType::Particle { .. } => {
                matches_text("パーティクル")
            }
        },
    };
    matches_kind_or_source
        || (layer.locked && matches_text("ロック"))
        || (layer.visible && matches_text("表示"))
        || (!layer.visible && matches_text("非表示"))
        || (layer.solo && matches_text("ソロ"))
        || (layer.is_shy && matches_text("シャイ"))
        || (layer.motion_blur && matches_text("モーションブラー"))
}

pub(crate) fn move_layer_keyframes_at_frame(
    layer: &mut kagari_vfx::core::timeline::Layer,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let mut changed = layer.transform.position.move_keyframe(from_frame, to_frame);
    changed |= layer
        .transform
        .anchor_point
        .move_keyframe(from_frame, to_frame);
    changed |= layer.transform.scale.move_keyframe(from_frame, to_frame);
    changed |= layer.transform.rotation.move_keyframe(from_frame, to_frame);
    changed |= layer.transform.opacity.move_keyframe(from_frame, to_frame);
    if layer.is_3d {
        changed |= layer
            .transform_3d
            .position
            .move_keyframe(from_frame, to_frame);
        changed |= layer
            .transform_3d
            .rotation
            .move_keyframe(from_frame, to_frame);
        changed |= layer.transform_3d.scale.move_keyframe(from_frame, to_frame);
    }
    if let Some(time_remap) = &mut layer.time_remap {
        changed |= time_remap.move_keyframe(from_frame, to_frame);
    }
    match &mut layer.layer_type {
        kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
            changed |= volume.move_keyframe(from_frame, to_frame);
        }
        kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
            kagari_vfx::core::timeline::ShapeType::Rectangle {
                width,
                height,
                corner_radius,
            } => {
                changed |= width.move_keyframe(from_frame, to_frame);
                changed |= height.move_keyframe(from_frame, to_frame);
                changed |= corner_radius.move_keyframe(from_frame, to_frame);
            }
            kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                changed |= width.move_keyframe(from_frame, to_frame);
                changed |= height.move_keyframe(from_frame, to_frame);
            }
            _ => {}
        },
        _ => {}
    }
    for effect in &mut layer.effects {
        for (_, parameter) in effect.effect_type.animatable_params() {
            use kagari_vfx::core::effect_params::ParamRef;
            changed |= match parameter {
                ParamRef::Scalar(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec2(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec3(track) => track.move_keyframe(from_frame, to_frame),
                ParamRef::Vec4Color(track) => track.move_keyframe(from_frame, to_frame),
            };
        }
    }
    for mask in &mut layer.masks {
        changed |= mask.path.vertices.move_keyframe(from_frame, to_frame);
        changed |= mask.feather.move_keyframe(from_frame, to_frame);
        changed |= mask.opacity.move_keyframe(from_frame, to_frame);
        changed |= mask.expansion.move_keyframe(from_frame, to_frame);
    }
    changed
}

pub(crate) fn move_camera_keyframes_at_frame(
    camera: &mut kagari_vfx::core::timeline::Camera3D,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let mut changed = false;
    for track in [
        camera.fov_animation.as_mut(),
        camera.focus_distance_animation.as_mut(),
        camera.aperture_animation.as_mut(),
        camera.dof_max_blur_animation.as_mut(),
        camera.dof_enabled_animation.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        changed |= track.move_keyframe(from_frame, to_frame);
    }
    changed
}

pub(crate) fn layer_keyframe_frames(layer: &kagari_vfx::core::timeline::Layer) -> Vec<u32> {
    let mut frames = Vec::new();
    macro_rules! add_track_frames {
        ($track:expr) => {
            if let Some(keyframes) = $track.keyframes() {
                frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
            }
        };
    }

    add_track_frames!(layer.transform.anchor_point);
    add_track_frames!(layer.transform.position);
    add_track_frames!(layer.transform.scale);
    add_track_frames!(layer.transform.rotation);
    add_track_frames!(layer.transform.opacity);
    if layer.is_3d {
        add_track_frames!(layer.transform_3d.position);
        add_track_frames!(layer.transform_3d.rotation);
        add_track_frames!(layer.transform_3d.scale);
    }
    if let Some(time_remap) = &layer.time_remap {
        add_track_frames!(time_remap);
    }
    for effect in &layer.effects {
        for (_, parameter) in effect.effect_type.animatable_params_ref() {
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Scalar(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec2(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec3(track) => {
                    add_track_frames!(track);
                }
                kagari_vfx::core::effect_params::ParamRefRef::Vec4Color(track) => {
                    add_track_frames!(track);
                }
            }
        }
    }
    for mask in &layer.masks {
        add_track_frames!(mask.path.vertices);
        add_track_frames!(mask.feather);
        add_track_frames!(mask.opacity);
        add_track_frames!(mask.expansion);
    }
    match &layer.layer_type {
        kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
            add_track_frames!(volume);
        }
        kagari_vfx::core::timeline::LayerType::Shape { shape_type, .. } => match shape_type {
            kagari_vfx::core::timeline::ShapeType::Rectangle {
                width,
                height,
                corner_radius,
            } => {
                add_track_frames!(width);
                add_track_frames!(height);
                add_track_frames!(corner_radius);
            }
            kagari_vfx::core::timeline::ShapeType::Ellipse { width, height } => {
                add_track_frames!(width);
                add_track_frames!(height);
            }
            kagari_vfx::core::timeline::ShapeType::Star {
                points,
                inner_radius,
                outer_radius,
            } => {
                add_track_frames!(points);
                add_track_frames!(inner_radius);
                add_track_frames!(outer_radius);
            }
            kagari_vfx::core::timeline::ShapeType::Polygon { sides, radius } => {
                add_track_frames!(sides);
                add_track_frames!(radius);
            }
            kagari_vfx::core::timeline::ShapeType::FreeformBezier { .. } => {}
        },
        _ => {}
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn layer_keyframe_frames_for_composition(
    composition: &kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Vec<u32> {
    let mut frames = layer_keyframe_frames(layer);
    let camera_id = layer
        .scene_object
        .as_ref()
        .and_then(|scene_object| match scene_object {
            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => Some(id.as_str()),
            _ => None,
        });
    if let Some(camera) =
        camera_id.and_then(|id| composition.cameras.iter().find(|camera| camera.id == id))
    {
        frames.extend(camera_keyframe_frames(camera));
    }
    let light_id = layer
        .scene_object
        .as_ref()
        .and_then(|scene_object| match scene_object {
            kagari_vfx::core::timeline::SceneObjectRef::Light { id } => Some(id.as_str()),
            _ => None,
        });
    if let Some(track) = light_id
        .and_then(|id| composition.lights.iter().find(|light| light.id == id))
        .and_then(|light| light.intensity_animation.as_ref())
        .and_then(|track| track.keyframes())
    {
        frames.extend(track.iter().map(|key| key.frame));
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn camera_keyframe_frames(camera: &kagari_vfx::core::timeline::Camera3D) -> Vec<u32> {
    let mut frames = Vec::new();
    for track in [
        camera.fov_animation.as_ref(),
        camera.focus_distance_animation.as_ref(),
        camera.aperture_animation.as_ref(),
        camera.dof_max_blur_animation.as_ref(),
        camera.dof_enabled_animation.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(keyframes) = track.keyframes() {
            frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
        }
    }
    frames.sort_unstable();
    frames.dedup();
    frames
}

pub(crate) fn camera_for_layer<'a>(
    composition: &'a kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Option<&'a kagari_vfx::core::timeline::Camera3D> {
    let kagari_vfx::core::timeline::SceneObjectRef::Camera { id } = layer.scene_object.as_ref()?
    else {
        return None;
    };
    composition.cameras.iter().find(|camera| camera.id == *id)
}

pub(crate) fn light_for_layer<'a>(
    composition: &'a kagari_vfx::core::timeline::Composition,
    layer: &kagari_vfx::core::timeline::Layer,
) -> Option<&'a kagari_vfx::core::timeline::Light3D> {
    let light_id = match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Light { id } => id,
        _ => return None,
    };
    composition.lights.iter().find(|light| light.id == *light_id)
}

pub(crate) fn camera_id_for_layer(layer: &kagari_vfx::core::timeline::Layer) -> Option<String> {
    match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => Some(id.clone()),
        _ => None,
    }
}

pub(crate) fn graph_targets_mut<'a>(
    composition: &'a mut kagari_vfx::core::timeline::Composition,
    layer_id: &str,
) -> Option<(
    &'a mut kagari_vfx::core::timeline::Layer,
    Option<&'a mut kagari_vfx::core::timeline::Camera3D>,
    Option<&'a mut kagari_vfx::core::timeline::Light3D>,
)> {
    let camera_id = composition
        .layers
        .iter()
        .find(|layer| layer.id == layer_id && !layer.locked)
        .and_then(camera_id_for_layer);
    let light_id = composition
        .layers
        .iter()
        .find(|layer| layer.id == layer_id && !layer.locked)
        .and_then(light_id_for_layer);
    let (layers, cameras, lights) = (
        &mut composition.layers,
        &mut composition.cameras,
        &mut composition.lights,
    );
    let layer = layers
        .iter_mut()
        .find(|layer| layer.id == layer_id && !layer.locked)?;
    let camera = camera_id.and_then(|id| cameras.iter_mut().find(|camera| camera.id == id));
    let light = light_id.and_then(|id| lights.iter_mut().find(|light| light.id == id));
    Some((layer, camera, light))
}

pub(crate) fn light_id_for_layer(layer: &kagari_vfx::core::timeline::Layer) -> Option<String> {
    match layer.scene_object.as_ref()? {
        kagari_vfx::core::timeline::SceneObjectRef::Light { id } => Some(id.clone()),
        _ => None,
    }
}

