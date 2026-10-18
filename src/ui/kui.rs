//! Kagari UI Design System (kui)
//!
//! Submodules:
//! - `typography_layout`: Typography, badges, empty state, containers, and field scopes.
//! - `buttons`: Button styles, icon buttons, segmented buttons, breadcrumbs.
//! - `inputs`: NumberField, text fields, file inputs, switch/toggle, combo, PropertyGrid.
//! - `sliders_vectors`: Parameter sliders, scalar sliders, vector coordinate inputs.
//! - `color_meters`: Color edit buttons, color swatches, level meters, and vertical faders.

pub mod buttons;
pub mod color_meters;
pub mod hotspot;
pub mod inputs;
pub mod sliders_vectors;
pub mod typography_layout;

// Re-export hotspot & micro actions
pub use hotspot::{hotspot, micro_tag, svg_drag_value, svg_hotspot, HotspotResponse};

// Re-export typography & layout
pub use typography_layout::{
    body, caption, collapsing_section, divider, empty_state, error_text, field_scope, group, hint,
    icon_glyph, item_title, panel, readout, search_field, search_field_sized, secondary, section,
    section_title, selectable_item_row, separator, shortcut_hint, space, status, title, toolbar,
    toolbar_group, toolbar_separator, truncated_primary, truncated_secondary, StatusKind,
};

// Re-export buttons & actions
pub use buttons::{
    badge, badge_button, breadcrumb_current, breadcrumb_link, breadcrumb_separator, button,
    button_enabled, button_enabled_min_size, button_min_size, combo_popup, icon_button,
    icon_label_button_at, menu_button, menu_item, segmented_button, selectable_row,
    svg_icon_button, tool_button, with_button_style, ButtonKind,
};

// Re-export input widgets & property grid
pub use inputs::{
    combo, combo_with_selected, drag, file_field, integer, number, property, switch, text_field,
    text_field_with_hint, toggle, NumberField, PropertyGrid,
};

// Re-export sliders & vectors
pub use sliders_vectors::{
    parameter_slider, parameter_slider_impl, parameter_slider_step, scalar_slider, slider,
    slider_integer, slider_step, slider_step_without_value, slider_without_value, vector,
    vector_axes, vector_axes_unbounded,
};

// Re-export color & audio fader
pub use color_meters::{
    color, color_rgb, color_rgba, color_swatch, level_meter, progress_bar, vertical_fader,
    vertical_meter,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::colors;
    use eframe::egui;

    #[test]
    fn typed_and_vector_fields_keep_values_and_fit_narrow_panels() {
        for width in [180.0, 360.0, 600.0] {
            let ctx = egui::Context::default();
            let mut integer = 1920_u32;
            let mut point = [-1234.0, 0.0, 1000.0];
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 500.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let bounds = ui.max_rect();
                        let integer_response =
                            ui.add(drag(&mut integer).range(16..=16384).unit(" px"));
                        assert!(bounds.contains_rect(integer_response.rect));
                        let vector_response = vector_axes(
                            ui,
                            &mut point,
                            -10000.0..=10000.0,
                            1.0,
                            "°",
                            ["P", "Y", "R"],
                        );
                        assert!(bounds.contains_rect(vector_response.rect));
                        let mut gain = 0.0;
                        let fader_response = vertical_fader(ui, &mut gain, -60.0..=12.0, 108.0);
                        assert!(bounds.contains_rect(fader_response.rect));
                        assert!((fader_response.rect.height() - 108.0).abs() < 0.1);
                        assert!(
                            !integer_response.changed()
                                && !vector_response.changed()
                                && !fader_response.changed()
                        );
                    });
                },
            );
            assert_eq!(integer, 1920);
            assert_eq!(point, [-1234.0, 0.0, 1000.0]);
        }
    }

    #[test]
    fn property_controls_align_and_stay_inside_panel() {
        for width in [180.0, 320.0, 600.0] {
            for scale in [1.0, 2.0] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let mut rects = Vec::new();
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 600.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let bounds = ui.max_rect();
                            let grid = PropertyGrid::new(ui, &["Focus Distance", "焦点距離"]);
                            let mut value = 500.0;
                            for label in ["Focus Distance", "焦点距離"] {
                                let response = grid.row(ui, label, |ui| {
                                    number(ui, &mut value, 0.0..=10000.0, 1.0, " px")
                                });
                                assert!(
                                    bounds.contains_rect(response.rect),
                                    "{width} / {scale}: {:?}",
                                    response.rect
                                );
                                rects.push(response.rect);
                            }
                        });
                    },
                );
                assert!(
                    (rects[0].left() - rects[1].left()).abs() < 0.1,
                    "{width} / {scale}: {rects:?}"
                );
                assert!((rects[0].width() - rects[1].width()).abs() < 0.1);
                assert!(rects[0].bottom() <= rects[1].top());
            }
        }
    }

    #[test]
    fn hovered_button_background_precedes_text_and_keeps_rect() {
        let ctx = egui::Context::default();
        let mut rect = egui::Rect::NOTHING;
        let expected_label = crate::ui::localization::tr("Create");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = button(ui, "Create", ButtonKind::Accent).rect;
            });
        });
        let output = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(rect.center())],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = button(ui, "Create", ButtonKind::Accent);
                    assert!(response.hovered());
                    assert_eq!(response.rect, rect);
                });
            },
        );
        let background = output
            .shapes
            .iter()
            .position(|shape| {
                matches!(&shape.shape,
            egui::Shape::Rect(shape) if shape.fill == colors::ACCENT_ACTION_HOVER)
            })
            .expect("hover fill");
        let text = output
            .shapes
            .iter()
            .position(|shape| {
                matches!(&shape.shape,
            egui::Shape::Text(shape) if shape.galley.job.text == expected_label)
            })
            .expect("button label");
        assert!(background < text);
    }

    #[test]
    fn disabled_button_has_no_interactive_hover_state() {
        let ctx = egui::Context::default();
        let mut rect = egui::Rect::NOTHING;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = button_enabled(ui, "Undo", ButtonKind::Quiet, false).rect;
            });
        });
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(rect.center())],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = button_enabled(ui, "Undo", ButtonKind::Quiet, false);
                    assert!(!response.hovered());
                    assert!(!response.clicked());
                    assert_eq!(response.rect, rect);
                });
            },
        );
    }

    #[test]
    fn japanese_home_action_fits_reserved_rect() {
        let ctx = egui::Context::default();
        crate::ui::theme::configure_ae_theme(&ctx);
        crate::ui::icons::init_image_loaders(&ctx);
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(220.0, 62.0));
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = icon_label_button_at(
                    ui,
                    rect,
                    "プロジェクトを開く",
                    crate::ui::icons::SVG_OPEN_FOLDER,
                    ButtonKind::Default,
                );
                assert_eq!(response.rect, rect);
            });
        });
        let text = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "プロジェクトを開く" => {
                    Some(text)
                }
                _ => None,
            })
            .expect("Japanese action label");
        assert!(rect.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())));
    }
}
