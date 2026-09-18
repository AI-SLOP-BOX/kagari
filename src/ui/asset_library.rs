#![allow(clippy::possible_missing_else)]

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let width = ctx.screen_rect().width();
    let narrow = width < 1100.0;
    let sidebar_w = if narrow { 150.0 } else { 246.0 };
    let folder_w = if narrow { 180.0 } else { 252.0 };
    let detail_w = if narrow { 290.0 } else { 394.0 };
    egui::TopBottomPanel::top("asset_library_window_bar")
        .exact_height(40.0)
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 31, 37)))
        .show(ctx, |ui| {
            for (x, c) in [
                (23.0, egui::Color32::from_rgb(255, 82, 78)),
                (46.0, egui::Color32::from_rgb(255, 190, 45)),
                (69.0, egui::Color32::from_rgb(42, 211, 86)),
            ] {
                ui.painter().circle_filled(egui::pos2(x, 20.0), 6.5, c);
            }
            ui.painter().text(
                egui::pos2(82.0, 20.0),
                egui::Align2::LEFT_CENTER,
                "Kagari VFX",
                egui::FontId::proportional(14.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(width - 28.0, 20.0),
                egui::Align2::RIGHT_CENTER,
                "Create. Composite. Illuminate.",
                egui::FontId::proportional(12.0),
                colors::TEXT_MUTED,
            );
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(10, 18, 24)))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let left = egui::Rect::from_min_max(r.min, egui::pos2(sidebar_w, r.bottom()));
            let folder = egui::Rect::from_min_max(
                egui::pos2(left.right(), r.top() + 52.0),
                egui::pos2(left.right() + folder_w, r.bottom() - 18.0),
            );
            let grid = egui::Rect::from_min_max(
                folder.right_top(),
                egui::pos2(r.right() - detail_w, r.bottom() - 18.0),
            );
            let detail = egui::Rect::from_min_max(
                egui::pos2(grid.right(), r.top() + 176.0),
                egui::pos2(r.right(), r.bottom() - 18.0),
            );
            let border = egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
            ui.painter()
                .rect_filled(left, 0.0, egui::Color32::from_rgb(14, 24, 31));
            ui.painter().line_segment(
                [
                    egui::pos2(left.right(), 0.0),
                    egui::pos2(left.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(folder.right(), 0.0),
                    egui::pos2(folder.right(), r.bottom()),
                ],
                border,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(grid.right(), 0.0),
                    egui::pos2(grid.right(), r.bottom()),
                ],
                border,
            );
            draw_logo(app, ctx, ui, left, narrow);
            draw_nav(ui, left, r, narrow);
        draw_header(app, ui, r, folder.right(), grid.right(), narrow);
            draw_folders(ui, folder);
            draw_grid(ui, ctx, grid);
            draw_detail(ui, ctx, detail, narrow);
        });
}

fn draw_logo(
    app: &mut KagariApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    left: egui::Rect,
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
        let s = if narrow { 44.0 } else { 58.0 };
        let x = if narrow {
            (left.width() - s) * 0.5
        } else {
            27.0
        };
        ui.allocate_new_ui(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                egui::pos2(x, left.top() + 14.0),
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
                egui::pos2(97.0, left.top() + 43.0),
                egui::Align2::LEFT_CENTER,
                "Kagari",
                egui::FontId::proportional(25.0),
                colors::TEXT_PRIMARY,
            );
            ui.painter().text(
                egui::pos2(177.0, left.top() + 43.0),
                egui::Align2::LEFT_CENTER,
                "VFX",
                egui::FontId::proportional(25.0),
                egui::Color32::from_rgb(161, 174, 190),
            );
        }
    }
}

