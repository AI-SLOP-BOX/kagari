use super::*;

pub(in crate::ui::home_screen) fn draw_reference_templates_narrow(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let top = ui.cursor().min;
    let panel = egui::Rect::from_min_size(top, egui::vec2(314.0, 209.0));
    let stroke = egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE);
    ui.painter()
        .rect(panel, 4.0, crate::ui::theme::colors::BG_PANEL_ALT, stroke);
    ui.painter().text(
        egui::pos2(panel.left() + 5.0, panel.top() + 13.0),
        egui::Align2::LEFT_CENTER,
        "Project Templates",
        egui::FontId::proportional(9.0),
        colors::TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::pos2(panel.left() + 6.0, panel.top() + 37.0),
        egui::Align2::LEFT_CENTER,
        "⌕  Search templates...",
        egui::FontId::proportional(6.0),
        colors::TEXT_SECONDARY,
    );
    for (index, label) in [
        "All",
        "Film & Video",
        "Motion Graphics",
        "VFX",
        "Social Media",
    ]
    .into_iter()
    .enumerate()
    {
        let widths = [25.0, 45.0, 54.0, 25.0, 49.0];
        let x = panel.left() + 106.0 + widths[..index].iter().sum::<f32>() + index as f32 * 4.0;
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(x, panel.top() + 27.0),
                egui::vec2(widths[index], 20.0),
            ),
            egui::Button::new(egui::RichText::new(label).size(5.5)).fill(if index == 0 {
                colors::ACCENT_BLUE
            } else {
                egui::Color32::TRANSPARENT
            }),
        );
    }
    let cards = [
        (
            "templates/cinematic_title.webp",
            "Cinematic Title",
            "Film & Video",
        ),
        (
            "templates/logo_reveal.webp",
            "Logo Reveal",
            "Motion Graphics",
        ),
        ("templates/hud_interface.webp", "HUD Interface", "VFX"),
        (
            "templates/product_showcase.webp",
            "Product Showcase",
            "Motion Graphics",
        ),
        ("templates/vfx_breakdown.webp", "VFX Breakdown", "VFX"),
        (
            "templates/social_media_reel.webp",
            "Social Media Reel",
            "Social Media",
        ),
        (
            "templates/text_animation.webp",
            "Text Animation",
            "Motion Graphics",
        ),
        ("templates/clean_plate.webp", "Clean Plate", "VFX"),
    ];
    for (index, (asset, title, category)) in cards.into_iter().enumerate() {
        let card = egui::Rect::from_min_size(
            egui::pos2(
                panel.left() + 6.0 + (index % 4) as f32 * 77.0,
                panel.top() + 52.0 + (index / 4) as f32 * 76.0,
            ),
            egui::vec2(64.0, 70.0),
        );
        ui.painter().rect(
            card,
            4.0,
            crate::ui::theme::colors::BG_PANEL_RAISED,
            egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BG_ELEVATED),
        );
        if let Some(id) = reference_texture(app, ctx, asset) {
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(card.left() + 3.0, card.top() + 3.0),
                    egui::vec2(58.0, 36.0),
                ),
                egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(58.0, 36.0)))
                    .fit_to_exact_size(egui::vec2(58.0, 36.0)),
            );
        }
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(card.left() + 3.0, card.top() + 42.0),
                egui::vec2(58.0, 13.0),
            ),
            egui::Label::new(egui::RichText::new(title).size(5.8).strong()),
        );
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(card.left() + 3.0, card.top() + 55.0),
                egui::vec2(58.0, 11.0),
            ),
            egui::Label::new(
                egui::RichText::new(category)
                    .size(5.2)
                    .color(colors::TEXT_SECONDARY),
            ),
        );
    }
    ui.allocate_space(egui::vec2(314.0, 209.0));
}

