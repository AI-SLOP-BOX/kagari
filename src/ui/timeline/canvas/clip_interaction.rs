//! Layer clip bar dragging, trimming, slip editing, and markers.

use eframe::egui;
use crate::core::timeline::{Layer, LayerType};
use crate::ui::theme::colors;
use crate::ui::timeline::{
    draw_layer_waveform, layout, pending_actions::TimelineActions,
    state::LayerBodyDragState, waveform_cache, LayerWaveformParams,
};

pub struct LayerBarDragParams<'a> {
    pub layer: &'a mut Layer,
    pub i: usize,
    pub layers_len: usize,
    pub layer_rect: egui::Rect,
    pub bar_rect: egui::Rect,
    pub start_frame: u32,
    pub zoom_span: u32,
    pub total_frames: u32,
    pub fps: u32,
    pub snap_to_keyframes: bool,
    pub layer_edges: &'a [(u32, usize)],
    pub current_frame: u32,
}

pub fn handle_layer_bar_drag(
    ui: &mut egui::Ui,
    params: LayerBarDragParams<'_>,
    actions: &mut TimelineActions,
    project_changed: &mut bool,
) {
    let LayerBarDragParams {
        layer,
        i,
        layers_len,
        layer_rect,
        bar_rect,
        start_frame,
        zoom_span,
        total_frames,
        fps,
        snap_to_keyframes,
        layer_edges,
        current_frame,
    } = params;

    fn handle_rect_of(is_in: bool, lr: &egui::Rect) -> egui::Rect {
        const HW: f32 = 6.0;
        if is_in {
            egui::Rect::from_min_size(egui::pos2(lr.left() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
        } else {
            egui::Rect::from_min_size(egui::pos2(lr.right() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
        }
    }
    const HANDLE_W: f32 = 6.0;
    let in_handle = egui::Rect::from_min_size(
        egui::pos2(layer_rect.left() - HANDLE_W * 0.5, layer_rect.top()),
        egui::vec2(HANDLE_W, layer_rect.height()),
    );
    let out_handle = egui::Rect::from_min_size(
        egui::pos2(layer_rect.right() - HANDLE_W * 0.5, layer_rect.top()),
        egui::vec2(HANDLE_W, layer_rect.height()),
    );
    let in_resp = ui.interact(in_handle, egui::Id::new(("trim_in", i)), egui::Sense::click_and_drag());
    let out_resp = ui.interact(out_handle, egui::Id::new(("trim_out", i)), egui::Sense::click_and_drag());

    // ── Body drag: slide the whole layer (in/out preserved) ──
    {
        const HEADER_W: f32 = 6.0;
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(layer_rect.left() + HEADER_W, layer_rect.top()),
            egui::pos2(layer_rect.right() - HEADER_W, layer_rect.bottom()),
        );
        let body_resp = ui.interact(
            body_rect,
            egui::Id::new(("layer_slide", i, layer.id.as_str())),
            egui::Sense::drag(),
        );
        let body_drag_id = egui::Id::new(("layer_slide_state", layer.id.as_str()));
        if body_resp.drag_started() {
            ui.ctx().data_mut(|data| {
                data.insert_temp(body_drag_id, LayerBodyDragState::default());
            });
        }
        if body_resp.dragged() {
            let total_frame_offset =
                (body_resp.drag_delta().x / bar_rect.width() * zoom_span as f32).round() as i32;
            let mut drag_state = ui.ctx().data(|data| {
                data.get_temp::<LayerBodyDragState>(body_drag_id)
                    .unwrap_or_default()
            });
            let delta_frames = total_frame_offset - drag_state.frame_offset;
            drag_state.frame_offset = total_frame_offset;
            // ── Vertical drag → reorder layers in stack ──
            let row_h = layout::LAYER_ROW_HEIGHT;
            let total_y = body_resp.drag_delta().y;
            let incremental_y = total_y - drag_state.previous_y;
            drag_state.previous_y = total_y;
            let new_accum = drag_state.y_remainder + incremental_y;
            let row_offset = (new_accum / row_h).trunc() as i32;
            if row_offset != 0 {
                let target = (i as i32 - row_offset).clamp(0, layers_len as i32 - 1) as usize;
                if target != i {
                    actions.swap_request = Some((i, target));
                }
                drag_state.y_remainder = new_accum - row_offset as f32 * row_h;
            } else {
                drag_state.y_remainder = new_accum;
            }
            ui.ctx().data_mut(|data| data.insert_temp(body_drag_id, drag_state));
            if delta_frames != 0 {
                let alt_held = ui.input(|inp| inp.modifiers.alt);
                if alt_held {
                    // ── Slip edit: bar stays, content timing shifts ──
                    let shift = -delta_frames;
                    for kf in layer.transform.position.keyframes_mut().into_iter().flatten() {
                        kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                    }
                    if let Some(kfs) = layer.transform.position.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                    for kf in layer.transform.scale.keyframes_mut().into_iter().flatten() {
                        kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                    }
                    if let Some(kfs) = layer.transform.scale.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                    for kf in layer.transform.rotation.keyframes_mut().into_iter().flatten() {
                        kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                    }
                    if let Some(kfs) = layer.transform.rotation.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                    for kf in layer.transform.opacity.keyframes_mut().into_iter().flatten() {
                        kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                    }
                    if let Some(kfs) = layer.transform.opacity.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                    for m in layer.markers.iter_mut() {
                        m.frame = (m.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                    }
                    // Slip effect keyframes too
                    use crate::core::effect_params::ParamRef;
                    for effect in layer.effects.iter_mut() {
                        for (_, param) in effect.effect_type.animatable_params() {
                            match param {
                                ParamRef::Scalar(anim) => {
                                    if let Some(kfs) = anim.keyframes_mut() {
                                        for kf in kfs.iter_mut() {
                                            kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                        }
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                                ParamRef::Vec2(anim) => {
                                    if let Some(kfs) = anim.keyframes_mut() {
                                        for kf in kfs.iter_mut() {
                                            kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                        }
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                                ParamRef::Vec3(anim) => {
                                    if let Some(kfs) = anim.keyframes_mut() {
                                        for kf in kfs.iter_mut() {
                                            kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                        }
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                                ParamRef::Vec4Color(anim) => {
                                    if let Some(kfs) = anim.keyframes_mut() {
                                        for kf in kfs.iter_mut() {
                                            kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                        }
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Slide: move the whole bar
                    let span = layer.out_frame - layer.in_frame;
                    let raw_in = (layer.in_frame as i32 + delta_frames)
                        .clamp(0, (total_frames - span) as i32)
                        as u32;
                    let new_in = if snap_to_keyframes {
                        crate::ui::timeline::snapping::snap_to_sorted_layer_edges(
                            raw_in,
                            i,
                            layer_edges,
                        )
                    } else {
                        raw_in
                    };
                    layer.in_frame = new_in;
                    layer.out_frame = new_in + span;
                }
                *project_changed = true;
            }
        }
        if body_resp.drag_stopped() {
            ui.ctx().data_mut(|data| data.remove::<LayerBodyDragState>(body_drag_id));
        }
        if body_resp.hovered() && !in_resp.hovered() && !out_resp.hovered() {
            let alt_held = ui.input(|inp| inp.modifiers.alt);
            ui.ctx().set_cursor_icon(if alt_held { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::Grab });
            body_resp.on_hover_text(format!(
                "{} — In: {} Out: {} ({:.2}s)",
                layer.name,
                layer.in_frame,
                layer.out_frame,
                layer.duration_frames() as f32 / fps.max(1) as f32
            ));
        }
    }

    for (resp, is_in) in [(&in_resp, true), (&out_resp, false)] {
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.painter().rect_filled(handle_rect_of(is_in, &layer_rect), 2.0, colors::HANDLE_HOVER_FILL.linear_multiply(180.0 / 255.0));
        }
        if resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                let frame = (start_frame as f32 + norm * zoom_span as f32).round() as u32;
                let frame = frame.min(total_frames);
                if is_in {
                    layer.in_frame = frame.min(layer.out_frame.saturating_sub(1));
                } else {
                    let old_out = layer.out_frame;
                    layer.out_frame = frame.max(layer.in_frame + 1);
                    // ── Shift+drag Out = ripple edit: close gaps for layers below ──
                    if ui.input(|inp| inp.modifiers.shift) && layer.out_frame != old_out {
                        let shift = layer.out_frame as i64 - old_out as i64;
                        actions.pending_ripple = Some((i, old_out, shift));
                    }
                }
                *project_changed = true;
            }
        }
    }

    // ── Layer markers: small triangles on the bar (AE parity) ──
    for marker in &layer.markers {
        if marker.frame < start_frame || marker.frame > start_frame + zoom_span { continue; }
        let norm = (marker.frame - start_frame) as f32 / zoom_span as f32;
        let mx = layer_rect.left() + norm * layer_rect.width();
        let my = layer_rect.top() + 1.0;
        let [r, g, b] = marker.color;
        let mcol = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
        let pts = vec![
            egui::pos2(mx, my + 5.0),
            egui::pos2(mx - 4.0, my),
            egui::pos2(mx + 4.0, my),
        ];
        ui.painter().add(egui::Shape::convex_polygon(pts, mcol, egui::Stroke::NONE));
    }

    // ── Real waveform for video layers with audio ──
    if let LayerType::Video { audio_wav: Some(wav_path), .. } = &layer.layer_type {
        if let Some(waveform) = waveform_cache::get(
            ui.ctx(),
            std::path::Path::new(wav_path),
        ) {
            draw_layer_waveform(
                &ui.painter().with_clip_rect(layer_rect),
                LayerWaveformParams {
                    timeline_rect: bar_rect,
                    clip_rect: layer_rect,
                    layer,
                    waveform: &waveform,
                    start_frame,
                    visible_span: zoom_span,
                    fps,
                    color: colors::TIMELINE_WAVEFORM,
                },
            );
        }
    }

    if let LayerType::Audio { path, .. } = &layer.layer_type {
        if let Some(waveform) = waveform_cache::get(
            ui.ctx(),
            std::path::Path::new(path),
        ) {
            draw_layer_waveform(
                &ui.painter().with_clip_rect(layer_rect),
                LayerWaveformParams {
                    timeline_rect: bar_rect,
                    clip_rect: layer_rect,
                    layer,
                    waveform: &waveform,
                    start_frame,
                    visible_span: zoom_span,
                    fps,
                    color: colors::TIMELINE_WAVEFORM,
                },
            );
        }
    }

    let playhead_norm =
        (current_frame as f32 - start_frame as f32) / zoom_span as f32;
    if (0.0..=1.0).contains(&playhead_norm) {
        let playhead_x =
            bar_rect.left() + playhead_norm * bar_rect.width();
        ui.painter()
            .with_clip_rect(bar_rect)
            .line_segment(
                [
                    egui::pos2(playhead_x, bar_rect.top()),
                    egui::pos2(playhead_x, bar_rect.bottom()),
                ],
                egui::Stroke::new(
                    1.5_f32,
                    colors::TIMELINE_PLAYHEAD,
                ),
            );
    }
}
