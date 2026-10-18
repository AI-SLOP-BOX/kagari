use crate::KagariApp;
use eframe::egui;

pub(crate) fn draw_character_tab(app: &mut KagariApp, ui: &mut egui::Ui, current_frame: u32) {
    let edit_input = ui.input(|input| {
        input.pointer.any_pressed()
            || input.pointer.any_down()
            || input.pointer.any_released()
            || input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Text(_)
                        | egui::Event::Key { pressed: true, .. }
                        | egui::Event::MouseWheel { .. }
                )
            })
    });
    let pre_snapshot = if !app.drag_active() && edit_input {
        Some(app.history.current().clone())
    } else {
        None
    };
    let mut temp_comp = app.history.current().active_composition().clone();
    let changed =
        crate::ui::character_panel::draw_character_panel(app, ui, &mut temp_comp, current_frame);
    if changed {
        let is_pointer_down = ui.input(|i| i.pointer.any_down());
        *app.history.current_mut().active_composition_mut() = temp_comp;
        if is_pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = pre_snapshot {
                    app.begin_drag_with_snapshot(snapshot, "Character Edit");
                }
            }
            crate::core::frame_cache::bump_version();
        } else if app.drag_active() {
            app.commit_drag();
        } else if let Some(snapshot) = pre_snapshot {
            app.begin_drag_with_snapshot(snapshot, "Character Edit");
            app.commit_drag();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::effects_library::color_controls::{color_control_value, set_color_control};
    use crate::ui::effects_library::effect_controls::draw_effect_controls;

    fn draw_properties_panel(app: &mut KagariApp) {
        let ctx = egui::Context::default();
        let mut frame = 0u32;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 800.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut next_frame = None;
                    let mut slider_changed = false;
                    draw_effect_controls(app, ui, frame, &mut next_frame, &mut slider_changed);
                    frame = next_frame.unwrap_or(frame);
                });
            },
        );
    }

    #[test]
    fn properties_panel_renders_without_selection_and_with_layer() {
        let mut app = KagariApp::default();
        let comp_duration = app.history.current().active_composition().duration_frames;
        app.history
            .current_mut()
            .active_composition_mut()
            .layers
            .push(crate::core::timeline::Layer::new_null(
                "layer_1".to_string(),
                "Layer 1".to_string(),
                comp_duration,
            ));
        // No selection: must not panic, must not start a drag.
        draw_properties_panel(&mut app);
        assert!(!app.drag_active());

        // Selected layer without effects: transform section + empty state.
        app.selection.selected_layer_idx = Some(0);
        draw_properties_panel(&mut app);
        assert!(!app.drag_active());

        // Selected layer with an effect: transform + effect rows incl. Reset.
        let effect = crate::ui::effects_controls::presets::get_all_effect_presets()
            .iter()
            .find(|p| p.name == "Gaussian Blur")
            .map(|p| (p.create_fn)(0))
            .expect("Gaussian Blur preset must exist");
        app.modify_project(|p| {
            p.active_composition_mut().layers[0].effects.push(effect);
        });
        draw_properties_panel(&mut app);
        assert_eq!(
            app.history.current().active_composition().layers[0]
                .effects
                .len(),
            1
        );
    }

    #[test]
    fn color_controls_write_renderable_effect_parameters() {
        let mut layer = crate::core::timeline::Layer::new(
            "layer_1".to_string(),
            "Layer 1".to_string(),
            crate::core::timeline::LayerType::Null,
            30,
        );
        set_color_control(&mut layer, "exposure", 4, 1.0);
        set_color_control(&mut layer, "saturation", 4, 1.5);
        assert_eq!(color_control_value(&layer, "exposure", 4), 1.0);
        assert_eq!(color_control_value(&layer, "saturation", 4), 1.5);
        assert!(matches!(
            layer.effects.iter().find(|effect| effect.id == "ui_color_exposure").map(|effect| &effect.effect_type),
            Some(crate::core::timeline::EffectType::Levels { gamma, .. }) if (gamma.evaluate(4) - 2.0).abs() < 1e-6
        ));
    }
}
