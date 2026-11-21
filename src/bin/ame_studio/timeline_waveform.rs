#![allow(clippy::too_many_arguments)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_timeline_waveform(
    scene: &mut Scene,
    layer: &kagari_vfx::core::timeline::Layer,
    timeline_x: f64,
    timeline_width: f64,
    clip_top: f64,
    clip_bottom: f64,
    clip_left: f64,
    clip_right: f64,
    timeline_start: f64,
    visible_frames: f64,
    fps: u32,
    waveforms: &std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
) {
    use kagari_vfx::core::timeline::LayerType;
    let (path, video_speed) = match &layer.layer_type {
        LayerType::Audio { path, .. } => (path.as_str(), 1.0),
        LayerType::Video {
            audio_wav: Some(path),
            speed,
            ..
        } => (path.as_str(), *speed),
        _ => return,
    };
    let Some(waveform) = waveforms.get(path) else {
        return;
    };
    if waveform.peaks.len() < 2 || waveform.duration_seconds <= 0.0 {
        return;
    }
    let center_y = (clip_top + clip_bottom) * 0.5;
    let amplitude = (clip_bottom - clip_top) * 0.40;
    let fps = f64::from(fps.max(1));
    let video_fps = match &layer.layer_type {
        LayerType::Video { frames_dir, .. } => Some(f64::from(
            kagari_vfx::core::video_import::extraction_fps(frames_dir, fps as u32),
        )),
        _ => None,
    };
    let stroke = Stroke::new(1.0);
    let mut px = clip_left + 2.0;
    while px < clip_right - 2.0 {
        let fraction = ((px - timeline_x) / timeline_width.max(1.0)).clamp(0.0, 1.0);
        let comp_frame = timeline_start + fraction * visible_frames;
        let source_frame = match &layer.layer_type {
            LayerType::Video { .. } => {
                f64::from(kagari_vfx::core::timeline::video_sequence_frame_at_rate(
                    layer.remap_frame_f32(comp_frame.round() as u32),
                    video_speed,
                    video_fps.unwrap_or(fps) as f32,
                    fps as u32,
                ))
            }
            _ => comp_frame - f64::from(layer.in_frame),
        };
        let source_fps = video_fps.unwrap_or(fps);
        let seconds = source_frame / source_fps;
        if seconds >= 0.0 && seconds < f64::from(waveform.duration_seconds) {
            let index = ((seconds / f64::from(waveform.duration_seconds))
                * waveform.peaks.len() as f64) as usize;
            let peak = waveform.peaks[index.min(waveform.peaks.len() - 1)].clamp(0.0, 1.0);
            let half_height = (f64::from(peak) * amplitude).max(1.0);
            scene.stroke(
                &stroke,
                Affine::IDENTITY,
                Color::from_rgba8(136, 218, 190, 205),
                None,
                &Line::new((px, center_y - half_height), (px, center_y + half_height)),
            );
        }
        px += 2.0;
    }
}

pub(crate) fn draw_asset_waveform(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    waveform: &CachedAudioWaveform,
) {
    if waveform.peaks.is_empty() || width <= 24.0 || height <= 16.0 {
        return;
    }
    let left = x + 12.0;
    let right = x + width - 12.0;
    let center_y = y + height * 0.5;
    let amplitude = (height * 0.42).max(1.0);
    let count = waveform
        .peaks
        .len()
        .min(((right - left) * 0.5).floor().max(1.0) as usize);
    let stroke = Stroke::new(1.25);
    for index in 0..count {
        let sample_index = index * waveform.peaks.len() / count;
        let peak = f64::from(waveform.peaks[sample_index].clamp(0.0, 1.0));
        let half_height = (peak * amplitude).max(1.0);
        let px = left + index as f64 / count.max(1) as f64 * (right - left);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            Color::from_rgba8(136, 218, 190, 230),
            None,
            &Line::new((px, center_y - half_height), (px, center_y + half_height)),
        );
    }
}

pub(crate) fn format_timecode(frame: u32, fps: u32) -> String {
    let fps = fps.max(1);
    let total_seconds = frame / fps;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        total_seconds / 3600,
        (total_seconds / 60) % 60,
        total_seconds % 60,
        frame % fps
    )
}

pub(crate) fn format_duration_seconds(frames: u32, fps: u32) -> String {
    let seconds = f64::from(frames) / f64::from(fps.max(1));
    if seconds.fract().abs() < 0.000_1 {
        format!("{seconds:.0}")
    } else {
        format!("{seconds:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

pub(crate) fn format_timeline_seconds(frame: u64, fps: u32) -> String {
    let total_seconds = frame / u64::from(fps.max(1));
    if total_seconds < 60 {
        format!("{total_seconds}s")
    } else {
        format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
    }
}

pub(crate) fn model_preview_cache_key(path: &str) -> String {
    let revision =
        kagari_vfx::core::media_revision::media_file_revision(std::path::Path::new(path));
    format!("model:{path}:{revision}")
}

pub(crate) fn asset_image_preview_cache_key(asset_id: &str, path: &str) -> String {
    let revision =
        kagari_vfx::core::media_revision::media_file_revision(std::path::Path::new(path));
    format!("{asset_id}:{path}:{revision}")
}

pub(crate) fn render_asset_image_preview_pixels(
    cache: &mut kagari_vfx::core::image_cache::ImageCache,
    path: &str,
) -> Option<CachedAssetPreview> {
    const MAX_WIDTH: u32 = 640;
    const MAX_HEIGHT: u32 = 360;
    let image = cache.load_image(path)?;
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let scale = (f64::from(MAX_WIDTH) / f64::from(image.width))
        .min(f64::from(MAX_HEIGHT) / f64::from(image.height))
        .min(1.0);
    let width = (f64::from(image.width) * scale).round().max(1.0) as u32;
    let height = (f64::from(image.height) * scale).round().max(1.0) as u32;
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)?
        .checked_mul(4)?;
    if image.pixels.len() != expected {
        return None;
    }
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for y in 0..height {
        let source_y = (u64::from(y) * u64::from(image.height) / u64::from(height)) as u32;
        for x in 0..width {
            let source_x = (u64::from(x) * u64::from(image.width) / u64::from(width)) as u32;
            let source_offset =
                ((source_y as usize * image.width as usize) + source_x as usize) * 4;
            let destination_offset = ((y as usize * width as usize) + x as usize) * 4;
            pixels[destination_offset..destination_offset + 4]
                .copy_from_slice(&image.pixels[source_offset..source_offset + 4]);
        }
    }
    Some(CachedAssetPreview {
        pixels: std::sync::Arc::new(pixels),
        width,
        height,
    })
}
