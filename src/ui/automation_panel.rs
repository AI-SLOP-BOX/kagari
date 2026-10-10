use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_automation_panel {
        return;
    }
    let mut open = app.show_automation_panel;
    let mut remove_index = None;
    let mut selected_binding = app.selected_automation_binding;
    let mut move_key: Option<(usize, crate::core::unified_time::Time, i64)> = None;
    let mut undo_requested = false;
    let mut redo_requested = false;
    let before_bindings = app
        .production_document
        .as_ref()
        .map(|document| document.bindings.clone());
    let frame_rate = crate::core::unified_time::FrameRate::new(
        app.history.current().active_composition().fps.max(1),
        1,
    );
    crate::ui::modal::window("オートメーション")
        .open(&mut open)
        .default_width(520.0)
        .default_height(480.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 82.0))
        .show(ctx, |ui| {
            let Some(document) = app.production_document.as_mut() else {
                crate::ui::kui::secondary(ui, "制作ドキュメントが読み込まれていません");
                return;
            };
            let rate = frame_rate;
            let time = rate.map(|rate| {
                crate::core::unified_time::Time::from_frame(app.playback.current_frame as i64, rate)
            });
            ui.horizontal(|ui| {
                crate::ui::kui::title(ui, "オートメーション");
                crate::ui::kui::caption(
                    ui,
                    format!(
                        "{} 接続 · フレーム {}",
                        document.bindings.len(),
                        app.playback.current_frame
                    ),
                );
            });
            ui.horizontal(|ui| {
                if crate::ui::kui::button(
                    ui,
                    "グラフエディターを開く",
                    crate::ui::kui::ButtonKind::Quiet,
                )
                .on_hover_text("選択レイヤーのキーフレームグラフを表示")
                .clicked()
                {
                    app.show_graph_editor = true;
                }
                crate::ui::kui::caption(ui, "タイムラインと共有");
            });
            ui.horizontal(|ui| {
                if crate::ui::kui::button(ui, "＋ 接続を追加", crate::ui::kui::ButtonKind::Accent)
                    .clicked()
                {
                    document
                        .bindings
                        .push(crate::core::automation_binding::AutomationBinding {
                            source: "audio.source".into(),
                            target: "vfx.parameter".into(),
                            curve: crate::core::automation_binding::AutomationCurve {
                                points: vec![crate::core::automation_binding::AutomationPoint {
                                    time: crate::core::unified_time::Time::ZERO,
                                    value: 0.0,
                                }],
                            },
                            input_min: 0.0,
                            input_max: 1.0,
                            output_min: 0.0,
                            output_max: 1.0,
                        });
                    selected_binding = Some(document.bindings.len() - 1);
                }
                if crate::ui::kui::button_enabled(
                    ui,
                    "元に戻す",
                    crate::ui::kui::ButtonKind::Quiet,
                    !app.automation_undo.is_empty(),
                )
                .clicked()
                {
                    undo_requested = true;
                }
                if crate::ui::kui::button_enabled(
                    ui,
                    "やり直す",
                    crate::ui::kui::ButtonKind::Quiet,
                    !app.automation_redo.is_empty(),
                )
                .clicked()
                {
                    redo_requested = true;
                }
            });
            crate::ui::kui::section(ui, "接続一覧");
            if document.bindings.is_empty() {
                crate::ui::kui::secondary(
                    ui,
                    "接続はまだありません。入力元と操作対象をつなげて自動化できます。",
                );
            }
            egui::ScrollArea::vertical().show_rows(
                ui,
                60.0,
                document.bindings.len(),
                |ui, range| {
                    for index in range {
                        let binding = &document.bindings[index];
                        let value = time.and_then(|time| binding.evaluate(time));
                        crate::ui::kui::panel(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                crate::ui::kui::caption(ui, format!("{:02}", index + 1));
                                let right_controls_width =
                                    if value.is_some() { 96.0 } else { 44.0 };
                                let pair_width =
                                    (ui.available_width() - right_controls_width - 12.0).max(80.0);
                                let pair_rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(pair_width, 20.0),
                                );
                                let select_response = ui.interact(
                                    pair_rect,
                                    ui.make_persistent_id(("automation-binding-select", index)),
                                    egui::Sense::click(),
                                );
                                if select_response.hovered() {
                                    ui.painter().rect_filled(pair_rect, 3.0, colors::BG_HOVER);
                                }
                                if select_response.clicked() {
                                    selected_binding = Some(index);
                                }
                                ui.allocate_ui_with_layout(
                                    egui::vec2(pair_width, 20.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let column_width = (pair_width - 18.0) * 0.5;
                                        let _ = ui.allocate_ui_with_layout(
                                            egui::vec2(column_width, 20.0),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                crate::ui::kui::truncated_primary(
                                                    ui,
                                                    &binding.source,
                                                )
                                            },
                                        );
                                        crate::ui::kui::caption(ui, "·");
                                        let _ = ui.allocate_ui_with_layout(
                                            egui::vec2(column_width, 20.0),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                crate::ui::kui::truncated_secondary(
                                                    ui,
                                                    &binding.target,
                                                )
                                            },
                                        );
                                    },
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(value) = value {
                                            crate::ui::kui::readout(
                                                ui,
                                                format!("{value:.3}"),
                                                colors::ACCENT_GREEN,
                                            );
                                        }
                                        if crate::ui::kui::button(
                                            ui,
                                            "削除",
                                            crate::ui::kui::ButtonKind::Quiet,
                                        )
                                        .clicked()
                                        {
                                            remove_index = Some(index);
                                        }
                                    },
                                );
                            });
                            if let Err(error) = binding.validate() {
                                crate::ui::kui::error_text(ui, error);
                            } else {
                                crate::ui::kui::caption(
                                    ui,
                                    format!(
                                        "入力範囲 {:.2}–{:.2}　出力範囲 {:.2}–{:.2}　キー {} 件",
                                        binding.input_min,
                                        binding.input_max,
                                        binding.output_min,
                                        binding.output_max,
                                        binding.curve.points.len()
                                    ),
                                );
                            }
                            if selected_binding == Some(index) {
                                ui.painter().rect_stroke(
                                    ui.min_rect().expand(4.0),
                                    egui::Rounding::same(5.0),
                                    egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION),
                                );
                            }
                        });
                    }
                },
            );
            if let Some(index) = selected_binding.filter(|index| *index < document.bindings.len()) {
                crate::ui::kui::section(ui, &format!("接続 {:02} の設定", index + 1));
                let binding = &mut document.bindings[index];
                crate::ui::kui::panel(ui, |ui| {
                    let grid = crate::ui::kui::PropertyGrid::new(
                        ui,
                        &["入力元", "接続先", "入力範囲", "出力範囲"],
                    );
                    grid.row(ui, "入力元", |ui| {
                        crate::ui::kui::text_field(ui, &mut binding.source)
                    });
                    grid.row(ui, "接続先", |ui| {
                        crate::ui::kui::text_field(ui, &mut binding.target)
                    });
                    grid.row(ui, "入力範囲", |ui| {
                        ui.horizontal(|ui| {
                            ui.add(crate::ui::kui::drag(&mut binding.input_min).speed(0.01));
                            ui.add(crate::ui::kui::drag(&mut binding.input_max).speed(0.01));
                        });
                    });
                    grid.row(ui, "出力範囲", |ui| {
                        ui.horizontal(|ui| {
                            ui.add(crate::ui::kui::drag(&mut binding.output_min).speed(0.1));
                            ui.add(crate::ui::kui::drag(&mut binding.output_max).speed(0.1));
                        });
                    });
                    ui.horizontal(|ui| {
                        if let Some(value) = time.and_then(|time| binding.evaluate(time)) {
                            crate::ui::kui::readout(
                                ui,
                                format!("現在値　{value:.3}"),
                                colors::ACCENT_GREEN,
                            );
                        }
                        if let Some(time) = time {
                            let key_value = binding.curve.sample(time).unwrap_or(binding.input_min);
                            if crate::ui::kui::button(
                                ui,
                                "キーを追加／更新",
                                crate::ui::kui::ButtonKind::Quiet,
                            )
                            .on_hover_text("現在のフレームにキーを追加または更新")
                            .clicked()
                            {
                                let _ = binding.curve.upsert_point(time, key_value);
                            }
                            if crate::ui::kui::button(
                                ui,
                                "キーを削除",
                                crate::ui::kui::ButtonKind::Quiet,
                            )
                            .on_hover_text("現在のフレームのキーを削除")
                            .clicked()
                            {
                                let _ = binding.curve.remove_point_at(time);
                            }
                        }
                    });
                    if let Err(error) = binding.validate() {
                        crate::ui::kui::error_text(ui, error);
                    }
                });
                crate::ui::kui::caption(
                    ui,
                    format!("キーフレーム　{} 件", binding.curve.points.len()),
                );
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for (key_index, point) in binding.curve.points.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                crate::ui::kui::caption(ui, format!("#{key_index}"));
                                if crate::ui::kui::tool_button(ui, false, "前へ").clicked() {
                                    move_key = Some((index, point.time, -1));
                                }
                                ui.add(
                                    crate::ui::kui::drag(&mut point.time.numerator)
                                        .speed(1.0)
                                        .width(68.0),
                                );
                                crate::ui::kui::caption(ui, "/");
                                ui.add(
                                    crate::ui::kui::drag(&mut point.time.denominator)
                                        .speed(1.0)
                                        .range(1..=u32::MAX)
                                        .width(56.0),
                                );
                                ui.add(
                                    crate::ui::kui::drag(&mut point.value)
                                        .speed(0.01)
                                        .width(76.0),
                                );
                                if crate::ui::kui::tool_button(ui, false, "次へ").clicked() {
                                    move_key = Some((index, point.time, 1));
                                }
                            });
                        }
                    });
                crate::ui::kui::section_title(ui, "カーブプレビュー");
                let (plot_rect, plot_response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 110.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(plot_rect);
                painter.rect_filled(plot_rect, 3.0, colors::BG_PANEL);
                painter.rect_stroke(
                    plot_rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
                );
                if !binding.curve.points.is_empty() {
                    let min_time = binding
                        .curve
                        .points
                        .first()
                        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
                        .unwrap_or(0.0);
                    let max_time = binding
                        .curve
                        .points
                        .last()
                        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
                        .unwrap_or(min_time + 1.0)
                        .max(min_time + f32::EPSILON);
                    let min_value = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| point.value as f32)
                        .fold(f32::INFINITY, f32::min);
                    let max_value = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| point.value as f32)
                        .fold(f32::NEG_INFINITY, f32::max)
                        .max(min_value + f32::EPSILON);
                    let to_screen = |time: f32, value: f32| {
                        egui::pos2(
                            plot_rect.left()
                                + ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0)
                                    * plot_rect.width(),
                            plot_rect.bottom()
                                - ((value - min_value) / (max_value - min_value)).clamp(0.0, 1.0)
                                    * plot_rect.height(),
                        )
                    };
                    let sampled: Vec<_> = (0..=64)
                        .filter_map(|sample| {
                            let normalized = sample as f32 / 64.0;
                            let time = min_time + normalized * (max_time - min_time);
                            let rational_time = crate::core::unified_time::Time::new(
                                (time * 1_000_000.0).round() as i64,
                                1_000_000,
                            );
                            binding
                                .curve
                                .sample(rational_time)
                                .map(|value| to_screen(time, value as f32))
                        })
                        .collect();
                    for segment in sampled.windows(2) {
                        painter.line_segment(
                            [segment[0], segment[1]],
                            egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE),
                        );
                    }
                    let key_points: Vec<_> = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| {
                            to_screen(
                                point.time.numerator as f32 / point.time.denominator as f32,
                                point.value as f32,
                            )
                        })
                        .collect();
                    for point in key_points {
                        painter.circle_filled(point, 3.5, colors::TIMELINE_KEYFRAME);
                    }

                    if plot_response.dragged() {
                        if let Some(pointer) = plot_response.interact_pointer_pos() {
                            let normalized_time = ((pointer.x - plot_rect.left())
                                / plot_rect.width().max(1.0))
                            .clamp(0.0, 1.0);
                            let normalized_value = ((plot_rect.bottom() - pointer.y)
                                / plot_rect.height().max(1.0))
                            .clamp(0.0, 1.0);
                            let target_time = min_time + normalized_time * (max_time - min_time);
                            let nearest = binding
                                .curve
                                .points
                                .iter()
                                .enumerate()
                                .min_by(|(_, left), (_, right)| {
                                    let left_time =
                                        left.time.numerator as f32 / left.time.denominator as f32;
                                    let right_time =
                                        right.time.numerator as f32 / right.time.denominator as f32;
                                    (left_time - target_time)
                                        .abs()
                                        .total_cmp(&(right_time - target_time).abs())
                                })
                                .map(|(index, point)| (index, point.time));
                            if let Some((index, from)) = nearest {
                                if ui.input(|input| input.modifiers.shift) {
                                    let new_time = crate::core::unified_time::Time::new(
                                        (target_time * 1_000_000.0).round() as i64,
                                        1_000_000,
                                    );
                                    if new_time != from {
                                        let _ = binding.curve.move_point(from, new_time);
                                    }
                                } else if let Some(point) = binding.curve.points.get_mut(index) {
                                    point.value = (min_value
                                        + normalized_value * (max_value - min_value))
                                        as f64;
                                }
                            }
                        }
                    }
                    if plot_response.double_clicked() {
                        if let Some(pointer) = plot_response.interact_pointer_pos() {
                            let normalized_time = ((pointer.x - plot_rect.left())
                                / plot_rect.width().max(1.0))
                            .clamp(0.0, 1.0);
                            let normalized_value = ((plot_rect.bottom() - pointer.y)
                                / plot_rect.height().max(1.0))
                            .clamp(0.0, 1.0);
                            let time = min_time + normalized_time * (max_time - min_time);
                            let value = min_value + normalized_value * (max_value - min_value);
                            let _ = binding.curve.upsert_point(
                                crate::core::unified_time::Time::new(
                                    (time * 1_000_000.0).round() as i64,
                                    1_000_000,
                                ),
                                value as f64,
                            );
                        }
                    }
                }
            }
        });
    if let Some(index) = remove_index {
        if let Some(document) = app.production_document.as_mut() {
            if index < document.bindings.len() {
                document.bindings.remove(index);
                if selected_binding == Some(index) {
                    selected_binding = None;
                } else if selected_binding.is_some_and(|selected| selected > index) {
                    selected_binding = selected_binding.map(|selected| selected - 1);
                }
                app.toasts.info("バインディングを削除しました");
            }
        }
    }
    if let Some(before) = before_bindings {
        app.record_automation_edit(before);
    }
    if let Some((index, from, frames)) = move_key {
        if let (Some(document), Some(rate)) = (app.production_document.as_mut(), frame_rate) {
            if let Some(binding) = document.bindings.get_mut(index) {
                let _ = binding.curve.move_point_by_frames(from, frames, rate);
            }
        }
    }
    if undo_requested {
        app.undo_automation_edit();
    } else if redo_requested {
        app.redo_automation_edit();
    }
    app.show_automation_panel = open;
    app.selected_automation_binding = selected_binding.filter(|index| {
        app.production_document
            .as_ref()
            .is_some_and(|document| *index < document.bindings.len())
    });
}

#[cfg(test)]
mod tests {
    use super::draw;
    use crate::core::automation_binding::{AutomationBinding, AutomationCurve, AutomationPoint};
    use crate::core::production_document::ProductionDocument;
    use crate::core::unified_time::Time;
    use crate::KagariApp;
    use eframe::egui;

    #[test]
    fn selected_binding_detail_renders_with_editable_curve() {
        let mut app = KagariApp {
            show_automation_panel: true,
            ..Default::default()
        };
        let project = app.history.current().clone();
        let mut document = ProductionDocument::new(project);
        document.bindings.push(AutomationBinding {
            source: "audio.source".into(),
            target: "layer.scale.x".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.5,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 2.0,
        });
        app.production_document = Some(document);
        app.selected_automation_binding = Some(0);

        let context = egui::Context::default();
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                ..Default::default()
            },
            |context| {
                crate::ui::theme::configure_ae_theme(context);
                draw(&mut app, context);
            },
        );

        assert_eq!(app.selected_automation_binding, Some(0));
        assert_eq!(
            app.production_document
                .as_ref()
                .map(|document| document.bindings.len()),
            Some(1)
        );
    }
}
