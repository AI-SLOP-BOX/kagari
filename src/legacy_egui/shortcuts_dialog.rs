use crate::KagariApp;
use eframe::egui;

const CATEGORIES: &[&str] = &["File", "Edit", "Timeline", "Layers", "Viewer", "Panels"];

fn category_for(id: crate::ui::shortcuts::ShortcutId) -> &'static str {
    use crate::ui::shortcuts::ShortcutId::*;
    match id {
        Save | NewComposition | OpenRenderQueue => "File",
        CommandPalette | Undo | Redo => "Edit",
        HandTool | ZoomTool => "Viewer",
        SelectionTool | RotationTool | AnchorTool | RectangleTool | PenTool | CameraTool
        | TextTool | PuppetPinTool => "Panels",
        TogglePlayback | PreviousKeyframe | NextKeyframe | PlayForward | WorkAreaStart
        | WorkAreaEnd => "Timeline",
        EasyEase | DuplicateLayer | LayerInPoint | LayerOutPoint | RevealOpacity
        | RevealPosition | RevealScale | RevealRotation | RevealAnchorPoint => "Layers",
    }
}

fn valid_shortcut_map(
    bindings: &std::collections::HashMap<String, crate::ui::shortcuts::defs::ShortcutBinding>,
) -> bool {
    let registered = crate::ui::shortcuts::shortcut_registry();
    if !bindings.iter().all(|(id, binding)| {
        registered.iter().any(|shortcut| shortcut.id.key() == id)
            && (binding.key.is_empty() || crate::ui::shortcuts::defs::supports_key(&binding.key))
    }) {
        return false;
    }
    let mut assigned = std::collections::HashSet::new();
    registered.iter().all(|shortcut| {
        let binding = bindings
            .get(shortcut.id.key())
            .cloned()
            .unwrap_or_else(|| crate::ui::shortcuts::defs::default_binding(shortcut.id));
        binding.key.is_empty()
            || assigned.insert((binding.key, binding.command, binding.shift, binding.alt))
    })
}

fn draw_shortcut_row(ui: &mut egui::Ui, name: &str, binding: &str, index: usize, edit: bool) -> u8 {
    let fill = if index % 2 == 0 {
        crate::ui::theme::colors::BG_PANEL_DEEP
    } else {
        crate::ui::theme::colors::BG_DARKEST
    };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0_f32,
            crate::ui::theme::colors::BORDER_SUBTLE,
        ))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            let mut action = 0;
            ui.horizontal(|ui| {
                ui.label(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if edit {
                        if ui
                            .small_button(crate::ui::localization::tr("Edit"))
                            .on_hover_text(crate::ui::localization::tr("Edit shortcut"))
                            .clicked()
                        {
                            action = 1;
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Clear shortcut")
                            .clicked()
                        {
                            action = 2;
                        }
                    }
                    if binding == crate::ui::localization::tr("Unassigned") {
                        ui.label(
                            egui::RichText::new(binding)
                                .small()
                                .color(crate::ui::theme::colors::TEXT_MUTED),
                        );
                    } else {
                        for key in binding.split('+').rev() {
                            egui::Frame::none()
                                .fill(crate::ui::theme::colors::BG_MID)
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    crate::ui::theme::colors::BORDER_MEDIUM,
                                ))
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(7.0, 3.0))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(key.trim()).monospace().small());
                                });
                            if key != binding.split('+').next().unwrap_or_default() {
                                ui.label(
                                    egui::RichText::new("+")
                                        .small()
                                        .color(crate::ui::theme::colors::TEXT_MUTED),
                                );
                            }
                        }
                    }
                });
            });
            action
        })
        .inner
}

