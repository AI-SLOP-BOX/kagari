use super::*;

#[derive(Clone, Default)]
struct RotoLiveStroke {
    points: Vec<[f32; 2]>,
    radius: f32,
    is_foreground: bool,
}

fn roto_canvas_point(
    pointer: egui::Pos2,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
) -> Option<[f32; 2]> {
    if ![origin_x, origin_y, draw_w, draw_h, comp_w, comp_h]
        .into_iter()
        .all(f32::is_finite)
        || draw_w <= 0.0
        || draw_h <= 0.0
        || comp_w <= 0.0
        || comp_h <= 0.0
    {
        return None;
    }
    let u = (pointer.x - origin_x) / draw_w;
    let v = (pointer.y - origin_y) / draw_h;
    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
        return None;
    }
    Some([u * comp_w, v * comp_h])
}

fn matte_preview_polygon(path: &crate::core::mask::MaskPath, frame: u32) -> Option<Vec<[f32; 2]>> {
    const MAX_PREVIEW_POINTS: usize = 4096;
    let source_points = match &path.vertices {
        crate::core::property::Animatable::Constant(points) => points.len(),
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .map(|keyframe| keyframe.value.len())
            .max()
            .unwrap_or(0),
    };
    if !(3..=MAX_PREVIEW_POINTS).contains(&source_points) {
        return None;
    }

    // `to_polygon` can produce `source_points * segments` points. Select the
    // highest curve quality that stays within the preview triangulator's cap.
    let segments = (MAX_PREVIEW_POINTS / source_points)
        .min(crate::core::mask::MASK_CURVE_SEGMENTS as usize)
        .max(1) as u32;
    let polygon = path.to_polygon(frame, segments);
    (polygon.len() <= MAX_PREVIEW_POINTS + 1).then_some(polygon)
}

