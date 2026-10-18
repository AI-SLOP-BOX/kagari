use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct WaveformData {
    pub peaks: Vec<f32>,
    pub duration_seconds: f32,
}

struct WaveformCache {
    entries: HashMap<String, Arc<WaveformData>>,
    requested: HashSet<String>,
    failed: HashSet<String>,
    request_tx: std::sync::mpsc::SyncSender<String>,
    result_rx: std::sync::mpsc::Receiver<(String, Option<WaveformData>)>,
}

pub fn get(ctx: &egui::Context, path: &Path) -> Option<Arc<WaveformData>> {
    let path_key = path.to_string_lossy().into_owned();
    let cache_id = egui::Id::new("timeline_audio_waveforms");
    let cache = ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with(cache_id, || {
            let (request_tx, request_rx) = std::sync::mpsc::sync_channel::<String>(16);
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let repaint_ctx = ctx.clone();
            let _ = std::thread::Builder::new()
                .name("timeline-waveform-decode".into())
                .spawn(move || {
                    while let Ok(path) = request_rx.recv() {
                        let decoded =
                            crate::core::audio_engine::AudioBuffer::load_audio(Path::new(&path))
                                .ok()
                                .map(|buffer| {
                                    let duration_seconds = buffer.samples.len() as f32
                                        / (buffer.sample_rate.max(1) as f32
                                            * buffer.channels.max(1) as f32);
                                    WaveformData {
                                        peaks: buffer.waveform_peaks(1200),
                                        duration_seconds,
                                    }
                                });
                        if result_tx.send((path, decoded)).is_err() {
                            break;
                        }
                        repaint_ctx.request_repaint_after(std::time::Duration::from_millis(16));
                    }
                });
            Arc::new(Mutex::new(WaveformCache {
                entries: HashMap::new(),
                requested: HashSet::new(),
                failed: HashSet::new(),
                request_tx,
                result_rx,
            }))
        })
        .clone()
    });

    let Ok(mut cache) = cache.lock() else {
        return None;
    };
    while let Ok((completed_path, waveform)) = cache.result_rx.try_recv() {
        cache.requested.remove(&completed_path);
        if let Some(waveform) = waveform {
            if cache.entries.len() < 128 || cache.entries.contains_key(&completed_path) {
                cache.entries.insert(completed_path, Arc::new(waveform));
            }
        } else {
            cache.failed.insert(completed_path);
        }
    }

    if let Some(waveform) = cache.entries.get(&path_key) {
        return Some(waveform.clone());
    }
    if !cache.failed.contains(&path_key)
        && !cache.requested.contains(&path_key)
        && cache.request_tx.try_send(path_key.clone()).is_ok()
    {
        cache.requested.insert(path_key);
    }
    None
}
