use crate::core::layer_align::{self, AlignOperation};
use crate::KagariApp;

pub fn apply_alignment(
    app: &mut KagariApp,
    operation: AlignOperation,
    relative_to_composition: bool,
) -> bool {
    let mut selected: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
    if let Some(primary) = app.selection.selected_layer_idx {
        if !selected.contains(&primary) {
            selected.push(primary);
        }
    }
    if selected.is_empty() {
        return false;
    }

    let mut project = app.history.current().clone();
    let changed = layer_align::apply_to_composition(
        project.active_composition_mut(),
        &selected,
        app.playback.current_frame,
        operation,
        relative_to_composition,
    );
    if changed {
        app.commit_project(project);
    }
    changed
}
