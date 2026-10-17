use crate::core::timeline::{Layer, LayerType};
use crate::ui::theme::colors;
use eframe::egui;

type PreviewTextures<'a> = (f32, f32, &'a [Option<egui::TextureId>]);

pub(super) struct ClipBarParams<'a> {
    pub(super) layer: &'a Layer,
    pub(super) selected: bool,
    pub(super) has_parent: bool,
    pub(super) bar_rect: egui::Rect,
    pub(super) start_frame: u32,
    pub(super) zoom_span: u32,
    pub(super) comp_width: f32,
    pub(super) comp_height: f32,
    pub(super) fps: u32,
    pub(super) preview_textures: Option<&'a (f32, f32, Vec<Option<egui::TextureId>>)>,
}

pub(super) fn draw_clip_bar(ui: &mut egui::Ui, params: ClipBarParams<'_>) -> egui::Rect {
    let ClipBarParams {
        layer,
        selected,
        has_parent,
        bar_rect,
        start_frame,
        zoom_span,
        comp_width,
        comp_height,
        fps,
        preview_textures,
    } = params;
    if has_parent {
        ui.painter().text(
            egui::pos2(bar_rect.left() + 8.0, bar_rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            "↔",
            egui::FontId::proportional(10.0),
            colors::ACCENT_PURPLE,
        );
    }

    let norm_in = layer.in_frame.saturating_sub(start_frame) as f32 / zoom_span as f32;
    let norm_out = layer.out_frame.saturating_sub(start_frame) as f32 / zoom_span as f32;
    let layer_rect = egui::Rect::from_min_max(
        egui::pos2(
            bar_rect.left() + norm_in * bar_rect.width(),
            bar_rect.top() + 4.0,
        ),
        egui::pos2(
            bar_rect.left() + norm_out * bar_rect.width(),
            bar_rect.bottom() - 4.0,
        ),
    );

    let label_fill = layer_label_color(layer, selected);
    let fill_color = match &layer.layer_type {
        LayerType::Solid { color } => egui::Color32::from_rgba_unmultiplied(
            (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        ),
        LayerType::Text { .. } if layer.label == crate::core::timeline::LabelColor::None => {
            egui::Color32::from_rgb(112, 48, 160)
        }
        LayerType::AdjustmentLayer if layer.label == crate::core::timeline::LabelColor::None => {
            egui::Color32::from_rgb(98, 42, 172)
        }
        LayerType::Audio { .. } if layer.label == crate::core::timeline::LabelColor::None => {
            egui::Color32::from_rgb(18, 92, 84)
        }
        LayerType::Video { .. } if layer.label == crate::core::timeline::LabelColor::None => {
            egui::Color32::from_rgb(28, 56, 108)
        }
        _ => label_fill,
    };

    let has_preview = super::super::media_preview::layer_has_timeline_preview(&layer.layer_type);
    let bar_fill = if has_preview {
        egui::Color32::from_rgb(17, 20, 25)
    } else {
        fill_color
    };
    ui.painter().rect_filled(layer_rect, 2.0, bar_fill);
    if has_preview {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                layer_rect.left_top(),
                egui::pos2(
                    (layer_rect.left() + 3.0).min(layer_rect.right()),
                    layer_rect.bottom(),
                ),
            ),
            1.0,
            fill_color,
        );
    }

    let preview_painter = ui.painter().with_clip_rect(layer_rect.shrink(1.0));
    if layer_rect.width() > 46.0 {
        if let Some((preview_start, preview_end, textures)) = preview_textures {
            draw_video_preview_tiles(
                &preview_painter,
                layer_rect,
                (*preview_start, *preview_end, textures.as_slice()),
                comp_width,
                comp_height,
            );
        }
    }
    draw_clip_label(&preview_painter, layer, layer_rect);
    ui.painter().rect_stroke(
        layer_rect,
        2.0,
        egui::Stroke::new(1.0_f32, colors::TEXT_SECONDARY),
    );
    draw_audio_waveform(ui, layer, bar_rect, layer_rect, start_frame, zoom_span, fps);
    layer_rect
}

