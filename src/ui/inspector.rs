use crate::core::property::Animatable;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, _current_frame: &mut u32) {
    // ── Non-blocking Async Motion Tracker Event Receiver ──
    let mut tracker_completed = false;
    if let Some(ref rx) = app.export.tracker_rx {
        while let Ok(event) = rx.try_recv() {
            if let crate::TrackerEvent::Finished {
                ref layer_id,
                layer_idx,
                tracker_idx,
                keyframes,
            } = event
            {
                let keyframes = keyframes.clone();
                let mut completed_layer_name = None;
                app.modify_project(|project| {
                    let comp = project.active_composition_mut();
                    let layer_opt = comp.layers.iter().position(|l| l.id == *layer_id).or(
                        if layer_idx < comp.layers.len() {
                            Some(layer_idx)
                        } else {
                            None
                        },
                    );

                    if let Some(idx) = layer_opt {
                        let layer = &mut comp.layers[idx];
                        if tracker_idx < layer.trackers.len() {
                            layer.trackers[tracker_idx].position = Animatable::Animated(keyframes);
                            completed_layer_name = Some(layer.name.clone());
                        }
                    }
                });
                if let Some(layer_name) = completed_layer_name {
                    log::info!(
                        "Async Motion Tracker analysis completed for layer {} ({}), tracker {}",
                        layer_name,
                        layer_id,
                        tracker_idx
                    );
                }
                tracker_completed = true;
                break;
            }
        }
    }
    if tracker_completed {
        app.export.tracker_rx = None;
    }

    // Update panel animation
    let dt = ctx.input(|i| i.stable_dt);
    app.inspector_animation.update(dt);

    let screen_width = ctx.screen_rect().width();
    let studio_layout = ctx.screen_rect().width() >= 1200.0;

    let compact_workspace =
        ctx.screen_rect().width() < crate::ui::theme::layout::DOCKED_PANELS_MIN_WIDTH;
    let compact_project_drawer = ctx
        .data(|data| data.get_temp::<bool>(egui::Id::new("compact_project_drawer")))
        .unwrap_or(false);
    if !compact_workspace || compact_project_drawer {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(if studio_layout {
                (screen_width * 0.23).clamp(290.0, 440.0)
            } else {
                (screen_width * 0.27).clamp(220.0, 330.0)
            })
            .min_width(if studio_layout {
                (screen_width * 0.17).clamp(250.0, 350.0)
            } else {
                (screen_width * 0.23).clamp(190.0, 250.0)
            })
            .max_width(if studio_layout {
                (screen_width * 0.29).clamp(360.0, 500.0)
            } else {
                (screen_width * 0.32).clamp(280.0, 390.0)
            })
            .frame(if studio_layout {
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BG_DARKEST)
                    .inner_margin(egui::Margin::symmetric(17.0, 0.0))
            } else {
                egui::Frame::default().inner_margin(egui::Margin::symmetric(8.0, 0.0))
            })
            .show(ctx, |ui| {
                if studio_layout {
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            ui.available_width(),
                            crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
                        ),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if crate::ui::theme::draw_custom_tab(
                                ui,
                                app.ui_tabs.left_tab_idx == 0,
                                &crate::ui::localization::tr("Assets"),
                            )
                            .clicked()
                            {
                                app.ui_tabs.left_tab_idx = 0;
                            }
                            if crate::ui::theme::draw_custom_tab(
                                ui,
                                app.ui_tabs.left_tab_idx == 1,
                                &crate::ui::localization::tr("Effects"),
                            )
                            .clicked()
                            {
                                app.ui_tabs.left_tab_idx = 1;
                            }
                            if crate::ui::theme::draw_custom_tab(
                                ui,
                                app.ui_tabs.left_tab_idx == 2,
                                &crate::ui::localization::tr("Tools"),
                            )
                            .clicked()
                            {
                                app.ui_tabs.left_tab_idx = 2;
                            }
                        },
                    );
                    crate::ui::theme::draw_separator(ui);
                    match app.ui_tabs.left_tab_idx {
                        0 => crate::ui::project_panel::draw(app, ui),
                        1 => draw_effect_browser(app, ui),
                        _ => crate::ui::toolbar::draw_tool_browser(app, ui),
                    }
                    return;
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        ui.available_width(),
                        crate::ui::theme::layout::PANEL_HEADER_HEIGHT,
                    ),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if crate::ui::theme::draw_custom_tab(
                            ui,
                            app.ui_tabs.left_tab_idx == 0,
                            &crate::ui::localization::tr("Assets"),
                        )
                        .clicked()
                        {
                            app.ui_tabs.left_tab_idx = 0;
                        }
                        if crate::ui::theme::draw_custom_tab(
                            ui,
                            app.ui_tabs.left_tab_idx == 1,
                            &crate::ui::localization::tr("Effects"),
                        )
                        .clicked()
                        {
                            app.ui_tabs.left_tab_idx = 1;
                        }
                        if crate::ui::theme::draw_custom_tab(
                            ui,
                            app.ui_tabs.left_tab_idx == 2,
                            &crate::ui::localization::tr("Tools"),
                        )
                        .clicked()
                        {
                            app.ui_tabs.left_tab_idx = 2;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                "left-panel-expand",
                                crate::ui::icons::SVG_WINDOW_MAXIMIZE,
                                egui::vec2(14.0, 14.0),
                                colors::TEXT_SECONDARY,
                            );
                        });
                    },
                );
                crate::ui::theme::draw_separator(ui);

                if app.ui_tabs.left_tab_idx == 0 {
                    crate::ui::project_panel::draw(app, ui);
                    return;
                }

                if app.ui_tabs.left_tab_idx == 1 {
                    draw_effect_browser(app, ui);
                    return;
                }

                crate::ui::toolbar::draw_tool_browser(app, ui);
            });
    }
}

