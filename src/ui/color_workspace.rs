#![allow(clippy::possible_missing_else)]

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let narrow = width < 1100.0;
    let sidebar_w = if narrow { 72.0 } else { 231.0 };
    let shot_w = if narrow { 150.0 } else { 219.0 };
    let right_w = if narrow { 300.0 } else { 521.0 };
    egui::TopBottomPanel::top("color_workspace_window_bar")
        .exact_height(43.0)
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
            let sidebar = egui::Rect::from_min_max(r.min, egui::pos2(sidebar_w, r.bottom()));
            let shot = egui::Rect::from_min_max(
                egui::pos2(sidebar.right(), r.top() + 62.0),
                egui::pos2(sidebar.right() + shot_w, r.bottom() - 275.0),
            );
            let main = egui::Rect::from_min_max(
                shot.right_top(),
                egui::pos2(r.right() - right_w, r.bottom() - 275.0),
            );
            let inspector = egui::Rect::from_min_max(
                main.right_top(),
                egui::pos2(r.right(), r.bottom() - 275.0),
            );
            let lower = egui::Rect::from_min_max(
                egui::pos2(sidebar.right(), r.bottom() - 275.0),
                r.right_bottom(),
            );
            let border = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter()
                .rect_filled(sidebar, 0.0, egui::Color32::from_rgb(14, 24, 31));
            ui.painter().line_segment(
                [
                    egui::pos2(sidebar.right(), 0.0),
                    egui::pos2(sidebar.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(shot.right(), 0.0),
                    egui::pos2(shot.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(main.right(), 0.0),
                    egui::pos2(main.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(sidebar.right(), lower.top()),
                    egui::pos2(r.right(), lower.top()),
                ],
                border,
            );
            draw_logo(app, ctx, ui, sidebar, narrow);
            draw_nav(ui, sidebar, r, narrow);
            draw_shots(ui, ctx, shot);
            draw_viewer(ui, ctx, main, lower.top());
            draw_color_inspector(app, ui, inspector, narrow);
            draw_lower(ui, ctx, lower, narrow);
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
        if let Ok(img) = crate::ui::embedded_assets::open_image(
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
        (icons::SVG_PALETTE, "カラー補正"),
        (icons::SVG_EXPORT, "書き出し"),
    ];
    for (i, (icon, label)) in items.into_iter().enumerate() {
        let y = r.top() + 96.0 + i as f32 * 39.0;
        let active = label == "カラー補正";
        if active {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(9.0, y - 4.0),
                    egui::pos2(sidebar.right(), y + 35.0),
                ),
                0.0,
                crate::ui::theme::colors::BG_PANEL_DARK,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(9.0, y - 4.0), egui::vec2(4.0, 39.0)),
                0.0,
                crate::ui::theme::colors::ACCENT_BRAND,
            );
        }
        let x = if narrow {
            (sidebar.width() - 22.0) * 0.5
        } else {
            28.0
        };
        icons::render_svg_at(
            ui,
            format!("color-nav-{i}"),
            icon,
            egui::vec2(22.0, 22.0),
            if active {
                crate::ui::theme::colors::ACCENT_BRAND_HOVER
            } else {
                crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
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
                    crate::ui::theme::colors::TEXT_SECONDARY_BRIGHT
                },
            );
        }
    }
}

fn asset(ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/studio")
        .join(name);
    let id = egui::Id::new(("color-workspace", name));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t.id());
    }
    let img = crate::ui::embedded_assets::open_image(path).ok()?.to_rgba8();
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

