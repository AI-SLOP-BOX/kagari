//! Grid lines, safe action/title guides, and HUD diagnostic badges for viewport.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_grid(ui: &mut egui::Ui, origin_x: f32, origin_y: f32, draw_w: f32, draw_h: f32) {
    let grid_step = draw_w / 16.0;
    let grid_stroke = egui::Stroke::new(0.5_f32, colors::GRID_LINE);
    let mut gx = origin_x + grid_step;
    while gx < origin_x + draw_w {
        ui.painter().line_segment(
            [egui::pos2(gx, origin_y), egui::pos2(gx, origin_y + draw_h)],
            grid_stroke,
        );
        gx += grid_step;
    }
    let mut gy = origin_y + grid_step;
    while gy < origin_y + draw_h {
        ui.painter().line_segment(
            [egui::pos2(origin_x, gy), egui::pos2(origin_x + draw_w, gy)],
            grid_stroke,
        );
        gy += grid_step;
    }
}

pub fn draw_safe_guides(ui: &mut egui::Ui, origin_x: f32, origin_y: f32, draw_w: f32, draw_h: f32) {
    let guide_stroke = egui::Stroke::new(1.0_f32, colors::GUIDE_LINE);
    let cx = origin_x + draw_w * 0.5;
    let cy = origin_y + draw_h * 0.5;

    // Action Safe (90%)
    let as_w = draw_w * 0.9;
    let as_h = draw_h * 0.9;
    let as_rect = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(as_w, as_h));
    ui.painter().rect_stroke(as_rect, 0.0, guide_stroke);

    // Title Safe (80%)
    let ts_w = draw_w * 0.8;
    let ts_h = draw_h * 0.8;
    let ts_rect = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(ts_w, ts_h));
    ui.painter().rect_stroke(ts_rect, 0.0, guide_stroke);

    // Center Crosshair
    ui.painter().line_segment(
        [egui::pos2(cx - 10.0, cy), egui::pos2(cx + 10.0, cy)],
        guide_stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(cx, cy - 10.0), egui::pos2(cx, cy + 10.0)],
        guide_stroke,
    );
}

