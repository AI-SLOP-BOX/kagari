use super::*;

pub(in crate::ui::home_screen) fn draw_reference_effects_page(
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
            if narrow || medium {
                draw_reference_effects_narrow(app, ui, ctx);
                return;
            }
            let effect_names = [
                "Lumetri Color",
                "Brightness & Contrast",
                "Hue/Saturation",
                "Curves",
                "Levels",
                "Color Balance",
            ];
            let selected_effect_name = ctx
                .data_mut(|d| d.get_temp::<usize>(egui::Id::new("reference_effect_selection")))
                .and_then(|index| effect_names.get(index).copied())
                .unwrap_or(effect_names[0]);
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
                            65.0
                        } else if medium {
                            150.0
                        } else {
                            214.0
                        });
                        ui.label(egui::RichText::new("Effects").size(13.0));
                        ui.add_space(6.0);
                        let search_width = ui.available_width();
                        reference_search_field(
                            ui,
                            &mut app.home_search,
                            "Search effects...",
                            search_width,
                            if narrow { 22.0 } else { 28.0 },
                        );
                        ui.add_space(6.0);
                        let category_id = egui::Id::new("reference_effect_category");
                        let mut selected_category = ctx
                            .data_mut(|d| d.get_temp::<usize>(category_id))
                            .unwrap_or(2);
                        for (index, label) in [
                            "Favorites",
                            "Recent",
                            "Color",
                            "Distort",
                            "Keying",
                            "Blur & Sharpen",
                            "Generate",
                            "Stylize",
                            "Transition",
                            "Utility",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let active = selected_category == index;
                            let row_width = ui.available_width().min(if narrow {
                                ui.available_width()
                            } else {
                                154.0
                            });
                            let row_height = if narrow { 20.0 } else { 25.0 };
                            let category_icon = match index {
                                0 => crate::ui::icons::SVG_STAR,
                                1 => crate::ui::icons::SVG_CLOCK,
                                2 => crate::ui::icons::SVG_PALETTE,
                                _ => crate::ui::icons::SVG_CHEVRON_RIGHT,
                            };
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
                                                    crate::ui::icons::render_svg_bytes(
                                                        ui,
                                                        &format!("effect-category-{index}"),
                                                        category_icon,
                                                        egui::vec2(
                                                            if narrow { 8.0 } else { 12.0 },
                                                            if narrow { 8.0 } else { 12.0 },
                                                        ),
                                                        if active {
                                                            colors::TEXT_PRIMARY
                                                        } else {
                                                            colors::TEXT_SECONDARY
                                                        },
                                                    );
                                                    ui.add_space(3.0);
                                                    ui.label(
                                                        egui::RichText::new(label)
                                                            .size(if narrow { 7.0 } else { 12.0 })
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
                                selected_category = index;
                            }
                            if index == 2 && selected_category == 2 {
                                let effect_id = egui::Id::new("reference_effect_selection");
                                let mut selected_effect = ctx
                                    .data_mut(|d| d.get_temp::<usize>(effect_id))
                                    .unwrap_or(0);
                                for (sub_index, sub_label) in [
                                    "Lumetri Color",
                                    "Brightness & Contrast",
                                    "Hue/Saturation",
                                    "Curves",
                                    "Levels",
                                    "Color Balance",
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    let sub_active = selected_effect == sub_index;
                                    if ui
                                        .add_sized(
                                            [
                                                if narrow { ui.available_width() } else { 146.0 },
                                                if narrow { 18.0 } else { 23.0 },
                                            ],
                                            egui::Button::new(
                                                egui::RichText::new(format!("   {sub_label}"))
                                                    .size(if narrow { 6.0 } else { 11.0 })
                                                    .color(if sub_active {
                                                        colors::TEXT_PRIMARY
                                                    } else {
                                                        colors::TEXT_SECONDARY
                                                    }),
                                            )
                                            .fill(if sub_active {
                                                egui::Color32::from_rgb(22, 52, 84)
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            })
                                            .rounding(3.0),
                                        )
                                        .clicked()
                                    {
                                        selected_effect = sub_index;
                                    }
                                }
                                ctx.data_mut(|d| d.insert_temp(effect_id, selected_effect));
                            }
                        }
                        ctx.data_mut(|d| d.insert_temp(category_id, selected_category));
                    });
                ui.add_space(if narrow { 4.0 } else { 12.0 });
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BG_PANEL_ALT)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        crate::ui::theme::colors::BORDER_SUBTLE,
                    ))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::same(if narrow { 3.0 } else { 12.0 }))
                    .show(ui, |ui| {
                        ui.set_width(if narrow {
                            132.0
                        } else if medium {
                            (ui.available_width() * 0.62).max(260.0)
                        } else {
                            (ui.available_width() * 0.62).max(360.0)
                        });
                        ui.label(egui::RichText::new(selected_effect_name).size(if narrow {
                            10.0
                        } else {
                            16.0
                        }));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            reference_select_button(
                                ui,
                                "Fit",
                                if narrow { 30.0 } else { 54.0 },
                                narrow,
                            );
                            ui.add_space(if narrow { 3.0 } else { 6.0 });
                            reference_select_button(
                                ui,
                                "Full",
                                if narrow { 34.0 } else { 62.0 },
                                narrow,
                            );
                            ui.add_space(if narrow { 3.0 } else { 8.0 });
                            for (index, icon) in
                                [crate::ui::icons::SVG_GRID, crate::ui::icons::SVG_SORT]
                                    .into_iter()
                                    .enumerate()
                            {
                                crate::ui::icons::render_svg_bytes(
                                    ui,
                                    &format!("effect-view-tool-{index}"),
                                    icon,
                                    egui::vec2(
                                        if narrow { 14.0 } else { 18.0 },
                                        if narrow { 14.0 } else { 18.0 },
                                    ),
                                    colors::TEXT_SECONDARY,
                                );
                                ui.add_space(if narrow { 3.0 } else { 6.0 });
                            }
                        });
                        ui.add_space(8.0);
                        if let Some(id) = reference_texture(app, ctx, "recent_project_rift.webp") {
                            let image_size = egui::vec2(
                                ui.available_width(),
                                if narrow || compact { 170.0 } else { 280.0 },
                            );
                            reference_cover_image(ui, id, image_size, 2.64);
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("00:00:00:00")
                                .small()
                                .color(colors::ACCENT_CYAN),
                        );
                        ui.horizontal(|ui| {
                            for (index, icon) in [
                                crate::ui::icons::SVG_STEP_BACK,
                                crate::ui::icons::SVG_PLAY,
                                crate::ui::icons::SVG_STEP_FORWARD,
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                crate::ui::icons::render_svg_bytes(
                                    ui,
                                    &format!("effect-transport-{index}"),
                                    icon,
                                    egui::vec2(
                                        if narrow { 12.0 } else { 16.0 },
                                        if narrow { 12.0 } else { 16.0 },
                                    ),
                                    colors::TEXT_SECONDARY,
                                );
                                ui.add_space(if narrow { 3.0 } else { 6.0 });
                            }
                        });
                    });
                ui.add_space(if narrow { 4.0 } else { 12.0 });
                egui::Frame::none()
                    .fill(crate::ui::theme::colors::BG_PANEL_ALT)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        crate::ui::theme::colors::BORDER_SUBTLE,
                    ))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_width(if narrow {
                            72.0
                        } else if medium {
                            180.0
                        } else {
                            230.0
                        });
                        ui.label(egui::RichText::new("Recent Controls").size(if narrow {
                            9.0
                        } else {
                            14.0
                        }));
                        ui.add_space(if narrow { 3.0 } else { 8.0 });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(selected_effect_name).size(if narrow {
                                7.0
                            } else {
                                12.0
                            }));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new("Reset")
                                            .size(if narrow { 7.0 } else { 11.0 })
                                            .color(colors::ACCENT_CYAN),
                                    );
                                },
                            );
                        });
                        ui.add_space(if narrow { 3.0 } else { 6.0 });
                        ui.separator();
                        ui.add_space(if narrow { 3.0 } else { 6.0 });
                        ui.horizontal(|ui| {
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                "effect-basic-chevron",
                                crate::ui::icons::SVG_CHEVRON_DOWN,
                                egui::vec2(
                                    if narrow { 8.0 } else { 12.0 },
                                    if narrow { 8.0 } else { 12.0 },
                                ),
                                colors::TEXT_SECONDARY,
                            );
                            ui.label(
                                egui::RichText::new("Basic Correction")
                                    .size(if narrow { 7.0 } else { 12.0 })
                                    .strong(),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Input LUT")
                                    .size(if narrow { 7.0 } else { 12.0 })
                                    .color(colors::TEXT_SECONDARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    reference_select_button(
                                        ui,
                                        "None",
                                        if narrow { 38.0 } else { 94.0 },
                                        narrow,
                                    );
                                },
                            );
                        });
                        ui.add_space(if narrow { 3.0 } else { 6.0 });
                        for (label, default) in [
                            ("Exposure", 0.0_f32),
                            ("Contrast", 0.0),
                            ("Highlights", -23.3),
                            ("Shadows", 0.2),
                            ("Whites", 0.0),
                            ("Blacks", -0.2),
                        ] {
                            let value_id = egui::Id::new(("reference_effect_value", label));
                            let mut value = ctx
                                .data_mut(|d| d.get_temp::<f32>(value_id))
                                .unwrap_or(default);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(label)
                                        .size(if narrow { 7.0 } else { 12.0 })
                                        .color(colors::TEXT_SECONDARY),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_sized(
                                            [
                                                if narrow { 48.0 } else { 86.0 },
                                                if narrow { 18.0 } else { 22.0 },
                                            ],
                                            egui::DragValue::new(&mut value)
                                                .speed(0.1)
                                                .fixed_decimals(1),
                                        );
                                    },
                                );
                            });
                            ctx.data_mut(|d| d.insert_temp(value_id, value));
                            ui.add_space(if narrow { 2.0 } else { 5.0 });
                        }
                        for (index, label) in [
                            "Creative",
                            "Curves",
                            "Color Wheels & Match",
                            "HSL Secondary",
                            "Vignette",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            ui.separator();
                            ui.horizontal(|ui| {
                                crate::ui::icons::render_svg_bytes(
                                    ui,
                                    &format!("effect-section-chevron-{index}"),
                                    crate::ui::icons::SVG_CHEVRON_RIGHT,
                                    egui::vec2(
                                        if narrow { 8.0 } else { 12.0 },
                                        if narrow { 8.0 } else { 12.0 },
                                    ),
                                    colors::TEXT_SECONDARY,
                                );
                                ui.label(
                                    egui::RichText::new(label)
                                        .size(if narrow { 7.0 } else { 12.0 })
                                        .color(colors::TEXT_SECONDARY),
                                );
                            });
                        }
                    });
            });
        });
}
