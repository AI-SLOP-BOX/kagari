use super::*;

pub(in crate::ui::home_screen) fn draw_reference_effects_narrow(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    let top = ui.cursor().min;
    let left = egui::Rect::from_min_size(top, egui::vec2(77.0, 248.0));
    let center = egui::Rect::from_min_size(
        egui::pos2(left.right() + 2.0, top.y),
        egui::vec2(143.0, 248.0),
    );
    let right = egui::Rect::from_min_size(
        egui::pos2(center.right() + 2.0, top.y),
        egui::vec2(105.0, 248.0),
    );
    let fill = crate::ui::theme::colors::BG_PANEL_ALT;
    let stroke = egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_SUBTLE);
    for rect in [left, center, right] {
        ui.painter().rect(rect, 4.0, fill, stroke);
    }
    let category_id = egui::Id::new("reference-effect-narrow-category");
    let mut selected_category = ctx
        .data_mut(|d| d.get_temp::<usize>(category_id))
        .unwrap_or(2);
    ui.painter().text(
        egui::pos2(left.left() + 6.0, left.top() + 11.0),
        egui::Align2::LEFT_CENTER,
        "Effects",
        egui::FontId::proportional(10.0),
        colors::TEXT_PRIMARY,
    );
    let search = egui::Rect::from_min_size(
        egui::pos2(left.left() + 6.0, left.top() + 27.0),
        egui::vec2(65.0, 20.0),
    );
    ui.painter().rect(
        search,
        3.0,
        crate::ui::theme::colors::BG_DEEPEST,
        egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_MEDIUM),
    );
    ui.painter().text(
        egui::pos2(search.left() + 6.0, search.center().y),
        egui::Align2::LEFT_CENTER,
        "⌕  Search",
        egui::FontId::proportional(6.5),
        colors::TEXT_SECONDARY,
    );
    for (index, (icon, label)) in [
        (crate::ui::icons::SVG_STAR, "Favorites"),
        (crate::ui::icons::SVG_CLOCK, "Recent"),
        (crate::ui::icons::SVG_PALETTE, "Color"),
        (crate::ui::icons::SVG_LIGHT, "Distort"),
        (crate::ui::icons::SVG_KEYING, "Keying"),
        (crate::ui::icons::SVG_CLEAN_PLATE, "Blur"),
        (crate::ui::icons::SVG_PARTICLES, "Generate"),
        (crate::ui::icons::SVG_TEMPLATE_TEXT, "Stylize"),
        (crate::ui::icons::SVG_ARROW_RIGHT, "Transition"),
        (crate::ui::icons::SVG_LAYERS, "Utility"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = left.top() + 53.0 + index as f32 * 18.0;
        let row = egui::Rect::from_min_size(
            egui::pos2(left.left() + 5.0, y - 1.0),
            egui::vec2(67.0, 17.0),
        );
        let active = selected_category == index;
        if active {
            ui.painter().rect(
                row,
                3.0,
                crate::ui::theme::colors::BG_ACTIVE,
                egui::Stroke::NONE,
            );
        }
        if ui
            .interact(
                row,
                egui::Id::new(("reference-effect-narrow-category", index)),
                egui::Sense::click(),
            )
            .clicked()
        {
            selected_category = index;
        }
        ui.allocate_new_ui(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                egui::pos2(left.left() + 8.0, y + 3.0),
                egui::vec2(9.0, 9.0),
            )),
            |icon_ui| {
                crate::ui::icons::render_svg_bytes(
                    icon_ui,
                    &format!("effects-category-{index}"),
                    icon,
                    egui::vec2(9.0, 9.0),
                    if active {
                        colors::TEXT_PRIMARY
                    } else {
                        colors::TEXT_SECONDARY
                    },
                );
            },
        );
        ui.painter().text(
            egui::pos2(left.left() + 21.0, y + 7.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(6.5),
            if active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
    }
    ctx.data_mut(|d| d.insert_temp(category_id, selected_category));
    ui.painter().text(
        egui::pos2(center.left() + 6.0, center.top() + 12.0),
        egui::Align2::LEFT_CENTER,
        "Lumetri Color",
        egui::FontId::proportional(8.0),
        colors::TEXT_PRIMARY,
    );
    for (offset, label, width) in [(31.0, "Fit", 27.0), (61.0, "Full", 31.0)] {
        let button = egui::Rect::from_min_size(
            egui::pos2(center.left() + offset, center.top() + 5.0),
            egui::vec2(width, 20.0),
        );
        ui.painter().rect(
            button,
            3.0,
            crate::ui::theme::colors::BG_DARK,
            egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BG_ELEVATED),
        );
        ui.painter().text(
            button.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(6.0),
            colors::TEXT_PRIMARY,
        );
        if ui
            .interact(
                button,
                egui::Id::new(("reference-effect-narrow-view", label)),
                egui::Sense::click(),
            )
            .clicked()
        {
            ctx.data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("reference-effect-narrow-view-mode"),
                    label.to_string(),
                )
            });
        }
    }
    if let Some(id) = reference_texture(app, ctx, "recent_project_rift.webp") {
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(center.left() + 2.0, center.top() + 44.0),
                egui::vec2(135.0, 170.0),
            ),
            egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(135.0, 170.0)))
                .fit_to_exact_size(egui::vec2(135.0, 170.0)),
        );
    }
    ui.painter().text(
        egui::pos2(center.left() + 6.0, center.top() + 214.0),
        egui::Align2::LEFT_CENTER,
        "00:00:00:00",
        egui::FontId::proportional(6.5),
        colors::ACCENT_CYAN,
    );
    ui.painter().text(
        egui::pos2(right.left() + 6.0, right.top() + 11.0),
        egui::Align2::LEFT_CENTER,
        "Recent Controls",
        egui::FontId::proportional(8.0),
        colors::TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::pos2(right.left() + 6.0, right.top() + 35.0),
        egui::Align2::LEFT_CENTER,
        "Lumetri Color",
        egui::FontId::proportional(6.5),
        colors::TEXT_SECONDARY,
    );
    let reset = egui::Rect::from_min_size(
        egui::pos2(right.right() - 30.0, right.top() + 25.0),
        egui::vec2(24.0, 16.0),
    );
    ui.painter().text(
        reset.right_center(),
        egui::Align2::RIGHT_CENTER,
        "Reset",
        egui::FontId::proportional(5.5),
        colors::ACCENT_CYAN,
    );
    if ui
        .interact(
            reset,
            egui::Id::new("reference-effect-narrow-reset"),
            egui::Sense::click(),
        )
        .clicked()
    {
        for (label, default) in [
            ("Exposure", 0.0_f32),
            ("Contrast", 0.0),
            ("Highlights", -23.3),
            ("Shadows", 0.2),
            ("Whites", 0.0),
            ("Blacks", -0.2),
        ] {
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new(("reference_effect_value", label)), default)
            });
        }
    }
    for (index, (label, value)) in [
        ("Basic Correction", ""),
        ("Input LUT", "None"),
        ("Exposure", "+0.0"),
        ("Contrast", "+0.0"),
        ("Highlights", "+0.0"),
        ("Shadows", "+0.0"),
        ("Whites", "+0.0"),
        ("Blacks", "+0.0"),
        ("Creative", ""),
        ("Curves", ""),
        ("Color Wheels & Match", ""),
        ("HSL Secondary", ""),
        ("Vignette", ""),
    ]
    .into_iter()
    .enumerate()
    {
        let y = right.top() + 58.0 + index as f32 * 14.0;
        ui.painter().text(
            egui::pos2(right.left() + 6.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(5.7),
            colors::TEXT_SECONDARY,
        );
        if value.is_empty() {
            ui.painter().text(
                egui::pos2(right.right() - 7.0, y),
                egui::Align2::RIGHT_CENTER,
                "›",
                egui::FontId::proportional(8.0),
                colors::TEXT_MUTED,
            );
        } else {
            ui.painter().text(
                egui::pos2(right.right() - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                value,
                egui::FontId::proportional(5.7),
                colors::TEXT_SECONDARY,
            );
        }
    }
    ui.allocate_space(egui::vec2(322.0, 248.0));
}