fn draw_shots(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    let p = ui.painter().clone();
    let compact = rect.width() < 180.0;
    p.text(
        egui::pos2(rect.left() + 18.0, rect.top() + 30.0),
        egui::Align2::LEFT_CENTER,
        "ショット",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.right() - 28.0, rect.top() + 30.0),
        egui::Align2::CENTER_CENTER,
        "＋",
        egui::FontId::proportional(24.0),
        colors::TEXT_PRIMARY,
    );
    let rows = [
        ("01", "mountain.mp4", "00:00:00:00"),
        ("02", "clouds.mp4", "00:00:08:12"),
        ("03", "particles.mov", "00:00:16:03"),
        ("04", "sky_replace.mp4", "00:00:24:17"),
    ];
    for (i, (num, name, time)) in rows.into_iter().enumerate() {
        let y = rect.top() + 55.0 + i as f32 * if compact { 62.0 } else { 70.0 };
        let active = i == 0;
        if active {
            p.rect(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 10.0, y),
                    egui::vec2(rect.width() - 20.0, if compact { 60.0 } else { 68.0 }),
                ),
                6.0,
                egui::Color32::from_rgb(20, 32, 41),
                egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
            );
        }
        if let Some(id) = asset(
            ctx,
            if i == 0 {
                "assets_mountain.webp"
            } else if i == 1 {
                "assets_clouds.webp"
            } else {
                "assets_smoke.webp"
            },
        ) {
            p.image(
                id,
                egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + if compact { 12.0 } else { 18.0 },
                        y + if compact { 8.0 } else { 9.0 },
                    ),
                    egui::vec2(
                        if compact { 48.0 } else { 66.0 },
                        if compact { 38.0 } else { 48.0 },
                    ),
                ),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        p.text(
            egui::pos2(rect.left() + if compact { 68.0 } else { 93.0 }, y + 15.0),
            egui::Align2::LEFT_CENTER,
            num,
            egui::FontId::proportional(if compact { 12.0 } else { 15.0 }),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(rect.left() + if compact { 68.0 } else { 93.0 }, y + 34.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(if compact { 10.0 } else { 12.0 }),
            colors::TEXT_SECONDARY,
        );
        p.text(
            egui::pos2(rect.left() + if compact { 68.0 } else { 93.0 }, y + 51.0),
            egui::Align2::LEFT_CENTER,
            time,
            egui::FontId::proportional(10.0),
            colors::TEXT_MUTED,
        );
    }
}

fn draw_viewer(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect, lower_top: f32) {
    let p = ui.painter().clone();
    let compact = rect.height() < 500.0;
    p.text(
        egui::pos2(rect.left() + 15.0, rect.top() + 29.0),
        egui::Align2::LEFT_CENTER,
        "ビューアー",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    let image_bottom = if compact {
        lower_top - 170.0
    } else {
        rect.top() + 328.0
    };
    let image = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 13.0, rect.top() + 42.0),
        egui::pos2(rect.right() - 14.0, image_bottom.max(rect.top() + 100.0)),
    );
    if let Some(id) = asset(ctx, "assets_mountain.webp") {
        p.image(
            id,
            image,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    p.rect_stroke(
        image,
        0.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(
            rect.left() + 18.0,
            if compact {
                image.bottom() + 25.0
            } else {
                rect.top() + 355.0
            },
        ),
        egui::Align2::LEFT_CENTER,
        "00:00:12:17",
        egui::FontId::proportional(16.0),
        crate::ui::theme::colors::ACCENT_BRAND,
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
            format!("color-preview-transport-{i}"),
            icon,
            egui::vec2(16.0, 16.0),
            colors::TEXT_PRIMARY,
            egui::pos2(
                rect.center().x - 76.0 + i as f32 * 43.0,
                if compact {
                    image.bottom() + 17.0
                } else {
                    rect.top() + 347.0
                },
            ),
        );
    }
    let ty = if compact {
        lower_top - 125.0
    } else {
        lower_top - 171.0
    };
    p.text(
        egui::pos2(rect.left() + 18.0, ty),
        egui::Align2::LEFT_CENTER,
        "タイムライン",
        egui::FontId::proportional(16.0),
        colors::TEXT_PRIMARY,
    );
    p.line_segment(
        [
            egui::pos2(rect.left() + 14.0, ty + 37.0),
            egui::pos2(rect.right() - 14.0, ty + 37.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(rect.left() + 60.0, ty + if compact { 38.0 } else { 63.0 }),
        egui::Align2::CENTER_CENTER,
        "00:00        00:05        00:10        00:15        00:20        00:25        00:30",
        egui::FontId::proportional(10.0),
        colors::TEXT_SECONDARY,
    );
    p.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 59.0, ty + if compact { 50.0 } else { 78.0 }),
            egui::vec2(rect.width() - 75.0, if compact { 24.0 } else { 42.0 }),
        ),
        4.0,
        egui::Color32::from_rgb(23, 39, 50),
    );
    p.text(
        egui::pos2(rect.left() + 20.0, ty + if compact { 64.0 } else { 98.0 }),
        egui::Align2::LEFT_CENTER,
        "V1",
        egui::FontId::proportional(13.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.left() + 20.0, ty + if compact { 90.0 } else { 145.0 }),
        egui::Align2::LEFT_CENTER,
        "A1",
        egui::FontId::proportional(13.0),
        colors::TEXT_PRIMARY,
    );
    p.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 59.0, ty + if compact { 53.0 } else { 84.0 }),
            egui::vec2(rect.width() - 90.0, if compact { 17.0 } else { 30.0 }),
        ),
        3.0,
        egui::Color32::from_rgb(63, 103, 137),
    );
    p.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 59.0, ty + if compact { 79.0 } else { 128.0 }),
            egui::vec2(rect.width() - 80.0, if compact { 17.0 } else { 25.0 }),
        ),
        3.0,
        egui::Color32::from_rgb(89, 111, 123),
    );
    p.line_segment(
        [
            egui::pos2(
                rect.left() + rect.width() * 0.42,
                ty + if compact { 40.0 } else { 70.0 },
            ),
            egui::pos2(
                rect.left() + rect.width() * 0.42,
                ty + if compact { 103.0 } else { 165.0 },
            ),
        ],
        egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
}

