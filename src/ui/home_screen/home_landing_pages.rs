use super::*;

const LANDING_CARD_GAP: f32 = 20.0;

pub(in crate::ui::home_screen) fn draw_reference_nav(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let compact_brand = ui.available_width() < 130.0;
    ui.add_space(if compact_brand { 22.0 } else { 20.0 });
    if app.home_banner.is_none() {
        if let Ok(img) = crate::ui::embedded_assets::open_image(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = load_logo_texture(ctx, img);
        }
    }
    ui.horizontal(|ui| {
        ui.add_space(if compact_brand { 18.0 } else { 22.0 });
        if let Some(texture) = app.home_banner.as_ref() {
            let logo_size = if compact_brand { 56.0 } else { 68.0 };
            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                texture.id(),
                egui::vec2(logo_size, logo_size),
            )));
        }
    });
    ui.add_space(-5.0);
    if compact_brand {
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("Kagari VFX")
                    .size(16.0)
                    .color(colors::TEXT_PRIMARY),
            );
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.label(
                egui::RichText::new("Kagari VFX")
                    .size(17.0)
                    .color(colors::TEXT_PRIMARY),
            );
        });
    }
    ui.add_space(if compact_brand { 31.0 } else { 37.0 });
    let current = home_nav(ctx);
    let compact_sidebar = ui.available_width() < 130.0;
    for (nav, icon, label) in [
        (HomeNav::Home, crate::ui::icons::SVG_HOME, "Home"),
        (HomeNav::Projects, crate::ui::icons::SVG_FOLDER, "Projects"),
        (
            HomeNav::Templates,
            crate::ui::icons::SVG_LAYERS,
            "Templates",
        ),
    ] {
        let active = current == nav;
        let (row_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), egui::Sense::hover());
        let pointer_over_row = ctx
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|pointer| row_rect.contains(pointer));
        if active && pointer_over_row {
            ui.painter().rect_filled(
                row_rect.shrink2(egui::vec2(0.0, 1.0)),
                6.0,
                colors::BG_HOVER,
            );
        } else if active {
            ui.painter().rect_filled(
                row_rect.shrink2(egui::vec2(0.0, 1.0)),
                6.0,
                crate::ui::theme::colors::BG_ACTIVE,
            );
        } else if pointer_over_row {
            ui.painter().rect_filled(
                row_rect.shrink2(egui::vec2(0.0, 1.0)),
                6.0,
                colors::BG_HOVER,
            );
        }
        let icon_size = if compact_sidebar { 18.0 } else { 20.0 };
        crate::ui::icons::render_svg_at(
            ui,
            format!("reference-sidebar-{label}"),
            icon,
            egui::vec2(icon_size, icon_size),
            if active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
            egui::pos2(
                row_rect.left() + if compact_sidebar { 12.0 } else { 18.0 },
                row_rect.center().y - icon_size * 0.5,
            ),
        );
        let label_left = row_rect.left() + if compact_sidebar { 40.0 } else { 54.0 };
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(label_left, row_rect.top()),
                egui::pos2(row_rect.right() - 10.0, row_rect.bottom()),
            ),
            egui::Label::new(egui::RichText::new(label).size(13.0).color(if active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            }))
            .truncate(),
        );
        if ui
            .interact(
                row_rect,
                egui::Id::new(("reference-sidebar", label)),
                egui::Sense::click(),
            )
            .clicked()
        {
            set_home_nav(ctx, nav);
        }
        ui.add_space(3.0);
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        let footer_bottom_space = if compact_brand {
            (ui.available_height() * 0.038).clamp(14.0, 39.0)
        } else {
            (ui.available_height() * 0.038 + 12.0).clamp(14.0, 51.0)
        };
        let footer_item_gap = (ui.available_height() * 0.007).clamp(6.0, 10.0);
        let footer_divider_gap = (ui.available_height() * 0.025).clamp(14.0, 26.0);
        ui.add_space(footer_bottom_space);
        let help_clicked = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(23.0);
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        "home-help",
                        crate::ui::icons::SVG_HELP,
                        egui::vec2(28.0, 28.0),
                        colors::TEXT_SECONDARY,
                    );
                },
            )
            .response
            .interact(egui::Sense::click())
            .clicked();
        if help_clicked {
            app.show_shortcuts_dialog = true;
        }
        ui.add_space(footer_item_gap);
        let settings_clicked = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(23.0);
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        "home-settings",
                        crate::ui::icons::SVG_SETTINGS,
                        egui::vec2(28.0, 28.0),
                        colors::TEXT_SECONDARY,
                    );
                },
            )
            .response
            .interact(egui::Sense::click())
            .clicked();
        if settings_clicked {
            set_home_nav(ctx, HomeNav::Settings);
        }
        ui.add_space(footer_divider_gap);
        ui.horizontal(|ui| {
            ui.add_space(21.0);
            let line_width = ui.available_width().clamp(1.0, 95.0);
            let (line_rect, _) =
                ui.allocate_exact_size(egui::vec2(line_width, 1.0), egui::Sense::hover());
            ui.painter().line_segment(
                [line_rect.left_top(), line_rect.right_top()],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(47, 63, 78)),
            );
        });
        ui.add_space(10.0);
    });
}

pub(in crate::ui::home_screen) fn home_asset_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/home")
        .join(name)
}

pub(in crate::ui::home_screen) fn reference_texture(
    app: &mut KagariApp,
    ctx: &egui::Context,
    name: &str,
) -> Option<egui::TextureId> {
    if name == "kagari_logo.webp" || name == "logo.png" {
        if app.home_banner.is_none() {
            if let Ok(img) = crate::ui::embedded_assets::open_image(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("assets/kagari_logo.webp"),
            ) {
                app.home_banner = load_logo_texture(ctx, img);
            }
        }
        return app.home_banner.as_ref().map(egui::TextureHandle::id);
    }
    let path = if name == "render_preview.webp" {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/studio/render_preview.webp")
    } else if let Some(asset) = name.strip_prefix("templates/") {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/templates")
            .join(asset)
    } else {
        home_asset_path(name)
    };
    thumb_for(app, ctx, &path)
}

pub(in crate::ui::home_screen) fn reference_narrow(ui: &egui::Ui) -> bool {
    ui.available_width() < 380.0
}

pub(in crate::ui::home_screen) fn reference_medium(ui: &egui::Ui) -> bool {
    let width = ui.available_width();
    (380.0..760.0).contains(&width)
}

pub(in crate::ui::home_screen) fn reference_content_margin(ui: &egui::Ui) -> f32 {
    // The inner content is narrower by both side margins; keep the breakpoint
    // aligned with reference_narrow after the margin is applied.
    if ui.available_width() < 442.0 {
        4.0
    } else {
        31.0
    }
}

pub(in crate::ui::home_screen) fn fixed_card(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    fill: egui::Color32,
    stroke: egui::Color32,
    radius: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    fixed_card_with_inset(ui, size, fill, stroke, radius, 20.0, add)
}

