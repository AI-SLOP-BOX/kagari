use super::ame_studio_export::{
    find_existing_sequence_frame, sequence_prefix, write_png_file_new, write_png_file_overwriting,
    SequenceFileOwnership,
};
use super::*;

struct SequenceExportEvents {
    progress: std::sync::mpsc::SyncSender<kagari_vfx::core::ffmpeg_export::ExportEvent>,
    terminal:
        std::sync::Arc<std::sync::Mutex<Option<kagari_vfx::core::ffmpeg_export::ExportEvent>>>,
}

impl SequenceExportEvents {
    fn send(&self, event: kagari_vfx::core::ffmpeg_export::ExportEvent) -> Result<(), ()> {
        match event {
            event @ kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(_)
            | event @ kagari_vfx::core::ffmpeg_export::ExportEvent::Error(_) => {
                let mut pending = self.terminal.lock().map_err(|_| ())?;
                *pending = Some(event);
                Ok(())
            }
            progress => self.progress.try_send(progress).map_err(|_| ()),
        }
    }
}

fn new_sequence_ownership(
    sender: &SequenceExportEvents,
    directory: &std::path::Path,
    prefix: &str,
    extension: &str,
) -> Option<SequenceFileOwnership> {
    match SequenceFileOwnership::new(directory, prefix, extension) {
        Ok(ownership) => Some(ownership),
        Err(error) => {
            let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                format!("連番のロールバック記録を準備できません: {error}"),
            ));
            None
        }
    }
}

fn record_sequence_output(
    ownership: &mut SequenceFileOwnership,
    path: &std::path::Path,
    identity: kagari_vfx::core::ffmpeg_export::FileIdentity,
) -> Result<(), String> {
    ownership.record(identity).map_err(|error| {
        let cleanup_error = ownership.cleanup();
        cleanup_error.map_or_else(
            || format!("{} のロールバック記録に失敗しました: {error}。作成済みフレームを削除しました。", path.display()),
            |cleanup_error| format!("{} のロールバック記録に失敗しました: {error}。部分ファイルを削除できませんでした: {cleanup_error}", path.display()),
        )
    })
}

fn render_sequence_frame<T>(
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    render: impl FnOnce() -> T,
) -> Result<T, ()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        kagari_vfx::core::software_renderer::with_render_cancel_flag(cancel.clone(), render)
    }))
    .map_err(|_| ())
}