fn draw_color_inspector(app: &mut KagariApp, ui: &mut egui::Ui, rect: egui::Rect, narrow: bool) {
    let p = ui.painter().clone();
    if narrow {
        for (i, label) in ["カラー", "LUT", "ショット", "情報"]
            .into_iter()
            .enumerate()
        {
            let tab_w = (rect.width() - 20.0) / 4.0;
            p.text(
                egui::pos2(
                    rect.left() + 10.0 + tab_w * (i as f32 + 0.5),
                    rect.top() + 28.0,
                ),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                colors::TEXT_PRIMARY,
            );
        }
    } else {
        p.text(
            egui::pos2(rect.left() + 18.0, rect.top() + 28.0),
            egui::Align2::LEFT_CENTER,
            "カラー        LUT        ショットマッチ        情報",
            egui::FontId::proportional(14.0),
            colors::TEXT_PRIMARY,
        );
    }
    p.line_segment(
        [
            egui::pos2(rect.left() + 10.0, rect.top() + 45.0),
            egui::pos2(rect.left() + 105.0, rect.top() + 45.0),
        ],
        egui::Stroke::new(2.0_f32, crate::ui::theme::colors::ACCENT_BRAND),
    );
    p.text(
        egui::pos2(rect.left() + 18.0, rect.top() + 78.0),
        egui::Align2::LEFT_CENTER,
        "スコープ",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    let boxes = if narrow {
        [("波形", 0), ("ベクトル", 1), ("ヒスト", 2)]
    } else {
        [
            ("波形モニター", 0),
            ("ベクトルスコープ", 1),
            ("ヒストグラム", 2),
        ]
    };
    let bw = (rect.width() - 50.0) / 3.0;
    for (i, (label, _)) in boxes.into_iter().enumerate() {
        let x = rect.left() + 12.0 + i as f32 * (bw + 8.0);
        let b = egui::Rect::from_min_size(egui::pos2(x, rect.top() + 98.0), egui::vec2(bw, 178.0));
        p.rect(
            b,
            6.0,
            egui::Color32::from_rgb(11, 18, 23),
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(x + 8.0, b.top() + 16.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
        for j in 0..4 {
            p.line_segment(
                [
                    egui::pos2(x + 8.0, b.top() + 42.0 + j as f32 * 31.0),
                    egui::pos2(b.right() - 8.0, b.top() + 42.0 + j as f32 * 31.0),
                ],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 49, 55)),
            );
        }
        if i == 0 {
            for j in 0..26 {
                let x1 = x + 12.0 + j as f32 * (bw - 24.0) / 25.0;
                let h = (j as f32 * 1.7).sin().abs() * 75.0 + 16.0;
                p.line_segment(
                    [
                        egui::pos2(x1, b.bottom() - 14.0),
                        egui::pos2(x1, b.bottom() - 14.0 - h),
                    ],
                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(92, 151, 194)),
                );
            }
        } else if i == 1 {
            p.circle_stroke(
                b.center() + egui::vec2(0.0, 14.0),
                46.0,
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(161, 108, 65)),
            );
            p.line_segment(
                [
                    b.center() + egui::vec2(-30.0, 25.0),
                    b.center() + egui::vec2(30.0, -18.0),
                ],
                egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
            );
        } else {
            for j in 0..5 {
                let xx = x + 16.0 + j as f32 * 12.0;
                p.line_segment(
                    [
                        egui::pos2(xx, b.bottom() - 15.0),
                        egui::pos2(xx, b.top() + 70.0 + (j % 3) as f32 * 17.0),
                    ],
                    egui::Stroke::new(
                        5.0_f32,
                        [
                            egui::Color32::from_rgb(243, 79, 63),
                            egui::Color32::from_rgb(48, 117, 225),
                            egui::Color32::from_rgb(46, 194, 102),
                            egui::Color32::from_rgb(243, 190, 51),
                            egui::Color32::from_rgb(90, 100, 220),
                        ][j],
                    ),
                );
            }
        }
    }
    if narrow {
        let add_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 18.0, rect.bottom() - 40.0),
            egui::vec2(rect.width() - 36.0, 30.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(add_rect), |ui| {
            ui.add_enabled(true, egui::Button::new("カラー補正を追加"));
        });
        return;
    }
    p.text(
        egui::pos2(rect.left() + 18.0, rect.top() + 319.0),
        egui::Align2::LEFT_CENTER,
        "基本補正",
        egui::FontId::proportional(18.0),
        colors::TEXT_PRIMARY,
    );
    for (i, (label, val)) in [
        ("露出", "0.00"),
        ("コントラスト", "12.0"),
        ("ハイライト", "-25.0"),
        ("シャドウ", "18.0"),
        ("白レベル", "5.0"),
        ("黒レベル", "-8.0"),
        ("彩度", "1.10"),
        ("自然な彩度", "0.20"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = rect.top() + 357.0 + i as f32 * 29.0;
        p.text(
            egui::pos2(rect.left() + 18.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            colors::TEXT_SECONDARY,
        );
        p.line_segment(
            [
                egui::pos2(rect.left() + 155.0, y),
                egui::pos2(rect.right() - 92.0, y),
            ],
            egui::Stroke::new(4.0_f32, egui::Color32::from_rgb(53, 68, 78)),
        );
        p.circle_filled(
            egui::pos2(rect.left() + 155.0 + (i as f32 * 19.0) % 150.0, y),
            5.0,
            if i == 1 {
                crate::ui::theme::colors::ACCENT_BRAND
            } else {
                egui::Color32::from_rgb(210, 220, 230)
            },
        );
        p.rect(
            egui::Rect::from_min_size(
                egui::pos2(rect.right() - 80.0, y - 13.0),
                egui::vec2(65.0, 26.0),
            ),
            4.0,
            egui::Color32::from_rgb(20, 32, 41),
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(rect.right() - 47.0, y),
            egui::Align2::CENTER_CENTER,
            val,
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
    }

    let Some(layer_idx) = app.selection.selected_layer_idx else {
        return;
    };
    let frame = app.playback.current_frame;
    let existing = app
        .history
        .current()
        .active_composition()
        .layers
        .get(layer_idx)
        .and_then(|layer| {
            layer.effects.iter().find_map(|effect| {
                if let crate::core::timeline::EffectType::HueSaturation {
                    hue_shift,
                    saturation,
                    lightness,
                } = &effect.effect_type
                {
                    Some((
                        hue_shift.value_at(frame),
                        saturation.value_at(frame),
                        lightness.value_at(frame),
                    ))
                } else {
                    None
                }
            })
        });
    let mut hue = existing.as_ref().map(|v| v.0).unwrap_or(0.0);
    let mut saturation = existing.as_ref().map(|v| v.1).unwrap_or(0.0);
    let mut lightness = existing.as_ref().map(|v| v.2).unwrap_or(0.0);
    let mut changed = false;
    let controls = [
        ("Hue", &mut hue, -180.0..=180.0),
        ("Saturation", &mut saturation, -100.0..=100.0),
        ("Lightness", &mut lightness, -100.0..=100.0),
    ];
    for (i, (_label, value, range)) in controls.into_iter().enumerate() {
        let y = rect.top() + 357.0 + i as f32 * 29.0;
        let control_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 150.0, y - 12.0),
            egui::vec2((rect.width() - 245.0).max(70.0), 24.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(control_rect), |ui| {
            if ui
                .add(egui::Slider::new(value, range).show_value(false))
                .changed()
            {
                changed = true;
            }
        });
        p.text(
            egui::pos2(rect.right() - 47.0, y),
            egui::Align2::CENTER_CENTER,
            format!("{value:.1}"),
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
    }
    let add_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 18.0, rect.bottom() - 40.0),
        egui::vec2(rect.width() - 36.0, 30.0),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(add_rect), |ui| {
        if ui
            .add_enabled(
                existing.is_none(),
                egui::Button::new(if existing.is_none() {
                    "カラー補正を追加"
                } else {
                    "Hue / Saturation 適用中"
                }),
            )
            .clicked()
        {
            changed = true;
        }
    });
    if changed {
        app.modify_project(|project| {
            if let Some(layer) = project.active_composition_mut().layers.get_mut(layer_idx) {
                let effect = layer.effects.iter_mut().find(|effect| {
                    matches!(
                        effect.effect_type,
                        crate::core::timeline::EffectType::HueSaturation { .. }
                    )
                });
                if let Some(effect) = effect {
                    if let crate::core::timeline::EffectType::HueSaturation {
                        hue_shift,
                        saturation: sat,
                        lightness: light,
                    } = &mut effect.effect_type
                    {
                        *hue_shift = crate::core::property::Animatable::new_constant(hue);
                        *sat = crate::core::property::Animatable::new_constant(saturation);
                        *light = crate::core::property::Animatable::new_constant(lightness);
                    }
                } else {
                    layer.effects.push(crate::core::timeline::Effect {
                        id: format!("hue_saturation_{}", layer.effects.len()),
                        name: "Hue/Saturation".to_string(),
                        effect_type: crate::core::timeline::EffectType::HueSaturation {
                            hue_shift: crate::core::property::Animatable::new_constant(hue),
                            saturation: crate::core::property::Animatable::new_constant(saturation),
                            lightness: crate::core::property::Animatable::new_constant(lightness),
                        },
                        enabled: true,
                    });
                }
            }
        });
    }
}

