use crate::core::timeline::{Composition, LayerType, ProjectItem, ProjectItemType};
use crate::ui::theme::colors;
use eframe::egui;

pub(crate) const SVG_LIST_VIEW: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><path d="M9 5h11M9 12h11M9 19h11"/><path d="M4 5h.01M4 12h.01M4 19h.01"/></svg>"#;

pub(crate) fn project_icon_action(
    ui: &mut egui::Ui,
    id: &'static str,
    svg: &'static str,
    tooltip: &'static str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 24.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 2.0, colors::BG_HOVER);
    }
    crate::ui::icons::render_svg_at(
        ui,
        id.to_string(),
        svg,
        egui::vec2(16.0, 16.0),
        colors::TEXT_SECONDARY,
        rect.center() - egui::vec2(8.0, 8.0),
    );
    response.on_hover_text(tooltip)
}

pub(crate) fn project_view_toggle(
    ui: &mut egui::Ui,
    id: &'static str,
    svg: &'static str,
    selected: bool,
    tooltip: &'static str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 24.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            3.0,
            if selected {
                colors::BG_ACTIVE
            } else {
                colors::BG_HOVER
            },
        );
    }
    crate::ui::icons::render_svg_at(
        ui,
        id.to_owned(),
        svg,
        egui::vec2(16.0, 16.0),
        if selected {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_SECONDARY
        },
        rect.center() - egui::vec2(8.0, 8.0),
    );
    response.on_hover_text(tooltip)
}

pub(crate) fn asset_kind(item: &ProjectItem) -> &'static str {
    match item.item_type {
        ProjectItemType::Video { .. } => "video",
        ProjectItemType::Image { .. } => "image",
        ProjectItemType::Audio { .. } => "audio",
        ProjectItemType::Model3D { .. } => "model3d",
        ProjectItemType::Composition { .. } => "composition",
        ProjectItemType::Solid { .. } => "solid",
        ProjectItemType::Folder { .. } => "folder",
    }
}

pub(crate) fn video_preview_path(
    compositions: &[Composition],
    source: &str,
) -> Option<std::path::PathBuf> {
    for comp in compositions {
        for layer in &comp.layers {
            if let LayerType::Video {
                source: layer_source,
                frames_dir,
                ..
            } = &layer.layer_type
            {
                if layer_source == source && !frames_dir.is_empty() {
                    for extension in ["webp", "png"] {
                        let path = std::path::Path::new(frames_dir)
                            .join(format!("frame_00000.{extension}"));
                        if path.is_file() {
                            return Some(path);
                        }
                    }
                }
            }
        }
        if let Some(path) = video_preview_path(&comp.sub_compositions, source) {
            return Some(path);
        }
    }
    None
}

pub(crate) fn asset_preview_path(
    project: &crate::core::timeline::Project,
    item: &ProjectItem,
) -> Option<std::path::PathBuf> {
    match &item.item_type {
        ProjectItemType::Image { path, .. } => Some(path.into()),
        ProjectItemType::Video { path, .. } => video_preview_path(&project.compositions, path),
        _ => None,
    }
}

