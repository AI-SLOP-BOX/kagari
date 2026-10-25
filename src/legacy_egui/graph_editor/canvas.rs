pub mod anchor_point;
pub mod channel_points;
pub mod click_create;
pub mod tangent_handles;

use super::*;
use anchor_point::handle_anchor_interaction;
use channel_points::handle_channel_points_interaction;
use click_create::handle_canvas_click_create;
use tangent_handles::handle_tangent_handles;

pub fn draw_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    fps: u32,
    layer: &mut Layer,
    project_changed: &mut bool,
    linked_tangent: &mut bool,
) {
    let graph_height = 120.0f32;

    ui.group(|ui| {
        let graph_prop = selected_property
            .clone()
            .unwrap_or_else(|| "Position X".to_string());
        super::toolbar::draw_graph_toolbar(
            ui,
            selected_property,
            linked_tangent,
            layer,
            project_changed,
            fps,
        );

        let total_f = duration_frames.max(1);

        // Detect speed graph vs value graph mode (0 = speed graph, 1 = value graph)
        let speed_graph_mode = ui.ctx().data(|d| {
            d.get_temp::<i32>(egui::Id::new("ae_graph_mode_select"))
                .unwrap_or(0)
        }) == 0;

        let (samples, keyframes_ref) =
            super::sample_data::sample_graph_property(layer, &graph_prop, total_f);

        // Allocate drawing region
        let (rect, graph_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), graph_height),
            egui::Sense::click_and_drag(),
        );
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("ae_graph_canvas_rect", &layer.id, &graph_prop)),
                rect,
            );
        });
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_gray(25));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_gray(50)),
        );

        let min_val = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f32::INFINITY, f32::min);
        let max_val = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f32::NEG_INFINITY, f32::max);
        let val_range = (max_val - min_val).max(0.001);

        // Min/max value readouts on the right edge (curve readability)
        if !speed_graph_mode {
            let mono = egui::FontId::monospace(9.0);
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                egui::Align2::RIGHT_TOP,
                format!("{:.1}", max_val),
                mono.clone(),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{:.1}", min_val),
                mono,
                colors::TEXT_MUTED,
            );
        }

        // Convert keyframe time/value to screen space coordinates inside the allocated rect
        let display_fps = fps.max(1);
        let points: Vec<egui::Pos2> = if speed_graph_mode {
            // Speed Graph: compute velocity curve and map to screen
            let vel_curve = compute_velocity_curve(&keyframes_ref, display_fps);
            let vel_min = vel_curve
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::INFINITY, f32::min);
            let vel_max = vel_curve
                .iter()
                .map(|(_, v)| *v)
                .fold(f32::NEG_INFINITY, f32::max);
            let vel_range = (vel_max - vel_min).abs().max(0.001);

            // Update readout labels for velocity range
            {
                let mono = egui::FontId::monospace(9.0);
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{:.1} v/s", vel_max),
                    mono.clone(),
                    colors::TEXT_MUTED,
                );
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{:.1} v/s", vel_min),
                    mono,
                    colors::TEXT_MUTED,
                );
            }

            vel_curve
                .iter()
                .map(|&(frame, vel)| {
                    let x = rect.left() + (frame / total_f as f32) * rect.width();
                    let y =
                        rect.bottom() - 4.0 - ((vel - vel_min) / vel_range) * (rect.height() - 8.0);
                    egui::pos2(x, y)
                })
                .collect()
        } else {
            // Value Graph: original value curve
            samples
                .iter()
                .map(|&(f, v)| {
                    let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
                    let y =
                        rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
                    egui::pos2(x, y)
                })
                .collect()
        };

        // Draw graph spline segments (Value Curve)
        for window in points.windows(2) {
            ui.painter().line_segment(
                [window[0], window[1]],
                egui::Stroke::new(2.0_f32, colors::TIMELINE_KEYFRAME),
            );
        }

        // ── Click on empty graph area → create a keyframe at that frame/value ──
        handle_canvas_click_create(
            &graph_response,
            rect,
            total_f,
            min_val,
            val_range,
            &graph_prop,
            layer,
            project_changed,
        );

        // Draw Speed Graph Velocity Line (First Derivative v(t) = dy/dt)
        if !speed_graph_mode {
            let mut max_speed = 0.0f32;
            let mut speed_pts = Vec::with_capacity(samples.len());
            for win in samples.windows(2) {
                let dt = (win[1].0 as f32 - win[0].0 as f32).max(1.0);
                let speed = ((win[1].1 - win[0].1) / dt).abs();
                max_speed = max_speed.max(speed);
                speed_pts.push(((win[0].0 + win[1].0) as f32 * 0.5, speed));
            }

            if max_speed > 0.001 {
                for i in 0..speed_pts.len() {
                    let sx = rect.left() + (speed_pts[i].0 / total_f as f32) * rect.width();
                    let sy =
                        rect.bottom() - 4.0 - (speed_pts[i].1 / max_speed) * (rect.height() - 16.0);
                    let p1 = egui::pos2(sx, sy);

                    let next_i = (i + 1).min(speed_pts.len() - 1);
                    let nsx = rect.left() + (speed_pts[next_i].0 / total_f as f32) * rect.width();
                    let nsy = rect.bottom()
                        - 4.0
                        - (speed_pts[next_i].1 / max_speed) * (rect.height() - 16.0);
                    let p2 = egui::pos2(nsx, nsy);

                    ui.painter()
                        .line_segment([p1, p2], egui::Stroke::new(1.2_f32, colors::MOTION_PATH));
                }

                // Peak Speed Badge HUD
                let speed_badge_pos = egui::pos2(rect.right() - 110.0, rect.top() + 6.0);
                ui.painter().text(
                    speed_badge_pos,
                    egui::Align2::LEFT_TOP,
                    format!("Peak: {:.0} px/s", max_speed * display_fps as f32),
                    egui::FontId::monospace(10.0),
                    colors::MOTION_PATH,
                );
            }
        }

        // Render interactive keyframe anchor points & tangent handles
        {
            let frame_to_x = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let val_to_y =
                |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if graph_prop.starts_with("3D ") {
                let ci = axis_3d(&graph_prop);
                layer_keyframes_of_vec3(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value[ci]))
                            .collect()
                    })
                    .unwrap_or_default()
            } else if matches!(
                graph_prop.as_str(),
                "Position X" | "Position Y" | "Scale X" | "Scale Y"
            ) {
                let comp_idx = if graph_prop.ends_with('Y') {
                    1usize
                } else {
                    0usize
                };
                layer_keyframes_of_vec2(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value[comp_idx]))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else if is_effect_property(&graph_prop) {
                effect_keyframes_of_f32(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                layer_keyframes_of_f32(layer, &graph_prop)
                    .map(|kfs| {
                        kfs.iter()
                            .enumerate()
                            .map(|(i, kf)| (i, kf.frame, kf.value))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };

            if is_effect_property(&graph_prop) && graph_prop.contains('|') {
                let channel_keys = effect_parameter_channel_keyframes(layer, &graph_prop);
                handle_channel_points_interaction(
                    ui,
                    rect,
                    total_f,
                    val_range,
                    layer,
                    &graph_prop,
                    &channel_keys,
                    frame_to_x,
                    val_to_y,
                    project_changed,
                );
            }

            let drag_state_id = egui::Id::new(("graph_keyframe_drag", &layer.id, &graph_prop));
            let active_drag: Option<GraphKeyframeDrag> =
                ui.ctx().data(|d| d.get_temp(drag_state_id));

            for (kf_idx, kf_frame, kf_val) in &kf_positions {
                let drag_display = active_drag.filter(|drag| drag.current_frame == *kf_frame);
                let display_frame = drag_display
                    .map(|drag| drag.current_frame)
                    .unwrap_or(*kf_frame);
                let display_value = drag_display
                    .map(|drag| drag.current_value)
                    .unwrap_or(*kf_val);
                let pt = egui::pos2(frame_to_x(display_frame), val_to_y(display_value));

                let token = handle_anchor_interaction(
                    ui,
                    rect,
                    total_f,
                    val_range,
                    layer,
                    &graph_prop,
                    *kf_idx,
                    *kf_frame,
                    *kf_val,
                    drag_display,
                    display_frame,
                    pt,
                    drag_state_id,
                    project_changed,
                );

                let Some(anchor_token) = token else {
                    continue;
                };

                // --- Tangent handles: drag to edit custom bezier control points ---
                handle_tangent_handles(
                    ui,
                    layer,
                    &graph_prop,
                    anchor_token,
                    display_frame,
                    pt,
                    linked_tangent,
                    project_changed,
                );
            }
        }
    });
}
