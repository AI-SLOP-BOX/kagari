use super::*;

pub(super) fn draw_reference_topbar(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let narrow = reference_narrow(ui) || reference_medium(ui);
    let ultra_narrow = narrow && ui.available_width() < 270.0;
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(
            if ultra_narrow {
                4.0
            } else if narrow {
                12.0
            } else {
                0.0
            },
            0.0,
        ))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = if ultra_narrow {
                3.0
            } else if narrow {
                5.0
            } else {
                8.0
            };
            let nav_items = [
                (HomeNav::Home, "Home"),
                (HomeNav::Compositing, "Compositing"),
                (HomeNav::Effects, "Effects"),
                (HomeNav::Projects, "Assets"),
                (HomeNav::Render, "Render"),
            ];
            let active_top_nav = match home_nav(ctx) {
                HomeNav::NewProject => HomeNav::Projects,
                HomeNav::Settings => HomeNav::Effects,
                nav => nav,
            };
            let draw_brand = |ui: &mut egui::Ui, app: &mut KagariApp, ctx: &egui::Context| {
                if app.home_banner.is_none() {
                    if let Ok(img) = crate::ui::embedded_assets::open_image(
                        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("assets/kagari_logo.webp"),
                    ) {
                        app.home_banner = load_logo_texture(ctx, img);
                    }
                }
                if let Some(texture) = app.home_banner.as_ref() {
                    let logo_size = if ultra_narrow {
                        14.0
                    } else if narrow {
                        16.0
                    } else {
                        20.0
                    };
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        texture.id(),
                        egui::vec2(logo_size, logo_size),
                    )));
                }
                ui.label(
                    egui::RichText::new("Kagari VFX")
                        .size(if ultra_narrow {
                            7.5
                        } else if narrow {
                            9.0
                        } else {
                            12.0
                        })
                        .color(colors::TEXT_PRIMARY),
                );
            };
            let draw_account_actions = |ui: &mut egui::Ui| {
                let profile_size = if narrow { 16.0 } else { 24.0 };
                let settings_size = if narrow { 15.0 } else { 18.0 };
                crate::ui::icons::render_svg_bytes(
                    ui,
                    "reference-profile",
                    crate::ui::icons::SVG_PROFILE,
                    egui::vec2(profile_size, profile_size),
                    crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT,
                );
                ui.add_space(if narrow { 6.0 } else { 10.0 });
                if crate::ui::icons::render_svg_bytes(
                    ui,
                    "reference-settings",
                    crate::ui::icons::SVG_SETTINGS,
                    egui::vec2(settings_size, settings_size),
                    colors::TEXT_SECONDARY,
                )
                .clicked()
                {
                    set_home_nav(ctx, HomeNav::Settings);
                }
            };
            let draw_window_actions = |ui: &mut egui::Ui| {
                for (index, icon) in [
                    crate::ui::icons::SVG_WINDOW_MINIMIZE,
                    crate::ui::icons::SVG_WINDOW_MAXIMIZE,
                    crate::ui::icons::SVG_WINDOW_CLOSE,
                ]
                .into_iter()
                .enumerate()
                {
                    crate::ui::icons::render_svg_bytes(
                        ui,
                        &format!("reference-window-action-{index}"),
                        icon,
                        egui::vec2(
                            if ultra_narrow { 9.0 } else { 11.0 },
                            if ultra_narrow { 9.0 } else { 11.0 },
                        ),
                        colors::TEXT_SECONDARY,
                    );
                    if index < 2 {
                        ui.add_space(if ultra_narrow { 2.0 } else { 4.0 });
                    }
                }
            };

            if narrow {
                ui.horizontal(|ui| {
                    draw_brand(ui, app, ctx);
                    ui.add_space(if ultra_narrow { 5.0 } else { 8.0 });
                    for (nav, label) in nav_items {
                        let active = active_top_nav == nav;
                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(crate::ui::localization::tr(label))
                                    .size(if ultra_narrow { 6.0 } else { 7.0 })
                                    .color(if active {
                                        colors::TEXT_PRIMARY
                                    } else {
                                        colors::TEXT_SECONDARY
                                    }),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if active {
                            ui.painter().line_segment(
                                [
                                    egui::pos2(response.rect.left(), response.rect.bottom() - 1.0),
                                    egui::pos2(response.rect.right(), response.rect.bottom() - 1.0),
                                ],
                                egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION),
                            );
                        }
                        if response.clicked() {
                            set_home_nav(ctx, nav);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        draw_window_actions(ui);
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    draw_brand(ui, app, ctx);
                    ui.add_space(28.0);
                    for (nav, label) in nav_items {
                        let active = active_top_nav == nav;
                        let response = crate::ui::theme::draw_custom_tab(
                            ui,
                            active,
                            &crate::ui::localization::tr(label),
                        );
                        if response.clicked() {
                            set_home_nav(ctx, nav);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        draw_account_actions(ui);
                    });
                });
            }
        });
}

#[path = "documentation_pages.rs"]
mod documentation_pages;
#[path = "effect_narrow_pages.rs"]
mod effect_narrow_pages;
#[path = "effect_render_pages.rs"]
mod effect_render_pages;
#[path = "landing_pages.rs"]
mod landing_pages;
#[path = "project_pages.rs"]
mod project_pages;
#[path = "reference_home_pages.rs"]
mod reference_home_pages;
#[path = "template_settings_pages.rs"]
mod template_settings_pages;

pub(super) use documentation_pages::*;
pub(super) use effect_narrow_pages::*;
pub(super) use effect_render_pages::*;
pub(super) use landing_pages::*;
pub(super) use project_pages::*;
pub(super) use reference_home_pages::*;
pub(super) use template_settings_pages::*;
