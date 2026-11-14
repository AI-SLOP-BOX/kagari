use super::*;

impl App for AfterEffectsStudio {
    fn on_event_loop_ready(&mut self, sender: ame_ui::app::AppEventSender) {
        self.app_event_sender = Some(sender);
    }

    fn asset_store(&self) -> AssetStore {
        self.assets.clone()
    }

    fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    fn on_canvas_pointer_event(&mut self, canvas_id: &str, event: CanvasPointerEvent) -> bool {
        match canvas_id {
            "kagari.graph" => self.handle_graph_canvas_pointer(event),
            "kagari.viewport" => self.handle_viewport_canvas_pointer(event),
            "kagari.timeline" => self.handle_timeline_canvas_pointer(event),
            "kagari.flowchart" => self.handle_flowchart_canvas_pointer(event),
            _ => false,
        }
    }

    fn on_canvas_scroll(&mut self, canvas_id: &str, event: CanvasScrollEvent) -> bool {
        if canvas_id != "kagari.timeline"
            || !event.x.is_finite()
            || !event.delta_x.is_finite()
            || !event.delta_y.is_finite()
        {
            return false;
        }

        let duration = self
            .project_model
            .active_composition()
            .duration_frames
            .max(1);
        let total = f64::from(duration);
        let width = self.timeline_width.max(1.0);
        let (old_start, old_span) = self.timeline_window(duration);
        let anchor_fraction = (event.x / width).clamp(0.0, 1.0);

        if event.modifiers.is_primary() && event.delta_y != 0.0 {
            let anchor_frame = old_start + anchor_fraction * old_span;
            let zoom_factor = (1.0 - event.delta_y / 400.0).clamp(0.5, 2.0);
            self.timeline_zoom = (self.timeline_zoom * zoom_factor).clamp(1.0, 64.0);
            let new_span = (total / self.timeline_zoom).clamp(1.0, total);
            self.timeline_start_frame =
                (anchor_frame - anchor_fraction * new_span).clamp(0.0, (total - new_span).max(0.0));
        } else {
            let horizontal_delta = if event.delta_x != 0.0 {
                event.delta_x
            } else if event.modifiers.shift {
                event.delta_y
            } else {
                0.0
            };
            if horizontal_delta != 0.0 {
                self.timeline_start_frame = (old_start - horizontal_delta * old_span / width)
                    .clamp(0.0, (total - old_span).max(0.0));
            }
            if !event.modifiers.shift && event.delta_y != 0.0 {
                let visible_layers = self.timeline_layers().len();
                let max_scroll = (visible_layers as f64 * 28.0
                    - (self.timeline_canvas_height - 22.0).max(0.0))
                .max(0.0);
                self.timeline_vertical_scroll =
                    (self.timeline_vertical_scroll + event.delta_y).clamp(0.0, max_scroll);
            }
        }

        self.sync_bindings_logged();
        true
    }

    fn draw_canvas(
        &mut self,
        canvas_id: &str,
        scene: &mut Scene,
        bounds: CanvasBounds,
        context: &mut CanvasContext<'_>,
        viewport: Viewport,
        tree: &ViewTree,
    ) {
        self.draw_canvas_inner(canvas_id, scene, bounds, context, viewport, tree);
    }

    fn content_max_width(&self) -> Option<f32> {
        None
    }

    fn canvas_frame_interval(&self) -> Option<std::time::Duration> {
        if self.is_playing
            || self.render_events.is_some()
            || self.render_worker.is_some()
            || self.still_export_events.is_some()
            || self.still_export_worker.is_some()
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
        let still_export_changed = self.receive_still_export_result();
        let workers_reaped = self.reap_finished_export_workers();
        let import_changed = self.receive_video_import_result();
        let proxy_changed = self.receive_layer_proxy_generation();
        if !was_rendering && self.render_refresh_ticks > 0 {
            self.render_refresh_ticks -= 1;
        }
        if (was_rendering && events_changed)
            || still_export_changed
            || import_changed
            || proxy_changed
            || workers_reaped
        {
            self.sync_bindings_logged();
        }
    }

    fn theme(&self) -> Theme {
        let accent =
            parse_hex_color(&self.accent_color).unwrap_or_else(|| Color::from_rgb8(245, 140, 75));
        (Theme {
            // Keep chrome restrained and utility-like rather than toy-rounded.
            density: ame_ui::theme::UiDensity::Standard,
            panel_radius: 9.0,
            panel_padding_x: 0.0,
            panel_padding_y: 0.0,
            control_radius: 8.0,
            input_radius: 7.0,
            body_font_size: 13.0,
            title_font_size: 20.0,
            caption_font_size: 11.0,
            background: Color::from_rgb8(13, 15, 18),
            surface: Color::from_rgb8(23, 26, 31),
            surface_border: Color::from_rgb8(42, 47, 54),
            text: Color::from_rgb8(236, 239, 243),
            secondary_text: Color::from_rgb8(154, 161, 169),
            placeholder_text: Color::from_rgb8(111, 120, 128),
            accent,
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
        })
        .with_accent(accent)
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

    fn try_shared_view_tree_arc(
        &mut self,
    ) -> Result<Option<std::sync::Arc<ViewTree>>, String> {
        ame_ui::app::App::try_shared_view_tree_arc(&mut self.ui)
    }

    fn on_event(&mut self, event: &AmeEvent) {
        if matches!(event, AmeEvent::AppMessage(message) if message == super::ROTO_ANALYSIS_READY_MESSAGE)
        {
            self.complete_roto_analysis();
            return;
        }
        if matches!(event, AmeEvent::AppMessage(message) if message == super::ROTO_PROPAGATION_READY_MESSAGE)
        {
            self.complete_roto_propagation();
            return;
        }
        if let AmeEvent::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
        } = event
        {
            let action = if !modifiers.alt && !modifiers.shift && key.eq_ignore_ascii_case("escape")
            {
                if self.exit_confirm_open {
                    Some("cancel_exit_app")
                } else if self.project_switch_confirm_open {
                    Some("cancel_project_switch")
                } else if self.composition_remove_confirm_open {
                    Some("composition_remove_cancel")
                } else if self.layer_remove_confirm_open {
                    Some("cancel_remove_selected_layer")
                } else if self.settings_open {
                    Some("close_settings")
                } else {
                    None
                }
            } else if modifiers.is_primary() {
                match key.to_ascii_lowercase().as_str() {
                    "z" if modifiers.shift => Some("redo_project"),
                    "z" => Some("undo_project"),
                    "y" => Some("redo_project"),
                    "s" if self.screen == Screen::Editor => Some("save_project"),
                    "n" => Some("new_project"),
                    "m" if self.screen == Screen::Editor => Some("go_export"),
                    "," => Some("go_settings"),
                    "q" => Some("exit_app"),
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
            let compact = *width < 1300 || *height < 760;
            let narrow = *width < 950;
            let wide = *width >= 1500 && *height >= 850;
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
