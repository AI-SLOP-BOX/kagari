/// Scale a remapped source-frame position using the video layer's playback
/// speed while keeping malformed values from poisoning render-time sampling.
pub fn scaled_video_source_frame(source_frame: f32, speed: f32) -> f32 {
    if !source_frame.is_finite() || !speed.is_finite() {
        return 0.0;
    }
    (f64::from(source_frame.max(0.0)) * f64::from(speed.max(0.0))).min(f64::from(f32::MAX)) as f32
}

/// Map composition-frame time into the decoded sequence's frame domain.
/// Caches created before extraction-rate metadata existed preserve their
/// historical one-frame-per-composition-frame behavior.
pub fn video_sequence_frame(
    source_frame: f32,
    speed: f32,
    frames_dir: &str,
    composition_fps: u32,
) -> f32 {
    let extraction_fps = crate::core::video_import::extraction_fps(frames_dir, composition_fps);
    video_sequence_frame_at_rate(source_frame, speed, extraction_fps, composition_fps)
}

/// Rate-parameterized form for callers that sample many points from the same
/// media item, such as timeline waveform drawing.
pub fn video_sequence_frame_at_rate(
    source_frame: f32,
    speed: f32,
    extraction_fps: f32,
    composition_fps: u32,
) -> f32 {
    if !source_frame.is_finite() || !speed.is_finite() {
        return 0.0;
    }
    let extraction_fps = if extraction_fps.is_finite() && (1.0..=240.0).contains(&extraction_fps) {
        extraction_fps
    } else {
        composition_fps.max(1) as f32
    };
    let position =
        f64::from(source_frame.max(0.0)) * f64::from(speed.max(0.0)) * f64::from(extraction_fps)
            / f64::from(composition_fps.max(1));
    position.min(f64::from(f32::MAX)) as f32
}
