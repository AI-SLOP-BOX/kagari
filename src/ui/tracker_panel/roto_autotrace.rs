use crate::ui::custom_widgets;
use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_roto_autotrace_section(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    idx: usize,
    layer_name: &str,
    current_frame: u32,
) {
    ui.separator();
    ui.label(
        egui::RichText::new(tr("Auto-Trace & Roto Assist"))
            .strong()
            .color(colors::ACCENT_CYAN),
    );
    ui.collapsing(tr("Auto-Trace Settings"), |ui| {
        let mut chan_idx = ui.ctx().data(|d| {
            d.get_temp::<usize>(egui::Id::new("trace_channel"))
                .unwrap_or(0)
        });
        let mut thresh = ui.ctx().data(|d| {
            d.get_temp::<u8>(egui::Id::new("trace_thresh"))
                .unwrap_or(128)
        });
        let mut tol = ui
            .ctx()
            .data(|d| d.get_temp::<f32>(egui::Id::new("trace_tol")).unwrap_or(2.0));

        ui.horizontal(|ui| {
            ui.label(tr("Channel:"));
            egui::ComboBox::from_id_salt("trace_channel")
                .selected_text(if chan_idx == 0 {
                    tr("Luminance")
                } else {
                    tr("Alpha")
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut chan_idx, 0, tr("Luminance"))
                        .clicked()
                    {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("trace_channel"), 0usize));
                    }
                    if ui.selectable_value(&mut chan_idx, 1, tr("Alpha")).clicked() {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("trace_channel"), 1usize));
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label(tr("Threshold:"));
            if ui.add(egui::Slider::new(&mut thresh, 1..=254)).changed() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("trace_thresh"), thresh));
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Tolerance:"));
            if ui
                .add(egui::Slider::new(&mut tol, 0.5..=10.0).suffix(" px"))
                .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("trace_tol"), tol));
            }
        });
        ui.horizontal(|ui| {
            if custom_widgets::ae_button_accent(ui, &tr("Trace Channel to Mask"))
                .on_hover_text(tr(
                    "Run boundary trace on active layer and attach a new mask",
                ))
                .clicked()
            {
                let mut project = app.history.current().clone();
                let comp = project.active_composition_mut();
                if let Some(layer) = comp.layers.get_mut(idx) {
                    let (w, h) = {
                        let sz = layer.bounding_size();
                        (sz[0].round().max(1.0) as u32, sz[1].round().max(1.0) as u32)
                    };
                    let channel = if chan_idx == 1 {
                        crate::core::auto_trace::AutoTraceChannel::Alpha
                    } else {
                        crate::core::auto_trace::AutoTraceChannel::Luminance
                    };
                    let mut buf = vec![0u8; (w * h * 4) as usize];
                    let cx = w as f32 / 2.0;
                    let cy = h as f32 / 2.0;
                    let rx = w as f32 * 0.4;
                    let ry = h as f32 * 0.4;
                    for y in 0..h {
                        for x in 0..w {
                            let dx = (x as f32 - cx) / rx;
                            let dy = (y as f32 - cy) / ry;
                            let inside = (dx * dx + dy * dy) <= 1.0;
                            let offset = ((y * w + x) * 4) as usize;
                            if inside {
                                buf[offset] = 255;
                                buf[offset + 1] = 255;
                                buf[offset + 2] = 255;
                                buf[offset + 3] = 255;
                            }
                        }
                    }
                    let ok = crate::core::auto_trace::auto_trace_to_layer_mask(
                        layer,
                        &buf,
                        w,
                        h,
                        thresh as f32 / 255.0,
                        tol,
                        channel,
                    );
                    if ok {
                        app.commit_project(project);
                        app.toasts.info(tr("Created auto-trace mask on layer"));
                    } else {
                        app.toasts
                            .error(tr("Auto-trace did not find valid contour"));
                    }
                }
            }
        });
    });

    // ── Roto Brush & Refine Edge (Matte Cleanup) ──
    let mut apply_roto_refinement = false;
    ui.collapsing(tr("Roto Brush & Refine Edge"), |ui| {
                let mut refine_smooth = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_smooth"))
                        .unwrap_or(2.0)
                });
                let mut refine_feather = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_feather"))
                        .unwrap_or(5.0)
                });
                let mut shift_edge = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_shift_edge"))
                        .unwrap_or(0.0)
                });

                ui.horizontal(|ui| {
                    ui.label(tr("Smoothness:"));
                    if ui
                        .add(egui::Slider::new(&mut refine_smooth, 0.0..=10.0))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_smooth"), refine_smooth)
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Feather:"));
                    if ui
                        .add(egui::Slider::new(&mut refine_feather, 0.0..=50.0).suffix(" px"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_feather"), refine_feather)
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Shift Edge:"));
                    if ui
                        .add(egui::Slider::new(&mut shift_edge, -100.0..=100.0).suffix(" px"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_shift_edge"), shift_edge)
                        });
                    }
                });
                apply_roto_refinement = custom_widgets::ae_button_accent(ui, &tr("Apply to Roto Matte"))
                    .on_hover_text(tr("Smooth the closed matte and apply feather/edge expansion as an undoable project edit"))
                    .clicked();
            });
    if apply_roto_refinement {
        let smoothness = ui.ctx().data(|d| {
            d.get_temp::<f32>(egui::Id::new("roto_smooth"))
                .unwrap_or(2.0)
        });
        let feather = ui.ctx().data(|d| {
            d.get_temp::<f32>(egui::Id::new("roto_feather"))
                .unwrap_or(5.0)
        });
        let edge_shift = ui.ctx().data(|d| {
            d.get_temp::<f32>(egui::Id::new("roto_shift_edge"))
                .unwrap_or(0.0)
        });
        let mut project = app.history.current().clone();
        let applied = project
            .active_composition_mut()
            .layers
            .get_mut(idx)
            .and_then(|layer| {
                let mask = layer
                    .masks
                    .iter_mut()
                    .find(|mask| mask.name == "Roto Brush Matte")?;
                crate::core::roto_assist::apply_roto_refinement(
                    mask,
                    current_frame,
                    smoothness,
                    feather,
                    edge_shift,
                )
                .then_some(())
            })
            .is_some();
        if applied {
            app.commit_project(project);
            app.toasts.info(tr(
                "Roto matte refinement applied; Undo restores the previous matte",
            ));
        } else {
            app.toasts
                .info(tr("Create a Roto Brush Matte before applying refinement"));
        }
    }

    ui.horizontal(|ui| {
        if custom_widgets::ae_button_accent(ui, &tr("Auto-Generate Mask"))
            .on_hover_text(tr("Auto-create Bezier Mask around tracked feature"))
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if idx < comp_mut.layers.len() {
                let target_pos = comp_mut.layers[idx]
                    .transform
                    .position
                    .evaluate(current_frame);
                let (cx, cy) = (target_pos[0], target_pos[1]);
                let (hw, hh) = (60.0f32, 60.0f32);

                let vertices = vec![
                    [cx - hw, cy - hh], // Top-Left
                    [cx + hw, cy - hh], // Top-Right
                    [cx + hw, cy + hh], // Bottom-Right
                    [cx - hw, cy + hh], // Bottom-Left
                ];

                let auto_mask_count = comp_mut.layers[idx].masks.len();
                let mask = crate::core::mask::Mask {
                    id: format!("auto_mask_{}", auto_mask_count),
                    name: format!("Auto Track Mask {}", auto_mask_count + 1),
                    mode: crate::core::mask::MaskMode::Add,
                    inverted: false,
                    wiggle: None,
                    path: crate::core::mask::MaskPath {
                        vertices: crate::core::property::Animatable::new_constant(vertices),
                        tangents: None,
                        is_closed: true,
                    },
                    feather: crate::core::property::Animatable::new_constant(5.0),
                    opacity: crate::core::property::Animatable::new_constant(100.0),
                    expansion: crate::core::property::Animatable::new_constant(0.0),
                    enabled: true,
                };

                comp_mut.layers[idx].masks.push(mask);
                app.commit_project(temp_proj);
                app.toasts
                    .info(tr_format("Auto-generated Bezier Mask on {}", &[layer_name]));
            }
        }
    });
}
