use super::*;

pub(in crate::ui::home_screen) fn draw_reference_new_project_page(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let compact = ui.available_height() < 900.0;
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, crate::ui::theme::colors::BG_PANEL_DEEP);
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(reference_content_margin(ui), 0.0))
        .show(ui, |ui| {
            draw_reference_topbar(app, ui, ctx);
            ui.add_space(if reference_narrow(ui) {
                8.0
            } else if compact {
                22.0
            } else {
                42.0
            });
            let narrow = reference_narrow(ui);
            let medium = reference_medium(ui);
            let stacked = (!narrow && ui.available_width() < 720.0)
                || (narrow && ui.available_width() < 300.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BG_PANEL_ALT)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        crate::ui::theme::colors::BORDER_SUBTLE,
                    ))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::same(if narrow { 6.0 } else { 10.0 }))
                    .show(ui, |ui| {
                        ui.set_width(if narrow {
                            84.0
                        } else if medium {
                            180.0
                        } else if compact {
                            170.0
                        } else {
                            220.0
                        });
                        let menu_id = egui::Id::new("reference_new_project_menu");
                        let mut menu = ctx.data_mut(|d| d.get_temp::<usize>(menu_id)).unwrap_or(0);
                        for (index, (icon, label)) in [
                            (crate::ui::icons::SVG_FILE, "New Project"),
                            (crate::ui::icons::SVG_OPEN_FOLDER, "Open Project"),
                            (crate::ui::icons::SVG_LAYERS, "Templates"),
                            (crate::ui::icons::SVG_HELP, "Learning"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let active = menu == index;
                            let row_width = ui.available_width();
                            let row_height = if narrow { 20.0 } else { 30.0 };
                            let clicked = ui
                                .allocate_ui_with_layout(
                                    egui::vec2(row_width, row_height),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        egui::Frame::none()
                                            .fill(if active {
                                                crate::ui::theme::colors::BG_ACTIVE
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            })
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.set_width(row_width - 8.0);
                                                ui.horizontal(|ui| {
                                                    let icon_size =
                                                        if narrow { 13.0 } else { 16.0 };
                                                    crate::ui::icons::render_svg_bytes(
                                                        ui,
                                                        label,
                                                        icon,
                                                        egui::vec2(icon_size, icon_size),
                                                        if active {
                                                            colors::TEXT_PRIMARY
                                                        } else {
                                                            colors::TEXT_SECONDARY
                                                        },
                                                    );
                                                    ui.add_space(5.0);
                                                    ui.label(
                                                        egui::RichText::new(
                                                            crate::ui::localization::tr(label),
                                                        )
                                                        .size(if narrow { 9.0 } else { 12.0 })
                                                        .color(if active {
                                                            colors::TEXT_PRIMARY
                                                        } else {
                                                            colors::TEXT_SECONDARY
                                                        }),
                                                    );
                                                });
                                            })
                                            .response
                                            .interact(egui::Sense::click())
                                    },
                                )
                                .inner
                                .clicked();
                            if clicked {
                                menu = index;
                                match index {
                                    1 => enter_studio_open_dialog(app),
                                    2 => set_home_nav(ctx, HomeNav::Templates),
                                    3 => app.show_shortcuts_dialog = true,
                                    _ => {}
                                }
                            }
                        }
                        ctx.data_mut(|d| d.insert_temp(menu_id, menu));
                    });
                if stacked {
                    ui.end_row();
                    ui.add_space(9.0);
                } else {
                    ui.add_space(if narrow { 4.0 } else { 14.0 });
                }
                let form_margin = if narrow { 8.0 } else { 31.0 };
                let panel_padding = if narrow { 8.0 } else { 16.0 };
                let remaining_width = if stacked {
                    ui.available_width()
                } else {
                    (ui.max_rect().right() - ui.cursor().left()).max(0.0)
                };
                let form_content_width =
                    (remaining_width - form_margin - panel_padding * 2.0).clamp(176.0, 760.0);
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BG_PANEL_ALT)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        crate::ui::theme::colors::BORDER_SUBTLE,
                    ))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::same(if narrow { 8.0 } else { 16.0 }))
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(form_content_width);
                            ui.label(
                                egui::RichText::new(crate::ui::localization::tr(
                                    "Create a New Project",
                                ))
                                .size(if narrow { 11.0 } else { 17.0 })
                                .strong(),
                            );
                            ui.add_space(if narrow { 5.0 } else { 10.0 });
                            ui.separator();
                            let name_id = egui::Id::new("reference_new_project_name");
                            let location_id = egui::Id::new("reference_new_project_location");
                            let mut name = ctx
                                .data_mut(|d| d.get_temp::<String>(name_id))
                                .unwrap_or_else(|| crate::ui::localization::tr("Untitled Project"));
                            let mut location = ctx
                                .data_mut(|d| d.get_temp::<String>(location_id))
                                .unwrap_or_else(|| "~/Projects".to_string());
                            ui.add_space(if narrow { 4.0 } else { 8.0 });
                            ui.label(
                                egui::RichText::new(crate::ui::localization::tr("Project Name"))
                                    .size(if narrow { 8.0 } else { 11.0 })
                                    .color(colors::TEXT_SECONDARY),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut name)
                                    .desired_width(ui.available_width()),
                            );
                            ui.add_space(if narrow { 4.0 } else { 8.0 });
                            ui.label(
                                egui::RichText::new(crate::ui::localization::tr("Location"))
                                    .size(if narrow { 8.0 } else { 11.0 })
                                    .color(colors::TEXT_SECONDARY),
                            );
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut location).desired_width(
                                    ui.available_width() - if narrow { 24.0 } else { 30.0 },
                                ));
                                reference_icon_button(
                                    ui,
                                    "new-project-location",
                                    crate::ui::icons::SVG_FOLDER,
                                    if narrow { 18.0 } else { 24.0 },
                                );
                            });
                            ui.add_space(if narrow { 4.0 } else { 8.0 });
                            ui.vertical(|ui| {
                                for (label, value) in [
                                    ("Frame Rate", "24 fps"),
                                    ("Resolution", "3440 × 2160 (4K UHD)"),
                                    ("Color Space", "Rec.709"),
                                ] {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(crate::ui::localization::tr(label))
                                                .size(if narrow { 8.0 } else { 11.0 })
                                                .color(colors::TEXT_SECONDARY),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                reference_select_button(
                                                    ui,
                                                    value,
                                                    if narrow {
                                                        ui.available_width().min(154.0)
                                                    } else {
                                                        210.0
                                                    },
                                                    narrow,
                                                );
                                            },
                                        );
                                    });
                                    ui.add_space(if narrow { 2.0 } else { 5.0 });
                                }
                            });
                            ui.collapsing(crate::ui::localization::tr("Advanced Settings"), |ui| {
                                ui.label(
                                    egui::RichText::new(crate::ui::localization::tr(
                                        "Project defaults and cache options",
                                    ))
                                    .small()
                                    .color(colors::TEXT_MUTED),
                                );
                            });
                            ui.add_space(if narrow { 6.0 } else { 12.0 });
                            ui.horizontal(|ui| {
                                if crate::ui::kui::button_min_size(
                                    ui,
                                    "Cancel",
                                    crate::ui::kui::ButtonKind::Default,
                                    egui::vec2(
                                        if narrow { 42.0 } else { 76.0 },
                                        if narrow { 22.0 } else { 28.0 },
                                    ),
                                )
                                .clicked()
                                {
                                    set_home_nav(ui.ctx(), HomeNav::Home);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if crate::ui::kui::button_min_size(
                                            ui,
                                            "Create Project",
                                            crate::ui::kui::ButtonKind::Accent,
                                            egui::vec2(
                                                if narrow { 68.0 } else { 112.0 },
                                                if narrow { 22.0 } else { 28.0 },
                                            ),
                                        )
                                        .clicked()
                                        {
                                            enter_studio_new_project(app);
                                        }
                                    },
                                );
                            });
                            ctx.data_mut(|d| {
                                d.insert_temp(name_id, name);
                                d.insert_temp(location_id, location);
                            });
                        });
                    });
            });
        });
}

