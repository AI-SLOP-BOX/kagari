use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_lumetri_header_hud(app: &mut KagariApp, ui: &mut egui::Ui) {
    let histogram_pixels = {
        let comp = app.history.current().active_composition();
        let width = 128u32;
        let height = ((width * comp.height.max(1)) / comp.width.max(1)).clamp(24, 96);
        crate::core::software_renderer::render_frame_to_pixels(
            comp,
            app.playback.current_frame,
            width,
            height,
            0.0,
            0,
        )
    };
    let histogram = crate::core::color_correction::compute_luma_histogram(&histogram_pixels);
    let histogram_peak = histogram.iter().copied().max().unwrap_or(0).max(1) as f32;

    // ── Live 256-Bin Luma & RGB Histogram Analyzer HUD ──
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Live Luma Histogram")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Real-time Exposure & Waveform Monitor");
        });
        ui.separator();

        let histo_w = ui.available_width().max(200.0);
        let histo_h = 60.0;
        let (h_rect, _) =
            ui.allocate_exact_size(egui::vec2(histo_w, histo_h), egui::Sense::hover());
        ui.painter().rect_filled(h_rect, 2.0, colors::BG_DEEPEST);
        ui.painter().rect_stroke(
            h_rect,
            2.0,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );

        let bins = 64;
        let bin_w = histo_w / bins as f32;

        for i in 0..bins {
            let start = i * 256 / bins;
            let end = ((i + 1) * 256 / bins).max(start + 1).min(256);
            let sample_count: u32 = histogram[start..end].iter().sum();
            let luma_val = (sample_count as f32 / histogram_peak)
                .sqrt()
                .clamp(0.0, 1.0);
            let bar_h = luma_val * histo_h;
            let norm_x = i as f32 / bins as f32;

            let bx = h_rect.left() + i as f32 * bin_w;
            let by = h_rect.bottom() - bar_h;
            let b_rect =
                egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(bin_w.max(1.0), bar_h));

            let bar_color = if norm_x < 0.15 {
                colors::ACCENT_BLUE
            } else if norm_x > 0.85 {
                colors::ACCENT_YELLOW
            } else {
                colors::ACCENT_GREEN
            };
            ui.painter().rect_filled(b_rect, 0.0, bar_color);
        }

        ui.horizontal(|ui| {
            ui.small("Blacks [0]");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small("Whites [255]");
            });
        });
    });

    ui.add_space(4.0);
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Master Gradient Ramp Palette")
                .strong()
                .color(colors::ACCENT_CYAN),
        );
        ui.small("1-Tap Apply Trend Gradient Ramps:");
        let mut picked_ramp: Option<super::effects_io::LookPreset> = None;
        ui.horizontal(|ui| {
            if ui.button("Cyberpunk Pink/Cyan").clicked() {
                picked_ramp = Some((
                    "Cyberpunk Pink/Cyan",
                    [-20.0, 8.0, 28.0],
                    [18.0, -10.0, 24.0],
                    [30.0, -6.0, 18.0],
                ));
            }
            if ui.button("Sunset Gold").clicked() {
                picked_ramp = Some((
                    "Sunset Gold",
                    [28.0, 10.0, -18.0],
                    [16.0, 4.0, -8.0],
                    [-6.0, -2.0, 22.0],
                ));
            }
            if ui.button("Deep Ocean").clicked() {
                picked_ramp = Some((
                    "Deep Ocean",
                    [-18.0, 2.0, 30.0],
                    [-10.0, 0.0, 18.0],
                    [4.0, 8.0, 24.0],
                ));
            }
        });
        if let Some((name, shadows, midtones, highlights)) = picked_ramp {
            super::effects_io::write_cb(app, shadows, midtones, highlights, true);
            app.toasts.info(format!("Applied ramp '{name}'"));
        }
    });
}
