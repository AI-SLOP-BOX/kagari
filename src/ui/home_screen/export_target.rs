use super::*;

pub(in crate::ui::home_screen) fn draw_export_target(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let height = ctx.screen_rect().height();
    let narrow = width < 900.0;
    let short = height < 760.0;
    let left_width = if width >= 1200.0 {
        264.0
    } else if narrow {
        68.0
    } else {
        190.0
    };
    egui::TopBottomPanel::top("export_target_window_bar")
        .exact_height(43.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_PANEL_DARK))
        .show(ctx, |ui| {
            ui.painter().text(
                egui::pos2(24.0, 22.0),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(width - 28.0, 22.0),
                egui::Align2::RIGHT_CENTER,
                "創る。合成する。輝かせる。",
                egui::FontId::proportional(13.0),
                colors::TEXT_MUTED,
            );
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(crate::ui::theme::colors::BG_PANEL_DEEP))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let sidebar =
                egui::Rect::from_min_max(r.min, egui::pos2(r.left() + left_width, r.bottom()));
            ui.painter()
                .rect_filled(sidebar, 0.0, crate::ui::theme::colors::BG_PANEL_BASE);
            ui.painter().line_segment(
                [
                    egui::pos2(sidebar.right(), r.top()),
                    egui::pos2(sidebar.right(), r.bottom()),
                ],
                egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
            );
            if app.home_banner.is_none() {
                if let Ok(img) = crate::ui::embedded_assets::open_image(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("assets/kagari_logo.webp"),
                ) {
                    app.home_banner = load_logo_texture(ctx, img);
                }
            }
            let logo_size = if narrow {
                42.0
            } else if width >= 1200.0 {
                76.0
            } else {
                58.0
            };
            if let Some(texture) = app.home_banner.as_ref() {
                let x = if narrow {
                    (left_width - logo_size) * 0.5
                } else {
                    32.0
                };
                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        egui::pos2(x, r.top() + 17.0),
                        egui::vec2(logo_size, logo_size),
                    )),
                    |icon_ui| {
                        icon_ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            egui::vec2(logo_size, logo_size),
                        )));
                    },
                );
            }
            if !narrow {
                ui.painter().text(
                    egui::pos2(108.0, r.top() + 59.0),
                    egui::Align2::LEFT_CENTER,
                    "Kagari",
                    egui::FontId::proportional(if width >= 1200.0 { 27.0 } else { 20.0 }),
                    colors::TEXT_PRIMARY,
                );
                ui.painter().text(
                    egui::pos2(if width >= 1200.0 { 190.0 } else { 167.0 }, r.top() + 59.0),
                    egui::Align2::LEFT_CENTER,
                    "VFX",
                    egui::FontId::proportional(if width >= 1200.0 { 27.0 } else { 20.0 }),
                    crate::ui::theme::colors::TEXT_SECONDARY,
                );
            }
            let nav = [
                (crate::ui::icons::SVG_HOME, "ホーム"),
                (crate::ui::icons::SVG_FOLDER, "プロジェクトを開く"),
                (crate::ui::icons::SVG_FILE_PLUS, "新規プロジェクト"),
                (crate::ui::icons::SVG_LAYERS, "編集"),
                (crate::ui::icons::SVG_EFFECTS, "エフェクト"),
                (crate::ui::icons::SVG_FOLDER, "素材"),
                (crate::ui::icons::SVG_AUDIO, "オーディオ"),
                (crate::ui::icons::SVG_TOOL_TEXT, "テキスト"),
                (crate::ui::icons::SVG_ARROW_RIGHT, "トランジション"),
                (crate::ui::icons::SVG_PALETTE, "カラー"),
                (crate::ui::icons::SVG_EXPORT, "書き出し"),
                (crate::ui::icons::SVG_BOOK, "チュートリアル"),
                (crate::ui::icons::SVG_DOCUMENT, "ドキュメント"),
                (crate::ui::icons::SVG_SETTINGS, "設定"),
                (crate::ui::icons::SVG_POWER, "終了"),
            ];
            let start = r.top() + if width >= 1200.0 { 113.0 } else { 104.0 };
            for (index, (icon, label)) in nav.into_iter().enumerate() {
                let y = start + index as f32 * if width >= 1200.0 { 41.0 } else { 36.0 };
                let active = label == "書き出し";
                let nav_rect = egui::Rect::from_min_max(
                    egui::pos2(10.0, y - 5.0),
                    egui::pos2(sidebar.right(), y + 37.0),
                );
                if active {
                    ui.painter().rect_filled(
                        nav_rect,
                        0.0,
                        crate::ui::theme::colors::BG_PANEL_DARK,
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(egui::pos2(10.0, y - 5.0), egui::vec2(4.0, 42.0)),
                        0.0,
                        crate::ui::theme::colors::ACCENT_BRAND,
                    );
                }
                let ix = if narrow {
                    (left_width - 22.0) * 0.5
                } else {
                    34.0
                };
                crate::ui::icons::render_svg_at(
                    ui,
                    format!("export-nav-{index}"),
                    icon,
                    egui::vec2(22.0, 22.0),
                    if active {
                        crate::ui::theme::colors::ACCENT_BRAND_HOVER
                    } else {
                        crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
                    },
                    egui::pos2(ix, y + 5.0),
                );
                if !narrow {
                    ui.painter().text(
                        egui::pos2(80.0, y + 16.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(if width >= 1200.0 { 14.0 } else { 11.0 }),
                        if active {
                            colors::TEXT_PRIMARY
                        } else {
                            crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
                        },
                    );
                }
                interact_target_nav(ui, ctx, nav_rect, label, "export-target-nav");
            }
            if !narrow && !short {
                ui.painter().text(
                    egui::pos2(32.0, r.bottom() - 27.0),
                    egui::Align2::LEFT_CENTER,
                    "Kagari VFX   v0.1.0",
                    egui::FontId::proportional(13.0),
                    colors::TEXT_SECONDARY,
                );
            }
            let main =
                egui::Rect::from_min_max(egui::pos2(sidebar.right(), r.top()), r.right_bottom());
            let margin = if width >= 1200.0 { 34.0 } else { 16.0 };
            let title_x = main.left() + margin;
            ui.painter().text(
                egui::pos2(title_x, r.top() + 43.0),
                egui::Align2::LEFT_CENTER,
                "書き出し設定",
                egui::FontId::proportional(if width >= 1200.0 { 32.0 } else { 23.0 }),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(title_x, r.top() + 79.0),
                egui::Align2::LEFT_CENTER,
                "あなたの作品を、世界へ。高品質な映像を書き出しましょう。",
                egui::FontId::proportional(if width >= 1200.0 { 17.0 } else { 12.0 }),
                colors::TEXT_SECONDARY,
            );
            let preview_w = if width >= 1200.0 {
                703.0
            } else {
                ((main.width() - margin * 2.0 - 14.0) * 0.5).max(280.0)
            };
            let settings_w = if width >= 1200.0 {
                main.width() - margin * 2.0 - preview_w - 14.0
            } else {
                preview_w
            };
            let top = r.top() + 118.0;
            let preview = egui::Rect::from_min_size(
                egui::pos2(title_x, top),
                egui::vec2(preview_w, if width >= 1200.0 { 449.0 } else { 260.0 }),
            );
            let settings = egui::Rect::from_min_size(
                egui::pos2(
                    if width >= 1200.0 {
                        preview.right() + 14.0
                    } else {
                        title_x + preview_w + 14.0
                    },
                    top - 34.0,
                ),
                egui::vec2(settings_w, if width >= 1200.0 { 482.0 } else { 400.0 }),
            );
            let fill = crate::ui::theme::colors::BG_PANEL_BASE;
            let stroke = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter().rect(preview, 8.0, fill, stroke);
            ui.painter().rect(settings, 8.0, fill, stroke);
            let image_rect = egui::Rect::from_min_size(
                egui::pos2(preview.left() + 18.0, preview.top() + 15.0),
                egui::vec2(
                    preview.width() - 36.0,
                    if width >= 1200.0 { 352.0 } else { 155.0 },
                ),
            );
            if let Some(id) = reference_texture(app, ctx, "render_preview.webp") {
                ui.painter().image(
                    id,
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            ui.painter().rect_stroke(image_rect, 0.0, stroke);
            ui.painter().text(
                egui::pos2(preview.left() + 18.0, preview.bottom() - 55.0),
                egui::Align2::LEFT_CENTER,
                "00:00:04:12",
                egui::FontId::proportional(16.0),
                crate::ui::theme::colors::ACCENT_BRAND,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(preview.left() + 135.0, preview.bottom() - 56.0),
                    egui::pos2(preview.right() - 115.0, preview.bottom() - 56.0),
                ],
                egui::Stroke::new(3.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
            );
            for (i, icon) in [
                crate::ui::icons::SVG_JUMP_BACK,
                crate::ui::icons::SVG_PLAY,
                crate::ui::icons::SVG_PAUSE,
                crate::ui::icons::SVG_JUMP_FORWARD,
            ]
            .into_iter()
            .enumerate()
            {
                crate::ui::icons::render_svg_at(
                    ui,
                    format!("export-preview-transport-{i}"),
                    icon,
                    egui::vec2(16.0, 16.0),
                    colors::TEXT_PRIMARY,
                    egui::pos2(
                        preview.center().x - 83.0 + i as f32 * 44.0,
                        preview.bottom() - 35.0,
                    ),
                );
            }
            ui.painter().text(
                egui::pos2(settings.left() + 20.0, settings.top() + 28.0),
                egui::Align2::LEFT_CENTER,
                "ビデオ        オーディオ        詳細設定",
                egui::FontId::proportional(if width >= 1200.0 { 14.0 } else { 11.0 }),
                colors::TEXT_PRIMARY,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(settings.left() + 30.0, settings.top() + 42.0),
                    egui::pos2(settings.left() + 127.0, settings.top() + 42.0),
                ],
                egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
            );
            let rows = [
                ("形式", "MP4 (H.264)"),
                ("コーデック", "H.264 (AVC)"),
                ("解像度", "1920 × 1080 (フルHD)"),
                ("フレームレート", "30 fps"),
                ("ビットレート", "高品質（20 Mbps）"),
            ];
            let field_x = settings.left()
                + if width >= 1200.0 {
                    214.0
                } else {
                    settings.width() * 0.35
                };
            for (i, (lab, val)) in rows.into_iter().enumerate() {
                let yy = settings.top() + 72.0 + i as f32 * 38.0;
                ui.painter().text(
                    egui::pos2(settings.left() + 20.0, yy + 11.0),
                    egui::Align2::LEFT_CENTER,
                    lab,
                    egui::FontId::proportional(if width >= 1200.0 { 14.0 } else { 10.0 }),
                    colors::TEXT_SECONDARY,
                );
                ui.painter().rect(
                    egui::Rect::from_min_size(
                        egui::pos2(field_x, yy - 5.0),
                        egui::vec2(settings.right() - field_x - 20.0, 32.0),
                    ),
                    5.0,
                    crate::ui::theme::colors::BG_DARK,
                    stroke,
                );
                ui.painter().text(
                    egui::pos2(field_x + 12.0, yy + 11.0),
                    egui::Align2::LEFT_CENTER,
                    val,
                    egui::FontId::proportional(if width >= 1200.0 { 13.0 } else { 9.0 }),
                    colors::TEXT_PRIMARY,
                );
            }
            let checkbox = |y: f32, label: &str, checked: bool| {
                let box_rect = egui::Rect::from_min_size(
                    egui::pos2(settings.left() + 214.0, y - 9.0),
                    egui::vec2(18.0, 18.0),
                );
                ui.painter().rect(
                    box_rect,
                    3.0,
                    if checked {
                        crate::ui::theme::colors::ACCENT_BRAND
                    } else {
                        egui::Color32::TRANSPARENT
                    },
                    stroke,
                );
                if checked {
                    ui.painter().text(
                        box_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "✓",
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                }
                ui.painter().text(
                    egui::pos2(settings.left() + 244.0, y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(if width >= 1200.0 { 13.0 } else { 9.0 }),
                    colors::TEXT_SECONDARY,
                );
            };
            checkbox(
                settings.top() + 264.0,
                "アルファチャンネルを含める（透過）",
                true,
            );
            checkbox(settings.top() + 295.0, "HDR（Rec. 2100 / HLG）", false);
            ui.painter().line_segment(
                [
                    egui::pos2(settings.left() + 20.0, settings.top() + 319.0),
                    egui::pos2(settings.right() - 20.0, settings.top() + 319.0),
                ],
                stroke,
            );
            for (i, (lab, val)) in [
                ("保存先", "~/Movies/Kagari VFX"),
                ("ファイル名", "Sample Project"),
            ]
            .into_iter()
            .enumerate()
            {
                let yy = settings.top() + 338.0 + i as f32 * 38.0;
                ui.painter().text(
                    egui::pos2(settings.left() + 20.0, yy + 11.0),
                    egui::Align2::LEFT_CENTER,
                    lab,
                    egui::FontId::proportional(if width >= 1200.0 { 14.0 } else { 10.0 }),
                    colors::TEXT_SECONDARY,
                );
                ui.painter().rect(
                    egui::Rect::from_min_size(
                        egui::pos2(field_x, yy - 5.0),
                        egui::vec2(settings.right() - field_x - 20.0, 32.0),
                    ),
                    5.0,
                    crate::ui::theme::colors::BG_DARK,
                    stroke,
                );
                ui.painter().text(
                    egui::pos2(field_x + 12.0, yy + 11.0),
                    egui::Align2::LEFT_CENTER,
                    val,
                    egui::FontId::proportional(if width >= 1200.0 { 13.0 } else { 9.0 }),
                    colors::TEXT_PRIMARY,
                );
            }
            let export_action = egui::Rect::from_min_size(
                egui::pos2(settings.left() + 20.0, settings.bottom() - 60.0),
                egui::vec2(settings.width() - 40.0, 47.0),
            );
            ui.painter().rect(
                export_action,
                6.0,
                crate::ui::theme::colors::ACCENT_BRAND,
                egui::Stroke::NONE,
            );
            crate::ui::icons::render_svg_at(
                ui,
                "export-start-icon".to_string(),
                crate::ui::icons::SVG_EXPORT,
                egui::vec2(18.0, 18.0),
                egui::Color32::WHITE,
                egui::pos2(settings.center().x - 84.0, settings.bottom() - 45.0),
            );
            ui.painter().text(
                egui::pos2(settings.center().x + 10.0, settings.bottom() - 36.0),
                egui::Align2::CENTER_CENTER,
                "書き出し開始",
                egui::FontId::proportional(if width >= 1200.0 { 15.0 } else { 11.0 }),
                egui::Color32::WHITE,
            );
            if ui
                .interact(
                    export_action,
                    egui::Id::new("reference-export-start"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                let comp_id = app.history.current().active_composition().id.clone();
                crate::ui::export_dialog::start_comp_export(app, ctx, &comp_id);
            }
            let queue_y = if width >= 1200.0 {
                r.top() + 582.0
            } else {
                settings.bottom() + 14.0
            };
            let queue = egui::Rect::from_min_size(
                egui::pos2(title_x, queue_y),
                egui::vec2(
                    main.width() - margin * 2.0,
                    (height - queue_y - r.top() - 34.0).max(150.0),
                ),
            );
            draw_home_render_queue(app, ui, queue);
        });
}

pub(in crate::ui::home_screen) fn draw_home_render_queue(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    rect: egui::Rect,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let queue_items = app.render_queue_items.clone();
    let queue_status = app.render_item_status.clone();
    let compositions: std::collections::HashMap<_, _> = app
        .history
        .current()
        .compositions
        .iter()
        .map(|composition| (composition.id.clone(), composition.name.clone()))
        .collect();
    let active_comp = app.history.current().active_composition();
    let active_id = active_comp.id.clone();
    let rendering_id = app.export.export_comp_id.clone();
    let export_progress = app.export.export_progress.clamp(0.0, 1.0);
    let exporting = app.export.is_exporting;
    let mut add_active = false;
    let mut clear_queue = false;
    let mut remove_index = None;
    let mut activate_id = None;

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        egui::Frame::none()
            .fill(crate::ui::theme::colors::BG_PANEL_BASE)
            .stroke(egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
            .rounding(8.0)
            .inner_margin(egui::Margin::symmetric(16.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("レンダーキュー  {}", queue_items.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        clear_queue = ui
                            .add_enabled(!exporting && !queue_items.is_empty(), egui::Button::new("キューを消去"))
                            .clicked();
                        add_active = ui
                            .add_enabled(
                                !queue_items.contains(&active_id),
                                egui::Button::new("現在のコンポを追加"),
                            )
                            .clicked();
                    });
                });
                ui.separator();
                if queue_items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("キューは空です。書き出すコンポジションを追加してください。")
                                .color(colors::TEXT_MUTED),
                        );
                    });
                } else {
                    let row_height = 34.0;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_rows(ui, row_height, queue_items.len(), |ui, visible| {
                            for index in visible {
                                let composition_id = &queue_items[index];
                                let display_name = compositions
                                    .get(composition_id)
                                    .map(String::as_str)
                                    .unwrap_or("Missing composition");
                                let missing = !compositions.contains_key(composition_id);
                                let status = queue_status.get(composition_id).copied().unwrap_or(
                                    crate::app_state::QueueItemStatus::Queued,
                                );
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", index + 1));
                                    let label = if missing {
                                        egui::RichText::new(display_name).color(colors::TEXT_MUTED)
                                    } else {
                                        egui::RichText::new(display_name).color(colors::TEXT_PRIMARY)
                                    };
                                    let label_width = (ui.available_width() - 180.0).max(80.0);
                                    if ui
                                        .add_sized(
                                            egui::vec2(label_width, 24.0),
                                            egui::Label::new(label).truncate().selectable(true),
                                        )
                                        .clicked()
                                        && !missing
                                    {
                                        activate_id = Some(composition_id.clone());
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.add_enabled(!exporting, egui::Button::new("削除")).clicked() {
                                                remove_index = Some(index);
                                            }
                                            let (status_text, status_color) = if missing {
                                                ("コンポジションがありません", colors::ACCENT_RED)
                                            } else {
                                                match status {
                                                    crate::app_state::QueueItemStatus::Queued => ("待機中", colors::TEXT_SECONDARY),
                                                    crate::app_state::QueueItemStatus::Rendering => ("書き出し中", colors::ACCENT_CYAN),
                                                    crate::app_state::QueueItemStatus::Done => ("完了", colors::ACCENT_GREEN),
                                                    crate::app_state::QueueItemStatus::Failed => ("失敗", colors::ACCENT_RED),
                                                }
                                            };
                                            let progress = if exporting && rendering_id.as_deref() == Some(composition_id.as_str()) {
                                                format!("{:.0}%", export_progress * 100.0)
                                            } else {
                                                status_text.to_string()
                                            };
                                            ui.label(egui::RichText::new(progress).color(status_color));
                                        },
                                    );
                                });
                            }
                        });
                }
            });
    });

    if clear_queue && !app.export.is_exporting {
        app.render_queue_items.clear();
        app.render_item_status.clear();
    } else if let Some(index) = remove_index {
        if index < app.render_queue_items.len() && !app.export.is_exporting {
            let removed = app.render_queue_items.remove(index);
            app.render_item_status.remove(&removed);
        }
    } else if add_active {
        queue_active_composition(app);
    }
    if let Some(composition_id) = activate_id {
        let mut project = app.history.current().clone();
        if let Some(index) = project
            .compositions
            .iter()
            .position(|composition| composition.id == composition_id)
        {
            project.active_composition_idx = index;
            app.commit_project(project);
            app.selection.selected_layer_idx = None;
            app.selection.selected_layers.clear();
        }
    }
}

pub(in crate::ui::home_screen) fn queue_active_composition(app: &mut KagariApp) -> bool {
    let comp_id = app.history.current().active_composition().id.clone();
    if app.render_queue_items.contains(&comp_id) {
        return false;
    }
    app.render_queue_items.push(comp_id.clone());
    app.render_item_status
        .insert(comp_id, crate::app_state::QueueItemStatus::Queued);
    true
}