pub(in crate::ui::home_screen) fn draw_reference_project_browser_page(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let compact = ui.available_height() < 900.0;
    let narrow = reference_narrow(ui);
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, crate::ui::theme::colors::BG_PANEL_DEEP);
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(
            if narrow {
                8.0
            } else {
                reference_content_margin(ui)
            },
            0.0,
        ))
        .show(ui, |ui| {
            draw_reference_topbar(app, ui, ctx);
            ui.add_space(if narrow {
                8.0
            } else if compact {
                22.0
            } else {
                42.0
            });
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("プロジェクト")
                        .size(if narrow { 22.0 } else { 30.0 })
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("＋ 新規プロジェクト").size(if narrow {
                                    9.0
                                } else {
                                    12.0
                                }),
                            )
                            .fill(colors::ACCENT_ORANGE)
                            .rounding(5.0),
                        )
                        .clicked()
                    {
                        enter_studio_new_project(app);
                        app.show_new_comp_dialog = true;
                    }
                    ui.add_space(8.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("▱ プロジェクトを開く").size(if narrow {
                                    9.0
                                } else {
                                    12.0
                                }),
                            )
                            .rounding(5.0),
                        )
                        .clicked()
                    {
                        enter_studio_open_dialog(app);
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("保存済みのプロジェクトを管理し、前回の作業を続けます。")
                    .size(if narrow { 9.0 } else { 13.0 })
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(if compact { 12.0 } else { 18.0 });
            reference_search_field(
                ui,
                &mut app.home_search,
                "プロジェクトを検索...",
                ui.available_width().min(480.0),
                if narrow { 26.0 } else { 34.0 },
            );
            ui.add_space(if compact { 12.0 } else { 22.0 });
            let query = app.home_search.trim().to_ascii_lowercase();
            let projects = recent_project_entries(app, ctx)
                .into_iter()
                .filter(|project| {
                    query.is_empty() || project.name.to_ascii_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("最近のプロジェクト  ({})", projects.len()))
                        .size(if narrow { 13.0 } else { 18.0 })
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    reference_select_button(
                        ui,
                        "更新日時",
                        if narrow { 76.0 } else { 108.0 },
                        narrow,
                    );
                    ui.add_space(6.0);
                    reference_select_button(ui, "すべて", if narrow { 58.0 } else { 82.0 }, narrow);
                });
            });
            ui.add_space(10.0);
            if projects.is_empty() {
                // Keep the call-to-action inside the empty-state card at compact
                // window heights; the icon, two labels, and button need more
                // than the old 190pt card allowed.
                let empty_height = if compact { 240.0 } else { 280.0 };
                fixed_card_with_inset(
                    ui,
                    egui::vec2(ui.available_width(), empty_height),
                    egui::Color32::TRANSPARENT,
                    crate::ui::theme::colors::BORDER_MEDIUM,
                    7.0,
                    18.0,
                    |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(if compact { 28.0 } else { 52.0 });
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                "projects-empty-folder",
                                crate::ui::icons::SVG_FOLDER,
                                egui::vec2(44.0, 44.0),
                                colors::TEXT_SECONDARY,
                            );
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(if query.is_empty() {
                                    "まだプロジェクトはありません"
                                } else {
                                    "一致するプロジェクトはありません"
                                })
                                .size(if narrow { 14.0 } else { 19.0 })
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(if query.is_empty() {
                                    "新規プロジェクトを作成して、はじめましょう。"
                                } else {
                                    "検索条件を変更してください。"
                                })
                                .size(if narrow { 10.0 } else { 13.0 })
                                .color(colors::TEXT_SECONDARY),
                            );
                            if query.is_empty() {
                                ui.add_space(14.0);
                                if reference_cta_button(ui, "新規プロジェクト", compact).clicked()
                                {
                                    enter_studio_new_project(app);
                                    app.show_new_comp_dialog = true;
                                }
                            }
                        });
                    },
                );
            } else {
                let columns = if narrow {
                    1.0
                } else if ui.available_width() < 820.0 {
                    2.0
                } else {
                    3.0
                };
                let gap = if compact { 10.0 } else { 16.0 };
                let card_width =
                    ((ui.available_width() - gap * (columns - 1.0)) / columns).max(170.0);
                ui.horizontal_wrapped(|ui| {
                    // Keep intentional vertical breathing room when the cards wrap
                    // into additional rows; horizontal spacing is placed below.
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, gap);
                    for (index, project) in projects.iter().enumerate() {
                        let card = fixed_card_with_inset(
                            ui,
                            egui::vec2(card_width, if compact { 154.0 } else { 204.0 }),
                            crate::ui::theme::colors::BG_PANEL_RAISED,
                            crate::ui::theme::colors::BG_ELEVATED,
                            6.0,
                            0.0,
                            |ui| {
                                if let Some(id) = reference_texture(
                                    app,
                                    ctx,
                                    [
                                        "recent_project_eclipse.webp",
                                        "recent_project_citadel.webp",
                                        "recent_project_rift.webp",
                                        "recent_project_atlas.webp",
                                    ][index % 4],
                                ) {
                                    reference_cover_image(
                                        ui,
                                        id,
                                        egui::vec2(card_width, if compact { 72.0 } else { 112.0 }),
                                        2.64,
                                    );
                                }
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(&project.name)
                                            .size(if compact { 13.0 } else { 16.0 })
                                            .strong(),
                                    );
                                });
                                let comp_name = project
                                    .summary
                                    .as_ref()
                                    .and_then(|summary| summary.comp_names.get(summary.active_idx))
                                    .map(String::as_str)
                                    .unwrap_or("Project");
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(comp_name)
                                            .size(11.0)
                                            .color(colors::TEXT_SECONDARY),
                                    );
                                });
                                let age = project
                                    .modified
                                    .map(fmt_age)
                                    .unwrap_or_else(|| "Not found".to_string());
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(age)
                                            .size(11.0)
                                            .color(colors::TEXT_MUTED),
                                    );
                                });
                            },
                        );
                        if card.clicked() {
                            open_project_path(app, &project.path);
                        }
                        if index % columns as usize != columns as usize - 1 {
                            ui.add_space(gap);
                        }
                    }
                });
            }
        });
}
