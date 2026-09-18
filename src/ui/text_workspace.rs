use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let narrow = width < 1100.0;
    let left_w = if narrow { 150.0 } else { 231.0 };
    let preset_w = if narrow { 230.0 } else { 364.0 };
    egui::TopBottomPanel::top("text_workspace_window_bar")
        .exact_height(43.0)
        .resizable(false)
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 31, 37)))
        .show(ctx, |ui| {
            for (x, c) in [
                (23.0, egui::Color32::from_rgb(255, 82, 78)),
                (46.0, egui::Color32::from_rgb(255, 190, 45)),
                (69.0, egui::Color32::from_rgb(42, 211, 86)),
            ] {
                ui.painter().circle_filled(egui::pos2(x, 22.0), 7.0, c);
            }
            ui.painter().text(
                egui::pos2(95.0, 22.0),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(width - 28.0, 22.0),
                egui::Align2::RIGHT_CENTER,
                "Create. Composite. Illuminate.",
                egui::FontId::proportional(13.0),
                colors::TEXT_MUTED,
            );
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(10, 18, 24)))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let sidebar = egui::Rect::from_min_max(r.min, egui::pos2(left_w, r.bottom()));
            let preset = egui::Rect::from_min_max(
                egui::pos2(left_w, r.top() + 62.0),
                egui::pos2(left_w + preset_w, r.bottom()),
            );
            let right_w = if narrow { 250.0 } else { 384.0 };
            let center = egui::Rect::from_min_max(
                egui::pos2(preset.right(), r.top() + 62.0),
                egui::pos2(r.right() - right_w, r.bottom()),
            );
            let inspector = egui::Rect::from_min_max(center.right_top(), r.right_bottom());
            let timeline_top = r.bottom() - if narrow { 260.0 } else { 316.0 };
            let border = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter()
                .rect_filled(sidebar, 0.0, egui::Color32::from_rgb(14, 24, 31));
            ui.painter().line_segment(
                [egui::pos2(left_w, 0.0), egui::pos2(left_w, r.bottom())],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(preset.right(), 0.0),
                    egui::pos2(preset.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(center.right(), 0.0),
                    egui::pos2(center.right(), r.bottom()),
                ],
                border,
            );
            draw_logo(app, ctx, ui, sidebar, narrow);
            draw_nav(ui, sidebar, r, narrow);
            draw_presets(ui, ctx, preset);
            draw_preview(app, ui, ctx, center, timeline_top, narrow);
            draw_inspector(app, ui, inspector, narrow);
            draw_timeline(app, ui, timeline_top, center.left(), r.right(), narrow);
        });
}

fn draw_logo(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sidebar: egui::Rect,
    narrow: bool,
) {
    if app.home_banner.is_none() {
        if let Ok(img) = image::open(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/kagari_logo.webp"),
        ) {
            app.home_banner = crate::ui::home_screen::load_logo_texture(ctx, img);
        }
    }
    if let Some(t) = app.home_banner.as_ref() {
        let s = if narrow { 44.0 } else { 50.0 };
        let x = if narrow {
            (sidebar.width() - s) * 0.5
        } else {
            18.0
        };
        let y = sidebar.top() + 16.0;
        ui.allocate_new_ui(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(s, s),
            )),
            |u| {
                u.add(egui::Image::new(egui::load::SizedTexture::new(
                    t.id(),
                    egui::vec2(s, s),
                )));
            },
        );
        if !narrow {
            ui.painter().text(
                egui::pos2(68.0, sidebar.top() + 41.0),
                egui::Align2::LEFT_CENTER,
                "Kagari",
                egui::FontId::proportional(24.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(143.0, sidebar.top() + 41.0),
                egui::Align2::LEFT_CENTER,
                "VFX",
                egui::FontId::proportional(24.0),
                egui::Color32::from_rgb(161, 174, 190),
            );
        }
    }
}

