pub fn ruler_label_step(visible_span: u32, ruler_width: f32, measured_label_width: f32) -> u32 {
    let minimum_label_spacing = measured_label_width.max(0.0) + 8.0;
    let required_frames = ((visible_span.max(1) as f64 * minimum_label_spacing as f64)
        / ruler_width.max(1.0) as f64)
        .ceil()
        .clamp(1.0, u32::MAX as f64) as u32;
    let mut decade = 1_u32;
    while decade <= required_frames / 10 {
        decade = decade.saturating_mul(10);
    }
    [1_u32, 2, 5, 10]
        .into_iter()
        .map(|factor| decade.saturating_mul(factor))
        .find(|step| *step >= required_frames)
        .unwrap_or(u32::MAX)
        .max(1)
}

pub fn ruler_tick_frames(start_frame: u32, visible_span: u32, step: u32) -> Vec<u32> {
    let step = step.max(1);
    let remainder = start_frame % step;
    let first_frame = start_frame.saturating_add(if remainder == 0 { 0 } else { step - remainder });
    let end_frame = start_frame.saturating_add(visible_span);
    let mut frames = Vec::new();
    let mut frame = first_frame;
    while frame <= end_frame {
        frames.push(frame);
        let Some(next_frame) = frame.checked_add(step) else {
            break;
        };
        frame = next_frame;
    }
    frames
}

pub fn format_timecode_label(frame: u32, fps: u32) -> String {
    let fps = fps.max(1);
    let seconds = frame / fps;
    let minutes = seconds / 60;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        minutes / 60,
        minutes % 60,
        seconds % 60,
        frame % fps
    )
}
