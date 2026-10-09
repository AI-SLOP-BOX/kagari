use crate::KagariApp;
use eframe::egui;
use super::commands::{get_all_commands, PaletteCommand};

pub fn draw_command_palette(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    let mut open = app.show_command_palette;
    crate::ui::modal::dialog(format!(
        "Command Palette ({})",
        crate::ui::shortcuts::format_shortcut("K", true, false, false)
    ))
    .open(&mut open)
    .resizable(false)
    .collapsible(false)
    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 100.0))
    .fixed_size(egui::vec2(520.0, 340.0))
    .show(ctx, |ui| {
        let search_response = crate::ui::kui::search_field(
            ui,
            &mut app.command_palette_search,
            "Type a command or effect... (e.g. Blur, Text, Export)",
        );
        search_response.request_focus();
        crate::ui::kui::separator(ui);

        let query = app.command_palette_search.to_lowercase();
        let commands = get_all_commands();

        let filtered: Vec<&PaletteCommand> = commands
            .iter()
            .filter(|cmd| {
                query.is_empty()
                    || cmd.name.to_lowercase().contains(&query)
                    || cmd.category.to_lowercase().contains(&query)
            })
            .collect();

        let mut executed_command_idx: Option<usize> = None;

        // Handle Keyboard Navigation (Up / Down / Enter / Esc)
        let input = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if input.3 {
            app.show_command_palette = false;
            return;
        }

        if !filtered.is_empty() {
            if input.0 {
                app.command_palette_selected_idx =
                    (app.command_palette_selected_idx + 1) % filtered.len();
            }
            if input.1 {
                app.command_palette_selected_idx = if app.command_palette_selected_idx == 0 {
                    filtered.len() - 1
                } else {
                    app.command_palette_selected_idx - 1
                };
            }
            if input.2 && app.command_palette_selected_idx < filtered.len() {
                executed_command_idx = Some(app.command_palette_selected_idx);
            }
        }

        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                if filtered.is_empty() {
                    crate::ui::kui::empty_state(ui, "No matching commands found", 72.0);
                } else {
                    for (idx, cmd) in filtered.iter().enumerate() {
                        let is_selected = idx == app.command_palette_selected_idx;
                        let row =
                            crate::ui::kui::selectable_item_row(ui, is_selected, 32.0, |ui| {
                                // Category Pill Badge
                                let (badge_bg, badge_fg) = match cmd.category {
                                    "Effects" => (egui::Color32::from_rgb(45, 25, 55), egui::Color32::from_rgb(220, 150, 255)),
                                    "Layer" | "Layer Styles" => (egui::Color32::from_rgb(20, 45, 60), egui::Color32::from_rgb(100, 200, 255)),
                                    "Animation" => (egui::Color32::from_rgb(50, 40, 20), egui::Color32::from_rgb(255, 200, 100)),
                                    "Export" | "File" => (egui::Color32::from_rgb(25, 45, 30), egui::Color32::from_rgb(120, 230, 150)),
                                    "Tools" => (egui::Color32::from_rgb(45, 35, 50), egui::Color32::from_rgb(210, 170, 240)),
                                    _ => (egui::Color32::from_rgb(32, 36, 44), egui::Color32::from_rgb(170, 180, 195)),
                                };

                                let font_id = egui::FontId::proportional(10.0);
                                let galley = ui.painter().layout_no_wrap(cmd.category.to_string(), font_id, badge_fg);
                                let badge_size = galley.size() + egui::vec2(12.0, 6.0);
                                let (badge_rect, _) = ui.allocate_exact_size(badge_size, egui::Sense::hover());
                                ui.painter().rect_filled(badge_rect, 4.0, badge_bg);
                                ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0f32, badge_fg.gamma_multiply(0.25f32)));
                                ui.painter().galley(badge_rect.min + egui::vec2(6.0, 3.0), galley, badge_fg);

                                ui.add_space(4.0);
                                crate::ui::kui::item_title(ui, cmd.name);

                                if !cmd.shortcut_hint.is_empty() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let hint =
                                                crate::ui::shortcuts::format_platform_shortcut_hint(
                                                    cmd.shortcut_hint,
                                                );
                                            crate::ui::kui::shortcut_hint(ui, &hint);
                                        },
                                    );
                                }
                            });
                        if row.response.clicked() {
                            executed_command_idx = Some(idx);
                        }
                    }
                }
            });

        if let Some(exec_idx) = executed_command_idx {
            if exec_idx < filtered.len() {
                let command_name = filtered[exec_idx].name;
                let generation_before = app.history.generation();
                let snapshot = app.history.current().clone();
                (filtered[exec_idx].action)(app);

                // Some commands use `modify_project`, while older commands
                // mutate the live history entry for speed. Normalize both
                // paths into one undoable edit without adding a duplicate
                // entry for commands that already committed.
                if app.history.generation() == generation_before {
                    let current = app.history.current().clone();
                    let changed =
                        match (serde_json::to_vec(&snapshot), serde_json::to_vec(&current)) {
                            (Ok(before), Ok(after)) => before != after,
                            _ => false,
                        };
                    if changed {
                        app.begin_drag_with_snapshot(snapshot, command_name);
                        app.commit_drag();
                    }
                }
                app.show_command_palette = false;
                app.command_palette_search.clear();
                app.command_palette_selected_idx = 0;
                app.toasts
                    .info(format!("Executed: {}", filtered[exec_idx].name));
            }
        }
    });

    if !open {
        app.show_command_palette = false;
    }
}
