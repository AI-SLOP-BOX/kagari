use super::*;

pub(in crate::ui::home_screen) fn draw_reference_home(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let bg = crate::ui::theme::colors::BG_PANEL_DEEP;
    let compact = ui.available_height() < 900.0;
    let top_space = if compact { 28.0 } else { 40.0 };
    let mobile = ui.available_width() < 520.0;
    let base_first_row_height = if mobile {
        360.0
    } else if compact {
        200.0
    } else {
        266.0
    };
    let preview_height = if compact { 130.0 } else { 191.0 };
    ui.painter().rect_filled(ui.max_rect(), 0.0, bg);
    let home_margin = if ui.available_width() >= 900.0 {
        egui::Margin {
            left: 35.0,
            right: 29.0,
            top: 0.0,
            bottom: 0.0,
        }
    } else {
        egui::Margin::symmetric(reference_content_margin(ui), 0.0)
    };
    egui::Frame::none()
        .inner_margin(home_margin)
        .show(ui, |ui| {
            ui.add_space(if compact { 12.0 } else { 24.0 });
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 46.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let profile_size = if mobile {
                        26.0
                    } else if compact {
                        30.0
                    } else {
                        34.0
                    };
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        "home-profile",
                        crate::ui::icons::SVG_PROFILE,
                        egui::vec2(profile_size, profile_size),
                        egui::Color32::WHITE,
                    );
                    ui.add_space(if compact { 12.0 } else { 35.0 });
                    let desired_search_width: f32 = if mobile {
                        220.0
                    } else if compact {
                        300.0
                    } else {
                        418.0
                    };
                    let search_width =
                        desired_search_width.min((ui.available_width() - 52.0).max(1.0));
                    ui.allocate_ui(egui::vec2(search_width, 46.0), |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(14, 23, 32))
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                egui::Color32::from_rgb(49, 67, 84),
                            ))
                            .rounding(7.0)
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let search_icon_size = if mobile { 15.0 } else { 18.0 };
                                    let search_font_size = if mobile { 11.0 } else { 14.0 };
                                    crate::ui::icons::render_svg_bytes(
                                        ui,
                                        "home-search",
                                        crate::ui::icons::SVG_SEARCH,
                                        egui::vec2(search_icon_size, search_icon_size),
                                        crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.home_search)
                                            .hint_text("Search projects, templates...")
                                            .font(egui::FontId::proportional(search_font_size))
                                            .frame(false)
                                            .desired_width(
                                                (search_width - if mobile { 48.0 } else { 54.0 })
                                                    .max(1.0),
                                            ),
                                    );
                                });
                            });
                    });
                },
            );
            ui.add_space(top_space);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(0.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Pick up where you left off.")
                                .size(if mobile { 20.0 } else { 28.0 })
                                .strong()
                                .color(egui::Color32::from_rgb(188, 201, 232)),
                        );
                    });
                    ui.add_space(10.0);
                    let row_width = ui.available_width();
                    let stacked = row_width < 920.0;
                    let first_row_height = if stacked && row_width < 560.0 {
                        if compact {
                            300.0
                        } else {
                            382.0
                        }
                    } else {
                        base_first_row_height
                    };
                    let first_row_gap = if compact { 12.0 } else { 18.0 };
                    let continue_width = if stacked {
                        row_width
                    } else {
                        (row_width * 0.571).min(row_width - first_row_gap - 330.0)
                    };
                    let start_width = if stacked {
                        row_width
                    } else {
                        (row_width - continue_width - first_row_gap).max(330.0)
                    };
                    let draw_home_first_row = |ui: &mut egui::Ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let width = continue_width;
                        fixed_card(
                            ui,
                            egui::vec2(width, first_row_height),
                            crate::ui::theme::colors::BG_PANEL_RAISED,
                            crate::ui::theme::colors::BG_ELEVATED,
                            8.0,
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Continue Working").size(20.0).strong(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            crate::ui::icons::render_svg_bytes(
                                                ui,
                                                "continue-more",
                                                crate::ui::icons::SVG_MORE,
                                                egui::vec2(18.0, 18.0),
                                                egui::Color32::from_rgb(170, 188, 216),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(if compact { 8.0 } else { 12.0 });
                                draw_reference_continue_body(
                                    app,
                                    ui,
                                    ctx,
                                    compact,
                                    width < 560.0,
                                    preview_height,
                                );
                            },
                        );
                        if stacked {
                            ui.end_row();
                            ui.add_space(10.0);
                        } else {
                            ui.add_space(first_row_gap);
                        }
                        let start_height = if stacked && row_width < 560.0 {
                            214.0
                        } else {
                            first_row_height
                        };
                        fixed_card(
                            ui,
                            egui::vec2(start_width, start_height),
                            crate::ui::theme::colors::BG_PANEL_RAISED,
                            crate::ui::theme::colors::BG_ELEVATED,
                            8.0,
                            |ui| {
                                ui.label(egui::RichText::new("Start New").size(18.0).strong());
                                ui.add_space(12.0);
                                let action_height: f32 = if compact { 60.0 } else { 92.0 };
                                if ui.available_width() < 260.0 {
                                    for (icon, title, subtitle, action) in [
                                        (
                                            crate::ui::icons::SVG_FILE,
                                            "New Project",
                                            "Start from scratch",
                                            0_u8,
                                        ),
                                        (
                                            crate::ui::icons::SVG_OPEN_FOLDER,
                                            "Open Project",
                                            "Browse files",
                                            1_u8,
                                        ),
                                        (
                                            crate::ui::icons::SVG_COMPOSITION,
                                            "New Composition",
                                            "Create a comp",
                                            2_u8,
                                        ),
                                        (
                                            crate::ui::icons::SVG_IMPORT,
                                            "Import Footage",
                                            "Add media",
                                            3_u8,
                                        ),
                                    ] {
                                        let clicked = reference_action_button(
                                            ui,
                                            icon,
                                            title,
                                            subtitle,
                                            action == 0,
                                            action_height.min(62.0),
                                        )
                                        .clicked();
                                        if clicked {
                                            match action {
                                                0 => set_home_nav(ui.ctx(), HomeNav::NewProject),
                                                1 => enter_studio_open_dialog(app),
                                                2 => new_composition(app),
                                                _ => import_footage_dialog(app, ui.ctx()),
                                            }
                                        }
                                        ui.add_space(8.0);
                                    }
                                } else {
                                    let action_gap = 14.0;
                                    ui.horizontal(|ui| {
                                        let button_width =
                                            ((ui.available_width() - action_gap) / 2.0).max(72.0);
                                        if ui
                                            .allocate_ui(
                                                egui::vec2(button_width, action_height),
                                                |ui| {
                                                    reference_action_button(
                                                        ui,
                                                        crate::ui::icons::SVG_FILE,
                                                        "New Project",
                                                        "Start from scratch",
                                                        true,
                                                        action_height,
                                                    )
                                                },
                                            )
                                            .inner
                                            .clicked()
                                        {
                                            set_home_nav(ui.ctx(), HomeNav::NewProject);
                                        }
                                        ui.add_space(action_gap);
                                        if ui
                                            .allocate_ui(
                                                egui::vec2(button_width, action_height),
                                                |ui| {
                                                    reference_action_button(
                                                        ui,
                                                        crate::ui::icons::SVG_OPEN_FOLDER,
                                                        "Open Project",
                                                        "Browse files",
                                                        false,
                                                        action_height,
                                                    )
                                                },
                                            )
                                            .inner
                                            .clicked()
                                        {
                                            enter_studio_open_dialog(app);
                                        }
                                    });
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        let button_width =
                                            ((ui.available_width() - action_gap) / 2.0).max(72.0);
                                        if ui
                                            .allocate_ui(
                                                egui::vec2(button_width, action_height),
                                                |ui| {
                                                    reference_action_button(
                                                        ui,
                                                        crate::ui::icons::SVG_COMPOSITION,
                                                        "New Composition",
                                                        "Create a comp",
                                                        false,
                                                        action_height,
                                                    )
                                                },
                                            )
                                            .inner
                                            .clicked()
                                        {
                                            new_composition(app);
                                        }
                                        ui.add_space(action_gap);
                                        if ui
                                            .allocate_ui(
                                                egui::vec2(button_width, action_height),
                                                |ui| {
                                                    reference_action_button(
                                                        ui,
                                                        crate::ui::icons::SVG_IMPORT,
                                                        "Import Footage",
                                                        "Add media",
                                                        false,
                                                        action_height,
                                                    )
                                                },
                                            )
                                            .inner
                                            .clicked()
                                        {
                                            import_footage_dialog(app, ui.ctx());
                                        }
                                    });
                                }
                            },
                        );
                    };
                    if stacked {
                        ui.horizontal_wrapped(draw_home_first_row);
                    } else {
                        ui.horizontal(draw_home_first_row);
                    }
                    ui.add_space(if compact { 10.0 } else { 23.0 });
                    draw_reference_project_row(app, ui, ctx, compact);
                    ui.add_space(if compact { 10.0 } else { 24.0 });
                    draw_reference_templates(ui, compact, true);
                    ui.add_space(if compact { 10.0 } else { 20.0 });
                });
        });
}
