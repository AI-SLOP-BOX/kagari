pub mod breadcrumb;
mod canvas;
pub mod geometry;
pub mod header;
mod keyframe_menu;
pub mod keyframe_rows;
pub mod keyframe_tick;
pub mod layers;
pub mod layout;
mod media_preview;
pub mod pending_actions;
pub mod precomp_children;
pub mod ruler;
pub mod ruler_bar;
pub mod snapping;
mod state;
pub mod utils;
mod waveform_cache;

use crate::core::timeline::LayerType;
use crate::KagariApp;
use eframe::egui;

const TIMELINE_THUMBNAIL_WIDTH: f32 = 54.0;
const MAX_TIMELINE_THUMBNAILS_PER_CLIP: usize = 12;

struct LayerWaveformParams<'a> {
    timeline_rect: egui::Rect,
    clip_rect: egui::Rect,
    layer: &'a crate::core::timeline::Layer,
    waveform: &'a waveform_cache::WaveformData,
    start_frame: u32,
    visible_span: u32,
    fps: u32,
    color: egui::Color32,
}

fn draw_layer_waveform(painter: &egui::Painter, params: LayerWaveformParams<'_>) {
    let LayerWaveformParams {
        timeline_rect,
        clip_rect,
        layer,
        waveform,
        start_frame,
        visible_span,
        fps,
        color,
    } = params;

    if waveform.peaks.len() < 2
        || waveform.duration_seconds <= 0.0
        || timeline_rect.width() <= 8.0
        || clip_rect.width() <= 0.0
    {
        return;
    }
    let fps = fps.max(1) as f32;
    let amp = clip_rect.height() * 0.40;
    let mut x = clip_rect.left() + 2.0;
    while x <= clip_rect.right() - 2.0 {
        let t = (x - timeline_rect.left()) / timeline_rect.width();
        let comp_frame = start_frame as f32 + t * visible_span.max(1) as f32;
        let source_frame = match &layer.layer_type {
            LayerType::Video { speed, .. } => {
                layer.remap_frame_f32(comp_frame.round() as u32) * speed.max(0.0)
            }
            _ => comp_frame - layer.in_frame as f32,
        };
        let seconds = source_frame / fps;
        if seconds >= 0.0 && seconds < waveform.duration_seconds {
            let index =
                ((seconds / waveform.duration_seconds) * waveform.peaks.len() as f32) as usize;
            let peak = waveform.peaks[index.min(waveform.peaks.len() - 1)];
            let height = (peak * amp).clamp(1.0, amp.max(1.0));
            let middle = clip_rect.center().y;
            painter.line_segment(
                [
                    egui::pos2(x, middle - height),
                    egui::pos2(x, middle + height),
                ],
                egui::Stroke::new(1.0_f32, color),
            );
        }
        x += 2.0;
    }
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    canvas::draw(app, ctx, current_frame, total_frames);
}

#[cfg(test)]
mod tests {
    use super::state::would_create_parent_cycle;
    use std::collections::HashMap;

    #[test]
    fn parent_pick_whip_rejects_descendant_and_self() {
        let parents = HashMap::from([
            ("child".to_string(), "middle".to_string()),
            ("middle".to_string(), "root".to_string()),
        ]);

        assert!(would_create_parent_cycle("root", "child", &parents));
        assert!(would_create_parent_cycle("middle", "middle", &parents));
        assert!(!would_create_parent_cycle("child", "root", &parents));
    }
}
