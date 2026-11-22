use super::*;

pub(crate) const UI_REDRAW_MESSAGE: &str = "__kagari_ui_redraw__";

impl AfterEffectsStudio {
    pub(crate) fn request_ui_redraw(&self) {
        if let Some(sender) = &self.app_event_sender {
            let _ = sender.send(UI_REDRAW_MESSAGE);
        }
    }

    fn drain_pending_ui_actions(&mut self) -> bool {
        let mut drained = false;
        while let Some(action) = self.ui.take_pending_event() {
            drained = true;
            if let Err(error) = self.handle_action(action) {
                eprintln!("Ame VFX action failed: {error}");
            }
        }
        if drained {
            self.request_ui_redraw();
        }
        drained
    }
}

impl App for AfterEffectsStudio {
    fn full_bleed_layout(&self) -> bool {
        true
    }

    fn on_event_loop_ready(&mut self, sender: ame_ui::app::AppEventSender) {
        ame_ui::app::App::on_event_loop_ready(&mut self.ui, sender.clone());
        self.app_event_sender = Some(sender);
    }

    fn on_task_manager_ready(&mut self, tasks: ame_ui::task::TaskManager) {
        ame_ui::app::App::on_task_manager_ready(&mut self.ui, tasks);
    }

    fn window_title(&self) -> Option<String> {
        ame_ui::app::App::window_title(&self.ui)
    }

    fn window_modified(&self) -> bool {
        self.project_is_dirty
    }

    fn state_json(&self, name: &str) -> Option<serde_json::Value> {
        ame_ui::app::App::state_json(&self.ui, name)
    }

    fn accessibility_adaptation(&self) -> ame_ui::lang::AccessibilityAdaptation {
        ame_ui::app::App::accessibility_adaptation(&self.ui)
    }