fn draw_lower(ui: &mut egui::Ui, _ctx: &egui::Context, rect: egui::Rect, narrow: bool) {
    let p = ui.painter();
    let columns = if narrow { 2 } else { 4 };
    let gap = 10.0;
    let card_w = (rect.width() - gap * (columns as f32 + 1.0)) / columns as f32;
    let card_h = if narrow {
        (rect.height() - gap * 3.0) / 2.0
    } else {
        rect.height() - 20.0
    };
    let titles = ["カラーホイール", "カーブ", "LUT", "ショットマッチ"];
    for (i, title) in titles.into_iter().enumerate() {
        let col = i % columns;
        let row = if narrow { i / columns } else { 0 };
        let x = rect.left() + gap + col as f32 * (card_w + gap);
        let y = rect.top() + gap + row as f32 * (card_h + gap);
        let b = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));
        p.rect(
            b,
            7.0,
            crate::ui::theme::colors::BG_PANEL_BASE,
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
        p.text(
            egui::pos2(b.left() + 14.0, b.top() + 27.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(17.0),
            colors::TEXT_PRIMARY,
        );
        if i == 0 {
            let radius = if narrow { 28.0 } else { 50.0 };
            for j in 0..3 {
                let c = egui::pos2(
                    b.left()
                        + if narrow { 55.0 } else { 72.0 }
                        + j as f32 * (b.width() - if narrow { 110.0 } else { 120.0 }) / 2.0,
                    b.top() + if narrow { 76.0 } else { 126.0 },
                );
                p.circle_stroke(
                    c,
                    radius,
                    egui::Stroke::new(
                        if narrow { 3.0_f32 } else { 5.0_f32 },
                        egui::Color32::from_rgb(37, 191, 179),
                    ),
                );
                p.circle_filled(
                    c,
                    if narrow { 4.0 } else { 5.0 },
                    egui::Color32::from_rgb(210, 220, 230),
                );
                p.text(
                    egui::pos2(c.x, b.top() + if narrow { 31.0 } else { 57.0 }),
                    egui::Align2::CENTER_CENTER,
                    ["リフト", "ガンマ", "ゲイン"][j],
                    egui::FontId::proportional(if narrow { 10.0 } else { 12.0 }),
                    colors::TEXT_PRIMARY,
                );
            }
        } else if i == 1 {
            p.line_segment(
                [
                    egui::pos2(
                        b.left() + 25.0,
                        b.bottom() - if narrow { 20.0 } else { 35.0 },
                    ),
                    egui::pos2(b.right() - 25.0, b.top() + if narrow { 38.0 } else { 55.0 }),
                ],
                egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
            );
        } else if i == 2 {
            for j in 0..3 {
                for k in 0..3 {
                    let x2 = b.left()
                        + if narrow { 25.0 } else { 45.0 }
                        + j as f32 * if narrow { 55.0 } else { 70.0 };
                    let y2 = b.top()
                        + if narrow { 40.0 } else { 64.0 }
                        + k as f32 * if narrow { 28.0 } else { 56.0 };
                    p.rect(
                        egui::Rect::from_min_size(
                            egui::pos2(x2, y2),
                            egui::vec2(
                                if narrow { 48.0 } else { 62.0 },
                                if narrow { 30.0 } else { 42.0 },
                            ),
                        ),
                        4.0,
                        egui::Color32::from_rgb(24, 46, 60),
                        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                    );
                }
            }
        } else {
            p.text(
                b.center(),
                egui::Align2::CENTER_CENTER,
                "参照ショットを選択",
                egui::FontId::proportional(13.0),
                colors::TEXT_SECONDARY,
            );
        }
    }
}
