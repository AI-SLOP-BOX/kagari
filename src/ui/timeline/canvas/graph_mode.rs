use crate::core::automation_binding::AutomationCurve;
use crate::core::timeline::Project;
use crate::KagariApp;
use eframe::egui;

pub(super) fn draw(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    automation_curve: &mut Option<AutomationCurve>,
    project_changed: &mut bool,
    pre_edit_snapshot: Option<Project>,
) {
    if let Some(selected_idx) = app.selection.selected_layer_idx {
        let duration_f = app.history.current().active_composition().duration_frames;
        let fps = app.history.current().active_composition().fps;
        if let Some(layer) = app
            .history
            .current_mut()
            .active_composition_mut()
            .layers
            .get_mut(selected_idx)
        {
            crate::ui::graph_editor::draw_graph_editor(
                &mut app.selection.selected_property,
                ui,
                duration_f,
                fps,
                layer,
                project_changed,
                &mut app.linked_tangent,
            );
        }
        if let Some(curve) = automation_curve.as_mut() {
            crate::ui::graph_editor::draw_automation_curve(ui, curve, project_changed);
        }
    } else {
        ui.label("Select a layer to edit keyframe curves in Graph Editor");
    }

    if let (Some(curve), Some(index), Some(document)) = (
        automation_curve.take(),
        app.selected_automation_binding,
        app.production_document.as_mut(),
    ) {
        if let Some(binding) = document.bindings.get_mut(index) {
            binding.curve = curve;
        }
    }
    if *project_changed {
        if let Some(snapshot) = pre_edit_snapshot {
            app.begin_drag_with_snapshot(snapshot, "Graph Editor Edit");
        }
        crate::core::frame_cache::bump_version();
        app.autosave.mark_dirty();
    }
    if !ui.input(|input| input.pointer.any_down()) {
        app.commit_drag();
    }
}
