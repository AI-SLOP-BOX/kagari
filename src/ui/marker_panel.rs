use crate::ui::localization::{tr, tr_format};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_marker_panel(app: &mut KagariApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading(tr("Composition Markers"));
    ui.separator();

    ui.horizontal(|ui| {
        if ui
            .button(tr("Add Marker at Current Time (*)"))
            .on_hover_text(tr("Adds marker at current playhead frame"))
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            let m_count = comp_mut.markers.len() + 1;
            comp_mut
                .markers
                .push(crate::core::timeline::TimelineMarker {
                    frame: current_frame,
                    label: tr_format("Cue {}", &[&m_count.to_string()]),
                    color: [0.0, 0.8, 1.0],
                });
            app.commit_project(temp_proj);
            app.toasts.info(tr_format(
                "Added marker at frame {}",
                &[&current_frame.to_string()],
            ));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    let (comp_name, markers_list) = {
        let comp = app.history.current().active_composition();
        (comp.name.clone(), comp.markers.clone())
    };

    ui.label(tr_format("Active Composition: {}", &[&comp_name]));
    ui.label(tr_format(
        "Current Playhead: Frame {}",
        &[&current_frame.to_string()],
    ));

    ui.add_space(6.0);
    ui.label(tr("Markers List:"));
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            if markers_list.is_empty() {
                ui.weak(tr(
                    "No markers in this composition. Click 'Add Marker' above to create one.",
                ));
            } else {
                let mut marker_to_delete = None;
                egui::Grid::new("markers_list_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(tr("Frame")).strong());
                        ui.label(egui::RichText::new(tr("Comment / Note")).strong());
                        ui.label(egui::RichText::new(tr("Action")).strong());
                        ui.end_row();

                        for (m_idx, marker) in markers_list.iter().enumerate() {
                            ui.label(
                                egui::RichText::new(format!("{}", marker.frame))
                                    .monospace()
                                    .color(colors::ACCENT_CYAN),
                            );
                            ui.label(&marker.label);
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button(tr("⏩ Jump"))
                                    .on_hover_text(tr("Jump Playhead to this marker frame"))
                                    .clicked()
                                {
                                    app.playback.current_frame = marker.frame;
                                }
                                if ui.small_button("×").clicked() {
                                    marker_to_delete = Some(m_idx);
                                }
                            });
                            ui.end_row();
                        }
                    });

                if let Some(del_idx) = marker_to_delete {
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    if del_idx < comp_mut.markers.len() {
                        comp_mut.markers.remove(del_idx);
                        app.commit_project(temp_proj);
                    }
                }
            }
        });
}
