use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

/// AE Keyframe Assistant → Sequence Layers:
/// rearranges the selected layers so they play one after another,
/// optionally overlapping by a fixed number of frames.
pub fn draw_sequence_layers_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_sequence_layers {
        return;
    }

    let mut open = app.show_sequence_layers;
    crate::ui::modal::window(crate::ui::localization::tr("Sequence Layers"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            let selected: Vec<usize> = {
                let mut s: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
                s.sort_unstable();
                s
            };

            if selected.len() < 2 {
                kui::empty_state(
                    ui,
                    &crate::ui::localization::tr("Select at least 2 layers to sequence."),
                    64.0,
                );
                kui::separator(ui);
                if kui::button(ui, "Close", ButtonKind::Default).clicked() {
                    app.show_sequence_layers = false;
                }
                return;
            }

            kui::secondary(
                ui,
                crate::ui::localization::tr_format(
                    "{} layers selected",
                    &[&selected.len().to_string()],
                ),
            );
            ui.add_space(kui::space::XS);

            let overlap_id = egui::Id::new("ae_seq_layers_overlap");
            let mut overlap = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(overlap_id, || 0i32));
            let overlap_label = crate::ui::localization::tr("Overlap");
            let grid = kui::PropertyGrid::new(ui, &[&overlap_label]);
            grid.row(ui, &overlap_label, |ui| {
                let unit = crate::ui::localization::tr(" frames");
                ui.add(kui::drag(&mut overlap).range(-300..=300).unit(&unit))
                    .on_hover_text(crate::ui::localization::tr(
                        "Negative = gap between layers, 0 = butt-jointed, positive = crossfade zone",
                    ))
            });
            ui.ctx().data_mut(|d| d.insert_temp(overlap_id, overlap));

            // Preview of resulting schedule
            ui.add_space(kui::space::SM);
            {
                let comp = app.history.current().active_composition();
                let mut cursor: Option<u32> = None;
                for &idx in &selected {
                    if idx >= comp.layers.len() {
                        continue;
                    }
                    let l = &comp.layers[idx];
                    let start = cursor.unwrap_or(l.in_frame);
                    ui.monospace(format!(
                        "  {:<18} {} → {}",
                        truncate(&l.name, 16),
                        start,
                        start + (l.out_frame - l.in_frame)
                    ));
                    let span = (l.out_frame - l.in_frame) as i32;
                    cursor = Some((start as i32 + span - overlap).max(0) as u32);
                }
            }

            kui::separator(ui);
            ui.horizontal(|ui| {
                if kui::button(ui, "Apply Sequence", ButtonKind::Accent).clicked() {
                    let overlap_f = overlap;
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    // Anchor at earliest in-point among selected layers
                    let anchor = selected
                        .iter()
                        .filter_map(|&i| comp_mut.layers.get(i).map(|l| l.in_frame))
                        .min()
                        .unwrap_or(0);
                    let mut cursor = anchor;
                    for &idx in &selected {
                        if idx >= comp_mut.layers.len() {
                            continue;
                        }
                        let l = &mut comp_mut.layers[idx];
                        let span = l.out_frame - l.in_frame;
                        l.in_frame = cursor;
                        l.out_frame = cursor + span.max(1);
                        cursor =
                            (cursor as i64 + span.max(1) as i64 - overlap_f as i64).max(0) as u32;
                    }
                    app.commit_project(temp_proj);
                    app.toasts.info(crate::ui::localization::tr_format(
                        "Sequenced {} layers (overlap {}f)",
                        &[&selected.len().to_string(), &overlap.to_string()],
                    ));
                    app.show_sequence_layers = false;
                }
                if kui::button(ui, "Cancel", ButtonKind::Quiet).clicked() {
                    app.show_sequence_layers = false;
                }
            });
        });

    app.show_sequence_layers = open;
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}