pub(in crate::ui::home_screen) fn fixed_card_with_inset(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    fill: egui::Color32,
    stroke: egui::Color32,
    radius: f32,
    inset: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(
        rect,
        radius,
        egui::Stroke::new(
            1.0_f32,
            if response.hovered() {
                colors::ACCENT_BLUE
            } else {
                stroke
            },
        ),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(inset))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect.shrink(1.0));
    add(&mut child);
    response
}

pub(in crate::ui::home_screen) fn reference_action_button(
    ui: &mut egui::Ui,
    icon: &'static str,
    title: &str,
    subtitle: &str,
    accent: bool,
    height: f32,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(width, height),
        &format!("{title}: {subtitle}"),
        true,
        false,
    );
    let fill = if accent {
        egui::Color32::from_rgb(24, 92, 180)
    } else {
        egui::Color32::from_rgb(28, 39, 51)
    };
    let fill = if response.hovered() {
        crate::ui::theme::colors::BG_HOVER
    } else {
        fill
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let stroke = if accent {
        colors::ACCENT_BLUE
    } else {
        egui::Color32::from_rgb(45, 62, 79)
    };
    ui.painter()
        .rect_stroke(rect, 6.0, egui::Stroke::new(1.0_f32, stroke));
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(12.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner.spacing_mut().item_spacing.x = 10.0;
    let icon_size = if height >= 80.0 { 32.0 } else { 24.0 };
    crate::ui::icons::render_svg_bytes(
        &mut inner,
        title,
        icon,
        egui::vec2(icon_size, icon_size),
        egui::Color32::from_rgb(220, 230, 244),
    );
    inner.vertical(|ui| {
        ui.label(
            egui::RichText::new(crate::ui::localization::tr(title))
                .size(if height >= 80.0 { 14.0 } else { 11.0 })
                .strong(),
        );
        if !subtitle.is_empty() {
            ui.label(
                egui::RichText::new(crate::ui::localization::tr(subtitle))
                    .size(if height >= 80.0 { 12.0 } else { 9.0 })
                    .color(colors::TEXT_SECONDARY),
            );
        }
    });
    inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            &format!("{title}-arrow"),
            crate::ui::icons::SVG_ARROW_RIGHT,
            egui::vec2(15.0, 15.0),
            egui::Color32::from_rgb(190, 204, 226),
        );
    });
    response
}

pub(in crate::ui::home_screen) fn reference_cta_button(
    ui: &mut egui::Ui,
    title: &str,
    compact: bool,
) -> egui::Response {
    let width = ui.available_width();
    let height = if compact { 24.0 } else { 43.0 };
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(width, height),
        title,
        true,
        false,
    );
    ui.painter().rect_filled(
        rect,
        6.0,
        if response.hovered() {
            crate::ui::theme::colors::ACCENT_ACTION_HOVER
        } else {
            crate::ui::theme::colors::ACCENT_ACTION
        },
    );
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(10.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner.label(
        egui::RichText::new(title)
            .size(if compact { 12.0 } else { 14.0 })
            .strong(),
    );
    inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "cta-arrow",
            crate::ui::icons::SVG_ARROW_RIGHT,
            egui::vec2(17.0, 17.0),
            egui::Color32::WHITE,
        );
    });
    response
}

pub(in crate::ui::home_screen) fn reference_view_all_button(
    ui: &mut egui::Ui,
    label: &str,
) -> bool {
    ui.allocate_ui_with_layout(
        egui::vec2(82.0, 24.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let clicked = ui
                .scope(|ui| {
                    ui.spacing_mut().button_padding.x = 4.0;
                    ui.add(
                        egui::Button::new(egui::RichText::new(label).color(colors::TEXT_SECONDARY))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .rounding(4.0),
                    )
                })
                .inner
                .clicked();
            ui.add_space(2.0);
            let arrow_clicked = crate::ui::icons::render_svg_bytes(
                ui,
                "view-all-arrow",
                crate::ui::icons::SVG_ARROW_RIGHT,
                egui::vec2(15.0, 15.0),
                colors::TEXT_SECONDARY,
            )
            .interact(egui::Sense::click())
            .clicked();
            clicked || arrow_clicked
        },
    )
    .inner
}

pub(in crate::ui::home_screen) fn reference_icon_button(
    ui: &mut egui::Ui,
    id: &str,
    icon: &'static str,
    size: f32,
) -> egui::Response {
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(size, size),
        id,
        true,
        false,
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, crate::ui::theme::colors::BG_HOVER);
    }
    let icon_size = size - 7.0;
    crate::ui::icons::render_svg_at(
        ui,
        id.to_owned(),
        icon,
        egui::vec2(icon_size, icon_size),
        colors::TEXT_SECONDARY,
        rect.center() - egui::vec2(icon_size * 0.5, icon_size * 0.5),
    );
    response
}

pub(in crate::ui::home_screen) fn reference_search_field(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    width: f32,
    height: f32,
) {
    let font_size = if width < 120.0 {
        7.0
    } else if width < 260.0 {
        11.0
    } else {
        12.0
    };
    egui::Frame::none()
        .fill(crate::ui::theme::colors::BG_DEEPEST)
        .stroke(egui::Stroke::new(
            1.0_f32,
            crate::ui::theme::colors::BORDER_MEDIUM,
        ))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
        .show(ui, |ui| {
            let inner_width = (width - 12.0).max(1.0);
            ui.set_width(inner_width);
            ui.set_min_height(height);
            ui.horizontal(|ui| {
                crate::ui::icons::render_svg_bytes(
                    ui,
                    "reference-search",
                    crate::ui::icons::SVG_SEARCH,
                    egui::vec2(13.0, 13.0),
                    colors::TEXT_SECONDARY,
                );
                ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text(hint)
                        .font(egui::FontId::proportional(font_size))
                        .frame(false)
                        .desired_width((inner_width - 13.0).max(1.0)),
                );
            });
        });
}

pub(in crate::ui::home_screen) fn reference_select_button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    compact: bool,
) -> egui::Response {
    let height = if compact { 20.0 } else { 24.0 };
    let (rect, response) = crate::ui::custom_widgets::painter_button_target(
        ui,
        egui::vec2(width, height),
        label,
        true,
        false,
    );
    ui.painter().rect_filled(
        rect,
        4.0,
        if response.hovered() {
            crate::ui::theme::colors::BG_HOVER
        } else {
            egui::Color32::from_rgb(24, 35, 47)
        },
    );
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_MEDIUM),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(6.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.label(
        egui::RichText::new(label)
            .size(if compact { 8.5 } else { 11.0 })
            .color(colors::TEXT_PRIMARY),
    );
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "reference-select-chevron",
            crate::ui::icons::SVG_CHEVRON_DOWN,
            egui::vec2(11.0, 11.0),
            colors::TEXT_SECONDARY,
        );
    });
    response
}

