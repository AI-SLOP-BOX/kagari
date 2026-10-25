use crate::core::timeline::EffectType;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::MergePaths { operation } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Operation",
                operation,
                |ui, v| {
                    let options = [
                        (0.0_f32, "Add"),
                        (1.0, "Subtract"),
                        (2.0, "Intersect"),
                        (3.0, "Exclude"),
                    ];
                    let mut selected = v.round().clamp(0.0, 3.0);
                    if crate::ui::kui::combo(ui, "merge_paths_operation", &mut selected, &options) {
                        *v = selected;
                    }
                },
            );
        }
        EffectType::OffsetPath {
            amount,
            line_join,
            miter_limit,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -100.0..=100.0, " px");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Line Join",
                line_join,
                |ui, v| {
                    let options = [(0.0_f32, "Miter"), (1.0, "Round"), (2.0, "Bevel")];
                    let mut selected = v.round().clamp(0.0, 2.0);
                    if crate::ui::kui::combo(ui, "offset_path_line_join", &mut selected, &options) {
                        *v = selected;
                    }
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Miter Limit",
                miter_limit,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=100.0, "");
                },
            );
        }
        _ => {}
    }
}
