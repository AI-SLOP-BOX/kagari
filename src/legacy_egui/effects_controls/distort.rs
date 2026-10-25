mod glitch_displace;
mod warp_lens;

use crate::core::timeline::EffectType;
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    if warp_lens::draw_warp_lens(effect_type, ui, current_frame, project_changed, next_frame) {
        return;
    }
    glitch_displace::draw_glitch_displace(
        effect_type,
        ui,
        current_frame,
        project_changed,
        next_frame,
    );
}
