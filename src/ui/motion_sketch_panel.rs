//! Motion Sketch Panel (AE Parity).
//!
//! Interactive live mouse path recording for real-time motion path capture.
//! When armed and dragging in viewport, records cursor coordinates into position keyframes
//! as time steps forward.

use crate::ui::kui::{self, ButtonKind, StatusKind};
use crate::KagariApp;
use eframe::egui;

pub fn draw_motion_sketch_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "✏️ Motion Sketch");
    kui::secondary(
        ui,
        "Draw motion paths in real-time by dragging in the viewport",
    );
    kui::separator(ui);

    let smoothing_id = egui::Id::new("ae_motion_sketch_smoothing");
    let mut smoothing: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(smoothing_id, || 2.0));

    let capture_speed_id = egui::Id::new("ae_motion_sketch_speed");
    let mut speed_pct: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(capture_speed_id, || 100.0));

    let grid = kui::PropertyGrid::new(ui, &["Smoothing", "Capture Speed"]);
    if grid
        .row(ui, "Smoothing", |ui| {
            kui::slider(ui, &mut smoothing, 0.0..=20.0, " px")
        })
        .changed()
    {
        ui.ctx()
            .data_mut(|d| d.insert_temp(smoothing_id, smoothing));
    }
    if grid
        .row(ui, "Capture Speed", |ui| {
            kui::slider(ui, &mut speed_pct, 25.0..=200.0, " %")
        })
        .changed()
    {
        ui.ctx()
            .data_mut(|d| d.insert_temp(capture_speed_id, speed_pct));
    }

    kui::separator(ui);

    let is_sketching_id = egui::Id::new("ae_motion_sketch_armed");
    let is_armed: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(is_sketching_id, || false));

    if is_armed {
        kui::status(
            ui,
            StatusKind::Warning,
            "ARMED: Click and drag in Viewport to record motion path...",
        );
        if kui::button(ui, "⏹ Stop / Disarm", ButtonKind::Quiet).clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(is_sketching_id, false));
            app.toasts
                .info(crate::ui::localization::tr("Motion Sketch disarmed"));
        }
    } else {
        if kui::button_enabled(
            ui,
            "Start Capture",
            ButtonKind::Accent,
            app.selection.selected_layer_idx.is_some(),
        )
        .on_hover_text(crate::ui::localization::tr(
            "Arm motion sketch. Drag in viewport to record.",
        ))
        .clicked()
        {
            if app.selection.selected_layer_idx.is_some() {
                ui.ctx().data_mut(|d| d.insert_temp(is_sketching_id, true));
                app.toasts.info(crate::ui::localization::tr(
                    "Motion Sketch armed: Drag in Viewport to record!",
                ));
            } else {
                app.toasts
                    .error(crate::ui::localization::tr("Select a target layer first"));
            }
        }
    }
}
