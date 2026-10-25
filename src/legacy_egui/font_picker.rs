use crate::ui::kui::{self, ButtonKind};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

fn apply_text_style(
    layer: &mut crate::core::timeline::Layer,
    family: &str,
    faux_bold: bool,
    faux_italic: bool,
    all_caps: bool,
    small_caps: bool,
) {
    let crate::core::timeline::LayerType::Text { font_family, .. } = &mut layer.layer_type else {
        return;
    };
    *font_family = family.to_string();
    let formatting = layer
        .text_formatting
        .get_or_insert_with(crate::core::timeline::TextFormatting::default);
    formatting.font_family = family.to_string();
    formatting.faux_bold = faux_bold;
    formatting.faux_italic = faux_italic;
    formatting.all_caps = all_caps;
    formatting.small_caps = small_caps;
}

pub fn draw_font_picker(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "Typography & Faux Font Switches");

    // Real families available to the rasterizer
    let families = crate::core::font_rasterizer::with_font_rasterizer(|r| r.available_families());
    if families.is_empty() {
        kui::empty_state(ui, "No system fonts detected.", 72.0);
        return;
    }

    // Current family = the rendered text style, falling back to the legacy
    // LayerType field for projects created before TextFormatting existed.
    let current_family = app
        .selection
        .selected_layer_idx
        .and_then(|idx| {
            let comp = app.history.current().active_composition();
            comp.layers.get(idx).and_then(|l| {
                l.text_formatting
                    .as_ref()
                    .map(|tf| tf.font_family.clone())
                    .or_else(|| match &l.layer_type {
                        crate::core::timeline::LayerType::Text { font_family, .. } => {
                            Some(font_family.clone())
                        }
                        _ => None,
                    })
            })
        })
        .unwrap_or_else(|| families[0].clone());

    if let Some(idx) = app.selection.selected_layer_idx {
        if let Some(layer) = app.history.current().active_composition().layers.get(idx) {
            if let Some(tf) = layer.text_formatting.as_ref() {
                app.faux_font_switches = (tf.faux_bold, tf.faux_italic, tf.all_caps, tf.small_caps);
            }
        }
    }

    let pending_id = egui::Id::new(("font_picker_pending", app.selection.selected_layer_idx));
    let pending_family = ui.ctx().data(|data| data.get_temp::<String>(pending_id));
    let initial_family = pending_family.as_deref().unwrap_or(&current_family);
    let mut selected = families
        .iter()
        .position(|family| family == initial_family)
        .unwrap_or(0);
    let family_options: Vec<_> = families
        .iter()
        .enumerate()
        .map(|(index, family)| (index, family.as_str()))
        .collect();
    let grid = kui::PropertyGrid::new(ui, &["Font Family"]);
    let family_changed = grid.row(ui, "Font Family", |ui| {
        kui::combo(ui, "font_family_combo", &mut selected, &family_options)
    });
    if family_changed {
        ui.ctx().data_mut(|data| {
            data.insert_temp(pending_id, families[selected].clone());
        });
    }

    ui.add_space(kui::space::SM);
    let selected_text_layer = app.selection.selected_layer_idx.is_some_and(|idx| {
        app.history
            .current()
            .active_composition()
            .layers
            .get(idx)
            .is_some_and(|layer| {
                matches!(
                    &layer.layer_type,
                    crate::core::timeline::LayerType::Text { .. }
                )
            })
    });
    if kui::button_enabled(
        ui,
        "Apply to Selected Text",
        ButtonKind::Accent,
        selected_text_layer,
    )
    .on_hover_text("Writes the family into the selected text layer's formatting")
    .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            let fam = families[selected].clone();
            let fam_for_project = fam.clone();
            let (faux_bold, faux_italic, all_caps, small_caps) = app.faux_font_switches;
            app.modify_project(|project| {
                let comp = project.active_composition_mut();
                if let Some(layer) = comp.layers.get_mut(idx) {
                    apply_text_style(
                        layer,
                        &fam_for_project,
                        faux_bold,
                        faux_italic,
                        all_caps,
                        small_caps,
                    );
                }
            });
            app.toasts.info(crate::ui::localization::tr_format(
                "Font set to {}",
                &[&fam],
            ));
            ui.ctx().data_mut(|data| data.remove::<String>(pending_id));
        }
    }

    ui.add_space(kui::space::SM);
    kui::separator(ui);
    let previous_switches = app.faux_font_switches;
    ui.add_enabled_ui(selected_text_layer, |ui| {
        kui::group(ui, "AE Faux Font Switches", |ui| {
            ui.horizontal_wrapped(|ui| {
                kui::toggle(ui, &mut app.faux_font_switches.0, "B (Faux Bold)");
                kui::toggle(ui, &mut app.faux_font_switches.1, "I (Faux Italic)");
                kui::toggle(ui, &mut app.faux_font_switches.2, "TT (All Caps)");
                kui::toggle(ui, &mut app.faux_font_switches.3, "Tt (Small Caps)");
            });
        });
    });

    if previous_switches != app.faux_font_switches {
        if let Some(idx) = app.selection.selected_layer_idx {
            let (bold, italic, caps, small) = app.faux_font_switches;
            app.modify_project(|project| {
                if let Some(layer) = project.active_composition_mut().layers.get_mut(idx) {
                    apply_text_style(layer, &current_family, bold, italic, caps, small);
                }
            });
        }
    }

    kui::caption(
        ui,
        "Applied to the selected text layer and included in renders.",
    );

    ui.add_space(kui::space::SM);
    kui::separator(ui);
    kui::section_title(ui, "Preview");
    let base = families.get(selected).cloned().unwrap_or_default();
    let sample = if app.faux_font_switches.2 {
        "KAGARI VFX STUDIO"
    } else {
        "Kagari VFX Studio"
    };
    ui.add(egui::Label::new(
        egui::RichText::new(sample)
            .size(22.0)
            .color(colors::TEXT_PRIMARY),
    ));
    let weight = if app.faux_font_switches.0 {
        crate::ui::localization::tr("Bold")
    } else {
        crate::ui::localization::tr("Regular")
    };
    kui::readout(
        ui,
        crate::ui::localization::tr_format("family: {} / weight: {}", &[&base, &weight]),
        colors::TEXT_MUTED,
    );
}
