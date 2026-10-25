use super::*;

pub(super) fn pen_path(
    points: Vec<[f32; 2]>,
    tangents: Vec<([f32; 2], [f32; 2])>,
) -> crate::core::mask::MaskPath {
    let mut path = crate::core::mask::MaskPath::new_closed(points);
    if tangents
        .iter()
        .any(|(out, incoming)| *out != [0.0; 2] || *incoming != [0.0; 2])
    {
        let mut tangents: Vec<_> = tangents
            .into_iter()
            .map(|(out, incoming)| (incoming, out))
            .collect();
        tangents.resize(path.vertices_at_frame(0).len(), ([0.0; 2], [0.0; 2]));
        path.tangents = Some(tangents);
    }
    path
}

pub(super) fn handle_pen_pointer(
    app: &mut KagariApp,
    response: &egui::Response,
    origin: [f32; 2],
    scale: [f32; 2],
) {
    let Some(pointer) = response.interact_pointer_pos() else {
        return;
    };
    if response.drag_started() {
        let start = response
            .ctx
            .input(|i| i.pointer.press_origin())
            .unwrap_or(pointer);
        app.pen_bezier_drag = Some((start, pointer));
    }
    if let Some(drag) = app.pen_bezier_drag.as_mut() {
        drag.1 = pointer;
    }
    if response.drag_stopped() {
        if let Some((start, end)) = app.pen_bezier_drag.take() {
            let tangent = [(end.x - start.x) * scale[0], (end.y - start.y) * scale[1]];
            app.pen_points.push([
                (start.x - origin[0]) * scale[0],
                (start.y - origin[1]) * scale[1],
            ]);
            app.pen_tangents.push((tangent, [-tangent[0], -tangent[1]]));
        }
    } else if response.clicked() {
        app.pen_bezier_drag = None;
        app.pen_points.push([
            (pointer.x - origin[0]) * scale[0],
            (pointer.y - origin[1]) * scale[1],
        ]);
        app.pen_tangents.push(([0.0, 0.0], [0.0, 0.0]));
    }
}

pub(super) fn move_mask_vertices(
    path: &mut crate::core::mask::MaskPath,
    frame: u32,
    starts: &[(usize, [f32; 2])],
    delta: [f32; 2],
) {
    for &(index, start) in starts {
        path.set_vertex_at_frame(frame, index, [start[0] + delta[0], start[1] + delta[1]]);
    }
}

pub(super) fn move_mask_tangent(
    path: &mut crate::core::mask::MaskPath,
    index: usize,
    outgoing: bool,
    handle: [f32; 2],
    linked: bool,
) {
    let (old_in, old_out) = path
        .tangents
        .as_ref()
        .and_then(|t| t.get(index))
        .copied()
        .unwrap_or_default();
    let opposite = [-handle[0], -handle[1]];
    if outgoing {
        path.set_tangents_at_vertex(
            index,
            if linked { opposite } else { old_in },
            handle,
            linked,
        );
    } else {
        path.set_tangents_at_vertex(
            index,
            handle,
            if linked { opposite } else { old_out },
            linked,
        );
    }
}

