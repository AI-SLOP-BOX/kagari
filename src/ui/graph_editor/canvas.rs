use super::*;

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

        // Sample values along timeline duration for drawing curve (screen-adaptive step)
        let max_samples = 2000usize;
        let step = (total_f as usize / max_samples).max(1) as u32;
        let mut samples = Vec::with_capacity((total_f / step) as usize + 2);
        let mut f = 0u32;
        while f <= total_f {
            let raw_val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                p if p.starts_with("3D Position") => {
                    layer.transform_3d.position.evaluate(f)[axis_3d(p)]
                }
                p if p.starts_with("3D Rotation") => {
                    layer.transform_3d.rotation.evaluate(f)[axis_3d(p)]
                }
                p if p.starts_with("3D Scale") => layer.transform_3d.scale.evaluate(f)[axis_3d(p)],
                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                    let ci = pin_component(p);
                    let pid = p.split(':').nth(1).unwrap_or("");
                    layer
                        .puppet_pins
                        .iter()
                        .find(|pp| pp.id == pid)
                        .map(|pp| pp.position.evaluate(f)[ci])
                        .unwrap_or(0.0)
                }
                p if is_effect_property(p) => effect_parameter_channel_value(layer, p, f),
                _ => layer.transform.position.evaluate(f)[0],
            };
            let val = if raw_val.is_nan() { 0.0 } else { raw_val };
            samples.push((f, val));
            if f < total_f && f + step > total_f {
                f = total_f;
            } else {
                f += step;
            }
        }

        // Compute per-keyframe velocity when in speed graph mode
        let keyframes_ref: Vec<(u32, f32)> = match graph_prop.as_str() {
            "Position X" => layer
                .transform
                .position
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Position Y" => layer
                .transform
                .position
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Scale X" => layer
                .transform
                .scale
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Scale Y" => layer
                .transform
                .scale
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Rotation" => layer
                .transform
                .rotation
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            "Opacity" => layer
                .transform
                .opacity
                .keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            p if p.starts_with("3D Position") => layer
                .transform_3d
                .position
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("3D Rotation") => layer
                .transform_3d
                .rotation
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("3D Scale") => layer
                .transform_3d
                .scale
                .keyframes()
                .map(|kfs| {
                    kfs.iter()
                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                        .collect()
                })
                .unwrap_or_default(),
            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                let ci = pin_component(p);
                let pid = p.split(':').nth(1).unwrap_or("");
                layer
                    .puppet_pins
                    .iter()
                    .find(|pp| pp.id == pid)
                    .and_then(|pp| pp.position.keyframes())
                    .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                    .unwrap_or_default()
            }
            p if is_effect_property(p) => effect_parameter_channel_keyframes(layer, p),
            _ => vec![],
        };

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
        {
            use crate::core::keyframe::{InterpolationType as GInterp, Keyframe as GKeyframe};

            let x_of = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y_of =
                |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            if graph_response.clicked() {
                if let Some(pos) = graph_response.interact_pointer_pos() {
                    if rect.contains(pos) {
                        // Existing anchor proximity guard: clicks on anchors belong to the anchor drag
                        let chan_kfs = |p: &crate::core::property::Animatable<[f32; 2]>,
                                        ci: usize|
                         -> Vec<(u32, f32)> {
                            p.keyframes()
                                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                                .unwrap_or_default()
                        };
                        let scalar_kfs =
                            |p: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
                                p.keyframes()
                                    .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                                    .unwrap_or_default()
                            };
                        let anchor_pts: Vec<(u32, f32)> = match graph_prop.as_str() {
                            "Position X" => chan_kfs(&layer.transform.position, 0),
                            "Position Y" => chan_kfs(&layer.transform.position, 1),
                            "Scale X" => chan_kfs(&layer.transform.scale, 0),
                            "Scale Y" => chan_kfs(&layer.transform.scale, 1),
                            "Rotation" => scalar_kfs(&layer.transform.rotation),
                            "Opacity" => scalar_kfs(&layer.transform.opacity),
                            p if p.starts_with("3D Position") => layer
                                .transform_3d
                                .position
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("3D Rotation") => layer
                                .transform_3d
                                .rotation
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("3D Scale") => layer
                                .transform_3d
                                .scale
                                .keyframes()
                                .map(|k| {
                                    k.iter()
                                        .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                let ci = pin_component(p);
                                let pid = p.split(':').nth(1).unwrap_or("");
                                layer
                                    .puppet_pins
                                    .iter()
                                    .find(|pp| pp.id == pid)
                                    .map(|pp| chan_kfs(&pp.position, ci))
                                    .unwrap_or_default()
                            }
                            p if is_effect_property(p) => {
                                effect_parameter_channel_keyframes(layer, p)
                            }
                            _ => vec![],
                        };
                        let near_anchor = anchor_pts
                            .iter()
                            .any(|&(f, v)| egui::pos2(x_of(f), y_of(v)).distance(pos) < 8.0);

                        if !near_anchor {
                            let new_frame = (((pos.x - rect.left()) / rect.width())
                                * total_f as f32)
                                .round()
                                .clamp(0.0, total_f as f32)
                                as u32;
                            let new_val = min_val
                                + ((rect.bottom() - 4.0 - pos.y) / (rect.height() - 8.0))
                                    * val_range;
                            match graph_prop.as_str() {
                                "Position X" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Position Y" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Scale X" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Scale Y" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                "Rotation" => {
                                    layer.transform.rotation.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        new_val,
                                        GInterp::Linear,
                                    ));
                                }
                                "Opacity" => {
                                    layer.transform.opacity.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        new_val.clamp(0.0, 100.0),
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Position") => {
                                    let mut v = layer.transform_3d.position.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.position.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Rotation") => {
                                    let mut v = layer.transform_3d.rotation.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.rotation.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("3D Scale") => {
                                    let mut v = layer.transform_3d.scale.evaluate(new_frame);
                                    v[axis_3d(p)] = new_val;
                                    layer.transform_3d.scale.add_keyframe(GKeyframe::new(
                                        new_frame,
                                        v,
                                        GInterp::Linear,
                                    ));
                                }
                                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                    if let Some(pin) = pin_anim_mut(layer, p) {
                                        let ci = pin_component(p);
                                        let mut v = pin.evaluate(new_frame);
                                        v[ci] = new_val;
                                        pin.add_keyframe(GKeyframe::new(
                                            new_frame,
                                            v,
                                            GInterp::Linear,
                                        ));
                                    }
                                }
                                p if is_effect_property(p) => {
                                    let Some((effect_id, parameter_name, component)) =
                                        parse_effect_property(p)
                                    else {
                                        return;
                                    };
                                    if let Some(effect) = layer
                                        .effects
                                        .iter_mut()
                                        .find(|effect| effect.id == effect_id)
                                    {
                                        if let Some(component) = component {
                                            effect.effect_type.set_parameter_component_keyframe(
                                                parameter_name,
                                                component,
                                                new_frame,
                                                new_val,
                                            );
                                        } else {
                                            effect.effect_type.set_scalar_parameter_keyframe(
                                                parameter_name,
                                                new_frame,
                                                new_val,
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                            *project_changed = true;
                        }
                    }
                }
            }
        }

        // Draw Speed Graph Velocity Line (First Derivative v(t) = dy/dt)
        // Skip overlay when in speed graph mode (main curve already shows velocity)
        if !speed_graph_mode {
            let mut max_speed = 0.0f32;
            let mut speed_pts = Vec::with_capacity(points.len());
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

        // Render interactive keyframe anchor points & tangent handles (real editing)
        // Anchors are drawn at actual keyframe positions and can be dragged in time;
        // tangent handles edit the keyframe's custom bezier control points.
        {
            use crate::core::keyframe::{BezierControlPoint, InterpolationType};

            // Mutable access to Vec2-typed keyframe tracks (Position / Scale)
            fn keyframes_of_vec2<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 2]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 2]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Position X" | "Position Y" => animated(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => animated(&mut layer.transform.scale),
                    _ => None,
                }
            }

            // Mutable access to scalar keyframe tracks (Rotation / Opacity)
            fn keyframes_of_f32<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<f32>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Rotation" => animated(&mut layer.transform.rotation),
                    "Opacity" => animated(&mut layer.transform.opacity),
                    _ => None,
                }
            }

            fn effect_keyframes_of_f32<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                let (effect_id, label, component) = parse_effect_property(prop)?;
                if component.is_some() {
                    return None;
                }
                let effect = layer
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == effect_id)?;
                for (name, parameter) in effect.effect_type.animatable_params() {
                    if name == label {
                        if let crate::core::effect_params::ParamRef::Scalar(track) = parameter {
                            return track.keyframes_mut();
                        }
                    }
                }
                None
            }

            fn keyframes_of_vec3<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 3]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 3]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    p if p.starts_with("3D Position") => animated(&mut layer.transform_3d.position),
                    p if p.starts_with("3D Rotation") => animated(&mut layer.transform_3d.rotation),
                    p if p.starts_with("3D Scale") => animated(&mut layer.transform_3d.scale),
                    _ => None,
                }
            }

            macro_rules! with_keyframes {
                ($layer:expr, $prop:expr, $kfs:ident => $body:expr) => {{
                    let prop: String = $prop.clone();
                    if prop.starts_with("3D ") {
                        if let Some($kfs) = keyframes_of_vec3($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else if matches!(
                        prop.as_str(),
                        "Position X" | "Position Y" | "Scale X" | "Scale Y"
                    ) {
                        if let Some($kfs) = keyframes_of_vec2($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else if is_effect_property(&prop) {
                        if let Some($kfs) = effect_keyframes_of_f32($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    } else {
                        if let Some($kfs) = keyframes_of_f32($layer, &prop) {
                            Some({ $body })
                        } else {
                            None
                        }
                    }
                }};
            }

            let frame_to_x = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let val_to_y =
                |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if graph_prop.starts_with("3D ") {
                let ci = axis_3d(&graph_prop);
                keyframes_of_vec3(layer, &graph_prop)
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
                keyframes_of_vec2(layer, &graph_prop)
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
                keyframes_of_f32(layer, &graph_prop)
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
                let channel_drag_id =
                    egui::Id::new(("effect_channel_drag", &layer.id, &graph_prop));
                let active_channel_drag: Option<GraphKeyframeDrag> =
                    ui.ctx().data(|d| d.get_temp(channel_drag_id));
                for (frame, value) in &channel_keys {
                    let drag_display =
                        active_channel_drag.filter(|drag| drag.current_frame == *frame);
                    let display_frame = drag_display
                        .map(|drag| drag.current_frame)
                        .unwrap_or(*frame);
                    let display_value = drag_display
                        .map(|drag| drag.current_value)
                        .unwrap_or(*value);
                    // Use the frame identity for egui IDs. An array index is
                    // not stable after a retime sorts the keyframes and can
                    // make the drag state attach to a neighbouring key.
                    let key_token = drag_display
                        .map(|drag| drag.original_frame as usize)
                        .unwrap_or(*frame as usize);
                    let key_pos = egui::pos2(frame_to_x(display_frame), val_to_y(display_value));
                    let key_rect = egui::Rect::from_center_size(key_pos, egui::vec2(14.0, 14.0));
                    if !ui.is_rect_visible(key_rect) && drag_display.is_none() {
                        continue;
                    }
                    let key_response = ui.interact(
                        key_rect,
                        egui::Id::new(("effect_channel_key", &layer.id, &graph_prop, key_token)),
                        egui::Sense::click_and_drag(),
                    );
                    ui.painter()
                        .circle_filled(key_pos, 4.0, colors::TIMELINE_KEYFRAME);
                    if let Some(points) =
                        effect_channel_bezier_points(layer, &graph_prop, display_frame)
                    {
                        let out =
                            egui::pos2(key_pos.x + points[2] * 44.0, key_pos.y - points[3] * 24.0);
                        let incoming =
                            egui::pos2(key_pos.x - points[0] * 44.0, key_pos.y + points[1] * 24.0);
                        let out_drag_id = egui::Id::new((
                            "effect_bezier_drag_out",
                            &layer.id,
                            &graph_prop,
                            key_token,
                        ));
                        let in_drag_id = egui::Id::new((
                            "effect_bezier_drag_in",
                            &layer.id,
                            &graph_prop,
                            key_token,
                        ));
                        let out_resp = ui.interact(
                            egui::Rect::from_center_size(out, egui::vec2(14.0, 14.0)),
                            egui::Id::new(("effect_bezier_out", &layer.id, &graph_prop, key_token)),
                            egui::Sense::drag(),
                        );
                        let in_resp = ui.interact(
                            egui::Rect::from_center_size(incoming, egui::vec2(14.0, 14.0)),
                            egui::Id::new(("effect_bezier_in", &layer.id, &graph_prop, key_token)),
                            egui::Sense::drag(),
                        );
                        ui.painter().line_segment(
                            [key_pos, out],
                            egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
                        );
                        ui.painter().line_segment(
                            [key_pos, incoming],
                            egui::Stroke::new(1.0_f32, colors::MOTION_PATH),
                        );
                        ui.painter().circle_filled(out, 3.0, colors::HANDLE_NORMAL);
                        ui.painter()
                            .circle_filled(incoming, 3.0, colors::HANDLE_NORMAL);
                        if out_resp.drag_started() {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(out_drag_id, points));
                        }
                        if in_resp.drag_started() {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(in_drag_id, points));
                        }
                        let out_base = ui
                            .ctx()
                            .data(|data| data.get_temp::<[f32; 4]>(out_drag_id))
                            .unwrap_or(points);
                        let in_base = ui
                            .ctx()
                            .data(|data| data.get_temp::<[f32; 4]>(in_drag_id))
                            .unwrap_or(points);
                        let mut next = points;
                        if out_resp.dragged() {
                            next[2] = clamp_bezier_x(
                                out_base[2] + out_resp.drag_delta().x / 44.0,
                                out_base[0] + 0.01,
                                1.0,
                            );
                            next[3] =
                                (out_base[3] - out_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if in_resp.dragged() {
                            next[0] = clamp_bezier_x(
                                in_base[0] - in_resp.drag_delta().x / 44.0,
                                0.0,
                                in_base[2] - 0.01,
                            );
                            next[1] = (in_base[1] + in_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if (out_resp.dragged() || in_resp.dragged())
                            && set_effect_channel_bezier(layer, &graph_prop, display_frame, next)
                        {
                            *project_changed = true;
                        }
                        if out_resp.drag_stopped() {
                            ui.ctx()
                                .data_mut(|data| data.remove::<[f32; 4]>(out_drag_id));
                        }
                        if in_resp.drag_stopped() {
                            ui.ctx()
                                .data_mut(|data| data.remove::<[f32; 4]>(in_drag_id));
                        }
                    }
                    if key_response.drag_started() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                channel_drag_id,
                                GraphKeyframeDrag {
                                    original_frame: *frame,
                                    current_frame: *frame,
                                    original_value: *value,
                                    current_value: *value,
                                },
                            )
                        });
                    }
                    if key_response.secondary_clicked() {
                        if remove_effect_channel_at_frame(layer, &graph_prop, display_frame) {
                            *project_changed = true;
                        }
                        continue;
                    }
                    if key_response.dragged() {
                        let state = ui
                            .ctx()
                            .data(|d| d.get_temp::<GraphKeyframeDrag>(channel_drag_id))
                            .unwrap_or(GraphKeyframeDrag {
                                original_frame: *frame,
                                current_frame: *frame,
                                original_value: *value,
                                current_value: *value,
                            });
                        let next_frame = (state.original_frame as i32
                            + (key_response.drag_delta().x / rect.width() * total_f as f32).round()
                                as i32)
                            .clamp(0, total_f as i32)
                            as u32;
                        let next_value = state.original_value
                            - key_response.drag_delta().y / (rect.height() - 8.0) * val_range;
                        let target_frame = if next_frame == state.current_frame
                            || move_effect_channel_keyframe(
                                layer,
                                &graph_prop,
                                state.current_frame,
                                next_frame,
                            ) {
                            next_frame
                        } else {
                            state.current_frame
                        };
                        let value_changed = set_effect_channel_at_frame(
                            layer,
                            &graph_prop,
                            target_frame,
                            next_value,
                        );
                        *project_changed |= target_frame != state.current_frame || value_changed;
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                channel_drag_id,
                                GraphKeyframeDrag {
                                    current_frame: target_frame,
                                    current_value: next_value,
                                    ..state
                                },
                            )
                        });
                    }
                    if key_response.drag_stopped() {
                        ui.ctx()
                            .data_mut(|d| d.remove::<GraphKeyframeDrag>(channel_drag_id));
                    }
                }
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

                // --- Anchor point: drag horizontally to retime, vertically to change value ---
                let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
                #[cfg(test)]
                ui.ctx().data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new(("ae_graph_anchor_rect", &layer.id, &graph_prop, *kf_frame)),
                        anchor_rect,
                    );
                });
                if !ui.is_rect_visible(anchor_rect) && drag_display.is_none() {
                    continue;
                }
                let anchor_token = drag_display
                    .map(|drag| drag.original_frame as usize)
                    .unwrap_or(*kf_frame as usize);
                let anchor_resp = ui.interact(
                    anchor_rect,
                    egui::Id::new(("graph_anchor", &layer.id, &graph_prop, anchor_token)),
                    egui::Sense::click_and_drag(),
                );
                if anchor_resp.hovered() {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            egui::Id::new(("ae_graph_hovered_kf", &layer.id, &graph_prop)),
                            *kf_idx,
                        )
                    });
                }
                if anchor_resp.secondary_clicked() {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(index) = kfs.iter().position(|key| key.frame == *kf_frame) {
                            kfs.remove(index);
                            *project_changed = true;
                        }
                    });
                    continue;
                }
                if anchor_resp.drag_started() {
                    let state = GraphKeyframeDrag {
                        original_frame: *kf_frame,
                        current_frame: *kf_frame,
                        original_value: *kf_val,
                        current_value: *kf_val,
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(drag_state_id, state));
                }
                if anchor_resp.dragged() {
                    let state = ui
                        .ctx()
                        .data(|d| d.get_temp::<GraphKeyframeDrag>(drag_state_id))
                        .unwrap_or(GraphKeyframeDrag {
                            original_frame: *kf_frame,
                            current_frame: *kf_frame,
                            original_value: *kf_val,
                            current_value: *kf_val,
                        });
                    let delta_frames =
                        (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
                    let new_frame = (state.original_frame as i32 + delta_frames)
                        .clamp(0, total_f as i32) as u32;
                    let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
                    let new_value = state.original_value + delta_val;
                    let axis = if graph_prop.starts_with("3D ") {
                        axis_3d(&graph_prop)
                    } else {
                        usize::from(graph_prop.ends_with('Y'))
                    };

                    let changed = if is_effect_property(&graph_prop) && !graph_prop.contains('|') {
                        let moved = state.current_frame != new_frame
                            && move_effect_scalar_keyframe(
                                layer,
                                &graph_prop,
                                state.current_frame,
                                new_frame,
                            );
                        let updated =
                            set_effect_channel_at_frame(layer, &graph_prop, new_frame, new_value);
                        moved || updated
                    } else if graph_prop == "Rotation" {
                        move_and_set_channel(
                            &mut layer.transform.rotation,
                            state.current_frame,
                            new_frame,
                            0,
                            new_value,
                        )
                    } else if graph_prop == "Opacity" {
                        move_and_set_channel(
                            &mut layer.transform.opacity,
                            state.current_frame,
                            new_frame,
                            0,
                            new_value.clamp(0.0, 100.0),
                        )
                    } else if graph_prop.starts_with("3D Position") {
                        move_and_set_channel(
                            &mut layer.transform_3d.position,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("3D Rotation") {
                        move_and_set_channel(
                            &mut layer.transform_3d.rotation,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("3D Scale") {
                        move_and_set_channel(
                            &mut layer.transform_3d.scale,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("Position") {
                        move_and_set_channel(
                            &mut layer.transform.position,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("Scale") {
                        move_and_set_channel(
                            &mut layer.transform.scale,
                            state.current_frame,
                            new_frame,
                            axis,
                            new_value,
                        )
                    } else if graph_prop.starts_with("PinX:") || graph_prop.starts_with("PinY:") {
                        pin_anim_mut(layer, &graph_prop)
                            .map(|pin| {
                                move_and_set_channel(
                                    pin,
                                    state.current_frame,
                                    new_frame,
                                    axis,
                                    new_value,
                                )
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    if changed {
                        *project_changed = true;
                    }
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            drag_state_id,
                            GraphKeyframeDrag {
                                current_frame: new_frame,
                                current_value: new_value,
                                ..state
                            },
                        )
                    });
                }
                if anchor_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|d| d.remove::<GraphKeyframeDrag>(drag_state_id));
                }
                let anchor_color = if anchor_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else if anchor_resp.hovered() {
                    colors::HANDLE_HOVER_FILL
                } else {
                    colors::TIMELINE_KEYFRAME
                };
                ui.painter().circle_filled(pt, 4.0, anchor_color);

                // ── Double-click anchor → numeric value popup ──
                let dbl_id =
                    ui.make_persistent_id(("graph_kf_popup", &layer.id, &graph_prop, anchor_token));
                let mut show_popup: bool = ui
                    .ctx()
                    .data_mut(|d| *d.get_temp_mut_or_insert_with(dbl_id, || false));
                if anchor_resp.double_clicked() {
                    show_popup = true;
                    let frame_edit_id = egui::Id::new((
                        "graph_kf_frame_text",
                        &layer.id,
                        &graph_prop,
                        anchor_token,
                    ));
                    let value_edit_id = egui::Id::new((
                        "graph_kf_value_text",
                        &layer.id,
                        &graph_prop,
                        anchor_token,
                    ));
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(frame_edit_id, kf_frame.to_string());
                        data.insert_temp(value_edit_id, format!("{:.2}", kf_val));
                    });
                }
                if show_popup {
                    let popup_id =
                        egui::Id::new(("graph_kf_val_popup", &layer.id, &graph_prop, anchor_token));
                    let resp = egui::Area::new(popup_id)
                        .fixed_pos(pt + egui::vec2(12.0, -20.0))
                        .order(egui::Order::Foreground)
                        .show(ui.ctx(), |ui| {
                            ui.group(|ui| {
                                ui.set_min_width(140.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Frame:").small());
                                    let frame_edit_id = egui::Id::new((
                                        "graph_kf_frame_text",
                                        &layer.id,
                                        &graph_prop,
                                        anchor_token,
                                    ));
                                    let mut frame_str = ui.ctx().data(|data| {
                                        data.get_temp::<String>(frame_edit_id)
                                            .unwrap_or_else(|| kf_frame.to_string())
                                    });
                                    let frame_response = ui.add(
                                        egui::TextEdit::singleline(&mut frame_str)
                                            .desired_width(50.0),
                                    );
                                    if frame_response.changed() {
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(frame_edit_id, frame_str.clone())
                                        });
                                    }
                                    if frame_response.lost_focus()
                                        || ui.input(|input| input.key_pressed(egui::Key::Enter))
                                    {
                                        if let Ok(f) = frame_str.parse::<u32>() {
                                            let new_f = f.min(total_f);
                                            let handled_effect_move =
                                                if is_effect_property(&graph_prop)
                                                    && new_f != *kf_frame
                                                {
                                                    if graph_prop.contains('|') {
                                                        move_effect_channel_keyframe(
                                                            layer,
                                                            &graph_prop,
                                                            *kf_frame,
                                                            new_f,
                                                        )
                                                    } else {
                                                        move_effect_scalar_keyframe(
                                                            layer,
                                                            &graph_prop,
                                                            *kf_frame,
                                                            new_f,
                                                        )
                                                    }
                                                } else if graph_prop == "Rotation"
                                                    && new_f != *kf_frame
                                                {
                                                    layer
                                                        .transform
                                                        .rotation
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Opacity"
                                                    && new_f != *kf_frame
                                                {
                                                    layer
                                                        .transform
                                                        .opacity
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Position X"
                                                    || graph_prop == "Position Y"
                                                {
                                                    layer
                                                        .transform
                                                        .position
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop == "Scale X"
                                                    || graph_prop == "Scale Y"
                                                {
                                                    layer
                                                        .transform
                                                        .scale
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Position") {
                                                    layer
                                                        .transform_3d
                                                        .position
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Rotation") {
                                                    layer
                                                        .transform_3d
                                                        .rotation
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("3D Scale") {
                                                    layer
                                                        .transform_3d
                                                        .scale
                                                        .move_keyframe(*kf_frame, new_f)
                                                } else if graph_prop.starts_with("PinX:")
                                                    || graph_prop.starts_with("PinY:")
                                                {
                                                    pin_anim_mut(layer, &graph_prop).is_some_and(
                                                        |pin| pin.move_keyframe(*kf_frame, new_f),
                                                    )
                                                } else {
                                                    false
                                                };
                                            if handled_effect_move {
                                                *project_changed = true;
                                            }
                                            ui.ctx().data_mut(|data| {
                                                data.insert_temp(frame_edit_id, new_f.to_string())
                                            });
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Value:").small());
                                    let value_edit_id = egui::Id::new((
                                        "graph_kf_value_text",
                                        &layer.id,
                                        &graph_prop,
                                        anchor_token,
                                    ));
                                    let mut val_str = ui.ctx().data(|data| {
                                        data.get_temp::<String>(value_edit_id)
                                            .unwrap_or_else(|| format!("{:.2}", kf_val))
                                    });
                                    let value_response = ui.add(
                                        egui::TextEdit::singleline(&mut val_str)
                                            .desired_width(70.0),
                                    );
                                    if value_response.changed() {
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(value_edit_id, val_str.clone())
                                        });
                                    }
                                    if value_response.lost_focus()
                                        || ui.input(|input| input.key_pressed(egui::Key::Enter))
                                    {
                                        if let Ok(v) = val_str.parse::<f32>() {
                                            if graph_prop.starts_with("Position") {
                                                let ci =
                                                    if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) =
                                                    layer.transform.position.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[ci] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("Scale") {
                                                let ci =
                                                    if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) =
                                                    layer.transform.scale.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[ci] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop == "Rotation" {
                                                if let Some(kfs) =
                                                    layer.transform.rotation.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop == "Opacity" {
                                                if let Some(kfs) =
                                                    layer.transform.opacity.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value = v.clamp(0.0, 100.0);
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Position") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.position.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Rotation") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.rotation.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if graph_prop.starts_with("3D Scale") {
                                                if let Some(kfs) =
                                                    layer.transform_3d.scale.keyframes_mut()
                                                {
                                                    if let Some(kf) = kfs
                                                        .iter_mut()
                                                        .find(|kf| kf.frame == *kf_frame)
                                                    {
                                                        kf.value[axis_3d(&graph_prop)] = v;
                                                        *project_changed = true;
                                                    }
                                                }
                                            } else if is_effect_property(&graph_prop)
                                                && set_effect_channel_at_frame(
                                                    layer,
                                                    &graph_prop,
                                                    *kf_frame,
                                                    v,
                                                )
                                            {
                                                *project_changed = true;
                                            }
                                        }
                                    }
                                });
                                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                                    show_popup = false;
                                }
                                if ui.button("Done").clicked() {
                                    show_popup = false;
                                }
                            });
                        });
                    // Click outside popup → dismiss
                    if ui.input(|i| i.pointer.any_click()) && !resp.response.contains_pointer() {
                        show_popup = false;
                    }
                }
                ui.ctx().data_mut(|d| d.insert_temp(dbl_id, show_popup));
                if anchor_resp.hovered() {
                    ui.painter().circle_stroke(
                        pt,
                        7.0,
                        egui::Stroke::new(1.0_f32, colors::TIMELINE_KEYFRAME),
                    );
                }

                // --- Tangent handles: drag to edit custom bezier control points ---
                // Extract current bezier points (default Easy Ease if linear/hold)
                fn bezier_pts<T>(kf: &crate::core::keyframe::Keyframe<T>) -> (f32, f32, f32, f32) {
                    match &kf.interpolation {
                        InterpolationType::Bezier {
                            custom_bezier: Some(pts),
                            ..
                        } => (pts[0], pts[1], pts[2], pts[3]),
                        _ => (0.33, 0.0, 0.67, 1.0),
                    }
                }
                let (bx1, by1, bx2, by2): (f32, f32, f32, f32) =
                    with_keyframes!(layer, graph_prop, kfs => {
                        kfs.iter()
                            .find(|key| key.frame == display_frame)
                            .map(bezier_pts)
                            .unwrap_or((0.33, 0.0, 0.67, 1.0))
                    })
                    .unwrap_or((0.33, 0.0, 0.67, 1.0));

                let h_out = egui::pos2(pt.x + bx2 * 44.0, pt.y - by2 * 24.0);
                let h_in = egui::pos2(pt.x - bx1 * 44.0, pt.y + by1 * 24.0);

                let h_out_rect = egui::Rect::from_center_size(h_out, egui::vec2(14.0, 14.0));
                let h_in_rect = egui::Rect::from_center_size(h_in, egui::vec2(14.0, 14.0));
                let h_out_id = egui::Id::new(("graph_h_out", &layer.id, &graph_prop, anchor_token));
                let h_in_id = egui::Id::new(("graph_h_in", &layer.id, &graph_prop, anchor_token));
                let h_out_drag_id = egui::Id::new((
                    "graph_bezier_drag_out",
                    &layer.id,
                    &graph_prop,
                    anchor_token,
                ));
                let h_in_drag_id =
                    egui::Id::new(("graph_bezier_drag_in", &layer.id, &graph_prop, anchor_token));
                let h_out_resp = ui.interact(h_out_rect, h_out_id, egui::Sense::drag());
                let h_in_resp = ui.interact(h_in_rect, h_in_id, egui::Sense::drag());

                let mut new_pts: Option<[f32; 4]> = None;
                if h_out_resp.drag_started() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(h_out_drag_id, [bx1, by1, bx2, by2]));
                }
                if h_in_resp.drag_started() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(h_in_drag_id, [bx1, by1, bx2, by2]));
                }
                let out_base = ui
                    .ctx()
                    .data(|data| data.get_temp::<[f32; 4]>(h_out_drag_id))
                    .unwrap_or([bx1, by1, bx2, by2]);
                let in_base = ui
                    .ctx()
                    .data(|data| data.get_temp::<[f32; 4]>(h_in_drag_id))
                    .unwrap_or([bx1, by1, bx2, by2]);
                if h_out_resp.dragged() {
                    let d = h_out_resp.drag_delta();
                    let nx2 = clamp_bezier_x(out_base[2] + d.x / 44.0, out_base[0] + 0.01, 1.0);
                    let ny2 = (out_base[3] - d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = clamp_bezier_x(1.0 - nx2, 0.0, nx2 - 0.01);
                        new_pts = Some([mir_x, -ny2, nx2, ny2]);
                    } else {
                        new_pts = Some([out_base[0], out_base[1], nx2, ny2]);
                    }
                }
                if h_in_resp.dragged() {
                    let d = h_in_resp.drag_delta();
                    let nx1 = clamp_bezier_x(in_base[0] - d.x / 44.0, 0.0, in_base[2] - 0.01);
                    let ny1 = (in_base[1] + d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = clamp_bezier_x(1.0 - nx1, nx1 + 0.01, 1.0);
                        new_pts = Some([nx1, ny1, mir_x, -ny1]);
                    } else {
                        new_pts = Some([nx1, ny1, in_base[2], in_base[3]]);
                    }
                }
                if let Some(pts) = new_pts {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(kf) = kfs.iter_mut().find(|key| key.frame == display_frame) {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some(pts),
                            };
                            *project_changed = true;
                        }
                    });
                }
                if h_out_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|data| data.remove::<[f32; 4]>(h_out_drag_id));
                }
                if h_in_resp.drag_stopped() {
                    ui.ctx()
                        .data_mut(|data| data.remove::<[f32; 4]>(h_in_drag_id));
                }

                let any_hover = h_out_resp.hovered()
                    || h_in_resp.dragged()
                    || h_in_resp.hovered()
                    || h_out_resp.dragged();
                let stroke_color = if any_hover {
                    colors::ACCENT_ORANGE
                } else {
                    colors::MOTION_PATH
                };
                ui.painter()
                    .line_segment([pt, h_out], egui::Stroke::new(1.2_f32, stroke_color));
                ui.painter()
                    .line_segment([pt, h_in], egui::Stroke::new(1.2_f32, stroke_color));

                let h_out_color = if h_out_resp.hovered() || h_out_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else {
                    colors::MOTION_PATH
                };
                let h_in_color = if h_in_resp.hovered() || h_in_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else {
                    colors::MOTION_PATH
                };
                ui.painter().circle_filled(h_out, 4.0, h_out_color);
                ui.painter().circle_filled(h_in, 4.0, h_in_color);
            }
        }
    });
}
