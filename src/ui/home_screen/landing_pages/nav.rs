use super::*;

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