pub(crate) fn asset_preview_texture(
    cache: &mut std::collections::HashMap<std::path::PathBuf, egui::TextureHandle>,
    failed: &mut std::collections::HashSet<std::path::PathBuf>,
    ctx: &egui::Context,
    path: &std::path::Path,
) -> Option<(egui::TextureId, egui::Vec2)> {
    if let Some(texture) = cache.get(path) {
        return Some((texture.id(), texture.size_vec2()));
    }
    if cache.len() >= 96 || failed.contains(path) {
        return None;
    }
    let image = match crate::ui::embedded_assets::open_image(path) {
        Ok(image) => image.thumbnail(512, 288),
        Err(_) => {
            failed.insert(path.to_path_buf());
            return None;
        }
    };
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    if size[0] == 0 || size[1] == 0 {
        return None;
    }
    let texture = ctx.load_texture(
        format!("project-asset-preview:{}", path.display()),
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let result = (texture.id(), texture.size_vec2());
    cache.insert(path.to_path_buf(), texture);
    Some(result)
}

pub(crate) fn draw_asset_card(
    ui: &mut egui::Ui,
    item: &ProjectItem,
    item_index: usize,
    width: f32,
    selected: bool,
    preview: Option<(egui::TextureId, egui::Vec2)>,
) -> egui::Response {
    let card_height = (width * 0.58).clamp(72.0, 94.0);
    let card_size = egui::vec2(width, card_height);
    let (card, response) = ui.allocate_exact_size(card_size, egui::Sense::click());
    let painter = ui.painter().clone();
    let bg_color = if response.hovered() {
        colors::BG_HOVER
    } else {
        colors::BG_DARKEST
    };
    painter.rect(
        card,
        6.0,
        bg_color,
        egui::Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected {
                colors::ACCENT_BRAND
            } else if response.hovered() {
                colors::BORDER_MEDIUM
            } else {
                colors::BORDER_SUBTLE
            },
        ),
    );
    let cover_h = (card_height * 0.62).round();
    let preview_rect = egui::Rect::from_min_max(
        card.left_top() + egui::vec2(3.0, 3.0),
        egui::pos2(card.right() - 3.0, card.top() + cover_h),
    );
    painter.rect_filled(preview_rect, 4.0, colors::BG_SURFACE);
    if let Some((texture, texture_size)) = preview {
        let scale =
            (preview_rect.width() / texture_size.x).max(preview_rect.height() / texture_size.y);
        let destination = egui::Rect::from_center_size(preview_rect.center(), texture_size * scale);
        painter.with_clip_rect(preview_rect).image(
            texture,
            destination,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        let icon = match item.item_type {
            ProjectItemType::Composition { .. } => crate::ui::icons::SVG_COMPOSITION,
            ProjectItemType::Video { .. } | ProjectItemType::Image { .. } => {
                crate::ui::icons::SVG_FILE
            }
            ProjectItemType::Audio { .. } => crate::ui::icons::SVG_AUDIO,
            ProjectItemType::Model3D { .. } => crate::ui::icons::SVG_SWITCH_3D,
            ProjectItemType::Solid { .. } => crate::ui::icons::SVG_WINDOW_MAXIMIZE,
            ProjectItemType::Folder { .. } => crate::ui::icons::SVG_FOLDER,
        };
        let tint = if response.hovered() {
            colors::ACCENT_BRAND
        } else {
            colors::TEXT_MUTED
        };
        let icon_size = egui::vec2(22.0, 22.0);
        crate::ui::icons::render_svg_at(
            ui,
            format!("asset-card-icon-{item_index}"),
            icon,
            icon_size,
            tint,
            preview_rect.center() - icon_size * 0.5,
        );
    }

    // Duration badge on bottom-right of preview cover (matching ideal e.g. 00:15)
    let duration_str = match &item.item_type {
        ProjectItemType::Video { duration_sec, .. }
        | ProjectItemType::Audio { duration_sec, .. }
            if duration_sec.is_finite() && *duration_sec > 0.0 =>
        {
            let secs = duration_sec.round() as u32;
            Some(format!("{:02}:{:02}", secs / 60, secs % 60))
        }
        _ => None,
    };
    if let Some(dur) = duration_str {
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(preview_rect.right() - 32.0, preview_rect.bottom() - 14.0),
            egui::vec2(30.0, 12.0),
        );
        painter.rect_filled(badge_rect, 2.0, egui::Color32::from_black_alpha(170));
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            dur,
            egui::FontId::monospace(8.5),
            egui::Color32::WHITE,
        );
    }

    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(card.left() + 6.0, preview_rect.bottom() + 3.0),
            egui::pos2(card.right() - 5.0, card.bottom() - 14.0),
        ),
        egui::Label::new(
            egui::RichText::new(&item.name)
                .size(10.5)
                .strong()
                .color(colors::TEXT_PRIMARY),
        )
        .truncate(),
    );
    let detail = match &item.item_type {
        ProjectItemType::Video { duration_sec, .. }
            if duration_sec.is_finite() && *duration_sec > 0.0 =>
        {
            format!("映像  ·  {:.2}秒", duration_sec)
        }
        ProjectItemType::Image { width, height, .. } => format!("画像  ·  {width} × {height}"),
        ProjectItemType::Audio { duration_sec, .. }
            if duration_sec.is_finite() && *duration_sec > 0.0 =>
        {
            format!("オーディオ  ·  {:.2}秒", duration_sec)
        }
        _ => match asset_kind(item) {
            "video" => "映像".to_owned(),
            "audio" => "オーディオ".to_owned(),
            "image" => "画像".to_owned(),
            "model3d" => "3Dモデル".to_owned(),
            "composition" => "コンポジション".to_owned(),
            "folder" => "フォルダー".to_owned(),
            _ => "ソリッド".to_owned(),
        },
    };
    let detail_color = if item.is_media_missing() {
        colors::ACCENT_RED
    } else {
        colors::TEXT_MUTED
    };
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(card.left() + 6.0, card.bottom() - 12.0),
            egui::pos2(card.right() - 6.0, card.bottom() - 1.0),
        ),
        egui::Label::new(egui::RichText::new(&detail).size(8.5).color(detail_color)).truncate(),
    );
    if item.is_media_missing() {
        response.on_hover_text(format!(
            "ファイルが見つかりません: {}",
            item.media_path().unwrap_or_default()
        ))
    } else {
        response.on_hover_text(format!("{}\n{}", item.name, detail))
    }
}