fn layer_label_color(layer: &Layer, selected: bool) -> egui::Color32 {
    let [r, g, b] = layer.label.to_rgb();
    if selected {
        egui::Color32::from_rgb((r * 170.0) as u8, (g * 170.0) as u8, (b * 170.0) as u8)
    } else {
        egui::Color32::from_rgb(
            (r * 115.0 + 14.0) as u8,
            (g * 115.0 + 16.0) as u8,
            (b * 115.0 + 19.0) as u8,
        )
    }
}

fn draw_video_preview_tiles(
    painter: &egui::Painter,
    layer_rect: egui::Rect,
    preview: PreviewTextures<'_>,
    comp_width: f32,
    comp_height: f32,
) {
    let (preview_start, preview_end, textures) = preview;
    let preview_left = layer_rect.left() + layer_rect.width() * preview_start;
    let preview_right = layer_rect.left() + layer_rect.width() * preview_end;
    let tile_width = (preview_right - preview_left).max(0.0) / textures.len().max(1) as f32;
    for (tile_index, texture) in textures.iter().enumerate() {
        let x = preview_left + tile_index as f32 * tile_width;
        let tile_right = if tile_index + 1 == textures.len() {
            preview_right
        } else {
            x + tile_width + 0.5
        };
        let Some(texture) = texture.filter(|_| tile_width > 0.0) else {
            continue;
        };
        let tile = egui::Rect::from_min_max(
            egui::pos2(x, layer_rect.top()),
            egui::pos2(tile_right.min(layer_rect.right()), layer_rect.bottom()),
        );
        let source_aspect = comp_width / comp_height;
        let tile_aspect = tile.width() / tile.height().max(1.0);
        let uv = if source_aspect > tile_aspect {
            let visible = tile_aspect / source_aspect.max(0.01);
            egui::Rect::from_min_max(
                egui::pos2((1.0 - visible) * 0.5, 0.0),
                egui::pos2(1.0 - (1.0 - visible) * 0.5, 1.0),
            )
        } else {
            let visible = source_aspect / tile_aspect.max(0.01);
            egui::Rect::from_min_max(
                egui::pos2(0.0, (1.0 - visible) * 0.5),
                egui::pos2(1.0, 1.0 - (1.0 - visible) * 0.5),
            )
        };
        painter.image(texture, tile, uv, egui::Color32::WHITE);
    }
}

fn draw_clip_label(painter: &egui::Painter, layer: &Layer, layer_rect: egui::Rect) {
    if layer_rect.width() <= 72.0 {
        return;
    }
    let label = layer.name.as_str();
    let label_galley = painter.layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(10.0),
        egui::Color32::WHITE,
    );
    let label_rect = egui::Rect::from_min_size(
        egui::pos2(layer_rect.left() + 2.0, layer_rect.top() + 1.0),
        egui::vec2(
            (label_galley.size().x + 10.0).min(layer_rect.width() - 4.0),
            (label_galley.size().y + 4.0)
                .min(layer_rect.height() - 2.0)
                .max(1.0),
        ),
    );
    painter.rect_filled(label_rect, 1.0, egui::Color32::from_black_alpha(92));
    painter.with_clip_rect(label_rect).text(
        label_rect.left_center() + egui::vec2(4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(10.0),
        egui::Color32::WHITE,
    );
}

fn draw_audio_waveform(
    ui: &egui::Ui,
    layer: &Layer,
    bar_rect: egui::Rect,
    layer_rect: egui::Rect,
    start_frame: u32,
    zoom_span: u32,
    fps: u32,
) {
    let audio_path = match &layer.layer_type {
        LayerType::Audio { path, .. } => Some(path.as_str()),
        LayerType::Video {
            audio_wav: Some(path),
            ..
        } => Some(path.as_str()),
        _ => None,
    };
    let Some(waveform) = audio_path
        .and_then(|path| super::super::waveform_cache::get(ui.ctx(), std::path::Path::new(path)))
    else {
        return;
    };
    super::super::draw_layer_waveform(
        &ui.painter().with_clip_rect(layer_rect),
        super::super::LayerWaveformParams {
            timeline_rect: bar_rect,
            clip_rect: layer_rect,
            layer,
            waveform: &waveform,
            start_frame,
            visible_span: zoom_span,
            fps,
            color: colors::TIMELINE_WAVEFORM,
        },
    );
}