fn triangulate_matte_preview(points: &[egui::Pos2]) -> Option<egui::Mesh> {
    const MAX_TRIANGULATED_POINTS: usize = 4096;
    const EPSILON: f32 = 1.0e-5;

    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let mut unique_points = Vec::with_capacity(points.len());
    for &point in points {
        if unique_points.last().is_none_or(|last: &egui::Pos2| {
            (last.x - point.x).abs() > EPSILON || (last.y - point.y).abs() > EPSILON
        }) {
            unique_points.push(point);
        }
    }
    let mut points = unique_points;
    if points.len() > 1 {
        let first = points[0];
        let last = *points.last()?;
        if (first.x - last.x).abs() <= EPSILON && (first.y - last.y).abs() <= EPSILON {
            points.pop();
        }
    }
    if points.len() < 3 || points.len() > MAX_TRIANGULATED_POINTS {
        return None;
    }

    let cross = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    };
    let area: f32 = (0..points.len())
        .map(|index| {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            a.x * b.y - b.x * a.y
        })
        .sum();
    if !area.is_finite() || area.abs() <= EPSILON {
        return None;
    }

    let mut remaining: Vec<usize> = (0..points.len()).collect();
    if area < 0.0 {
        remaining.reverse();
    }
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    while remaining.len() > 3 {
        let mut ear = None;
        for current in 0..remaining.len() {
            let previous = remaining[(current + remaining.len() - 1) % remaining.len()];
            let vertex = remaining[current];
            let next = remaining[(current + 1) % remaining.len()];
            let (a, b, c) = (points[previous], points[vertex], points[next]);
            if cross(a, b, c) <= EPSILON {
                continue;
            }
            let contains_vertex = remaining.iter().copied().any(|candidate| {
                if candidate == previous || candidate == vertex || candidate == next {
                    return false;
                }
                let point = points[candidate];
                cross(a, b, point) >= -EPSILON
                    && cross(b, c, point) >= -EPSILON
                    && cross(c, a, point) >= -EPSILON
            });
            if !contains_vertex {
                ear = Some((current, [previous as u32, vertex as u32, next as u32]));
                break;
            }
        }
        let (current, triangle) = ear?;
        triangles.push(triangle);
        remaining.remove(current);
    }
    triangles.push([
        remaining[0] as u32,
        remaining[1] as u32,
        remaining[2] as u32,
    ]);

    let fill = egui::Color32::from_rgba_unmultiplied(80, 220, 120, 52);
    let mut mesh = egui::Mesh::default();
    for point in points {
        mesh.colored_vertex(point, fill);
    }
    for [a, b, c] in triangles {
        mesh.add_triangle(a, b, c);
    }
    Some(mesh)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::ui::viewport) fn draw_roto_brush_interaction(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    current_frame: u32,
    viewport_response: &egui::Response,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
) {
    // ── Roto Brush Tool (Interactive Green/Red Strokes -> Auto-Mask) ──
    if app.active_tool == crate::ui::toolbar::ActiveTool::RotoBrush {
        let mut clear_roto = false;
        let mut propagate_roto = false;
        let mut propagate_roto_flow = false;
        let segment_busy = ctx
            .data(|data| {
                data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new("roto_segment_job"))
            })
            .is_some();
        let flow_job = ctx.data(|data| {
            data.get_temp::<std::sync::Arc<RotoFlowJob>>(egui::Id::new("roto_flow_bake_job"))
        });
        let propagation_busy = segment_busy || flow_job.is_some();
        // Radius HUD
        {
            let hud_id = egui::Id::new("roto_brush_hud");
            egui::Area::new(hud_id)
                .anchor(egui::Align2::RIGHT_TOP, [-14.0, 60.0])
                .show(ctx, |ui| {
                    let hud_frame = egui::Frame::default()
                        .fill(crate::ui::theme::colors::OVERLAY_SURFACE)
                        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                        .inner_margin(8.0);
                    hud_frame.show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(170.0);
                        let alt_held = ctx.input(|i| i.modifiers.alt);
                        let mut fg_mode = ctx.data_mut(|d| {
                            d.get_temp::<bool>(egui::Id::new("roto_fg_mode"))
                                .unwrap_or(true)
                        });
                        if alt_held {
                            fg_mode = false;
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(fg_mode, "FG")
                                .on_hover_text("Foreground subject mark (Green)")
                                .clicked()
                            {
                                fg_mode = true;
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new("roto_fg_mode"), true)
                                });
                            }
                            if ui
                                .selectable_label(!fg_mode, "BG")
                                .on_hover_text("Background mark (Red / Alt+Drag)")
                                .clicked()
                            {
                                fg_mode = false;
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new("roto_fg_mode"), false)
                                });
                            }
                        });

                        let mut radius = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("roto_brush_radius"))
                                .unwrap_or(8.0)
                        });
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            if ui
                                .add(egui::Slider::new(&mut radius, 1.0..=50.0).suffix(" px"))
                                .changed()
                            {
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new("roto_brush_radius"), radius)
                                });
                            }
                        });

                        let mut matte_preview = ctx.data_mut(|d| {
                            d.get_temp::<bool>(egui::Id::new("roto_matte_preview"))
                                .unwrap_or(false)
                        });
                        if ui
                            .checkbox(&mut matte_preview, "Matte Preview")
                            .on_hover_text("Show high-contrast black/white silhouette matte")
                            .changed()
                        {
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new("roto_matte_preview"), matte_preview)
                            });
                        }
                        ui.separator();
                        let mut feather = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("roto_feather_radius"))
                                .unwrap_or(3.0)
                        });
                        if ui
                            .add(egui::Slider::new(&mut feather, 0.0..=32.0).text("Feather"))
                            .changed()
                        {
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new("roto_feather_radius"), feather)
                            });
                        }
                        let mut contrast = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("roto_contrast"))
                                .unwrap_or(1.0)
                        });
                        if ui
                            .add(egui::Slider::new(&mut contrast, 0.1..=3.0).text("Contrast"))
                            .changed()
                        {
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new("roto_contrast"), contrast)
                            });
                        }
                        if ui
                            .add_enabled(
                                !propagation_busy,
                                egui::Button::new("Clear strokes and matte"),
                            )
                            .clicked()
                        {
                            clear_roto = true;
                        }
                        if ui
                            .add_enabled(
                                !propagation_busy,
                                egui::Button::new("Propagate with layer tracker"),
                            )
                            .clicked()
                        {
                            propagate_roto = true;
                        }
                        if segment_busy {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Analyzing matte…");
                            });
                        }
                        if let Some(job) = flow_job {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Propagating matte…");
                                if ui.button("Cancel").clicked() {
                                    job.cancelled
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                }
                            });
                        } else if ui
                            .add_enabled(
                                !segment_busy,
                                egui::Button::new("Propagate with optical flow"),
                            )
                            .clicked()
                        {
                            propagate_roto_flow = true;
                        }
                    });
                });
        }
        if let Some(sel_li) = app.selection.selected_layer_idx {
            let stroke_layer_id = app
                .history
                .current()
                .active_composition()
                .layers
                .get(sel_li)
                .map(|layer| layer.id.as_str())
                .unwrap_or("missing-layer");
            let stroke_list_id = egui::Id::new(("roto_strokes", stroke_layer_id));
            if ctx
                .data(|d| d.get_temp::<Vec<EngineRotoStroke>>(stroke_list_id))
                .is_none()
            {
                let persisted_strokes = app
                    .history
                    .current()
                    .active_composition()
                    .layers
                    .get(sel_li)
                    .map(|layer| layer.roto_brush_strokes.clone())
                    .unwrap_or_default();
                ctx.data_mut(|d| d.insert_temp(stroke_list_id, persisted_strokes));
            }
            if clear_roto {
                ctx.data_mut(|d| d.remove::<Vec<EngineRotoStroke>>(stroke_list_id));
                let mut cleared = app.history.current().clone();
                if let Some(layer) = cleared.active_composition_mut().layers.get_mut(sel_li) {
                    layer
                        .masks
                        .retain(|mask| mask.id != crate::core::roto_assist::ROTO_BRUSH_MATTE_ID);
                    layer.roto_brush_strokes.clear();
                }
                app.commit_project(cleared);
                app.toasts.info("Roto Brush matte cleared");
            }
            if propagate_roto {
                let mut propagated = app.history.current().clone();
                let active_tracker_idx = ctx.data(|d| {
                    d.get_temp::<usize>(egui::Id::new("ae_active_tracker_pt_idx"))
                        .unwrap_or(0)
                });
                let result = propagated
                    .active_composition_mut()
                    .layers
                    .get_mut(sel_li)
                    .ok_or_else(|| format!("Selected layer {} no longer exists", sel_li + 1))
                    .and_then(|layer| {
                        let mask_idx = layer
                            .masks
                            .iter()
                            .position(|mask| {
                                mask.id == crate::core::roto_assist::ROTO_BRUSH_MATTE_ID
                            })
                            .ok_or_else(|| {
                                "Create a Roto Brush Matte before propagation".to_owned()
                            })?;
                        let baked = crate::core::roto_assist::bake_tracked_mask_from_trackers(
                            &layer.masks[mask_idx],
                            &layer.trackers,
                            active_tracker_idx,
                            layer.in_frame,
                            layer.out_frame.saturating_sub(1).max(layer.in_frame),
                        )?;
                        layer.masks[mask_idx].path.vertices = baked;
                        Ok(())
                    });
                match result {
                    Ok(()) => {
                        app.commit_project(propagated);
                        app.toasts
                            .info("Roto Brush matte propagated with the selected tracker");
                    }
                    Err(error) => app.toasts.error(error),
                }
            }
            if propagate_roto_flow {
                match start_roto_flow_bake(app, ctx, sel_li, current_frame) {
                    Ok(()) => app.toasts.info("Roto optical-flow propagation started"),
                    Err(error) => app.toasts.error(error),
                }
            }
            let stroke_id = egui::Id::new(("roto_live_stroke", sel_li));
            let alt_held = ctx.input(|i| i.modifiers.alt);
            let fg_pref = ctx.data_mut(|d| {
                d.get_temp::<bool>(egui::Id::new("roto_fg_mode"))
                    .unwrap_or(true)
            });
            let is_fg = if alt_held { false } else { fg_pref };
            if !segment_busy && viewport_response.drag_started() {
                if let Some(pp) = viewport_response.interact_pointer_pos() {
                    if let Some(point) =
                        roto_canvas_point(pp, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h)
                    {
                        let radius = ctx.data(|data| {
                            data.get_temp::<f32>(egui::Id::new("roto_brush_radius"))
                                .unwrap_or(8.0)
                        });
                        ctx.data_mut(|d| {
                            d.insert_temp(
                                stroke_id,
                                RotoLiveStroke {
                                    points: vec![point],
                                    radius,
                                    is_foreground: is_fg,
                                },
                            )
                        });
                    }
                }
            }
            if !segment_busy && viewport_response.dragged() {
                if let Some(pp) = viewport_response.interact_pointer_pos() {
                    if let Some(point) =
                        roto_canvas_point(pp, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h)
                    {
                        let mut stroke = ctx.data_mut(|d| {
                            d.get_temp::<RotoLiveStroke>(stroke_id).unwrap_or_default()
                        });
                        stroke.points.push(point);
                        ctx.data_mut(|d| d.insert_temp(stroke_id, stroke));
                    }
                }
            }
            if !segment_busy && viewport_response.drag_stopped() {
                let stroke_opt = ctx.data_mut(|d| d.remove_temp::<RotoLiveStroke>(stroke_id));
                if let Some(stroke) = stroke_opt {
                    if stroke.points.len() >= 2 {
                        let previous_strokes = ctx.data_mut(|data| {
                            data.get_temp::<Vec<EngineRotoStroke>>(stroke_list_id)
                                .unwrap_or_default()
                        });
                        let mut all_strokes = previous_strokes.clone();
                        all_strokes.push(EngineRotoStroke {
                            stroke_type: if stroke.is_foreground {
                                RotoStrokeType::Foreground
                            } else {
                                RotoStrokeType::Background
                            },
                            points: stroke.points,
                            pressures: Vec::new(),
                            radius: stroke.radius,
                            frame: Some(current_frame),
                        });
                        ctx.data_mut(|data| data.insert_temp(stroke_list_id, all_strokes.clone()));
                        let feather = ctx.data_mut(|data| {
                            data.get_temp::<f32>(egui::Id::new("roto_feather_radius"))
                                .unwrap_or(3.0)
                        });
                        let contrast = ctx.data_mut(|data| {
                            data.get_temp::<f32>(egui::Id::new("roto_contrast"))
                                .unwrap_or(1.0)
                        });
                        let settings = RotoBrushSettings {
                            feather_radius: feather,
                            contrast,
                            ..Default::default()
                        };
                        match start_roto_segment_job(
                            app,
                            ctx,
                            RotoSegmentRequest {
                                layer_index: sel_li,
                                frame: current_frame,
                                previous_strokes: previous_strokes.clone(),
                                strokes: all_strokes,
                                settings,
                                feather,
                            },
                        ) {
                            Ok(()) => app.toasts.info("Roto Brush matte analysis started"),
                            Err(error) => {
                                ctx.data_mut(|data| {
                                    data.insert_temp(stroke_list_id, previous_strokes)
                                });
                                app.toasts.error(error);
                            }
                        }
                    }
                }
            }

            // Render live green (FG) or red (BG) stroke preview
            let preview = ctx.data(|d| d.get_temp::<RotoLiveStroke>(stroke_id));
            if let Some(stroke) = preview {
                if stroke.points.len() >= 2 {
                    let stroke_col = if stroke.is_foreground {
                        colors::ACCENT_GREEN
                    } else {
                        colors::ACCENT_RED
                    };
                    let screen_pts: Vec<egui::Pos2> = stroke
                        .points
                        .iter()
                        .map(|p| {
                            egui::pos2(
                                origin_x + p[0] / comp_w * draw_w,
                                origin_y + p[1] / comp_h * draw_h,
                            )
                        })
                        .collect();
                    let screen_scale = if comp_w > 0.0 && comp_h > 0.0 {
                        ((draw_w / comp_w) * (draw_h / comp_h)).sqrt()
                    } else {
                        1.0
                    };
                    let screen_scale = if screen_scale.is_finite() {
                        screen_scale
                    } else {
                        1.0
                    };
                    let radius = if stroke.radius.is_finite() {
                        stroke.radius.max(0.0)
                    } else {
                        0.0
                    };
                    let max_brush_width = (2.0 * draw_w.max(draw_h)).max(1.0);
                    let brush_width = (2.0 * radius * screen_scale).clamp(1.0, max_brush_width);
                    for w in screen_pts.windows(2) {
                        ui.painter()
                            .line_segment([w[0], w[1]], egui::Stroke::new(brush_width, stroke_col));
                    }
                }
            }

            // Matte Preview is intentionally drawn from the committed
            // mask, so it stays in sync with the same mask the renderer
            // uses instead of becoming a second, visual-only result.
            let matte_preview = ctx.data(|d| {
                d.get_temp::<bool>(egui::Id::new("roto_matte_preview"))
                    .unwrap_or(false)
            });
            if matte_preview {
                let comp_state = app.history.current().active_composition();
                if let Some(mask) = comp_state.layers.get(sel_li).and_then(|layer| {
                    layer
                        .masks
                        .iter()
                        .find(|mask| mask.id == crate::core::roto_assist::ROTO_BRUSH_MATTE_ID)
                }) {
                    if let Some(polygon) = matte_preview_polygon(&mask.path, current_frame) {
                        if polygon.len() >= 3 {
                            let screen_verts: Vec<egui::Pos2> = polygon
                                .iter()
                                .map(|p| {
                                    egui::pos2(
                                        origin_x + p[0] / comp_w * draw_w,
                                        origin_y + p[1] / comp_h * draw_h,
                                    )
                                })
                                .collect();
                            if let Some(mesh) = triangulate_matte_preview(&screen_verts) {
                                ui.painter().add(mesh);
                            }
                            ui.painter().add(egui::Shape::closed_line(
                                screen_verts,
                                egui::Stroke::new(
                                    1.0_f32,
                                    colors::ACCENT_GREEN.linear_multiply(0.8),
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}