fn draw_nav(ui: &mut egui::Ui, sidebar: egui::Rect, r: egui::Rect, narrow: bool) {
    let items = [
        (icons::SVG_HOME, "ホーム"),
        (icons::SVG_FOLDER, "プロジェクトを開く"),
        (icons::SVG_FILE_PLUS, "新規プロジェクト"),
        (icons::SVG_BOOK, "チュートリアル"),
        (icons::SVG_DOCUMENT, "ドキュメント"),
        (icons::SVG_EFFECTS, "エフェクト"),
        (icons::SVG_FOLDER, "素材"),
        (icons::SVG_AUDIO, "オーディオ"),
        (icons::SVG_TOOL_TEXT, "テキスト"),
        (icons::SVG_ARROW_RIGHT, "トランジション"),
        (icons::SVG_PALETTE, "カラー"),
    ];
    for (i, (icon, label)) in items.into_iter().enumerate() {
        let y = r.top()
            + if i < 3 {
                96.0 + i as f32 * 46.0
            } else {
                235.0 + i as f32 * 39.0
            };
        let active = label == "テキスト";
        if active {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(9.0, y - 4.0),
                    egui::pos2(sidebar.right(), y + 35.0),
                ),
                0.0,
                egui::Color32::from_rgb(27, 35, 42),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(9.0, y - 4.0), egui::vec2(4.0, 39.0)),
                0.0,
                egui::Color32::from_rgb(255, 111, 28),
            );
        }
        let x = if narrow {
            (sidebar.width() - 22.0) * 0.5
        } else {
            28.0
        };
        icons::render_svg_at(
            ui,
            format!("text-nav-{i}"),
            icon,
            egui::vec2(22.0, 22.0),
            if active {
                egui::Color32::from_rgb(255, 145, 50)
            } else {
                egui::Color32::from_rgb(193, 205, 218)
            },
            egui::pos2(x, y + 4.0),
        );
        if !narrow {
            ui.painter().text(
                egui::pos2(72.0, y + 15.0),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(14.0),
                if active {
                    colors::TEXT_PRIMARY
                } else {
                    egui::Color32::from_rgb(193, 205, 218)
                },
            );
        }
    }
    ui.painter().line_segment(
        [
            egui::pos2(25.0, r.top() + 247.0),
            egui::pos2(sidebar.right() - 25.0, r.top() + 247.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
}

fn texture(ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/home")
        .join(name);
    let id = egui::Id::new(("text-workspace", name));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t.id());
    }
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let t = ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let out = t.id();
    ctx.data_mut(|d| d.insert_temp(id, t));
    Some(out)
}

