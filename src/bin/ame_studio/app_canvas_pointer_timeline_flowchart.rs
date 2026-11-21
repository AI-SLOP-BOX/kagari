use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_timeline_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
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
                            self.timeline_work_area_drag =
                                Some(if start_distance <= end_distance {
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
                let track_index = ((event.y - 22.0 + self.timeline_vertical_scroll) / 28.0).floor();
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
                    if let Some((layer_id, in_frame, out_frame, locked, keyframes, marker_frames)) =
                        layer_data
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
                        let row_top = 22.0 + track_index * 28.0 - self.timeline_vertical_scroll;
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
                self.current_frame = ((timeline_start + fraction * visible_frames).round() as u32)
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
                self.current_frame = ((timeline_start + fraction * visible_frames).round() as u32)
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
    }

    pub(crate) fn handle_flowchart_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        let composition = self.project_model.active_composition();
        let graph =
            kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(composition);
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

                    let p_dist_sq = (event.x - parent_pin_center.0).powi(2)
                        + (event.y - parent_pin_center.1).powi(2);
                    let m_dist_sq = (event.x - matte_pin_center.0).powi(2)
                        + (event.y - matte_pin_center.1).powi(2);

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
                    if event.x >= nx
                        && event.x <= nx + node_w
                        && event.y >= ny
                        && event.y <= ny + node_h
                    {
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
                        let dist_sq = (event.x - in_pin_center.0).powi(2)
                            + (event.y - in_pin_center.1).powi(2);
                        let inside_node = event.x >= *nx
                            && event.x <= nx + node_w
                            && event.y >= *ny
                            && event.y <= ny + node_h;
                        if dist_sq <= 144.0 || inside_node {
                            if let Some(target_layer) = composition.layers.get(idx) {
                                if target_layer.id != drag.source_layer_id {
                                    let target_id = target_layer.id.clone();
                                    let source_id = drag.source_layer_id.clone();
                                    let comp = self.project_model.active_composition_mut();
                                    if let Some(target) =
                                        comp.layers.iter_mut().find(|l| l.id == target_id)
                                    {
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
    }
}
