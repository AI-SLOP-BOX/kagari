use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;
use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// Apply a loaded snapshot (project + production document) to the app.
/// Returns true when the restore succeeded.
fn apply_restored_snapshot(
    app: &mut KagariApp,
    loaded: Option<(Project, Option<ProductionDocument>)>,
) -> bool {
    let Some((project, production_document)) = loaded else {
        app.toasts
            .error("復旧データを読み込めませんでした。スナップショットは保持しています。");
        return false;
    };
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
    true
}

/// Start loading one recovery snapshot on a background thread. The JSON parse
/// (bounded but potentially large) happens off the UI thread; the result comes
/// back through [`crate::app_state::RecoveryLoadResult`].
fn begin_recovery_load(
    app: &mut KagariApp,
    path: std::path::PathBuf,
    kind: crate::app_state::RecoveryLoadKind,
) {
    if app.recovery_load_rx.is_some() {
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    app.recovery_load_rx = Some(rx);
    app.recovery_load_kind = kind;
    let handle = match std::thread::Builder::new()
        .name("kagari-recovery-load".into())
        .spawn(move || {
            let loaded = crate::core::autosave::manager::load_snapshot_file(&path);
            let _ = tx.send(crate::app_state::RecoveryLoadResult { path, loaded });
        }) {
        Ok(handle) => handle,
        Err(e) => {
            log::error!("recovery load thread failed to spawn: {e}");
            app.recovery_load_rx = None;
            return;
        }
    };
    app.track_background_thread(handle);
}

/// Apply a finished background load. Returns true when the dialog should close.
///
/// `RecoveryLoadKind::Latest` mirrors the old "復元する" flow (restore, then
/// clear recovery). `RecoveryLoadKind::Generation` deliberately does **not**
/// call `clear_recovery`: restoring one version-history entry must leave the
/// remaining generations selectable instead of deleting them.
fn finish_recovery_load(app: &mut KagariApp, result: crate::app_state::RecoveryLoadResult) -> bool {
    let kind = app.recovery_load_kind;
    let Some((project, document)) = result.loaded else {
        // Same failure toast as a sync restore; recovery files are kept.
        apply_restored_snapshot(app, None);
        return false;
    };
    app.autosave.note_loaded_recovery(&result.path);
    if !apply_restored_snapshot(app, Some((project, document))) {
        return false;
    }
    match kind {
        crate::app_state::RecoveryLoadKind::Latest => {
            app.autosave.clear_recovery();
            true
        }
        crate::app_state::RecoveryLoadKind::Generation => {
            app.toasts
                .info("復元しました。履歴は保持されているので別の世代へ切り替えられます。");
            false
        }
    }
}

/// Startup crash-recovery prompt: offers to restore the latest autosave snapshot.
pub fn draw_recovery_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_recovery_dialog {
        return;
    }

    // Fetch the version-history list exactly once per dialog open. The list
    // reads and JSON-parses every snapshot; the background fetch runs off the
    // UI thread and delivers on a channel, so the dialog does not parse
    // snapshots while painting. The cached list is released on close.
    if app.recovery_generations.is_none() && app.recovery_generations_rx.is_none() {
        let (_tx, rx, handle) = app.autosave.fetch_generations_async();
        if let Some(h) = handle {
            app.track_background_thread(h);
        }
        app.recovery_generations_rx = Some(rx);
    }

    // Poll the background snapshot load (parse happens off the UI thread).
    let polled = app.recovery_load_rx.as_ref().map(|rx| rx.try_recv());
    let mut finished_load = None;
    match polled {
        Some(Ok(result)) => {
            app.recovery_load_rx = None;
            finished_load = Some(result);
        }
        Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
            app.recovery_load_rx = None;
            app.recovery_load_kind = crate::app_state::RecoveryLoadKind::Latest;
            apply_restored_snapshot(app, None);
        }
        Some(Err(std::sync::mpsc::TryRecvError::Empty)) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        None => {}
    }

    // Poll the background generation-list fetch (parse happens off the UI
    // thread), mirroring the snapshot-load poll above.
    if let Some(rx) = &app.recovery_generations_rx {
        match rx.try_recv() {
            Ok(generations) => {
                app.recovery_generations = Some(generations);
                app.recovery_generations_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                app.recovery_generations = None;
                app.recovery_generations_rx = None;
            }
        }
    }

    let mut open = app.show_recovery_dialog;
    let mut close_after_action = false;
    if let Some(result) = finished_load {
        if finish_recovery_load(app, result) {
            close_after_action = true;
        }
    }
    let restoring = app.recovery_load_rx.is_some();
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

            // Version history: pick any surviving generation instead of only the
            // newest. Rendered from the list cached at dialog open — no disk I/O
            // per frame — and a pick starts a background snapshot load.
            ui.add_space(8.0);
            egui::CollapsingHeader::new("以前のバージョンから復元…")
                .default_open(false)
                .show(ui, |ui| {
                    let mut pick: Option<std::path::PathBuf> = None;
                    if let Some(generations) = app.recovery_generations.as_ref() {
                        if generations.is_empty() {
                            ui.label("保存されたバージョンはありません。");
                        }
                        for generation in generations {
                            let stamp = format_unix_time(generation.saved_at_secs);
                            let origin = if generation.label.is_empty() {
                                "不明".to_string()
                            } else {
                                generation.label.clone()
                            };
                            if ui
                                .selectable_label(false, format!("{stamp}　({origin})"))
                                .on_hover_text(generation.path.to_string_lossy())
                                .clicked()
                            {
                                pick = Some(generation.path.clone());
                            }
                        }
                    }
                    if !restoring {
                        if let Some(path) = pick {
                            begin_recovery_load(
                                app,
                                path,
                                crate::app_state::RecoveryLoadKind::Generation,
                            );
                        }
                    }
                });

            ui.add_space(12.0);
            if restoring {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("復元データを読み込み中…");
                    if custom_widgets::ae_button(ui, "キャンセル").clicked() {
                        app.recovery_load_rx = None;
                        app.recovery_load_kind = crate::app_state::RecoveryLoadKind::Latest;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if custom_widgets::ae_button_accent(ui, "復元する").clicked() {
                        let newest = app
                            .recovery_generations
                            .as_ref()
                            .and_then(|generations| generations.first())
                            .map(|generation| generation.path.clone());
                        match newest {
                            Some(path) => begin_recovery_load(
                                app,
                                path,
                                crate::app_state::RecoveryLoadKind::Latest,
                            ),
                            None => {
                                apply_restored_snapshot(app, None);
                            }
                        }
                    }
                    if custom_widgets::ae_button(ui, "破棄して新規開始").clicked() {
                        // Record the newest generation's namespace so
                        // clear_recovery targets this recovery set — without
                        // re-parsing snapshot JSON on the UI thread.
                        if let Some(newest) = app
                            .recovery_generations
                            .as_ref()
                            .and_then(|generations| generations.first())
                        {
                            app.autosave.note_loaded_recovery(&newest.path);
                        }
                        app.autosave.clear_recovery();
                        close_after_action = true;
                    }
                });
            }
        });

    if close_after_action {
        open = false;
    }
    app.show_recovery_dialog = open;
    if !open {
        // Release per-open state: reopening refetches history, and an in-flight
        // load whose dialog was dismissed must not be applied later.
        app.recovery_generations = None;
        app.recovery_load_rx = None;
    }
}

/// Formats a Unix timestamp as `YYYY-MM-DD HH:MM` in UTC.
/// (Local-time formatting is intentionally avoided: no chrono dependency.)
fn format_unix_time(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let time = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year,
        m,
        d,
        time / 3_600,
        (time % 3_600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::format_unix_time;

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(format_unix_time(0), "1970-01-01 00:00");
        // 2000-03-01 00:00:00 UTC
        assert_eq!(format_unix_time(951_868_800), "2000-03-01 00:00");
        // Leap-day check: 2024-02-29 12:34 UTC
        assert_eq!(format_unix_time(1_709_210_040), "2024-02-29 12:34");
    }
}
