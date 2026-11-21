//! Waveform generation and RGBA oscilloscope rasterization.

use super::types::AudioWaveformOptions;

/// Extract RMS waveform envelope
pub fn extract_waveform(samples: &[f32], target_len: usize) -> Vec<f32> {
    if samples.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    let chunk_size = (samples.len() / target_len).max(1);
    let mut out = Vec::with_capacity(target_len);
    for i in 0..target_len {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(samples.len());
        if start >= samples.len() {
            out.push(0.0);
            continue;
        }
        let mut max_val = 0.0f32;
        for &s in &samples[start..end] {
            if s.is_finite() {
                max_val = max_val.max(s.abs());
            }
        }
        out.push(max_val);
    }
    out
}

/// Generates evaluated 2D point sequence for an oscilloscope / audio waveform.
pub fn generate_waveform_points(samples: &[f32], options: &AudioWaveformOptions) -> Vec<[f32; 2]> {
    let count = options.sample_count.clamp(16, 2048);
    let mut points = Vec::with_capacity(count);

    let step = if samples.is_empty() {
        0
    } else {
        samples.len() / count
    };

    if options.is_polar {
        let center = options.start_point;
        let base_r = options.polar_radius;

        for i in 0..count {
            let s_idx = (i * step).min(samples.len().saturating_sub(1));
            let val = if samples.is_empty() {
                0.0
            } else {
                samples[s_idx]
            };

            let theta = (i as f32 / count as f32) * std::f32::consts::TAU;
            let (sin_t, cos_t) = theta.sin_cos();

            let r = base_r + val * options.max_height;
            points.push([center[0] + cos_t * r, center[1] + sin_t * r]);
        }
    } else {
        let p0 = options.start_point;
        let p1 = options.end_point;
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let normal = [-dy / len, dx / len];

        for i in 0..count {
            let t = i as f32 / (count.saturating_sub(1)).max(1) as f32;
            let s_idx = (i * step).min(samples.len().saturating_sub(1));
            let val = if samples.is_empty() {
                0.0
            } else {
                samples[s_idx]
            };

            let base_x = p0[0] + dx * t;
            let base_y = p0[1] + dy * t;
            let h = val * options.max_height;

            points.push([base_x + normal[0] * h, base_y + normal[1] * h]);
        }
    }

    points
}

/// Rasterizes oscilloscope audio waveform directly into an RGBA pixel buffer.
pub fn render_audio_waveform(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    samples: &[f32],
    options: &AudioWaveformOptions,
    color: [u8; 4],
) {
    if width == 0 || height == 0 {
        return;
    }

    let pts = generate_waveform_points(samples, options);
    if pts.len() < 2 {
        return;
    }

    let stroke_r = (options.stroke_width * 0.5).max(0.5);
    let stroke_r_sq = (stroke_r + options.softness).powi(2);

    for w in pts.windows(2) {
        let p0 = w[0];
        let p1 = w[1];

        let min_x = ((p0[0].min(p1[0]) - stroke_r - 1.0).floor() as i32).clamp(0, width as i32 - 1);
        let max_x = ((p0[0].max(p1[0]) + stroke_r + 1.0).ceil() as i32).clamp(0, width as i32 - 1);
        let min_y =
            ((p0[1].min(p1[1]) - stroke_r - 1.0).floor() as i32).clamp(0, height as i32 - 1);
        let max_y = ((p0[1].max(p1[1]) + stroke_r + 1.0).ceil() as i32).clamp(0, height as i32 - 1);

        let seg_dx = p1[0] - p0[0];
        let seg_dy = p1[1] - p0[1];
        let seg_len_sq = (seg_dx * seg_dx + seg_dy * seg_dy).max(1e-5);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32;
                let py = y as f32;

                // Project point onto line segment
                let u =
                    (((px - p0[0]) * seg_dx + (py - p0[1]) * seg_dy) / seg_len_sq).clamp(0.0, 1.0);
                let proj_x = p0[0] + u * seg_dx;
                let proj_y = p0[1] + u * seg_dy;

                let dist_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
                if dist_sq <= stroke_r_sq {
                    let dist = dist_sq.sqrt();
                    let alpha_factor = if dist <= stroke_r {
                        1.0
                    } else {
                        (1.0 - (dist - stroke_r) / options.softness.max(0.1)).clamp(0.0, 1.0)
                    };

                    let idx = ((y as u32 * width + x as u32) * 4) as usize;
                    if idx + 3 < buffer.len() {
                        let a = (color[3] as f32 * alpha_factor) as u8;
                        if a > buffer[idx + 3] {
                            buffer[idx] = color[0];
                            buffer[idx + 1] = color[1];
                            buffer[idx + 2] = color[2];
                            buffer[idx + 3] = a;
                        }
                    }
                }
            }
        }
    }
}
