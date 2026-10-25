use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_viewport_transport_bar(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    comp_fps: u32,
    comp_duration_frames: u32,
) {
    ui.add_space(2.0);
    let b_rect = ui.available_rect_before_wrap();
    let bar_h = 34.0;
    let trans_rect = egui::Rect::from_min_size(b_rect.min, egui::vec2(b_rect.width(), bar_h));
    ui.painter()
        .rect_filled(trans_rect, 0.0, colors::BG_DARKEST);
    ui.painter().line_segment(
        [trans_rect.left_top(), trans_rect.right_top()],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );

    let mut bottom_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(trans_rect.shrink2(egui::vec2(16.0, 3.0))));
    bottom_ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        let fps = comp_fps;
        let timecode_str = crate::ui::timeline::ruler::format_timecode_label(current_frame, fps);

        ui.label(
            egui::RichText::new(timecode_str)
                .size(13.0)
                .strong()
                .monospace()
                .color(colors::TEXT_PRIMARY),
        );

        let total_f = comp_duration_frames.max(1);
        let scrubber_w = 120.0_f32;
        let (scrub_rect, scrub_resp) =
            ui.allocate_exact_size(egui::vec2(scrubber_w, 16.0), egui::Sense::click_and_drag());
        let track_rect =
            egui::Rect::from_center_size(scrub_rect.center(), egui::vec2(scrubber_w, 4.0));
        ui.painter()
            .rect_filled(track_rect, 2.0, colors::BG_SURFACE);
        let progress = (current_frame as f32 / total_f as f32).clamp(0.0, 1.0);
        let filled_rect = egui::Rect::from_min_max(
            track_rect.left_top(),
            egui::pos2(
                track_rect.left() + track_rect.width() * progress,
                track_rect.bottom(),
            ),
        );
        ui.painter()
            .rect_filled(filled_rect, 2.0, colors::ACCENT_BRAND);
        let knob_pos = egui::pos2(
            track_rect.left() + track_rect.width() * progress,
            track_rect.center().y,
        );
        ui.painter()
            .circle_filled(knob_pos, 5.0, colors::ACCENT_BRAND);

        if scrub_resp.clicked() || scrub_resp.dragged() {
            if let Some(pos) = scrub_resp.interact_pointer_pos() {
                let t = ((pos.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
                app.playback.current_frame = (t * total_f as f32).round() as u32;
            }
        }

        ui.add_space(6.0);

        // Transport buttons: |<, <, Play/Pause, >, >|
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("⏮")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                )
                .frame(false),
            )
            .clicked()
        {
            app.playback.current_frame = 0;
        }
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("◀")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                )
                .frame(false),
            )
            .clicked()
        {
            app.playback.current_frame = current_frame.saturating_sub(1);
        }

        let play_label = if app.playback.is_playing {
            "⏸"
        } else {
            "▶"
        };
        let play_btn = ui.add(
            egui::Button::new(
                egui::RichText::new(play_label)
                    .size(15.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .frame(false),
        );
        if play_btn.clicked() {
            app.playback.is_playing = !app.playback.is_playing;
        }

        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("▶")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                )
                .frame(false),
            )
            .clicked()
        {
            app.playback.current_frame = (current_frame + 1).min(total_f);
        }
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("⏭")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                )
                .frame(false),
            )
            .clicked()
        {
            app.playback.current_frame = total_f;
        }

        // Right side: Total Timecode + Quality combo + Fullscreen icon
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (btn_rect, btn_resp) =
                ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
            if btn_resp.hovered() {
                ui.painter().rect_filled(btn_rect, 3.0, colors::BG_HOVER);
            }
            crate::ui::icons::render_svg_at(
                ui,
                "viewport-bottom-fullscreen".to_string(),
                crate::ui::icons::SVG_WINDOW_MAXIMIZE,
                egui::vec2(14.0, 14.0),
                colors::TEXT_SECONDARY,
                btn_rect.center() - egui::vec2(7.0, 7.0),
            );
            if btn_resp.clicked() {
                app.viewer_maximized = !app.viewer_maximized;
            }

            ui.add_space(4.0);

            egui::ComboBox::from_id_salt("viewport_transport_quality")
                .selected_text(match app.viewport_fast_preview {
                    0 => "フル",
                    1 => "適応",
                    2 => "ドラフト",
                    _ => "ワイヤー",
                })
                .width(62.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.viewport_fast_preview, 0, "フル");
                    ui.selectable_value(&mut app.viewport_fast_preview, 1, "適応");
                    ui.selectable_value(&mut app.viewport_fast_preview, 2, "ドラフト");
                    ui.selectable_value(&mut app.viewport_fast_preview, 3, "ワイヤー");
                });

            ui.add_space(8.0);

            let dur_timecode = crate::ui::timeline::ruler::format_timecode_label(total_f, fps);
            ui.label(
                egui::RichText::new(dur_timecode)
                    .size(12.5)
                    .monospace()
                    .color(colors::TEXT_MUTED),
            );
        });
    });
}