fn draw_presets(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    ui.painter().text(
        egui::pos2(rect.left() + 17.0, rect.top() + 28.0),
        egui::Align2::LEFT_CENTER,
        "タイトルプリセット",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    let search = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 17.0, rect.top() + 51.0),
        egui::vec2(rect.width() - 30.0, 36.0),
    );
    ui.painter().rect(
        search,
        5.0,
        egui::Color32::from_rgb(16, 29, 38),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.painter().text(
        egui::pos2(search.left() + 12.0, search.center().y),
        egui::Align2::LEFT_CENTER,
        "⌕  プリセットを検索...",
        egui::FontId::proportional(12.0),
        colors::TEXT_SECONDARY,
    );
    for (i, label) in [
        "すべて",
        "シネマティック",
        "モダン",
        "ミニマル",
        "テクノロジー",
        "和風",
        "VFX / グロー",
        "字幕（テロップ）",
        "ソーシャルメディア",
        "カスタム",
    ]
    .into_iter()
    .enumerate()
    {
        let y = rect.top() + 92.0 + i as f32 * 28.0;
        if i == 0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y - 14.0),
                    egui::vec2(rect.width() - 20.0, 34.0),
                ),
                4.0,
                egui::Color32::from_rgb(27, 35, 42),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y - 14.0),
                    egui::vec2(4.0, 34.0),
                ),
                1.0,
                egui::Color32::from_rgb(255, 111, 28),
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + 38.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            if i == 0 {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
    }
    let cards = [
        ("recent_project_eclipse.webp", "シネマティック 01"),
        ("recent_project_atlas.webp", "モダン 01"),
        ("recent_project_rift.webp", "和風 01"),
        ("recent_project_citadel.webp", "テクノロジー 01"),
        ("recent_project_atlas.webp", "ミニマル 01"),
        ("recent_project_eclipse.webp", "グロー 01"),
    ];
    let cw = (rect.width() - 47.0) * 0.5;
    for (i, (asset, label)) in cards.into_iter().enumerate() {
        let x = rect.left() + 17.0 + (i % 2) as f32 * (cw + 14.0);
        let y = rect.top() + 378.0 + (i / 2) as f32 * 106.0;
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cw, 83.0));
        ui.painter().rect(
            card,
            6.0,
            egui::Color32::from_rgb(15, 26, 33),
            egui::Stroke::new(
                1.0_f32,
                if i == 0 {
                    egui::Color32::from_rgb(255, 107, 22)
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );
        if let Some(id) = texture(ctx, asset) {
            ui.painter().image(
                id,
                egui::Rect::from_min_size(egui::pos2(x + 2.0, y + 2.0), egui::vec2(cw - 4.0, 57.0)),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        ui.painter().text(
            egui::pos2(x + 4.0, y + 76.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(10.0),
            colors::TEXT_SECONDARY,
        );
    }
}

fn draw_preview(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    timeline_top: f32,
    narrow: bool,
) {
    ui.painter().text(
        egui::pos2(rect.left() + 15.0, rect.top() + 26.0),
        egui::Align2::LEFT_CENTER,
        "プレビュー",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    let image = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 15.0, rect.top() + 50.0),
        egui::pos2(rect.right() - 14.0, timeline_top - 98.0),
    );
    if let Some(id) = texture(ctx, "continue_working_preview.webp") {
        ui.painter().image(
            id,
            image,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    ui.painter().rect_stroke(
        image,
        0.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let box_rect = egui::Rect::from_min_size(
        egui::pos2(
            image.left() + image.width() * 0.16,
            image.top() + image.height() * 0.32,
        ),
        egui::vec2(image.width() * 0.68, image.height() * 0.35),
    );
    ui.painter().rect_stroke(
        box_rect,
        0.0,
        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 107, 22)),
    );
    let preview_text = selected_text(app)
        .map(|v| v.1)
        .unwrap_or_else(|| "まだ見ぬ世界へ".to_string());
    ui.painter().text(
        box_rect.center(),
        egui::Align2::CENTER_CENTER,
        preview_text,
        egui::FontId::proportional(if narrow { 24.0 } else { 36.0 }),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(box_rect.center().x, box_rect.center().y + 35.0),
        egui::Align2::CENTER_CENTER,
        "A BRIGHTER TOMORROW",
        egui::FontId::proportional(11.0),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 15.0, timeline_top - 62.0),
        egui::Align2::LEFT_CENTER,
        "00:00:04:12",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(255, 107, 22),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 143.0, timeline_top - 62.0),
            egui::pos2(rect.right() - 85.0, timeline_top - 62.0),
        ],
        egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(255, 107, 22)),
    );
    for (i, icon) in [
        crate::ui::icons::SVG_JUMP_BACK,
        crate::ui::icons::SVG_STEP_BACK,
        crate::ui::icons::SVG_PLAY,
        crate::ui::icons::SVG_STEP_FORWARD,
    ]
    .into_iter()
    .enumerate()
    {
        crate::ui::icons::render_svg_at(
            ui,
            format!("text-preview-transport-{i}"),
            icon,
            egui::vec2(16.0, 16.0),
            colors::TEXT_PRIMARY,
            egui::pos2(rect.center().x - 84.0 + i as f32 * 42.0, timeline_top - 34.0),
        );
    }
}

fn selected_text(app: &KagariApp) -> Option<(usize, String, u32, [f32; 4])> {
    let idx = app.selection.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    if let crate::core::timeline::LayerType::Text {
        text,
        font_size,
        color,
        ..
    } = &layer.layer_type
    {
        Some((idx, text.clone(), *font_size, *color))
    } else {
        None
    }
}

fn draw_inspector(app: &mut KagariApp, ui: &mut egui::Ui, rect: egui::Rect, narrow: bool) {
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.top() + 27.0),
        egui::Align2::LEFT_CENTER,
        "テキスト       アニメーション       詳細設定",
        egui::FontId::proportional(if narrow { 10.0 } else { 14.0 }),
        colors::TEXT_PRIMARY,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 10.0, rect.top() + 44.0),
            egui::pos2(rect.left() + 96.0, rect.top() + 44.0),
        ],
        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 107, 22)),
    );
    let Some((idx, mut text, mut font_size, mut color)) = selected_text(app) else {
        return;
    };
    let mut changed = false;
    let label_x = rect.left() + 20.0;
    let field_x = rect.left() + 138.0;
    let field_w = (rect.right() - field_x - 20.0).max(60.0);
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
            egui::pos2(field_x, rect.top() + 58.0),
            egui::pos2(rect.right() - 20.0, rect.top() + 102.0),
        )),
        |u| {
            u.add(egui::TextEdit::singleline(&mut text).desired_width(field_w));
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 75.0),
        egui::Align2::LEFT_CENTER,
        "テキスト内容",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(field_x, rect.top() + 106.0),
            egui::vec2(field_w, 32.0),
        )),
        |u| {
            if u.add(
                egui::DragValue::new(&mut font_size)
                    .range(8..=512)
                    .prefix("サイズ "),
            )
            .changed()
            {
                changed = true;
            }
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 122.0),
        egui::Align2::LEFT_CENTER,
        "フォントサイズ",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            egui::pos2(field_x, rect.top() + 145.0),
            egui::vec2(field_w, 32.0),
        )),
        |u| {
            if u.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                changed = true;
            }
        },
    );
    ui.painter().text(
        egui::pos2(label_x, rect.top() + 161.0),
        egui::Align2::LEFT_CENTER,
        "塗り（フィル）",
        egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
        colors::TEXT_SECONDARY,
    );
    changed |= text != selected_text(app).map(|v| v.1).unwrap_or_default();
    if changed {
        app.modify_project(|project| {
            if let Some(layer) = project.active_composition_mut().layers.get_mut(idx) {
                if let crate::core::timeline::LayerType::Text {
                    text: dst,
                    font_size: dst_size,
                    color: dst_color,
                    ..
                } = &mut layer.layer_type
                {
                    *dst = text.clone();
                    *dst_size = font_size;
                    *dst_color = color;
                }
            }
        });
    }
}

