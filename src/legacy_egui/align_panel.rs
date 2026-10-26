use crate::core::layer_align::{AlignMode, AlignOperation, DistributeMode};
use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

pub fn draw_align_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "Align & Distribute");

    let align_to_id = egui::Id::new("ae_align_relative_to");
    let mut align_to_comp = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(align_to_id, || true));
    let grid = kui::PropertyGrid::new(ui, &["Align Layers To"]);
    grid.row(ui, "Align Layers To", |ui| {
        if kui::combo(
            ui,
            align_to_id,
            &mut align_to_comp,
            &[(true, "Composition"), (false, "Selection")],
        ) {
            ui.ctx()
                .data_mut(|data| data.insert_temp(align_to_id, align_to_comp));
        }
    });

    let has_selection =
        !app.selection.selected_layers.is_empty() || app.selection.selected_layer_idx.is_some();
    if !has_selection {
        ui.add_space(kui::space::SM);
        kui::secondary(
            ui,
            "No layers selected. Select one or more layers in timeline.",
        );
        return;
    }

    let mut operation = None;
    kui::group(ui, "Align Layers", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, tooltip, mode) in [
                ("Left", "Align Left", AlignMode::Left),
                (
                    "Center H",
                    "Align Horizontal Center",
                    AlignMode::HorizontalCenter,
                ),
                ("Right", "Align Right", AlignMode::Right),
                ("Top", "Align Top", AlignMode::Top),
                (
                    "Center V",
                    "Align Vertical Center",
                    AlignMode::VerticalCenter,
                ),
                ("Bottom", "Align Bottom", AlignMode::Bottom),
            ] {
                if kui::button(ui, label, kui::ButtonKind::Default)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    operation = Some(AlignOperation::Align(mode));
                }
            }
        });
    });

    kui::group(ui, "Distribute Layers", |ui| {
        ui.horizontal_wrapped(|ui| {
            if kui::button(ui, "Distribute H", kui::ButtonKind::Default)
                .on_hover_text("Distribute Horizontal Centers")
                .clicked()
            {
                operation = Some(AlignOperation::Distribute(
                    DistributeMode::HorizontalCenters,
                ));
            }
            if kui::button(ui, "Distribute V", kui::ButtonKind::Default)
                .on_hover_text("Distribute Vertical Centers")
                .clicked()
            {
                operation = Some(AlignOperation::Distribute(DistributeMode::VerticalCenters));
            }
        });
    });

    if let Some(operation) = operation {
        super::align_actions::apply_alignment(app, operation, align_to_comp);
    }
}
