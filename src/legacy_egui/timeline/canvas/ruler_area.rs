use super::super::pending_actions::TimelineActions;
use super::{layout, ruler_draw};
use crate::core::{frame_cache::FrameCache, timeline::Composition};
use crate::ui::timeline::snapping::maybe_snap_frame;
use crate::ui::{notification::ToastManager, theme::colors};
use eframe::egui;

pub(super) struct RulerAreaState<'a> {
    pub frame_cache: &'a FrameCache,
    pub work_area_in: &'a mut Option<u32>,
    pub work_area_out: &'a mut Option<u32>,
    pub selected_layer_idx: Option<usize>,
    pub snap_to_keyframes: bool,
    pub timeline_view_start: &'a mut u32,
    pub timeline_zoom: &'a mut f32,
    pub global_shy_active: &'a mut bool,
    pub toasts: &'a mut ToastManager,
    pub actions: &'a mut TimelineActions,
    pub comp: &'a mut Composition,
    pub current_frame: &'a mut u32,
    pub total_frames: u32,
    pub keyframe_frames: &'a [u32],
    pub project_changed: &'a mut bool,
}

pub(super) fn draw(ui: &mut egui::Ui, state: &mut RulerAreaState<'_>) -> (f32, f32) {
    let current_frame = &mut *state.current_frame;
    let total_frames = state.total_frames;
    let comp = &mut *state.comp;
    let total_w = ui.available_width();
    let left_pane_width_id = egui::Id::new("timeline_left_pane_width_v2");
    let default_left_pane_w = crate::ui::theme::layout::TIMELINE_LEFT_PANE_WIDTH;
    let requested_left_pane_w = ui
        .ctx()
        .data_mut(|data| data.get_persisted::<f32>(left_pane_width_id))
        .unwrap_or(default_left_pane_w);
    let initial_columns =
        layout::TimelineColumns::new(total_w, requested_left_pane_w, ui.spacing().item_spacing.x);
    let mut left_pane_w = initial_columns.left_width;
    let max_left_pane_w = initial_columns.max_left_width;

    // ── Work Area (In / Out) Handles Bar ──
    ui.horizontal(|ui| {
        let divider_rect = egui::Rect::from_min_size(
            egui::pos2(
                ui.max_rect().left() + left_pane_w - 7.0,
                ui.cursor().top() - 4.0,
            ),
            egui::vec2(14.0, 26.0),
        );
        let divider_response = ui.interact(
            divider_rect,
            left_pane_width_id.with("divider"),
            egui::Sense::drag(),
        );
        let divider_color = if divider_response.hovered() || divider_response.dragged() {
            colors::BORDER_ACTIVE
        } else {
            colors::BORDER_SUBTLE
        };
        if divider_response.hovered() || divider_response.dragged() {
            ui.painter()
                .rect_filled(divider_rect, 3.0, colors::BG_HOVER);
        }
        ui.painter().line_segment(
            [divider_rect.center_top(), divider_rect.center_bottom()],
            egui::Stroke::new(1.0_f32, divider_color),
        );
        if divider_response.hovered() || divider_response.dragged() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
        if divider_response.dragged() || divider_response.drag_stopped() {
            if let Some(pointer) = divider_response.interact_pointer_pos() {
                left_pane_w = (pointer.x - ui.max_rect().left())
                    .clamp(initial_columns.min_left_width, max_left_pane_w);
                ui.ctx().data_mut(|data| {
                    data.insert_persisted(left_pane_width_id, left_pane_w);
                });
            }
        }
        divider_response.on_hover_text("ドラッグしてレイヤー列の幅を調整");

        let columns =
            layout::TimelineColumns::new(total_w, left_pane_w, ui.spacing().item_spacing.x);

        ui.allocate_ui(
            egui::vec2(left_pane_w, layout::COLUMN_HEADER_HEIGHT),
            |ui| {
                ui.set_min_width(left_pane_w);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("レイヤー")
                            .small()
                            .strong()
                            .color(colors::TEXT_SECONDARY),
                    )
                    .on_hover_text("B：作業範囲の開始点、N：終了点を現在位置に設定");

                    if ui
                        .selectable_label(*state.global_shy_active, "シャイ")
                        .on_hover_text("シャイレイヤーの表示／非表示を切り替えます")
                        .clicked()
                    {
                        *state.global_shy_active = !*state.global_shy_active;
                    }

                    let is_w_hovered = ui.rect_contains_pointer(ui.max_rect());
                    if state.work_area_in.is_some() || state.work_area_out.is_some() {
                        let w_in_frame = state.work_area_in.unwrap_or(0);
                        let w_out_frame = state.work_area_out.unwrap_or(total_frames);
                        ui.weak(format!("範囲 {}–{}f", w_in_frame, w_out_frame));
                    }

                    if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::B)) {
                        *state.work_area_in = Some(*current_frame);
                        log::info!("Set Work Area In point to frame {}", *current_frame);
                    }
                    if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::N)) {
                        *state.work_area_out = Some(*current_frame);
                        log::info!("Set Work Area Out point to frame {}", *current_frame);
                    }
                });
            },
        );

        let avail_w = ui.available_width().max(0.0).min(columns.time_width);
        let (ruler_rect, ruler_response) = ui.allocate_exact_size(
            egui::vec2(avail_w, layout::COLUMN_HEADER_HEIGHT),
            egui::Sense::click_and_drag(),
        );

        // Ruler scrub cursor feedback
        if ruler_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        ui.painter().rect_filled(ruler_rect, 0.0, colors::BG_DARK);

        let zoom_span = (total_frames as f32 / (*state.timeline_zoom).max(0.01)).max(10.0) as u32;
        let playhead_moved = ui.ctx().data_mut(|data| {
            let id = egui::Id::new("timeline_last_playhead_frame");
            let previous = data.get_temp::<u32>(id);
            data.insert_temp(id, *current_frame);
            previous.is_some_and(|frame| frame != *current_frame)
        });
        // Preserve user-panned/zoomed views while idle. Recenter only when
        // playback or a scrub actually moves the playhead outside the view.
        if playhead_moved
            && (*current_frame < *state.timeline_view_start
                || *current_frame >= (*state.timeline_view_start).saturating_add(zoom_span))
        {
            *state.timeline_view_start = current_frame
                .saturating_sub(zoom_span / 2)
                .min(total_frames.saturating_sub(zoom_span));
        }
        let start_frame = *state.timeline_view_start;

        crate::ui::timeline::ruler_bar::draw_ram_ruler(
            state.frame_cache,
            comp,
            ui,
            total_frames,
            ruler_rect,
            start_frame,
            zoom_span,
        );

        // Work area highlighted bar
        let wa_in = state.work_area_in.unwrap_or(0);
        let wa_out = state.work_area_out.unwrap_or(total_frames);
        let mut wa_drag_active = false;
        let has_explicit_work_area = state.work_area_in.is_some() || state.work_area_out.is_some();
        if has_explicit_work_area
            && wa_out > wa_in
            && wa_out >= start_frame
            && wa_in <= start_frame + zoom_span
        {
            let norm_in = (wa_in.saturating_sub(start_frame)) as f32 / zoom_span as f32;
            let norm_out = (wa_out.saturating_sub(start_frame)) as f32 / zoom_span as f32;
            let wa_rect = egui::Rect::from_min_max(
                egui::pos2(
                    ruler_rect.left() + norm_in * ruler_rect.width(),
                    ruler_rect.top() + 2.0,
                ),
                egui::pos2(
                    ruler_rect.left() + norm_out * ruler_rect.width(),
                    ruler_rect.bottom() - 2.0,
                ),
            );
            ui.painter()
                .rect_filled(wa_rect, 2.0, colors::TIMELINE_SELECTION);
            ui.painter().rect_stroke(
                wa_rect,
                2.0,
                egui::Stroke::new(1.0_f32, colors::BORDER_ACTIVE),
            );

            // ── Work-area edge handles: drag to set In/Out points ──
            const WA_HW: f32 = 5.0;
            let wa_in_handle = egui::Rect::from_min_size(
                egui::pos2(wa_rect.left() - WA_HW * 0.5, ruler_rect.top()),
                egui::vec2(WA_HW, ruler_rect.height()),
            );
            let wa_out_handle = egui::Rect::from_min_size(
                egui::pos2(wa_rect.right() - WA_HW * 0.5, ruler_rect.top()),
                egui::vec2(WA_HW, ruler_rect.height()),
            );
            let wa_in_resp = ui.interact(
                wa_in_handle,
                egui::Id::new("wa_in_handle"),
                egui::Sense::drag(),
            );
            let wa_out_resp = ui.interact(
                wa_out_handle,
                egui::Id::new("wa_out_handle"),
                egui::Sense::drag(),
            );
            let wa_frame_from_pointer = |resp: &egui::Response| -> Option<u32> {
                resp.interact_pointer_pos().map(|p| {
                    let norm = ((p.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                    start_frame.saturating_add((norm * zoom_span as f32).round() as u32)
                })
            };
            if wa_in_resp.dragged() {
                wa_drag_active = true;
                if let Some(f) = wa_frame_from_pointer(&wa_in_resp) {
                    *state.work_area_in = Some(
                        f.min(
                            state
                                .work_area_out
                                .unwrap_or(total_frames)
                                .saturating_sub(1),
                        ),
                    );
                    crate::core::frame_cache::bump_version();
                }
            }
            if wa_out_resp.dragged() {
                wa_drag_active = true;
                if let Some(f) = wa_frame_from_pointer(&wa_out_resp) {
                    *state.work_area_out = Some(f.max(state.work_area_in.unwrap_or(0) + 1));
                    crate::core::frame_cache::bump_version();
                }
            }
            if wa_in_resp.hovered() || wa_out_resp.hovered() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
            }
        }

        ruler_draw::draw_time_ruler(
            ui,
            ruler_rect,
            start_frame,
            zoom_span,
            comp.fps,
            *current_frame,
        );

        // ── Ctrl/Cmd+drag on ruler: rubber-band Work Area definition ──
        let cmd_mod = ui.input(|i| i.modifiers.command);
        if !wa_drag_active && cmd_mod && ruler_response.dragged() {
            if let Some(pos) = ruler_response.interact_pointer_pos() {
                let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                let drag_id = egui::Id::new("wa_rubber_start");
                let anchor_f = ui.ctx().data_mut(|d| {
                    let cur = d.get_temp::<u32>(drag_id);
                    if cur.is_none() {
                        d.insert_temp(drag_id, raw_f);
                    }
                    cur
                });
                if let Some(s) = anchor_f {
                    *state.work_area_in = Some(s.min(raw_f));
                    *state.work_area_out = Some(s.max(raw_f).saturating_add(1));
                }
            }
        }
        if ruler_response.drag_stopped() {
            ui.ctx()
                .data_mut(|d| d.remove_temp::<u32>(egui::Id::new("wa_rubber_start")));
        }

        if ruler_response.double_clicked() && !cmd_mod {
            if let Some(pos) = ruler_response.interact_pointer_pos() {
                let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                comp.markers.push(crate::core::timeline::TimelineMarker {
                    frame: raw_f,
                    label: format!("Marker {}", comp.markers.len() + 1),
                    color: [0.35, 0.75, 1.0],
                });
                *state.project_changed = true;
                state
                    .toasts
                    .info(format!("Comp marker added at frame {}", raw_f));
            }
        }

        let ruler_scrub_response = ui.interact(
            ruler_rect,
            egui::Id::new("timeline_ruler_scrub"),
            egui::Sense::click_and_drag(),
        );
        let (pointer_pos, primary_pressed, primary_down, primary_released) = ui.input(|input| {
            (
                input
                    .pointer
                    .interact_pos()
                    .or_else(|| input.pointer.hover_pos()),
                input.pointer.button_pressed(egui::PointerButton::Primary),
                input.pointer.button_down(egui::PointerButton::Primary),
                input.pointer.button_released(egui::PointerButton::Primary),
            )
        });
        let scrub_id = egui::Id::new("timeline_ruler_scrub_active");
        if primary_pressed && pointer_pos.is_some_and(|pos| ruler_rect.contains(pos)) {
            ui.ctx().data_mut(|data| data.insert_temp(scrub_id, true));
        }
        let scrub_active = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(scrub_id).unwrap_or(false));
        let should_scrub = !wa_drag_active
            && !cmd_mod
            && pointer_pos.is_some_and(|pos| ruler_rect.contains(pos))
            && ((scrub_active && (primary_down || primary_released))
                || ruler_scrub_response.clicked()
                || ruler_scrub_response.dragged());
        if should_scrub {
            if let Some(pos) = pointer_pos {
                let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                *current_frame =
                    maybe_snap_frame(raw_f, state.snap_to_keyframes, state.keyframe_frames);
            }
        }
        if primary_released || (scrub_active && !primary_down) {
            ui.ctx().data_mut(|data| data.remove_temp::<bool>(scrub_id));
        }

        // ── Ruler context menu (AE parity) ──
        ruler_response.context_menu(|ui| {
            if ui.button("Add Composition Marker at Playhead").clicked() {
                comp.markers.push(crate::core::timeline::TimelineMarker {
                    frame: *current_frame,
                    label: format!("Marker {}", comp.markers.len() + 1),
                    color: [0.35, 0.75, 1.0],
                });
                *state.project_changed = true;
                state
                    .toasts
                    .info(format!("Comp marker at frame {}", current_frame));
                ui.close_menu();
            }
            if !comp.markers.is_empty() && ui.button("Clear All Composition Markers").clicked() {
                comp.markers.clear();
                *state.project_changed = true;
                ui.close_menu();
            }
            if let Some(layer_idx) = state.selected_layer_idx {
                if ui.button("Split Selected Layer at Playhead").clicked() {
                    state.actions.pending_split_layer = Some(layer_idx);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("Zoom to Work Area").clicked() {
                let w_in = state.work_area_in.unwrap_or(0);
                let w_out = state.work_area_out.unwrap_or(total_frames).max(w_in + 1);
                let span = (w_out - w_in).max(10);
                *state.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                *state.timeline_view_start = w_in;
                ui.close_menu();
            }
            if ui.button("Set Comp Duration to Work Area End").clicked() {
                let w_out = state.work_area_out.unwrap_or(total_frames);
                if w_out > 0 && w_out <= total_frames {
                    state.actions.pending_duration = Some(w_out);
                    state
                        .toasts
                        .info(format!("Comp duration set to {} frames", w_out));
                }
                ui.close_menu();
            }
            if ui
                .button("Trim Comp to Work Area")
                .on_hover_text("Set comp duration to work area and trim layers beyond it")
                .clicked()
            {
                let w_in = state.work_area_in.unwrap_or(0);
                let w_out = state.work_area_out.unwrap_or(total_frames);
                if w_out > w_in {
                    state.actions.pending_duration = Some(w_out);
                    state.actions.pending_trim_work_area = Some((w_in, w_out));
                    state
                        .toasts
                        .info(format!("Trimmed to work area: {}f–{}f", w_in, w_out));
                }
                ui.close_menu();
            }
            if ui.button("Reset Work Area to Full Comp").clicked() {
                *state.work_area_in = None;
                *state.work_area_out = None;
                ui.close_menu();
            }
        });

        // ⌨️ B / N Keyboard Shortcuts for Work Area In / Out
        if ui.input(|i| i.key_pressed(egui::Key::B)) {
            *state.work_area_in = Some(*current_frame);
            state
                .toasts
                .info(format!("Set Work Area Start at frame {}", current_frame));
        }
        if ui.input(|i| i.key_pressed(egui::Key::N)) {
            *state.work_area_out = Some(*current_frame);
            state
                .toasts
                .info(format!("Set Work Area End at frame {}", current_frame));
        }
        // Timeline Zoom Shortcuts (= / -), anchored on the playhead so the
        // frame under it stays visually in place while zooming.
        {
            // Returns the new window start that keeps the playhead stationary.
            let view_start = *state.timeline_view_start;
            let anchored_start = |old_zoom: f32, new_zoom: f32| -> u32 {
                let old_span = (total_frames as f32 / old_zoom.max(0.01)).max(10.0) as u32;
                let new_span = (total_frames as f32 / new_zoom.max(0.01)).max(10.0) as u32;
                // The playhead's offset from the window's left edge scales with
                // the span ratio, keeping the playhead stationary on screen.
                let head_offset = current_frame.saturating_sub(view_start);
                let new_head_offset =
                    ((head_offset as f32) * (new_span as f32 / old_span.max(1) as f32)) as u32;
                current_frame
                    .saturating_sub(new_head_offset)
                    .min(total_frames.saturating_sub(new_span))
            };
            let pointer_anchored_start =
                |old_zoom: f32, old_start: u32, new_zoom: f32, pointer_x: f32| -> u32 {
                    let old_span = (total_frames as f32 / old_zoom.max(0.01)).max(10.0) as u32;
                    let new_span = (total_frames as f32 / new_zoom.max(0.01)).max(10.0) as u32;
                    let pointer_fraction = ((pointer_x - ruler_rect.left())
                        / ruler_rect.width().max(1.0))
                    .clamp(0.0, 1.0);
                    let frame_under_pointer = old_start as f32 + pointer_fraction * old_span as f32;
                    ((frame_under_pointer - pointer_fraction * new_span as f32)
                        .round()
                        .max(0.0) as u32)
                        .min(total_frames.saturating_sub(new_span))
                };
            if ui.input(|i| i.key_pressed(egui::Key::Equals)) && *state.timeline_zoom < 20.0 {
                let new_zoom = (*state.timeline_zoom * 1.25).min(20.0);
                *state.timeline_view_start = anchored_start(*state.timeline_zoom, new_zoom);
                *state.timeline_zoom = new_zoom;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Minus)) && *state.timeline_zoom > 0.1 {
                let new_zoom = (*state.timeline_zoom / 1.25).max(0.1);
                *state.timeline_view_start = anchored_start(*state.timeline_zoom, new_zoom);
                *state.timeline_zoom = new_zoom;
            }
            if ruler_response.hovered() {
                let (scroll, command, pointer_x) = ui.input(|i| {
                    (
                        i.raw_scroll_delta.y,
                        i.modifiers.command,
                        i.pointer.hover_pos().map(|pos| pos.x),
                    )
                });
                if scroll != 0.0 {
                    if command {
                        let max_start = total_frames.saturating_sub(zoom_span);
                        let pan_frames = (-scroll * zoom_span as f32 * 0.08).round() as i64;
                        *state.timeline_view_start =
                            (*state.timeline_view_start as i64 + pan_frames)
                                .clamp(0, max_start as i64) as u32;
                    } else if let Some(pointer_x) = pointer_x {
                        let factor = if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 };
                        let new_zoom = (*state.timeline_zoom * factor).clamp(0.1, 20.0);
                        if new_zoom != *state.timeline_zoom {
                            *state.timeline_view_start = pointer_anchored_start(
                                *state.timeline_zoom,
                                *state.timeline_view_start,
                                new_zoom,
                                pointer_x,
                            );
                            *state.timeline_zoom = new_zoom;
                        }
                    }
                }
            }
        }
    });

    (total_w, left_pane_w)
}
