use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// Startup crash-recovery prompt: offers to restore the latest autosave snapshot.
pub fn draw_recovery_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_recovery_dialog {
        return;
    }

    let mut open = app.show_recovery_dialog;
    let mut close_after_action = false;
    crate::ui::modal::dialog("Crash Recovery")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠")
                        .size(28.0)
                        .color(colors::ACCENT_WARNING),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("前回のセッションが異常終了した可能性があります")
                            .strong(),
                    );
                    ui.label("リカバリースナップショットが見つかりました。");
                });
            });

            ui.add_space(8.0);
            if let Some(ref recovered_at) = app.recovery_snapshot_time.clone() {
                ui.label(format!("スナップショット時刻: {}", recovered_at));
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "復元する").clicked() {
                    if let Some((project, production_document)) =
                        app.autosave.load_latest_snapshot()
                    {
                        app.replace_project_history(project);
                        let recovered_lut_path = production_document
                            .as_ref()
                            .and_then(|document| document.color_lut_path.clone());
                        app.production_document = production_document;
                        if let Some(path) = recovered_lut_path {
                            crate::ui::project_io::begin_color_lut_restore(app, path);
                        }
                        let recovered_audio = app.production_document.as_ref().map(|document| {
                            (
                                document.audio.correction,
                                document.audio.channels.clone(),
                                document.audio.master_gain.clamp(0.0, 1.0),
                            )
                        });
                        if let Some((correction, channels, master_gain)) = recovered_audio {
                            app.apply_audio_correction_settings(correction);
                            app.audio_mixer_channels = channels;
                            app.playback.master_volume = master_gain;
                        } else {
                            app.apply_audio_correction_settings(Default::default());
                            app.audio_mixer_channels.clear();
                            app.playback.master_volume = 0.8;
                        }
                        crate::core::frame_cache::bump_version();
                        app.frame_cache.collect_garbage();
                        app.autosave.clear_recovery();
                        close_after_action = true;
                    } else {
                        app.toasts.error(
                            "復旧データを読み込めませんでした。スナップショットは保持しています。",
                        );
                    }
                }
                if custom_widgets::ae_button(ui, "破棄して新規開始").clicked() {
                    let _ = app.autosave.load_latest_snapshot();
                    app.autosave.clear_recovery();
                    close_after_action = true;
                }
            });
        });

    if close_after_action {
        open = false;
    }
    app.show_recovery_dialog = open;
}
