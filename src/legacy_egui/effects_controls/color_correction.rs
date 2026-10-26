mod advanced;
mod basic;

use crate::core::timeline::EffectType;
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    if basic::draw_basic_color(effect_type, ui, current_frame, project_changed, next_frame) {
        return;
    }
    advanced::draw_advanced_color(effect_type, ui, current_frame, project_changed, next_frame);
}
