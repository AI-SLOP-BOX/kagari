use super::*;

impl AfterEffectsStudio {
    pub(crate) fn export_current_frame_png(&mut self, overwrite_confirmed: bool) {
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
            self.still_overwrite_pending = true;
            self.still_status = format!(
                "{} は既にあります。上書きしますか？",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            return;
        }
        let composition = self.project_model.active_composition();
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
        let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels(
            composition,
            frame,
            width,
            height,
            0.0,
            0,
        );
        if pixels.len() as u64 != byte_len.unwrap_or_default() {
            self.still_status = "フレームを描画できませんでした。".to_owned();
            return;
        }
        let result = image::save_buffer_with_format(
            &path,
            &pixels,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        );
        self.still_status = match result {
            Ok(()) => format!("PNGを書き出しました: {}", path.display()),
            Err(error) => format!("PNGを書き出せませんでした: {error}"),
        };
        self.still_overwrite_pending = false;
    }

    pub(crate) fn cycle_video_export_codec(&mut self) {
        self.video_export_codec_idx = (self.video_export_codec_idx + 1) % 4;
        let mut path = expand_home_path(self.video_output_path.trim());
        path.set_extension(if self.video_export_codec_idx >= 2 {
            "mov"
        } else {
            "mp4"
        });
        self.video_output_path = path.to_string_lossy().into_owned();
        self.video_overwrite_pending = false;
        self.render_status.clear();
    }

    pub(crate) fn start_video_export(&mut self, overwrite_confirmed: bool) {
        if self.render_events.is_some() {
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
            self.video_overwrite_pending = true;
            self.render_status = format!("同名ファイルがあります: {}", path.display());
            return;
        }
        self.video_overwrite_pending = false;
        let composition = self.project_model.active_composition().clone();
        let frame_bytes = u64::from(composition.width.max(1))
            .checked_mul(u64::from(composition.height.max(1)))
            .and_then(|pixels| pixels.checked_mul(4));
        if frame_bytes.is_none_or(|bytes| bytes > 256 * 1024 * 1024) {
            self.render_status =
                "1フレームが256 MiBを超えるため書き出せません。コンポジションの解像度を下げてください。"
                    .to_owned();
            return;
        }
        if !kagari_vfx::core::ffmpeg_export::is_ffmpeg_available() {
            self.render_status =
                "FFmpegが見つかりません。FFmpegをインストールしてから再試行してください。"
                    .to_owned();
            return;
        }

        let config = kagari_vfx::core::ffmpeg_export::ExportConfig {
            output_path: path.to_string_lossy().into_owned(),
            width: composition.width.max(1),
            height: composition.height.max(1),
            fps: composition.fps.max(1),
            total_frames: composition.duration_frames.max(1),
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
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let master_dsp = kagari_vfx::core::audio_engine::MasterDspParams::default();
            let audio_wav = match kagari_vfx::core::audio_engine::mix_composition_to_wav(
                &composition,
                0,
                config.total_frames,
                48_000,
                None,
                &master_dsp,
            ) {
                Ok(audio) => audio,
                Err(error) => {
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        format!("音声を合成できませんでした: {error}"),
                    ));
                    return;
                }
            };
            if worker_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                if let Some(audio_path) = audio_wav {
                    let _ = std::fs::remove_file(audio_path);
                }
                let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                    "Export canceled by user".to_owned(),
                ));
                return;
            }
            let mut config = config;
            config.audio_wav = audio_wav
                .as_ref()
                .map(|audio_path| audio_path.to_string_lossy().into_owned());
            let (progress_sender, progress_receiver) = std::sync::mpsc::channel();
            let render_composition = composition.clone();
            let result = kagari_vfx::core::ffmpeg_export::start_export_cancelable(
                config,
                progress_sender,
                worker_cancel.clone(),
                move |frame| {
                    kagari_vfx::core::software_renderer::render_frame_to_pixels(
                        &render_composition,
                        frame,
                        width,
                        height,
                        0.0,
                        0,
                    )
                },
            );
            if let Err(error) = result {
                let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error));
            } else {
                for event in progress_receiver {
                    let terminal = matches!(
                        &event,
                        kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(_)
                            | kagari_vfx::core::ffmpeg_export::ExportEvent::Error(_)
                    );
                    let _ = sender.send(event);
                    if terminal {
                        break;
                    }
                }
            }
            if let Some(audio_path) = audio_wav {
                let _ = std::fs::remove_file(audio_path);
            }
        });
        self.video_output_path = path.to_string_lossy().into_owned();
        self.render_progress = 0;
        self.render_status = "音声を準備し、動画を書き出しています…".to_owned();
        self.render_events = Some(receiver);
        self.render_cancel = Some(cancel);
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
        let had_events = !events.is_empty();
        let mut finished = disconnected;
        for event in events {
            match event {
                kagari_vfx::core::ffmpeg_export::ExportEvent::Progress(progress, _detail) => {
                    self.render_progress =
                        (f64::from(progress).clamp(0.0, 1.0) * 100.0).round() as i64;
                    self.render_status =
                        format!("動画を書き出しています… {}%", self.render_progress);
                }
                kagari_vfx::core::ffmpeg_export::ExportEvent::Finished(_detail) => {
                    self.render_progress = 100;
                    self.render_status =
                        format!("書き出しが完了しました: {}", self.video_output_path);
                    finished = true;
                }
                kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error) => {
                    self.render_status = if error.to_ascii_lowercase().contains("cancel") {
                        "動画の書き出しをキャンセルしました。".to_owned()
                    } else {
                        format!("動画を書き出せませんでした: {error}")
                    };
                    finished = true;
                }
            }
        }
        if disconnected && !had_events {
            self.render_status =
                "書き出し処理との接続が終了しました。もう一度お試しください。".to_owned();
        }
        if finished {
            self.render_events = None;
            self.render_cancel = None;
            self.render_refresh_ticks = 2;
        }
        had_events || disconnected
    }
}
