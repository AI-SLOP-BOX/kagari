//! Motion path trajectories and puppet pin paths overlay.

use crate::ui::theme::colors;
use crate::ui::viewport_state::ViewportTransform;
use crate::KagariApp;
use eframe::egui;

pub fn draw_motion_paths(ui: &mut egui::Ui, app: &KagariApp, transform: &ViewportTransform) {
    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selection.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer = &comp.layers[idx];
            if let Some(kfs) = layer.transform.position.keyframes() {
                if kfs.len() >= 2 {
                    let path_stroke = egui::Stroke::new(1.5_f32, colors::MOTION_PATH);
                    let to_screen = |v: [f32; 2]| transform.comp_to_screen(v);

                    for seg in kfs.windows(2) {
                        let (a, b) = (&seg[0], &seg[1]);
                        let span = (b.frame - a.frame).max(1);
                        let steps = ((span as usize) / 4).clamp(4, 32);
                        let mut prev = to_screen(a.value);
                        for s in 1..=steps {
                            let raw_t = s as f32 / steps as f32;
                            let eased_t = match &a.interpolation {
                                crate::core::keyframe::InterpolationType::Hold => 0.0,
                                crate::core::keyframe::InterpolationType::Linear => raw_t,
                                crate::core::keyframe::InterpolationType::Bezier {
                                    custom_bezier,
                                    ..
                                } => {
                                    let c = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                                    crate::core::keyframe::solve_bezier_eased_time(
                                        raw_t, c[0], c[1], c[2], c[3],
                                    )
                                }
                            };
                            let val = [
                                a.value[0] + (b.value[0] - a.value[0]) * eased_t,
                                a.value[1] + (b.value[1] - a.value[1]) * eased_t,
                            ];
                            let p = to_screen(val);
                            ui.painter().line_segment([prev, p], path_stroke);
                            prev = p;
                        }
                    }

                    // Keyframe Anchor Point Markers
                    for kf in kfs {
                        let p = to_screen(kf.value);
                        ui.painter().circle_filled(p, 4.0, colors::KEYFRAME_DOT);
                        ui.painter().circle_stroke(
                            p,
                            4.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                        );
                        if app
                            .viewport_pos_kf_drag_state
                            .is_some_and(|(li, f, _, _)| li == idx && f == kf.frame)
                        {
                            ui.painter().circle_stroke(
                                p,
                                6.5,
                                egui::Stroke::new(1.5_f32, colors::TIMELINE_PLAYHEAD),
                            );
                        }
                    }

                    // Spatial Bezier Tangent Handles
                    for seg in kfs.windows(2) {
                        let (a, b) = (&seg[0], &seg[1]);
                        if let crate::core::keyframe::InterpolationType::Bezier {
                            custom_bezier,
                            ..
                        } = &a.interpolation
                        {
                            let c = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                            let val_delta = [b.value[0] - a.value[0], b.value[1] - a.value[1]];

                            // Outgoing handle from keyframe a
                            let out_pos = [
                                a.value[0] + c[0] * val_delta[0],
                                a.value[1] + c[1] * val_delta[1],
                            ];
                            let a_screen = to_screen(a.value);
                            let out_screen = to_screen(out_pos);
                            let handle_stroke = egui::Stroke::new(
                                1.0_f32,
                                colors::MOTION_PATH.linear_multiply(0.5),
                            );
                            ui.painter()
                                .line_segment([a_screen, out_screen], handle_stroke);
                            ui.painter().circle_filled(
                                out_screen,
                                2.5,
                                colors::MOTION_PATH.linear_multiply(0.6),
                            );

                            // Incoming handle for keyframe b
                            let in_pos = [
                                b.value[0] - (1.0 - c[2]) * val_delta[0],
                                b.value[1] - (1.0 - c[3]) * val_delta[1],
                            ];
                            let b_screen = to_screen(b.value);
                            let in_screen = to_screen(in_pos);
                            ui.painter()
                                .line_segment([b_screen, in_screen], handle_stroke);
                            ui.painter().circle_filled(
                                in_screen,
                                2.5,
                                colors::MOTION_PATH.linear_multiply(0.6),
                            );
                        }
                    }

                    // Current playhead position marker on the path
                    let cur = layer
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    let cp = to_screen(cur);
                    ui.painter()
                        .circle_filled(cp, 3.0, colors::TIMELINE_PLAYHEAD);
                    ui.painter().circle_stroke(
                        cp,
                        3.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                    );
                }
            }
        }
    }

    // Puppet pin trajectory paths for the selected layer
    {
        let to_screen = |v: [f32; 2]| transform.comp_to_screen(v);
        let draw_v2_path = |kfs: &[crate::core::keyframe::Keyframe<[f32; 2]>],
                            stroke: egui::Stroke,
                            dot_color: egui::Color32,
                            dot_r: f32,
                            painter: &egui::Painter| {
            if kfs.len() < 2 {
                return;
            }
            for seg in kfs.windows(2) {
                let (a, b) = (&seg[0], &seg[1]);
                let span = (b.frame - a.frame).max(1);
                let steps = ((span as usize) / 4).clamp(4, 32);
                let mut prev = to_screen(a.value);
                for s in 1..=steps {
                    let raw_t = s as f32 / steps as f32;
                    let eased_t = match &a.interpolation {
                        crate::core::keyframe::InterpolationType::Hold => 0.0,
                        crate::core::keyframe::InterpolationType::Linear => raw_t,
                        crate::core::keyframe::InterpolationType::Bezier {
                            custom_bezier, ..
                        } => {
                            let cc = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                            crate::core::keyframe::solve_bezier_eased_time(
                                raw_t, cc[0], cc[1], cc[2], cc[3],
                            )
                        }
                    };
                    let val = [
                        a.value[0] + (b.value[0] - a.value[0]) * eased_t,
                        a.value[1] + (b.value[1] - a.value[1]) * eased_t,
                    ];
                    let pt = to_screen(val);
                    painter.line_segment([prev, pt], stroke);
                    prev = pt;
                }
            }
            for kf in kfs {
                let p = to_screen(kf.value);
                painter.circle_filled(p, dot_r, dot_color);
                painter.circle_stroke(p, dot_r, egui::Stroke::new(0.8_f32, egui::Color32::BLACK));
            }
        };

        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if let Some(layer) = comp.layers.get(idx) {
                for pin in &layer.puppet_pins {
                    if let Some(pkfs) = pin.position.keyframes() {
                        draw_v2_path(
                            pkfs,
                            egui::Stroke::new(1.2_f32, colors::MOTION_PATH.linear_multiply(0.45)),
                            colors::ACCENT_YELLOW.linear_multiply(0.7),
                            3.0,
                            ui.painter(),
                        );
                    }
                }
            }
        }
    }
}
