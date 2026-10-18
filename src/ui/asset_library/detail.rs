//! Detail inspector panel and format helpers for selected assets.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn asset_kind_label(key: &str) -> &'static str {
    match key {
        "video" => "映像",
        "image" => "画像",
        "audio" => "オーディオ",
        "model3d" => "3Dモデル",
        "composition" => "コンポジション",
        "solid" => "ソリッド",
        _ => "すべて",
    }
}

pub fn asset_kind_matches(item: &crate::core::timeline::ProjectItem, key: &str) -> bool {
    use crate::core::timeline::ProjectItemType;

    match key {
        "video" => matches!(item.item_type, ProjectItemType::Video { .. }),
        "image" => matches!(item.item_type, ProjectItemType::Image { .. }),
        "audio" => matches!(item.item_type, ProjectItemType::Audio { .. }),
        "model3d" => matches!(item.item_type, ProjectItemType::Model3D { .. }),
        "composition" => matches!(item.item_type, ProjectItemType::Composition { .. }),
        "solid" => matches!(item.item_type, ProjectItemType::Solid { .. }),
        _ => true,
    }
}

pub fn item_type_label(item: &crate::core::timeline::ProjectItem) -> &'static str {
    match &item.item_type {
        crate::core::timeline::ProjectItemType::Composition { .. } => "コンポジション",
        crate::core::timeline::ProjectItemType::Image { .. } => "画像",
        crate::core::timeline::ProjectItemType::Model3D { .. } => "3Dモデル",
        crate::core::timeline::ProjectItemType::Video { .. } => "映像",
        crate::core::timeline::ProjectItemType::Audio { .. } => "オーディオ",
        crate::core::timeline::ProjectItemType::Solid { .. } => "ソリッド",
        crate::core::timeline::ProjectItemType::Folder { .. } => "フォルダー",
    }
}

pub fn format_file_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn draw_detail(
    app: &KagariApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    narrow: bool,
) {
    let p = ui.painter().clone();
    let selected_id =
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("asset-library-selected-id")));
    let selected = selected_id.and_then(|id| {
        app.history
            .current()
            .assets
            .iter()
            .find(|item| item.id == id)
    });
    p.text(
        egui::pos2(rect.left() + 16.0, rect.top() + 26.0),
        egui::Align2::LEFT_CENTER,
        "素材の詳細",
        egui::FontId::proportional(15.0),
        colors::TEXT_PRIMARY,
    );
    let preview = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 14.0, rect.top() + 42.0),
        egui::vec2(rect.width() - 28.0, if narrow { 96.0 } else { 155.0 }),
    );
    p.rect_filled(preview, 6.0, crate::ui::theme::colors::BG_DARKEST);
    p.rect_stroke(
        preview,
        6.0,
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let Some(item) = selected else {
        p.text(
            preview.center(),
            egui::Align2::CENTER_CENTER,
            "素材を選択すると詳細を表示します",
            egui::FontId::proportional(12.0),
            colors::TEXT_MUTED,
        );
        return;
    };
    p.text(
        egui::pos2(preview.center().x, preview.center().y),
        egui::Align2::CENTER_CENTER,
        item_type_label(item),
        egui::FontId::proportional(16.0),
        colors::TEXT_SECONDARY,
    );
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 14.0, preview.bottom() + 8.0),
            egui::pos2(rect.right() - 14.0, preview.bottom() + 34.0),
        ),
        egui::Label::new(
            egui::RichText::new(&item.name)
                .size(if narrow { 12.0 } else { 14.0 })
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    );
    let path = item.media_path();
    let mut details = vec![("種類", item_type_label(item).to_string())];
    if let Some(path) = path {
        details.push(("保存先", path.to_string()));
        details.push((
            "状態",
            if item.is_media_missing() {
                "ファイルが見つかりません"
            } else {
                "利用可能"
            }
            .to_string(),
        ));
        if let Ok(metadata) = std::fs::metadata(path) {
            details.push(("サイズ", format_file_size(metadata.len())));
        }
    }
    for (index, (label, value)) in details.into_iter().enumerate() {
        let y = preview.bottom() + 55.0 + index as f32 * if narrow { 34.0 } else { 42.0 };
        p.text(
            egui::pos2(rect.left() + 14.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            colors::TEXT_SECONDARY,
        );
        ui.put(
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + if narrow { 68.0 } else { 108.0 }, y - 12.0),
                egui::pos2(rect.right() - 12.0, y + 12.0),
            ),
            egui::Label::new(
                egui::RichText::new(value)
                    .size(11.0)
                    .color(colors::TEXT_PRIMARY),
            )
            .truncate(),
        );
    }
}
