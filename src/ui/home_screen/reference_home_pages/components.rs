use super::*;

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