pub fn draw_shortcuts_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    let mut show = app.show_shortcuts_dialog;
    if !show {
        return;
    }

    let query_id = egui::Id::new("shortcut_dialog_query");
    let category_id = egui::Id::new("shortcut_dialog_category");
    let mut query = ctx.data_mut(|data| data.get_temp::<String>(query_id).unwrap_or_default());
    let mut category_index = ctx
        .data_mut(|data| data.get_temp::<usize>(category_id))
        .unwrap_or(0)
        .min(CATEGORIES.len() - 1);
    let mut close_requested = false;
    let mut prefs = crate::ui::preferences_dialog::load();
    let capture_id = egui::Id::new("shortcut_dialog_capture");
    let mut capture = ctx.data_mut(|d| d.get_temp::<String>(capture_id));
    let mut changed = false;

    crate::ui::modal::dialog(crate::ui::localization::tr("Keyboard Shortcuts"))
        .open(&mut show)
        .title_bar(false)
        .resizable(true)
        .default_size(egui::vec2(900.0, 520.0))
        .min_size(egui::vec2(660.0, 420.0))
        .max_size(egui::vec2(1100.0, 540.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::proportional(15.0),
            );
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::proportional(13.0),
            );
            ui.horizontal(|ui| {
                crate::ui::icons::render_svg_bytes(
                    ui,
                    "keyboard-shortcuts-header",
                    crate::ui::icons::SVG_KEYBOARD,
                    egui::vec2(24.0, 24.0),
                    crate::ui::theme::colors::ACCENT_BRAND,
                );
                ui.vertical(|ui| {
                    ui.heading(crate::ui::localization::tr("Keyboard Shortcuts"));
                    ui.label(egui::RichText::new(crate::ui::localization::tr("Search the commands available in Kagari VFX"))
                        .small().color(crate::ui::theme::colors::TEXT_SECONDARY));
                });
            });
            ui.add_space(12.0);

            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DARKEST)
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_MEDIUM))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        crate::ui::icons::render_svg_bytes(
                            ui,
                            "shortcut-search",
                            crate::ui::icons::SVG_SEARCH,
                            egui::vec2(18.0, 18.0),
                            crate::ui::theme::colors::TEXT_MUTED,
                        );
                        ui.add(egui::TextEdit::singleline(&mut query)
                            .hint_text(crate::ui::localization::tr("Search commands or shortcuts..."))
                            .desired_width(f32::INFINITY)
                            .frame(false));
                        if !query.is_empty() && ui.small_button("×").clicked() {
                            query.clear();
                        }
                    });
                });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let panel_height = ui.available_height();
                ui.allocate_ui_with_layout(
                    egui::vec2(156.0, panel_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |navigation| {
                for (index, name) in CATEGORIES.iter().enumerate() {
                    let label = crate::ui::localization::tr(name);
                    let response = crate::ui::kui::with_button_style(
                        navigation,
                        crate::ui::kui::ButtonKind::Quiet,
                        |ui| ui.add_sized(
                            [ui.available_width(), 38.0],
                            egui::Button::new(egui::RichText::new(label).size(15.0))
                                .selected(category_index == index),
                        ),
                    );
                    if response.clicked() { category_index = index; }
                    navigation.add_space(4.0);
                }
                    },
                );
                let content_width = ui.available_width().max(220.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(content_width, panel_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |content| {
                egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_DARKEST)
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(8.0))
                .show(content, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(crate::ui::localization::tr("Command")).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(crate::ui::localization::tr("Shortcut")).strong());
                        });
                    });
                    ui.add_space(4.0);
                    crate::ui::kui::separator(ui);

                    let query_lower = query.trim().to_lowercase();
                    let active_category = CATEGORIES[category_index];
                    let mut visible_count = 0;
                    egui::ScrollArea::vertical()
                        .id_salt("shortcut_dialog_rows")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for shortcut in crate::ui::shortcuts::shortcut_registry() {
                                let id_key = shortcut.id.key().to_owned();
                                let binding = crate::ui::shortcuts::display_shortcut(shortcut.id);
                                let name = crate::ui::localization::tr(shortcut.description);
                                if category_for(shortcut.id) != active_category {
                                    continue;
                                }
                                let searchable = format!("{name} {binding}").to_lowercase();
                                if !query_lower.is_empty() && !searchable.contains(&query_lower) {
                                    continue;
                                }
                                match draw_shortcut_row(ui, &name, &binding, visible_count, true) {
                                    1 => capture = Some(id_key.clone()),
                                    2 => {
                                        prefs.shortcut_bindings.insert(id_key, crate::ui::shortcuts::defs::ShortcutBinding {
                                            key: String::new(), command: false, shift: false, alt: false,
                                        });
                                        crate::ui::shortcuts::defs::set_overrides(&prefs.shortcut_bindings);
                                        changed = true;
                                    },
                                    _ => {}
                                }
                                visible_count += 1;
                                ui.add_space(3.0);
                            }

                            if active_category == "Timeline" {
                                for (binding, description) in crate::ui::shortcuts::ADDITIONAL_SHORTCUTS {
                                    let binding = crate::ui::shortcuts::format_platform_shortcut_hint(binding);
                                    let name = crate::ui::localization::tr(description);
                                    let searchable = format!("{name} {binding}").to_lowercase();
                                    if !query_lower.is_empty() && !searchable.contains(&query_lower) {
                                        continue;
                                    }
                                    draw_shortcut_row(ui, &name, &binding, visible_count, false);
                                    visible_count += 1;
                                    ui.add_space(3.0);
                                }
                            }

                            if active_category == "Layers" {
                                for (binding, description) in crate::ui::shortcuts::SHORTCUT_GESTURES {
                                    let name = crate::ui::localization::tr(description);
                                    let searchable = format!("{name} {binding}").to_lowercase();
                                    if !query_lower.is_empty() && !searchable.contains(&query_lower) {
                                        continue;
                                    }
                                    draw_shortcut_row(ui, &name, binding, visible_count, false);
                                    visible_count += 1;
                                    ui.add_space(3.0);
                                }
                            }

                            if visible_count == 0 {
                                ui.add_space(18.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new(crate::ui::localization::tr("No matching shortcuts"))
                                        .color(crate::ui::theme::colors::TEXT_SECONDARY));
                                });
                            }
                        });
                });
                    },
                );
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(crate::ui::localization::tr(capture.as_ref().map(|_| "Press a key combination…  Esc cancels · Backspace clears").unwrap_or("Changes are saved automatically.")))
                    .small().color(crate::ui::theme::colors::TEXT_MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::kui::button(ui, &crate::ui::localization::tr("Reset"), crate::ui::kui::ButtonKind::Default).clicked() {
                        prefs.shortcut_bindings.clear();
                        crate::ui::shortcuts::defs::set_overrides(&prefs.shortcut_bindings);
                        changed = true;
                    }
                    if crate::ui::kui::button(ui, &crate::ui::localization::tr("Export…"), crate::ui::kui::ButtonKind::Default).clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).set_file_name("kagari-shortcuts.json").save_file() {
                            if let Ok(json) = serde_json::to_string_pretty(&prefs.shortcut_bindings) { let _ = std::fs::write(path, json); }
                        }
                    }
                    if crate::ui::kui::button(ui, &crate::ui::localization::tr("Import…"), crate::ui::kui::ButtonKind::Default).clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                            if let Ok(text) = std::fs::read_to_string(path) {
                                match serde_json::from_str::<std::collections::HashMap<String, crate::ui::shortcuts::defs::ShortcutBinding>>(&text) {
                                    Ok(map) if valid_shortcut_map(&map) => {
                                        prefs.shortcut_bindings = map;
                                        crate::ui::shortcuts::defs::set_overrides(&prefs.shortcut_bindings);
                                        changed = true;
                                    }
                                    _ => app.toasts.error("Invalid shortcut file: unsupported command or key"),
                                }
                            }
                        }
                    }
                    if crate::ui::kui::button(ui, &crate::ui::localization::tr("Apply"), crate::ui::kui::ButtonKind::Accent).clicked() {
                        match crate::ui::preferences_dialog::save(&prefs) {
                            Ok(()) => close_requested = true,
                            Err(error) => app.toasts.error(error),
                        }
                    }
                    if crate::ui::kui::button(ui, &crate::ui::localization::tr("Close"), crate::ui::kui::ButtonKind::Default).clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    ctx.data_mut(|data| {
        data.insert_temp(query_id, query);
        data.insert_temp(category_id, category_index);
    });
    if let Some(id) = capture.clone() {
        let events = ctx.input(|input| input.raw.events.clone());
        for event in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            {
                if key == egui::Key::Escape {
                    capture = None;
                    break;
                }
                if key == egui::Key::Backspace || key == egui::Key::Delete {
                    prefs.shortcut_bindings.insert(
                        id.clone(),
                        crate::ui::shortcuts::defs::ShortcutBinding {
                            key: String::new(),
                            command: false,
                            shift: false,
                            alt: false,
                        },
                    );
                } else {
                    let candidate = crate::ui::shortcuts::defs::ShortcutBinding {
                        key: format!("{key:?}"),
                        command: modifiers.command || modifiers.ctrl,
                        shift: modifiers.shift,
                        alt: modifiers.alt,
                    };
                    if !crate::ui::shortcuts::defs::supports_key(&candidate.key) {
                        app.toasts
                            .warning("This key is not supported for shortcut assignments");
                        continue;
                    }
                    let conflict = crate::ui::shortcuts::shortcut_registry()
                        .iter()
                        .find(|other| {
                            other.id.key() != id
                                && crate::ui::shortcuts::defs::current_binding(other.id)
                                    == candidate
                        });
                    if let Some(conflict) = conflict {
                        app.toasts.warning(format!(
                            "That shortcut is already assigned to {}",
                            conflict.description
                        ));
                        continue;
                    }
                    prefs.shortcut_bindings.insert(id.clone(), candidate);
                }
                crate::ui::shortcuts::defs::set_overrides(&prefs.shortcut_bindings);
                changed = true;
                capture = None;
                break;
            }
        }
    }
    ctx.data_mut(|d| {
        if let Some(id) = capture {
            d.insert_temp(capture_id, id)
        } else {
            d.remove_temp::<String>(capture_id);
        }
    });
    if changed {
        if let Err(error) = crate::ui::preferences_dialog::save(&prefs) {
            app.toasts.error(error);
        }
    }
    app.show_shortcuts_dialog = show && !close_requested;
}

pub fn draw_shortcuts_reference_dialog(ctx: &egui::Context, open: &mut bool) {
    crate::ui::modal::window("Keyboard Shortcuts Reference")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Kagari VFX — Shortcuts Reference");
            ui.separator();
            crate::ui::shortcuts::draw_shortcuts_reference(ui);
        });
}
