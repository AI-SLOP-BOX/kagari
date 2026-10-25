use super::*;

impl AfterEffectsStudio {
    pub(crate) fn begin_playback(&mut self) {
        if !self.is_playing {
            let duration = self
                .project_model
                .active_composition()
                .duration_frames
                .max(1);
            if self.playback_loop {
                if let Some((start, end)) = self.playback_range {
                    let last = duration.saturating_sub(1);
                    let start = start.min(last);
                    let end = end.min(last).max(start);
                    self.current_frame = self.current_frame.clamp(start, end);
                }
            }
            if !self.playback_loop && self.current_frame >= duration.saturating_sub(1) {
                self.current_frame = 0;
                self.current_time = format_timecode(
                    self.current_frame,
                    self.project_model.active_composition().fps.max(1),
                );
            }
            self.playback_anchor = Some((std::time::Instant::now(), self.current_frame));
        }
        self.is_playing = true;
    }

    pub(crate) fn stop_playback(&mut self) {
        self.is_playing = false;
        self.playback_anchor = None;
    }

    pub(crate) fn set_playback_range_start(&mut self) {
        self.stop_playback();
        let last = self
            .project_model
            .active_composition()
            .duration_frames
            .saturating_sub(1);
        let frame = self.current_frame.min(last);
        let end = self.playback_range.map(|(_, end)| end.min(last)).unwrap_or(last);
        self.playback_range = Some((frame, frame.max(end)));
    }

    pub(crate) fn set_playback_range_end(&mut self) {
        self.stop_playback();
        let last = self
            .project_model
            .active_composition()
            .duration_frames
            .saturating_sub(1);
        let frame = self.current_frame.min(last);
        let start = self.playback_range.map(|(start, _)| start.min(last)).unwrap_or(0);
        self.playback_range = Some(if frame < start {
            (frame, frame)
        } else {
            (start, frame)
        });
    }

    pub(crate) fn update_playback_range_drag(&mut self, pointer_x: f64) {
        let Some(mode) = self.timeline_work_area_drag else {
            return;
        };
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let (timeline_start, visible_frames) = self.timeline_window(duration);
        let fraction = (pointer_x / self.timeline_width.max(1.0)).clamp(0.0, 1.0);
        let frame = ((timeline_start + fraction * visible_frames).round() as u32)
            .min(duration.saturating_sub(1));
        let (start, end) = self.playback_range.unwrap_or((0, duration.saturating_sub(1)));
        self.playback_range = Some(match mode {
            TimelineWorkAreaDragMode::Start if frame <= end => (frame, end),
            TimelineWorkAreaDragMode::Start => (frame, frame),
            TimelineWorkAreaDragMode::End if frame >= start => (start, frame),
            TimelineWorkAreaDragMode::End => (frame, frame),
        });
        self.sync_bindings_logged();
    }

    pub(crate) fn step_playhead(&mut self, direction: i32) {
        self.stop_playback();
        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        self.current_frame = if direction < 0 {
            self.current_frame.saturating_sub(1)
        } else {
            self.current_frame.saturating_add(1).min(duration.saturating_sub(1))
        };
        self.current_time = format_timecode(
            self.current_frame,
            self.project_model.active_composition().fps.max(1),
        );
    }

    pub(crate) fn advance_playback(&mut self) {
        let Some((started_at, start_frame)) = self.playback_anchor else {
            return;
        };
        let prior_frame = self.current_frame;
        let composition = self.project_model.active_composition();
        let fps = composition.fps.max(1);
        let frame_count = composition.duration_frames.max(1);
        let elapsed_frames = (started_at.elapsed().as_secs_f64() * f64::from(fps)).floor() as u32;
        let next_frame = start_frame.saturating_add(elapsed_frames);
        if self.playback_loop {
            if let Some((start, end)) = self.playback_range {
                let last = frame_count.saturating_sub(1);
                let start = start.min(last);
                let end = end.min(last).max(start);
                let range_len = end.saturating_sub(start).saturating_add(1).max(1);
                self.current_frame = start.saturating_add(next_frame.saturating_sub(start) % range_len);
            } else {
                self.current_frame = next_frame % frame_count;
            }
        } else {
            let last_frame = frame_count.saturating_sub(1);
            self.current_frame = next_frame.min(last_frame);
            if next_frame >= last_frame {
                self.stop_playback();
            }
        }
        self.current_time = format_timecode(self.current_frame, fps);
        if self.current_frame != prior_frame {
            self.sync_selected_layer_controls();
            self.sync_bindings_logged();
        }
    }

    pub(crate) fn handle_playback_action(&mut self, action_name: &str, action_value: Option<&ActionValue>) -> bool {
        match (action_name, action_value) {
            ("play_toggle", _) => {
                if self.is_playing {
                    self.stop_playback();
                } else {
                    self.begin_playback();
                }
                true
            }
            ("play_start", _) => {
                self.begin_playback();
                true
            }
            ("play_stop", _) => {
                self.stop_playback();
                true
            }
            ("step_frame_backward", _) => {
                self.step_playhead(-1);
                true
            }
            ("step_frame_forward", _) => {
                self.step_playhead(1);
                true
            }
            ("toggle_playback_loop", _) => {
                self.playback_loop = !self.playback_loop;
                true
            }
            ("set_playback_range_start", _) => {
                self.set_playback_range_start();
                true
            }
            ("set_playback_range_end", _) => {
                self.set_playback_range_end();
                true
            }
            ("work_area_start_frame_changed", Some(ActionValue::Text(value))) => {
                if let Ok(frame) = value.trim().parse::<u32>() {
                    let last = self.project_model.active_composition().duration_frames.saturating_sub(1);
                    let (_, end) = self.playback_range.unwrap_or((0, last));
                    self.stop_playback();
                    self.playback_range = Some((frame.min(last).min(end.min(last)), end.min(last)));
                }
                true
            }
            ("work_area_end_frame_changed", Some(ActionValue::Text(value))) => {
                if let Ok(frame) = value.trim().parse::<u32>() {
                    let last = self.project_model.active_composition().duration_frames.saturating_sub(1);
                    let (start, _) = self.playback_range.unwrap_or((0, last));
                    let start = start.min(last);
                    self.stop_playback();
                    self.playback_range = Some((start, frame.min(last).max(start)));
                }
                true
            }
            ("clear_playback_range", _) => {
                self.stop_playback();
                self.playback_range = None;
                true
            }
            ("play_rewind", _) => {
                self.stop_playback();
                self.current_frame = self
                    .playback_range
                    .map(|(start, _)| start)
                    .unwrap_or(0)
                    .min(
                        self.project_model
                            .active_composition()
                            .duration_frames
                            .saturating_sub(1),
                    );
                self.current_time = format_timecode(self.current_frame, self.project_fps);
                true
            }
            ("play_forward", _) => {
                self.stop_playback();
                let last = self
                    .project_model
                    .active_composition()
                    .duration_frames
                    .saturating_sub(1);
                self.current_frame = self
                    .playback_range
                    .map(|(_, end)| end)
                    .unwrap_or(last)
                    .min(last);
                self.current_time = format_timecode(self.current_frame, self.project_fps);
                true
            }
            _ => false,
        }
    }
}

