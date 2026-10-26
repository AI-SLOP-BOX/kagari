use crate::core::text_layout::TextAlign;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

fn alignment_selector(ui: &mut egui::Ui, value: &mut u32) -> bool {
    let options = [
        (TextAlign::Left, "L", "Align left"),
        (TextAlign::Center, "C", "Align center"),
        (TextAlign::Right, "R", "Align right"),
        (TextAlign::Justify, "J", "Justify text"),
    ];
    let mut changed = false;
    let columns = if ui.available_width() >= 240.0 { 4 } else { 2 };
    let rows = options.len().div_ceil(columns);
    for row in 0..rows {
        ui.horizontal(|ui| {
            for column in 0..columns {
                let index = row * columns + column;
                let Some((mode, label, tooltip)) = options.get(index).copied() else {
                    continue;
                };
                if crate::ui::kui::tool_button(ui, *value == mode as u32, label)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    *value = mode as u32;
                    changed = true;
                }
            }
        });
    }
    changed
}

pub fn draw_paragraph_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    let selected_text_layer = app
        .selection
        .selected_layer_idx
        .and_then(|index| app.history.current().active_composition().layers.get(index))
        .is_some_and(|layer| {
            matches!(
                &layer.layer_type,
                crate::core::timeline::LayerType::Text { .. }
            )
        });
    if selected_text_layer {
        app.ensure_panel_edit_snapshot("paragraph", ui);
    }
    let comp = app.history.current_mut().active_composition_mut();

    let layer_idx = match app.selection.selected_layer_idx {
        Some(idx) if idx < comp.layers.len() => idx,
        _ => {
            ui.label(
                egui::RichText::new("No layer selected.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            return;
        }
    };

    let layer = &mut comp.layers[layer_idx];

    if !matches!(
        layer.layer_type,
        crate::core::timeline::LayerType::Text { .. }
    ) {
        ui.label(
            egui::RichText::new("Selected layer is not a text layer.")
                .small()
                .color(colors::TEXT_MUTED),
        );
        return;
    }

    // Ensure text_formatting exists, preserving the legacy LayerType values
    // when opening an older text layer for the first time.
    let mut migrated_formatting = false;
    if layer.text_formatting.is_none() {
        let initial = match &layer.layer_type {
            crate::core::timeline::LayerType::Text {
                font_family,
                tracking,
                leading,
                align,
                stroke_color,
                stroke_width,
                ..
            } => crate::core::timeline::TextFormatting {
                font_family: font_family.clone(),
                tracking: *tracking,
                leading: *leading,
                stroke_color: (*stroke_width > 0.0).then_some(*stroke_color),
                stroke_width: *stroke_width,
                alignment: *align as u32,
                ..Default::default()
            },
            _ => crate::core::timeline::TextFormatting::default(),
        };
        layer.text_formatting = Some(initial);
        migrated_formatting = true;
    }
    let Some(fmt) = (match layer.text_formatting {
        Some(_) => layer.text_formatting.as_mut(),
        None => None,
    }) else {
        return;
    };
    let mut changed = false;

    let grid = crate::ui::kui::PropertyGrid::new(
        ui,
        &[
            "Alignment",
            "Width",
            "Height",
            "Tracking",
            "Leading",
            "Enabled",
            "Stroke Width",
            "Stroke Color",
        ],
    );

    crate::ui::kui::section(ui, "Paragraph");
    changed |= grid.row(ui, "Alignment", |ui| {
        alignment_selector(ui, &mut fmt.alignment)
    });

    crate::ui::kui::section(ui, "Text Box");
    changed |= grid.row(ui, "Width", |ui| {
        ui.add(
            crate::ui::kui::drag(&mut fmt.box_width)
                .range(0.0..=10000.0)
                .speed(1.0)
                .unit(" px"),
        )
        .changed()
    });
    changed |= grid.row(ui, "Height", |ui| {
        ui.add(
            crate::ui::kui::drag(&mut fmt.box_height)
                .range(0.0..=10000.0)
                .speed(1.0)
                .unit(" px"),
        )
        .changed()
    });

    crate::ui::kui::section(ui, "Spacing");
    changed |= grid.row(ui, "Tracking", |ui| {
        ui.add(
            crate::ui::kui::drag(&mut fmt.tracking)
                .range(-100.0..=1000.0)
                .speed(0.1)
                .unit(" px"),
        )
        .changed()
    });
    changed |= grid.row(ui, "Leading", |ui| {
        ui.add(
            crate::ui::kui::drag(&mut fmt.leading)
                .range(0.1..=10.0)
                .speed(0.01),
        )
        .changed()
    });

    crate::ui::kui::section(ui, "Stroke");
    let mut stroke_enabled = fmt.stroke_color.is_some();
    changed |= grid.row(ui, "Enabled", |ui| {
        crate::ui::kui::toggle(ui, &mut stroke_enabled, "Enabled").changed()
    });
    if stroke_enabled != fmt.stroke_color.is_some() {
        if stroke_enabled {
            fmt.stroke_color = Some([0.0, 0.0, 0.0, 1.0]);
        } else {
            fmt.stroke_color = None;
        }
        changed = true;
    }

    if let Some(ref mut stroke_color) = fmt.stroke_color {
        changed |= grid.row(ui, "Stroke Width", |ui| {
            ui.add(
                crate::ui::kui::drag(&mut fmt.stroke_width)
                    .range(0.0..=100.0)
                    .speed(0.1)
                    .unit(" px"),
            )
            .changed()
        });
        changed |= grid.row(ui, "Stroke Color", |ui| {
            crate::ui::kui::color_rgba(ui, stroke_color).changed()
        });
    }

    let rendered_formatting = fmt.clone();
    if changed {
        if let crate::core::timeline::LayerType::Text {
            font_family,
            tracking,
            leading,
            align,
            stroke_color,
            stroke_width,
            ..
        } = &mut layer.layer_type
        {
            *font_family = rendered_formatting.font_family.clone();
            *tracking = rendered_formatting.tracking;
            *leading = rendered_formatting.leading;
            *align = rendered_formatting.alignment as usize;
            if let Some(color) = rendered_formatting.stroke_color {
                *stroke_color = color;
                *stroke_width = rendered_formatting.stroke_width;
            } else {
                *stroke_width = 0.0;
            }
        }
        let pointer_down = ui.input(|input| input.pointer.any_down());
        if pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = app.take_panel_edit_snapshot("paragraph") {
                    app.begin_drag_with_snapshot(snapshot, "Paragraph Edit");
                }
            }
        } else if app.drag_active() {
            app.commit_drag();
        } else if let Some(snapshot) = app.take_panel_edit_snapshot("paragraph") {
            app.begin_drag_with_snapshot(snapshot, "Paragraph Edit");
            app.commit_drag();
        }
    } else if migrated_formatting {
    }
}
