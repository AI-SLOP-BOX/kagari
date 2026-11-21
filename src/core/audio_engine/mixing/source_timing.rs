use crate::core::timeline::Layer;

pub(super) fn audio_source_timing(
    layer: &Layer,
    frame: f32,
    source_fps: f32,
    output_fps: f32,
    source_frame_rate: f32,
) -> (f64, f64) {
    let source_start = layer.remap_frame_at_time(frame);
    let source_rate = layer.remap_frame_at_time(frame + 1.0) - source_start;
    let source_time =
        (f64::from(source_start) - f64::from(layer.in_frame)) / f64::from(source_fps.max(1.0));
    let sample_rate_scale = f64::from(source_frame_rate) * f64::from(output_fps)
        / f64::from(source_fps.max(1.0))
        * f64::from(source_rate);
    (source_time, sample_rate_scale)
}

pub(super) fn video_audio_source_timing(
    layer: &Layer,
    frame: f32,
    frames_dir: &str,
    source_fps: f32,
    output_fps: f32,
    source_frame_rate: f32,
    speed: f32,
) -> (f64, f64) {
    let source_start = f64::from(crate::core::timeline::video_sequence_frame(
        layer.remap_frame_at_time(frame),
        speed,
        frames_dir,
        source_fps as u32,
    ));
    let source_end = f64::from(crate::core::timeline::video_sequence_frame(
        layer.remap_frame_at_time(frame + 1.0),
        speed,
        frames_dir,
        source_fps as u32,
    ));
    let sequence_fps = f64::from(crate::core::video_import::extraction_fps(
        frames_dir,
        source_fps.max(1.0) as u32,
    ));
    let time_start = source_start / sequence_fps;
    let sample_rate_scale = f64::from(source_frame_rate) * f64::from(output_fps) / sequence_fps
        * (source_end - source_start);
    (time_start, sample_rate_scale)
}