pub(in crate::ui::home_screen) fn reference_cover_image(
    ui: &mut egui::Ui,
    id: egui::TextureId,
    size: egui::Vec2,
    source_aspect: f32,
) {
    let target_aspect = size.x / size.y.max(1.0);
    let (left, right, top, bottom) = if target_aspect < source_aspect {
        let visible_u = (target_aspect / source_aspect).min(1.0);
        let left = (1.0 - visible_u) * 0.5;
        (left, left + visible_u, 0.0, 1.0)
    } else {
        let visible_v = (source_aspect / target_aspect).min(1.0);
        let top = (1.0 - visible_v) * 0.5;
        (0.0, 1.0, top, top + visible_v)
    };
    ui.add(
        egui::Image::new(egui::load::SizedTexture::new(id, size))
            .uv(egui::Rect::from_min_max(
                egui::pos2(left, top),
                egui::pos2(right, bottom),
            ))
            .fit_to_exact_size(size)
            .rounding(egui::Rounding::same(5.0))
            .maintain_aspect_ratio(false),
    );
}

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

pub(in crate::ui::home_screen) fn draw_reference_continue_body(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    compact: bool,
    vertical: bool,
    preview_height: f32,
) {
    if vertical {
        let height = (ui.available_width() / 1.82).clamp(76.0, preview_height);
        draw_reference_continue_preview(app, ctx, ui, height);
        ui.add_space(8.0);
        draw_reference_continue_details(app, ctx, ui, compact);
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let preview_width = ui.available_width() * if compact { 0.54 } else { 0.567 };
            ui.allocate_ui(egui::vec2(preview_width, preview_height), |ui| {
                draw_reference_continue_preview(app, ctx, ui, preview_height);
            });
            ui.add_space(if compact { 12.0 } else { 24.0 });
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width().max(1.0), preview_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| draw_reference_continue_details(app, ctx, ui, compact),
            );
        });
    }
}

pub(in crate::ui::home_screen) fn draw_reference_continue_preview(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    height: f32,
) {
    if let Some(id) = reference_texture(app, ctx, "continue_working_preview.webp") {
        let image_width = ui.available_width().max(1.0);
        egui::Frame::none()
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(54, 72, 91),
            ))
            .rounding(5.0)
            .show(ui, |ui| {
                reference_cover_image(ui, id, egui::vec2(image_width, height), 406.0 / 189.0);
            });
    }
}

pub(in crate::ui::home_screen) fn draw_reference_continue_details(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    compact: bool,
) {
    ui.label(
        egui::RichText::new("Eclipse")
            .size(if compact { 18.0 } else { 20.0 })
            .strong(),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "comp-meta",
            crate::ui::icons::SVG_COMP,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("SH_0140 / portal_final_v18")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.horizontal_wrapped(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "resolution-meta",
            crate::ui::icons::SVG_RESOLUTION,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("3840 × 2160")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
        ui.add_space(if compact { 4.0 } else { 12.0 });
        crate::ui::icons::render_svg_bytes(
            ui,
            "frame-meta",
            crate::ui::icons::SVG_FRAME,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("1001 – 1148")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.horizontal(|ui| {
        crate::ui::icons::render_svg_bytes(
            ui,
            "clock-meta",
            crate::ui::icons::SVG_CLOCK,
            egui::vec2(16.0, 16.0),
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
        );
        ui.label(
            egui::RichText::new("Edited 18 minutes ago")
                .size(if compact { 11.0 } else { 14.0 })
                .color(colors::TEXT_SECONDARY),
        );
    });
    ui.add_space(if compact { 4.0 } else { 13.0 });
    if reference_cta_button(ui, "Resume Composition", compact).clicked() {
        if let Some(top) = recent_project_entries(ctx)
            .into_iter()
            .find(|e| e.path.is_file())
        {
            open_project_path(app, &top.path);
        }
    }
}

pub(in crate::ui::home_screen) fn landing_nav_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    current: HomeNav,
    nav: HomeNav,
    icon: &'static str,
    label: &str,
    active_orange: bool,
) {
    let pixel_scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let scale = pixel_scale * (ui.ctx().screen_rect().width() / (1584.0 * pixel_scale)).max(1.0);
    let compact = ui.available_width() < 150.0 * scale;
    let row_height = if ui.ctx().screen_rect().height() < 760.0 * scale {
        44.0 * scale
    } else {
        56.0 * scale
    };
    let active = current == nav;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_height),
        egui::Sense::hover(),
    );
    let pointer_over_row = ctx
        .input(|input| input.pointer.hover_pos())
        .is_some_and(|pointer| rect.contains(pointer));
    if active && pointer_over_row {
        ui.painter().rect_filled(rect, 0.0, colors::BG_HOVER);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(5.0 * scale, rect.height())),
            0.0,
            if active_orange {
                crate::ui::theme::colors::ACCENT_BRAND
            } else {
                colors::ACCENT_CYAN
            },
        );
    } else if active {
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(19, 27, 29));
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(5.0 * scale, rect.height())),
            0.0,
            if active_orange {
                crate::ui::theme::colors::ACCENT_BRAND
            } else {
                colors::ACCENT_CYAN
            },
        );
    } else if pointer_over_row {
        ui.painter().rect_filled(rect, 0.0, colors::BG_HOVER);
    }
    let icon_size = if compact { 20.0 * scale } else { 22.0 * scale };
    crate::ui::icons::render_svg_at(
        ui,
        format!("landing-nav-{label}"),
        icon,
        egui::vec2(icon_size, icon_size),
        if active {
            if active_orange {
                crate::ui::theme::colors::ACCENT_BRAND_HOVER
            } else {
                colors::TEXT_PRIMARY
            }
        } else {
            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
        },
        egui::pos2(
            if compact {
                rect.center().x - icon_size * 0.5
            } else {
                rect.left() + 42.0 * scale
            },
            rect.center().y - icon_size * 0.5,
        ),
    );
    if !compact {
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 92.0 * scale, rect.top()),
            egui::pos2(rect.right() - 18.0 * scale, rect.bottom()),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(label_rect), |label_ui| {
            label_ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center),
                |label_ui| {
                    label_ui.add(
                        egui::Label::new(egui::RichText::new(label).size(16.0 * scale).color(
                            if active {
                                colors::TEXT_PRIMARY
                            } else {
                                crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
                            },
                        ))
                        .truncate(),
                    );
                },
            );
        });
    }
    let response = ui.interact(
        rect,
        egui::Id::new(("landing-nav-row", label)),
        egui::Sense::click(),
    );
    if response.clicked() {
        if label == "終了" {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            set_home_nav(ctx, nav);
        }
    }
    response.on_hover_text(label);
}

