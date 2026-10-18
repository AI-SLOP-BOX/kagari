#![allow(clippy::field_reassign_with_default)]

use crate::core::timeline::{Composition, LayerType};
use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

pub mod presets;
pub use presets::{apply_text_preset, draw_animation_presets};

pub fn draw_character_panel(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    comp: &mut Composition,
    _current_frame: u32,
) -> bool {
    let mut project_changed = false;
    let sel_idx = app.selection.selected_layer_idx;

    ui.vertical(|ui| {
        kui::section(ui, "文字・段落");

        if let Some(idx) = sel_idx {
            if idx < comp.layers.len() {
                let layer = &mut comp.layers[idx];
                if let LayerType::Text {
                    text,
                    font_size,
                    color,
                    font_family,
                    tracking,
                    leading,
                    align,
                    stroke_color,
                    stroke_width,
                    text_on_path: _,
                } = &mut layer.layer_type
                {
                    kui::panel(ui, |ui| {
                        kui::section(ui, "文字プロパティ");
                        let grid = kui::PropertyGrid::new(ui, &["テキスト", "フォント", "サイズ", "トラッキング", "行送り", "塗り", "境界線", "境界色"]);
                        project_changed |= grid.row(ui, "テキスト", |ui| kui::text_field(ui, text)).changed();
                        grid.row(ui, "フォント", |ui| {
                            let custom_font = font_family.clone();
                            let names = ["Inter", "Roboto", "Helvetica", "Arial", "Courier New", "Impact", "Times New Roman"];
                            let mut options: Vec<(String, &str)> = names.iter().map(|name| ((*name).to_owned(), *name)).collect();
                            if !names.contains(&custom_font.as_str()) { options.push((custom_font.clone(), custom_font.as_str())); }
                            project_changed |= kui::combo(ui, "ae_text_font_family", font_family, &options);
                        });
                        project_changed |= grid.row(ui, "サイズ", |ui| ui.add(kui::drag(font_size).range(6..=300).unit(" px"))).changed();
                        project_changed |= grid.row(ui, "トラッキング", |ui| ui.add(kui::drag(tracking).speed(1.0).range(-100.0..=500.0).unit(" VA"))).changed();
                        project_changed |= grid.row(ui, "行送り", |ui| ui.add(kui::drag(leading).speed(0.05).range(0.5..=3.0).unit(" em"))).changed();
                        kui::section(ui, "カラー・境界線");
                        project_changed |= grid.row(ui, "塗り", |ui| kui::color_rgba(ui, color)).changed();
                        project_changed |= grid.row(ui, "境界線の幅", |ui| ui.add(kui::drag(stroke_width).speed(0.5).range(0.0..=50.0).unit(" px"))).changed();
                        if *stroke_width > 0.0 {
                            project_changed |= grid.row(ui, "境界線の色", |ui| kui::color_rgba(ui, stroke_color)).changed();
                        }

                        kui::collapsing_section(ui, "character_font_features", "フォント機能", false, |ui| {
                            kui::secondary(ui, "可変フォント軸とオプティカルカーニングは、現在のテキスト描画では未対応です。");
                        });
                    });

                    ui.add_space(kui::space::SM);
                    kui::panel(ui, |ui| {
                        kui::section(ui, "段落の配置");
                        ui.horizontal(|ui| {
                            for (value, glyph, hint) in [
                                (0, "☰", "左揃え"),
                                (1, "≡", "中央揃え"),
                                (2, "☷", "右揃え"),
                            ] {
                                if kui::tool_button(ui, *align == value, glyph)
                                    .on_hover_text(hint)
                                    .clicked()
                                {
                                    *align = value;
                                    project_changed = true;
                                }
                            }
                        });
                    });

                    // ── 〰 Path Options (Text on Path) ──
                    ui.add_space(kui::space::SM);
                    kui::collapsing_section(ui, "character_text_on_path", "パスに沿ったテキスト", false, |ui| {
                        let mut path_idx = ui.ctx().data(|d| d.get_temp::<i32>(egui::Id::new("top_path_idx")).unwrap_or(0));
                        let path_options = [(0_i32, "なし"), (1, "マスク1（ベジェ）")];
                        let path_grid = kui::PropertyGrid::new(ui, &["パスマスク"]);
                        if path_grid.row(ui, "パスマスク", |ui| {
                            kui::combo(ui, "top_path_combo", &mut path_idx, &path_options)
                        }) {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("top_path_idx"), path_idx)
                            });
                            project_changed = true;
                        }

                        if path_idx > 0 {
                            let mut rev = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("top_rev_path")).unwrap_or(false));
                            let mut perp = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("top_perp_path")).unwrap_or(true));
                            let mut force = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("top_force_align")).unwrap_or(false));
                            let mut first_margin = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("top_first_margin")).unwrap_or(0.0));
                            let mut last_margin = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("top_last_margin")).unwrap_or(0.0));

                            ui.horizontal(|ui| {
                                if kui::toggle(ui, &mut rev, "パスを反転").changed() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("top_rev_path"), rev)); project_changed = true; }
                                if kui::toggle(ui, &mut perp, "垂直方向に配置").changed() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("top_perp_path"), perp)); project_changed = true; }
                                if kui::toggle(ui, &mut force, "強制整列").changed() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("top_force_align"), force)); project_changed = true; }
                            });

                            let margin_grid = kui::PropertyGrid::new(ui, &["開始マージン", "終了マージン"]);
                            if margin_grid.row(ui, "開始マージン", |ui| {
                                ui.add(kui::drag(&mut first_margin).speed(1.0).unit(" px"))
                            }).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("top_first_margin"), first_margin));
                                    project_changed = true;
                            }
                            if margin_grid.row(ui, "終了マージン", |ui| {
                                ui.add(kui::drag(&mut last_margin).speed(1.0).unit(" px"))
                            }).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("top_last_margin"), last_margin));
                                    project_changed = true;
                            }
                        }
                    });

                    ui.add_space(kui::space::SM);
                    // ── AE Multi-Animator Stack (Layered Animators) ──
                    kui::panel(ui, |ui| {
                        let has_stack = layer.text_animator_stack.is_some();
                        ui.horizontal(|ui| {
                            kui::section_title(ui, "テキストアニメーター");
                            if kui::button(ui, "＋ 追加", kui::ButtonKind::Accent).on_hover_text("テキストアニメーターを追加").clicked() {
                                if layer.text_animator_stack.is_none() {
                                    layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default());
                                }
                                if let Some(ref mut stack) = layer.text_animator_stack {
                                    stack.animators.push(crate::core::text_animator_advanced::TextAnimatorAdvanced::default());
                                }
                                project_changed = true;
                            }
                            if has_stack && kui::button(ui, "スタックを消去", kui::ButtonKind::Quiet).clicked() {
                                layer.text_animator_stack = None;
                                project_changed = true;
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            kui::caption(ui, "プリセット");
                            if kui::tool_button(ui, false, "タイプライター").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.opacity = 0.0;
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Characters;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                            if kui::tool_button(ui, false, "文字ウェーブ").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.position = [0.0, -30.0];
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Characters;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                            if kui::tool_button(ui, false, "単語ホップ").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.position = [0.0, -50.0];
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Words;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                        });

                        if let Some(ref mut stack) = layer.text_animator_stack {
                            let mut to_remove = None;
                            for (ai, anim) in stack.animators.iter_mut().enumerate() {
                                ui.push_id(ai, |ui| {
                                    kui::collapsing_section(ui, ("character_animator", ai), &format!("アニメーター {}", ai + 1), false, |ui| {
                                        ui.horizontal(|ui| {
                                            if kui::toggle(ui, &mut anim.enabled, "有効").changed() { project_changed = true; }
                                            kui::caption(ui, "対象単位:");
                                            let mut unit_idx = match anim.unit {
                                                crate::core::text_animator_advanced::SelectorUnit::Characters => 0_usize,
                                                crate::core::text_animator_advanced::SelectorUnit::Words => 1,
                                                crate::core::text_animator_advanced::SelectorUnit::Lines => 2,
                                            };
                                            let unit_options = [(0_usize, "文字"), (1, "単語"), (2, "行")];
                                            if kui::combo(ui, "anim_unit", &mut unit_idx, &unit_options) {
                                                anim.unit = match unit_idx {
                                                    1 => crate::core::text_animator_advanced::SelectorUnit::Words,
                                                    2 => crate::core::text_animator_advanced::SelectorUnit::Lines,
                                                    _ => crate::core::text_animator_advanced::SelectorUnit::Characters,
                                                };
                                                project_changed = true;
                                            }
                                            if kui::button(ui, "削除", kui::ButtonKind::Quiet).clicked() { to_remove = Some(ai); }
                                        });

                                        // Range Selector
                                        kui::collapsing_section(ui, "character_animator_range", "範囲セレクター", false, |ui| {
                                            let sel = &mut anim.selector;
                                            let range_grid = kui::PropertyGrid::new(
                                                ui,
                                                &["開始", "終了", "オフセット", "形状"],
                                            );
                                            if range_grid
                                                .row(ui, "開始", |ui| {
                                                    kui::slider(ui, &mut sel.start, 0.0..=100.0, "%")
                                                })
                                                .changed()
                                            {
                                                project_changed = true;
                                            }
                                            if range_grid
                                                .row(ui, "終了", |ui| {
                                                    kui::slider(ui, &mut sel.end, 0.0..=100.0, "%")
                                                })
                                                .changed()
                                            {
                                                project_changed = true;
                                            }
                                            if range_grid
                                                .row(ui, "オフセット", |ui| {
                                                    kui::slider(ui, &mut sel.offset, -100.0..=100.0, "%")
                                                })
                                                .changed()
                                            {
                                                project_changed = true;
                                            }
                                            let mut shape_idx = match sel.shape {
                                                crate::core::text_animator::SelectorShape::Square => 0_usize,
                                                crate::core::text_animator::SelectorShape::RampUp => 1,
                                                crate::core::text_animator::SelectorShape::RampDown => 2,
                                                crate::core::text_animator::SelectorShape::Triangle => 3,
                                                crate::core::text_animator::SelectorShape::Round => 4,
                                                crate::core::text_animator::SelectorShape::Smooth => 5,
                                                crate::core::text_animator::SelectorShape::Wobble => 6,
                                                crate::core::text_animator::SelectorShape::Random => 7,
                                                crate::core::text_animator::SelectorShape::Expression => 8,
                                            };
                                            let shape_options = [
                                                (0_usize, "スクエア"), (1, "ランプ上昇"), (2, "ランプ下降"),
                                                (3, "三角"), (4, "ラウンド"), (5, "スムーズ"),
                                                (6, "揺らぎ"), (7, "ランダム"), (8, "エクスプレッション"),
                                            ];
                                            if range_grid
                                                .row(ui, "形状", |ui| {
                                                    kui::combo(ui, "anim_shape", &mut shape_idx, &shape_options)
                                                })
                                            {
                                                sel.shape = match shape_idx {
                                                    1 => crate::core::text_animator::SelectorShape::RampUp,
                                                    2 => crate::core::text_animator::SelectorShape::RampDown,
                                                    3 => crate::core::text_animator::SelectorShape::Triangle,
                                                    4 => crate::core::text_animator::SelectorShape::Round,
                                                    5 => crate::core::text_animator::SelectorShape::Smooth,
                                                    6 => crate::core::text_animator::SelectorShape::Wobble,
                                                    7 => crate::core::text_animator::SelectorShape::Random,
                                                    8 => crate::core::text_animator::SelectorShape::Expression,
                                                    _ => crate::core::text_animator::SelectorShape::Square,
                                                };
                                                project_changed = true;
                                            }
                                            ui.horizontal(|ui| {
                                                if kui::toggle(ui, &mut sel.random_order, "順序をランダム化").changed() { project_changed = true; }
                                            });
                                        });

                                        // Property targets
                                        kui::collapsing_section(ui, "character_animator_transform", "変形・詳細プロパティ", false, |ui| {
                                            let grid = kui::PropertyGrid::new(
                                                ui,
                                                &["位置", "回転", "不透明度", "ぼかし", "字間", "傾斜", "文字オフセット"],
                                            );
                                            if grid.row(ui, "位置", |ui| {
                                                kui::vector_axes_unbounded(
                                                    ui,
                                                    &mut anim.position,
                                                    1.0,
                                                    "",
                                                    ["X", "Y"],
                                                )
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "回転", |ui| {
                                                ui.add(kui::drag(&mut anim.rotation).unit("°"))
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "不透明度", |ui| {
                                                kui::slider(ui, &mut anim.opacity, 0.0..=1.0, "")
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "ぼかし", |ui| {
                                                kui::number(ui, &mut anim.blur, 0.0..=100.0, 0.1, " px")
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "字間", |ui| {
                                                ui.add(kui::drag(&mut anim.tracking).unit(" px"))
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "傾斜", |ui| {
                                                ui.add(kui::drag(&mut anim.advanced.skew).unit("°"))
                                            }).changed() {
                                                project_changed = true;
                                            }
                                            if grid.row(ui, "文字オフセット", |ui| {
                                                ui.add(kui::drag(&mut anim.advanced.character_offset).speed(1.0))
                                            }).changed() {
                                                project_changed = true;
                                            }
                                        });
                                    });
                                });
                            }
                            if let Some(r) = to_remove {
                                stack.animators.remove(r);
                                project_changed = true;
                            }
                        }
                    });

                    ui.add_space(kui::space::SM);
                    kui::panel(ui, |ui| {
                        kui::section_title(ui, "モーションプリセット");
                        ui.horizontal(|ui| {
                            if kui::button(ui, "タイプライター", ButtonKind::Default).on_hover_text("文字を順番に表示").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.opacity = 0.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Square;
                                anim.selector.start = 0.0;
                                anim.selector.end = 100.0;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if kui::button(ui, "ワードポップ", ButtonKind::Default).on_hover_text("文字を順番に弾ませて表示").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.position_offset = [0.0, 50.0];
                                anim.opacity = 0.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::RampUp;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            if kui::button(ui, "トラッキングウェーブ", ButtonKind::Default).on_hover_text("文字間隔を滑らかに広げる").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.tracking = 25.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Triangle;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if kui::button(ui, "ネオンパルス", ButtonKind::Default).on_hover_text("不透明度と文字間隔を波状に変化").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.opacity = 0.2;
                                anim.tracking = 15.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Smooth;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                        });
                    });
                } else {
                    kui::secondary(ui, "テキストレイヤーを選択すると文字設定を編集できます。");
                }
            } else {
                kui::secondary(ui, "テキストレイヤーを選択すると文字設定を編集できます。");
            }
        } else {
            kui::secondary(ui, "Select a Text Layer to edit character formatting.");
        }
    });

    // Character and Paragraph panels edit the same rendered text. Keep the
    // legacy LayerType fields in sync so switching panels cannot make one
    // panel's changes disappear behind TextFormatting overrides.
    if project_changed {
        if let Some(idx) = sel_idx {
            if let Some(layer) = comp.layers.get_mut(idx) {
                let text_values = match &layer.layer_type {
                    LayerType::Text {
                        font_family,
                        tracking,
                        leading,
                        align,
                        stroke_color,
                        stroke_width,
                        ..
                    } => Some((
                        font_family.clone(),
                        tracking,
                        leading,
                        align,
                        stroke_color,
                        stroke_width,
                    )),
                    _ => None,
                };
                if let Some((font_family, tracking, leading, align, stroke_color, stroke_width)) =
                    text_values
                {
                    let formatting = layer
                        .text_formatting
                        .get_or_insert_with(crate::core::timeline::TextFormatting::default);
                    formatting.font_family = font_family;
                    formatting.tracking = *tracking;
                    formatting.leading = *leading;
                    formatting.alignment = *align as u32;
                    formatting.stroke_color = (*stroke_width > 0.0).then_some(*stroke_color);
                    formatting.stroke_width = *stroke_width;
                }
            }
        }
    }

    project_changed
}
