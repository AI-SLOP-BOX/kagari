use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

pub fn apply_text_preset(
    app: &mut KagariApp,
    preset: &str,
    layer_idx: usize,
    current_frame: u32,
    duration_frames: u32,
) {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;

    let duration = if duration_frames > 0 {
        duration_frames
    } else {
        30
    };
    let ease = InterpolationType::Bezier {
        outgoing: crate::core::keyframe::BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        incoming: crate::core::keyframe::BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
    };

    app.modify_project(|project| {
        let composition = project.active_composition_mut();
        let Some(layer) = composition.layers.get_mut(layer_idx) else {
            return;
        };

        let base_position = layer.transform.position.evaluate(current_frame);
        let end_frame = current_frame + duration;

        match preset {
            "Fade In" => {
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Left" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(
                        current_frame,
                        [base_position[0] - 200.0, base_position[1]],
                        ease,
                    ),
                    Keyframe::new(end_frame, base_position, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Right" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(
                        current_frame,
                        [base_position[0] + 200.0, base_position[1]],
                        ease,
                    ),
                    Keyframe::new(end_frame, base_position, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Up" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(
                        current_frame,
                        [base_position[0], base_position[1] + 150.0],
                        ease,
                    ),
                    Keyframe::new(end_frame, base_position, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Scale Up" => {
                layer.transform.scale = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, [0.0, 0.0], ease),
                    Keyframe::new(end_frame, [100.0, 100.0], ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            _ => {}
        }
    });
}

pub fn draw_animation_presets(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    duration_frames: u32,
) {
    kui::separator(ui);
    kui::section_title(ui, "Text Animation Presets:");
    let presets = [
        "Fade In",
        "Slide In Left",
        "Slide In Right",
        "Slide In Up",
        "Scale Up",
    ];
    ui.horizontal_wrapped(|ui| {
        for preset in presets {
            if kui::button(ui, preset, ButtonKind::Default).clicked() {
                if let Some(idx) = app.selection.selected_layer_idx {
                    apply_text_preset(app, preset, idx, current_frame, duration_frames);
                }
            }
        }
    });
}