pub(in crate::ui::home_screen) fn draw_landing_sidebar(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let pixel_scale = ctx
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let scale = pixel_scale * (ctx.screen_rect().width() / (1584.0 * pixel_scale)).max(1.0);
    let visible = ui.max_rect();
    ui.painter()
        .rect_filled(visible, 0.0, egui::Color32::from_rgb(11, 20, 23));
    ui.painter().line_segment(
        [visible.right_top(), visible.right_bottom()],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let scroll_height = visible.height().max(680.0 * scale);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |scroll_ui| {
            scroll_ui.allocate_ui_with_layout(
                egui::vec2(scroll_ui.available_width(), scroll_height),
                egui::Layout::top_down(egui::Align::Min),
                |content_ui| draw_landing_sidebar_content(app, content_ui, ctx),
            );
        });
}

fn draw_landing_sidebar_content(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let pixel_scale = ctx
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let scale = pixel_scale * (ctx.screen_rect().width() / (1584.0 * pixel_scale)).max(1.0);
    let current = match home_nav(ctx) {
        HomeNav::NewProject => HomeNav::Projects,
        nav => nav,
    };
    let side = ui.max_rect();
    let compact = side.width() < 150.0 * scale;
    let medium = side.width() < 270.0 * scale;
    let row_height = if ctx.screen_rect().height() < 760.0 * scale {
        44.0 * scale
    } else {
        56.0 * scale
    };
    ui.add_space(if compact { 20.0 * scale } else { 47.0 * scale });
    let (brand_rect, _) = ui.allocate_exact_size(
        egui::vec2(
            side.width(),
            if compact {
                56.0 * scale
            } else if medium {
                72.0 * scale
            } else {
                92.0 * scale
            },
        ),
        egui::Sense::hover(),
    );
    if app.home_banner.is_none() {
        if let Ok(img) = crate::ui::embedded_assets::open_image(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = load_logo_texture(ctx, img);
        }
    }
    let brand_painter = ui.painter().with_clip_rect(brand_rect);
    if let Some(texture) = app.home_banner.as_ref() {
        let logo_size = if compact {
            44.0 * scale
        } else if medium {
            56.0 * scale
        } else {
            92.0 * scale
        };
        let logo_rect = egui::Rect::from_min_size(
            egui::pos2(
                if compact {
                    brand_rect.center().x - logo_size * 0.5
                } else if medium {
                    side.left() + 12.0 * scale
                } else {
                    side.left() + 20.0 * scale
                },
                brand_rect.center().y - logo_size * 0.5,
            ),
            egui::vec2(logo_size, logo_size),
        );
        brand_painter.image(
            texture.id(),
            logo_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    if !compact {
        let wordmark_y = brand_rect.center().y;
        if medium {
            brand_painter.text(
                egui::pos2(side.left() + 76.0 * scale, wordmark_y),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(17.0 * scale),
                colors::TEXT_PRIMARY,
            );
        } else {
            brand_painter.text(
                egui::pos2(side.left() + 115.0 * scale, wordmark_y),
                egui::Align2::LEFT_CENTER,
                "Kagari",
                egui::FontId::proportional(25.0 * scale),
                colors::TEXT_PRIMARY,
            );
            brand_painter.text(
                egui::pos2(side.left() + 198.0 * scale, wordmark_y),
                egui::Align2::LEFT_CENTER,
                "VFX",
                egui::FontId::proportional(25.0 * scale),
                crate::ui::theme::colors::TEXT_SECONDARY,
            );
        }
    }
    ui.add_space(if compact { 16.0 * scale } else { 21.5 * scale });
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Home,
        crate::ui::icons::SVG_HOME,
        "ホーム",
        true,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Projects,
        crate::ui::icons::SVG_FOLDER,
        "プロジェクト",
        true,
    );
    ui.add_space(16.0 * scale);
    ui.painter().line_segment(
        [
            egui::pos2(side.left() + 37.0 * scale, ui.cursor().top()),
            egui::pos2(side.right() - 37.0 * scale, ui.cursor().top()),
        ],
        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_MEDIUM),
    );
    ui.add_space(17.0 * scale);
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Tutorial,
        crate::ui::icons::SVG_BOOK,
        "チュートリアル",
        false,
    );
    landing_nav_row(
        ui,
        ctx,
        current,
        HomeNav::Documentation,
        crate::ui::icons::SVG_DOCUMENT,
        "ドキュメント",
        false,
    );
    ui.add_space(31.0 * scale);
    let settings_top = if side.height() < 760.0 * scale {
        ui.cursor().top() + 16.0 * scale
    } else {
        side.top() + side.height() * 0.523
    };
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(side.left(), settings_top),
            egui::vec2(side.width(), row_height),
        )),
        |settings_ui| {
            landing_nav_row(
                settings_ui,
                ctx,
                current,
                HomeNav::Settings,
                crate::ui::icons::SVG_SETTINGS,
                "設定",
                false,
            );
        },
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(side.left(), settings_top + row_height),
            egui::vec2(side.width(), row_height),
        )),
        |exit_ui| {
            landing_nav_row(
                exit_ui,
                ctx,
                current,
                HomeNav::Compositing,
                crate::ui::icons::SVG_POWER,
                "終了",
                false,
            );
        },
    );
    ui.painter().text(
        egui::pos2(side.left() + 38.0 * scale, side.bottom() - 65.0 * scale),
        egui::Align2::LEFT_CENTER,
        concat!("Kagari VFX\nv", env!("CARGO_PKG_VERSION")),
        egui::FontId::proportional(12.0 * scale),
        crate::ui::theme::colors::TEXT_SECONDARY,
    );
}

pub(in crate::ui::home_screen) fn landing_button_rect(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    label: &str,
    icon: &'static str,
    accent: bool,
) -> egui::Response {
    let base_scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let scale = base_scale
        * (rect.width() / (208.0 * base_scale))
            .min(rect.height() / (56.0 * base_scale))
            .clamp(0.6, 1.0);
    let response = ui.interact(rect, ui.next_auto_id(), egui::Sense::click());
    let painter = ui.painter().with_clip_rect(rect);
    let fill = if accent {
        if response.hovered() {
            egui::Color32::from_rgb(255, 105, 48)
        } else {
            egui::Color32::from_rgb(255, 79, 22)
        }
    } else if response.hovered() {
        colors::BG_HOVER
    } else {
        egui::Color32::from_rgb(15, 23, 26)
    };
    painter.rect_filled(rect, 7.0 * scale, fill);
    painter.rect_stroke(
        rect,
        7.0 * scale,
        egui::Stroke::new(
            1.0 * scale,
            if accent { fill } else { colors::BORDER_MEDIUM },
        ),
    );
    let icon_size = 22.0 * scale;
    crate::ui::icons::render_svg_at(
        ui,
        format!("landing-cta-{label}"),
        icon,
        egui::vec2(icon_size, icon_size),
        egui::Color32::WHITE,
        egui::pos2(
            rect.left() + 18.0 * scale,
            rect.center().y - icon_size * 0.5,
        ),
    );
    let text_center = egui::pos2(rect.center().x + 9.0 * scale, rect.center().y);
    painter.text(
        text_center,
        egui::Align2::CENTER_CENTER,
        crate::ui::localization::tr(label),
        egui::FontId::proportional(14.0 * scale),
        if accent {
            egui::Color32::WHITE
        } else {
            colors::TEXT_PRIMARY
        },
    );
    response.on_hover_text(crate::ui::localization::tr(label))
}

