use crate::ui::theme::colors;
use crate::KagariApp;
use crate::ViewportMode;
use eframe::egui;

use super::view_menu::draw_view_menu_contents;

pub(super) fn draw_viewport_toolbar(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.allocate_ui_with_layout(
        egui::vec2(
            ui.available_width(),
            crate::ui::theme::layout::TOOLBAR_HEIGHT,
        ),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let narrow_toolbar = ui.available_width() < 1180.0;
            crate::ui::icons::render_svg_bytes(
                ui,
                "viewport-composition",
                crate::ui::icons::SVG_FRAME,
                egui::vec2(14.0, 14.0),
                colors::TEXT_SECONDARY,
            );
            ui.label(
                egui::RichText::new("ビューアー")
                    .strong()
                    .color(colors::TEXT_PRIMARY),
            );
            let active_comp = app.history.current().active_composition();
            if active_comp.id != "comp_empty_start" {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(active_comp.name.as_str()).color(colors::ACCENT_BRAND),
                );
                ui.label(
                    egui::RichText::new("●")
                        .size(8.0)
                        .color(colors::ACCENT_BRAND),
                );
            }
            ui.add_space(6.0);
            ui.separator();
            let zoom_label = match app.ui_tabs.viewport_mag_ratio {
                ratio if ratio <= 0.001 => crate::ui::localization::tr("Fit"),
                ratio if ratio <= 0.26 => "25%".to_owned(),
                ratio if ratio <= 0.76 => "50%".to_owned(),
                ratio if ratio <= 1.26 => "100%".to_owned(),
                _ => "200%".to_owned(),
            };
            egui::ComboBox::from_id_salt("viewport-zoom")
                .selected_text(&zoom_label)
                .width(72.0)
                .show_ui(ui, |ui| {
                    for (label, ratio) in [
                        (crate::ui::localization::tr("Fit"), 0.0),
                        ("25%".to_owned(), 0.25),
                        ("50%".to_owned(), 0.5),
                        ("100%".to_owned(), 1.0),
                        ("200%".to_owned(), 2.0),
                    ] {
                        if ui.selectable_label(zoom_label == label, &label).clicked() {
                            app.ui_tabs.viewport_mag_ratio = ratio;
                            if ratio == 0.0 {
                                app.playback.viewport_pan = egui::Vec2::ZERO;
                            }
                        }
                    }
                });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            let mode_2d = app.viewport_mode == ViewportMode::Comp2D;
            if narrow_toolbar {
                ui.menu_button(if mode_2d { "2D ▾" } else { "3D ▾" }, |ui| {
                    if crate::ui::kui::menu_item(ui, "2D コンポジション").clicked() {
                        app.viewport_mode = ViewportMode::Comp2D;
                        ui.close_menu();
                    }
                    if crate::ui::kui::menu_item(ui, "3D カメラ").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        ui.close_menu();
                    }
                });
            } else {
                if crate::ui::theme::draw_custom_tab(ui, mode_2d, "2D").clicked() {
                    app.viewport_mode = ViewportMode::Comp2D;
                }
                if crate::ui::theme::draw_custom_tab(ui, !mode_2d, "3Dカメラ").clicked() {
                    app.viewport_mode = ViewportMode::Camera3D;
                }
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.menu_button("表示", |ui| {
                draw_view_menu_contents(ui, app);
                if narrow_toolbar {
                    ui.separator();
                    if ui
                        .button(if app.viewer_maximized {
                            "Restore panels"
                        } else {
                            "Maximize Viewer"
                        })
                        .clicked()
                    {
                        app.viewer_maximized = !app.viewer_maximized;
                        ui.close_menu();
                    }
                }
            });
            if !narrow_toolbar {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (button_rect, button_response) =
                        ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                    if button_response.hovered() {
                        ui.painter().rect_filled(button_rect, 2.0, colors::BG_HOVER);
                    }
                    crate::ui::icons::render_svg_at(
                        ui,
                        "viewport-fullscreen".to_string(),
                        crate::ui::icons::SVG_WINDOW_MAXIMIZE,
                        egui::vec2(16.0, 16.0),
                        colors::TEXT_SECONDARY,
                        button_rect.center() - egui::vec2(8.0, 8.0),
                    );
                    if button_response.clicked() {
                        app.viewer_maximized = !app.viewer_maximized;
                    }
                    button_response.on_hover_text(if app.viewer_maximized {
                        "Restore panels"
                    } else {
                        "Maximize Viewer"
                    });
                });
            }
        },
    );
}
