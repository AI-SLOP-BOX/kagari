use crate::core::layer_align::{AlignMode, AlignOperation, DistributeMode};
use crate::ui::kui;
use crate::ui::localization::tr;
use crate::KagariApp;
use eframe::egui;

pub fn draw_alignment_hud(app: &mut KagariApp, ui: &mut egui::Ui) {
    let mut operation = None;
    kui::toolbar(ui, |ui| {
        kui::toolbar_group(ui, |ui| {
            kui::caption(ui, tr("Align:"));
            for (glyph, label, mode) in [
                ("⇤", "Align Left", AlignMode::Left),
                ("↔", "Align Horizontal Center", AlignMode::HorizontalCenter),
                ("⇥", "Align Right", AlignMode::Right),
                ("⤒", "Align Top", AlignMode::Top),
                ("↕", "Align Vertical Center", AlignMode::VerticalCenter),
                ("⤓", "Align Bottom", AlignMode::Bottom),
            ] {
                let response = kui::icon_button(ui, glyph, label, false, 28.0);
                if response.clicked() || response.changed() {
                    operation = Some(AlignOperation::Align(mode));
                }
            }
        });

        ui.add_space(kui::space::MD);
        kui::toolbar_group(ui, |ui| {
            kui::caption(ui, tr("Distribute:"));
            let horizontal = kui::icon_button(
                ui,
                "⇤⇥",
                "Distribute Selected Layers Horizontally",
                false,
                28.0,
            );
            if horizontal.clicked() || horizontal.changed() {
                operation = Some(AlignOperation::Distribute(
                    DistributeMode::HorizontalCenters,
                ));
            }
            let vertical = kui::icon_button(
                ui,
                "⤒⤓",
                "Distribute Selected Layers Vertically",
                false,
                28.0,
            );
            if vertical.clicked() || vertical.changed() {
                operation = Some(AlignOperation::Distribute(DistributeMode::VerticalCenters));
            }
        });
    });

    if let Some(operation) = operation {
        super::align_actions::apply_alignment(app, operation, true);
    }
}
