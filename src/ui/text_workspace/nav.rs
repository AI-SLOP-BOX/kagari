//! Sidebar branding logo, icons, and navigation for text workspace.

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_logo(
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
                crate::ui::theme::colors::TEXT_SECONDARY,
            );
        }
    }
}

pub fn draw_nav(ui: &mut egui::Ui, sidebar: egui::Rect, r: egui::Rect, narrow: bool) {
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
            format!("text-nav-{i}"),
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
    ui.painter().line_segment(
        [
            egui::pos2(25.0, r.top() + 247.0),
            egui::pos2(sidebar.right() - 25.0, r.top() + 247.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
}

pub fn texture(ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/home")
        .join(name);
    let id = egui::Id::new(("text-workspace", name));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t.id());
    }
    let img = crate::ui::embedded_assets::open_image(path)
        .ok()?
        .to_rgba8();
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
