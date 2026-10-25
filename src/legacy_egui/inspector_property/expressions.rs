use crate::core::timeline::Expression;
use crate::ui::custom_widgets;
use crate::ui::kui;
use crate::ui::theme::colors;
use eframe::egui;

pub fn draw_expression_selector(
    ui: &mut egui::Ui,
    label: &str,
    expr_opt: &mut Option<Expression>,
    project_changed: &mut bool,
    current_frame: Option<u32>,
    fps: Option<u32>,
) {
    let active = expr_opt.is_some();
    egui::CollapsingHeader::new(if active {
        "fx  Expression • Active"
    } else {
        "fx"
    })
    .id_salt(("expression_section", label))
    .default_open(false)
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            let expr_text = match expr_opt {
                Some(Expression::Wiggle {
                    frequency,
                    amplitude,
                }) => format!("wiggle({}, {})", frequency, amplitude),
                Some(Expression::TimeDriver { multiplier, offset }) => {
                    format!("time * {} + {}", multiplier, offset)
                }
                Some(Expression::LoopOut) => "loopOut()".to_string(),
                Some(Expression::PingPong) => "loopOut(\"pingpong\")".to_string(),
                Some(Expression::Raw(s)) => expression_summary(s),
                None => "None".to_string(),
            };

            let before = expr_opt.clone();
            let combo_id = ui.make_persistent_id(format!("ae_expr_combo_{}", label));
            let expression_options = [
                (None, "None"),
                (
                    Some(Expression::Wiggle {
                        frequency: 2.0,
                        amplitude: 50.0,
                    }),
                    "Wiggle (2Hz, 50px)",
                ),
                (
                    Some(Expression::TimeDriver {
                        multiplier: 30.0,
                        offset: 0.0,
                    }),
                    "Time Spin (30°/s)",
                ),
                (Some(Expression::LoopOut), "loopOut(\"cycle\")"),
                (Some(Expression::PingPong), "loopOut(\"pingpong\")"),
                (Some(Expression::Raw("value".into())), "Custom Script..."),
            ];
            kui::combo_with_selected(ui, combo_id, expr_opt, &expr_text, &expression_options);

            if custom_widgets::ae_icon_button(
                ui,
                "fx",
                "Create a starter expression for this property",
            )
            .clicked()
            {
                *expr_opt = Some(Expression::Wiggle {
                    frequency: 3.0,
                    amplitude: 25.0,
                });
                *project_changed = true;
            }

            if before != *expr_opt {
                *project_changed = true;
            }
        });
    });

    let mut remove_requested = false;
    if let Some(Expression::Raw(script)) = expr_opt {
        ui.indent(format!("expr_editor_{}", label), |ui| {
            let editor_id = ui.make_persistent_id(format!("expr_textedit_{}", label));
            let response = ui.add(
                egui::TextEdit::multiline(script)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("// Rhai expression script...")
                    .id(editor_id),
            );
            if response.changed() {
                *project_changed = true;
            }

            ui.add_space(2.0);

            ui.horizontal(|ui| {
                let presets = [
                    ("wiggle(4, 30)", "Wiggle"),
                    ("loopOut(\"cycle\")", "Loop Cycle"),
                    ("loopOut(\"pingpong\")", "PingPong"),
                    ("value * 2", "Double"),
                    ("time * 100", "Time×100"),
                    ("Math.sin(time * 3) * 50", "Sine"),
                    ("posterizeTime(12); value", "12fps"),
                ];
                let preset_id = ui.make_persistent_id(format!("expr_presets_{}", label));
                kui::combo_popup(ui, preset_id, "Presets ▾", Some(80.0), |ui| {
                    for (expr, lbl) in presets {
                        if ui.selectable_label(false, lbl).clicked() {
                            *script = expr.to_string();
                            *project_changed = true;
                        }
                    }
                });

                let _lang_menu_id = ui.make_persistent_id(format!("expr_lang_menu_{}", label));
                kui::menu_button(ui, "▶ Language ▾", |ui| {
                    ui.label(
                        egui::RichText::new("Expression Language Library")
                            .strong()
                            .color(colors::ACCENT_CYAN),
                    );
                    ui.separator();

                    kui::menu_button(ui, "Global & Comp", |ui| {
                        if kui::menu_item(ui, "time (seconds)").clicked() {
                            *script = format!("{}\ntime", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "thisComp.duration").clicked() {
                            *script = format!("{}\nthisComp.duration", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "thisLayer.index").clicked() {
                            *script = format!("{}\nthisLayer.index", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "valueAtTime(time - 0.1)").clicked() {
                            *script = format!("{}\nvalueAtTime(time - 0.1)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                    });

                    kui::menu_button(ui, "Random Numbers", |ui| {
                        if kui::menu_item(ui, "wiggle(freq, amp)").clicked() {
                            *script = format!("{}\nwiggle(4, 25)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "random(min, max)").clicked() {
                            *script = format!("{}\nrandom(0.0, 100.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "noise(time)").clicked() {
                            *script = format!("{}\nnoise(time * 2.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                    });

                    kui::menu_button(ui, "Interpolation", |ui| {
                        if kui::menu_item(ui, "linear(t, tMin, tMax, val1, val2)").clicked() {
                            *script = format!("{}\nlinear(time, 0.0, 2.0, 0.0, 100.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "ease(t, tMin, tMax, val1, val2)").clicked() {
                            *script = format!("{}\nease(time, 0.0, 1.5, 0.0, 200.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "easeIn(t, 0, 1, 0, 100)").clicked() {
                            *script = format!("{}\neaseIn(time, 0.0, 1.0, 0.0, 100.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                    });

                    kui::menu_button(ui, "Looping & PingPong", |ui| {
                        if kui::menu_item(ui, "loopOut(\"cycle\")").clicked() {
                            *script = format!("{}\nloopOut(\"cycle\")", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "loopOut(\"pingpong\")").clicked() {
                            *script = format!("{}\nloopOut(\"pingpong\")", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "loopIn(\"cycle\")").clicked() {
                            *script = format!("{}\nloopIn(\"cycle\")", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                    });

                    kui::menu_button(ui, "Vector & Trigonometry Math", |ui| {
                        if kui::menu_item(ui, "Math.sin(time * 3.0) * 50.0").clicked() {
                            *script = format!("{}\nMath.sin(time * 3.0) * 50.0", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "Math.atan2(y, x)").clicked() {
                            *script = format!("{}\nMath.atan2(y, x)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                        if kui::menu_item(ui, "clamp(val, min, max)").clicked() {
                            *script = format!("{}\nclamp(value, 0.0, 100.0)", script);
                            *project_changed = true;
                            ui.close_menu();
                        }
                    });
                });

                let engine = crate::core::expression_engine::build_engine();
                let validation = crate::core::expression_engine::validate_script(&engine, script);
                let (icon, color, tip) = match &validation {
                    Ok(()) => ("✓", colors::ACCENT_GREEN, "Script is valid".to_string()),
                    Err(e) => ("✗", colors::ACCENT_RED, e.clone()),
                };
                ui.label(egui::RichText::new(icon).small().color(color))
                    .on_hover_text(tip);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if custom_widgets::ae_icon_button(ui, "✕", "Remove expression").clicked() {
                        remove_requested = true;
                    }
                    if custom_widgets::ae_svg_icon_button(
                        ui,
                        "expression-test",
                        crate::ui::icons::SVG_PLAY,
                        "Test expression at current frame",
                        colors::TEXT_SECONDARY,
                    )
                    .clicked()
                    {
                        if let (Some(cf), Some(fps)) = (current_frame, fps) {
                            let snap = ui.ctx().data(|d| {
                                d.get_temp::<std::sync::Arc<crate::core::expression_engine::CompSnapshot>>(
                                    egui::Id::new("ae_expr_comp_snap"),
                                )
                            });
                            let result = match snap {
                                Some(snap) => test_expression_with_comp(script, cf, fps, &snap),
                                None => test_expression_inline(script, cf, fps),
                            };
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new(("expr_test_result", label)), result);
                            });
                        }
                    }
                });
            });

            let test_result = ui.ctx().data(|d| {
                d.get_temp::<String>(egui::Id::new(("expr_test_result", label)))
            });
            if let Some(result) = test_result {
                ui.label(
                    egui::RichText::new(format!("→ {}", result))
                        .small()
                        .monospace()
                        .color(colors::TEXT_ACCENT),
                );
            }
        });
    }
    if remove_requested {
        *expr_opt = None;
        *project_changed = true;
    }
}

fn test_expression_inline(script: &str, frame: u32, fps: u32) -> String {
    let engine = crate::core::expression_engine::build_engine();
    let (result, diag) = crate::core::expression_engine::eval_v2_with_diagnostics(
        &engine,
        script,
        [0.0, 0.0],
        frame,
        fps,
    );
    if let Some(err) = diag {
        format!("Error: {}", err)
    } else {
        format!("({:.2}, {:.2})", result[0], result[1])
    }
}

fn test_expression_with_comp(
    script: &str,
    frame: u32,
    fps: u32,
    snap: &crate::core::expression_engine::CompSnapshot,
) -> String {
    let base_v2 = [0.0f32, 0.0];
    let result =
        crate::core::expression_engine::eval_v2_with_comp(script, base_v2, frame, fps, snap, None);
    format!("({:.2}, {:.2})", result[0], result[1])
}

fn expression_summary(script: &str) -> String {
    let mut characters = script.chars();
    let summary: String = characters.by_ref().take(20).collect();
    let suffix = if characters.next().is_some() {
        "…"
    } else {
        ""
    };
    format!("Custom: {summary}{suffix}")
}

#[cfg(test)]
mod summary_tests {
    use super::expression_summary;

    #[test]
    fn expression_summary_preserves_unicode_boundaries() {
        assert_eq!(expression_summary("日本語🔥"), "Custom: 日本語🔥");
        assert_eq!(
            expression_summary(&"あ".repeat(21)),
            format!("Custom: {}…", "あ".repeat(20))
        );
    }
}
