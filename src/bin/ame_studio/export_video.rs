use super::ame_studio_export::AudioMixFileGuard;
use super::*;

fn store_video_export_terminal_event(
    slot: &std::sync::Arc<std::sync::Mutex<Option<kagari_vfx::core::ffmpeg_export::ExportEvent>>>,
    event: kagari_vfx::core::ffmpeg_export::ExportEvent,
) {
    if let Ok(mut pending) = slot.lock() {
        *pending = Some(event);
    }
}

impl AfterEffectsStudio {
    pub(crate) fn start_video_export(&mut self, overwrite_confirmed: bool) {
        if self.export_in_progress() {
            return;
        }
        if self.video_output_path.trim().is_empty() {
            self.render_status = "動画の保存先を指定してください。".to_owned();
            return;
        }
        let mut path = expand_home_path(self.video_output_path.trim());
        let codec_index = self.video_export_codec_idx % 4;
        if matches!(codec_index, 2 | 3) {
            if !path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mov"))
            {
                path.set_extension("mov");
            }
        } else if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("mp4" | "mov" | "m4v")
        ) {
            path.set_extension("mp4");
        }
        if path.exists() && !overwrite_confirmed {
            self.video_overwrite_identity_pending =
                match kagari_vfx::core::ffmpeg_export::snapshot_overwrite_identity(&path, true) {
                    Ok(identity) => Some(identity),
                    Err(error) => {
                        self.render_status = format!("上書き先を確認できません: {error}");
                        return;
                    }
                };
            self.video_overwrite_pending = true;
            self.render_status = format!("同名ファイルがあります: {}", path.display());
            return;
        }
        self.video_overwrite_pending = false;
        let overwrite_identity = if overwrite_confirmed {
            match self.video_overwrite_identity_pending.take() {
                Some(identity) => identity,
                None => {
                    self.render_status =
                        "上書き確認が無効になりました。保存先を再確認してください。".to_owned();
                    return;
                }
            }
        } else {
            self.video_overwrite_identity_pending = None;
            None
        };
        let composition = self.project_model.active_composition().clone();
        let has_audio = self.video_export_audio_enabled
            && kagari_vfx::core::audio_engine::composition_contains_audio_sources(&composition);
        if matches!(codec_index, 0 | 1)
            && (!composition.width.is_multiple_of(2) || !composition.height.is_multiple_of(2))
        {
            self.render_status = "H.264 / H.265 の書き出しには幅と高さの偶数指定が必要です。コンポジションサイズを調整するか、ProResを選択してください。".to_owned();
            return;
        }
        let frame_bytes = u64::from(composition.width.max(1))
            .checked_mul(u64::from(composition.height.max(1)))
            .and_then(|pixels| pixels.checked_mul(4));
        if frame_bytes.is_none_or(|bytes| bytes > 256 * 1024 * 1024) {
            self.render_status =
                "1フレームが256 MiBを超えるため書き出せません。コンポジションの解像度を下げてください。"
                    .to_owned();
            return;
        }
        let config = kagari_vfx::core::ffmpeg_export::ExportConfig {
            output_path: path.to_string_lossy().into_owned(),
            width: composition.width.max(1),
            height: composition.height.max(1),
            fps: composition.fps.max(1),
            total_frames: composition.duration_frames.max(1),
            quality: self.video_export_quality,
            audio_bitrate_kbps: 192,
            audio_wav: None,
            codec: match codec_index {
                0 => kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
                1 => kagari_vfx::core::ffmpeg_export::VideoCodec::H265,
                2 => kagari_vfx::core::ffmpeg_export::VideoCodec::ProRes422,
                _ => kagari_vfx::core::ffmpeg_export::VideoCodec::ProRes4444,
            },
        };
        let width = config.width;
        let height = config.height;
        let (sender, receiver) = std::sync::mpsc::sync_channel(4);
        let terminal_event = std::sync::Arc::new(std::sync::Mutex::new(None));
        let worker_terminal_event = terminal_event.clone();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let spawn_result = std::thread::Builder::new()
            .name("kagari-video-export".to_owned())
            .spawn(move || {
                let audio_wav = if has_audio {
                    let master_dsp = kagari_vfx::core::audio_engine::MasterDspParams::default();
                    match kagari_vfx::core::audio_engine::mix_composition_to_wav_cancelable(
                        &composition,
                        0,
                        config.total_frames,
                        48_000,
                        None,
                        &master_dsp,
                        &worker_cancel,
                    ) {
                        Ok(audio) => audio,
                        Err(error) => {
                            let message = if worker_cancel.load(std::sync::atomic::Ordering::SeqCst)
                            {
                                "Export canceled by user".to_owned()
                            } else {
                                format!("音声を合成できませんでした: {error}")
                            };
                            store_video_export_terminal_event(
                                &worker_terminal_event,
                                kagari_vfx::core::ffmpeg_export::ExportEvent::Error(message),
                            );
                            return;
                        }
                    }
                } else {
                    None
                };
                let _audio_wav_guard = AudioMixFileGuard::new(audio_wav.clone());
                if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    store_video_export_terminal_event(
                        &worker_terminal_event,
                        kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                            "Export canceled by user".to_owned(),
                        ),
                    );
                    return;
                }
                let mut config = config;
                config.audio_wav = audio_wav
                    .map(|audio_path| audio_path.to_string_lossy().into_owned());
                let (progress_sender, progress_receiver) = std::sync::mpsc::channel();
                let render_composition = composition.clone();
                let result =
                    kagari_vfx::core::ffmpeg_export::start_export_cancelable_with_expected_identity_tracked(
                        config,
                        progress_sender,
                        worker_cancel.clone(),
                        overwrite_identity,
                        move |frame| {
                            render_export_pixels_gpu_or_cpu(&render_composition, frame, width, height)
                        },
                    );
                let worker = match result {
                    Ok(worker) => worker,
                    Err(error) => {
                        store_video_export_terminal_event(
                            &worker_terminal_event,
                            kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error),
                        );
                        return;
                    }
                };
                for event in progress_receiver {
                    let terminal = matches!(
                        &event,
                        kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(_)
                            | kagari_vfx::core::ffmpeg_export::ExportEvent::Error(_)
                    );
                    if terminal {
                        store_video_export_terminal_event(&worker_terminal_event, event);
                    } else {
                        // Progress is replaceable; keep a slow UI from retaining
                        // one allocation per encoded frame in an unbounded queue.
                        let _ = sender.try_send(event);
                    }
                    if terminal {
                        break;
                    }
                }
                if worker.join().is_err() {
                    store_video_export_terminal_event(
                        &worker_terminal_event,
                        kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                            "動画書き出しworkerが異常終了しました。出力を確認してください。"
                                .to_owned(),
                        ),
                    );
                }
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(error) => {
                self.render_status = format!("動画の書き出しを開始できませんでした: {error}");
                return;
            }
        };
        self.video_output_path = path.to_string_lossy().into_owned();
        self.render_progress = 0;
        self.render_status = if has_audio {
            "音声を準備し、動画を書き出しています…".to_owned()
        } else {
            "動画を書き出しています…".to_owned()
        };
        self.render_events = Some(receiver);
        self.render_terminal_event = Some(terminal_event);
        self.render_cancel = Some(cancel);
        self.render_worker = Some(worker);
        self.render_refresh_ticks = 0;
    }

    pub(crate) fn cancel_video_export(&mut self) {
        if let Some(cancel) = &self.render_cancel {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            self.render_status = "書き出しをキャンセルしています…".to_owned();
        }
    }

    pub(crate) fn receive_video_export_events(&mut self) -> bool {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.render_events {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if let Some(slot) = &self.render_terminal_event {
            if let Ok(mut pending) = slot.lock() {
                if let Some(event) = pending.take() {
                    events.push(event);
                }
            }
        }
        let had_events = !events.is_empty();
        let mut finished = disconnected;
        let mut received_terminal_event = false;
        for event in events {
            match event {
                kagari_vfx::core::ffmpeg_export::ExportEvent::Progress(progress, _detail) => {
                    self.render_progress =
                        (f64::from(progress).clamp(0.0, 1.0) * 100.0).round() as i64;
                    self.render_status = format!("書き出し中… {}%", self.render_progress);
                }
                kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(_detail) => {
                    received_terminal_event = true;
                    self.render_progress = 100;
                    self.render_status =
                        format!("書き出しが完了しました: {}", self.video_output_path);
                    finished = true;
                }
                kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error) => {
                    received_terminal_event = true;
                    self.render_status = if error.to_ascii_lowercase().contains("cancel")
                        || error.contains("キャンセル")
                    {
                        error
                    } else {
                        format!("書き出せませんでした: {error}")
                    };
                    finished = true;
                }
            }
        }
        if disconnected && !received_terminal_event {
            self.render_status =
                "書き出し処理との接続が終了しました。もう一度お試しください。".to_owned();
        }
        if finished {
            self.render_events = None;
            self.render_terminal_event = None;
            self.render_cancel = None;
            self.render_refresh_ticks = 2;
        }
        had_events || disconnected
    }

    pub(crate) fn reap_finished_export_workers(&mut self) -> bool {
        let mut reaped = false;
        if self
            .render_worker
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            reaped = true;
            if self
                .render_worker
                .take()
                .is_some_and(|worker| worker.join().is_err())
            {
                self.render_status =
                    "書き出しworkerが異常終了しました。出力を確認してください。".to_owned();
                self.render_events = None;
                self.render_terminal_event = None;
                self.render_cancel = None;
            }
        }
        if self
            .still_export_worker
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            reaped = true;
            if self
                .still_export_worker
                .take()
                .is_some_and(|worker| worker.join().is_err())
            {
                self.still_status =
                    "PNG書き出しworkerが異常終了しました。出力を確認してください。".to_owned();
                self.still_export_events = None;
                self.still_export_cancel = None;
            }
        }
        reaped
    }
}