fn draw_timeline(
    app: &KagariApp,
    ui: &mut egui::Ui,
    top: f32,
    left: f32,
    right: f32,
    narrow: bool,
) {
    ui.painter().line_segment(
        [egui::pos2(left, top), egui::pos2(right, top)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.painter().text(
        egui::pos2(left + 18.0, top + 29.0),
        egui::Align2::LEFT_CENTER,
        "タイムライン",
        egui::FontId::proportional(16.0),
        colors::TEXT_PRIMARY,
    );
    for (i, g) in ["＋", "▱", "✂", "⌕", "▣"].into_iter().enumerate() {
        ui.painter().text(
            egui::pos2(left + 160.0 + i as f32 * 43.0, top + 29.0),
            egui::Align2::CENTER_CENTER,
            g,
            egui::FontId::proportional(18.0),
            colors::TEXT_SECONDARY,
        );
    }
    let ruler = top + 57.0;
    ui.painter().line_segment(
        [egui::pos2(left, ruler), egui::pos2(right, ruler)],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    for i in 0..8 {
        ui.painter().text(
            egui::pos2(left + 145.0 + i as f32 * 80.0, ruler - 15.0),
            egui::Align2::CENTER_CENTER,
            format!("00:00:{:02}:00", i * 5),
            egui::FontId::proportional(10.0),
            colors::TEXT_SECONDARY,
        );
    }
    let selected_name = selected_text(app)
        .map(|v| v.1)
        .unwrap_or_else(|| "まだ見ぬ世界へ".to_string());
    let rows: [(&str, f32, egui::Color32, String); 4] = [
        (
            "メインタイトル",
            190.0,
            egui::Color32::from_rgb(145, 76, 166),
            selected_name,
        ),
        (
            "サブタイトル",
            160.0,
            egui::Color32::from_rgb(74, 67, 175),
            "A BRIGHTER TOMORROW".to_string(),
        ),
        (
            "背景シェイプ",
            390.0,
            egui::Color32::from_rgb(80, 76, 94),
            "光のライン".to_string(),
        ),
        (
            "映像",
            500.0,
            egui::Color32::from_rgb(35, 105, 150),
            "mountain.mp4".to_string(),
        ),
    ];
    for (i, (name, span, color, value)) in rows.into_iter().enumerate() {
        let y = ruler + 14.0 + i as f32 * 43.0;
        ui.painter().line_segment(
            [egui::pos2(left, y + 39.0), egui::pos2(right, y + 39.0)],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        ui.painter().text(
            egui::pos2(left + 55.0, y + 18.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(if narrow { 9.0 } else { 12.0 }),
            colors::TEXT_PRIMARY,
        );
        let bar = egui::Rect::from_min_size(
            egui::pos2(left + 190.0 + i as f32 * 95.0, y + 4.0),
            egui::vec2(span.min(right - left - 205.0), 30.0),
        );
        ui.painter().rect_filled(bar, 4.0, color);
        ui.painter().text(
            egui::pos2(bar.left() + 10.0, y + 19.0),
            egui::Align2::LEFT_CENTER,
            value,
            egui::FontId::proportional(10.0),
            colors::TEXT_PRIMARY,
        );
    }
    let play_x = left + 252.0;
    ui.painter().line_segment(
        [
            egui::pos2(play_x, ruler - 5.0),
            egui::pos2(play_x, top + 300.0),
        ],
        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 107, 22)),
    );
}
