use super::*;

pub(crate) struct AudioMixFileGuard(Option<std::path::PathBuf>);

impl AudioMixFileGuard {
    pub(crate) fn new(path: Option<std::path::PathBuf>) -> Self {
        Self(path)
    }
}

impl Drop for AudioMixFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            if let Err(error) = std::fs::remove_file(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!(
                        "could not remove temporary audio mix {}: {error}",
                        path.display()
                    );
                }
            }
        }
    }
}

fn write_text_file_atomically(path: &std::path::Path, text: &str) -> Result<(), String> {
    kagari_vfx::core::atomic_file::write_atomic(path, text.as_bytes())
        .map_err(|error| format!("書き出しファイルを確定できません: {error}"))
}

pub(crate) fn find_existing_sequence_frame(
    directory: &std::path::Path,
    prefix: &str,
    extension: &str,
) -> Result<Option<std::path::PathBuf>, std::io::Error> {
    let file_prefix = format!("{prefix}_");
    let file_suffix = format!(".{extension}");
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(frame_text) = file_name
            .strip_prefix(&file_prefix)
            .and_then(|name| name.strip_suffix(&file_suffix))
        else {
            continue;
        };
        let Ok(frame) = frame_text.parse::<u32>() else {
            continue;
        };
        if frame_text == format!("{frame:05}") {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

pub(crate) fn sequence_prefix(project_name: &str) -> String {
    let prefix = project_name
        .trim()
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    if prefix.trim().is_empty() {
        "frame".to_owned()
    } else {
        prefix
    }
}

pub(crate) fn write_png_file_new(
    path: &std::path::Path,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<kagari_vfx::core::ffmpeg_export::FileIdentity, String> {
    write_png_file(path, pixels, width, height, false, None)
}

pub(crate) fn write_png_file_overwriting(
    path: &std::path::Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    expected_identity: Option<kagari_vfx::core::ffmpeg_export::FileIdentity>,
) -> Result<(), String> {
    write_png_file(path, pixels, width, height, true, expected_identity).map(|_| ())
}

fn write_png_file(
    path: &std::path::Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    overwrite: bool,
    expected_identity: Option<kagari_vfx::core::ffmpeg_export::FileIdentity>,
) -> Result<kagari_vfx::core::ffmpeg_export::FileIdentity, String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let file_name = path
        .file_name()
        .ok_or_else(|| "PNG output path must include a filename".to_owned())?
        .to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut temporary = None;
    #[cfg(unix)]
    let mut original_permissions = None;
    #[cfg(unix)]
    let mut temporary_identity = None;
    let mut owns_temporary = false;
    let result = (|| {
        let mut staged = None;
        for _ in 0..100 {
            let candidate = parent.join(format!(
                ".{file_name}.kagari-tmp-{}-{}",
                std::process::id(),
                TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => {
                    temporary = Some(candidate.clone());
                    owns_temporary = true;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        original_permissions = Some(
                            file.metadata()
                                .map_err(|error| {
                                    format!("failed to inspect temporary PNG permissions: {error}")
                                })?
                                .permissions(),
                        );
                        let identity = kagari_vfx::core::ffmpeg_export::types::file_identity(&file)
                            .map_err(|error| {
                                format!("failed to identify temporary PNG: {error}")
                            })?;
                        temporary_identity = Some(identity);
                        let mut private_permissions = original_permissions
                            .as_ref()
                            .expect("temporary PNG permissions were just captured")
                            .clone();
                        private_permissions.set_mode(0o600);
                        file.set_permissions(private_permissions)
                            .map_err(|error| format!("failed to secure temporary PNG: {error}"))?;
                    }
                    staged = Some(file);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("failed to create temporary PNG: {error}"));
                }
            }
        }
        let Some(file) = staged else {
            return Err("could not reserve a unique temporary PNG after 100 attempts".to_owned());
        };
        let temporary_path = temporary
            .as_deref()
            .expect("staged PNG file has a temporary path");
        let mut writer = std::io::BufWriter::new(file);
        image::write_buffer_with_format(
            &mut writer,
            pixels,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|error| error.to_string())?;
        writer
            .flush()
            .map_err(|error| format!("failed to flush temporary PNG: {error}"))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| format!("failed to sync temporary PNG: {error}"))?;
        drop(writer);

        #[cfg(unix)]
        {
            let file = std::fs::File::open(temporary_path)
                .map_err(|error| format!("failed to verify temporary PNG: {error}"))?;
            if Some(
                kagari_vfx::core::ffmpeg_export::types::file_identity(&file).map_err(|error| {
                    format!("failed to identify temporary PNG before publish: {error}")
                })?,
            ) != temporary_identity
            {
                return Err("temporary PNG changed before publication".to_owned());
            }
            file.set_permissions(
                original_permissions
                    .take()
                    .expect("temporary PNG permissions were captured"),
            )
            .map_err(|error| format!("failed to restore PNG output permissions: {error}"))?;
        }

        let published_identity = if overwrite {
            if let Some(expected) = expected_identity {
                let current =
                    kagari_vfx::core::ffmpeg_export::path_file_identity(path).map_err(|error| {
                        format!("failed to verify PNG output before replacement: {error}")
                    })?;
                if current != expected {
                    return Err("PNG destination changed after overwrite confirmation; leaving it untouched".to_owned());
                }
                kagari_vfx::core::atomic_file::replace_file_with_identity(temporary_path, path)
                    .map_err(|error| format!("failed to replace PNG output: {error}"))?
            } else {
                kagari_vfx::core::ffmpeg_export::publish_file_no_replace(temporary_path, path)
                    .map_err(|error| {
                        format!("failed to publish PNG output without overwriting: {error}")
                    })?
            }
        } else {
            kagari_vfx::core::ffmpeg_export::publish_file_no_replace(temporary_path, path).map_err(
                |error| format!("failed to create PNG output without overwriting: {error}"),
            )?
        };
        if std::fs::remove_file(temporary_path).is_ok() {
            owns_temporary = false;
        }
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(published_identity)
    })();

    match result {
        Ok(identity) => Ok(identity),
        Err(error) if owns_temporary => match std::fs::remove_file(
            temporary
                .as_deref()
                .expect("owned temporary PNG has a path"),
        ) {
            Ok(()) => Err(error),
            Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => {
                Err(error)
            }
            Err(cleanup_error) => Err(format!(
                "{error}; failed to remove temporary PNG {}: {cleanup_error}",
                temporary
                    .as_deref()
                    .expect("owned temporary PNG has a path")
                    .display()
            )),
        },
        Err(error) => Err(error),
    }
}

