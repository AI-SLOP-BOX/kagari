use crate::core::timeline::{Layer, LayerType};
use crate::ui::kui;
use eframe::egui;

pub(crate) fn draw_shape_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) -> bool {
    if let LayerType::Shape {
        shape_type,
        color,
        stroke_color,
        stroke_width,
        ..
    } = &mut layer.layer_type
    {
        ui.label(format!("シェイプ: {:?}", shape_type));
        let mut c_arr = *color;
        if kui::property(ui, "塗りつぶしの色", |ui| kui::color_rgba(ui, &mut c_arr)).changed() {
            *color = c_arr;
            *project_changed = true;
        }
        let mut sc_arr = *stroke_color;
        if kui::property(ui, "線の色", |ui| kui::color_rgba(ui, &mut sc_arr)).changed() {
            *stroke_color = sc_arr;
            *project_changed = true;
        }
        *project_changed |= kui::property(ui, "線幅", |ui| {
            kui::number(ui, stroke_width, 0.0..=50.0, 0.5, " px")
        })
        .changed();

        // ── Trim Paths (AE Shape Operator) ──
        ui.add_space(4.0);
        ui.collapsing("パスのトリミング", |ui| {
            let has_trim = layer.trim_paths.is_some();
            if !has_trim {
                if kui::button(ui, "+ パスのトリミングを追加", kui::ButtonKind::Default).clicked() {
                    layer.trim_paths = Some(crate::core::timeline::TrimPaths::default());
                    *project_changed = true;
                }
            } else if kui::button(ui, "削除", kui::ButtonKind::Quiet).clicked() {
                layer.trim_paths = None;
                *project_changed = true;
            }

            if let Some(ref mut trim) = layer.trim_paths {
                let mut start_val = trim.start.evaluate(current_frame);
                if kui::property(ui, "開始", |ui| {
                    kui::slider(ui, &mut start_val, 0.0..=100.0, "%")
                })
                .changed()
                {
                    trim.start = crate::core::property::Animatable::new_constant(start_val);
                    *project_changed = true;
                }

                let mut end_val = trim.end.evaluate(current_frame);
                if kui::property(ui, "終了", |ui| {
                    kui::slider(ui, &mut end_val, 0.0..=100.0, "%")
                })
                .changed()
                {
                    trim.end = crate::core::property::Animatable::new_constant(end_val);
                    *project_changed = true;
                }

                let mut offset_val = trim.offset.evaluate(current_frame);
                if kui::property(ui, "オフセット", |ui| {
                    ui.add(kui::drag(&mut offset_val).unit("°"))
                })
                .changed()
                {
                    trim.offset = crate::core::property::Animatable::new_constant(offset_val);
                    *project_changed = true;
                }
            }
        });

        // ── Merge Paths & Offset Paths (AE Vector Operators) ──
        ui.collapsing("ベクターパス演算", |ui| {
            let mut merge_mode = ui.ctx().data(|d| {
                d.get_temp::<i32>(egui::Id::new("shape_merge_mode"))
                    .unwrap_or(0)
            });
            let merge_options = [
                (0, "追加 (合成)"),
                (1, "前面オブジェクトで型抜き"),
                (2, "交差"),
                (3, "中マド (排他)"),
            ];
            if kui::property(ui, "パスの結合モード", |ui| {
                kui::combo(
                    ui,
                    "shape_merge_mode_combo",
                    &mut merge_mode,
                    &merge_options,
                )
            }) {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("shape_merge_mode"), merge_mode));
                *project_changed = true;
            }

            let mut offset_dist = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("shape_offset_path_amt"))
                    .unwrap_or(0.0)
            });
            if kui::property(ui, "パスのオフセット", |ui| {
                kui::number(ui, &mut offset_dist, -100.0..=100.0, 0.5, " px")
            })
            .changed()
            {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("shape_offset_path_amt"), offset_dist)
                });
                *project_changed = true;
            }
        });

        // ── Repeater (AE Shape Modifier) ──
        ui.collapsing("リピーター", |ui| {
            let mut has_rep = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("has_shape_repeater"))
                    .unwrap_or(false)
            });
            if kui::toggle(ui, &mut has_rep, "リピーターを有効化").changed() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("has_shape_repeater"), has_rep));
                *project_changed = true;
            }

            if has_rep {
                let mut copies = ui
                    .ctx()
                    .data(|d| d.get_temp::<u32>(egui::Id::new("rep_copies")).unwrap_or(3));
                let mut offset = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("rep_offset"))
                        .unwrap_or(0.0)
                });
                let mut rep_pos_x = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("rep_pos_x"))
                        .unwrap_or(100.0)
                });
                let mut rep_pos_y = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(egui::Id::new("rep_pos_y")).unwrap_or(0.0));
                let mut rep_rot = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(egui::Id::new("rep_rot")).unwrap_or(0.0));
                let mut rep_scale = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("rep_scale"))
                        .unwrap_or(100.0)
                });

                if kui::property(ui, "Copies", |ui| {
                    ui.add(kui::drag(&mut copies).range(1..=100))
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("rep_copies"), copies));
                    *project_changed = true;
                }
                if kui::property(ui, "Offset", |ui| ui.add(kui::drag(&mut offset).speed(0.1)))
                    .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("rep_offset"), offset));
                    *project_changed = true;
                }

                ui.collapsing("Transform: Repeater", |ui| {
                    let mut rep_position = [rep_pos_x, rep_pos_y];
                    if kui::property(ui, "Position", |ui| {
                        kui::vector_axes_unbounded(ui, &mut rep_position, 1.0, "", ["X", "Y"])
                    })
                    .changed()
                    {
                        rep_pos_x = rep_position[0];
                        rep_pos_y = rep_position[1];
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("rep_pos_x"), rep_pos_x);
                            d.insert_temp(egui::Id::new("rep_pos_y"), rep_pos_y);
                        });
                        *project_changed = true;
                    }
                    if kui::property(ui, "Rotation", |ui| {
                        ui.add(kui::drag(&mut rep_rot).unit("°"))
                    })
                    .changed()
                    {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("rep_rot"), rep_rot));
                        *project_changed = true;
                    }
                    if kui::property(ui, "Scale", |ui| {
                        ui.add(kui::drag(&mut rep_scale).unit("%"))
                    })
                    .changed()
                    {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("rep_scale"), rep_scale));
                        *project_changed = true;
                    }
                });
            }
        });

        // ── Zig Zag & Round Corners (AE Vector Modifiers) ──
        ui.collapsing("Zig Zag & Round Corners", |ui| {
            let mut has_zigzag = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("has_shape_zigzag"))
                    .unwrap_or(false)
            });
            ui.horizontal(|ui| {
                if kui::toggle(ui, &mut has_zigzag, "Enable Zig Zag").changed() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("has_shape_zigzag"), has_zigzag));
                    *project_changed = true;
                }
            });
            if has_zigzag {
                let mut zz_size = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(egui::Id::new("zz_size")).unwrap_or(10.0));
                let mut zz_ridges = ui
                    .ctx()
                    .data(|d| d.get_temp::<u32>(egui::Id::new("zz_ridges")).unwrap_or(4));
                let mut zz_smooth = ui.ctx().data(|d| {
                    d.get_temp::<bool>(egui::Id::new("zz_smooth"))
                        .unwrap_or(false)
                });

                if kui::property(ui, "Size", |ui| {
                    kui::number(ui, &mut zz_size, 0.0..=200.0, 0.5, " px")
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("zz_size"), zz_size));
                    *project_changed = true;
                }
                if kui::property(ui, "Ridges", |ui| {
                    ui.add(kui::drag(&mut zz_ridges).range(1..=50))
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("zz_ridges"), zz_ridges));
                    *project_changed = true;
                }
                if kui::property(ui, "Points", |ui| {
                    kui::combo(
                        ui,
                        "shape-zigzag-points",
                        &mut zz_smooth,
                        &[(false, "Corner"), (true, "Smooth")],
                    )
                }) {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("zz_smooth"), zz_smooth));
                    *project_changed = true;
                }
            }

            ui.separator();
            let mut round_radius = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("shape_round_radius"))
                    .unwrap_or(0.0)
            });
            if kui::property(ui, "Round Corners", |ui| {
                kui::number(ui, &mut round_radius, 0.0..=100.0, 0.5, " px")
            })
            .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("shape_round_radius"), round_radius));
                *project_changed = true;
            }

            // ── Pucker & Bloat (AE Vector Modifier) ──
            ui.separator();
            let mut pucker_bloat = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("shape_pucker_bloat"))
                    .unwrap_or(0.0)
            });
            if kui::property(ui, "Pucker & Bloat", |ui| {
                kui::slider(ui, &mut pucker_bloat, -100.0..=100.0, "%")
            })
            .changed()
            {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("shape_pucker_bloat"), pucker_bloat));
                *project_changed = true;
            }

            // ── 〰 Wiggle Paths (AE Vector Modifier) ──
            ui.separator();
            let mut has_wiggle_paths = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("shape_has_wiggle"))
                    .unwrap_or(false)
            });
            ui.horizontal(|ui| {
                if kui::toggle(ui, &mut has_wiggle_paths, "Wiggle Paths").changed() {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("shape_has_wiggle"), has_wiggle_paths)
                    });
                    *project_changed = true;
                }
            });
            if has_wiggle_paths {
                let mut wp_size = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(egui::Id::new("wp_size")).unwrap_or(15.0));
                let mut wp_detail = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(egui::Id::new("wp_detail")).unwrap_or(5.0));
                let mut wp_temporal = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("wp_temporal"))
                        .unwrap_or(2.0)
                });

                if kui::property(ui, "Size", |ui| {
                    ui.add(kui::drag(&mut wp_size).speed(0.5).unit(" px"))
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("wp_size"), wp_size));
                    *project_changed = true;
                }
                if kui::property(ui, "Detail", |ui| {
                    kui::number(ui, &mut wp_detail, 1.0..=50.0, 0.5, "")
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("wp_detail"), wp_detail));
                    *project_changed = true;
                }
                if kui::property(ui, "Wiggles/sec", |ui| {
                    kui::number(ui, &mut wp_temporal, 0.1..=20.0, 0.1, " Hz")
                })
                .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("wp_temporal"), wp_temporal));
                    *project_changed = true;
                }
            }
        });
        return true;
    }
    false
}
