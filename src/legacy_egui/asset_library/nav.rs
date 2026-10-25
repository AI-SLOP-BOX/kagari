//! Header area and folder list navigation for asset library.

use crate::ui::icons;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_header(app: &mut KagariApp, ui: &mut egui::Ui, r: egui::Rect, base: f32, narrow: bool) {
    let p = ui.painter().clone();
    p.line_segment(
        [
            egui::pos2(base + 14.0, r.top() + 52.0),
            egui::pos2(r.right() - 20.0, r.top() + 52.0),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    icons::render_svg_at(
        ui,
        "asset-breadcrumb-folder".to_string(),
        icons::SVG_FOLDER,
        egui::vec2(18.0, 18.0),
        colors::TEXT_SECONDARY,
        egui::pos2(base + 14.0, r.top() + 18.0),
    );
    p.text(
        egui::pos2(base + 40.0, r.top() + 27.0),
        egui::Align2::LEFT_CENTER,
        "素材ライブラリ    ›    すべての素材",
        egui::FontId::proportional(if narrow { 12.0 } else { 14.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(base + 14.0, r.top() + 84.0),
        egui::Align2::LEFT_CENTER,
        "素材ライブラリ",
        egui::FontId::proportional(if narrow { 22.0 } else { 28.0 }),
        colors::TEXT_PRIMARY,
    );
    let import_rect = egui::Rect::from_min_size(
        egui::pos2(
            r.right() - if narrow { 160.0 } else { 187.0 },
            r.top() + 63.0,
        ),
        egui::vec2(if narrow { 140.0 } else { 167.0 }, 42.0),
    );
    let import_response = ui.interact(
        import_rect,
        egui::Id::new("asset-library-import"),
        egui::Sense::click(),
    );
    p.rect_filled(
        import_rect,
        7.0,
        if import_response.hovered() {
            crate::ui::theme::colors::ACCENT_BRAND_HOVER
        } else {
            crate::ui::theme::colors::ACCENT_BRAND
        },
    );
    icons::render_svg_at(
        ui,
        "asset-import".to_string(),
        icons::SVG_IMPORT,
        egui::vec2(18.0, 18.0),
        egui::Color32::WHITE,
        egui::pos2(import_rect.left() + 12.0, import_rect.top() + 12.0),
    );
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(import_rect.left() + 37.0, import_rect.top()),
            egui::pos2(import_rect.right() - 4.0, import_rect.bottom()),
        ),
        egui::Label::new(
            egui::RichText::new("素材をインポート")
                .size(12.0)
                .color(egui::Color32::WHITE),
        )
        .truncate(),
    );
    if import_response.clicked() {
        crate::ui::home_screen::import_footage_dialog(app, ui.ctx());
    }
    let stock_rect = egui::Rect::from_min_size(
        egui::pos2(
            import_rect.left() - if narrow { 148.0 } else { 158.0 },
            import_rect.top(),
        ),
        egui::vec2(if narrow { 140.0 } else { 150.0 }, import_rect.height()),
    );
    let stock_response = ui.interact(
        stock_rect,
        egui::Id::new("asset-library-stock"),
        egui::Sense::click(),
    );
    p.rect_filled(
        stock_rect,
        7.0,
        if stock_response.hovered() {
            colors::BG_HOVER
        } else {
            crate::ui::theme::colors::BG_PANEL_BASE
        },
    );
    p.rect_stroke(
        stock_rect,
        7.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
    );
    p.text(
        stock_rect.center(),
        egui::Align2::CENTER_CENTER,
        "内蔵素材パック",
        egui::FontId::proportional(12.0),
        colors::TEXT_PRIMARY,
    );
    if stock_response.clicked() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("asset-library-stock-dialog"), true));
    }
    let description_rect = egui::Rect::from_min_max(
        egui::pos2(base + 14.0, r.top() + 106.0),
        egui::pos2((stock_rect.left() - 12.0).max(base + 18.0), r.top() + 132.0),
    );
    p.with_clip_rect(description_rect).text(
        description_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        "映像、画像、オーディオ、3Dモデルなどの素材を管理します。",
        egui::FontId::proportional(13.0),
        colors::TEXT_SECONDARY,
    );
    let asset_count = app
        .history
        .current()
        .assets
        .iter()
        .filter(|item| {
            !matches!(
                item.item_type,
                crate::core::timeline::ProjectItemType::Folder { .. }
            )
        })
        .count();
    p.text(
        egui::pos2(base + 14.0, r.top() + 143.0),
        egui::Align2::LEFT_CENTER,
        format!("{asset_count} 件の素材"),
        egui::FontId::proportional(13.0),
        colors::TEXT_SECONDARY,
    );
}