    fn canvas_accessibility_items(
        &self,
        canvas_id: &str,
    ) -> Vec<ame_ui::app::CanvasAccessibilityItem> {
        use ame_ui::app::{CanvasAccessibilityItem, CanvasItemRole};
        let comp = self.project_model.active_composition();
        match canvas_id {
            "kagari.viewport" => {
                let mut items = Vec::with_capacity(comp.layers.len() + 1);
                items.push(CanvasAccessibilityItem {
                    label: comp.name.clone(),
                    role: CanvasItemRole::Label,
                    x: 8.0,
                    y: 8.0,
                    width: 240.0,
                    height: 24.0,
                    value: Some(format!("{}x{}", comp.width, comp.height)),
                    selected: None,
                });
                for (idx, layer) in comp.layers.iter().take(64).enumerate() {
                    items.push(CanvasAccessibilityItem {
                        label: layer.name.clone(),
                        role: CanvasItemRole::Button,
                        x: 12.0,
                        y: 36.0 + idx as f32 * 24.0,
                        width: 220.0,
                        height: 22.0,
                        value: Some(format!(
                            "visible={} frames={}..={}",
                            layer.visible, layer.in_frame, layer.out_frame
                        )),
                        selected: Some(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    });
                }
                items
            }
            "kagari.timeline" => {
                let mut items = Vec::with_capacity(comp.layers.len() + 1);
                items.push(CanvasAccessibilityItem {
                    label: "Playhead".to_owned(),
                    role: CanvasItemRole::Slider,
                    x: 0.0,
                    y: 0.0,
                    width: 16.0,
                    height: 28.0,
                    value: Some(format!("frame {}", self.current_frame)),
                    selected: None,
                });
                for (idx, layer) in comp.layers.iter().take(64).enumerate() {
                    items.push(CanvasAccessibilityItem {
                        label: layer.name.clone(),
                        role: CanvasItemRole::Button,
                        x: 16.0,
                        y: 32.0 + idx as f32 * 28.0,
                        width: 260.0,
                        height: 24.0,
                        value: Some(format!("{}..={}", layer.in_frame, layer.out_frame)),
                        selected: Some(self.selected_layer_id.as_deref() == Some(layer.id.as_str())),
                    });
                }
                items
            }
            _ => Vec::new(),
        }
    }

    fn layout_motion_interpolation(&self) -> bool {
        ame_ui::app::App::layout_motion_interpolation(&self.ui)
    }

    fn drain_toasts(&mut self) -> Vec<ame_ui::notification::ToastEntry> {
        ame_ui::app::App::drain_toasts(&mut self.ui)
    }

    fn font_family(&self) -> Option<&str> {
        ame_ui::app::App::font_family(&self.ui)
    }

    fn font_profile(&self) -> Option<&ame_ui::font::FontProfile> {
        ame_ui::app::App::font_profile(&self.ui)
    }

    fn native_menus(&self) -> Vec<ame_ui::app::NativeMenu> {
        ame_ui::app::App::native_menus(&self.ui)
    }

    fn on_native_menu_action(&mut self, id: &str) {
        ame_ui::app::App::on_native_menu_action(&mut self.ui, id);
        let _ = self.drain_pending_ui_actions();
    }

    fn on_shortcut(&mut self, key: &str, modifiers: ame_ui::event::KeyModifiers) -> bool {
        let handled = ame_ui::app::App::on_shortcut(&mut self.ui, key, modifiers);
        if handled && !self.drain_pending_ui_actions() {
            self.request_ui_redraw();
        }
        handled
    }

    fn has_global_shortcuts(&self) -> bool {
        ame_ui::app::App::has_global_shortcuts(&self.ui)
    }

    fn captures_keyboard_input(&self) -> bool {
        ame_ui::app::App::captures_keyboard_input(&self.ui)
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
            "kagari.node_graph" => self.handle_node_graph_canvas_pointer(event),
            "kagari.scopes" => self.handle_scopes_canvas_pointer(event),
            "kagari.color_wheels" => self.handle_color_wheels_canvas_pointer(event),
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

    fn on_frame_metrics(&mut self, metrics: ame_ui::app::FrameMetrics) {
        if std::env::var_os("KAGARI_PROFILE_UI").is_some() {
            eprintln!(
                "kagari frame#{} nodes={} tree={:.2}ms ui_rec={:.2}ms gpu={:.2}ms in_disp={:?} in_to_redraw={:?} in_to_present={:?} redraw_to_present={:?}",
                metrics.frame_index,
                metrics.node_count,
                metrics.tree_build_ms,
                metrics.ui_record_ms,
                metrics.gpu_submit_ms,
                metrics.input_dispatch_ms,
                metrics.input_to_redraw_start_ms,
                metrics.input_to_present_ms,
                metrics.redraw_to_present_return_ms,
            );
        }
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
            // Sharp Linear/Raycast/DaVinci-inspired studio chrome (see DESIGN.md).
            density: ame_ui::theme::UiDensity::Standard,
            panel_radius: 6.0,
            panel_padding_x: 0.0,
            panel_padding_y: 0.0,
            control_radius: 5.0,
            input_radius: 5.0,
            body_font_size: 14.0,
            title_font_size: 24.0,
            caption_font_size: 13.0,
            background: Color::from_rgb8(11, 12, 14),
            surface: Color::from_rgb8(23, 26, 32),
            surface_border: Color::from_rgb8(46, 52, 64),
            text: Color::from_rgb8(245, 247, 250),
            secondary_text: Color::from_rgb8(184, 192, 206),
            placeholder_text: Color::from_rgb8(124, 134, 150),
            accent,
            accent_hover: Color::from_rgb8(255, 160, 99),
            accent_pressed: Color::from_rgb8(222, 113, 54),
            accent_text: Color::from_rgb8(255, 255, 255),
            // Navigation uses text-only secondary buttons, matching the original
            // sidebar where only the selected row owns a background surface.
            secondary_button: Color::from_rgb8(42, 48, 58),
            secondary_button_hover: Color::from_rgb8(54, 61, 74),
            secondary_button_pressed: Color::from_rgb8(66, 74, 88),
            secondary_button_text: Color::from_rgb8(236, 240, 246),
            destructive_button: Color::from_rgb8(69, 43, 45),
            destructive_button_hover: Color::from_rgb8(84, 49, 52),
            destructive_button_pressed: Color::from_rgb8(98, 52, 55),
            destructive_text: Color::from_rgb8(255, 166, 166),
            disabled_button: Color::from_rgb8(43, 47, 51),
            disabled_text: Color::from_rgb8(112, 114, 120),
            input_background: Color::from_rgb8(13, 15, 18),
            input_border: Color::from_rgb8(43, 48, 58),
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

    fn try_shared_view_tree(&mut self) -> Result<Option<ViewTree>, String> {
        ame_ui::app::App::try_shared_view_tree(&mut self.ui)
    }

    fn try_shared_view_tree_arc(
        &mut self,
    ) -> Result<Option<std::sync::Arc<ViewTree>>, String> {
        ame_ui::app::App::try_shared_view_tree_arc(&mut self.ui)
    }

    fn set_virtual_list_ranges(
        &mut self,
        ranges: &std::collections::HashMap<String, std::ops::Range<usize>>,
    ) {
        if self.workspace == Workspace::Audio {
            let mut normalized_ranges = ranges.clone();
            let source_id = self.audio_tracks_virtual_list_source_id();
            if let Some(source_id) = source_id.as_deref() {
                if let Some(range) = normalized_ranges.remove(source_id) {
                    normalized_ranges
                        .entry("audio_tracks".to_owned())
                        .or_insert(range);
                }
            }
            let audio_range_changed = self.virtual_list_ranges.get("audio_tracks")
                != normalized_ranges.get("audio_tracks");
            self.virtual_list_ranges = normalized_ranges;
            if audio_range_changed {
                let _ = self.sync_audio_tracks_binding();
            } else {
                let mut ui_ranges = self.virtual_list_ranges.clone();
                if let Some(range) = self.virtual_list_ranges.get("audio_tracks").cloned() {
                    let len = range.end.saturating_sub(range.start);
                    Self::insert_audio_tracks_ui_range(
                        &mut ui_ranges,
                        source_id.as_deref(),
                        len,
                    );
                }
                ame_ui::app::App::set_virtual_list_ranges(&mut self.ui, &ui_ranges);
            }
        } else {
            self.virtual_list_ranges.clone_from(ranges);
            ame_ui::app::App::set_virtual_list_ranges(&mut self.ui, ranges);
        }
    }

    fn on_event(&mut self, event: &AmeEvent) {
        if matches!(event, AmeEvent::AppMessage(message) if message == UI_REDRAW_MESSAGE) {
            return;
        }
        if let AmeEvent::AppMessage(message) = event {
            if let Some(path) = message.strip_prefix("open_project_path:") {
                self.request_project_open(std::path::Path::new(path));
                self.sync_bindings_logged();
                self.request_ui_redraw();
                return;
            }
        }
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
        let mut needs_redraw = false;
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
            } else if self.screen == Screen::Projects && !modifiers.alt && !modifiers.shift {
                match key.to_ascii_lowercase().as_str() {
                    "enter" | "return" => Some("project_open_selected"),
                    "delete" | "backspace" => Some("project_remove_selected"),
                    "escape" => Some("project_clear_selection"),
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
                needs_redraw = true;
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
                needs_redraw = true;
                self.sync_bindings_logged();
            }
        }
        App::on_event(&mut self.ui, event);
        let drained = self.drain_pending_ui_actions();
        if !drained
            && (needs_redraw
                || matches!(
                    event,
                    AmeEvent::ButtonClicked(_)
                        | AmeEvent::ActionInvoked { .. }
                        | AmeEvent::CheckboxChanged { .. }
                        | AmeEvent::TextFieldChanged(_, _)
                        | AmeEvent::SliderChanged { .. }
                        | AmeEvent::SplitPaneChanged { .. }
                        | AmeEvent::TableColumnWidthChanged { .. }
                        | AmeEvent::TextInput(_)
                        | AmeEvent::ImePreedit { .. }
                        | AmeEvent::ImeCommit(_)
                        | AmeEvent::NotificationRetry { .. }
                        | AmeEvent::NotificationCancel { .. }
                        | AmeEvent::FileDropped { .. }
                        | AmeEvent::TaskUpdated { .. }
                ))
        {
            self.request_ui_redraw();
        }
    }

    fn on_close_requested(&mut self) -> bool {
        if self.confirm_exit {
            self.exit_confirm_open = true;
            self.sync_bindings_logged();
            self.request_ui_redraw();
            false
        } else {
            true
        }
    }
}
