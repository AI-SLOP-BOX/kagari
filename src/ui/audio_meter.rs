use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_content(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("MASTER VU");
        ui.separator();

        let vol = app.playback.master_volume;
        let left_peak = (app.audio_meter.0 * vol).clamp(0.0, 1.0);
        let right_peak = (app.audio_meter.1 * vol).clamp(0.0, 1.0);

        let meter_height = 200.0;
        let meter_width = 16.0;

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Left Channel VU Bar
            draw_vu_channel(ui, "L", left_peak, meter_width, meter_height);
            ui.add_space(4.0);
            // Right Channel VU Bar
            draw_vu_channel(ui, "R", right_peak, meter_width, meter_height);
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("32-Band FFT Spectrum")
                .small()
                .strong()
                .color(colors::ACCENT_CYAN),
        );

        // 32-Band Live Equalizer Bars
        let spectrum_w = 180.0;
        let spectrum_h = 45.0;
        let (s_rect, _) =
            ui.allocate_exact_size(egui::vec2(spectrum_w, spectrum_h), egui::Sense::hover());
        ui.painter().rect_filled(s_rect, 2.0, colors::BG_DEEPEST);

        let bands = app.audio_spectrum_bands.len().max(1);
        let bar_w = (spectrum_w / bands as f32) - 1.0;

        for i in 0..bands {
            let amp = app.audio_spectrum_bands.get(i).copied().unwrap_or(0.0) * vol;
            let bar_h = (amp * spectrum_h).clamp(2.0, spectrum_h);
            let bx = s_rect.left() + i as f32 * (bar_w + 1.0);
            let by = s_rect.bottom() - bar_h;

            let bar_color = if amp > 0.85 {
                colors::ACCENT_RED
            } else if amp > 0.6 {
                colors::ACCENT_YELLOW
            } else {
                colors::ACCENT_GREEN
            };

            let b_rect =
                egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(bar_w.max(1.0), bar_h));
            ui.painter().rect_filled(b_rect, 0.5, bar_color);
        }

        ui.add_space(8.0);
        ui.separator();

        // Master Volume Slider
        ui.label(egui::RichText::new("Master").small().strong());
        ui.add(egui::Slider::new(&mut app.playback.master_volume, 0.0..=1.5).show_value(false));
        ui.small(format!("{:.0}%", app.playback.master_volume * 100.0));
    });
}

#[allow(dead_code)]
pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    egui::SidePanel::right("audio_meter_panel")
        .default_width(85.0)
        .resizable(false)
        .show(ctx, |ui| {
            draw_content(app, ui);
        });
}

fn draw_vu_channel(ui: &mut egui::Ui, label: &str, peak: f32, width: f32, height: f32) {
    ui.vertical_centered(|ui| {
        ui.small(label);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter();

        // 0dB Clip Warning Indicator Light
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, rect.top() - 12.0),
            egui::vec2(width - 4.0, 8.0),
        );
        let clip_color = if peak > 0.92 {
            colors::ACCENT_RED
        } else {
            colors::BG_DEEPEST
        };
        painter.rect_filled(clip_rect, 1.0, clip_color);
        painter.rect_stroke(
            clip_rect,
            1.0,
            egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
        );

        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));

        let segments = 24;
        let seg_gap = 1.5;
        let total_gap = seg_gap * (segments - 1) as f32;
        let seg_height = (height - total_gap - 4.0) / segments as f32;

        let active_segs = (peak * segments as f32).round() as usize;

        for i in 0..segments {
            let seg_idx_from_bottom = i;
            let ratio = seg_idx_from_bottom as f32 / segments as f32;

            let color = if ratio < 0.70 {
                colors::ACCENT_GREEN
            } else if ratio < 0.88 {
                colors::ACCENT_YELLOW
            } else {
                colors::ACCENT_RED
            };

            let seg_y_bottom = rect.bottom() - 2.0 - (i as f32 * (seg_height + seg_gap));
            let seg_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 2.0, seg_y_bottom - seg_height),
                egui::vec2(width - 4.0, seg_height),
            );

            if i < active_segs {
                painter.rect_filled(seg_rect, 1.0, color);
            } else {
                let dim_color = egui::Color32::from_rgba_unmultiplied(
                    color.r() / 5,
                    color.g() / 5,
                    color.b() / 5,
                    120,
                );
                painter.rect_filled(seg_rect, 1.0, dim_color);
            }
        }
    });
}
