use crate::ui::kui;
use crate::KagariApp;
use eframe::egui;

pub fn draw_content(app: &mut KagariApp, ui: &mut egui::Ui) {
    let volume = app.playback.master_volume;
    let left_peak = (app.audio_meter.0 * volume).clamp(0.0, 1.0);
    let right_peak = (app.audio_meter.1 * volume).clamp(0.0, 1.0);

    kui::group(ui, "MASTER VU", |ui| {
        ui.horizontal(|ui| {
            kui::vertical_meter(ui, "L", left_peak, 18.0, 174.0);
            kui::vertical_meter(ui, "R", right_peak, 18.0, 174.0);
        });

        kui::caption(ui, "32-Band FFT Spectrum");
        let spectrum_size = egui::vec2(ui.available_width(), 48.0);
        let (rect, _) = ui.allocate_exact_size(spectrum_size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 3.0, crate::ui::theme::colors::BG_DEEPEST);

        let bands = app.audio_spectrum_bands.len();
        if bands > 0 {
            let gap = 1.0;
            let bar_width = ((rect.width() - gap * (bands - 1) as f32) / bands as f32).max(1.0);
            for index in 0..bands {
                let amplitude = (app.audio_spectrum_bands[index] * volume).clamp(0.0, 1.0);
                let bar_height = (amplitude * rect.height()).max(1.0);
                let color = if amplitude > 0.85 {
                    crate::ui::theme::colors::ACCENT_RED
                } else if amplitude > 0.6 {
                    crate::ui::theme::colors::ACCENT_YELLOW
                } else {
                    crate::ui::theme::colors::ACCENT_GREEN
                };
                let bar = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + index as f32 * (bar_width + gap),
                        rect.bottom() - bar_height,
                    ),
                    egui::vec2(bar_width, bar_height),
                );
                ui.painter().rect_filled(bar, 1.0, color);
            }
        }
    });

    kui::group(ui, "Master", |ui| {
        kui::parameter_slider(ui, &mut app.playback.master_volume, 0.0..=1.5, "×");
        kui::caption(ui, format!("{:.0}%", app.playback.master_volume * 100.0));
    });
}

#[allow(dead_code)]
pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    egui::SidePanel::right("audio_meter_panel")
        .default_width(85.0)
        .resizable(false)
        .show(ctx, |ui| draw_content(app, ui));
}
