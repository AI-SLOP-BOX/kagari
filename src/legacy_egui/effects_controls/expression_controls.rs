use crate::core::timeline::EffectType;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    effect_id: &str,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::SliderControl { value } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Value",
                value,
                |ui, v| {
                    ui.add(crate::ui::kui::drag(v).speed(0.1));
                },
            );
        }
        EffectType::AngleControl { angle_degrees } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_degrees,
                |ui, v| {
                    ui.add(crate::ui::kui::drag(v).speed(1.0).unit("°"));
                },
            );
        }
        EffectType::PointControl { point } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Point",
                point,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            );
        }
        EffectType::Letterbox { frac } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bars (frame frac)",
                frac,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=0.45, "");
                },
            );
        }
        EffectType::ColorControl { color } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Color",
                color,
                |ui, val| {
                    crate::ui::kui::color_rgba(ui, val);
                },
            );
        }
        EffectType::CheckboxControl { checked } => {
            if crate::ui::kui::toggle(ui, checked, "Enabled").changed() {
                *project_changed = true;
            }
        }
        EffectType::DropdownControl { value, options } => {
            let id = ui.make_persistent_id((effect_id, "dropdown_ctrl"));
            let values = options
                .iter()
                .enumerate()
                .map(|(index, option)| (index as i32, option.as_str()))
                .collect::<Vec<_>>();
            *project_changed |= crate::ui::kui::combo(ui, id, value, &values);
        }
        EffectType::Point3DControl { point } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Point 3D",
                point,
                |ui, val| {
                    crate::ui::kui::vector(ui, val, f32::MIN..=f32::MAX, 1.0, "");
                },
            );
        }
        _ => {}
    }
}
