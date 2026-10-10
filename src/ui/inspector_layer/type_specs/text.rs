use crate::core::text_animator::{SelectorShape, TextAnimatorSettings};
use crate::core::timeline::{Layer, LayerType};
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub(crate) fn draw_text_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    _current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) -> bool {
    if let LayerType::Text {
        text,
        font_size,
        color,
        tracking,
        leading,
        align,
        stroke_color,
        stroke_width,
        ..
    } = &mut layer.layer_type
    {
        let val_before_text = text.clone();
        let val_before_sz = *font_size;
        let val_before_col = *color;
        let val_before_tr = *tracking;
        let val_before_ld = *leading;
        let val_before_al = *align;
        let val_before_sc = *stroke_color;
        let val_before_sw = *stroke_width;

        ui.text_edit_multiline(text);
        kui::property(ui, "Font Size", |ui| {
            ui.add(kui::drag(font_size).range(8..=256))
        });
        kui::property(ui, "Color", |ui| kui::color_rgba(ui, color));
        kui::property(ui, "Tracking", |ui| {
            ui.add(
                kui::drag(tracking)
                    .speed(0.5)
                    .range(-50.0..=200.0)
                    .unit("px"),
            )
        });
        kui::property(ui, "Leading", |ui| {
            ui.add(kui::drag(leading).speed(0.05).range(0.5..=3.0).unit("x"))
        });
        kui::property(ui, "Align", |ui| {
            let align_options = [(0, "Left"), (1, "Center"), (2, "Right")];
            kui::combo(ui, "text_align_combo", align, &align_options)
        });
        kui::property(ui, "Stroke", |ui| {
            ui.horizontal(|ui| {
                kui::color_rgba(ui, stroke_color);
                ui.add(
                    kui::drag(stroke_width)
                        .speed(0.2)
                        .range(0.0..=50.0)
                        .unit("px"),
                );
            })
        });
        if val_before_text != *text
            || val_before_sz != *font_size
            || val_before_col != *color
            || val_before_tr != *tracking
            || val_before_ld != *leading
            || val_before_al != *align
            || val_before_sc != *stroke_color
            || val_before_sw != *stroke_width
        {
            *project_changed = true;
        }

        ui.separator();
        ui.collapsing("Text Animator", |ui| {
            if layer.text_animator.is_none() {
                layer.text_animator = Some(TextAnimatorSettings::default());
            }
            let Some(anim) = layer.text_animator.as_mut() else {
                ui.label(
                    egui::RichText::new("Animator unavailable")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
                return;
            };
            let enabled_before = anim.enabled;
            kui::property(ui, "Enable Animator", |ui| {
                kui::toggle(ui, &mut anim.enabled, "")
            });
            if enabled_before != anim.enabled {
                *project_changed = true;
            }

            *project_changed |= kui::property(ui, "Range Start", |ui| {
                kui::number(ui, &mut anim.selector.start, 0.0..=100.0, 0.5, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Range End", |ui| {
                kui::number(ui, &mut anim.selector.end, 0.0..=100.0, 0.5, "%")
            })
            .changed();
            *project_changed |= kui::property(ui, "Range Offset", |ui| {
                kui::slider(ui, &mut anim.selector.offset, -100.0..=100.0, "%")
            })
            .changed();
            let shapes = [
                (SelectorShape::Square, "Square"),
                (SelectorShape::RampUp, "Ramp Up"),
                (SelectorShape::RampDown, "Ramp Down"),
                (SelectorShape::Triangle, "Triangle"),
                (SelectorShape::Round, "Round"),
                (SelectorShape::Smooth, "Smooth"),
            ];
            *project_changed |= kui::property(ui, "Shape", |ui| {
                kui::combo(
                    ui,
                    format!("text_anim_shape_{}", layer.id),
                    &mut anim.selector.shape,
                    &shapes,
                )
            });
            *project_changed |= kui::property(ui, "Position Offset", |ui| {
                kui::vector_axes_unbounded(ui, &mut anim.position_offset, 0.5, "", ["X", "Y"])
            })
            .changed();
            *project_changed |= kui::property(ui, "Opacity Target", |ui| {
                kui::slider(ui, &mut anim.opacity, 0.0..=1.0, "")
            })
            .changed();
            *project_changed |= kui::property(ui, "Tracking", |ui| {
                ui.add(kui::drag(&mut anim.tracking).speed(0.5))
            })
            .changed();
        });

        // ── Advanced Animator Stack (multi-animator composition) ──
        ui.separator();
        ui.collapsing("Animator Stack", |ui| {
            if layer.text_animator_stack.is_none() {
                layer.text_animator_stack =
                    Some(crate::core::text_animator_advanced::AnimatorStack::default());
            }
            let Some(stack) = layer.text_animator_stack.as_mut() else {
                return;
            };
            let mut pending_remove: Option<usize> = None;

            for (ai, adv) in stack.animators.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        *project_changed |= kui::toggle(ui, &mut adv.enabled, "").changed();
                        ui.label(egui::RichText::new(format!("Animator {}", ai + 1)).strong());
                        if kui::button(ui, "✕", kui::ButtonKind::Quiet)
                            .on_hover_text("Remove")
                            .clicked()
                        {
                            pending_remove = Some(ai);
                        }
                    });
                    use crate::core::text_animator::SelectorShape;
                    use crate::core::text_animator_advanced::SelectorUnit;
                    let unit_options = [
                        (SelectorUnit::Characters, "Characters"),
                        (SelectorUnit::Words, "Words"),
                        (SelectorUnit::Lines, "Lines"),
                    ];
                    *project_changed |= kui::property(ui, "Unit", |ui| {
                        kui::combo(
                            ui,
                            format!("adv_unit_{}_{}", layer.id, ai),
                            &mut adv.unit,
                            &unit_options,
                        )
                    });
                    *project_changed |= kui::property(ui, "Range Start", |ui| {
                        kui::number(ui, &mut adv.selector.start, 0.0..=100.0, 0.5, "%")
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Range End", |ui| {
                        kui::number(ui, &mut adv.selector.end, 0.0..=100.0, 0.5, "%")
                    })
                    .changed();
                    let shape_options = [
                        (SelectorShape::Square, "Square"),
                        (SelectorShape::RampUp, "Ramp Up"),
                        (SelectorShape::RampDown, "Ramp Down"),
                        (SelectorShape::Triangle, "Triangle"),
                        (SelectorShape::Round, "Round"),
                        (SelectorShape::Smooth, "Smooth"),
                    ];
                    *project_changed |= kui::property(ui, "Shape", |ui| {
                        kui::combo(
                            ui,
                            format!("adv_shape_{}_{}", layer.id, ai),
                            &mut adv.selector.shape,
                            &shape_options,
                        )
                    });
                    *project_changed |= kui::property(ui, "Position", |ui| {
                        kui::vector_axes_unbounded(ui, &mut adv.position, 0.5, "", ["X", "Y"])
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Scale", |ui| {
                        kui::vector_axes(ui, &mut adv.scale, 0.0..=10.0, 0.5, "", ["X", "Y"])
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Opacity", |ui| {
                        kui::slider(ui, &mut adv.opacity, 0.0..=1.0, "")
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Rotation", |ui| {
                        ui.add(kui::drag(&mut adv.rotation).speed(1.0).unit("°"))
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Blur", |ui| {
                        kui::number(ui, &mut adv.blur, 0.0..=50.0, 0.5, "")
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Tracking", |ui| {
                        ui.add(kui::drag(&mut adv.tracking).speed(0.5))
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Skew", |ui| {
                        ui.add(kui::drag(&mut adv.advanced.skew).speed(0.5).unit("°"))
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Axis", |ui| {
                        ui.add(kui::drag(&mut adv.advanced.skew_axis).speed(0.5).unit("°"))
                    })
                    .changed();
                    *project_changed |= kui::property(ui, "Char Offset", |ui| {
                        ui.add(kui::drag(&mut adv.advanced.character_offset).speed(1.0))
                    })
                    .changed();
                });
                ui.add_space(2.0);
            }

            if let Some(ai) = pending_remove {
                stack.animators.remove(ai);
                *project_changed = true;
            }

            if kui::button(ui, "+ Add Animator", kui::ButtonKind::Default).clicked() {
                stack
                    .animators
                    .push(crate::core::text_animator_advanced::TextAnimatorAdvanced::default());
                *project_changed = true;
            }
        });
        return true;
    }
    false
}