pub(in crate::ui::home_screen) fn draw_reference_templates_page(
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
            if narrow {
                draw_reference_templates_narrow(app, ui, ctx);
                return;
            }
            egui::Frame::none()
                .fill(crate::ui::theme::colors::BG_PANEL_ALT)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    crate::ui::theme::colors::BORDER_SUBTLE,
                ))
                .rounding(4.0)
                .inner_margin(egui::Margin::same(if narrow { 3.0 } else { 14.0 }))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Project Templates")
                            .size(if narrow { 10.0 } else { 18.0 })
                            .strong(),
                    );
                    ui.add_space(if narrow { 5.0 } else { 8.0 });
                    ui.spacing_mut().item_spacing = egui::vec2(if narrow { 3.0 } else { 8.0 }, 0.0);
                    ui.horizontal_wrapped(|ui| {
                        let search_width = if narrow { 92.0 } else { 220.0 };
                        reference_search_field(
                            ui,
                            &mut app.home_search,
                            "Search templates...",
                            search_width,
                            if narrow { 20.0 } else { 28.0 },
                        );
                        ui.add_space(6.0);
                        let filter_id = egui::Id::new("reference_template_filter");
                        let mut selected_filter = ctx
                            .data_mut(|d| d.get_temp::<usize>(filter_id))
                            .unwrap_or(0);
                        for (index, label) in [
                            "All",
                            "Film & Video",
                            "Motion Graphics",
                            "VFX",
                            "Social Media",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let active = selected_filter == index;
                            let filter_width = if narrow {
                                [22.0, 42.0, 52.0, 22.0, 42.0][index]
                            } else {
                                0.0
                            };
                            let filter = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(if narrow { 7.0 } else { 11.0 })
                                    .color(if active {
                                        colors::TEXT_PRIMARY
                                    } else {
                                        colors::TEXT_SECONDARY
                                    }),
                            )
                            .fill(if active {
                                colors::ACCENT_BLUE
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .rounding(4.0);
                            let clicked = if narrow {
                                ui.add_sized([filter_width, 20.0], filter).clicked()
                            } else {
                                ui.add(filter).clicked()
                            };
                            if clicked {
                                selected_filter = index;
                            }
                        }
                        ctx.data_mut(|d| d.insert_temp(filter_id, selected_filter));
                    });
                    ui.add_space(12.0);
                    let selected_filter = ctx.data_mut(|d| {
                        d.get_temp::<usize>(egui::Id::new("reference_template_filter"))
                            .unwrap_or(0)
                    });
                    let search_query = app.home_search.clone();
                    draw_reference_template_catalog(
                        app,
                        ctx,
                        ui,
                        compact,
                        selected_filter,
                        &search_query,
                    );
                });
        });
}

pub(in crate::ui::home_screen) fn draw_reference_template_catalog(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    compact: bool,
    selected_filter: usize,
    query: &str,
) {
    let narrow = reference_narrow(ui);
    let gap = if narrow { 6.0 } else { 10.0 };
    let columns = 4.0;
    let width = ((ui.available_width() - gap * (columns - 1.0)) / columns).max(if narrow {
        54.0
    } else {
        130.0
    });
    let height = if narrow {
        68.0
    } else if compact {
        124.0
    } else {
        154.0
    };
    let icon_height = if narrow {
        36.0
    } else if compact {
        58.0
    } else {
        78.0
    };
    let cards = [
        (
            "templates/cinematic_title.webp",
            "Cinematic Title",
            "Film & Video",
        ),
        (
            "templates/logo_reveal.webp",
            "Logo Reveal",
            "Motion Graphics",
        ),
        ("templates/hud_interface.webp", "HUD Interface", "VFX"),
        (
            "templates/product_showcase.webp",
            "Product Showcase",
            "Motion Graphics",
        ),
        ("templates/vfx_breakdown.webp", "VFX Breakdown", "VFX"),
        (
            "templates/social_media_reel.webp",
            "Social Media Reel",
            "Social Media",
        ),
        (
            "templates/text_animation.webp",
            "Text Animation",
            "Motion Graphics",
        ),
        ("templates/clean_plate.webp", "Clean Plate", "VFX"),
    ];
    ui.spacing_mut().item_spacing = egui::vec2(0.0, if narrow { 12.0 } else { 10.0 });
    let normalized_query = query.trim().to_ascii_lowercase();
    let matches_filter = |title: &str, category: &str| {
        let category_index = match category {
            "Film & Video" => 1,
            "Motion Graphics" => 2,
            "VFX" => 3,
            "Social Media" => 4,
            _ => 0,
        };
        (normalized_query.is_empty()
            || format!("{title} {category}")
                .to_ascii_lowercase()
                .contains(&normalized_query))
            && (selected_filter == 0 || selected_filter == category_index)
    };
    let visible_template_count = cards
        .iter()
        .filter(|(_, title, category)| matches_filter(title, category))
        .count();
    let mut rendered_templates = 0_usize;
    ui.horizontal_wrapped(|ui| {
        for (asset, title, category) in cards {
            if !matches_filter(title, category) {
                continue;
            }
            rendered_templates += 1;
            fixed_card_with_inset(
                ui,
                egui::vec2(width, height),
                crate::ui::theme::colors::BG_PANEL_RAISED,
                crate::ui::theme::colors::BG_ELEVATED,
                5.0,
                if narrow { 2.0 } else { 14.0 },
                |ui| {
                    ui.vertical(|ui| {
                        if narrow {
                            ui.spacing_mut().item_spacing.y = 0.0;
                        }
                        ui.horizontal_centered(|ui| {
                            let image_size = egui::vec2(
                                (width - if narrow { 4.0 } else { 28.0 }).max(10.0),
                                icon_height,
                            );
                            if let Some(id) = reference_texture(app, ctx, asset) {
                                reference_cover_image(ui, id, image_size, 70.0 / 40.0);
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(title)
                                .size(if narrow { 7.0 } else { 12.0 })
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(category)
                                .size(if narrow { 7.0 } else { 10.5 })
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                },
            );
            if rendered_templates < visible_template_count {
                ui.add_space(gap);
            }
        }
    });
    if visible_template_count == 0 {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("No templates found")
                    .size(14.0)
                    .color(colors::TEXT_PRIMARY),
            );
            ui.label(
                egui::RichText::new("Try another category or search term.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
        });
    }
}
