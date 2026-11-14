use super::{
    draw_layer_waveform, header, layout, media_preview, pending_actions,
    state::timeline_derived_data, waveform_cache, LayerWaveformParams,
    MAX_TIMELINE_THUMBNAILS_PER_CLIP, TIMELINE_THUMBNAIL_WIDTH,
};
use crate::core::timeline::LabelColor;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use header::draw_timeline_header;

mod clip_draw;
mod clip_interaction;
mod dock_state;
mod graph_mode;
mod layer_context_menu;
mod ruler_area;
mod ruler_draw;
mod switches_col;
mod track_rows;

fn collect_composition_durations(
    composition: &crate::core::timeline::Composition,
    durations: &mut std::collections::HashMap<String, u32>,
) {
    durations.insert(composition.id.clone(), composition.duration_frames);
    for child in &composition.sub_compositions {
        collect_composition_durations(child, durations);
    }
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    let screen_height = ctx.screen_rect().height();
    let narrow_timeline = ctx.screen_rect().width() < 950.0;
    // Keep the timeline large enough to show several tracks without forcing the
    // user to resize the dock on every launch.
    let (timeline_default_height, timeline_min_height, viewer_min_height) = if narrow_timeline {
        (
            (screen_height * 0.30).clamp(160.0, 250.0),
            (screen_height * 0.15).clamp(80.0, 140.0),
            (screen_height * 0.35).clamp(180.0, 320.0),
        )
    } else {
        (
            (screen_height * 0.38).clamp(200.0, 340.0),
            (screen_height * 0.15).clamp(90.0, 150.0),
            (screen_height * 0.40).clamp(200.0, 420.0),
        )
    };
    let timeline_max_height =
        (screen_height * 0.94).min((screen_height - viewer_min_height).max(timeline_min_height));
    egui::TopBottomPanel::bottom("timeline_panel_editor_v12")
        .resizable(true)
        .default_height(timeline_default_height)
        .min_height(timeline_min_height)
        .max_height(timeline_max_height)
        .show(ctx, |ui| {
            if dock_state::draw_special_state(app, ui) {
                return;
            }

            let mut project_changed = false;
            // A full project clone is only needed while a pointer gesture can
            // mutate the timeline. Cloning merely because Graph Editor is
            // visible made an idle editor pay the snapshot cost every frame.
            let pointer_gesture = ui.input(|i| {
                i.pointer.any_pressed() || i.pointer.any_down() || i.pointer.any_released()
            });
            let pre_edit_snapshot = if !app.drag_active() && pointer_gesture {
                Some(app.history.current().clone())
            } else {
                None
            };
            let _compact_mode = ui.ctx().data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new("ae_compact_timeline"), || false)
            });
            let mut actions = pending_actions::TimelineActions::default();

            crate::ui::timeline::breadcrumb::draw_comp_breadcrumb(app, ui);

            let mut header_state = header::TimelineHeaderState {
                bottom_dock_tab: &mut app.ui_tabs.bottom_dock_tab,
                show_switches_pane: &mut app.ui_tabs.show_switches_pane,
                timeline_zoom: &mut app.timeline_zoom,
                snap_to_keyframes: &mut app.snap_to_keyframes,
                show_graph_editor: &mut app.show_graph_editor,
                layer_filter_text: &mut app.ui_tabs.layer_filter_text,
                timeline_view_start: &mut app.timeline_view_start,
                work_area_in: &mut app.playback.work_area_in,
                work_area_out: &mut app.playback.work_area_out,
                expanded_layers: &mut app.selection.expanded_layers,
                fit_to_selection: &mut app.timeline_fit_to_selection,
                fit_all: &mut app.timeline_fit_all,
            };

            // Access live project mutably without per-frame cloning
            {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if draw_timeline_header(
                    &mut header_state,
                    ui,
                    comp_mut,
                    current_frame,
                    total_frames,
                ) {
                    project_changed = true;
                }

                // ── Fit to Selection: zoom timeline to selected layers' time range ──
                if app.timeline_fit_to_selection {
                    app.timeline_fit_to_selection = false;
                    let comp = app.history.current().active_composition();
                    let layers = if app.timeline_fit_all {
                        comp.layers.iter().enumerate().collect::<Vec<_>>()
                    } else {
                        let sel = &app.selection.selected_layers;
                        comp.layers
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| sel.contains(i))
                            .collect::<Vec<_>>()
                    };
                    if !layers.is_empty() {
                        let mut min_f = u32::MAX;
                        let mut max_f = 0u32;
                        for &(_, layer) in &layers {
                            min_f = min_f.min(layer.in_frame);
                            max_f = max_f.max(layer.out_frame);
                        }
                        if max_f > min_f {
                            let span = (max_f - min_f).max(10);
                            app.timeline_zoom =
                                (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                            app.timeline_view_start = min_f;
                        }
                    }
                    app.timeline_fit_all = false;
                }
            }

            let mut automation_curve = app
                .production_document
                .as_ref()
                .and_then(|document| {
                    app.selected_automation_binding
                        .and_then(|index| document.bindings.get(index))
                })
                .map(|binding| binding.curve.clone());
            // Graph mode is the animation curve workspace; dependency nodes are
            // intentionally kept out of the compositing timeline.
            if app.show_graph_editor {
                graph_mode::draw(
                    app,
                    ui,
                    &mut automation_curve,
                    &mut project_changed,
                    pre_edit_snapshot,
                );
                return;
            }

            track_rows::draw_track_rows(
                app,
                ui,
                current_frame,
                total_frames,
                &mut actions,
                &mut project_changed,
            );

            crate::ui::timeline::pending_actions::apply_deferred(
                app,
                ui,
                *current_frame,
                &mut project_changed,
                &mut actions,
            );

            if project_changed {
                // Transactional undo commit: snapshot on pointer-down, single entry on release
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if is_pointer_down {
                    let edit_label = if app.show_graph_editor {
                        "Graph Editor Edit"
                    } else {
                        "Timeline Edit"
                    };
                    if let Some(snapshot) = pre_edit_snapshot {
                        app.begin_drag_with_snapshot(snapshot, edit_label);
                    } else if !app.drag_active() {
                        app.begin_drag(edit_label);
                    }
                } else {
                    app.commit_drag();
                }
                crate::core::frame_cache::bump_version();
            }
        });
}
