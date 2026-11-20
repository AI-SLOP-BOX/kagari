use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_graph_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let (timeline_start, visible_frames) = self.timeline_window(duration);
        let plot_left = 14.0;
        let plot_right = (self.graph_canvas_width - 14.0).max(plot_left + 1.0);
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
                    if event.button == Some(ame_ui::event::PointerButton::Secondary)
                        && !layer_locked
                    {
                        let comp = self.project_model.active_composition_mut();
                        if let Some((target_layer, target_camera, target_light)) =
                            graph_targets_mut(comp, layer_id)
                        {
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
                    snap_points.extend(
                        composition
                            .markers
                            .iter()
                            .map(|marker| marker.frame.min(duration.saturating_sub(1))),
                    );
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
                let Some(drag) = self.graph_key_drag.clone() else {
                    return false;
                };
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
                    let snap_distance = (8.0 * visible_frames / self.graph_canvas_width.max(1.0))
                        .ceil()
                        .max(1.0) as u32;
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
                    if let Some((layer, camera, light)) = graph_targets_mut(
                        self.project_model.active_composition_mut(),
                        &drag.layer_id,
                    ) {
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
    }

    pub(crate) fn handle_viewport_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        let Some((left, top, scale_x, scale_y)) = self.viewport_mapping else {
            return false;
        };
        if scale_x <= 0.0 || scale_y <= 0.0 {
            return false;
        }
        let point = [(event.x - left) / scale_x, (event.y - top) / scale_y];
        let roto_samples = event
            .coalesced_samples
            .iter()
            .map(|sample| {
                (
                    [(sample.x - left) / scale_x, (sample.y - top) / scale_y],
                    sample.pressure,
                )
            })
            .collect::<Vec<_>>();

        // Interactive A/B wipe split drag when wipe comparison is active
        if self.viewport_wipe_enabled {
            let comp_w = f64::from(self.project_model.active_composition().width.max(1));
            let split_screen_x = left + comp_w * self.viewport_wipe_split.clamp(0.05, 0.95) * scale_x;
            match event.phase {
                CanvasPointerPhase::Down if !event.modifiers.alt => {
                    if (event.x - split_screen_x).abs() <= 14.0 {
                        self.viewport_wipe_dragging = true;
                        return true;
                    }
                }
                CanvasPointerPhase::Moved if self.viewport_wipe_dragging => {
                    self.viewport_wipe_split = (point[0] / comp_w).clamp(0.05, 0.95);
                    return true;
                }
                CanvasPointerPhase::Up if self.viewport_wipe_dragging => {
                    self.viewport_wipe_dragging = false;
                    return true;
                }
                _ => {}
            }
        }

        // Interactive 4-corner drag gizmo for CornerPin / MeshWarp on the selected layer
        if !event.modifiers.alt {
            let (comp_w, comp_h) = {
                let comp = self.project_model.active_composition();
                (
                    f64::from(comp.width.max(1)),
                    f64::from(comp.height.max(1)),
                )
            };
            match event.phase {
                CanvasPointerPhase::Down => {
                    if let Some(layer_id) = self.selected_layer_id.as_deref() {
                        let comp = self.project_model.active_composition();
                        if let Some(layer) = comp.layers.iter().find(|l| l.id == layer_id && !l.locked) {
                            for eff in layer.effects.iter().filter(|e| e.enabled) {
                                let corners = match &eff.effect_type {
                                    kagari_vfx::core::timeline::EffectType::CornerPin {
                                        top_left,
                                        top_right,
                                        bottom_right,
                                        bottom_left,
                                    }
                                    | kagari_vfx::core::timeline::EffectType::MeshWarp {
                                        top_left,
                                        top_right,
                                        bottom_right,
                                        bottom_left,
                                    } => Some([
                                        top_left.evaluate(self.current_frame),
                                        top_right.evaluate(self.current_frame),
                                        bottom_right.evaluate(self.current_frame),
                                        bottom_left.evaluate(self.current_frame),
                                    ]),
                                    _ => None,
                                };
                                if let Some(pts) = corners {
                                    for (idx, p) in pts.iter().enumerate() {
                                        let sx = left + f64::from(p[0]) * comp_w * scale_x;
                                        let sy = top + f64::from(p[1]) * comp_h * scale_y;
                                        let dx = event.x - sx;
                                        let dy = event.y - sy;
                                        if dx * dx + dy * dy <= 196.0 {
                                            self.corner_pin_drag = Some(idx);
                                            return true;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
                CanvasPointerPhase::Moved if self.corner_pin_drag.is_some() => {
                    if let (Some(idx), Some(layer_id)) =
                        (self.corner_pin_drag, self.selected_layer_id.clone())
                    {
                        let nx = (point[0] / comp_w).clamp(-0.5, 1.5) as f32;
                        let ny = (point[1] / comp_h).clamp(-0.5, 1.5) as f32;
                        if let Some(layer) = self
                            .project_model
                            .active_composition_mut()
                            .layers
                            .iter_mut()
                            .find(|l| l.id == layer_id && !l.locked)
                        {
                            for eff in layer.effects.iter_mut().filter(|e| e.enabled) {
                                match &mut eff.effect_type {
                                    kagari_vfx::core::timeline::EffectType::CornerPin {
                                        top_left,
                                        top_right,
                                        bottom_right,
                                        bottom_left,
                                    }
                                    | kagari_vfx::core::timeline::EffectType::MeshWarp {
                                        top_left,
                                        top_right,
                                        bottom_right,
                                        bottom_left,
                                    } => {
                                        let target = match idx {
                                            0 => top_left,
                                            1 => top_right,
                                            2 => bottom_right,
                                            _ => bottom_left,
                                        };
                                        *target = kagari_vfx::core::timeline::Animatable::new_constant([nx, ny]);
                                        self.mark_project_changed();
                                        return true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    return true;
                }
                CanvasPointerPhase::Up | CanvasPointerPhase::Cancelled
                    if self.corner_pin_drag.is_some() =>
                {
                    self.corner_pin_drag = None;
                    return true;
                }
                _ => {}
            }
        }

        // 3D Camera interactive navigation: Alt + Primary (Orbit), Alt + Secondary (Pan), Alt + Shift / Tertiary (Dolly)
        if event.phase == CanvasPointerPhase::Down && event.modifiers.alt {
            let comp = self.project_model.active_composition();
            let cam = comp.resolve_camera();
            let cam_pos = cam.transform.position.evaluate(self.current_frame);
            let cam_rot = cam.transform.rotation.evaluate(self.current_frame);
            let mode = if event.modifiers.shift
                || event.button == Some(ame_ui::event::PointerButton::Middle)
            {
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
                            .filter(|(_, distance)| *distance <= 12.0)
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
            if let Some(layer_id) = self.selected_layer_id.clone() {
                let corner_hit = {
                    let comp = self.project_model.active_composition();
                    comp.layers
                        .iter()
                        .find(|l| l.id == layer_id && !l.locked && l.visible && l.is_active(self.current_frame))
                        .and_then(|layer| {
                            let pos = layer.transform.position.evaluate(self.current_frame);
                            let anchor = layer.transform.anchor_point.evaluate(self.current_frame);
                            let rot = f64::from(layer.transform.rotation.evaluate(self.current_frame)).to_radians();
                            let scale = layer.transform.scale.evaluate(self.current_frame);
                            let size = layer.bounding_size();
                            let lw = f64::from(size[0]);
                            let lh = f64::from(size[1]);
                            let (sin_r, cos_r) = rot.sin_cos();
                            let sx = f64::from(scale[0]) / 100.0;
                            let sy = f64::from(scale[1]) / 100.0;
                            let corners = [
                                (-f64::from(anchor[0]), -f64::from(anchor[1])),
                                (lw - f64::from(anchor[0]), -f64::from(anchor[1])),
                                (lw - f64::from(anchor[0]), lh - f64::from(anchor[1])),
                                (-f64::from(anchor[0]), lh - f64::from(anchor[1])),
                            ];
                            let hit = corners.iter().any(|&(lx, ly)| {
                                let rx = (lx * sx) * cos_r - (ly * sy) * sin_r;
                                let ry = (lx * sx) * sin_r + (ly * sy) * cos_r;
                                let cx = left + (f64::from(pos[0]) + rx) * scale_x;
                                let cy = top + (f64::from(pos[1]) + ry) * scale_y;
                                (event.x - cx).hypot(event.y - cy) <= 12.0
                            });
                            hit.then_some((pos, scale))
                        })
                };
                if let Some((pos, scale)) = corner_hit {
                    self.viewport_scale_drag = Some((layer_id, pos, scale, point));
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
                    let factor = if event.button == Some(ame_ui::event::PointerButton::Secondary) {
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
                "RotoBrush" if !event.space_pressed => {
                    self.begin_roto_stroke_from_event(
                        point,
                        event.pressure,
                        &roto_samples,
                        event.modifiers.alt,
                    );
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
                    let (lx, ly) = if let Some(norm) =
                        kagari_vfx::core::timeline::project_point_to_screen_at_frame(
                            active_cam,
                            l_pos,
                            src_w,
                            src_h,
                            self.current_frame,
                        ) {
                        (
                            f64::from(norm[0]) * f64::from(src_w),
                            f64::from(norm[1]) * f64::from(src_h),
                        )
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
                            let position = layer.transform.position.evaluate(self.current_frame);
                            let dx = f64::from(position[0]) - point[0];
                            let dy = f64::from(position[1]) - point[1];
                            let rotation =
                                f64::from(layer.transform.rotation.evaluate(self.current_frame))
                                    .to_radians();
                            let (sin, cos) = rotation.sin_cos();
                            let local_x = dx * cos + dy * sin;
                            let local_y = -dx * sin + dy * cos;
                            let size = layer.bounding_size();
                            let transform_scale =
                                layer.transform.scale.evaluate(self.current_frame);
                            let half_width =
                                (f64::from(size[0]) * f64::from(transform_scale[0].abs()) / 200.0)
                                    .max(1.0)
                                    + hit_padding;
                            let half_height =
                                (f64::from(size[1]) * f64::from(transform_scale[1].abs()) / 200.0)
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
                if self.roto_live_stroke.is_some() {
                    self.extend_roto_stroke_from_event(point, event.pressure, &roto_samples);
                    return true;
                }
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
                            cam.transform
                                .rotation
                                .set_value_at_frame(self.current_frame, new_rot);
                        }
                        CameraNavMode::Pan => {
                            // Pan camera along local screen X and Y
                            let new_pos = [
                                drag.start_pos[0] - dx * 1.5,
                                drag.start_pos[1] - dy * 1.5,
                                drag.start_pos[2],
                            ];
                            cam.transform
                                .position
                                .set_value_at_frame(self.current_frame, new_pos);
                        }
                        CameraNavMode::Dolly => {
                            // Dolly camera along depth (Z axis)
                            let new_pos = [
                                drag.start_pos[0],
                                drag.start_pos[1],
                                drag.start_pos[2] + dy * 3.0,
                            ];
                            cam.transform
                                .position
                                .set_value_at_frame(self.current_frame, new_pos);
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
                if let Some((ref layer_id, pos, start_scale, start_point)) =
                    self.viewport_scale_drag.clone()
                {
                    let start_dist = (start_point[0] - f64::from(pos[0]))
                        .hypot(start_point[1] - f64::from(pos[1]))
                        .max(1.0);
                    let cur_dist = (point[0] - f64::from(pos[0]))
                        .hypot(point[1] - f64::from(pos[1]))
                        .max(1.0);
                    let ratio = (cur_dist / start_dist) as f32;
                    let next_scale = [
                        (start_scale[0] * ratio).clamp(1.0, 2000.0),
                        (start_scale[1] * ratio).clamp(1.0, 2000.0),
                    ];
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|l| l.id == *layer_id && !l.locked)
                    {
                        layer
                            .transform
                            .scale
                            .set_value_at_frame(self.current_frame, next_scale);
                        self.mark_project_changed();
                        self.sync_selected_layer_controls();
                        self.sync_bindings_logged();
                    }
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
                        light
                            .position
                            .set_value_at_frame(self.current_frame, new_pos_3d);
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
                if self.roto_live_stroke.is_some() {
                    if event.phase == CanvasPointerPhase::Up {
                        self.extend_roto_stroke_from_event(point, event.pressure, &roto_samples);
                        self.finish_roto_stroke(None, None);
                    } else {
                        self.roto_live_stroke = None;
                        self.roto_status = "ストロークをキャンセルしました".to_owned();
                        self.sync_bindings_logged();
                    }
                    return true;
                }
                self.viewport_drag = None;
                self.viewport_scale_drag = None;
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
    }
}
