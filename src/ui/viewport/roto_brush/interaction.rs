use super::*;

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
                        if ui.button("Clear strokes and matte").clicked() {
                            clear_roto = true;
                        }
                        if ui.button("Propagate with layer tracker").clicked() {
                            propagate_roto = true;
                        }
                        let segment_job = ctx.data(|data| {
                            data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new(
                                "roto_segment_job",
                            ))
                        });
                        if segment_job.is_some() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Analyzing matte…");
                            });
                        }
                        let flow_job = ctx.data(|data| {
                            data.get_temp::<std::sync::Arc<RotoFlowJob>>(egui::Id::new(
                                "roto_flow_bake_job",
                            ))
                        });
                        if let Some(job) = flow_job {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Propagating matte…");
                                if ui.button("Cancel").clicked() {
                                    job.cancelled
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                }
                            });
                        } else if ui.button("Propagate with optical flow").clicked() {
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
                    layer.masks.retain(|mask| mask.name != "Roto Brush Matte");
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
                            .position(|mask| mask.name == "Roto Brush Matte")
                            .ok_or_else(|| {
                                "Create a Roto Brush Matte before propagation".to_owned()
                            })?;
                        let baked = crate::core::roto_assist::bake_tracked_mask_from_trackers(
                            &layer.masks[mask_idx],
                            &layer.trackers,
                            active_tracker_idx,
                            layer.in_frame,
                            layer.out_frame.max(layer.in_frame.saturating_add(1)),
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
            let segment_busy = ctx
                .data(|data| {
                    data.get_temp::<std::sync::Arc<RotoSegmentJob>>(egui::Id::new(
                        "roto_segment_job",
                    ))
                })
                .is_some();
            let alt_held = ctx.input(|i| i.modifiers.alt);
            let fg_pref = ctx.data_mut(|d| {
                d.get_temp::<bool>(egui::Id::new("roto_fg_mode"))
                    .unwrap_or(true)
            });
            let is_fg = if alt_held { false } else { fg_pref };
            if !segment_busy && viewport_response.drag_started() {
                if let Some(pp) = viewport_response.interact_pointer_pos() {
                    let cx = (pp.x - origin_x) / draw_w * comp_w;
                    let cy = (pp.y - origin_y) / draw_h * comp_h;
                    ctx.data_mut(|d| d.insert_temp(stroke_id, vec![[cx, cy]]));
                }
            }
            if !segment_busy && viewport_response.dragged() {
                if let Some(pp) = viewport_response.interact_pointer_pos() {
                    let cx = (pp.x - origin_x) / draw_w * comp_w;
                    let cy = (pp.y - origin_y) / draw_h * comp_h;
                    let mut pts = ctx
                        .data_mut(|d| d.get_temp::<Vec<[f32; 2]>>(stroke_id).unwrap_or_default());
                    pts.push([cx, cy]);
                    ctx.data_mut(|d| d.insert_temp(stroke_id, pts));
                }
            }
            if !segment_busy && viewport_response.drag_stopped() {
                let pts_opt = ctx.data_mut(|d| d.remove_temp::<Vec<[f32; 2]>>(stroke_id));
                if let Some(pts) = pts_opt {
                    if pts.len() >= 2 {
                        let radius = ctx.data_mut(|data| {
                            data.get_temp::<f32>(egui::Id::new("roto_brush_radius"))
                                .unwrap_or(8.0)
                        });
                        let previous_strokes = ctx.data_mut(|data| {
                            data.get_temp::<Vec<EngineRotoStroke>>(stroke_list_id)
                                .unwrap_or_default()
                        });
                        let mut all_strokes = previous_strokes.clone();
                        all_strokes.push(EngineRotoStroke {
                            stroke_type: if is_fg {
                                RotoStrokeType::Foreground
                            } else {
                                RotoStrokeType::Background
                            },
                            points: pts,
                            radius,
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
            let preview = ctx.data(|d| d.get_temp::<Vec<[f32; 2]>>(stroke_id));
            if let Some(pts) = preview {
                if pts.len() >= 2 {
                    let stroke_col = if is_fg {
                        colors::ACCENT_GREEN
                    } else {
                        colors::ACCENT_RED
                    };
                    let screen_pts: Vec<egui::Pos2> = pts
                        .iter()
                        .map(|p| {
                            egui::pos2(
                                origin_x + p[0] / comp_w * draw_w,
                                origin_y + p[1] / comp_h * draw_h,
                            )
                        })
                        .collect();
                    for w in screen_pts.windows(2) {
                        ui.painter()
                            .line_segment([w[0], w[1]], egui::Stroke::new(6.0_f32, stroke_col));
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
                        .find(|mask| mask.name == "Roto Brush Matte")
                }) {
                    let verts = mask.path.vertices_at_frame(current_frame);
                    if verts.len() >= 3 {
                        let screen_verts: Vec<egui::Pos2> = verts
                            .iter()
                            .map(|p| {
                                egui::pos2(
                                    origin_x + p[0] / comp_w * draw_w,
                                    origin_y + p[1] / comp_h * draw_h,
                                )
                            })
                            .collect();
                        ui.painter().add(egui::Shape::convex_polygon(
                            screen_verts,
                            egui::Color32::from_rgba_unmultiplied(80, 220, 120, 52),
                            egui::Stroke::new(1.0_f32, colors::ACCENT_GREEN.linear_multiply(0.8)),
                        ));
                    }
                }
            }
        }
    }
}
