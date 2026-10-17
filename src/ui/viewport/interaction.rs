use eframe::egui;
use crate::app_state::KagariApp;
use crate::core::property::Animatable;
use crate::core::timeline::Layer;
use super::mask_tools::{move_mask_vertices, move_mask_tangent};

pub struct InteractionContext {
    pub origin_x: f32,
    pub origin_y: f32,
    pub draw_w: f32,
    pub draw_h: f32,
    pub comp_w: f32,
    pub comp_h: f32,
    pub current_frame: u32,
    pub draw_rect: egui::Rect,
}

/// Handles all interactive pointer events (layer drag, mask drag/tangents, scale drag,
/// motion sketch sample recording, pen point addition, shape/text layer creation on drag end).
/// Returns true if a pen stroke was committed.
pub fn handle_viewport_interaction(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    viewport_response: &egui::Response,
    geom: InteractionContext,
) -> bool {
    let mut pen_commit = false;
    let InteractionContext {
        origin_x,
        origin_y,
        draw_w,
        draw_h,
        comp_w,
        comp_h,
        current_frame,
        draw_rect,
    } = geom;

    if let Some(pointer_pos) = viewport_response.interact_pointer_pos() {
        let comp_px = (pointer_pos.x - origin_x) / draw_w * comp_w;
        let comp_py = (pointer_pos.y - origin_y) / draw_h * comp_h;

        // ── Double-click a text layer → inline source-text editing (AE parity) ──
        if viewport_response.double_clicked() && app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
            let comp_state = app.history.current().active_composition();
            for (i, l) in comp_state.layers.iter().enumerate().rev() {
                if !l.is_active(current_frame) || l.locked { continue; }
                if matches!(l.layer_type, crate::core::timeline::LayerType::Text { .. }) {
                    let pos = l.transform.position.evaluate(current_frame);
                    let scale = l.transform.scale.evaluate(current_frame);
                    let hw = (scale[0].abs() * 0.6).max(60.0);
                    let hh = (scale[1].abs() * 0.6).max(30.0);
                    if (comp_px - pos[0]).abs() <= hw && (comp_py - pos[1]).abs() <= hh {
                        app.inline_text_edit_layer = Some(i);
                        app.selection.selected_layer_idx = Some(i);
                        break;
                    }
                }
            }
        }

        // ── Rectangle tool: track drag start for rubber-band preview ──
        if viewport_response.drag_started() && app.active_tool == crate::ui::toolbar::ActiveTool::Rectangle {
            app.rect_drag_start = Some(pointer_pos);
        }

        // Direct transform tools: Selection, Rotation, AnchorPoint
        // Navigation and Creation tools have dedicated interaction paths and must not move layers.
        let is_transform_tool = matches!(
            app.active_tool,
            crate::ui::toolbar::ActiveTool::Selection
                | crate::ui::toolbar::ActiveTool::Rotation
                | crate::ui::toolbar::ActiveTool::AnchorPoint
        );
        if (viewport_response.drag_started() || viewport_response.clicked())
            && is_transform_tool
            && !ui.input(|i| i.modifiers.alt)
        {
            if viewport_response.drag_started() {
                app.begin_drag("Viewport Transform");
            }

            let comp_state = app.history.current().active_composition();
            let mut mask_hit: Option<(usize, usize, usize)> = None;
            let mut mask_tangent_hit: Option<(usize, usize, usize, bool, [f32; 2])> = None;

            // 1. Check if clicking on selected layer's mask tangent handles
            if app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
                if let Some(sel_li) = app.selection.selected_layer_idx {
                    if sel_li < comp_state.layers.len() {
                        let l = &comp_state.layers[sel_li];
                        for (mi, mask) in l.masks.iter().enumerate() {
                            if mask.enabled {
                                let verts = mask.path.vertices_at_frame(current_frame);
                                if let Some(tangents) = &mask.path.tangents {
                                    for (vi, vertex_pt) in verts.iter().enumerate() {
                                        if vi < tangents.len() {
                                            let (t_in, t_out) = tangents[vi];
                                            // Check outgoing handle
                                            if t_out[0] != 0.0 || t_out[1] != 0.0 {
                                                let hx = origin_x + ((vertex_pt[0] + t_out[0]) / comp_w) * draw_w;
                                                let hy = origin_y + ((vertex_pt[1] + t_out[1]) / comp_h) * draw_h;
                                                let dist = ((pointer_pos.x - hx).powi(2) + (pointer_pos.y - hy).powi(2)).sqrt();
                                                if dist <= 8.0 {
                                                    mask_tangent_hit = Some((sel_li, mi, vi, true, t_out));
                                                    break;
                                                }
                                            }
                                            // Check incoming handle
                                            if t_in[0] != 0.0 || t_in[1] != 0.0 {
                                                let hx = origin_x + ((vertex_pt[0] + t_in[0]) / comp_w) * draw_w;
                                                let hy = origin_y + ((vertex_pt[1] + t_in[1]) / comp_h) * draw_h;
                                                let dist = ((pointer_pos.x - hx).powi(2) + (pointer_pos.y - hy).powi(2)).sqrt();
                                                if dist <= 8.0 {
                                                    mask_tangent_hit = Some((sel_li, mi, vi, false, t_in));
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if mask_tangent_hit.is_some() { break; }
                        }
                    }
                }
            }

            // 1b. Check if clicking on selected layer's mask vertices (only if no tangent hit)
            if mask_tangent_hit.is_none() {
                if let Some(sel_li) = app.selection.selected_layer_idx {
                    if sel_li < comp_state.layers.len() {
                        let l = &comp_state.layers[sel_li];
                        for (mi, mask) in l.masks.iter().enumerate() {
                            if mask.enabled {
                                let verts = mask.path.vertices_at_frame(current_frame);
                                for (vi, vertex_pt) in verts.iter().enumerate() {
                                    let vx = vertex_pt[0];
                                    let vy = vertex_pt[1];

                                    // Calculate viewport screen position
                                    let screen_x = origin_x + (vx / comp_w) * draw_w;
                                    let screen_y = origin_y + (vy / comp_h) * draw_h;
                                    let dist = ((pointer_pos.x - screen_x).powi(2) + (pointer_pos.y - screen_y).powi(2)).sqrt();
                                    if dist <= 12.0 {
                                        mask_hit = Some((sel_li, mi, vi));
                                        break;
                                    }
                                }
                            }
                            if mask_hit.is_some() { break; }
                        }
                    }
                }
            }

            // 0. Position keyframe dots on the motion path (spatial editing)
            let kf_hit: Option<(usize, u32, [f32; 2])> =
                if app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
                    app.selection.selected_layer_idx.and_then(|sel_li| {
                        let l = comp_state.layers.get(sel_li)?;
                        if l.locked || !l.is_active(current_frame) {
                            return None;
                        }
                        let kfs = l.transform.position.keyframes()?;
                        kfs.iter().find_map(|kf| {
                            let sx = origin_x + (kf.value[0] / comp_w) * draw_w;
                            let sy = origin_y + (kf.value[1] / comp_h) * draw_h;
                            let dist =
                                ((pointer_pos.x - sx).powi(2) + (pointer_pos.y - sy).powi(2)).sqrt();
                            (dist <= 8.0).then_some((kf.frame, kf.value))
                        }).map(|(frame, val)| (sel_li, frame, val))
                    })
                } else {
                    None
                };

            // 1a-tangent. Bezier tangent handle hit detection
            let tangent_hit: Option<(usize, u32, u8, [f32; 4])> = if app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
                if let Some(sel) = app.selection.selected_layer_idx {
                    if sel < comp_state.layers.len() {
                        let l = &comp_state.layers[sel];
                        let pos_kfs = l.transform.position.keyframes().unwrap_or(&[]);
                        if pos_kfs.len() >= 2 {
                            let aspect = comp_w / comp_h;
                            let (ox, oy, dw, dh) = crate::ui::viewport_state::compute_draw_layout_pan(draw_rect, aspect, app.ui_tabs.viewport_mag_ratio, app.playback.viewport_pan);
                            let to_scr = |v: [f32; 2]| -> eframe::egui::Pos2 {
                                egui::pos2(ox + (v[0] / comp_w) * dw, oy + (v[1] / comp_h) * dh)
                            };
                            let mut found = None;
                            'outer: for seg in pos_kfs.windows(2) {
                                let (a, b) = (&seg[0], &seg[1]);
                                if let crate::core::keyframe::InterpolationType::Bezier { custom_bezier, .. } = &a.interpolation {
                                    let c = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                                    let val_delta = [b.value[0] - a.value[0], b.value[1] - a.value[1]];
                                    let out_pos = [a.value[0] + c[0] * val_delta[0], a.value[1] + c[1] * val_delta[1]];
                                    let out_screen = to_scr(out_pos);
                                    let d_out = ((pointer_pos.x - out_screen.x).powi(2) + (pointer_pos.y - out_screen.y).powi(2)).sqrt();
                                    if d_out <= 8.0 {
                                        found = Some((sel, a.frame, 0u8, c));
                                        break 'outer;
                                    }
                                    let in_pos = [b.value[0] - (1.0 - c[2]) * val_delta[0], b.value[1] - (1.0 - c[3]) * val_delta[1]];
                                    let in_screen = to_scr(in_pos);
                                    let d_in = ((pointer_pos.x - in_screen.x).powi(2) + (pointer_pos.y - in_screen.y).powi(2)).sqrt();
                                    if d_in <= 8.0 {
                                        found = Some((sel, b.frame, 1u8, c));
                                        break 'outer;
                                    }
                                }
                            }
                            found
                        } else { None }
                    } else { None }
                } else { None }
            } else { None };

            if let Some((l_idx, kf_frame, handle_type, start_bez)) = tangent_hit {
                app.viewport_tangent_drag_state = Some((l_idx, kf_frame, handle_type, start_bez, pointer_pos));
                app.viewport_pos_kf_drag_state = None;
                app.viewport_drag_state = None;
                app.viewport_mask_drag_state = None;
            } else if let Some((l_idx, kf_frame, kf_start)) = kf_hit {
                app.viewport_pos_kf_drag_state =
                    Some((l_idx, kf_frame, kf_start, pointer_pos));
                app.viewport_drag_state = None;
                app.viewport_mask_drag_state = None;
            } else if let Some((l_idx, m_idx, v_idx, is_out, start_handle)) = mask_tangent_hit {
                app.mask_tangent_drag_state = Some((l_idx, m_idx, v_idx, is_out, start_handle, pointer_pos));
                app.viewport_drag_state = None;
                app.viewport_mask_drag_state = None;
            } else if let Some((l_idx, m_idx, v_idx)) = mask_hit {
                let shift_held = ui.input(|i| i.modifiers.shift);
                // Multi-vertex selection: Shift+click toggles vertex in selection set
                if shift_held {
                    if let Some(ref mut sel) = app.mask_selected_vertices {
                        if sel.0 == l_idx && sel.1 == m_idx {
                            if sel.2.contains(&v_idx) {
                                sel.2.remove(&v_idx);
                            } else {
                                sel.2.insert(v_idx);
                            }
                        } else {
                            app.mask_selected_vertices = Some((l_idx, m_idx, vec![v_idx].into_iter().collect()));
                        }
                    } else {
                        app.mask_selected_vertices = Some((l_idx, m_idx, vec![v_idx].into_iter().collect()));
                    }
                } else {
                    // Check if clicking on already-selected vertex set
                    let already_selected = app.mask_selected_vertices.as_ref()
                        .map(|s| s.0 == l_idx && s.1 == m_idx && s.2.contains(&v_idx))
                        .unwrap_or(false);
                    if !already_selected {
                        app.mask_selected_vertices = Some((l_idx, m_idx, vec![v_idx].into_iter().collect()));
                    }
                }
                // Store the starting positions of all selected vertices for multi-drag
                let selected_set = app.mask_selected_vertices.as_ref()
                    .filter(|s| s.0 == l_idx && s.1 == m_idx)
                    .map(|s| s.2.clone())
                    .unwrap_or_else(|| vec![v_idx].into_iter().collect());
                let verts = comp_state.layers[l_idx].masks[m_idx].path.vertices_at_frame(current_frame);
                app.viewport_mask_drag_vertices = selected_set.iter()
                    .filter_map(|&vi| verts.get(vi).map(|&pos| (vi, pos))).collect();
                let start_vertex_pos = if v_idx < verts.len() { verts[v_idx] } else { [0.0, 0.0] };
                app.viewport_mask_drag_state = Some((l_idx, m_idx, v_idx, start_vertex_pos, pointer_pos));
                app.viewport_drag_state = None;
            } else {
                app.viewport_mask_drag_state = None;
                // Clear mask vertex selection when clicking empty space
                if app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
                    app.mask_selected_vertices = None;
                }

                // 1b. Corner handles of the selected layer → scale drag (Selection tool)
                use crate::ui::toolbar::ActiveTool;
                let mut scale_hit = false;
                if app.active_tool == ActiveTool::Selection {
                    if let Some(sel) = app.selection.selected_layer_idx {
                        if sel < comp_state.layers.len() && !comp_state.layers[sel].locked {
                            let l = &comp_state.layers[sel];
                            if l.is_active(current_frame) {
                                let pos = l.transform.position.evaluate(current_frame);
                                let scl = l.transform.scale.evaluate(current_frame);
                                let cx = origin_x + (pos[0] / comp_w) * draw_w;
                                let cy = origin_y + (pos[1] / comp_h) * draw_h;
                                let sxw = (scl[0].abs() * 0.6) / comp_w * draw_w;
                                let syh = (scl[1].abs() * 0.6) / comp_h * draw_h;
                                for hx in [cx - sxw, cx + sxw] {
                                    for hy in [cy - syh, cy + syh] {
                                        let d2 = (pointer_pos.x - hx).powi(2) + (pointer_pos.y - hy).powi(2);
                                        if d2 <= 100.0 {
                                            let dist = ((pointer_pos.x - cx).powi(2) + (pointer_pos.y - cy).powi(2)).sqrt().max(1.0);
                                            app.viewport_scale_drag = Some((sel, scl, dist));
                                            app.viewport_drag_state = None;
                                            scale_hit = true;
                                            break;
                                        }
                                    }
                                    if scale_hit { break; }
                                }
                            }
                        }
                    }
                }

                // 2. Fallback to Layer translation/rotation drag
                if !scale_hit {
                    let mut hit_idx: Option<usize> = None;
                    for (i, layer) in comp_state.layers.iter().enumerate().rev() {
                        let l: &Layer = layer;
                        if !l.is_active(current_frame) || l.locked { continue; }
                        let pos = l.transform.position.evaluate(current_frame);
                        let scale = l.transform.scale.evaluate(current_frame);
                        let hw = scale[0].abs() * 0.6;
                        let hh = scale[1].abs() * 0.6;
                        if (comp_px - pos[0]).abs() <= hw && (comp_py - pos[1]).abs() <= hh {
                            hit_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = hit_idx {
                        let shift_held = ui.input(|i| i.modifiers.shift);
                        // ── Selection bookkeeping (syncs timeline multi-select) ──
                        if shift_held {
                            if !app.selection.selected_layers.insert(idx) {
                                app.selection.selected_layers.remove(&idx);
                            }
                            app.selection.selected_layer_idx = Some(idx);
                        } else if !app.selection.selected_layers.contains(&idx) {
                            app.selection.selected_layers.clear();
                            app.selection.selected_layers.insert(idx);
                            app.selection.selected_layer_idx = Some(idx);
                        }

                        // ── Group drag: hit layer part of a multi-selection → move all together ──
                        if app.active_tool == crate::ui::toolbar::ActiveTool::Selection && app.selection.selected_layers.len() > 1 && app.selection.selected_layers.contains(&idx) {
                            let starts: Vec<(usize, [f32; 2])> = app.selection.selected_layers.iter()
                                .filter_map(|&li| {
                                    let l = comp_state.layers.get(li)?;
                                    if l.locked { return None; }
                                    Some((li, l.transform.position.evaluate(current_frame)))
                                })
                                .collect();
                            if starts.len() > 1 {
                                app.viewport_multi_drag = Some((starts, pointer_pos));
                                app.viewport_drag_state = None;
                            } else {
                                let pos_now = comp_state.layers[idx].transform.position.evaluate(current_frame);
                                app.viewport_drag_state = Some((idx, pos_now, pointer_pos));
                            }
                        } else {
                            let pos_now = comp_state.layers[idx].transform.position.evaluate(current_frame);
                            app.viewport_drag_state = Some((idx, pos_now, pointer_pos));
                        }
                    } else {
                        app.viewport_drag_state = None;
                    }
                }
            }
        }

        if viewport_response.dragged() {
            if let Some((l_idx, kf_frame, start_val, start_ptr)) = app.viewport_pos_kf_drag_state {
                let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;

                let comp_mut = app.history.current_mut().active_composition_mut();
                if let Some(layer) = comp_mut.layers.get_mut(l_idx) {
                    if let Some(kfs) = layer.transform.position.keyframes_mut() {
                        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == kf_frame) {
                            kf.value = [
                                start_val[0] + delta_x,
                                start_val[1] + delta_y,
                            ];
                        }
                    }
                }
            } else if let Some((l_idx, kf_frame, handle_type, start_bez, start_ptr)) = app.viewport_tangent_drag_state {
                let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;

                let comp_mut = app.history.current_mut().active_composition_mut();
                if let Some(layer) = comp_mut.layers.get_mut(l_idx) {
                    if let Some(kfs) = layer.transform.position.keyframes_mut() {
                        // Find the keyframe and its neighbor
                        let mut update: Option<(usize, [f32; 4])> = None;
                        for i in 0..kfs.len() {
                            if kfs[i].frame == kf_frame {
                                if let crate::core::keyframe::InterpolationType::Bezier { custom_bezier: Some(ref pts), .. } = kfs[i].interpolation {
                                    let mut new_pts = *pts;
                                    if handle_type == 0 && i + 1 < kfs.len() {
                                        // Dragging outgoing handle of keyframe i
                                        let val_delta = [kfs[i+1].value[0] - kfs[i].value[0], kfs[i+1].value[1] - kfs[i].value[1]];
                                        let normalized = |base: f32, delta: f32, span: f32| {
                                            if span.abs() > f32::EPSILON { (base * span + delta) / span } else { base }
                                        };
                                        let new_out_x = normalized(start_bez[0], delta_x, val_delta[0]).clamp(0.01, 0.99);
                                        let new_out_y = normalized(start_bez[1], delta_y, val_delta[1]).clamp(-1.5, 2.5);
                                        if app.viewport_linked_tangent {
                                            new_pts[2] = 1.0 - new_out_x;
                                            new_pts[3] = -new_out_y;
                                        }
                                        new_pts[0] = new_out_x;
                                        new_pts[1] = new_out_y;
                                        update = Some((i, new_pts));
                                    } else if handle_type == 1 && i > 0 {
                                        // Dragging incoming handle of keyframe i (which is the outgoing of i-1... actually incoming of this kf)
                                        let val_delta = [kfs[i].value[0] - kfs[i-1].value[0], kfs[i].value[1] - kfs[i-1].value[1]];
                                        let normalized = |base: f32, delta: f32, span: f32| {
                                            if span.abs() > f32::EPSILON { (base * span + delta) / span } else { base }
                                        };
                                        let new_in_x = normalized(start_bez[2], -delta_x, val_delta[0]).clamp(0.01, 0.99);
                                        let new_in_y = normalized(start_bez[3], -delta_y, val_delta[1]).clamp(-1.5, 2.5);
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
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
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
                        // Move all selected vertices if multi-selection is active
                        move_mask_vertices(&mut mask.path, current_frame, &app.viewport_mask_drag_vertices, [delta_x, delta_y]);
                    }
                }
            } else if let Some((l_idx, m_idx, v_idx, is_out, start_handle, start_ptr)) = app.mask_tangent_drag_state {
                // Drag mask tangent handle
                let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
                let new_handle = [
                    start_handle[0] + delta_x,
                    start_handle[1] + delta_y,
                ];
                let comp_mut = app.history.current_mut().active_composition_mut();
                if l_idx < comp_mut.layers.len() {
                    let layer = &mut comp_mut.layers[l_idx];
                    if m_idx < layer.masks.len() {
                        let mask = &mut layer.masks[m_idx];
                        let verts = mask.path.vertices_at_frame(current_frame);
                        if v_idx < verts.len() {
                            move_mask_tangent(&mut mask.path, v_idx, is_out, new_handle, ui.input(|i| i.modifiers.shift));
                            crate::core::frame_cache::bump_version();
                        }
                    }
                }
            } else if let Some((drag_idx, start_scale, start_dist)) = app.viewport_scale_drag {
                // Corner-handle scale: distance from layer center drives uniform scale
                let pos = {
                    let comp_state = app.history.current().active_composition();
                    comp_state.layers.get(drag_idx)
                        .map(|l| l.transform.position.evaluate(current_frame))
                };
                if let Some(pos) = pos {
                    let cx = origin_x + (pos[0] / comp_w) * draw_w;
                    let cy = origin_y + (pos[1] / comp_h) * draw_h;
                    let cur_dist = ((pointer_pos.x - cx).powi(2) + (pointer_pos.y - cy).powi(2)).sqrt().max(1.0);
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
                                    kfs.push(crate::core::keyframe::Keyframe::new(current_frame, new_scl, crate::core::keyframe::InterpolationType::Linear));
                                    kfs.sort_by_key(|k| k.frame);
                                }
                            }
                        }
                    }
                }
            } else if let Some((starts, start_ptr)) = app.viewport_multi_drag.clone() {
                // Group move: every selected layer follows the pointer delta
                let mut delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                let mut delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
                if ui.input(|i| i.modifiers.shift) {
                    if delta_x.abs() >= delta_y.abs() { delta_y = 0.0; } else { delta_x = 0.0; }
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
                // Shift = axis lock (AE parity): keep the dominant axis only.
                if ui.input(|i| i.modifiers.shift) {
                    if delta_x.abs() >= delta_y.abs() { delta_y = 0.0; } else { delta_x = 0.0; }
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
                                        kfs.push(crate::core::keyframe::Keyframe::new(current_frame, current_r + rot_delta, crate::core::keyframe::InterpolationType::Linear));
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
                                        kfs.push(crate::core::keyframe::Keyframe::new(current_frame, [cur_ap[0] + delta_x, cur_ap[1] + delta_y], crate::core::keyframe::InterpolationType::Linear));
                                        kfs.sort_by_key(|k| k.frame);
                                    }
                                }
                            }
                        }
                        _ => {
                            match &mut layer.transform.position {
                                Animatable::Constant(ref mut pos) => {
                                    if app.motion_sketch_active && app.playback.is_playing {
                                        // Motion Sketch: promote Constant → Animated with frame-0 + current
                                        let old_pos = *pos;
                                        let mut kfs = vec![
                                            crate::core::keyframe::Keyframe::new(0, old_pos, crate::core::keyframe::InterpolationType::Linear),
                                            crate::core::keyframe::Keyframe::new(current_frame, new_pos, crate::core::keyframe::InterpolationType::Linear),
                                        ];
                                        kfs.sort_by_key(|k| k.frame);
                                        layer.transform.position = Animatable::Animated(kfs);
                                    } else {
                                        *pos = new_pos;
                                    }
                                }
                                Animatable::Animated(ref mut keyframes) => {
                                    let existing_idx = keyframes.iter().position(|kf| kf.frame == current_frame);
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
                            // Record into motion sketch buffer for the commit-on-stop pass
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

        // ── Pen tool: click adds mask vertices; Enter/double-click commits ──
        // ── Click-drag creates Bezier handles (AE standard behavior) ──
        if app.active_tool == crate::ui::toolbar::ActiveTool::Pen {
            super::handle_pen_pointer(app, viewport_response, [origin_x, origin_y], [comp_w / draw_w, comp_h / draw_h]);
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if viewport_response.double_clicked() && !app.pen_points.is_empty() {
                // Double-click pushes a duplicate point — drop it before commit
                app.pen_points.pop();
                app.pen_tangents.pop();
                pen_commit = true;
            }
            if enter && app.pen_points.len() >= 3 {
                pen_commit = true;
            }
            if escape {
                app.pen_bezier_drag = None;
                app.pen_points.clear();
                app.pen_tangents.clear();
            }
        }

        // ── Creation tools: Q tool (Rectangle) and T tool (Text) commit on click or drag end ──
        if viewport_response.drag_stopped() || viewport_response.clicked() {
            if app.active_tool == crate::ui::toolbar::ActiveTool::Rectangle {
                // Compute shape dimensions from drag (fallback to 220×160 if simple click)
                let (cx, cy, sw, sh) = if let Some(start) = app.rect_drag_start.take() {
                    let mid_x = (pointer_pos.x + start.x) / 2.0;
                    let mid_y = (pointer_pos.y + start.y) / 2.0;
                    let cx = ((mid_x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
                    let cy = ((mid_y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
                    let dx = ((pointer_pos.x - start.x) / draw_w * comp_w).abs().max(4.0);
                    let dy = ((pointer_pos.y - start.y) / draw_h * comp_h).abs().max(4.0);
                    (cx, cy, dx, dy)
                } else {
                    let cx = ((pointer_pos.x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
                    let cy = ((pointer_pos.y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
                    (cx, cy, 220.0, 160.0)
                };
                let (n, dur) = {
                    let c = app.history.current().active_composition();
                    (c.layers.len(), c.duration_frames)
                };
                let mut shape_layer = crate::core::timeline::Layer::new(
                    format!("shape_{}", n),
                    format!("Rectangle {}", n + 1),
                    crate::core::timeline::LayerType::Shape {
                        shape_type: crate::core::timeline::ShapeType::Rectangle {
                            width: crate::core::property::Animatable::new_constant(sw),
                            height: crate::core::property::Animatable::new_constant(sh),
                            corner_radius: crate::core::property::Animatable::new_constant(0.0),
                        },
                        color: [0.25, 0.55, 1.0, 1.0],
                        stroke_color: [1.0, 1.0, 1.0, 1.0],
                        stroke_width: 0.0,
                        fill_type: Default::default(),
                        extrusion_depth: 0.0,
                        bevel_depth: 0.0,
                    },
                    dur,
                );
                shape_layer.transform.position = crate::core::property::Animatable::new_constant([cx, cy]);
                let comp_mut = app.history.current_mut().active_composition_mut();
                comp_mut.layers.push(shape_layer);
                let new_idx = comp_mut.layers.len() - 1;
                app.selection.selected_layer_idx = Some(new_idx);
                app.selection.selected_layers.clear();
                app.selection.selected_layers.insert(new_idx);
                crate::core::frame_cache::bump_version();
                app.toasts.info("Rectangle created (Q tool)");
            } else if app.active_tool == crate::ui::toolbar::ActiveTool::Text && viewport_response.clicked() {
                let cx = ((pointer_pos.x - origin_x) / draw_w * comp_w).clamp(0.0, comp_w);
                let cy = ((pointer_pos.y - origin_y) / draw_h * comp_h).clamp(0.0, comp_h);
                let (n, dur) = {
                    let c = app.history.current().active_composition();
                    (c.layers.len(), c.duration_frames)
                };
                let mut text_layer = crate::core::timeline::Layer::new(
                    format!("text_{}", n),
                    format!("Text {}", n + 1),
                    crate::core::timeline::LayerType::new_text(
                        "Text",
                        72,
                        [1.0, 1.0, 1.0, 1.0],
                    ),
                    dur,
                );
                text_layer.transform.position = crate::core::property::Animatable::new_constant([cx, cy]);
                let comp_mut = app.history.current_mut().active_composition_mut();
                comp_mut.layers.push(text_layer);
                let new_idx = comp_mut.layers.len() - 1;
                app.selection.selected_layer_idx = Some(new_idx);
                app.selection.selected_layers.clear();
                app.selection.selected_layers.insert(new_idx);
                app.inline_text_edit_layer = Some(new_idx);
                crate::core::frame_cache::bump_version();
            }
            let was_dragging = app.viewport_drag_state.is_some()
                || app.viewport_mask_drag_state.is_some()
                || app.mask_tangent_drag_state.is_some()
                || app.viewport_pos_kf_drag_state.is_some()
                || app.viewport_tangent_drag_state.is_some()
                || app.viewport_scale_drag.is_some()
                || app.viewport_multi_drag.is_some();
            if was_dragging {
                // Commit the single Undo entry for the entire drag gesture
                app.commit_drag();
            }
            // Motion Sketch: finalize recording buffer into keyframes
            if app.motion_sketch_active && !app.motion_sketch_recording.is_empty() {
                if let Some(sel_idx) = app.selection.selected_layer_idx {
                    let recording = std::mem::take(&mut app.motion_sketch_recording);
                    let recording_count = recording.len();
                    let target = app
                        .history
                        .current()
                        .active_composition()
                        .layers
                        .get(sel_idx)
                        .map(|layer| {
                            (
                                app.history.current().active_composition().id.clone(),
                                layer.id.clone(),
                            )
                        });
                    if let Some((composition_id, layer_id)) = target {
                        let project_session_id = app.project_session_id;
                        let history_generation = app.history.generation();
                        let (sender, receiver) = std::sync::mpsc::channel();
                        app.motion_sketch_bake_rx = Some(receiver);
                        app.toasts.info("Motion Sketchをベイクしています…");
                        std::thread::spawn(move || {
                            let keyframes = crate::core::motion_sketch::build_position_keyframes(
                                &recording,
                                2.0,
                            );
                            let _ = sender.send(crate::app_state::MotionSketchBakeResult {
                                project_session_id,
                                history_generation,
                                composition_id,
                                layer_id,
                                raw_sample_count: recording_count,
                                keyframes,
                            });
                        });
                    } else {
                        app.toasts.warning(
                            "Motion Sketchの対象レイヤーが見つかりませんでした。",
                        );
                    }
                }
            }
            app.viewport_drag_state = None;
            app.viewport_mask_drag_state = None;
            app.viewport_mask_drag_vertices.clear();
            app.mask_tangent_drag_state = None;
            app.viewport_pos_kf_drag_state = None;
            app.viewport_tangent_drag_state = None;
            app.viewport_scale_drag = None;
            app.viewport_multi_drag = None;
        }
    }

    pen_commit
}
