//! Grid view for browsing assets with type filtering and card selection.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::detail::{asset_kind_label, asset_kind_matches, item_type_label};

pub fn draw_grid(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    narrow: bool,
) {
    let painter = ui.painter().clone();
    let kind_filter_id = egui::Id::new("asset-library-kind-filter");
    let mut selected_kind = ctx
        .data(|data| data.get_temp::<String>(kind_filter_id))
        .unwrap_or_else(|| "all".to_owned());
    let filter_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 12.0, rect.top() + 8.0),
        egui::pos2(
            (rect.left() + 260.0).min(rect.right() - 12.0),
            rect.top() + 40.0,
        ),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(filter_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("種類")
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
                egui::ComboBox::from_id_salt("asset-library-kind-filter-combo")
                    .selected_text(asset_kind_label(&selected_kind))
                    .width((filter_rect.width() - 36.0).max(100.0))
                    .show_ui(ui, |ui| {
                        for (key, label) in [
                            ("all", "すべて"),
                            ("video", "映像"),
                            ("image", "画像"),
                            ("audio", "オーディオ"),
                            ("model3d", "3Dモデル"),
                            ("composition", "コンポジション"),
                            ("solid", "ソリッド"),
                        ] {
                            ui.selectable_value(&mut selected_kind, key.to_owned(), label);
                        }
                    });
            });
        },
    );
    let previous_kind = ctx.data(|data| data.get_temp::<String>(kind_filter_id));
    if previous_kind.as_deref() != Some(selected_kind.as_str()) {
        ctx.data_mut(|data| {
            data.insert_temp(kind_filter_id, selected_kind.clone());
            data.remove::<String>(egui::Id::new("asset-library-selected-id"));
        });
    }
    let folder_id =
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("asset-library-folder-filter")));
    let items: Vec<_> = app
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
        .filter(|item| {
            folder_id
                .as_deref()
                .is_none_or(|folder| item.parent_folder.as_deref() == Some(folder))
        })
        .filter(|item| asset_kind_matches(item, &selected_kind))
        .cloned()
        .collect();
    let selected_id =
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("asset-library-selected-id")));
    if items.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if folder_id.is_some() || selected_kind != "all" {
                "この条件に一致する素材はありません"
            } else {
                "このプロジェクトに素材はありません"
            },
            egui::FontId::proportional(15.0),
            colors::TEXT_MUTED,
        );
        return;
    }
    let cols = if rect.width() < 360.0 {
        1
    } else if rect.width() < 620.0 {
        2
    } else {
        4
    };
    let gap = super::metrics::CARD_GAP;
    let card_h = if narrow {
        super::metrics::CARD_HEIGHT_COMPACT
    } else {
        super::metrics::CARD_HEIGHT
    };
    let row_step = card_h + gap;
    let card_w = ((rect.width() - super::metrics::PANEL_GUTTER * 2.0 - gap * (cols - 1) as f32)
        / cols as f32)
        .max(112.0);
    for (index, item) in items.iter().enumerate() {
        let x = rect.left() + super::metrics::PANEL_GUTTER + (index % cols) as f32 * (card_w + gap);
        let y = rect.top() + super::metrics::PANEL_HEADER_HEIGHT + (index / cols) as f32 * row_step;
        let card = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));
        if !ui.is_rect_visible(card) {
            continue;
        }
        let selected = selected_id.as_deref() == Some(item.id.as_str());
        let response = ui.interact(
            card,
            egui::Id::new(("asset-card", item.id.as_str())),
            egui::Sense::click(),
        );
        painter.rect(
            card,
            7.0,
            if response.hovered() {
                colors::BG_HOVER
            } else {
                crate::ui::theme::colors::BG_PANEL_BASE
            },
            egui::Stroke::new(
                if selected { 2.0_f32 } else { 1.0_f32 },
                if selected {
                    crate::ui::theme::colors::ACCENT_BRAND
                } else {
                    colors::BORDER_SUBTLE
                },
            ),
        );
        let label = item_type_label(item);
        painter.text(
            egui::pos2(card.left() + 12.0, card.top() + 23.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            crate::ui::theme::colors::ACCENT_BRAND_HOVER,
        );
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(card.left() + 12.0, card.top() + 37.0),
                egui::pos2(card.right() - 12.0, card.top() + 67.0),
            ),
            egui::Label::new(
                egui::RichText::new(&item.name)
                    .size(13.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
        let status = if item.is_media_missing() {
            "ファイルが見つかりません"
        } else {
            "プロジェクト素材"
        };
        painter.text(
            egui::pos2(card.left() + 12.0, card.bottom() - 16.0),
            egui::Align2::LEFT_CENTER,
            status,
            egui::FontId::proportional(10.0),
            if item.is_media_missing() {
                colors::TEXT_MUTED
            } else {
                colors::TEXT_SECONDARY
            },
        );
        if response.clicked() {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("asset-library-selected-id"), item.id.clone())
            });
        }
    }
}
