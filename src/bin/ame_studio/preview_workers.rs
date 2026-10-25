use super::*;

pub(crate) fn render_model_preview_pixels(path: &str) -> Option<CachedAssetPreview> {
    use kagari_vfx::core::timeline::{Composition, Layer, LayerType};
    const WIDTH: u32 = 480;
    const HEIGHT: u32 = 270;
    let source = std::path::Path::new(path);
    let metadata = std::fs::metadata(source).ok()?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 {
        return None;
    }
    let mut composition = Composition::new(
        "asset_preview".to_owned(),
        "3D Model Preview".to_owned(),
        WIDTH,
        HEIGHT,
        30,
        1,
    );
    let mut layer = Layer::new(
        "model_preview".to_owned(),
        "3D Model".to_owned(),
        LayerType::Model3D {
            path: path.to_owned(),
        },
        1,
    );
    layer.is_3d = true;
    composition.layers.push(layer);
    let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        &composition,
        0,
        WIDTH,
        HEIGHT,
        0.0,
        0,
    );
    if pixels.len() != WIDTH as usize * HEIGHT as usize * 4 {
        return None;
    }
    let background = &pixels[..4];
    let visible = pixels
        .as_chunks::<4>().0.iter()
        .any(|pixel| pixel != background);
    visible.then_some(CachedAssetPreview {
        pixels: std::sync::Arc::new(pixels),
        width: WIDTH,
        height: HEIGHT,
    })
}

pub(crate) fn render_composition_preview(
    composition: &kagari_vfx::core::timeline::Composition,
) -> Option<CachedAssetPreview> {
    let scale = (640.0 / f64::from(composition.width.max(1)))
        .min(360.0 / f64::from(composition.height.max(1)))
        .min(1.0);
    let width = (f64::from(composition.width.max(1)) * scale)
        .round()
        .max(1.0) as u32;
    let height = (f64::from(composition.height.max(1)) * scale)
        .round()
        .max(1.0) as u32;
    let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        composition, 0, width, height, 0.0, 0,
    );
    if pixels.len() != width as usize * height as usize * 4 {
        return None;
    }
    Some(CachedAssetPreview {
        pixels: std::sync::Arc::new(pixels),
        width,
        height,
    })
}

