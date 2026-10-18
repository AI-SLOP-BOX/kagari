mod camera_tracker;
mod planar_tracker;
mod point_tracker;
mod roto_autotrace;
mod workers;

use crate::ui::localization::tr;
use crate::KagariApp;
use eframe::egui;

pub(crate) use workers::{poll_camera_solve, poll_quad_track};

pub fn draw_tracker_panel(app: &mut KagariApp, ui: &mut egui::Ui, current_frame: u32) {
    poll_camera_solve(app, ui.ctx());
    poll_quad_track(app, ui.ctx());
    ui.heading(tr("Tracker"));
    ui.separator();

    let sel_idx = app.selection.selected_layer_idx;
    let layer_name = {
        let comp = app.history.current().active_composition();
        match sel_idx.and_then(|i| comp.layers.get(i)) {
            Some(l) => l.name.clone(),
            None => {
                ui.weak(tr("No layer selected. Select a layer in timeline."));
                return;
            }
        }
    };

    if let Some(idx) = sel_idx {
        if idx < app.history.current().active_composition().layers.len() {
            point_tracker::draw_point_tracker_section(app, ui, idx, current_frame);

            camera_tracker::draw_camera_tracker_section(app, ui, idx);

            planar_tracker::draw_planar_tracker_section(app, ui, idx, current_frame);

            roto_autotrace::draw_roto_autotrace_section(app, ui, idx, &layer_name, current_frame);
        } else {
            ui.weak(tr("Select a layer to perform motion tracking."));
        }
    } else {
        ui.weak(tr("No layer selected. Select a layer in timeline."));
    }
}
