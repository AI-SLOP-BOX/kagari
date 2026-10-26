use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_brush_eraser(
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
    // ── Brush / Eraser tool: paint strokes onto the selected layer ──
    if matches!(
        app.active_tool,
        crate::ui::toolbar::ActiveTool::Brush | crate::ui::toolbar::ActiveTool::Eraser
    ) {
        // Floating mini-HUD: size (+ color for Brush)
        if app.active_tool == crate::ui::toolbar::ActiveTool::Brush
            || app.active_tool == crate::ui::toolbar::ActiveTool::Eraser
        {
            let hud_id = egui::Id::new("paint_hud");
            egui::Area::new(hud_id)
                .anchor(egui::Align2::RIGHT_TOP, [-14.0, 60.0])
                .show(ctx, |ui| {
                    let hud_frame = egui::Frame::default()
                        .fill(crate::ui::theme::colors::OVERLAY_SURFACE)
                        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                        .inner_margin(8.0);
                    hud_frame.show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(150.0);
                        let is_erase = app.active_tool == crate::ui::toolbar::ActiveTool::Eraser;
                        ui.label(
                            egui::RichText::new(if is_erase { "Eraser Size" } else { "Brush" })
                                .strong()
                                .small(),
                        );
                        let mut size = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_size"))
                                .unwrap_or(12.0)
                        });
                        if ui
                            .add(egui::Slider::new(&mut size, 1.0..=80.0).suffix(" px"))
                            .changed()
                        {
                            ctx.data_mut(|d| d.insert_temp(egui::Id::new("paint_size"), size));
                        }
                        if app.active_tool != crate::ui::toolbar::ActiveTool::Eraser {
                            let mut col = ctx.data_mut(|d| {
                                d.get_temp::<[f32; 4]>(egui::Id::new("paint_color"))
                                    .unwrap_or([1.0, 1.0, 1.0, 1.0])
                            });
                            ui.color_edit_button_rgba_unmultiplied(&mut col);
                            ctx.data_mut(|d| d.insert_temp(egui::Id::new("paint_color"), col));
                        }
                    });
                });
        }

        if let Some(sel_li) = app.selection.selected_layer_idx {
            let pointer = viewport_response.interact_pointer_pos();
            let comp_pt: Option<[f32; 2]> = pointer.map(|pp| {
                [
                    (pp.x - origin_x) / draw_w * comp_w,
                    (pp.y - origin_y) / draw_h * comp_h,
                ]
            });

            // Eraser: click removes the topmost stroke hit near a point.
            if app.active_tool == crate::ui::toolbar::ActiveTool::Eraser {
                if viewport_response.clicked() {
                    if let Some(pt) = comp_pt {
                        let radius = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_size"))
                                .unwrap_or(12.0)
                        }) * 0.5;
                        app.begin_drag("Erase Paint Stroke");
                        let mut erased = false;
                        {
                            let proj = app.history.current_mut().active_composition_mut();
                            if let Some(layer) = proj.layers.get_mut(sel_li) {
                                if let Some(pos) = layer.paint_strokes.iter().rposition(|s| {
                                    s.points.iter().any(|p| {
                                        let dx = p[0] - pt[0];
                                        let dy = p[1] - pt[1];
                                        dx * dx + dy * dy <= radius * radius
                                    })
                                }) {
                                    layer.paint_strokes.remove(pos);
                                    erased = true;
                                }
                            }
                        }
                        if erased {
                            app.commit_drag();
                            app.toasts.info("Stroke erased");
                        } else {
                            app.cancel_drag();
                        }
                    }
                }
            } else {
                // Brush: build a live stroke while dragging.
                let stroke_id = egui::Id::new(("paint_live", sel_li));
                if viewport_response.drag_started() {
                    if let Some(pt) = comp_pt {
                        // Convert comp -> layer-local (inverse transform).
                        let local = {
                            let comp_ro = app.history.current().active_composition();
                            match comp_ro.layers.get(sel_li) {
                                Some(l) => {
                                    let (pos, scl, rot, _) =
                                        comp_ro.resolve_world_transform(l, current_frame);
                                    let rad = rot.to_radians();
                                    let (cr, sr) = rad.sin_cos();
                                    let dx = (pt[0] - pos[0]) * 100.0;
                                    let dy = (pt[1] - pos[1]) * 100.0;
                                    let lx = (dx * cr + dy * sr) / scl[0].abs().max(0.001);
                                    let ly = (-dx * sr + dy * cr) / scl[1].abs().max(0.001);
                                    [lx, ly]
                                }
                                None => pt,
                            }
                        };
                        ctx.data_mut(|d| d.insert_temp(stroke_id, vec![local]));
                        app.begin_drag("Paint Stroke");
                    }
                }
                if viewport_response.dragged() && comp_pt.is_some() {
                    let has = ctx.data_mut(|d| d.get_temp::<Vec<[f32; 2]>>(stroke_id).is_some());
                    if has {
                        if let Some(pt) = comp_pt {
                            let local = {
                                let comp_ro = app.history.current().active_composition();
                                match comp_ro.layers.get(sel_li) {
                                    Some(l) => {
                                        let (pos, scl, rot, _) =
                                            comp_ro.resolve_world_transform(l, current_frame);
                                        let rad = rot.to_radians();
                                        let (cr, sr) = rad.sin_cos();
                                        let dx = (pt[0] - pos[0]) * 100.0;
                                        let dy = (pt[1] - pos[1]) * 100.0;
                                        let lx = (dx * cr + dy * sr) / scl[0].abs().max(0.001);
                                        let ly = (-dx * sr + dy * cr) / scl[1].abs().max(0.001);
                                        [lx, ly]
                                    }
                                    None => pt,
                                }
                            };
                            ctx.data_mut(|d| {
                                let mut pts =
                                    d.get_temp::<Vec<[f32; 2]>>(stroke_id).unwrap_or_default();
                                if pts
                                    .last()
                                    .map(|last| {
                                        let dx = last[0] - local[0];
                                        let dy = last[1] - local[1];
                                        dx * dx + dy * dy >= 4.0
                                    })
                                    .unwrap_or(true)
                                {
                                    pts.push(local);
                                    d.insert_temp(stroke_id, pts);
                                }
                            });
                        }
                    }
                }
                if viewport_response.drag_stopped() {
                    let pts_opt = ctx.data_mut(|d| d.remove_temp::<Vec<[f32; 2]>>(stroke_id));
                    if let Some(pts) = pts_opt {
                        if !pts.is_empty() {
                            let color = ctx.data_mut(|d| {
                                d.get_temp::<[f32; 4]>(egui::Id::new("paint_color"))
                                    .unwrap_or([1.0, 1.0, 1.0, 1.0])
                            });
                            let bsize = ctx.data_mut(|d| {
                                d.get_temp::<f32>(egui::Id::new("paint_size"))
                                    .unwrap_or(12.0)
                            });
                            let hardness = ctx.data_mut(|d| {
                                d.get_temp::<f32>(egui::Id::new("paint_hardness"))
                                    .unwrap_or(80.0)
                            }) / 100.0;
                            let opacity = ctx.data_mut(|d| {
                                d.get_temp::<f32>(egui::Id::new("paint_opacity"))
                                    .unwrap_or(100.0)
                            }) / 100.0;
                            let flow = ctx.data_mut(|d| {
                                d.get_temp::<f32>(egui::Id::new("paint_flow"))
                                    .unwrap_or(100.0)
                            }) / 100.0;
                            let start_f = current_frame;
                            let proj = app.history.current_mut().active_composition_mut();
                            if let Some(layer) = proj.layers.get_mut(sel_li) {
                                layer
                                    .paint_strokes
                                    .push(crate::core::timeline::PaintStroke {
                                        color,
                                        size: bsize,
                                        mode: crate::core::timeline::PaintStrokeMode::Brush,
                                        clone_offset: [0.0, 0.0],
                                        hardness,
                                        opacity,
                                        flow,
                                        points: pts,
                                        start_frame: start_f,
                                        end_frame: 0,
                                    });
                                crate::core::frame_cache::bump_version();
                                app.commit_drag();
                                app.toasts.info("Stroke painted");
                            }
                        } else {
                            app.cancel_drag();
                        }
                    }
                }

                // Live preview of the in-progress stroke on screen.
                let preview_pts = ctx.data(|d| d.get_temp::<Vec<[f32; 2]>>(stroke_id));
                if let Some(pts) = preview_pts {
                    if pts.len() >= 2 {
                        let comp_ro = app.history.current().active_composition();
                        if let Some(l) = comp_ro.layers.get(sel_li) {
                            let (pos, scl, rot, _) =
                                comp_ro.resolve_world_transform(l, current_frame);
                            let rad = rot.to_radians();
                            let (cr, sr) = rad.sin_cos();
                            let to_screen = |lp: [f32; 2]| {
                                let wx = pos[0] + (lp[0] * cr - lp[1] * sr) * scl[0] / 100.0;
                                let wy = pos[1] + (lp[0] * sr + lp[1] * cr) * scl[1] / 100.0;
                                egui::pos2(
                                    origin_x + wx / comp_w * draw_w,
                                    origin_y + wy / comp_h * draw_h,
                                )
                            };
                            for seg in pts.windows(2) {
                                ui.painter().line_segment(
                                    [to_screen(seg[0]), to_screen(seg[1])],
                                    egui::Stroke::new(
                                        ctx.data_mut(|d| {
                                            d.get_temp::<f32>(egui::Id::new("paint_size"))
                                                .unwrap_or(12.0)
                                        }) * 0.5
                                            * (scl[0].abs() / 100.0),
                                        colors::ACCENT_YELLOW.linear_multiply(0.8),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_clone_stamp(
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: u32,
    viewport_response: &egui::Response,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
) {
    // ── Clone Stamp tool: paint strokes onto the selected layer from a source offset ──
    if app.active_tool == crate::ui::toolbar::ActiveTool::CloneStamp {
        // HUD for clone stamp settings
        {
            let hud_id = egui::Id::new("clone_stamp_hud");
            egui::Area::new(hud_id)
                .anchor(egui::Align2::RIGHT_TOP, [-14.0, 60.0])
                .show(ctx, |ui| {
                    let hud_frame = egui::Frame::default()
                        .fill(crate::ui::theme::colors::OVERLAY_SURFACE)
                        .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                        .inner_margin(8.0);
                    hud_frame.show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(150.0);
                        ui.label(egui::RichText::new("Clone Stamp").strong().small());
                        let mut size = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_size"))
                                .unwrap_or(12.0)
                        });
                        if ui
                            .add(egui::Slider::new(&mut size, 1.0..=100.0).suffix(" px"))
                            .changed()
                        {
                            ctx.data_mut(|d| d.insert_temp(egui::Id::new("paint_size"), size));
                        }
                        ui.label(
                            egui::RichText::new("Alt+Click to set source")
                                .small()
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                });
        }
        if let Some(sel_li) = app.selection.selected_layer_idx {
            let pointer = viewport_response.interact_pointer_pos();
            let comp_pt: Option<[f32; 2]> = pointer.map(|pp| {
                [
                    (pp.x - origin_x) / draw_w * comp_w,
                    (pp.y - origin_y) / draw_h * comp_h,
                ]
            });
            // Alt+Click to set source offset
            if viewport_response.clicked() && ctx.input(|i| i.modifiers.alt) {
                if let Some(pt) = comp_pt {
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("clone_source_offset"), pt));
                    app.toasts.info("Clone source set");
                }
            } else if viewport_response.drag_started() {
                if let Some(pt) = comp_pt {
                    let local = {
                        let comp_ro = app.history.current().active_composition();
                        match comp_ro.layers.get(sel_li) {
                            Some(l) => {
                                let (pos, scl, rot, _) =
                                    comp_ro.resolve_world_transform(l, current_frame);
                                let rad = rot.to_radians();
                                let (cr, sr) = rad.sin_cos();
                                let dx = (pt[0] - pos[0]) * 100.0;
                                let dy = (pt[1] - pos[1]) * 100.0;
                                let lx = (dx * cr + dy * sr) / scl[0].abs().max(0.001);
                                let ly = (-dx * sr + dy * cr) / scl[1].abs().max(0.001);
                                [lx, ly]
                            }
                            None => pt,
                        }
                    };
                    let source_offset = ctx.data_mut(|d| {
                        d.get_temp::<[f32; 2]>(egui::Id::new("clone_source_offset"))
                            .unwrap_or([0.0, 0.0])
                    });
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new(("clone_live", sel_li)), vec![local])
                    });
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            egui::Id::new(("clone_src_offset_live", sel_li)),
                            source_offset,
                        )
                    });
                    app.begin_drag("Clone Stamp Stroke");
                }
            } else if viewport_response.dragged() {
                if let Some(pt) = comp_pt {
                    let has = ctx.data_mut(|d| {
                        d.get_temp::<Vec<[f32; 2]>>(egui::Id::new(("clone_live", sel_li)))
                            .is_some()
                    });
                    if has {
                        let local = {
                            let comp_ro = app.history.current().active_composition();
                            match comp_ro.layers.get(sel_li) {
                                Some(l) => {
                                    let (pos, scl, rot, _) =
                                        comp_ro.resolve_world_transform(l, current_frame);
                                    let rad = rot.to_radians();
                                    let (cr, sr) = rad.sin_cos();
                                    let dx = (pt[0] - pos[0]) * 100.0;
                                    let dy = (pt[1] - pos[1]) * 100.0;
                                    let lx = (dx * cr + dy * sr) / scl[0].abs().max(0.001);
                                    let ly = (-dx * sr + dy * cr) / scl[1].abs().max(0.001);
                                    [lx, ly]
                                }
                                None => pt,
                            }
                        };
                        ctx.data_mut(|d| {
                            let mut pts = d
                                .get_temp::<Vec<[f32; 2]>>(egui::Id::new(("clone_live", sel_li)))
                                .unwrap_or_default();
                            if pts
                                .last()
                                .map(|last| {
                                    let dx = last[0] - local[0];
                                    let dy = last[1] - local[1];
                                    dx * dx + dy * dy >= 4.0
                                })
                                .unwrap_or(true)
                            {
                                pts.push(local);
                                d.insert_temp(egui::Id::new(("clone_live", sel_li)), pts);
                            }
                        });
                    }
                }
            } else if viewport_response.drag_stopped() {
                let pts_opt = ctx.data_mut(|d| {
                    d.remove_temp::<Vec<[f32; 2]>>(egui::Id::new(("clone_live", sel_li)))
                });
                let src_opt = ctx.data_mut(|d| {
                    d.remove_temp::<[f32; 2]>(egui::Id::new(("clone_src_offset_live", sel_li)))
                });
                if let Some(pts) = pts_opt {
                    if !pts.is_empty() {
                        let bsize = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_size"))
                                .unwrap_or(12.0)
                        });
                        let hardness = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_hardness"))
                                .unwrap_or(80.0)
                        }) / 100.0;
                        let opacity = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_opacity"))
                                .unwrap_or(100.0)
                        }) / 100.0;
                        let flow = ctx.data_mut(|d| {
                            d.get_temp::<f32>(egui::Id::new("paint_flow"))
                                .unwrap_or(100.0)
                        }) / 100.0;
                        let clone_offset = if let Some(source_comp) = src_opt {
                            let source_local = {
                                let comp_ro = app.history.current().active_composition();
                                match comp_ro.layers.get(sel_li) {
                                    Some(l) => {
                                        let (pos, scl, rot, _) =
                                            comp_ro.resolve_world_transform(l, current_frame);
                                        let rad = rot.to_radians();
                                        let (cr, sr) = rad.sin_cos();
                                        let dx = (source_comp[0] - pos[0]) * 100.0;
                                        let dy = (source_comp[1] - pos[1]) * 100.0;
                                        [
                                            (dx * cr + dy * sr) / scl[0].abs().max(0.001),
                                            (-dx * sr + dy * cr) / scl[1].abs().max(0.001),
                                        ]
                                    }
                                    None => source_comp,
                                }
                            };
                            let target = pts.first().copied().unwrap_or(source_local);
                            [source_local[0] - target[0], source_local[1] - target[1]]
                        } else {
                            [0.0, 0.0]
                        };
                        let proj = app.history.current_mut().active_composition_mut();
                        if let Some(layer) = proj.layers.get_mut(sel_li) {
                            layer
                                .paint_strokes
                                .push(crate::core::timeline::PaintStroke {
                                    color: [1.0, 1.0, 1.0, 1.0],
                                    size: bsize,
                                    mode: crate::core::timeline::PaintStrokeMode::CloneStamp,
                                    clone_offset,
                                    hardness,
                                    opacity,
                                    flow,
                                    points: pts,
                                    start_frame: current_frame,
                                    end_frame: 0,
                                });
                            crate::core::frame_cache::bump_version();
                            app.commit_drag();
                            app.toasts.info("Clone stamp stroke applied");
                        }
                    } else {
                        app.cancel_drag();
                    }
                }
            }
        }
    }
}
