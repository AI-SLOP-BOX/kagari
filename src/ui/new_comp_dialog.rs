//! New Composition dialog: name + resolution/fps/duration presets +
//! editable fields. Replaces the old instant-create Cmd+N behavior.
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

const PRESETS: &[(&str, u32, u32, u32, u32)] = &[
    ("HD 1920×1080 · 30fps · 10 sec", 1920, 1080, 30, 300),
    ("Cinema 1920×1080 · 24fps · 10 sec", 1920, 1080, 24, 240),
    ("4K UHD 3840×2160 · 30fps · 10 sec", 3840, 2160, 30, 300),
    ("Square 1080×1080 · 30fps · 10 sec", 1080, 1080, 30, 300),
    (
        "Vertical Video 1080×1920 · 30fps · 15 sec",
        1080,
        1920,
        30,
        450,
    ),
    ("Custom…", 1920, 1080, 30, 300),
];

pub fn draw_new_comp_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_new_comp_dialog {
        return;
    }

    let mut open = true;
    let mut keep_open = true;
    let draft_id = egui::Id::new("ae_newcomp_draft");
    crate::ui::modal::dialog(crate::ui::localization::tr("New Composition"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Draft state: (name, preset_idx, w, h, fps, dur)
            let project = app.history.current();
            let pristine_start = project.has_pristine_start_composition();
            let count = project
                .compositions
                .len()
                .saturating_sub(usize::from(pristine_start));
            let mut draft = ctx.data_mut(|d| {
                d.get_temp::<(String, usize, u32, u32, u32, u32)>(draft_id)
                    .unwrap_or_else(|| {
                        (
                            format!(
                                "{} {}",
                                crate::ui::localization::tr("Composition"),
                                count + 1
                            ),
                            0usize,
                            1920,
                            1080,
                            30,
                            300,
                        )
                    })
            });

            ui.horizontal(|ui| {
                ui.label(crate::ui::localization::tr("Name:"));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut draft.0)
                        .desired_width(220.0)
                        .hint_text(crate::ui::localization::tr("Composition name")),
                );
                if !resp.has_focus() && draft.0.trim().is_empty() {
                    draft.0 = format!(
                        "{} {}",
                        crate::ui::localization::tr("Composition"),
                        count + 1
                    );
                }
            });

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(crate::ui::localization::tr("Preset"))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let mut picked = None;
            egui::ComboBox::from_id_salt("newcomp_preset")
                .selected_text(crate::ui::localization::tr(PRESETS[draft.1].0))
                .width(260.0)
                .show_ui(ui, |ui| {
                    for (i, (label, w, h, f, d)) in PRESETS.iter().enumerate() {
                        if ui
                            .selectable_label(draft.1 == i, crate::ui::localization::tr(label))
                            .clicked()
                        {
                            picked = Some((i, *w, *h, *f, *d));
                        }
                    }
                });
            if let Some((i, w, h, f, d)) = picked {
                draft.1 = i;
                if i + 1 < PRESETS.len() {
                    draft.2 = w;
                    draft.3 = h;
                    draft.4 = f;
                    draft.5 = d;
                }
            }

            ui.add_space(6.0);
            let mut fields_changed = false;
            egui::Grid::new("newcomp_fields")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label(crate::ui::localization::tr("Width:"));
                    fields_changed |= ui
                        .add(
                            egui::DragValue::new(&mut draft.2)
                                .range(16..=7680)
                                .suffix(" px"),
                        )
                        .changed();
                    ui.label(crate::ui::localization::tr("Height:"));
                    fields_changed |= ui
                        .add(
                            egui::DragValue::new(&mut draft.3)
                                .range(16..=4320)
                                .suffix(" px"),
                        )
                        .changed();
                    ui.end_row();
                    ui.label(crate::ui::localization::tr("Frame Rate:"));
                    fields_changed |= ui
                        .add(
                            egui::DragValue::new(&mut draft.4)
                                .range(1..=120)
                                .suffix(" fps"),
                        )
                        .changed();
                    ui.label(crate::ui::localization::tr("Duration:"));
                    fields_changed |= ui
                        .add(
                            egui::DragValue::new(&mut draft.5)
                                .range(1..=180_000)
                                .suffix(" fr"),
                        )
                        .changed();
                    ui.end_row();
                });
            if fields_changed {
                draft.1 = PRESETS.len() - 1;
            }
            if draft.1 + 1 < PRESETS.len() {
                ui.label(
                    egui::RichText::new(crate::ui::localization::tr(
                        "Changing values switches to Custom",
                    ))
                    .small()
                    .color(colors::TEXT_MUTED),
                );
            }

            ui.add_space(8.0);
            let depth_id = egui::Id::new("ae_newcomp_depth");
            let mut depth_idx =
                ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(depth_id, || 0usize));
            ui.horizontal(|ui| {
                ui.label(crate::ui::localization::tr("Color Depth:"));
                let label = match depth_idx {
                    0 => crate::ui::localization::tr("8 bpc (Integer)"),
                    1 => crate::ui::localization::tr("16 bpc (Half Float)"),
                    _ => crate::ui::localization::tr("32 bpc (Float / HDR)"),
                };
                egui::ComboBox::from_id_salt("newcomp_depth_combo")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut depth_idx,
                            0,
                            crate::ui::localization::tr("8 bpc (Integer)"),
                        );
                        ui.selectable_value(
                            &mut depth_idx,
                            1,
                            crate::ui::localization::tr("16 bpc (Half Float)"),
                        );
                        ui.selectable_value(
                            &mut depth_idx,
                            2,
                            crate::ui::localization::tr("32 bpc (Float / HDR)"),
                        );
                    });
            });
            ctx.data_mut(|d| d.insert_temp(depth_id, depth_idx));

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(format!("✓ {}", crate::ui::localization::tr("Create")))
                            .min_size(egui::vec2(90.0, 26.0)),
                    )
                    .clicked()
                {
                    let name = if draft.0.trim().is_empty() {
                        format!(
                            "{} {}",
                            crate::ui::localization::tr("Composition"),
                            count + 1
                        )
                    } else {
                        draft.0.trim().to_string()
                    };
                    let mut comp = crate::core::timeline::Composition::new(
                        app.history.current().next_composition_id("comp"),
                        name.clone(),
                        draft.2,
                        draft.3,
                        draft.4,
                        draft.5,
                    );
                    comp.dither_output = true;
                    comp.bit_depth = match depth_idx {
                        1 => crate::core::color_science::BitDepth::SixteenBit,
                        2 => crate::core::color_science::BitDepth::ThirtyTwoBitFloat,
                        _ => crate::core::color_science::BitDepth::EightBit,
                    };
                    app.modify_project(|proj| {
                        if proj.has_pristine_start_composition() {
                            proj.compositions[0] = comp;
                            proj.active_composition_idx = 0;
                        } else {
                            proj.compositions.push(comp);
                            proj.active_composition_idx = proj.compositions.len() - 1;
                        }
                    });
                    let width = draft.2.to_string();
                    let height = draft.3.to_string();
                    let frame_rate = draft.4.to_string();
                    let depth = match depth_idx {
                        1 => "16bpc",
                        2 => "32bpc Float",
                        _ => "8bpc",
                    };
                    app.toasts.info(crate::ui::localization::tr_format(
                        "Composition \"{}\" created — {}×{} / {}fps ({})",
                        &[&name, &width, &height, &frame_rate, depth],
                    ));
                    app.show_home = false;
                    keep_open = false;
                }
                if ui.button(crate::ui::localization::tr("Cancel")).clicked() {
                    keep_open = false;
                }
            });

            ctx.data_mut(|d| d.insert_temp(draft_id, draft));
        });

    app.show_new_comp_dialog = open && keep_open;
    if !app.show_new_comp_dialog {
        ctx.data_mut(|d| d.remove::<(String, usize, u32, u32, u32, u32)>(draft_id));
    }
}
