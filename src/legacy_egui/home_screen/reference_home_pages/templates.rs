use super::*;

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
