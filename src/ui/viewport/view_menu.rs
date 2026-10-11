use super::*;

pub(super) fn draw_view_menu_contents(ui: &mut egui::Ui, app: &mut KagariApp) {
    ui.checkbox(&mut app.show_handles, "レイヤーハンドル");
    ui.checkbox(&mut app.show_guides, "セーフエリアガイド");
    ui.checkbox(&mut app.show_grid, "グリッド");
    ui.checkbox(&mut app.viewport_show_stats, "プレビュー情報");
    ui.separator();
    ui.collapsing("プレビュー品質", |ui| {
        crate::ui::proxy_panel::draw_proxy_controls(app, ui);
    });
    let navigator_id = egui::Id::new(crate::ui::viewport_minimap::SHOW_NAVIGATOR_ID);
    let mut show_navigator = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(navigator_id).unwrap_or(false));
    if ui
        .checkbox(&mut show_navigator, "コンポジションナビゲーター")
        .changed()
    {
        ui.ctx()
            .data_mut(|data| data.insert_temp(navigator_id, show_navigator));
    }
    if let Some(idx) = app.selection.selected_layer_idx {
        ui.separator();
        ui.menu_button("アンカースナップ", |ui| {
            for (label, x, y) in [
                ("左上", 0.0, 0.0),
                ("上中央", 0.5, 0.0),
                ("右上", 1.0, 0.0),
                ("左中央", 0.0, 0.5),
                ("中央", 0.5, 0.5),
                ("右中央", 1.0, 0.5),
                ("左下", 0.0, 1.0),
                ("下中央", 0.5, 1.0),
                ("右下", 1.0, 1.0),
            ] {
                if ui.button(label).clicked() {
                    app.modify_project(|project| {
                        let comp = project.active_composition_mut();
                        if let Some(layer) = comp.layers.get_mut(idx) {
                            let [width, height] = layer.bounding_size();
                            layer.transform.anchor_point =
                                crate::core::property::Animatable::new_constant([
                                    width * x,
                                    height * y,
                                ]);
                        }
                    });
                    ui.close_menu();
                }
            }
        });
    }
    {
        ui.separator();
        ui.label(
            egui::RichText::new("ビューアー")
                .small()
                .color(colors::TEXT_MUTED),
        );
        egui::ComboBox::from_id_salt("compact_cam_view_menu")
            .selected_text(match app.viewport_cam_view {
                0 => "アクティブカメラ",
                1 => "正面",
                2 => "左面",
                3 => "上面",
                _ => "カスタムビュー",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut app.viewport_cam_view, 0, "アクティブカメラ")
                    .clicked()
                {
                    app.viewport_mode = ViewportMode::Comp2D;
                }
                if ui
                    .selectable_value(&mut app.viewport_cam_view, 1, "正面")
                    .clicked()
                {
                    app.viewport_mode = ViewportMode::Camera3D;
                    app.camera_orbit = (0.0, 0.0, 1000.0);
                }
                if ui
                    .selectable_value(&mut app.viewport_cam_view, 2, "左面")
                    .clicked()
                {
                    app.viewport_mode = ViewportMode::Camera3D;
                    app.camera_orbit = (-90.0, 0.0, 1000.0);
                }
                if ui
                    .selectable_value(&mut app.viewport_cam_view, 3, "上面")
                    .clicked()
                {
                    app.viewport_mode = ViewportMode::Camera3D;
                    app.camera_orbit = (0.0, -89.0, 1000.0);
                }
            });
        egui::ComboBox::from_id_salt("compact_resolution_menu")
            .selected_text(match app.viewport_render_resolution {
                crate::app_state::PreviewResolution::Full => "フル",
                crate::app_state::PreviewResolution::Half => "1/2",
                crate::app_state::PreviewResolution::Third => "1/3",
                crate::app_state::PreviewResolution::Quarter => "1/4",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut app.viewport_render_resolution,
                    crate::app_state::PreviewResolution::Full,
                    "フル",
                );
                ui.selectable_value(
                    &mut app.viewport_render_resolution,
                    crate::app_state::PreviewResolution::Half,
                    "1/2",
                );
                ui.selectable_value(
                    &mut app.viewport_render_resolution,
                    crate::app_state::PreviewResolution::Third,
                    "1/3",
                );
                ui.selectable_value(
                    &mut app.viewport_render_resolution,
                    crate::app_state::PreviewResolution::Quarter,
                    "1/4",
                );
            });
        egui::ComboBox::from_id_salt("compact_channel_menu")
            .selected_text(match app.viewport_color_channel {
                0 => "RGB",
                1 => "赤",
                2 => "緑",
                3 => "青",
                _ => "アルファ",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.viewport_color_channel, 0, "RGB");
                ui.selectable_value(&mut app.viewport_color_channel, 1, "赤");
                ui.selectable_value(&mut app.viewport_color_channel, 2, "緑");
                ui.selectable_value(&mut app.viewport_color_channel, 3, "青");
                ui.selectable_value(&mut app.viewport_color_channel, 4, "アルファ");
            });
        egui::ComboBox::from_id_salt("compact_quality_menu")
            .selected_text(match app.viewport_fast_preview {
                0 => "最終品質",
                1 => "適応解像度",
                2 => "高速ドラフト",
                _ => "ワイヤーフレーム",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.viewport_fast_preview, 0, "最終品質");
                ui.selectable_value(&mut app.viewport_fast_preview, 1, "適応解像度");
                ui.selectable_value(&mut app.viewport_fast_preview, 2, "高速ドラフト");
                ui.selectable_value(&mut app.viewport_fast_preview, 3, "ワイヤーフレーム");
            });
        if ui.button("スナップショットを撮影").clicked() {
            let frame = app.playback.current_frame;
            crate::ui::viewport_state::set_snap_frame(ui.ctx(), frame);
            app.toasts
                .info(format!("Snapshot saved at frame {}", frame));
            ui.close_menu();
        }
        let comparing = crate::ui::viewport_state::is_comparing(ui.ctx());
        if ui
            .selectable_label(comparing, "スナップショットを比較")
            .clicked()
        {
            if crate::ui::viewport_state::snap_frame(ui.ctx()).is_some() {
                crate::ui::viewport_state::toggle_comparing(ui.ctx());
            } else {
                app.toasts.warning("先にスナップショットを撮影してください");
            }
        }
    }
}