pub fn draw_folders(app: &mut KagariApp, ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    let p = ui.painter().clone();
    let compact = rect.width() < 200.0;
    let row_step = if compact {
        super::metrics::FOLDER_ROW_COMPACT
    } else {
        super::metrics::FOLDER_ROW
    };
    p.text(
        egui::pos2(rect.left() + 14.0, rect.top() + 25.0),
        egui::Align2::LEFT_CENTER,
        if compact {
            "フォルダ"
        } else {
            "フォルダー"
        },
        egui::FontId::proportional(15.0),
        colors::TEXT_PRIMARY,
    );
    let project_items = app.history.current().assets.clone();
    let mut folders: Vec<_> = project_items
        .iter()
        .filter_map(|item| match &item.item_type {
            crate::core::timeline::ProjectItemType::Folder { name } => {
                Some((item.id.clone(), name.clone()))
            }
            _ => None,
        })
        .collect();
    folders.sort_by(|left, right| left.1.cmp(&right.1));
    let folder_filter_id = egui::Id::new("asset-library-folder-filter");
    let selected_folder = ctx.data(|data| data.get_temp::<String>(folder_filter_id));
    let rows = std::iter::once((None, "すべての素材".to_string()))
        .chain(folders.into_iter().map(|(id, name)| (Some(id), name)))
        .collect::<Vec<_>>();
    for (index, (folder_id, name)) in rows.iter().enumerate() {
        let y = rect.top() + super::metrics::PANEL_HEADER_HEIGHT + index as f32 * row_step;
        let row = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, y - 13.0),
            egui::vec2(rect.width() - 12.0, row_step - 2.0),
        );
        let active = match (selected_folder.as_deref(), folder_id.as_deref()) {
            (None, None) => true,
            (Some(selected), Some(folder)) => selected == folder,
            _ => false,
        };
        let response = ui.interact(
            row,
            egui::Id::new(("asset-library-folder", index)),
            egui::Sense::click(),
        );
        if active || response.hovered() {
            p.rect_filled(
                row,
                4.0,
                if active {
                    crate::ui::theme::colors::BG_PANEL_DARK
                } else {
                    colors::BG_HOVER
                },
            );
        }
        icons::render_svg_at(
            ui,
            format!("asset-folder-icon-{index}"),
            icons::SVG_FOLDER,
            egui::vec2(16.0, 16.0),
            if active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
            egui::pos2(row.left() + 10.0, row.center().y - 8.0),
        );
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(row.left() + 34.0, row.top()),
                egui::pos2(row.right() - 6.0, row.bottom()),
            ),
            egui::Label::new(
                egui::RichText::new(name)
                    .size(if compact { 11.0 } else { 12.0 })
                    .color(if active {
                        colors::TEXT_PRIMARY
                    } else {
                        colors::TEXT_SECONDARY
                    }),
            )
            .truncate(),
        );
        if response.clicked() {
            ctx.data_mut(|data| match folder_id {
                Some(id) => data.insert_temp(folder_filter_id, id.clone()),
                None => data.remove::<String>(folder_filter_id),
            });
        }
    }
}
