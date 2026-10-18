use crate::core::timeline::{Composition, Layer, LayerType};
use eframe::egui;

const MAX_TIMELINE_THUMBNAIL_TEXTURES: usize = 256;

struct TimelineThumbnailRequest {
    key: String,
    path: std::path::PathBuf,
    target_dim: u32,
}

type ThumbnailResultReceiver = std::sync::mpsc::Receiver<(String, Option<(usize, usize, Vec<u8>)>)>;

struct TimelineThumbnailCache {
    version: u64,
    textures: std::collections::HashMap<String, egui::TextureHandle>,
    order: std::collections::VecDeque<String>,
    requested: std::collections::HashSet<String>,
    failed: std::collections::HashSet<String>,
    request_tx: std::sync::mpsc::SyncSender<TimelineThumbnailRequest>,
    result_rx: ThumbnailResultReceiver,
}

pub(super) fn layer_has_timeline_preview(layer_type: &LayerType) -> bool {
    matches!(
        layer_type,
        LayerType::Video { .. } | LayerType::Image { .. }
    )
}

pub(super) fn timeline_layer_preview_texture(
    ctx: &egui::Context,
    composition: &Composition,
    layer: &Layer,
    timeline_frame: u32,
) -> Option<egui::TextureId> {
    let version = crate::core::frame_cache::current_version();
    let (path, source_frame) = match &layer.layer_type {
        LayerType::Image { path } => (std::path::PathBuf::from(path), Some(0)),
        LayerType::Video {
            frames_dir,
            frame_count,
            speed,
            ..
        } => {
            let source_position = (match &layer.posterize_time {
                Some(settings) if settings.enabled => {
                    layer.effective_render_frame(timeline_frame, composition.fps) as f32
                }
                _ => layer.remap_frame_f32(timeline_frame),
            } * speed.max(0.0))
            .max(0.0);
            let source_frame = (source_position.floor() as u32).min(frame_count.saturating_sub(1));
            (
                crate::core::video_import::frame_path_in_dir(frames_dir, source_frame),
                Some(source_frame),
            )
        }
        _ => return None,
    };
    let key = format!(
        "{}:{}:{}:{}:{}",
        version,
        composition.id,
        layer.id,
        path.display(),
        source_frame.unwrap_or(timeline_frame)
    );
    let cache_id = egui::Id::new("timeline_media_thumbnail_cache_v2");
    let cache = ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with(cache_id, || {
            let (request_tx, request_rx) =
                std::sync::mpsc::sync_channel::<TimelineThumbnailRequest>(8);
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let preview_ctx = ctx.clone();
            std::thread::Builder::new()
                .name("timeline-media-thumbnails".into())
                .spawn(move || {
                    while let Ok(request) = request_rx.recv() {
                        let decoded = image::open(&request.path).ok().map(|image| {
                            let (orig_w, orig_h) = (image.width(), image.height());
                            let (target_w, target_h) = if orig_w == 0 || orig_h == 0 {
                                (request.target_dim, request.target_dim)
                            } else if orig_w >= orig_h {
                                let w = request.target_dim;
                                let h = ((request.target_dim as f32 * orig_h as f32)
                                    / orig_w as f32)
                                    .round()
                                    .max(1.0) as u32;
                                (w, h)
                            } else {
                                let h = request.target_dim;
                                let w = ((request.target_dim as f32 * orig_w as f32)
                                    / orig_h as f32)
                                    .round()
                                    .max(1.0) as u32;
                                (w, h)
                            };
                            let thumbnail = image::imageops::resize(
                                &image.to_rgba8(),
                                target_w,
                                target_h,
                                image::imageops::FilterType::Triangle,
                            );
                            (target_w as usize, target_h as usize, thumbnail.into_raw())
                        });
                        if result_tx.send((request.key, decoded)).is_err() {
                            break;
                        }
                        preview_ctx.request_repaint_after(std::time::Duration::from_millis(16));
                    }
                })
                .ok();
            std::sync::Arc::new(std::sync::Mutex::new(TimelineThumbnailCache {
                version,
                textures: std::collections::HashMap::new(),
                order: std::collections::VecDeque::new(),
                requested: std::collections::HashSet::new(),
                failed: std::collections::HashSet::new(),
                request_tx,
                result_rx,
            }))
        })
        .clone()
    });
    let Ok(mut cache) = cache.lock() else {
        return None;
    };
    if cache.version != version {
        cache.version = version;
        cache.textures.clear();
        cache.order.clear();
        cache.requested.clear();
        cache.failed.clear();
    }
    while let Ok((completed_key, decoded)) = cache.result_rx.try_recv() {
        cache.requested.remove(&completed_key);
        if !completed_key.starts_with(&format!("{version}:")) {
            continue;
        }
        match decoded {
            Some((width, height, pixels)) if width > 0 && height > 0 => {
                while cache.textures.len() >= MAX_TIMELINE_THUMBNAIL_TEXTURES {
                    if let Some(oldest_key) = cache.order.pop_front() {
                        cache.textures.remove(&oldest_key);
                    } else {
                        break;
                    }
                }
                let texture = ctx.load_texture(
                    format!("timeline-media-thumbnail:{completed_key}"),
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels),
                    egui::TextureOptions::LINEAR,
                );
                cache.textures.insert(completed_key.clone(), texture);
                cache.order.push_back(completed_key);
            }
            Some(_) => {}
            None => {
                cache.failed.insert(completed_key);
            }
        }
    }
    if let Some(texture) = cache.textures.get(&key) {
        return Some(texture.id());
    }
    if (cache.textures.len() + cache.requested.len()) < MAX_TIMELINE_THUMBNAIL_TEXTURES * 2
        && !cache.failed.contains(&key)
        && !cache.requested.contains(&key)
    {
        let request = TimelineThumbnailRequest {
            key: key.clone(),
            path,
            target_dim: 96,
        };
        if cache.request_tx.try_send(request).is_ok() {
            cache.requested.insert(key);
        }
    }
    None
}