fn draw_nav(ui: &mut egui::Ui, left: egui::Rect, r: egui::Rect, narrow: bool) {
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
        let y = r.top() + 100.0 + i as f32 * 41.0;
        let active = label == "素材";
        if active {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(9.0, y - 4.0),
                    egui::pos2(left.right(), y + 36.0),
                ),
                0.0,
                egui::Color32::from_rgb(27, 35, 42),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(9.0, y - 4.0), egui::vec2(4.0, 40.0)),
                0.0,
                egui::Color32::from_rgb(255, 111, 28),
            );
        }
        let x = if narrow {
            (left.width() - 22.0) * 0.5
        } else {
            29.0
        };
        let center_y = y + 15.0;
        icons::render_svg_at(
            ui,
            format!("asset-nav-{i}"),
            icon,
            egui::vec2(22.0, 22.0),
            if active {
                egui::Color32::from_rgb(255, 145, 50)
            } else {
                egui::Color32::from_rgb(193, 205, 218)
            },
            egui::pos2(x, center_y - 11.0),
        );
        if !narrow {
            let label_rect = egui::Rect::from_min_max(
                egui::pos2(73.0, y),
                egui::pos2(left.right() - 12.0, y + 30.0),
            );
            ui.put(
                label_rect,
                egui::Label::new(
                    egui::RichText::new(label).size(14.0).color(if active {
                        colors::TEXT_PRIMARY
                    } else {
                        egui::Color32::from_rgb(193, 205, 218)
                    }),
                )
                .truncate(),
            );
        }
    }
}