pub(in crate::ui::home_screen) fn draw_landing_hero(ui: &mut egui::Ui, rect: egui::Rect) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let width_scale = (rect.width() / (1217.0 * scale)).max(0.45);
    let height_scale = (rect.height() / (300.0 * scale)).max(0.5);
    let painter = ui.painter().with_clip_rect(rect);
    let curve = |start: egui::Pos2, c1: egui::Pos2, c2: egui::Pos2, end: egui::Pos2| {
        (0..=48)
            .map(move |index| {
                let t = index as f32 / 48.0;
                let u = 1.0 - t;
                egui::pos2(
                    u * u * u * start.x
                        + 3.0 * u * u * t * c1.x
                        + 3.0 * u * t * t * c2.x
                        + t * t * t * end.x,
                    u * u * u * start.y
                        + 3.0 * u * u * t * c1.y
                        + 3.0 * u * t * t * c2.y
                        + t * t * t * end.y,
                )
            })
            .collect::<Vec<_>>()
    };
    let start = egui::pos2(
        rect.left() + rect.width() * 0.405,
        rect.bottom() + 15.0 * scale * height_scale,
    );
    let end = egui::pos2(
        rect.right() + 20.0 * scale,
        rect.top() + rect.height() * 0.267,
    );
    for (delta, width, color) in [
        (0.0_f32, 3.0_f32, egui::Color32::from_rgb(123, 43, 25)),
        (
            -95.0_f32 * scale * width_scale,
            12.0_f32,
            egui::Color32::from_rgb(181, 53, 22),
        ),
        (
            -210.0_f32 * scale * width_scale,
            40.0_f32,
            egui::Color32::from_rgb(255, 79, 22),
        ),
    ] {
        let points = curve(
            egui::pos2(start.x - delta, start.y),
            egui::pos2(
                rect.left() + rect.width() * 0.64 - delta,
                rect.top() + rect.height() * 0.44,
            ),
            egui::pos2(
                rect.left() + rect.width() * 0.78 - delta,
                rect.top() + rect.height() * 0.143,
            ),
            egui::pos2(end.x - delta * 0.16, end.y),
        );
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(
                width * scale * width_scale * height_scale,
                color.linear_multiply(0.72),
            ),
        ));
    }
}

pub(in crate::ui::home_screen) fn draw_landing_home(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let scale = ctx
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let viewport = ui.available_rect_before_wrap();
    let width = viewport.width();
    let height = viewport.height();
    let responsive_scale = (width / (1298.0 * scale)).max(0.72);
    let margin = (width * 0.032).clamp(
        18.0 * scale * responsive_scale,
        41.0 * scale * responsive_scale,
    );
    let max_content_width = 1217.0 * scale * responsive_scale;
    let inner_width = (width - margin * 2.0).clamp(1.0, max_content_width);
    let content_inset = ((width - inner_width) * 0.5).max(margin);
    let density = scale * responsive_scale * (inner_width / max_content_width).clamp(0.68, 1.0);
    let vertical = scale * (height / (993.0 * scale)).clamp(0.68, 1.3);
    let projects = recent_project_entries(ctx);
    let visible_project_count = projects.len().min(4);
    let card_gap = LANDING_CARD_GAP * density;
    let minimum_card_width = 274.0 * density;
    let mut columns = ((inner_width + card_gap) / (minimum_card_width + card_gap))
        .floor()
        .clamp(1.0, 4.0) as usize;
    if visible_project_count == 4 && columns == 3 {
        columns = 2;
    }
    let card_height = match columns {
        4 => 216.0 * density,
        3 => 200.0 * density,
        2 => 184.0 * density,
        _ => 164.0 * density,
    };
    let project_rows = visible_project_count.div_ceil(columns);
    let card_grid_height = if project_rows == 0 {
        card_height
    } else {
        project_rows as f32 * card_height + project_rows.saturating_sub(1) as f32 * 16.0 * density
    };
    let hero_height = (height * 0.30).clamp(150.0 * vertical, 300.0 * vertical);
    let header_top = hero_height - 3.0 * vertical;
    let stacked_header = inner_width < 850.0 * density;
    let stacked_buttons = inner_width < 470.0 * density;
    let button_height = if stacked_header {
        48.0 * density
    } else {
        56.0 * density
    };
    let cards_top = if stacked_header {
        header_top
            + 44.0 * vertical
            + button_height * if stacked_buttons { 2.0 } else { 1.0 }
            + if stacked_buttons {
                8.0 * vertical
            } else {
                18.0 * vertical
            }
    } else {
        header_top + button_height + 19.0 * vertical
    };
    let work_heading_y = cards_top + card_grid_height + 65.0 * vertical;
    let rows_top = work_heading_y + 44.0 * vertical;
    let work_rows = projects
        .iter()
        .filter(|project| project.path.is_file())
        .take(5)
        .count()
        .max(1);
    let content_height =
        (rows_top + work_rows as f32 * 44.0 * density + 64.0 * vertical).max(height);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(egui::vec2(width, content_height), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(content);
            painter.rect_filled(content, 0.0, egui::Color32::from_rgb(8, 15, 17));
            let left = content.left() + content_inset;
            let right = left + inner_width;
            let hero = egui::Rect::from_min_max(
                egui::pos2(content.left() + margin, content.top()),
                egui::pos2(content.right() - margin, content.top() + hero_height),
            );
            draw_landing_hero(ui, hero);
            let title_size = (inner_width * 0.044).clamp(30.0 * density, 53.0 * density) * 0.86;
            painter.text(
                egui::pos2(left, content.top() + hero_height * 0.633),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(title_size),
                colors::TEXT_PRIMARY,
            );

            if stacked_header {
                painter.text(
                    egui::pos2(left, content.top() + header_top + 14.0 * vertical),
                    egui::Align2::LEFT_CENTER,
                    "最近のプロジェクト",
                    egui::FontId::proportional(21.0 * density),
                    colors::TEXT_PRIMARY,
                );
            } else {
                painter.text(
                    egui::pos2(left, content.top() + header_top + button_height * 0.5),
                    egui::Align2::LEFT_CENTER,
                    "最近のプロジェクト",
                    egui::FontId::proportional(21.0 * density),
                    colors::TEXT_PRIMARY,
                );
            }

            let button_gap = 18.0 * density;
            let button_width = if stacked_header {
                if stacked_buttons {
                    inner_width
                } else {
                    ((inner_width - 10.0 * density) * 0.5).min(208.0 * density)
                }
            } else {
                208.0 * density
            };
            let button_y = if stacked_header {
                content.top() + header_top + 44.0 * vertical
            } else {
                content.top() + header_top
            };
            let new_button = if stacked_header {
                egui::Rect::from_min_size(
                    egui::pos2(left, button_y),
                    egui::vec2(button_width, button_height),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(right - button_width * 2.0 - button_gap, button_y),
                    egui::vec2(button_width, button_height),
                )
            };
            let open_button = if stacked_header {
                let open_y = button_y
                    + if stacked_buttons {
                        button_height + 8.0 * vertical
                    } else {
                        0.0
                    };
                egui::Rect::from_min_size(
                    egui::pos2(
                        if stacked_buttons {
                            left
                        } else {
                            left + button_width + 10.0 * density
                        },
                        open_y,
                    ),
                    egui::vec2(button_width, button_height),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(right - button_width, button_y),
                    egui::vec2(button_width, button_height),
                )
            };
            if landing_button_rect(
                ui,
                new_button,
                "新規プロジェクト",
                crate::ui::icons::SVG_FILE_PLUS,
                true,
            )
            .clicked()
            {
                set_home_nav(ctx, HomeNav::NewProject);
            }
            if landing_button_rect(
                ui,
                open_button,
                "プロジェクトを開く",
                crate::ui::icons::SVG_OPEN_FOLDER,
                false,
            )
            .clicked()
            {
                enter_studio_open_dialog(app);
            }

            let cards_rect = egui::Rect::from_min_size(
                egui::pos2(left, content.top() + cards_top),
                egui::vec2(inner_width, card_grid_height),
            );
            draw_landing_recent_projects(app, ui, cards_rect, &projects);

            painter.text(
                egui::pos2(left, content.top() + work_heading_y),
                egui::Align2::LEFT_CENTER,
                "最近の作業",
                egui::FontId::proportional(21.0 * density),
                colors::TEXT_PRIMARY,
            );
            let view_all_rect = egui::Rect::from_min_size(
                egui::pos2(
                    right - 126.0 * density,
                    content.top() + work_heading_y - 16.0 * density,
                ),
                egui::vec2(126.0 * density, 32.0 * density),
            );
            let view_all = ui.interact(
                view_all_rect,
                egui::Id::new("landing-view-all-projects"),
                egui::Sense::click(),
            );
            painter.text(
                view_all_rect.center() - egui::vec2(8.0 * density, 0.0),
                egui::Align2::CENTER_CENTER,
                "すべて表示",
                egui::FontId::proportional(13.0 * density),
                if view_all.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
            );
            crate::ui::icons::render_svg_at(
                ui,
                "landing-view-all-arrow".to_owned(),
                crate::ui::icons::SVG_ARROW_RIGHT,
                egui::vec2(14.0 * density, 14.0 * density),
                if view_all.hovered() {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                },
                egui::pos2(
                    view_all_rect.center().x + 39.0 * density,
                    view_all_rect.center().y - 7.0 * density,
                ),
            );
            if view_all.clicked() {
                set_home_nav(ctx, HomeNav::Projects);
            }
            draw_landing_recent_work(app, ui, left, right, content.top() + rows_top, &projects);
        });
}

