use crate::core::property::Animatable;
use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

pub fn draw_the_smoother_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "The Smoother");
    kui::secondary(ui, "Reduce keyframe density using RDP curve fitting");
    kui::separator(ui);

    let tol_id = egui::Id::new("ae_smoother_tolerance");
    let mut tolerance: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(tol_id, || 2.0));

    let mode_id = egui::Id::new("ae_smoother_mode");
    let mut mode_idx: usize = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(mode_id, || 0));

    let grid = kui::PropertyGrid::new(ui, &["Apply To", "Tolerance"]);
    if grid.row(ui, "Apply To", |ui| {
        kui::combo(
            ui,
            mode_id,
            &mut mode_idx,
            &[(0, "Spatial Path"), (1, "Temporal Graph")],
        )
    }) {
        ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx));
    }
    if grid
        .row(ui, "Tolerance", |ui| {
            kui::slider(
                ui,
                &mut tolerance,
                0.1..=50.0,
                if mode_idx == 0 { " px" } else { " val" },
            )
        })
        .changed()
    {
        ui.ctx().data_mut(|d| d.insert_temp(tol_id, tolerance));
    }

    kui::separator(ui);

    if kui::button_enabled(
        ui,
        "Apply Smoother",
        ButtonKind::Accent,
        app.selection.selected_layer_idx.is_some(),
    )
    .on_hover_text(crate::ui::localization::tr(
        "Simplify keyframes on active selected layer",
    ))
    .clicked()
    {
        let Some(layer_idx) = app.selection.selected_layer_idx else {
            app.toasts.error(crate::ui::localization::tr(
                "Select a layer with animated keyframes first",
            ));
            return;
        };

        let mut temp_proj = app.history.current().clone();
        let comp = temp_proj.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(layer_idx) else {
            return;
        };

        let mut reduced_count = 0;

        if mode_idx == 0 {
            // Simplify 2D position path
            if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_vec2(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
        } else {
            // Simplify temporal scalar and vector curves (rotation, opacity, scale)
            if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_scalar(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
            if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_scalar(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
            if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_vec2(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
        }

        if reduced_count > 0 {
            app.commit_project(temp_proj);
            app.toasts.info(crate::ui::localization::tr_format(
                "The Smoother: Removed {} redundant keyframes",
                &[&reduced_count.to_string()],
            ));
        } else {
            app.toasts.info(crate::ui::localization::tr(
                "Keyframes are already optimal for this tolerance",
            ));
        }
    }
}