fn texture(ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(if name.starts_with("library/") {
            "assets"
        } else if name.contains("template") {
            "assets/templates"
        } else if name.contains("recent") || name.contains("eclipse") {
            "assets/home"
        } else {
            "assets/studio"
        })
        .join(name);
    let id = egui::Id::new(("asset-library", name));
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

fn draw_header(app: &mut KagariApp, ui: &mut egui::Ui, r: egui::Rect, base: f32, right: f32, narrow: bool) {
    let p = ui.painter().clone();
    p.line_segment(
        [
            egui::pos2(base + 14.0, r.top() + 52.0),
            egui::pos2(r.right() - 20.0, r.top() + 52.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(275.0, r.top() + 27.0),
        egui::Align2::LEFT_CENTER,
        "▰   素材ライブラリ    ›    すべての素材",
        egui::FontId::proportional(if narrow { 12.0 } else { 14.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(270.0, r.top() + 88.0),
        egui::Align2::LEFT_CENTER,
        "素材ライブラリ",
        egui::FontId::proportional(if narrow { 24.0 } else { 28.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(474.0, r.top() + 88.0),
        egui::Align2::LEFT_CENTER,
        "映像、画像、オーディオ、3Dモデルなどの素材を管理し、創造の可能性を広げましょう。",
        egui::FontId::proportional(13.0),
        colors::TEXT_SECONDARY,
    );
    p.rect(
        egui::Rect::from_min_size(
            egui::pos2(r.right() - 350.0, r.top() + 63.0),
            egui::vec2(145.0, 42.0),
        ),
        7.0,
        egui::Color32::from_rgb(18, 29, 37),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(r.right() - 278.0, r.top() + 84.0),
        egui::Align2::CENTER_CENTER,
        "▱  フォルダ作成",
        egui::FontId::proportional(12.0),
        colors::TEXT_PRIMARY,
    );
    p.rect(
        egui::Rect::from_min_size(
            egui::pos2(r.right() - 187.0, r.top() + 63.0),
            egui::vec2(167.0, 42.0),
        ),
        7.0,
        egui::Color32::from_rgb(255, 103, 24),
        egui::Stroke::NONE,
    );
    p.text(
        egui::pos2(r.right() - 103.0, r.top() + 84.0),
        egui::Align2::CENTER_CENTER,
        "⇩  素材をインポート",
        egui::FontId::proportional(13.0),
        egui::Color32::WHITE,
    );
    let tabs = ["全て", "映像", "画像", "オーディオ", "3D", "LUT", "その他"];
    let tw = (right - base - 25.0) / 7.0;
    for (i, t) in tabs.into_iter().enumerate() {
        let x = base + i as f32 * tw;
        if i == 0 {
            p.rect(
                egui::Rect::from_min_size(
                    egui::pos2(x, r.top() + 122.0),
                    egui::vec2(tw - 1.0, 41.0),
                ),
                6.0,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 107, 22)),
            );
        }
        p.text(
            egui::pos2(x + tw * 0.5, r.top() + 143.0),
            egui::Align2::CENTER_CENTER,
            t,
            egui::FontId::proportional(12.0),
            if i == 0 {
                egui::Color32::from_rgb(255, 145, 50)
            } else {
                colors::TEXT_PRIMARY
            },
        );
    }
    p.rect(
        egui::Rect::from_min_size(
            egui::pos2(right - 348.0, r.top() + 122.0),
            egui::vec2(210.0, 41.0),
        ),
        6.0,
        egui::Color32::from_rgb(16, 29, 38),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(right - 330.0, r.top() + 143.0),
        egui::Align2::LEFT_CENTER,
        "⌕  素材を検索...",
        egui::FontId::proportional(12.0),
        colors::TEXT_SECONDARY,
    );
    p.rect(
        egui::Rect::from_min_size(
            egui::pos2(right - 127.0, r.top() + 122.0),
            egui::vec2(112.0, 41.0),
        ),
        6.0,
        egui::Color32::from_rgb(16, 29, 38),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(right - 71.0, r.top() + 143.0),
        egui::Align2::CENTER_CENTER,
        "▽  フィルター",
        egui::FontId::proportional(12.0),
        colors::TEXT_PRIMARY,
    );
    let import_rect = egui::Rect::from_min_size(
        egui::pos2(r.right() - 187.0, r.top() + 63.0),
        egui::vec2(167.0, 42.0),
    );
    if ui
        .interact(import_rect, egui::Id::new("asset-library-import"), egui::Sense::click())
        .clicked()
    {
        crate::ui::home_screen::import_footage_dialog(app);
    }
}

fn draw_folders(ui: &mut egui::Ui, rect: egui::Rect) {
    let p = ui.painter();
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 25.0),
        egui::Align2::LEFT_CENTER,
        "フォルダー",
        egui::FontId::proportional(15.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.right() - 20.0, rect.top() + 25.0),
        egui::Align2::CENTER_CENTER,
        "＋",
        egui::FontId::proportional(23.0),
        colors::TEXT_PRIMARY,
    );
    let rows = [
        ("▱  すべての素材", "1,245"),
        ("★  お気に入り", ""),
        ("◷  最近追加した項目", ""),
        ("⌄  ▱  ローカル素材", ""),
        ("    ▱  Footage", "421"),
        ("    ▱  Elements", "156"),
        ("    ▱  Environments", "87"),
        ("    ▱  Textures", "203"),
        ("    ▱  3D Models", "64"),
        ("    ▱  Audio", "98"),
        ("    ▱  LUTs", "42"),
        ("›  ▱  クラウド素材", ""),
        ("⌄  ♧  スマートコレクション", ""),
        ("    ☆  星付き", "12"),
        ("    ▱  使用中", "38"),
        ("    ▱  未使用", "892"),
    ];
    for (i, (name, count)) in rows.into_iter().enumerate() {
        let y = rect.top() + 62.0 + i as f32 * 30.0;
        if i == 0 {
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 6.0, y - 15.0),
                    egui::vec2(rect.width() - 15.0, 31.0),
                ),
                3.0,
                egui::Color32::from_rgb(27, 35, 42),
            );
            p.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 6.0, y - 15.0),
                    egui::vec2(3.0, 31.0),
                ),
                0.0,
                egui::Color32::from_rgb(255, 107, 22),
            );
        }
        p.text(
            egui::pos2(rect.left() + 17.0, y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(12.0),
            if i == 0 {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
        if !count.is_empty() {
            p.rect(
                egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 45.0, y - 11.0),
                    egui::vec2(32.0, 22.0),
                ),
                4.0,
                egui::Color32::from_rgb(25, 38, 47),
                egui::Stroke::NONE,
            );
            p.text(
                egui::pos2(rect.right() - 29.0, y),
                egui::Align2::CENTER_CENTER,
                count,
                egui::FontId::proportional(10.0),
                colors::TEXT_SECONDARY,
            );
        }
    }
}