fn draw_landing_recent_work(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    left: f32,
    right: f32,
    top: f32,
    projects: &[RecentProject],
) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let painter = ui.painter().clone();
    let entries = projects
        .iter()
        .filter(|project| project.path.is_file())
        .take(5);
    let mut open_path = None;
    let row_height = 44.0 * scale;
    let mut count = 0;
    for (index, project) in entries.enumerate() {
        count += 1;
        let row = egui::Rect::from_min_size(
            egui::pos2(left, top + index as f32 * row_height),
            egui::vec2(right - left, row_height),
        );
        let response = ui.interact(
            row,
            egui::Id::new(("landing-recent-work", index)),
            egui::Sense::click(),
        );
        if response.hovered() {
            painter.rect_filled(row, 3.0, colors::BG_HOVER);
        }
        painter.line_segment(
            [row.left_bottom(), row.right_bottom()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        crate::ui::icons::render_svg_at(
            ui,
            format!("landing-work-icon-{index}"),
            crate::ui::icons::SVG_COMPOSITION,
            egui::vec2(20.0 * scale, 20.0 * scale),
            colors::TEXT_SECONDARY_BRIGHT,
            egui::pos2(left + 10.0 * scale, row.center().y - 10.0 * scale),
        );
        let composition = project
            .summary
            .as_ref()
            .and_then(|summary| {
                summary.comp_names.get(
                    summary
                        .active_idx
                        .min(summary.comp_names.len().saturating_sub(1)),
                )
            })
            .map(String::as_str)
            .unwrap_or("プロジェクト");
        let compact = right - left < 680.0 * scale;
        if compact {
            let name_rect = egui::Rect::from_min_max(
                egui::pos2(left + 42.0 * scale, row.top()),
                egui::pos2(right - 94.0 * scale, row.bottom()),
            );
            let label = format!("{composition} · {}", project.name);
            ui.put(
                name_rect,
                egui::Label::new(
                    egui::RichText::new(label)
                        .size(13.0 * scale)
                        .color(colors::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
        } else {
            let project_rect = egui::Rect::from_min_max(
                egui::pos2(left + 68.0 * scale, row.top()),
                egui::pos2(left + (right - left) * 0.43, row.bottom()),
            );
            let comp_rect = egui::Rect::from_min_max(
                egui::pos2(left + (right - left) * 0.43, row.top()),
                egui::pos2(right - 132.0 * scale, row.bottom()),
            );
            ui.put(
                project_rect,
                egui::Label::new(
                    egui::RichText::new(composition)
                        .size(14.0 * scale)
                        .color(colors::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
            ui.put(
                comp_rect,
                egui::Label::new(
                    egui::RichText::new(&project.name)
                        .size(14.0 * scale)
                        .color(colors::TEXT_SECONDARY),
                )
                .truncate()
                .sense(egui::Sense {
                    click: false,
                    drag: false,
                    focusable: false,
                }),
            );
        }
        painter.text(
            egui::pos2(right - 66.0 * scale, row.center().y),
            egui::Align2::CENTER_CENTER,
            project.modified.map(fmt_age).unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(12.0 * scale),
            colors::TEXT_SECONDARY,
        );
        if response.clicked() {
            open_path = Some(project.path.clone());
        }
    }
    if count == 0 {
        let empty =
            egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(right - left, row_height));
        painter.line_segment(
            [empty.left_top(), empty.right_top()],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        painter.text(
            empty.left_center() + egui::vec2(14.0 * scale, 0.0),
            egui::Align2::LEFT_CENTER,
            "最近開いたプロジェクトはここに表示されます",
            egui::FontId::proportional(13.0 * scale),
            colors::TEXT_MUTED,
        );
    }
    if let Some(path) = open_path {
        open_project_path(app, &path);
    }
}

pub(in crate::ui::home_screen) fn draw_landing_recent_projects(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    rect: egui::Rect,
    entries: &[RecentProject],
) {
    let scale = ui
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let density = scale * (rect.width() / (1217.0 * scale)).max(0.68);
    let painter = ui.painter().with_clip_rect(rect);
    if entries.is_empty() {
        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(13, 22, 25));
        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "プロジェクトを作成するか、既存のプロジェクトを開いてください",
            egui::FontId::proportional(14.0 * density),
            colors::TEXT_MUTED,
        );
        return;
    }

    let gap = LANDING_CARD_GAP * density;
    let minimum_card_width = 274.0 * density;
    let mut columns = ((rect.width() + gap) / (minimum_card_width + gap))
        .floor()
        .clamp(1.0, 4.0) as usize;
    if entries.len().min(4) == 4 && columns == 3 {
        columns = 2;
    }
    let card_width =
        ((rect.width() - gap * (columns - 1) as f32) / columns as f32).max(150.0 * density);
    let card_height = match columns {
        4 => 216.0 * density,
        3 => 200.0 * density,
        2 => 184.0 * density,
        _ => 164.0 * density,
    };
    let row_gap = 16.0 * density;
    let cover_height = (card_height * 0.54).clamp(96.0 * density, 120.0 * density);

    for (index, project) in entries.iter().take(4).enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = rect.left() + column as f32 * (card_width + gap);
        let y = rect.top() + row as f32 * (card_height + row_gap);
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_width, card_height));
        let response = ui.interact(
            card,
            egui::Id::new(("landing-recent-project-card", index)),
            egui::Sense::click(),
        );
        painter.rect_filled(card, 6.0, egui::Color32::from_rgb(16, 25, 28));
        painter.rect_stroke(
            card,
            6.0,
            egui::Stroke::new(
                if response.hovered() {
                    1.5_f32 * density
                } else {
                    1.0_f32 * density
                },
                if response.hovered() {
                    colors::ACCENT_BRAND
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );

        let cover = egui::Rect::from_min_size(card.min, egui::vec2(card_width, cover_height));
        let preview = project
            .summary
            .as_ref()
            .and_then(|summary| summary.preview_path.as_deref())
            .map(std::path::PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    project
                        .path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join(path)
                }
            })
            .filter(|path| path.is_file())
            .and_then(|path| thumb_for(app, ui.ctx(), &path));
        if let Some(texture) = preview {
            painter.image(
                texture,
                cover,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            draw_project_cover(&painter, cover, index, &project.name);
        }

        let name_rect = egui::Rect::from_min_max(
            egui::pos2(card.left() + 14.0 * density, cover.bottom() + 7.0 * density),
            egui::pos2(
                card.right() - 18.0 * density,
                cover.bottom() + 31.0 * density,
            ),
        );
        painter.with_clip_rect(name_rect).text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            &project.name,
            egui::FontId::proportional(14.0 * density),
            colors::TEXT_PRIMARY,
        );
        let summary = project.summary.as_ref();
        let metadata = summary
            .map(|summary| {
                format!(
                    "{}コンポジション・{}レイヤー",
                    summary.comp_names.len(),
                    summary.layers
                )
            })
            .unwrap_or_else(|| "Kagari プロジェクト".to_string());
        painter.text(
            egui::pos2(
                card.left() + 14.0 * density,
                cover.bottom() + 48.0 * density,
            ),
            egui::Align2::LEFT_CENTER,
            metadata,
            egui::FontId::proportional(11.0 * density),
            colors::TEXT_SECONDARY,
        );
        painter.text(
            egui::pos2(
                card.left() + 14.0 * density,
                cover.bottom() + 72.0 * density,
            ),
            egui::Align2::LEFT_CENTER,
            project.modified.map(fmt_age).unwrap_or_else(|| "—".into()),
            egui::FontId::proportional(11.0 * density),
            colors::TEXT_MUTED,
        );
        if response.clicked() {
            app.home_selected = Some(project.path.clone());
            open_project_path(app, &project.path);
        }
    }
}

fn draw_project_cover(painter: &egui::Painter, rect: egui::Rect, index: usize, name: &str) {
    let painter = painter.with_clip_rect(rect);
    let seed = name.bytes().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(33).wrapping_add(u32::from(byte))
    });
    let scale = painter
        .ctx()
        .native_pixels_per_point()
        .unwrap_or(1.0)
        .recip()
        .clamp(0.5, 1.0);
    let palettes = [
        (
            egui::Color32::from_rgb(13, 22, 25),
            egui::Color32::from_rgb(255, 79, 22),
        ),
        (
            egui::Color32::from_rgb(18, 23, 25),
            egui::Color32::from_rgb(159, 86, 60),
        ),
        (
            egui::Color32::from_rgb(24, 23, 22),
            egui::Color32::from_rgb(166, 125, 96),
        ),
        (
            egui::Color32::from_rgb(16, 23, 27),
            egui::Color32::from_rgb(100, 119, 132),
        ),
    ];
    let (base, accent) = palettes[index % palettes.len()];
    painter.rect_filled(rect, 6.0, base);
    for wave in 0..2 {
        let mut points = Vec::new();
        for step in 0..=32 {
            let t = step as f32 / 32.0;
            let phase = ((seed >> (wave * 5)) & 31) as f32 * 0.025;
            let x = rect.left() + t * rect.width();
            let y = rect.top() + rect.height() * (0.64 - wave as f32 * 0.09)
                - (t * 3.6 + phase).sin().abs() * rect.height() * (0.3 + wave as f32 * 0.08);
            points.push(egui::pos2(x, y));
        }
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(
                if wave == 0 { 2.0_f32 } else { 1.0_f32 } * scale,
                accent.linear_multiply(if wave == 0 { 0.35 } else { 0.16 }),
            ),
        ));
    }
}

