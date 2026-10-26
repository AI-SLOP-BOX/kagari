use super::super::mask_tools::move_mask_vertices;
use crate::app_state::KagariApp;
use crate::core::property::Animatable;
use eframe::egui;

pub fn handle_active_drag_step(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    pointer_pos: egui::Pos2,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
    current_frame: u32,
) {
    if let Some((l_idx, kf_frame, start_val, start_ptr)) = app.viewport_pos_kf_drag_state {
        let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;

        let comp_mut = app.history.current_mut().active_composition_mut();
        if let Some(layer) = comp_mut.layers.get_mut(l_idx) {
            if let Some(kfs) = layer.transform.position.keyframes_mut() {
                if let Some(kf) = kfs.iter_mut().find(|k| k.frame == kf_frame) {
                    kf.value = [start_val[0] + delta_x, start_val[1] + delta_y];
                }
            }
        }
    } else if let Some((l_idx, kf_frame, handle_type, start_bez, start_ptr)) =
        app.viewport_tangent_drag_state
    {
        let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;

        let comp_mut = app.history.current_mut().active_composition_mut();
        if let Some(layer) = comp_mut.layers.get_mut(l_idx) {
            if let Some(kfs) = layer.transform.position.keyframes_mut() {
                let mut update: Option<(usize, [f32; 4])> = None;
                for i in 0..kfs.len() {
                    if kfs[i].frame == kf_frame {
                        if let crate::core::keyframe::InterpolationType::Bezier {
                            custom_bezier: Some(ref pts),
                            ..
                        } = kfs[i].interpolation
                        {
                            let mut new_pts = *pts;
                            if handle_type == 0 && i + 1 < kfs.len() {
                                let val_delta = [
                                    kfs[i + 1].value[0] - kfs[i].value[0],
                                    kfs[i + 1].value[1] - kfs[i].value[1],
                                ];
                                let normalized = |base: f32, delta: f32, span: f32| {
                                    if span.abs() > f32::EPSILON {
                                        (base * span + delta) / span
                                    } else {
                                        base
                                    }
                                };
                                let new_out_x = normalized(start_bez[0], delta_x, val_delta[0])
                                    .clamp(0.01, 0.99);
                                let new_out_y = normalized(start_bez[1], delta_y, val_delta[1])
                                    .clamp(-1.5, 2.5);
                                if app.viewport_linked_tangent {
                                    new_pts[2] = 1.0 - new_out_x;
                                    new_pts[3] = -new_out_y;
                                }
                                new_pts[0] = new_out_x;
                                new_pts[1] = new_out_y;
                                update = Some((i, new_pts));
                            } else if handle_type == 1 && i > 0 {
                                let val_delta = [
                                    kfs[i].value[0] - kfs[i - 1].value[0],
                                    kfs[i].value[1] - kfs[i - 1].value[1],
                                ];
                                let normalized = |base: f32, delta: f32, span: f32| {
                                    if span.abs() > f32::EPSILON {
                                        (base * span + delta) / span
                                    } else {
                                        base
                                    }
                                };
                                let new_in_x = normalized(start_bez[2], -delta_x, val_delta[0])
                                    .clamp(0.01, 0.99);
                                let new_in_y = normalized(start_bez[3], -delta_y, val_delta[1])
                                    .clamp(-1.5, 2.5);
                                if app.viewport_linked_tangent {
                                    new_pts[0] = 1.0 - new_in_x;
                                    new_pts[1] = -new_in_y;
                                }
                                new_pts[2] = new_in_x;
                                new_pts[3] = new_in_y;
                                update = Some((i, new_pts));
                            }
                            break;
                        }
                    }
                }
                if let Some((idx, pts)) = update {
                    kfs[idx].interpolation = crate::core::keyframe::InterpolationType::Bezier {
                        outgoing: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(pts),
                    };
                    crate::core::frame_cache::bump_version();
                }
            }
        }
    } else if let Some((l_idx, m_idx, _, _, start_ptr)) = app.viewport_mask_drag_state {
        let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;

        let comp_mut = app.history.current_mut().active_composition_mut();
        if l_idx < comp_mut.layers.len() {
            let layer = &mut comp_mut.layers[l_idx];
            if m_idx < layer.masks.len() {
                let mask = &mut layer.masks[m_idx];
                move_mask_vertices(
                    &mut mask.path,
                    current_frame,
                    &app.viewport_mask_drag_vertices,
                    [delta_x, delta_y],
                );
            }
        }
    } else if let Some((l_idx, m_idx, v_idx, is_out, start_handle, start_ptr)) =
        app.mask_tangent_drag_state
    {
        let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
        let new_handle = [start_handle[0] + delta_x, start_handle[1] + delta_y];
        let comp_mut = app.history.current_mut().active_composition_mut();
        if l_idx < comp_mut.layers.len() {
            let layer = &mut comp_mut.layers[l_idx];
            if m_idx < layer.masks.len() {
                let mask = &mut layer.masks[m_idx];
                let verts = mask.path.vertices_at_frame(current_frame);
                if v_idx < verts.len() {
                    super::super::mask_tools::move_mask_tangent(
                        &mut mask.path,
                        v_idx,
                        is_out,
                        new_handle,
                        ui.input(|i| i.modifiers.shift),
                    );
                    crate::core::frame_cache::bump_version();
                }
            }
        }
    } else if let Some((drag_idx, start_scale, start_dist)) = app.viewport_scale_drag {
        let pos = {
            let comp_state = app.history.current().active_composition();
            comp_state
                .layers
                .get(drag_idx)
                .map(|l| l.transform.position.evaluate(current_frame))
        };
        if let Some(pos) = pos {
            let cx = origin_x + (pos[0] / comp_w) * draw_w;
            let cy = origin_y + (pos[1] / comp_h) * draw_h;
            let cur_dist = ((pointer_pos.x - cx).powi(2) + (pointer_pos.y - cy).powi(2))
                .sqrt()
                .max(1.0);
            let factor = (cur_dist / start_dist).clamp(0.05, 50.0);
            let new_scl = [start_scale[0] * factor, start_scale[1] * factor];

            let comp_mut = app.history.current_mut().active_composition_mut();
            if drag_idx < comp_mut.layers.len() {
                let layer = &mut comp_mut.layers[drag_idx];
                match &mut layer.transform.scale {
                    Animatable::Constant(ref mut s) => *s = new_scl,
                    Animatable::Animated(ref mut kfs) => {
                        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == current_frame) {
                            kf.value = new_scl;
                        } else {
                            kfs.push(crate::core::keyframe::Keyframe::new(
                                current_frame,
                                new_scl,
                                crate::core::keyframe::InterpolationType::Linear,
                            ));
                            kfs.sort_by_key(|k| k.frame);
                        }
                    }
                }
            }
        }
    } else if let Some((starts, start_ptr)) = app.viewport_multi_drag.clone() {
        let mut delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let mut delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
        if ui.input(|i| i.modifiers.shift) {
            if delta_x.abs() >= delta_y.abs() {
                delta_y = 0.0;
            } else {
                delta_x = 0.0;
            }
        }
        let comp_mut = app.history.current_mut().active_composition_mut();
        for (li, start_pos) in &starts {
            if let Some(layer) = comp_mut.layers.get_mut(*li) {
                layer.transform.position = crate::core::property::Animatable::new_constant([
                    start_pos[0] + delta_x,
                    start_pos[1] + delta_y,
                ]);
            }
        }
    } else if let Some((drag_idx, start_pos, start_ptr)) = app.viewport_drag_state {
        let mut delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
        let mut delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
        if ui.input(|i| i.modifiers.shift) {
            if delta_x.abs() >= delta_y.abs() {
                delta_y = 0.0;
            } else {
                delta_x = 0.0;
            }
        }
        let new_pos = [start_pos[0] + delta_x, start_pos[1] + delta_y];

        let comp_mut = app.history.current_mut().active_composition_mut();
        if drag_idx < comp_mut.layers.len() {
            let layer = &mut comp_mut.layers[drag_idx];
            use crate::ui::toolbar::ActiveTool;
            match app.active_tool {
                ActiveTool::Rotation => {
                    let rot_delta = delta_x * 0.5;
                    let current_r = layer.transform.rotation.evaluate(current_frame);
                    match &mut layer.transform.rotation {
                        Animatable::Constant(ref mut r) => *r += rot_delta,
                        Animatable::Animated(ref mut kfs) => {
                            if let Some(kf) = kfs.iter_mut().find(|k| k.frame == current_frame) {
                                kf.value += rot_delta;
                            } else {
                                kfs.push(crate::core::keyframe::Keyframe::new(
                                    current_frame,
                                    current_r + rot_delta,
                                    crate::core::keyframe::InterpolationType::Linear,
                                ));
                                kfs.sort_by_key(|k| k.frame);
                            }
                        }
                    }
                }
                ActiveTool::AnchorPoint => {
                    let cur_ap = layer.transform.anchor_point.evaluate(current_frame);
                    match &mut layer.transform.anchor_point {
                        Animatable::Constant(ref mut ap) => {
                            ap[0] += delta_x;
                            ap[1] += delta_y;
                        }
                        Animatable::Animated(ref mut kfs) => {
                            if let Some(kf) = kfs.iter_mut().find(|k| k.frame == current_frame) {
                                kf.value[0] += delta_x;
                                kf.value[1] += delta_y;
                            } else {
                                kfs.push(crate::core::keyframe::Keyframe::new(
                                    current_frame,
                                    [cur_ap[0] + delta_x, cur_ap[1] + delta_y],
                                    crate::core::keyframe::InterpolationType::Linear,
                                ));
                                kfs.sort_by_key(|k| k.frame);
                            }
                        }
                    }
                }
                _ => {
                    match &mut layer.transform.position {
                        Animatable::Constant(ref mut pos) => {
                            if app.motion_sketch_active && app.playback.is_playing {
                                let old_pos = *pos;
                                let mut kfs = vec![
                                    crate::core::keyframe::Keyframe::new(
                                        0,
                                        old_pos,
                                        crate::core::keyframe::InterpolationType::Linear,
                                    ),
                                    crate::core::keyframe::Keyframe::new(
                                        current_frame,
                                        new_pos,
                                        crate::core::keyframe::InterpolationType::Linear,
                                    ),
                                ];
                                kfs.sort_by_key(|k| k.frame);
                                layer.transform.position = Animatable::Animated(kfs);
                            } else {
                                *pos = new_pos;
                            }
                        }
                        Animatable::Animated(ref mut keyframes) => {
                            let existing_idx =
                                keyframes.iter().position(|kf| kf.frame == current_frame);
                            if let Some(idx) = existing_idx {
                                keyframes[idx].value = new_pos;
                            } else {
                                keyframes.push(crate::core::keyframe::Keyframe::new(
                                    current_frame,
                                    new_pos,
                                    crate::core::keyframe::InterpolationType::Linear,
                                ));
                                keyframes.sort_by_key(|kf| kf.frame);
                            }
                        }
                    }
                    if app.motion_sketch_active {
                        crate::core::motion_sketch::record_motion_sketch_sample(
                            &mut app.motion_sketch_recording,
                            current_frame,
                            new_pos,
                        );
                    }
                }
            }
        }
        ctx.request_repaint();
    }
}
