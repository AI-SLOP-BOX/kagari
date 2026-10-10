//! Render logic and floating step dialog for the interactive tutorial.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::steps::{chapter_name, steps};

pub enum StepAction {
    Next,
    Prev,
    Close,
    ShowChapterSelect,
}

pub fn draw_tutorial_card(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(state) = app.tutorial.as_mut() else {
        return;
    };
    if state.completed {
        return;
    }

    if state.show_chapter_select {
        draw_chapter_select(app, ctx);
        return;
    }

    let all = steps();
    let step_idx = state.current_step.min(all.len() - 1);
    let step = &all[step_idx];

    let mut action: Option<StepAction> = None;
    let mut open_flag = true;

    crate::ui::modal::window(
        egui::RichText::new(step.title.to_string())
            .strong()
            .color(colors::TEXT_PRIMARY),
    )
    .open(&mut open_flag)
    .default_width(440.0)
    .min_width(340.0)
    .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
    .collapsible(false)
    .resizable(false)
    .frame(egui::Frame::window(&ctx.style()).fill(crate::ui::theme::colors::BG_EXTREME))
    .show(ctx, |ui| {
        // Chapter badge
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(chapter_name(step.chapter))
                    .small()
                    .color(crate::ui::theme::colors::ACCENT_BLUE_LIGHT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("章一覧")
                    .on_hover_text("チュートリアルの章一覧を表示")
                    .clicked()
                {
                    action = Some(StepAction::ShowChapterSelect);
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);

        // Body text
        for &line in step.body {
            if line.is_empty() {
                ui.add_space(4.0);
            } else {
                ui.label(
                    egui::RichText::new(line)
                        .size(13.0)
                        .color(colors::TEXT_PRIMARY),
                );
            }
        }

        ui.add_space(10.0);

        // Shortcut badge
        if let Some(shortcut_label) = step.shortcut.label() {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("ショートカット: ")
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(shortcut_label)
                        .small()
                        .strong()
                        .color(egui::Color32::from_rgb(255, 220, 100)),
                );
            });
        }

        // Hint
        ui.add_space(4.0);
        egui::Frame::default()
            .fill(crate::ui::theme::colors::TUTORIAL_HINT_BACKGROUND)
            .rounding(6.0)
            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(step.hint.to_string())
                        .size(12.5)
                        .color(crate::ui::theme::colors::ACCENT_GREEN),
                );
            });

        // Advanced tip (collapsed)
        if !step.advanced_tip.is_empty() {
            ui.add_space(4.0);
            ui.collapsing(
                egui::RichText::new("上級者向けヒント")
                    .small()
                    .color(crate::ui::theme::colors::ACCENT_PURPLE),
                |ui| {
                    ui.label(
                        egui::RichText::new(step.advanced_tip)
                            .small()
                            .color(colors::TEXT_SECONDARY),
                    );
                },
            );
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("スキップ (終了)").clicked() {
                action = Some(StepAction::Close);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_last = step_idx + 1 >= all.len();
                if is_last {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("完了")
                                    .strong()
                                    .color(crate::ui::theme::colors::ACCENT_GREEN),
                            )
                            .fill(crate::ui::theme::colors::TUTORIAL_COMPLETE_BACKGROUND),
                        )
                        .clicked()
                    {
                        action = Some(StepAction::Close);
                    }
                } else if ui
                    .add(egui::Button::new(egui::RichText::new("次へ ▶").strong()))
                    .clicked()
                {
                    action = Some(StepAction::Next);
                }

                if step_idx > 0 && ui.button("◀ 戻る").clicked() {
                    action = Some(StepAction::Prev);
                }

                // Step counter
                ui.label(
                    egui::RichText::new(format!("{}/{}", step_idx + 1, all.len()))
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
            });
        });

        // Progress bar (segmented dots)
        ui.add_space(6.0);
        let dot_w = 20.0;
        let dot_h = 4.0;
        let gap = 3.0;
        let total_w = all.len() as f32 * (dot_w + gap) - gap;
        let (_, bar_rect) = ui.allocate_space(egui::vec2(total_w, dot_h + 4.0));
        let painter = ui.painter();
        for (i, _) in all.iter().enumerate() {
            let x = bar_rect.min.x + i as f32 * (dot_w + gap);
            let r = egui::Rect::from_min_size(
                egui::pos2(x, bar_rect.min.y + 2.0),
                egui::vec2(dot_w, dot_h),
            );
            let color = if i == step_idx {
                crate::ui::theme::colors::ACCENT_BLUE_LIGHT
            } else if i < step_idx {
                egui::Color32::from_rgb(60, 120, 200)
            } else {
                egui::Color32::from_rgb(50, 50, 60)
            };
            painter.rect_filled(r, 2.0, color);
        }
    });

    if !open_flag && action.is_none() {
        action = Some(StepAction::Close);
    }

    apply_action(app, action);
}

