//! Submenus for layer transforms, time operations, and arrangement.

use eframe::egui;

pub fn draw_layer_transform_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Center in Comp")
        .on_hover_text("Move position to the composition center")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position =
                        crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                }
            });
            app.toasts.info("Centered in Comp");
            ui.close_menu();
        }
    }
    if ui
        .button("Fit to Comp")
        .on_hover_text("Scale so the layer covers the full comp frame")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (cw, ch) = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let bs = l.bounding_size();
                    if bs[0] > 1.0 && bs[1] > 1.0 {
                        let s = (cw / bs[0] * 100.0).max(ch / bs[1] * 100.0);
                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                    }
                }
            });
            app.toasts.info("Fit to Comp");
            ui.close_menu();
        }
    }
    if ui.button("Flip Horizontal").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let cf = app.playback.current_frame;
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let s = l.transform.scale.evaluate(cf);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([-s[0], s[1]]);
                }
            });
            app.toasts.info("Flipped Horizontal");
            ui.close_menu();
        }
    }
    if ui.button("Flip Vertical").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let cf = app.playback.current_frame;
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    let s = l.transform.scale.evaluate(cf);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([s[0], -s[1]]);
                }
            });
            app.toasts.info("Flipped Vertical");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui.button("Reset All Transforms").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            let dims = {
                let c = app.history.current().active_composition();
                (c.width as f32, c.height as f32)
            };
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.transform.position = crate::core::property::Animatable::new_constant([
                        dims.0 / 2.0,
                        dims.1 / 2.0,
                    ]);
                    l.transform.scale =
                        crate::core::property::Animatable::new_constant([100.0, 100.0]);
                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                    l.transform.opacity = crate::core::property::Animatable::new_constant(100.0);
                }
            });
            app.toasts.info("All transforms reset");
            ui.close_menu();
        }
    }
    ui.separator();
    if ui
        .add(egui::Button::new("Auto-Orient...").shortcut_text(
            crate::ui::shortcuts::format_shortcut("O", true, false, true),
        ))
        .on_hover_text("Rotate layer automatically along its motion path direction")
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.auto_orient = match l.auto_orient {
                        crate::core::auto_orient::AutoOrientMode::Off => {
                            crate::core::auto_orient::AutoOrientMode::OrientAlongPath
                        }
                        _ => crate::core::auto_orient::AutoOrientMode::Off,
                    };
                }
            });
            app.toasts.info("Toggled Auto-Orient along Motion Path");
        }
        ui.close_menu();
    }
}

pub fn draw_layer_time_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .add(egui::Button::new("Enable Time Remapping").shortcut_text(
            crate::ui::shortcuts::format_shortcut("T", true, false, true),
        ))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.enable_time_remapping();
                }
            });
            app.toasts
                .info("Time remapping enabled — edit keyframes in the Graph Editor");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.add(egui::Button::new("Time-Reverse Layer")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_reverse();
                }
            });
            app.toasts.info("Layer time-reversed");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.button("Freeze Frame at Playhead").clicked() {
        let frame = app.playback.current_frame;
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.freeze_at(frame);
                }
            });
            app.toasts.info(format!("Frozen at source frame {}", frame));
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui.button("Remove Time Remapping").clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.clear_time_remap();
                }
            });
            app.toasts.info("Time remapping removed");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add(egui::Button::new("Time Stretch ×2 (Slow)"))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_stretch(2.0);
                }
            });
            app.toasts.info("Layer stretched to ×2 duration");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Time Stretch ×0.5 (Fast)"))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.time_stretch(0.5);
                }
            });
            app.toasts.info("Layer compressed to ×0.5 duration");
        } else {
            app.toasts.info("Select a layer first");
        }
        ui.close_menu();
    }
}

pub fn draw_layer_arrange_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    let len = app.history.current().active_composition().layers.len();
    let Some(i) = app.selection.selected_layer_idx else {
        ui.label("Select a layer first");
        return;
    };
    if ui.button("Bring to Front").clicked() {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if i < comp.layers.len() && comp.layers.len() > 1 {
                let l = comp.layers.remove(i);
                comp.layers.push(l);
            }
        });
        app.selection.selected_layer_idx = if i < len {
            Some(len - 1)
        } else {
            app.selection.selected_layer_idx
        };
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Bring Forward").shortcut_text(
            crate::ui::shortcuts::format_shortcut("]", true, false, false),
        ))
        .clicked()
    {
        if i + 1 < len {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i + 1 < comp.layers.len() {
                    comp.layers.swap(i, i + 1);
                }
            });
            app.selection.selected_layer_idx = Some(i + 1);
        }
        ui.close_menu();
    }
    if ui
        .add(egui::Button::new("Send Backward").shortcut_text(
            crate::ui::shortcuts::format_shortcut("[", true, false, false),
        ))
        .clicked()
    {
        if i > 0 {
            app.modify_project(move |p| {
                let comp = p.active_composition_mut();
                if i < comp.layers.len() {
                    comp.layers.swap(i - 1, i);
                }
            });
            app.selection.selected_layer_idx = Some(i - 1);
        }
        ui.close_menu();
    }
    if ui.button("Send to Back").clicked() {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if i < comp.layers.len() && comp.layers.len() > 1 {
                let l = comp.layers.remove(i);
                comp.layers.insert(0, l);
            }
        });
        app.selection.selected_layer_idx = if i < len {
            Some(0)
        } else {
            app.selection.selected_layer_idx
        };
        ui.close_menu();
    }
}