const SEQUENCE_OWNERSHIP_CHUNK_SIZE: usize = 4096;
static SEQUENCE_OWNERSHIP_FILE_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

pub(crate) struct SequenceFileOwnership {
    manifest: Option<std::fs::File>,
    manifest_path: std::path::PathBuf,
    manifest_count: usize,
    recent: Vec<kagari_vfx::core::ffmpeg_export::FileIdentity>,
    directory: std::path::PathBuf,
    prefix: String,
    extension: String,
    rollback_attempted: bool,
    committed: bool,
}

impl SequenceFileOwnership {
    pub(crate) fn new(
        directory: &std::path::Path,
        prefix: &str,
        extension: &str,
    ) -> std::io::Result<Self> {
        use std::sync::atomic::Ordering;

        let manifest_directory = std::env::temp_dir();
        for _ in 0..100 {
            let sequence = SEQUENCE_OWNERSHIP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = manifest_directory.join(format!(
                "kagari-sequence-ownership-{}-{sequence}.tmp",
                std::process::id()
            ));
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(manifest) => {
                    return Ok(Self {
                        manifest: Some(manifest),
                        manifest_path: path,
                        manifest_count: 0,
                        recent: Vec::with_capacity(SEQUENCE_OWNERSHIP_CHUNK_SIZE),
                        directory: directory.to_path_buf(),
                        prefix: prefix.to_owned(),
                        extension: extension.to_owned(),
                        rollback_attempted: false,
                        committed: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique sequence ownership manifest",
        ))
    }

    pub(crate) fn record(
        &mut self,
        identity: kagari_vfx::core::ffmpeg_export::FileIdentity,
    ) -> std::io::Result<()> {
        self.recent.push(identity);
        if self.recent.len() == SEQUENCE_OWNERSHIP_CHUNK_SIZE {
            self.flush_recent()?;
        }
        Ok(())
    }

    fn flush_recent(&mut self) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};

        let record_size = kagari_vfx::core::ffmpeg_export::FileIdentity::RECORD_SIZE;
        let manifest = self
            .manifest
            .as_mut()
            .expect("manifest remains open until drop");
        let start = manifest.seek(SeekFrom::End(0))?;
        let mut records = Vec::with_capacity(self.recent.len() * record_size);
        for identity in &self.recent {
            records.extend_from_slice(&identity.to_record());
        }
        if let Err(error) = manifest.write_all(&records) {
            let _ = manifest.set_len(start);
            let _ = manifest.seek(SeekFrom::End(0));
            return Err(error);
        }
        self.manifest_count += self.recent.len();
        self.recent.clear();
        Ok(())
    }

