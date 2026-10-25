//! Composition breadcrumb bar: shows the trail of nested-composition
//! navigation (double-click PreComp) and lets the user jump back.
use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

/// Draw the breadcrumb strip. No-op when the nav stack is empty
/// (i.e. user is at a top-level composition).
pub fn draw_comp_breadcrumb(app: &mut KagariApp, ui: &mut egui::Ui) {
    if app.comp_nav_stack.is_empty() {
        return;
    }

    // Snapshot labels first so the project borrow ends before mutations.
    let (stack_labels, current_name) = {
        let proj = app.history.current();
        let labels: Vec<String> = app
            .comp_nav_stack
            .iter()
            .map(|&idx| {
                proj.compositions
                    .get(idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Comp #{}", idx))
            })
            .collect();
        (labels, proj.active_composition().name.clone())
    };

    let mut jump_to: Option<(usize, usize)> = None; // (stack_pos, comp_idx)
    ui.horizontal(|ui| {
        if kui::breadcrumb_link(ui, "⌂", "トップレベルのコンポジション").clicked() {
            if let Some(root) = app.comp_nav_stack.first().copied() {
                jump_to = Some((0, root));
            }
        }

        for (pos, label) in stack_labels.iter().enumerate() {
            if kui::breadcrumb_link(ui, label, &format!("{} に移動", label)).clicked() {
                jump_to = Some((pos, app.comp_nav_stack[pos]));
            }
            kui::breadcrumb_separator(ui);
        }

        // Current location is primary text; ancestors remain quiet navigation links.
        kui::breadcrumb_current(ui, &current_name);
    });

    if let Some((pos, comp_idx)) = jump_to {
        // Truncate stack so clicked crumb becomes current, then navigate.
        app.comp_nav_stack.truncate(pos);
        let mut project = app.history.current().clone();
        project.active_composition_idx = comp_idx;
        app.commit_project(project);
        app.selection.selected_layer_idx = None;
        app.selection.selected_layers.clear();
    }
}
