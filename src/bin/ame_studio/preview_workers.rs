use super::*;

fn headless_wgpu_renderer(
) -> &'static std::sync::Mutex<Option<kagari_vfx::core::renderer::Renderer>> {
    static RENDERER: std::sync::OnceLock<
        std::sync::Mutex<Option<kagari_vfx::core::renderer::Renderer>>,
    > = std::sync::OnceLock::new();
    RENDERER.get_or_init(|| {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kagari_preview_worker_gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            },
        ))
        .ok()?;
        Some(kagari_vfx::core::renderer::Renderer::new(
            std::sync::Arc::new(device),
            std::sync::Arc::new(queue),
        ))
    })
    .into()
}

fn render_preview_pixels_gpu_or_cpu(
    composition: &kagari_vfx::core::timeline::Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<u8> {
    if let Ok(mut guard) = headless_wgpu_renderer().lock() {
        if let Some(renderer) = guard.as_mut() {
            if let Some(pixels) = renderer.render_frame_to_rgba8_gpu(
                composition,
                frame,
                width,
                height,
                exposure_ev,
                lut_mode,
            ) {
                return pixels;
            }
        }
    }
    kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        composition,
        frame,
        width,
        height,
        exposure_ev,
        lut_mode,
    )
}

pub(crate) type WaveformDecodeRequest = (String, u64);
pub(crate) type WaveformDecodeResult = (String, u64, Option<CachedAudioWaveform>);
const MAX_CACHED_AUDIO_WAVEFORMS: usize = 128;

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
    let pixels = render_preview_pixels_gpu_or_cpu(
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
        .as_chunks::<4>()
        .0
        .iter()
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
    let pixels = render_preview_pixels_gpu_or_cpu(
        composition,
        0,
        width,
        height,
        0.0,
        0,
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

fn render_viewport_preview(job: &ViewportPreviewJob) -> Option<std::sync::Arc<Vec<u8>>> {
    let expected = (job.key.width as usize)
        .saturating_mul(job.key.height as usize)
        .saturating_mul(4);
    let cancel = job.cancel.clone();
    let rendered =
        kagari_vfx::core::software_renderer::with_render_cancel_flag(cancel.clone(), || {
            render_preview_pixels_gpu_or_cpu(
                &job.composition,
                job.key.frame,
                job.key.width,
                job.key.height,
                0.0,
                0,
            )
        });
    (!cancel.load(std::sync::atomic::Ordering::Relaxed) && rendered.len() == expected)
        .then(|| std::sync::Arc::new(rendered))
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
                let pixels = render_viewport_preview(&job);
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
    let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<(String, AssetPreviewJob)>(2);
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
    std::sync::mpsc::SyncSender<WaveformDecodeRequest>,
    std::sync::mpsc::Receiver<WaveformDecodeResult>,
) {
    let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<WaveformDecodeRequest>(16);
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("ame-audio-waveform".to_owned())
        .spawn(move || {
            while let Ok((path, revision)) = request_rx.recv() {
                let waveform = kagari_vfx::core::audio_engine::AudioBuffer::load_audio(
                    std::path::Path::new(&path),
                )
                .ok()
                .map(|buffer| CachedAudioWaveform {
                    duration_seconds: buffer.samples.len() as f32
                        / (buffer.sample_rate.max(1) as f32 * buffer.channels.max(1) as f32),
                    peaks: buffer.waveform_peaks(1200),
                });
                if result_tx.send((path, revision, waveform)).is_err() {
                    break;
                }
            }
        });
    (request_tx, result_rx)
}

impl AfterEffectsStudio {
    fn audio_waveform_file_revision(path: &str) -> u64 {
        kagari_vfx::core::media_revision::media_file_revision(std::path::Path::new(path))
    }

    fn invalidate_stale_audio_waveform(&mut self, path: &str, revision: u64) {
        if self
            .waveform_cache_revisions
            .get(path)
            .is_some_and(|cached_revision| *cached_revision != revision)
        {
            self.waveform_cache.remove(path);
            self.waveform_cache_revisions.remove(path);
            self.waveform_cache_last_used.remove(path);
        }
        if self
            .waveform_failed
            .get(path)
            .is_some_and(|failed_revision| *failed_revision != revision)
        {
            self.waveform_failed.remove(path);
        }
    }

    fn touch_audio_waveform(&mut self, path: &str) {
        self.waveform_cache_use_clock = self.waveform_cache_use_clock.saturating_add(1);
        self.waveform_cache_last_used
            .insert(path.to_owned(), self.waveform_cache_use_clock);
    }

    fn evict_least_recent_audio_waveform(&mut self) {
        let Some(path) = self
            .waveform_cache
            .keys()
            .min_by_key(|path| {
                self.waveform_cache_last_used
                    .get(*path)
                    .copied()
                    .unwrap_or(0)
            })
            .cloned()
        else {
            return;
        };
        self.waveform_cache.remove(&path);
        self.waveform_cache_revisions.remove(&path);
        self.waveform_cache_last_used.remove(&path);
    }

    fn record_audio_waveform_failure(&mut self, path: String, revision: u64) {
        const MAX_FAILED_WAVEFORM_PATHS: usize = 128;
        if self.waveform_failed.len() >= MAX_FAILED_WAVEFORM_PATHS
            && !self.waveform_failed.contains_key(&path)
        {
            if let Some(evicted) = self.waveform_failed.keys().next().cloned() {
                self.waveform_failed.remove(&evicted);
            }
        }
        self.waveform_failed.insert(path, revision);
    }

    fn request_audio_waveform(&mut self, path: &str, revision: u64) -> bool {
        match self
            .waveform_request_tx
            .try_send((path.to_owned(), revision))
        {
            Ok(()) => {
                self.waveform_requested.insert(path.to_owned(), revision);
                false
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => false,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                self.waveform_worker_failed = true;
                self.waveform_requested.clear();
                self.record_audio_waveform_failure(path.to_owned(), revision);
                true
            }
        }
    }

    pub(crate) fn update_audio_waveforms(
        &mut self,
        layers: &[kagari_vfx::core::timeline::Layer],
    ) -> bool {
        let mut active_paths = layers
            .iter()
            .filter_map(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Audio { path, .. }
                | kagari_vfx::core::timeline::LayerType::Video {
                    audio_wav: Some(path),
                    ..
                } => Some(path.clone()),
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>();
        if let Some(path) = self
            .selected_project_asset_id
            .as_deref()
            .and_then(|id| self.project_model.assets.iter().find(|item| item.id == id))
            .and_then(|item| match &item.item_type {
                kagari_vfx::core::timeline::ProjectItemType::Audio { path, .. } => {
                    Some(path.clone())
                }
                _ => None,
            })
        {
            active_paths.insert(path);
        }
        for path in active_paths {
            if self.waveform_cache.contains_key(&path) {
                self.touch_audio_waveform(&path);
            }
        }
        let mut changed = false;
        loop {
            match self.waveform_result_rx.try_recv() {
                Ok((path, revision, waveform)) => {
                    changed = true;
                    if self.waveform_requested.get(&path) == Some(&revision) {
                        self.waveform_requested.remove(&path);
                    }
                    if Self::audio_waveform_file_revision(&path) != revision {
                        continue;
                    }
                    if let Some(waveform) = waveform {
                        if !self.waveform_cache.contains_key(&path)
                            && self.waveform_cache.len() >= MAX_CACHED_AUDIO_WAVEFORMS
                        {
                            self.evict_least_recent_audio_waveform();
                        }
                        self.waveform_cache
                            .insert(path.clone(), std::sync::Arc::new(waveform));
                        self.waveform_cache_revisions.insert(path.clone(), revision);
                        self.touch_audio_waveform(&path);
                        self.waveform_failed.remove(&path);
                    } else {
                        self.record_audio_waveform_failure(path, revision);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if !self.waveform_worker_failed {
                        self.waveform_worker_failed = true;
                        self.waveform_requested.clear();
                        changed = true;
                    }
                    break;
                }
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
            let revision = Self::audio_waveform_file_revision(path);
            self.invalidate_stale_audio_waveform(path, revision);
            if self.waveform_cache.contains_key(path)
                || self.waveform_requested.get(path) == Some(&revision)
                || self.waveform_failed.get(path) == Some(&revision)
                || self.waveform_worker_failed
            {
                continue;
            }
            changed |= self.request_audio_waveform(path, revision);
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
            let revision = Self::audio_waveform_file_revision(&path);
            self.invalidate_stale_audio_waveform(&path, revision);
            if !self.waveform_cache.contains_key(&path)
                && self.waveform_requested.get(&path) != Some(&revision)
                && self.waveform_failed.get(&path) != Some(&revision)
                && !self.waveform_worker_failed
            {
                changed |= self.request_audio_waveform(&path, revision);
            }
        }
        changed
    }

    pub(crate) fn update_asset_preview_results(&mut self) -> bool {
        let mut received_result = false;
        loop {
            match self.asset_render_preview_result_rx.try_recv() {
                Ok((key, preview)) => {
                    received_result = true;
                    self.asset_render_preview_requested.remove(&key);
                    if self.asset_render_preview_cache.len() >= 32
                        && !self.asset_render_preview_cache.contains_key(&key)
                    {
                        if let Some(evicted) =
                            self.asset_render_preview_cache.keys().next().cloned()
                        {
                            self.asset_render_preview_cache.remove(&evicted);
                        }
                    }
                    self.asset_render_preview_cache.insert(key, preview);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if !self.asset_preview_worker_failed {
                        self.asset_preview_worker_failed = true;
                        for key in self.asset_render_preview_requested.drain() {
                            self.asset_render_preview_cache.insert(key, None);
                        }
                        for key in self.asset_render_preview_pending.keys() {
                            self.asset_render_preview_cache.insert(key.clone(), None);
                        }
                        self.asset_render_preview_pending.clear();
                        received_result = true;
                    }
                    break;
                }
            }
        }
        if self.asset_preview_worker_failed {
            return received_result;
        }
        let pending = self
            .asset_render_preview_pending
            .iter()
            .take(2)
            .map(|(key, job)| (key.clone(), job.clone()))
            .collect::<Vec<_>>();
        for (key, job) in pending {
            match self
                .asset_render_preview_request_tx
                .try_send((key.clone(), job))
            {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(_)) => break,
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                    self.asset_preview_worker_failed = true;
                    for key in self.asset_render_preview_requested.drain() {
                        self.asset_render_preview_cache.insert(key, None);
                    }
                    for key in self.asset_render_preview_pending.keys() {
                        self.asset_render_preview_cache.insert(key.clone(), None);
                    }
                    self.asset_render_preview_pending.clear();
                    return true;
                }
            }
            self.asset_render_preview_pending.remove(&key);
            self.asset_render_preview_requested.insert(key);
        }
        received_result
    }
}

#[cfg(test)]
mod viewport_preview_worker_tests {
    use super::*;

fn headless_wgpu_renderer(
) -> &'static std::sync::Mutex<Option<kagari_vfx::core::renderer::Renderer>> {
    static RENDERER: std::sync::OnceLock<
        std::sync::Mutex<Option<kagari_vfx::core::renderer::Renderer>>,
    > = std::sync::OnceLock::new();
    RENDERER.get_or_init(|| {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kagari_preview_worker_gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            },
        ))
        .ok()?;
        Some(kagari_vfx::core::renderer::Renderer::new(
            std::sync::Arc::new(device),
            std::sync::Arc::new(queue),
        ))
    })
    .into()
}

fn render_preview_pixels_gpu_or_cpu(
    composition: &kagari_vfx::core::timeline::Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<u8> {
    if let Ok(mut guard) = headless_wgpu_renderer().lock() {
        if let Some(renderer) = guard.as_mut() {
            if let Some(pixels) = renderer.render_frame_to_rgba8_gpu(
                composition,
                frame,
                width,
                height,
                exposure_ev,
                lut_mode,
            ) {
                return pixels;
            }
        }
    }
    kagari_vfx::core::software_renderer::render_frame_to_pixels_preview(
        composition,
        frame,
        width,
        height,
        exposure_ev,
        lut_mode,
    )
}

    #[test]
    fn cancelled_preview_never_returns_a_partial_frame() {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let job = ViewportPreviewJob {
            key: ViewportPreviewKey {
                revision: 1,
                media_revision: 0,
                frame: 0,
                width: 8,
                height: 8,
            },
            composition: Box::new(kagari_vfx::core::timeline::Composition::new(
                "cancelled_preview".into(),
                "Cancelled Preview".into(),
                8,
                8,
                24,
                1,
            )),
            cancel,
        };

        assert!(render_viewport_preview(&job).is_none());
    }
}