    pub(crate) fn cleanup(&mut self) -> Option<String> {
        use std::io::{Read, Seek, SeekFrom};

        if self.rollback_attempted || self.committed {
            return None;
        }
        self.rollback_attempted = true;
        let mut first_failure = None;
        let manifest = self
            .manifest
            .as_mut()
            .expect("manifest remains open until drop");
        for (index, identity) in self.recent.iter().enumerate().rev() {
            let sequence_index = self.manifest_count + index;
            remove_owned_sequence_frame(
                &self.directory,
                &self.prefix,
                &self.extension,
                sequence_index,
                *identity,
                &mut first_failure,
            );
        }

        let record_size = kagari_vfx::core::ffmpeg_export::FileIdentity::RECORD_SIZE;
        let mut sequence_end = self.manifest_count;
        while sequence_end > 0 {
            let sequence_start = sequence_end.saturating_sub(SEQUENCE_OWNERSHIP_CHUNK_SIZE);
            let record_count = sequence_end - sequence_start;
            let offset = (sequence_start * record_size) as u64;
            if let Err(error) = manifest.seek(SeekFrom::Start(offset)) {
                first_failure.get_or_insert_with(|| {
                    format!("sequence ownership manifest seek failed: {error}")
                });
                break;
            }
            let mut records = vec![0; record_count * record_size];
            if let Err(error) = manifest.read_exact(&mut records) {
                first_failure.get_or_insert_with(|| {
                    format!("sequence ownership manifest read failed: {error}")
                });
                break;
            }
            for record_index in (0..record_count).rev() {
                let start = record_index * record_size;
                let record = records[start..start + record_size]
                    .try_into()
                    .expect("fixed identity record");
                remove_owned_sequence_frame(
                    &self.directory,
                    &self.prefix,
                    &self.extension,
                    sequence_start + record_index,
                    kagari_vfx::core::ffmpeg_export::FileIdentity::from_record(record),
                    &mut first_failure,
                );
            }
            sequence_end = sequence_start;
        }
        first_failure
    }

    pub(crate) fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for SequenceFileOwnership {
    fn drop(&mut self) {
        if let Some(error) = self.cleanup() {
            log::error!("failed to roll back incomplete image sequence export: {error}");
        }
        drop(self.manifest.take());
        if let Err(error) = std::fs::remove_file(&self.manifest_path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "could not remove sequence ownership manifest {}: {error}",
                    self.manifest_path.display()
                );
            }
        }
    }
}