pub(in crate::ui::home_screen) fn draw_reference_project_row(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    compact: bool,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Recent Projects").size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if reference_view_all_button(ui, "View All") {
                set_home_nav(ctx, HomeNav::Projects);
            }
        });
    });
    ui.add_space(if compact { 7.0 } else { 10.0 });
    let projects = recent_project_entries(ctx);
    if projects.is_empty() {
        let empty_height = if compact { 154.0 } else { 228.0 };
        let empty = fixed_card_with_inset(
            ui,
            egui::vec2(ui.available_width(), empty_height),
            egui::Color32::TRANSPARENT,
            crate::ui::theme::colors::BORDER_MEDIUM,
            7.0,
            if compact { 12.0 } else { 22.0 },
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(if compact { 20.0 } else { 34.0 });
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        "empty-project-folder",
                        crate::ui::icons::SVG_FOLDER,
                        egui::vec2(
                            if compact { 30.0 } else { 42.0 },
                            if compact { 30.0 } else { 42.0 },
                        ),
                        colors::TEXT_SECONDARY,
                    );
                    ui.add_space(if compact { 8.0 } else { 12.0 });
                    ui.label(
                        egui::RichText::new("まだプロジェクトはありません")
                            .size(if compact { 14.0 } else { 19.0 })
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("新規プロジェクトを作成して、はじめましょう。")
                            .size(if compact { 10.0 } else { 14.0 })
                            .color(colors::TEXT_SECONDARY),
                    );
                    ui.add_space(if compact { 10.0 } else { 16.0 });
                    if reference_cta_button(ui, "新規プロジェクト", compact).clicked() {
                        enter_studio_new_project(app);
                        app.show_new_comp_dialog = true;
                    }
                });
            },
        );
        let _ = empty;
        return;
    }
    let available_width = ui.available_width();
    let columns = if available_width < 680.0 {
        1.0
    } else if available_width < 1040.0 {
        2.0
    } else {
        4.0
    };
    let card_gap = if columns == 1.0 {
        0.0
    } else if compact {
        12.0
    } else {
        18.0
    };
    let card_width = ((available_width - card_gap * (columns - 1.0)) / columns).max(140.0);
    let card_height = if compact { 150.0 } else { 215.0 };
    let image_height = if compact { 74.0 } else { 124.0 };
    let project_inset = if compact { 10.0 } else { 20.0 };
    let thumbnails = [
        "recent_project_eclipse.webp",
        "recent_project_citadel.webp",
        "recent_project_rift.webp",
        "recent_project_atlas.webp",
    ];
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (card_index, project) in projects.iter().take(4).enumerate() {
            let asset = thumbnails[card_index % thumbnails.len()];
            let title = project.name.as_str();
            let meta = project
                .summary
                .as_ref()
                .and_then(|summary| summary.comp_names.get(summary.active_idx))
                .map(String::as_str)
                .unwrap_or("Project");
            let age = project
                .modified
                .map(fmt_age)
                .unwrap_or_else(|| "Not found".to_string());
            let response = fixed_card_with_inset(
                ui,
                egui::vec2(card_width, card_height),
                crate::ui::theme::colors::BG_PANEL_RAISED,
                crate::ui::theme::colors::BG_ELEVATED,
                5.0,
                0.0,
                |ui| {
                    if let Some(id) = reference_texture(app, ctx, asset) {
                        reference_cover_image(
                            ui,
                            id,
                            egui::vec2(card_width, image_height),
                            316.0 / 121.0,
                        );
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(egui::RichText::new(title).size(16.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(project_inset);
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                &format!("{title}-more"),
                                crate::ui::icons::SVG_MORE,
                                egui::vec2(15.0, 15.0),
                                colors::TEXT_SECONDARY,
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(
                            egui::RichText::new(meta)
                                .size(12.0)
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(project_inset);
                        ui.label(
                            egui::RichText::new(&age)
                                .size(12.0)
                                .color(colors::TEXT_MUTED),
                        );
                    });
                },
            );
            if response.clicked() {
                open_project_path(app, &project.path);
            }
            if card_index < 3 {
                ui.add_space(card_gap);
            }
        }
    });
}