pub(crate) fn spawn_viewport_preview_renderer() -> (
    std::sync::mpsc::SyncSender<ViewportPreviewJob>,
    std::sync::mpsc::Receiver<ViewportPreviewResult>,
) {
    let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<ViewportPreviewJob>(1);
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("ame-composition-preview".to_owned())
        .spawn(move || {
            while let Ok(job) = request_rx.recv() {
                let expected = (job.key.width as usize)
                    .saturating_mul(job.key.height as usize)
                    .saturating_mul(4);
                let pixels = kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
                    &job.composition,
                    job.key.frame,
                    job.key.width,
                    job.key.height,
                    0.0,
                    0,
                );
                let pixels = (pixels.len() == expected)
                    .then(|| std::sync::Arc::new(pixels));
                if result_tx
                    .send(ViewportPreviewResult {
                        key: job.key,
                        pixels,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
    (request_tx, result_rx)
}

#[allow(clippy::type_complexity)]
pub(crate) fn spawn_asset_preview_renderer() -> (
    std::sync::mpsc::SyncSender<(String, AssetPreviewJob)>,
    std::sync::mpsc::Receiver<(String, Option<CachedAssetPreview>)>,
) {
    let (request_tx, request_rx) =
        std::sync::mpsc::sync_channel::<(String, AssetPreviewJob)>(2);
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("ame-asset-preview".to_owned())
        .spawn(move || {
            let mut image_cache = kagari_vfx::core::image_cache::ImageCache::new();
            image_cache.set_max_bytes(128 * 1024 * 1024);
            while let Ok((key, job)) = request_rx.recv() {
                let preview = match job {
                    AssetPreviewJob::Image(path) => {
                        super::render_asset_image_preview_pixels(&mut image_cache, &path)
                    }
                    AssetPreviewJob::Model3D(path) => render_model_preview_pixels(&path),
                    AssetPreviewJob::Composition(composition) => {
                        render_composition_preview(&composition)
                    }
                };
                if result_tx.send((key, preview)).is_err() {
                    break;
                }
            }
        });
    (request_tx, result_rx)
}

pub(crate) fn spawn_waveform_decoder() -> (
    std::sync::mpsc::SyncSender<String>,
    std::sync::mpsc::Receiver<(String, Option<CachedAudioWaveform>)>,
) {
    let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<String>(16);
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("ame-audio-waveform".to_owned())
        .spawn(move || {
            while let Ok(path) = request_rx.recv() {
                let waveform = kagari_vfx::core::audio_engine::AudioBuffer::load_audio(
                    std::path::Path::new(&path),
                )
                .ok()
                .map(|buffer| CachedAudioWaveform {
                    duration_seconds: buffer.samples.len() as f32
                        / (buffer.sample_rate.max(1) as f32 * buffer.channels.max(1) as f32),
                    peaks: buffer.waveform_peaks(1200),
                });
                if result_tx.send((path, waveform)).is_err() {
                    break;
                }
            }
        });
    (request_tx, result_rx)
}

impl AfterEffectsStudio {
    pub(crate) fn update_audio_waveforms(&mut self, layers: &[kagari_vfx::core::timeline::Layer]) {
        while let Ok((path, waveform)) = self.waveform_result_rx.try_recv() {
            self.waveform_requested.remove(&path);
            if let Some(waveform) = waveform {
                if self.waveform_cache.len() < 128 || self.waveform_cache.contains_key(&path) {
                    self.waveform_cache
                        .insert(path, std::sync::Arc::new(waveform));
                }
            } else {
                self.waveform_failed.insert(path);
            }
        }

        for layer in layers {
            let path = match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Audio { path, .. } => Some(path),
                kagari_vfx::core::timeline::LayerType::Video {
                    audio_wav: Some(path),
                    ..
                } => Some(path),
                _ => None,
            };
            let Some(path) = path else { continue };
            if self.waveform_cache.contains_key(path)
                || self.waveform_requested.contains(path)
                || self.waveform_failed.contains(path)
                || self.waveform_cache.len() >= 128
            {
                continue;
            }
            if self.waveform_request_tx.try_send(path.clone()).is_ok() {
                self.waveform_requested.insert(path.clone());
            }
        }

        let selected_audio_path = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .and_then(|item| match &item.item_type {
                kagari_vfx::core::timeline::ProjectItemType::Audio { path, .. } => {
                    Some(path.clone())
                }
                _ => None,
            });
        if let Some(path) = selected_audio_path {
            if !self.waveform_cache.contains_key(&path)
                && !self.waveform_requested.contains(&path)
                && !self.waveform_failed.contains(&path)
                && self.waveform_cache.len() < 128
                && self.waveform_request_tx.try_send(path.clone()).is_ok()
            {
                self.waveform_requested.insert(path);
            }
        }
    }

    pub(crate) fn update_asset_preview_results(&mut self) -> bool {
        let mut received_result = false;
        while let Ok((key, preview)) = self.asset_render_preview_result_rx.try_recv() {
            received_result = true;
            self.asset_render_preview_requested.remove(&key);
            if self.asset_render_preview_cache.len() >= 32
                && !self.asset_render_preview_cache.contains_key(&key)
            {
                if let Some(evicted) = self.asset_render_preview_cache.keys().next().cloned() {
                    self.asset_render_preview_cache.remove(&evicted);
                }
            }
            self.asset_render_preview_cache.insert(key, preview);
        }
        let pending = self
            .asset_render_preview_pending
            .iter()
            .take(2)
            .map(|(key, job)| (key.clone(), job.clone()))
            .collect::<Vec<_>>();
        for (key, job) in pending {
            if self
                .asset_render_preview_request_tx
                .try_send((key.clone(), job))
                .is_err()
            {
                break;
            }
            self.asset_render_preview_pending.remove(&key);
            self.asset_render_preview_requested.insert(key);
        }
        received_result
    }
}