fn draw_effect_browser(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.heading(egui::RichText::new("エフェクト").size(16.0).strong());
    ui.label(
        egui::RichText::new("Search, preview, and apply effects")
            .small()
            .color(colors::TEXT_MUTED),
    );
    ui.add_space(8.0);

    let search_id = ui.make_persistent_id("left_effect_search");
    let mut search = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with(search_id, String::new)
            .clone()
    });
    let response = ui.add_sized(
        [ui.available_width(), 28.0],
        egui::TextEdit::singleline(&mut search).hint_text("エフェクトを検索"),
    );
    if response.changed() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(search_id, search.clone()));
    }
    ui.add_space(10.0);

    let categories = [
        (crate::ui::icons::SVG_STAR, "お気に入り"),
        (crate::ui::icons::SVG_CLOCK, "最近使った項目"),
        (crate::ui::icons::SVG_PALETTE, "カラー"),
        (crate::ui::icons::SVG_TOOL_ZOOM, "Blur & Sharpen"),
        (crate::ui::icons::SVG_TOOL_SHAPE, "変形"),
        (crate::ui::icons::SVG_EFFECTS, "生成"),
        (crate::ui::icons::SVG_KEYING, "キーイング"),
        (crate::ui::icons::SVG_TOOL_PEN, "スタイライズ"),
        (crate::ui::icons::SVG_ARROW_RIGHT, "トランジション"),
        (crate::ui::icons::SVG_SETTINGS, "ユーティリティ"),
    ];
    egui::ScrollArea::vertical()
        .id_salt("left_effect_browser_scroll")
        .show(ui, |ui| {
            for (index, (icon, category)) in categories.iter().enumerate() {
                let active = index == 2;
                let row = ui.allocate_ui(egui::vec2(ui.available_width(), 28.0), |ui| {
                    ui.horizontal(|ui| {
                        let icon_id = format!("left-effect-category-{index}");
                        crate::ui::icons::render_svg_bytes(
                            ui,
                            &icon_id,
                            icon,
                            egui::vec2(15.0, 15.0),
                            if active {
                                colors::ACCENT_SELECTION
                            } else {
                                colors::TEXT_SECONDARY
                            },
                        );
                        let _ = ui.selectable_label(
                            active,
                            egui::RichText::new(*category).size(12.0).color(if active {
                                colors::TEXT_PRIMARY
                            } else {
                                colors::TEXT_SECONDARY
                            }),
                        );
                    });
                });
                if row.response.hovered() {
                    ui.painter().rect_filled(
                        row.response.rect,
                        3.0,
                        colors::BG_HOVER.linear_multiply(0.65),
                    );
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Available effects (Click to Apply / Drag to Layer)")
                    .size(12.0)
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );

            let query = search.trim().to_lowercase();
            let presets = crate::ui::effects_controls::presets::get_all_effect_presets();
            for (idx, preset) in presets
                .iter()
                .enumerate()
                .filter(|(_, p)| query.is_empty() || p.name.to_lowercase().contains(&query))
                .take(20)
            {
                let row_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), 28.0),
                );
                let resp = ui.interact(
                    row_rect,
                    egui::Id::new(("left_effect_item", idx)),
                    egui::Sense::click_and_drag(),
                );
                let hovered = resp.hovered();
                if hovered {
                    ui.painter()
                        .rect_filled(row_rect, 3.0, colors::BG_HOVER.linear_multiply(0.7));
                }

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("◇")
                                .color(if hovered {
                                    colors::ACCENT_BRAND
                                } else {
                                    colors::ACCENT_CYAN
                                })
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(preset.name)
                                .size(12.0)
                                .color(if hovered {
                                    colors::TEXT_PRIMARY
                                } else {
                                    colors::TEXT_SECONDARY
                                }),
                        );
                    });
                });

                if resp.drag_started() {
                    app.dragging_effect = Some((preset.name.to_string(), idx));
                }

                if resp.double_clicked()
                    || (resp.clicked() && !ui.input(|i| i.pointer.is_decidedly_dragging()))
                {
                    crate::ui::menu::apply_effect_by_name(app, preset.name);
                }
            }
        });
}