fn draw_grid(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    let p = ui.painter();
    let names = [
        (
            "library/mountain_sunset.webp",
            "mountain_sunset.webp",
            "WEBP",
            "WEBP",
        ),
        (
            "library/av1_forest_poster.webp",
            "av1_forest.av1",
            "00:02",
            "AV1",
        ),
        ("assets_forest.webp", "forest_fog.mp4", "00:20", "MP4"),
        ("assets_city.webp", "city_night.mov", "00:15", "ProRes"),
        (
            "assets_light_leak.webp",
            "fire_element.mov",
            "00:10",
            "ProRes",
        ),
        ("assets_particles.webp", "embers_01.mov", "00:08", "ProRes"),
        ("assets_clouds.webp", "clouds_storm.mp4", "00:30", "MP4"),
        ("assets_floor_ref.webp", "ruins_01.exr", "00:42", "EXR"),
        ("assets_water.webp", "water_splash.mov", "00:14", "ProRes"),
        ("assets_moon.webp", "moon_4k.exr", "00:36", "EXR"),
        ("assets_light_leak.webp", "sparks_02.mov", "00:06", "ProRes"),
        ("assets_forest.webp", "forest_trees.mov", "00:18", "ProRes"),
    ];
    let cols = if rect.width() < 650.0 { 2 } else { 4 };
    let gap = 14.0;
    let selected = ctx.data_mut(|d| {
        d.get_temp::<usize>(egui::Id::new("asset-library-selected"))
            .unwrap_or(0)
    });
    let cw = (rect.width() - 42.0 - gap * 3.0) / cols as f32;
    for (i, (asset, name, time, kind)) in names.into_iter().enumerate() {
        let x = rect.left() + 14.0 + (i % cols) as f32 * (cw + gap);
        let y = rect.top() + 148.0 + (i / cols) as f32 * 158.0;
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cw, 144.0));
        p.rect(
            card,
            7.0,
            egui::Color32::from_rgb(15, 26, 33),
            egui::Stroke::new(
                if i == selected { 2.0_f32 } else { 1.0_f32 },
                if i == selected {
                    egui::Color32::from_rgb(255, 107, 22)
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );
        let path = if asset == "assets_forest.webp" {
            "assets_mountain.webp"
        } else if asset == "assets_water.webp" {
            "assets_particles.webp"
        } else if asset == "assets_moon.webp" {
            "assets_mountain.webp"
        } else {
            asset
        };
        let response = ui.interact(card, egui::Id::new(("asset-card", i)), egui::Sense::click());
        if response.clicked() {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("asset-library-selected"), i));
        }
        if let Some(id) = texture(ctx, path) {
            p.image(
                id,
                egui::Rect::from_min_size(egui::pos2(x + 2.0, y + 2.0), egui::vec2(cw - 4.0, 92.0)),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        p.text(
            egui::pos2(x + 8.0, y + 106.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(x + 8.0, y + 128.0),
            egui::Align2::LEFT_CENTER,
            "3840 × 2160",
            egui::FontId::proportional(10.0),
            colors::TEXT_SECONDARY,
        );
        p.text(
            egui::pos2(card.right() - 8.0, y + 106.0),
            egui::Align2::RIGHT_CENTER,
            time,
            egui::FontId::proportional(10.0),
            colors::TEXT_PRIMARY,
        );
        p.text(
            egui::pos2(card.right() - 8.0, y + 128.0),
            egui::Align2::RIGHT_CENTER,
            kind,
            egui::FontId::proportional(10.0),
            colors::TEXT_SECONDARY,
        );
    }
    p.text(
        egui::pos2(rect.left() + 16.0, rect.bottom() - 202.0),
        egui::Align2::LEFT_CENTER,
        "この素材の使用先 (3)",
        egui::FontId::proportional(16.0),
        colors::TEXT_PRIMARY,
    );
    p.rect(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 14.0, rect.bottom() - 184.0),
            egui::pos2(rect.right() - 14.0, rect.bottom() - 18.0),
        ),
        6.0,
        egui::Color32::from_rgb(15, 26, 33),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    for (i, n) in ["Sample Project", "City Overview", "EPIC Trailer"]
        .into_iter()
        .enumerate()
    {
        let y = rect.bottom() - 151.0 + i as f32 * 42.0;
        p.text(
            egui::pos2(rect.left() + 28.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{}   {}", i + 1, n),
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
        p.line_segment(
            [
                egui::pos2(rect.left() + 20.0, y + 19.0),
                egui::pos2(rect.right() - 20.0, y + 19.0),
            ],
            egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
        );
    }
}

fn draw_detail(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect, narrow: bool) {
    let p = ui.painter();
    let selected = ctx.data_mut(|d| {
        d.get_temp::<usize>(egui::Id::new("asset-library-selected"))
            .unwrap_or(0)
    });
    let selected_name = if selected == 1 {
        "av1_forest.av1"
    } else {
        "mountain_sunset.webp"
    };
    p.text(
        egui::pos2(rect.left() + 16.0, rect.top() + 26.0),
        egui::Align2::LEFT_CENTER,
        "素材の詳細",
        egui::FontId::proportional(15.0),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(rect.right() - 65.0, rect.top() + 26.0),
        egui::Align2::CENTER_CENTER,
        "★  ···",
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgb(255, 145, 50),
    );
    let preview = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 14.0, rect.top() + 42.0),
        egui::vec2(rect.width() - 28.0, 155.0),
    );
    if let Some(id) = texture(
        ctx,
        if selected == 1 {
            "library/av1_forest_poster.webp"
        } else {
            "library/mountain_sunset.webp"
        },
    ) {
        p.image(
            id,
            preview,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    p.rect_stroke(
        preview,
        6.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 227.0),
        egui::Align2::LEFT_CENTER,
        format!("{}   ✎", selected_name),
        egui::FontId::proportional(if narrow { 12.0 } else { 14.0 }),
        colors::TEXT_PRIMARY,
    );
    for (i, (a, b)) in [
        ("種類", "映像ファイル"),
        ("フォーマット", "MP4 (H.264)"),
        ("解像度", "3840 × 2160 (4K)"),
        ("フレームレート", "29.97 fps"),
        ("再生時間", "00:28"),
        ("ファイルサイズ", "412 MB"),
        ("追加日", "2025/08/28 14:32"),
        ("保存先", "/Users/username/Library/Kagari VFX/..."),
    ]
    .into_iter()
    .enumerate()
    {
        let y = rect.top() + 260.0 + i as f32 * 24.0;
        p.text(
            egui::pos2(rect.left() + 14.0, y),
            egui::Align2::LEFT_CENTER,
            a,
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        p.text(
            egui::pos2(rect.left() + 125.0, y),
            egui::Align2::LEFT_CENTER,
            b,
            egui::FontId::proportional(11.0),
            colors::TEXT_PRIMARY,
        );
    }
    p.line_segment(
        [
            egui::pos2(rect.left() + 14.0, rect.top() + 462.0),
            egui::pos2(rect.right() - 14.0, rect.top() + 462.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 490.0),
        egui::Align2::LEFT_CENTER,
        "▾  プロキシ",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    p.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 160.0, rect.top() + 472.0),
            egui::vec2(rect.width() - 174.0, 30.0),
        ),
        5.0,
        egui::Color32::from_rgb(255, 103, 24),
    );
    p.text(
        egui::pos2(
            rect.left() + 160.0 + (rect.width() - 174.0) / 2.0,
            rect.top() + 487.0,
        ),
        egui::Align2::CENTER_CENTER,
        "プロキシを生成",
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 540.0),
        egui::Align2::LEFT_CENTER,
        "▾  メディア管理",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    p.rect(
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 160.0, rect.top() + 523.0),
            egui::vec2(rect.width() - 174.0, 30.0),
        ),
        5.0,
        egui::Color32::from_rgb(20, 32, 41),
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 612.0),
        egui::Align2::LEFT_CENTER,
        "▸  使用状況",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
}
