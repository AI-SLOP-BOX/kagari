//! RAM Preview cache bar + comp markers + beat-detection transients.
use crate::core::frame_cache::FrameCache;
use crate::core::timeline::{Composition, LayerType};
use crate::ui::theme::colors;
use eframe::egui;

pub fn draw_ram_ruler(
    frame_cache: &FrameCache,
    comp_mut: &mut Composition,
    ui: &mut egui::Ui,
    total_frames: u32,
    ruler_rect: egui::Rect,
    start_frame: u32,
    visible_span: u32,
) {
    let bar_height = 3.0;
    let bar_rect =
        egui::Rect::from_min_size(ruler_rect.min, egui::vec2(ruler_rect.width(), bar_height));
    ui.painter().rect_filled(bar_rect, 0.0, colors::BG_DARKEST);

    let visible_span = visible_span.max(1);
    let first_frame = start_frame.min(total_frames);
    let last_frame = first_frame.saturating_add(visible_span).min(total_frames);
    if last_frame > first_frame {
        let frame_w = bar_rect.width() / visible_span as f32;
        // Vertex-count safety: coalesce consecutive cached frames
        // into ONE rect per run instead of one painter call per frame
        // (a 10k-frame comp would otherwise emit 10k quads every frame).
        let mut run_start: Option<u32> = None;
        for f in first_frame..=last_frame {
            let cached = f < last_frame && frame_cache.is_cached(f);
            if cached && run_start.is_none() {
                run_start = Some(f);
            } else if !cached {
                if let Some(s) = run_start.take() {
                    let x = bar_rect.left() + s.saturating_sub(first_frame) as f32 * frame_w;
                    let w = ((f - s) as f32 * frame_w).max(1.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x, bar_rect.top()),
                            egui::vec2(w, bar_height),
                        ),
                        0.0,
                        colors::TIMELINE_CACHE,
                    );
                }
            }
        }
    }

    // Render Timeline Markers & Beat Detection Transients
    // Real beat-detection transient lines from the comp's audio
    // sources (energy-flux onsets, cached per audio file).
    let audio_paths: Vec<String> = comp_mut
        .layers
        .iter()
        .filter_map(|l| match &l.layer_type {
            LayerType::Audio { path, .. } => Some(path.clone()),
            LayerType::Video {
                audio_wav: Some(w), ..
            } => Some(w.clone()),
            _ => None,
        })
        .collect();
    let fps_now = comp_mut.fps.max(1) as f32;
    let mut beat_frames: Vec<u32> = Vec::new();
    for ap in &audio_paths {
        let key = egui::Id::new(("beat_frames", ap.as_str()));
        let frames: std::sync::Arc<Vec<u32>> = ui.ctx().data_mut(|d| {
            d.get_temp::<std::sync::Arc<Vec<u32>>>(key)
                .unwrap_or_else(|| {
                    let v = crate::core::audio_engine::detect_beat_frames(
                        std::path::Path::new(ap),
                        total_frames,
                        fps_now,
                    );
                    let arc = std::sync::Arc::new(v);
                    d.insert_temp(key, arc.clone());
                    arc
                })
        });
        beat_frames.extend(frames.iter().copied());
    }
    beat_frames.sort_unstable();
    beat_frames.dedup();
    // Cap drawn lines to keep painter calls bounded (~200)
    if beat_frames.len() > 200 {
        let keep = beat_frames.len() / 200;
        beat_frames = beat_frames.into_iter().step_by(keep).collect();
    }
    for bf in beat_frames
        .iter()
        .filter(|frame| **frame >= first_frame && **frame < last_frame)
    {
        let b_norm = bf.saturating_sub(first_frame) as f32 / visible_span as f32;
        let bx = bar_rect.left() + b_norm * bar_rect.width();
        ui.painter().line_segment(
            [
                egui::pos2(bx, bar_rect.top()),
                egui::pos2(bx, bar_rect.bottom()),
            ],
            egui::Stroke::new(1.0_f32, colors::ACCENT_BEAT_MARKER),
        );
    }

    for marker in &comp_mut.markers {
        if marker.frame >= first_frame && marker.frame < last_frame {
            let norm = marker.frame.saturating_sub(first_frame) as f32 / visible_span as f32;
            let mx = bar_rect.left() + norm * bar_rect.width();
            let m_pts = vec![
                egui::pos2(mx - 4.0, bar_rect.top()),
                egui::pos2(mx + 4.0, bar_rect.top()),
                egui::pos2(mx, bar_rect.top() + 7.0),
            ];
            let mc = egui::Color32::from_rgb(
                (marker.color[0] * 255.0) as u8,
                (marker.color[1] * 255.0) as u8,
                (marker.color[2] * 255.0) as u8,
            );
            ui.painter()
                .add(egui::Shape::convex_polygon(m_pts, mc, egui::Stroke::NONE));
        }
    }
}
