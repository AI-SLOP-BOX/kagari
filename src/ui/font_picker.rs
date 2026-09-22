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
    ui.heading("Typography & Faux Font Switches");
    ui.separator();

    // Real families available to the rasterizer
    let families = crate::core::font_rasterizer::with_font_rasterizer(|r| r.available_families());
    if families.is_empty() {
        ui.weak("No system fonts detected.");
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

    let mut selected = families
        .iter()
        .position(|f| *f == current_family)
        .unwrap_or(0);
    ui.label("Font Family:");
    egui::ComboBox::from_id_salt("font_family_combo")
        .selected_text(families.get(selected).map(|s| s.as_str()).unwrap_or("?"))
        .width(ui.available_width() - 12.0)
        .show_ui(ui, |ui| {
            for (i, fam) in families.iter().enumerate() {
                ui.selectable_value(&mut selected, i, fam);
            }
        });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .button("Apply to Selected Text")
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
                app.toasts.info(format!("Font set to {}", fam));
            } else {
                app.toasts.error("Select a text layer first");
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("AE Faux Font Switches:");

    let previous_switches = app.faux_font_switches;
    let (ref mut faux_bold, ref mut faux_italic, ref mut all_caps, ref mut small_caps) =
        app.faux_font_switches;

    ui.horizontal(|ui| {
        ui.checkbox(faux_bold, "B (Faux Bold)");
        ui.checkbox(faux_italic, "I (Faux Italic)");
        ui.checkbox(all_caps, "TT (All Caps)");
        ui.checkbox(small_caps, "Tt (Small Caps)");
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

    ui.weak(
        egui::RichText::new("Applied to the selected text layer and included in renders.")
            .small()
            .color(colors::TEXT_MUTED),
    );

    ui.add_space(8.0);
    ui.separator();
    ui.label("Preview:");
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
    ui.monospace(format!(
        "family: {} / weight: {}",
        base,
        if app.faux_font_switches.0 {
            "Bold"
        } else {
            "Regular"
        }
    ));
}