fn remove_owned_sequence_frame(
    directory: &std::path::Path,
    prefix: &str,
    extension: &str,
    index: usize,
    expected_identity: kagari_vfx::core::ffmpeg_export::FileIdentity,
    first_failure: &mut Option<String>,
) {
    let path = directory.join(format!("{prefix}_{:05}.{extension}", index + 1));
    match kagari_vfx::core::atomic_file::remove_file_if_identity(&path, expected_identity) {
        Ok(true) => {}
        Ok(false) if std::fs::symlink_metadata(&path).is_ok() => {
            first_failure.get_or_insert_with(|| {
                format!(
                    "{} が別のファイルに置き換わったため削除しませんでした",
                    path.display()
                )
            });
        }
        Ok(false) => {}
        Err(error) => {
            first_failure.get_or_insert_with(|| format!("{}: {error}", path.display()));
        }
    }
}

impl AfterEffectsStudio {
    pub(crate) fn export_in_progress(&self) -> bool {
        self.render_events.is_some()
            || self.still_export_events.is_some()
            || self.render_worker.is_some()
            || self.still_export_worker.is_some()
    }
}
impl AfterEffectsStudio {
    pub(crate) fn export_mogrt(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Kagari Motion Template", &["kgtpl"])
            .set_file_name(format!(
                "{}.kgtpl",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        let project_json = match serde_json::to_string(&self.project_model) {
            Ok(json) => json,
            Err(error) => {
                self.render_status = format!("プロジェクトを保存できませんでした: {error}");
                return;
            }
        };
        let composition = self.project_model.active_composition();
        let properties = composition
            .layers
            .iter()
            .filter_map(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Text { text, .. } => {
                    Some(kagari_vfx::core::mogrt_engine::EssentialProperty {
                        id: layer.id.clone(),
                        name: layer.name.clone(),
                        comment: Some("テキストレイヤーの内容".to_owned()),
                        target_layer_id: layer.id.clone(),
                        target_property_path: "text.content".to_owned(),
                        property_type:
                            kagari_vfx::core::mogrt_engine::EssentialPropertyType::Text {
                                text: text.clone(),
                            },
                    })
                }
                _ => None,
            })
            .collect();
        let name = self.project_name.trim().to_owned();
        let author = std::env::var("USER").unwrap_or_else(|_| "Kagari".to_owned());
        let manifest = kagari_vfx::core::mogrt_engine::create_mogrt_manifest(
            composition,
            name.clone(),
            author,
            properties,
        );
        match kagari_vfx::core::mogrt_engine::MogrtPackage::new(manifest, project_json).to_json() {
            Ok(json) => match write_text_file_atomically(&path, &json) {
                Ok(()) => {
                    self.render_status =
                        format!("Kagari テンプレートを保存しました: {}", path.display())
                }
                Err(error) => {
                    self.render_status =
                        format!("Kagari テンプレートを保存できませんでした: {error}")
                }
            },
            Err(error) => {
                self.render_status = format!("Kagari テンプレートを生成できませんでした: {error}")
            }
        }
    }

    pub(crate) fn export_lottie(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Lottie Animation", &["json"])
            .set_file_name(format!(
                "{}.json",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        match kagari_vfx::core::lottie_exporter::export_project_to_json(&self.project_model) {
            Ok(json) => match write_text_file_atomically(&path, &json) {
                Ok(()) => self.render_status = format!("Lottie を保存しました: {}", path.display()),
                Err(error) => {
                    self.render_status = format!("Lottie を保存できませんでした: {error}")
                }
            },
            Err(error) => self.render_status = format!("Lottie を書き出せません: {error}"),
        }
    }

    pub(crate) fn export_mlt(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MLT XML", &["mlt"])
            .set_file_name(format!(
                "{}.mlt",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        let composition = self.project_model.active_composition();
        match kagari_vfx::core::mlt_export::MltExporter::export_to_xml(composition) {
            Ok(xml) => match write_text_file_atomically(&path, &xml) {
                Ok(()) => self.render_status = format!("MLT を保存しました: {}", path.display()),
                Err(error) => self.render_status = format!("MLT を保存できませんでした: {error}"),
            },
            Err(error) => self.render_status = format!("MLT を書き出せません: {error}"),
        }
    }

    pub(crate) fn export_subtitles(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SubRip 字幕", &["srt"])
            .set_file_name(format!(
                "{}.srt",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        let composition = self.project_model.active_composition();
        let srt =
            kagari_vfx::core::subtitles::layers_to_srt(&composition.layers, composition.fps.max(1));
        let export_path = if path.extension().is_some_and(|extension| extension == "vtt") {
            kagari_vfx::core::subtitles::layers_to_vtt(&composition.layers, composition.fps.max(1))
        } else {
            srt
        };
        match write_text_file_atomically(&path, &export_path) {
            Ok(()) => self.render_status = format!("字幕を保存しました: {}", path.display()),
            Err(error) => self.render_status = format!("字幕を保存できませんでした: {error}"),
        }
    }

    pub(crate) fn export_otio(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenTimelineIO", &["otio", "json"])
            .set_file_name(format!(
                "{}.otio",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        let composition = self.project_model.active_composition();
        let timeline = kagari_vfx::core::integration::OtioTimeline::from_composition(composition);
        match timeline.to_json() {
            Ok(json) => match write_text_file_atomically(&path, &json) {
                Ok(()) => self.render_status = format!("OTIO を保存しました: {}", path.display()),
                Err(error) => self.render_status = format!("OTIO を保存できませんでした: {error}"),
            },
            Err(error) => self.render_status = format!("OTIO を生成できませんでした: {error}"),
        }
    }

    pub(crate) fn export_gif(&mut self) {
        if self.export_in_progress() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("GIF Animation", &["gif"])
            .set_file_name(format!(
                "{}.gif",
                self.project_name.trim().replace(['/', '\\', ':'], "_")
            ))
            .save_file()
        else {
            return;
        };
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("gif"))
        {
            self.render_status = "GIF の拡張子は .gif にしてください。".to_owned();
            return;
        }
        if path.exists() {
            self.render_status = format!(
                "同名ファイルがあります: {}。別の保存先を選んでください。",
                path.display()
            );
            return;
        }
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
        let config = kagari_vfx::core::ffmpeg_export::ExportConfig {
            output_path: path.to_string_lossy().into_owned(),
            width: composition.width.max(1),
            height: composition.height.max(1),
            fps: composition.fps.max(1),
            total_frames: composition.duration_frames.max(1),
            quality: 85,
            audio_bitrate_kbps: 192,
            audio_wav: None,
            codec: kagari_vfx::core::ffmpeg_export::VideoCodec::Gif,
        };
        let width = config.width;
        let height = config.height;
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let spawn_result = std::thread::Builder::new()
            .name("kagari-gif-export".to_owned())
            .spawn(move || {
                let (progress_sender, progress_receiver) = std::sync::mpsc::channel();
                let render_composition = composition.clone();
                let result =
                    kagari_vfx::core::ffmpeg_export::start_gif_export_with_overwrite_tracked(
                        config,
                        progress_sender,
                        worker_cancel.clone(),
                        false,
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
                let worker = match result {
                    Ok(worker) => worker,
                    Err(error) => {
                        let _ =
                            sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(error));
                        return;
                    }
                };
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
                if worker.join().is_err() {
                    let _ = sender.send(kagari_vfx::core::ffmpeg_export::ExportEvent::Error(
                        "GIF書き出しworkerが異常終了しました。出力を確認してください。".to_owned(),
                    ));
                }
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(error) => {
                self.render_status = format!("GIF 書き出しを開始できませんでした: {error}");
                return;
            }
        };
        self.video_output_path = path.to_string_lossy().into_owned();
        self.render_progress = 0;
        self.render_status = "GIF を書き出しています…".to_owned();
        self.render_events = Some(receiver);
        self.render_cancel = Some(cancel);
        self.render_worker = Some(worker);
        self.render_refresh_ticks = 0;
    }
}
