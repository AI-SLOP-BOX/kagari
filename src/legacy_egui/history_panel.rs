//! Undo History panel: lists named history steps newest-first, highlights
//! the active entry, and jumps to any step on click.
use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

pub fn draw_history_panel(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_history_panel {
        return;
    }

    let mut open = app.show_history_panel;
    crate::ui::modal::window(crate::ui::localization::tr("Undo History"))
        .open(&mut open)
        .default_width(260.0)
        .default_height(320.0)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -46.0))
        .show(ctx, |ui| {
            // Snapshot metadata so project borrow ends before jump mutation.
            let (names, current_idx) = {
                let names: Vec<String> = (0..app.history.len())
                    .filter_map(|i| app.history.action_name_at(i).map(|s| s.to_string()))
                    .collect();
                (names, app.history.current_index())
            };

            if names.is_empty() {
                kui::empty_state(ui, &crate::ui::localization::tr("No history yet."), 72.0);
                return;
            }

            kui::secondary(
                ui,
                crate::ui::localization::tr_format(
                    "{} steps — click to jump",
                    &[&names.len().to_string()],
                ),
            );
            kui::separator(ui);

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut jump_target: Option<usize> = None;
                // Newest first for AE-like reading order.
                for (i, name) in names.iter().enumerate().rev() {
                    let is_current = i == current_idx;
                    let is_future = i > current_idx;
                    let title = if is_current {
                        format!("● {name}")
                    } else {
                        name.clone()
                    };
                    let row = kui::selectable_item_row(ui, is_current, 28.0, |ui| {
                        if is_future {
                            kui::truncated_secondary(ui, &title);
                        } else {
                            kui::truncated_primary(ui, &title);
                        }
                    });
                    if row.response.clicked() {
                        jump_target = Some(i);
                    }
                }
                if let Some(target) = jump_target {
                    if app.history.jump_to(target) {
                        app.toasts.info(crate::ui::localization::tr_format(
                            "History → step {} ({})",
                            &[&(target + 1).to_string(), &names[target]],
                        ));
                    }
                }
            });
        });

    app.show_history_panel = open;
}