pub(in crate::ui::home_screen) fn draw_reference_templates(
    ui: &mut egui::Ui,
    compact: bool,
    show_header: bool,
) {
    let narrow = reference_narrow(ui);
    if show_header {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Templates").size(18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if reference_view_all_button(ui, "View All") {
                    set_home_nav(ui.ctx(), HomeNav::Templates);
                }
            });
        });
        ui.add_space(if compact { 7.0 } else { 9.0 });
    }
    let available_width = ui.available_width();
    let columns = if available_width < 560.0 {
        2.0
    } else if available_width < 980.0 {
        3.0
    } else {
        6.0
    };
    let template_gap = if narrow {
        6.0
    } else if compact {
        4.0
    } else {
        15.0
    };
    let width_gap = if columns == 6.0 { 15.0 } else { template_gap };
    let card_width = (((available_width - width_gap * (columns - 1.0)) / columns)
        - if columns == 6.0 { 3.0 } else { 0.0 })
    .max(72.0);
    let card_height = if narrow {
        76.0
    } else if compact {
        116.0
    } else {
        166.0
    };
    let illustration_height = if narrow {
        34.0
    } else if compact {
        38.0
    } else {
        78.0
    };
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.horizontal_wrapped(|ui| {
        for (index, (icon, title, desc)) in [
            (
                crate::ui::icons::SVG_KEYING,
                "Keying Setup",
                "Green screen workflow",
            ),
            (
                crate::ui::icons::SVG_TRACKING,
                "Tracking Setup",
                "Planar / 3D tracking",
            ),
            (
                crate::ui::icons::SVG_CLEAN_PLATE,
                "Clean Plate Setup",
                "Remove objects",
            ),
            (
                crate::ui::icons::SVG_PARTICLES,
                "Particle Composite",
                "Sparks, smoke, fire",
            ),
            (
                crate::ui::icons::SVG_TITLE,
                "Title Animation",
                "Motion graphics",
            ),
            (
                crate::ui::icons::SVG_SCREEN,
                "Screen Replacement",
                "Compositing workflow",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let response = fixed_card_with_inset(
                ui,
                egui::vec2(card_width, card_height),
                crate::ui::theme::colors::BG_PANEL_RAISED,
                crate::ui::theme::colors::BG_ELEVATED,
                5.0,
                if narrow {
                    6.0
                } else if compact {
                    10.0
                } else {
                    18.0
                },
                |ui| {
                    ui.vertical(|ui| {
                        let illustration_width = illustration_height * 80.0 / 48.0;
                        ui.horizontal(|ui| {
                            ui.add_space(
                                ((ui.available_width() - illustration_width) * 0.5).max(0.0),
                            );
                            crate::ui::icons::render_svg_bytes(
                                ui,
                                title,
                                icon,
                                egui::vec2(illustration_width, illustration_height),
                                egui::Color32::WHITE,
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(if narrow { 0.0 } else { 6.0 });
                            ui.label(
                                egui::RichText::new(title)
                                    .size(if narrow {
                                        10.0
                                    } else if compact {
                                        12.0
                                    } else {
                                        16.0
                                    })
                                    .strong(),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(if narrow { 0.0 } else { 6.0 });
                            ui.label(
                                egui::RichText::new(desc)
                                    .size(if narrow {
                                        8.0
                                    } else if compact {
                                        10.0
                                    } else {
                                        13.0
                                    })
                                    .color(colors::TEXT_SECONDARY),
                            );
                        });
                    });
                },
            );
            if show_header && response.clicked() {
                set_home_nav(ui.ctx(), HomeNav::Templates);
            }
            if index < 5 {
                ui.add_space(template_gap);
            }
        }
    });
}
