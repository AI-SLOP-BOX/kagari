use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_timeline_marker_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
        selected_row_id: Option<String>,
    ) -> bool {
        match (action_name, action_value) {
            ("timeline_zoom_in", _) => {
                self.set_timeline_zoom(2.0);
                true
            }
            ("timeline_zoom_out", _) => {
                self.set_timeline_zoom(0.5);
                true
            }
            ("timeline_zoom_fit", _) => {
                self.fit_timeline();
                true
            }
            ("timeline_pan_left", _) => {
                self.pan_timeline(-1.0);
                true
            }
            ("timeline_pan_right", _) => {
                self.pan_timeline(1.0);
                true
            }
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
                    composition
                        .markers
                        .push(kagari_vfx::core::timeline::TimelineMarker {
                            frame: self.current_frame,
                            label,
                            color: [1.0, 0.38, 0.08],
                        });
                    self.selected_marker_frame = Some(self.current_frame);
                    self.mark_project_changed();
                };
                true
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
                };
                true
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
                };
                true
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
                };
                true
            }
            ("timeline_marker_frame_changed", Some(ActionValue::Text(value))) => {
                if let (Some(old_frame), Ok(requested_frame)) =
                    (self.selected_marker_frame, value.trim().parse::<u32>())
                {
                    let duration = self
                        .project_model
                        .active_composition()
                        .duration_frames
                        .max(1);
                    let new_frame = requested_frame.min(duration - 1);
                    let composition = self.project_model.active_composition_mut();
                    if !composition
                        .markers
                        .iter()
                        .any(|marker| marker.frame == new_frame && marker.frame != old_frame)
                    {
                        if let Some(marker) = composition
                            .markers
                            .iter_mut()
                            .find(|marker| marker.frame == old_frame)
                        {
                            if marker.frame != new_frame {
                                marker.frame = new_frame;
                                self.selected_marker_frame = Some(new_frame);
                                self.mark_project_changed();
                            }
                        }
                    }
                };
                true
            }
            ("add_layer_marker", _) => {
                if let Some(layer_id) = self.selected_layer_id.as_deref() {
                    let frame = self.current_frame;
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        if layer.markers.iter().any(|marker| marker.frame == frame) {
                            self.selected_layer_marker_frame = Some(frame);
                        } else {
                            let label = format!("マーカー {}", layer.markers.len() + 1);
                            layer
                                .markers
                                .push(kagari_vfx::core::timeline::TimelineMarker {
                                    frame,
                                    label,
                                    color: [0.22, 0.72, 0.68],
                                });
                            self.selected_layer_marker_frame = Some(frame);
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("go_layer_marker", ref val) => {
                let frame_opt = match val {
                    Some(ActionValue::Text(s)) => s.parse::<u32>().ok(),
                    _ => selected_row_id.and_then(|id| id.parse::<u32>().ok()),
                };
                if let (Some(layer_id), Some(frame)) =
                    (self.selected_layer_id.as_deref(), frame_opt)
                {
                    if self
                        .project_model
                        .active_composition()
                        .layers
                        .iter()
                        .find(|layer| layer.id == layer_id)
                        .is_some_and(|layer| {
                            layer.markers.iter().any(|marker| marker.frame == frame)
                        })
                    {
                        self.selected_layer_marker_frame = Some(frame);
                        self.current_frame = frame.min(
                            self.project_model
                                .active_composition()
                                .duration_frames
                                .saturating_sub(1),
                        );
                        self.current_time = format_timecode(self.current_frame, self.project_fps);
                    }
                };
                true
            }
            ("remove_layer_marker", _) => {
                if let (Some(layer_id), Some(frame)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_layer_marker_frame,
                ) {
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        let old_len = layer.markers.len();
                        layer.markers.retain(|marker| marker.frame != frame);
                        if layer.markers.len() != old_len {
                            self.selected_layer_marker_frame = None;
                            self.mark_project_changed();
                        }
                    }
                };
                true
            }
            ("layer_marker_label_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(frame)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_layer_marker_frame,
                ) {
                    let label = value.trim().chars().take(128).collect::<String>();
                    if !label.is_empty() {
                        if let Some(marker) = self
                            .project_model
                            .active_composition_mut()
                            .layers
                            .iter_mut()
                            .find(|layer| layer.id == layer_id && !layer.locked)
                            .and_then(|layer| {
                                layer
                                    .markers
                                    .iter_mut()
                                    .find(|marker| marker.frame == frame)
                            })
                        {
                            if marker.label != label {
                                marker.label = label;
                                self.mark_project_changed();
                            }
                        }
                    }
                };
                true
            }
            ("layer_marker_frame_changed", Some(ActionValue::Text(value))) => {
                if let (Some(layer_id), Some(old_frame), Ok(requested_frame)) = (
                    self.selected_layer_id.as_deref(),
                    self.selected_layer_marker_frame,
                    value.trim().parse::<u32>(),
                ) {
                    let duration = self
                        .project_model
                        .active_composition()
                        .duration_frames
                        .max(1);
                    let new_frame = requested_frame.min(duration - 1);
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id && !layer.locked)
                    {
                        if !layer
                            .markers
                            .iter()
                            .any(|marker| marker.frame == new_frame && marker.frame != old_frame)
                        {
                            if let Some(marker) = layer
                                .markers
                                .iter_mut()
                                .find(|marker| marker.frame == old_frame)
                            {
                                if marker.frame != new_frame {
                                    marker.frame = new_frame;
                                    self.selected_layer_marker_frame = Some(new_frame);
                                    self.mark_project_changed();
                                }
                            }
                        }
                    }
                };
                true
            }
            _ => false,
        }
    }
}