pub fn draw_chapter_select(app: &mut KagariApp, ctx: &egui::Context) {
    let all = steps();
    let mut close = false;
    let mut jump_to: Option<usize> = None;

    crate::ui::modal::window("Tutorial — Chapters")
        .default_width(480.0)
        .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
        .collapsible(false)
        .resizable(false)
        .frame(egui::Frame::window(&ctx.style()).fill(crate::ui::theme::colors::BG_EXTREME))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("学びたい章をクリックして直接ジャンプできます。")
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(8.0);

            // Group by chapter name
            let chapters = [
                ("Chapter 1: インターフェース", 0usize, 2usize),
                ("Chapter 2: コンポジションとレイヤー", 2, 4),
                ("Chapter 3: アニメーションとキーフレーム", 4, 6),
                ("Chapter 4: エフェクト", 6, 8),
                ("Chapter 5: 3D とアドバンスド機能", 8, 10),
                ("Chapter 6: 書き出しと公開", 10, 12),
            ];

            let current = app.tutorial.as_ref().map_or(0, |s| s.current_step);

            for (chapter_name, start, end) in chapters {
                let done = current >= end;
                let active = current >= start && current < end;
                let chapter_color = if active {
                    crate::ui::theme::colors::ACCENT_BLUE_LIGHT
                } else if done {
                    egui::Color32::from_rgb(60, 120, 60)
                } else {
                    egui::Color32::from_rgb(100, 100, 120)
                };

                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(
                        if active { 28 } else { 20 },
                        if active { 35 } else { 22 },
                        if active { 55 } else { 30 },
                    ))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let badge = if done {
                                "✅"
                            } else if active {
                                "▶"
                            } else {
                                "○"
                            };
                            ui.label(egui::RichText::new(badge).color(chapter_color));
                            ui.label(
                                egui::RichText::new(chapter_name)
                                    .strong()
                                    .color(chapter_color),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    for (step_i, s) in all.iter().enumerate().take(end).skip(start)
                                    {
                                        if ui
                                            .small_button(format!("{}→", step_i + 1))
                                            .on_hover_text(s.title)
                                            .clicked()
                                        {
                                            jump_to = Some(step_i);
                                        }
                                    }
                                },
                            );
                        });
                    });

                ui.add_space(4.0);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("閉じる").clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("最初から ↺").clicked() {
                        jump_to = Some(0);
                    }
                });
            });
        });

    if let Some(idx) = jump_to {
        if let Some(s) = app.tutorial.as_mut() {
            s.current_step = idx;
            s.completed = false;
            s.show_chapter_select = false;
        }
    } else if close {
        if let Some(s) = app.tutorial.as_mut() {
            s.show_chapter_select = false;
        }
    }
}

pub fn apply_action(app: &mut KagariApp, action: Option<StepAction>) {
    let all = steps();
    match action {
        Some(StepAction::Next) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.current_step = (s.current_step + 1).min(all.len() - 1);
            }
        }
        Some(StepAction::Prev) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.current_step = s.current_step.saturating_sub(1);
            }
        }
        Some(StepAction::Close) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.completed = true;
            }
            app.toasts.info("チュートリアル完了！Have fun animating");
        }
        Some(StepAction::ShowChapterSelect) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.show_chapter_select = true;
            }
        }
        None => {}
    }
}