pub fn draw_hud_badges(
    ui: &mut egui::Ui,
    app: &KagariApp,
    ctx: &egui::Context,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    current_frame: u32,
    rendered_gpu: bool,
) {
    if app.viewport_show_stats && draw_w >= 520.0 {
        let backend_text = if rendered_gpu {
            "[GPU] WGPU Acceleration"
        } else {
            "[CPU] Software Canvas"
        };
        let backend_color = if rendered_gpu {
            colors::ACCENT_GREEN
        } else {
            colors::ACCENT_ORANGE
        };

        // Top Left Performance & FPS HUD Overlay
        let dt = ctx.input(|i| i.stable_dt.max(0.001));
        let real_fps = (1.0 / dt).clamp(1.0, 240.0);
        let render_ms = dt * 1000.0;
        let cached_frames = app.frame_cache.cached_count();
        let total_comp_frames = app
            .history
            .current()
            .active_composition()
            .duration_frames
            .max(1);
        let _cache_pct =
            (cached_frames as f32 / total_comp_frames as f32 * 100.0).clamp(0.0, 100.0);

        // Adaptive quality indicator
        let quality_pct = (app.playback.adaptive_preview_factor * 100.0).round();
        let quality_label = if app.playback.adaptive_preview_factor >= 1.0 {
            "FULL"
        } else if app.playback.adaptive_preview_factor >= 0.5 {
            "½"
        } else {
            "¼"
        };

        let fps_text = format!(
            "{:.1}ms ({:.0} FPS) | Q:{}% ({}) | RAM:{}/{}",
            render_ms, real_fps, quality_pct, quality_label, cached_frames, total_comp_frames
        );
        let fps_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x + 10.0, origin_y + 10.0),
            egui::vec2(320.0, 24.0),
        );
        ui.painter().rect_filled(fps_rect, 4.0, colors::HUD_BG);
        let stroke_c = if render_ms > 33.3 {
            colors::FPS_BAD
        } else {
            colors::FPS_GOOD
        };
        ui.painter()
            .rect_stroke(fps_rect, 4.0, egui::Stroke::new(1.0_f32, stroke_c));
        ui.painter().text(
            fps_rect.center(),
            egui::Align2::CENTER_CENTER,
            fps_text,
            egui::FontId::monospace(10.5),
            colors::HUD_TEXT,
        );

        // Top Right Engine Badge
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x + draw_w - 180.0, origin_y + 10.0),
            egui::vec2(170.0, 24.0),
        );
        ui.painter().rect_filled(
            badge_rect,
            4.0,
            egui::Color32::from_rgba_unmultiplied(20, 20, 20, 210),
        );
        ui.painter()
            .rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0_f32, backend_color));
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            backend_text,
            egui::FontId::proportional(11.0),
            backend_color,
        );
    }

    // CPU-only Feature Notice
    if rendered_gpu {
        let comp_now = app.history.current().active_composition();
        let mut reasons: Vec<String> = Vec::new();

        let animated_text = comp_now
            .layers
            .iter()
            .filter(|l| l.is_active(current_frame))
            .filter(|l| l.text_animator.as_ref().map(|a| a.enabled).unwrap_or(false))
            .count();
        if animated_text > 0 {
            reasons.push(format!("text animator ×{}", animated_text));
        }

        let styled = comp_now
            .layers
            .iter()
            .filter(|l| l.is_active(current_frame))
            .filter(|l| {
                l.style.drop_shadow.enabled
                    || l.style.outer_glow.enabled
                    || l.style.inner_glow.enabled
                    || l.style.satin.enabled
            })
            .count();
        if styled > 0 {
            reasons.push(format!("layer styles ×{}", styled));
        }

        if comp_now.resolve_camera().dof_enabled
            && comp_now
                .layers
                .iter()
                .any(|l| l.is_active(current_frame) && l.is_3d)
        {
            reasons.push("3D DOF".to_string());
        }

        if !reasons.is_empty() {
            let warn_text = format!("⚠ {} → export only", reasons.join(", "));
            let warn_w = (warn_text.len() as f32 * 6.2 + 20.0).min(draw_w - 20.0);
            let warn_rect = egui::Rect::from_min_size(
                egui::pos2(origin_x + draw_w - warn_w - 10.0, origin_y + 40.0),
                egui::vec2(warn_w, 22.0),
            );
            ui.painter().rect_filled(
                warn_rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(30, 24, 8, 210),
            );
            ui.painter().rect_stroke(
                warn_rect,
                4.0,
                egui::Stroke::new(1.0_f32, colors::ACCENT_ORANGE),
            );
            ui.painter().text(
                warn_rect.center(),
                egui::Align2::CENTER_CENTER,
                &warn_text,
                egui::FontId::proportional(10.5),
                colors::ACCENT_ORANGE,
            );
        }
    }

    // Motion Sketch Recording Badge
    if app.motion_sketch_active {
        let ms_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x + 10.0, origin_y + 40.0),
            egui::vec2(180.0, 22.0),
        );
        ui.painter().rect_filled(
            ms_rect,
            4.0,
            egui::Color32::from_rgba_premultiplied(50, 10, 10, 220),
        );
        ui.painter()
            .rect_stroke(ms_rect, 4.0, egui::Stroke::new(1.0_f32, colors::ACCENT_RED));
        ui.painter().text(
            ms_rect.center(),
            egui::Align2::CENTER_CENTER,
            "● MOTION SKETCH",
            egui::FontId::proportional(11.0),
            colors::ACCENT_RED,
        );
    }

    // Bottom Left Selection HUD Status
    if app.viewport_drag_state.is_some() {
        if let Some(s_idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if s_idx < comp.layers.len() {
                let s_layer = &comp.layers[s_idx];
                let pos = s_layer.transform.position.evaluate(current_frame);
                let scale = s_layer.transform.scale.evaluate(current_frame);
                let rot = s_layer.transform.rotation.evaluate(current_frame);

                let status_str = format!(
                    "SELECTED: Layer {} ({}) | Pos: ({:.0}, {:.0}) | Scale: {:.0}% | Rot: {:.1}°",
                    s_idx + 1,
                    s_layer.name,
                    pos[0],
                    pos[1],
                    scale[0],
                    rot
                );

                let status_rect = egui::Rect::from_min_size(
                    egui::pos2(origin_x + 10.0, origin_y + draw_h - 34.0),
                    egui::vec2(status_str.len() as f32 * 6.8 + 20.0, 24.0),
                );
                ui.painter().rect_filled(status_rect, 4.0, colors::HUD_BG);
                ui.painter().rect_stroke(
                    status_rect,
                    4.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_ACTIVE),
                );
                ui.painter().text(
                    status_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    status_str,
                    egui::FontId::proportional(11.0),
                    colors::HUD_STATUS_TEXT,
                );
            }
        }
    }
}
