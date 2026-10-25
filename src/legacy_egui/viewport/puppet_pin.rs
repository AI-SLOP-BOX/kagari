use super::*;

pub(super) fn draw(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    viewport_response: &egui::Response,
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
    if app.active_tool == crate::ui::toolbar::ActiveTool::PuppetPin {
        if let Some(sel_li) = app.selection.selected_layer_idx {
            // 1) Draw pins & detect hover (read-only borrow).
            let mut hits: Vec<(usize, egui::Pos2)> = Vec::new();
            let mut placed: Vec<egui::Pos2> = Vec::new();
            {
                let comp_ro = app.history.current().active_composition();
                if let Some(layer) = comp_ro.layers.get(sel_li) {
                    for (pi, pin) in layer.puppet_pins.iter().enumerate() {
                        let wp = pin.position.evaluate(current_frame);
                        let sp = egui::pos2(
                            origin_x + wp[0] / comp_w * draw_w,
                            origin_y + wp[1] / comp_h * draw_h,
                        );
                        placed.push(sp);
                        if let Some(ptr) = viewport_response.interact_pointer_pos() {
                            if ptr.distance(sp) < 9.0 {
                                hits.push((pi, sp));
                            }
                        }
                    }
                }
            }
            for (i, sp) in placed.iter().enumerate() {
                let hovered = hits.iter().any(|(hi, _)| *hi == i);
                ui.painter().circle_filled(
                    *sp,
                    if hovered { 6.0 } else { 4.5 },
                    colors::ACCENT_YELLOW.linear_multiply(0.85),
                );
                ui.painter().circle_stroke(
                    *sp,
                    4.5,
                    egui::Stroke::new(1.2_f32, egui::Color32::WHITE),
                );
            }

            let drag_id = egui::Id::new(("puppet_pin_drag", sel_li));
            let pointer = viewport_response.interact_pointer_pos();
            let comp_pt = pointer.map(|pp| {
                [
                    (pp.x - origin_x) / draw_w * comp_w,
                    (pp.y - origin_y) / draw_h * comp_h,
                ]
            });

            // 2) Start drag on a pin.
            if viewport_response.drag_started() {
                if let Some(&(pi, _)) = hits.first() {
                    ui.ctx().data_mut(|d| d.insert_temp(drag_id, pi));
                    app.begin_drag("Puppet Pin Move");
                }
            }
            // 3) Drag update: upsert keyframe at playhead.
            if viewport_response.dragged() && comp_pt.is_some() {
                let active = ui.ctx().data(|d| d.get_temp::<usize>(drag_id));
                if let Some(pi) = active {
                    if let Some(pt) = comp_pt {
                        let proj = app.history.current_mut().active_composition_mut();
                        if let Some(layer) = proj.layers.get_mut(sel_li) {
                            if let Some(pin) = layer.puppet_pins.get_mut(pi) {
                                match &mut pin.position {
                                    crate::core::property::Animatable::Constant(v) => *v = pt,
                                    crate::core::property::Animatable::Animated(kfs) => {
                                        if let Some(k) =
                                            kfs.iter_mut().find(|k| k.frame == current_frame)
                                        {
                                            k.value = pt;
                                        } else {
                                            kfs.push(crate::core::keyframe::Keyframe::new(
                                                current_frame,
                                                pt,
                                                crate::core::keyframe::InterpolationType::Linear,
                                            ));
                                            kfs.sort_by_key(|k| k.frame);
                                        }
                                    }
                                }
                                crate::core::frame_cache::bump_version();
                            }
                        }
                    }
                }
            }
            // 4) End drag → single undo entry.
            if viewport_response.drag_stopped() {
                let had = ui.ctx().data(|d| d.get_temp::<usize>(drag_id).is_some());
                ui.ctx().data_mut(|d| d.remove_temp::<usize>(drag_id));
                if had {
                    app.commit_drag();
                }
            }
            // 5) Click empty space → create a new pin there.
            if viewport_response.clicked() && hits.is_empty() {
                if let Some(pt) = comp_pt {
                    let proj = app.history.current_mut().active_composition_mut();
                    if let Some(layer) = proj.layers.get_mut(sel_li) {
                        let n = layer.puppet_pins.len() + 1;
                        layer
                            .puppet_pins
                            .push(crate::core::timeline::PuppetPin::new(
                                format!("pin_{}", n),
                                format!("Pin {}", n),
                                pt,
                            ));
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Puppet pin {} added", n));
                    }
                }
            }
        }
    }
}
