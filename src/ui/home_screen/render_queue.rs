use super::*;

pub(in crate::ui::home_screen) fn draw_home_render_queue(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    rect: egui::Rect,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let queue_items = app.render_queue_items.clone();
    let queue_status = app.render_item_status.clone();
    let compositions: std::collections::HashMap<_, _> = app
        .history
        .current()
        .compositions
        .iter()
        .map(|composition| (composition.id.clone(), composition.name.clone()))
        .collect();
    let active_comp = app.history.current().active_composition();
    let active_id = active_comp.id.clone();
    let rendering_id = app.export.export_comp_id.clone();
    let export_progress = app.export.export_progress.clamp(0.0, 1.0);
    let exporting = app.export.is_exporting;
    let mut add_active = false;
    let mut clear_queue = false;
    let mut remove_index = None;
    let mut activate_id = None;

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        egui::Frame::none()
            .fill(crate::ui::theme::colors::BG_PANEL_BASE)
            .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
            .rounding(8.0)
            .inner_margin(egui::Margin::symmetric(16.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("レンダーキュー  {}", queue_items.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        clear_queue = ui
                            .add_enabled(!exporting && !queue_items.is_empty(), egui::Button::new("キューを消去"))
                            .clicked();
                        add_active = ui
                            .add_enabled(
                                !queue_items.contains(&active_id),
                                egui::Button::new("現在のコンポを追加"),
                            )
                            .clicked();
                    });
                });
                ui.separator();
                if queue_items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("キューは空です。書き出すコンポジションを追加してください。")
                                .color(colors::TEXT_MUTED),
                        );
                    });
                } else {
                    let row_height = 34.0;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_rows(ui, row_height, queue_items.len(), |ui, visible| {
                            for index in visible {
                                let composition_id = &queue_items[index];
                                let display_name = compositions
                                    .get(composition_id)
                                    .map(String::as_str)
                                    .unwrap_or("Missing composition");
                                let missing = !compositions.contains_key(composition_id);
                                let status = queue_status.get(composition_id).copied().unwrap_or(
                                    crate::app_state::QueueItemStatus::Queued,
                                );
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", index + 1));
                                    let label = if missing {
                                        egui::RichText::new(display_name).color(colors::TEXT_MUTED)
                                    } else {
                                        egui::RichText::new(display_name).color(colors::TEXT_PRIMARY)
                                    };
                                    let label_width = (ui.available_width() - 180.0).max(80.0);
                                    if ui
                                        .add_sized(
                                            egui::vec2(label_width, 24.0),
                                            egui::Label::new(label).truncate().selectable(true),
                                        )
                                        .clicked()
                                        && !missing
                                    {
                                        activate_id = Some(composition_id.clone());
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.add_enabled(!exporting, egui::Button::new("削除")).clicked() {
                                                remove_index = Some(index);
                                            }
                                            let (status_text, status_color) = if missing {
                                                ("コンポジションがありません", colors::ACCENT_RED)
                                            } else {
                                                match status {
                                                    crate::app_state::QueueItemStatus::Queued => ("待機中", colors::TEXT_SECONDARY),
                                                    crate::app_state::QueueItemStatus::Rendering => ("書き出し中", colors::ACCENT_CYAN),
                                                    crate::app_state::QueueItemStatus::Done => ("完了", colors::ACCENT_GREEN),
                                                    crate::app_state::QueueItemStatus::Failed => ("失敗", colors::ACCENT_RED),
                                                }
                                            };
                                            let progress = if exporting && rendering_id.as_deref() == Some(composition_id.as_str()) {
                                                format!("{:.0}%", export_progress * 100.0)
                                            } else {
                                                status_text.to_string()
                                            };
                                            ui.label(egui::RichText::new(progress).color(status_color));
                                        },
                                    );
                                });
                            }
                        });
                }
            });
    });

    if clear_queue && !app.export.is_exporting {
        app.render_queue_items.clear();
        app.render_item_status.clear();
    } else if let Some(index) = remove_index {
        if index < app.render_queue_items.len() && !app.export.is_exporting {
            let removed = app.render_queue_items.remove(index);
            app.render_item_status.remove(&removed);
        }
    } else if add_active {
        queue_active_composition(app);
    }
    if let Some(composition_id) = activate_id {
        let mut project = app.history.current().clone();
        if let Some(index) = project
            .compositions
            .iter()
            .position(|composition| composition.id == composition_id)
        {
            project.active_composition_idx = index;
            app.commit_project(project);
            app.selection.selected_layer_idx = None;
            app.selection.selected_layers.clear();
        }
    }
}

pub(in crate::ui::home_screen) fn queue_active_composition(app: &mut KagariApp) -> bool {
    let comp_id = app.history.current().active_composition().id.clone();
    if app.render_queue_items.contains(&comp_id) {
        return false;
    }
    app.render_queue_items.push(comp_id.clone());
    app.render_item_status
        .insert(comp_id, crate::app_state::QueueItemStatus::Queued);
    true
}