pub(super) fn draw_selection_overlays(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    viewport_response: &egui::Response,
    current_frame: u32,
    geometry: ViewportGeometry,
) {
    let ViewportGeometry {
        origin_x,
        origin_y,
        draw_w,
        draw_h,
        comp_w,
        comp_h,
    } = geometry;
    if app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
        let alt_held = ui.input(|i| i.modifiers.alt);
        if alt_held && viewport_response.clicked() {
            if let Some(sel_li) = app.selection.selected_layer_idx {
                let comp_state_check = app.history.current().active_composition();
                if sel_li < comp_state_check.layers.len() {
                    let l = &comp_state_check.layers[sel_li];
                    let ptr_pos_check =
                        viewport_response.interact_pointer_pos().unwrap_or_default();
                    let comp_px_check = (ptr_pos_check.x - origin_x) / draw_w * comp_w;
                    let comp_py_check = (ptr_pos_check.y - origin_y) / draw_h * comp_h;
                    for (mi, mask) in l.masks.iter().enumerate() {
                        if mask.enabled {
                            let verts = mask.path.vertices_at_frame(current_frame);
                            let n = verts.len();
                            if n >= 3 {
                                // Find closest edge segment
                                let mut best_dist = f32::MAX;
                                let mut best_seg = 0;
                                let mut best_t = 0.5;
                                let end = if mask.path.is_closed { n } else { n - 1 };
                                for seg in 0..end {
                                    let p0 = verts[seg];
                                    let p1 = verts[(seg + 1) % n];
                                    // Project point onto segment
                                    let dx = p1[0] - p0[0];
                                    let dy = p1[1] - p0[1];
                                    let len_sq = dx * dx + dy * dy;
                                    if len_sq < 0.001 {
                                        continue;
                                    }
                                    let t = ((comp_px_check - p0[0]) * dx
                                        + (comp_py_check - p0[1]) * dy)
                                        / len_sq;
                                    let t_clamped = t.clamp(0.0, 1.0);
                                    let proj_x = p0[0] + t_clamped * dx;
                                    let proj_y = p0[1] + t_clamped * dy;
                                    let dist = ((comp_px_check - proj_x).powi(2)
                                        + (comp_py_check - proj_y).powi(2))
                                    .sqrt();
                                    if dist < best_dist && dist < 20.0 {
                                        best_dist = dist;
                                        best_seg = seg;
                                        best_t = t_clamped;
                                    }
                                }
                                if best_dist < 20.0 {
                                    let mut next_project = app.history.current().clone();
                                    let comp_mut = next_project.active_composition_mut();
                                    if sel_li < comp_mut.layers.len() {
                                        let layer = &mut comp_mut.layers[sel_li];
                                        if mi < layer.masks.len() {
                                            let mask = &mut layer.masks[mi];
                                            if let Some(new_idx) = mask.path.insert_vertex_at_frame(
                                                current_frame,
                                                best_seg,
                                                best_t,
                                            ) {
                                                app.commit_project(next_project);
                                                app.mask_selected_vertices = Some((
                                                    sel_li,
                                                    mi,
                                                    vec![new_idx].into_iter().collect(),
                                                ));
                                                app.toasts.info(format!(
                                                    "Inserted vertex at index {}",
                                                    new_idx
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    // ── Selected-layer transform box + corner scale handles (Selection tool) ──
    if app.show_handles && app.active_tool == crate::ui::toolbar::ActiveTool::Selection {
        if let Some(sel) = app.selection.selected_layer_idx {
            let comp_state = app.history.current().active_composition();
            if sel < comp_state.layers.len() {
                let l = &comp_state.layers[sel];
                if l.is_active(current_frame) {
                    let pos = l.transform.position.evaluate(current_frame);
                    let scl = l.transform.scale.evaluate(current_frame);
                    let cx = origin_x + (pos[0] / comp_w) * draw_w;
                    let cy = origin_y + (pos[1] / comp_h) * draw_h;
                    let sxw = (scl[0].abs() * 0.6) / comp_w * draw_w;
                    let syh = (scl[1].abs() * 0.6) / comp_h * draw_h;
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(cx - sxw, cy - syh),
                        egui::vec2(sxw * 2.0, syh * 2.0),
                    );
                    let painter = ui.painter();
                    let box_stroke =
                        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::HANDLE_NORMAL);
                    // Edges
                    painter.rect_stroke(box_rect, 0.0, box_stroke);
                    // Corner squares (scale handles)
                    let handle_fill = crate::ui::theme::colors::ACCENT_BLUE;
                    for hx in [box_rect.left(), box_rect.right()] {
                        for hy in [box_rect.top(), box_rect.bottom()] {
                            let hr = egui::Rect::from_center_size(
                                egui::pos2(hx, hy),
                                egui::vec2(7.0, 7.0),
                            );
                            painter.rect_filled(hr, 1.5, handle_fill);
                            painter.rect_stroke(
                                hr,
                                1.5,
                                egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                            );
                        }
                    }
                }
            }
        }
    }
}