impl AfterEffectsStudio {
    pub(crate) fn export_png_sequence(&mut self) {
        if self.export_in_progress() {
            return;
        }
        let Some(directory) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let prefix = sequence_prefix(&self.project_name);
        let composition = self.project_model.active_composition().clone();
        let width = composition.width.max(1);
        let height = composition.height.max(1);
        let frame_count = composition.duration_frames.max(1);
        match find_existing_sequence_frame(&directory, &prefix, "png") {
            Ok(Some(path)) => {
                self.render_status = format!(
                    "{} が既にあります。連番内の既存ファイルを上書きしないため中止しました。",
                    path.display()
                );
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.render_status = format!("保存先を確認できませんでした: {error}");
                return;
            }
        }
        let frame_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4));
        if frame_bytes.is_none_or(|bytes| bytes > 256 * 1024 * 1024) {
            self.render_status =
                "1フレームが256 MiBを超えるため書き出せません。コンポジションの解像度を下げてください。"
                    .to_owned();
            return;
        }
        let (progress, receiver) = std::sync::mpsc::sync_channel(4);
        let terminal = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sender = SequenceExportEvents {
            progress,
            terminal: terminal.clone(),
        };
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let target = directory.clone();
        let spawn_result = std::thread::Builder::new()
            .name("kagari-png-sequence-export".to_owned())
            .spawn(move || {
            let Some(mut created_files) = new_sequence_ownership(&sender, &target, &prefix, "png") else {
                return;
            };
            for frame in 0..frame_count {
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        cleanup_error.map_or_else(
                            || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                            |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                        ),
                    ));
                    return;
                }
                let pixels = match render_sequence_frame(&worker_cancel, || {
                    kagari_vfx::core::software_renderer::render_frame_to_pixels(
                        &composition,
                        frame,
                        width,
                        height,
                        0.0,
                        0,
                    )
                }) {
                    Ok(pixels) => pixels,
                    Err(()) => {
                        let cleanup_error = created_files.cleanup();
                        let message = cleanup_error.map_or_else(
                            || format!("フレーム {} のレンダーが失敗しました。作成済みフレームを削除しました。", frame + 1),
                            |error| format!("フレーム {} のレンダーが失敗しました。部分ファイルを削除できませんでした: {error}", frame + 1),
                        );
                        let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(message));
                        return;
                    }
                };
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let message = cleanup_error.map_or_else(
                        || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                        |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                    );
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(message));
                    return;
                }
                let path = target.join(format!("{prefix}_{:05}.png", frame + 1));
                match write_png_file_new(&path, &pixels, width, height) {
                    Ok(identity) => {
                        if let Err(error) = record_sequence_output(
                            &mut created_files,
                            &path,
                            identity,
                        ) {
                            let _ = sender.send(
                                kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error),
                            );
                            return;
                        }
                    }
                    Err(error) => {
                        let cleanup_error = created_files.cleanup();
                        let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                            cleanup_error.map_or_else(
                                || format!("{} を書き出せませんでした: {error}。作成済みフレームを削除しました。", path.display()),
                                |cleanup_error| format!("{} を書き出せませんでした: {error}。部分ファイルを削除できませんでした: {cleanup_error}", path.display()),
                            ),
                        ));
                        return;
                    }
                }
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        cleanup_error.map_or_else(
                            || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                            |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                        ),
                    ));
                    return;
                }
                let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Progress(
                    (frame + 1) as f32 / frame_count as f32,
                    format!("{} / {}", frame + 1, frame_count),
                ));
            }
            created_files.commit();
            let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(
                format!("PNG 連番を書き出しました: {}", target.display()),
            ));
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(error) => {
                self.render_status = format!("PNG 連番の書き出しを開始できませんでした: {error}");
                return;
            }
        };
        self.video_output_path = directory.to_string_lossy().into_owned();
        self.render_progress = 0;
        self.render_status = "PNG 連番を書き出しています…".to_owned();
        self.render_events = Some(receiver);
        self.render_terminal_event = Some(terminal);
        self.render_cancel = Some(cancel);
        self.render_worker = Some(worker);
        self.render_refresh_ticks = 0;
    }

    pub(crate) fn export_exr_sequence(&mut self) {
        if self.export_in_progress() {
            return;
        }
        let Some(directory) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let prefix = sequence_prefix(&self.project_name);
        let composition = self.project_model.active_composition().clone();
        let width = composition.width.max(1);
        let height = composition.height.max(1);
        let frame_count = composition.duration_frames.max(1);
        match find_existing_sequence_frame(&directory, &prefix, "exr") {
            Ok(Some(path)) => {
                self.render_status = format!(
                    "{} が既にあります。連番内の既存ファイルを上書きしないため中止しました。",
                    path.display()
                );
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.render_status = format!("保存先を確認できませんでした: {error}");
                return;
            }
        }
        if width > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || height > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || kagari_vfx::core::software_renderer::rgba_buffer_size(width, height).is_none()
            || kagari_vfx::core::software_renderer::exr_rgba_working_set_size(width, height)
                .is_none()
        {
            self.render_status =
                "EXRの作業メモリが512 MiBを超えるか、レンダー可能な寸法ではありません。解像度を下げてください。"
                    .to_owned();
            return;
        }
        let (progress, receiver) = std::sync::mpsc::sync_channel(4);
        let terminal = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sender = SequenceExportEvents {
            progress,
            terminal: terminal.clone(),
        };
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let target = directory.clone();
        let spawn_result = std::thread::Builder::new()
            .name("kagari-exr-sequence-export".to_owned())
            .spawn(move || {
            let Some(mut created_files) = new_sequence_ownership(&sender, &target, &prefix, "exr") else {
                return;
            };
            for frame in 0..frame_count {
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        cleanup_error.map_or_else(
                            || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                            |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                        ),
                    ));
                    return;
                }
                let hdr = match render_sequence_frame(&worker_cancel, || {
                    kagari_vfx::core::software_renderer::render_frame_to_hdr_f32(
                        &composition,
                        frame,
                        width,
                        height,
                        0.0,
                        0,
                    )
                }) {
                    Ok(hdr) => hdr,
                    Err(()) => {
                        let cleanup_error = created_files.cleanup();
                        let message = cleanup_error.map_or_else(
                            || format!("フレーム {} のレンダーが失敗しました。作成済みフレームを削除しました。", frame + 1),
                            |error| format!("フレーム {} のレンダーが失敗しました。部分ファイルを削除できませんでした: {error}", frame + 1),
                        );
                        let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(message));
                        return;
                    }
                };
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let message = cleanup_error.map_or_else(
                        || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                        |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                    );
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(message));
                    return;
                }
                let image = match kagari_vfx::core::exr_writer::ExrImage::rgba_f32(
                    width, height, &hdr.data,
                ) {
                    Ok(image) => image,
                    Err(error) => {
                        let cleanup_error = created_files.cleanup();
                        let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                            cleanup_error.map_or_else(
                                || format!("EXRを書き出せません: {error}。作成済みフレームを削除しました。"),
                                |cleanup_error| format!("EXRを書き出せません: {error}。部分ファイルを削除できませんでした: {cleanup_error}"),
                            ),
                        ));
                        return;
                    }
                };
                let path = target.join(format!("{prefix}_{:05}.exr", frame + 1));
                match kagari_vfx::core::exr_writer::write_exr_file_new_with_identity(&path, &image)
                {
                    Ok(identity) => {
                        if let Err(error) = record_sequence_output(
                            &mut created_files,
                            &path,
                            identity,
                        ) {
                            let _ = sender.send(
                                kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error),
                            );
                            return;
                        }
                    }
                    Err(error) => {
                        let cleanup_error = created_files.cleanup();
                        let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                            cleanup_error.map_or_else(
                                || format!("EXR を書き出せませんでした: {error}。作成済みフレームを削除しました。"),
                                |cleanup_error| format!("EXR を書き出せませんでした: {error}。部分ファイルを削除できませんでした: {cleanup_error}"),
                            ),
                        ));
                        return;
                    }
                }
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    let cleanup_error = created_files.cleanup();
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        cleanup_error.map_or_else(
                            || "書き出しをキャンセルしました。作成済みフレームを削除しました。".to_owned(),
                            |error| format!("書き出しをキャンセルしました。部分ファイルを削除できませんでした: {error}"),
                        ),
                    ));
                    return;
                }
                let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Progress(
                    (frame + 1) as f32 / frame_count as f32,
                    format!("{} / {}", frame + 1, frame_count),
                ));
            }
            created_files.commit();
            let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(
                format!("EXR 連番を書き出しました: {}", target.display()),
            ));
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(error) => {
                self.render_status = format!("EXR 連番の書き出しを開始できませんでした: {error}");
                return;
            }
        };
        self.video_output_path = directory.to_string_lossy().into_owned();
        self.render_progress = 0;
        self.render_status = "EXR 連番を書き出しています…".to_owned();
        self.render_events = Some(receiver);
        self.render_terminal_event = Some(terminal);
        self.render_cancel = Some(cancel);
        self.render_worker = Some(worker);
        self.render_refresh_ticks = 0;
    }

    pub(crate) fn export_current_frame_png(&mut self, overwrite_confirmed: bool) {
        if self.export_in_progress() {
            self.still_status = "別の書き出しが完了してからPNGを保存してください。".to_owned();
            return;
        }
        let raw_path = self.output_path.trim();
        if raw_path.is_empty() {
            self.still_status = "先にPNGの保存先を選んでください。".to_owned();
            return;
        }
        let path = expand_home_path(raw_path);
        let path = if path.extension().is_some_and(|extension| extension != "png") {
            path.with_extension("png")
        } else {
            path
        };
        if path.exists() && !overwrite_confirmed {
            self.still_overwrite_identity_pending =
                match kagari_vfx::core::ffmpeg_export::snapshot_overwrite_identity(&path, true) {
                    Ok(identity) => Some(identity),
                    Err(error) => {
                        self.still_status = format!("PNGの上書き先を確認できません: {error}");
                        return;
                    }
                };
            self.still_overwrite_pending = true;
            self.still_status = format!(
                "{} は既にあります。上書きしますか？",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            return;
        }
        let overwrite_identity = if overwrite_confirmed {
            match self.still_overwrite_identity_pending.take() {
                Some(identity) => identity,
                None => {
                    self.still_overwrite_pending = false;
                    self.still_status =
                        "PNGの上書き確認が無効になりました。保存先を再確認してください。"
                            .to_owned();
                    return;
                }
            }
        } else {
            self.still_overwrite_pending = false;
            self.still_overwrite_identity_pending = None;
            None
        };
        if overwrite_confirmed {
            self.still_overwrite_pending = false;
        }
        let composition = self.project_model.active_composition().clone();
        let (width, height) = (composition.width.max(1), composition.height.max(1));
        let byte_len = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4));
        if byte_len.is_none_or(|bytes| bytes > 128 * 1024 * 1024) {
            self.still_status =
                "画像が大きすぎます。解像度を下げてください（上限128 MiB）。".to_owned();
            return;
        }
        let frame = self
            .current_frame
            .min(composition.duration_frames.saturating_sub(1));
        let expected_byte_len = byte_len.unwrap_or_default();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker_name = format!("kagari-still-png-{frame}");
        let spawn = std::thread::Builder::new()
            .name(worker_name)
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                        return Err("書き出しをキャンセルしました。".to_owned());
                    }
                    let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels(
                        &composition,
                        frame,
                        width,
                        height,
                        0.0,
                        0,
                    );
                    if pixels.len() as u64 != expected_byte_len {
                        return Err("フレームを描画できませんでした。".to_owned());
                    }
                    if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                        return Err("書き出しをキャンセルしました。".to_owned());
                    }
                    let write_result = if overwrite_confirmed {
                        write_png_file_overwriting(
                            &worker_path,
                            &pixels,
                            width,
                            height,
                            overwrite_identity,
                        )
                    } else {
                        write_png_file_new(&worker_path, &pixels, width, height).map(|_| ())
                    };
                    write_result.map(|()| worker_path)
                }))
                .unwrap_or_else(|_| Err("フレームの描画中にエラーが発生しました。".to_owned()));
                let _ = sender.send(result);
            });
        let worker = match spawn {
            Ok(worker) => worker,
            Err(error) => {
                self.still_status = format!("PNG書き出しを開始できませんでした: {error}");
                return;
            }
        };
        self.still_status = format!("PNGを書き出しています: {}", path.display());
        self.still_export_events = Some(receiver);
        self.still_export_cancel = Some(cancel);
        self.still_export_worker = Some(worker);
        self.still_overwrite_pending = false;
    }

    pub(crate) fn receive_still_export_result(&mut self) -> bool {
        let result = self
            .still_export_events
            .as_ref()
            .map(std::sync::mpsc::Receiver::try_recv);
        match result {
            Some(Ok(Ok(path))) => {
                self.still_status = format!("PNGを書き出しました: {}", path.display());
                self.still_export_events = None;
                self.still_export_cancel = None;
                true
            }
            Some(Ok(Err(error))) => {
                self.still_status = format!("PNGを書き出せませんでした: {error}");
                self.still_export_events = None;
                self.still_export_cancel = None;
                true
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.still_status = "PNG書き出し処理との接続が終了しました。".to_owned();
                self.still_export_events = None;
                self.still_export_cancel = None;
                true
            }
            Some(Err(std::sync::mpsc::TryRecvError::Empty)) | None => false,
        }
    }

    pub(crate) fn cycle_video_export_codec(&mut self) {
        if self.export_in_progress() {
            return;
        }
        self.video_export_codec_idx = (self.video_export_codec_idx + 1) % 4;
        let mut path = expand_home_path(self.video_output_path.trim());
        path.set_extension(if self.video_export_codec_idx >= 2 {
            "mov"
        } else {
            "mp4"
        });
        self.video_output_path = path.to_string_lossy().into_owned();
        self.video_overwrite_pending = false;
        self.video_overwrite_identity_pending = None;
        self.render_status.clear();
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::render_sequence_frame;

    #[test]
    fn render_scope_installs_and_restores_the_cancellation_flag() {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let previous = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        kagari_vfx::core::software_renderer::set_render_cancel_flag(Some(previous.clone()));

        let observed = render_sequence_frame(&flag, || {
            kagari_vfx::core::software_renderer::get_render_cancel_flag()
                .is_some_and(|current| std::sync::Arc::ptr_eq(&current, &flag))
        });

        assert_eq!(observed, Ok(true));
        assert!(
            kagari_vfx::core::software_renderer::get_render_cancel_flag()
                .is_some_and(|current| std::sync::Arc::ptr_eq(&current, &previous))
        );
        kagari_vfx::core::software_renderer::set_render_cancel_flag(None);
    }

    #[test]
    fn render_scope_restores_the_flag_after_a_panic() {
        let previous = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        kagari_vfx::core::software_renderer::set_render_cancel_flag(Some(previous.clone()));

        let result = render_sequence_frame::<()>(
            &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            || panic!("synthetic render panic"),
        );

        assert!(result.is_err());
        assert!(
            kagari_vfx::core::software_renderer::get_render_cancel_flag()
                .is_some_and(|current| std::sync::Arc::ptr_eq(&current, &previous))
        );
        kagari_vfx::core::software_renderer::set_render_cancel_flag(None);
    }
}
