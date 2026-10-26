use super::types::TimeRange;

/// Returns silent regions using windowed RMS, merging adjacent silent windows.
pub fn detect_silence(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    minimum_seconds: f64,
) -> Vec<TimeRange> {
    if samples.is_empty()
        || sample_rate == 0
        || !threshold_db.is_finite()
        || !minimum_seconds.is_finite()
        || minimum_seconds < 0.0
    {
        return Vec::new();
    }
    let window = (sample_rate as usize / 100).max(1);
    let amplitude_threshold = 10.0f32.powf(threshold_db.clamp(-120.0, 0.0) / 20.0);
    let mut ranges = Vec::new();
    let mut silent_start = None;
    for (window_index, chunk) in samples.chunks(window).enumerate() {
        let rms = (chunk
            .iter()
            .map(|sample| {
                if sample.is_finite() {
                    sample * sample
                } else {
                    1.0
                }
            })
            .sum::<f32>()
            / chunk.len() as f32)
            .sqrt();
        let start = window_index * window;
        if rms <= amplitude_threshold {
            silent_start.get_or_insert(start);
        } else if let Some(range_start) = silent_start.take() {
            push_silence_range(
                &mut ranges,
                range_start,
                start,
                sample_rate,
                minimum_seconds,
            );
        }
    }
    if let Some(range_start) = silent_start {
        push_silence_range(
            &mut ranges,
            range_start,
            samples.len(),
            sample_rate,
            minimum_seconds,
        );
    }
    ranges
}

fn push_silence_range(
    ranges: &mut Vec<TimeRange>,
    start: usize,
    end: usize,
    sample_rate: u32,
    minimum_seconds: f64,
) {
    let start_seconds = start as f64 / sample_rate as f64;
    let end_seconds = end as f64 / sample_rate as f64;
    if end_seconds - start_seconds >= minimum_seconds {
        ranges.push(TimeRange {
            start_seconds,
            end_seconds,
        });
    }
}

/// Returns transient/beat candidates in seconds using positive short-time energy changes.
pub fn detect_audio_beats(
    samples: &[f32],
    sample_rate: u32,
    sensitivity: f32,
    minimum_interval_seconds: f64,
) -> Vec<f64> {
    if samples.is_empty()
        || sample_rate == 0
        || !minimum_interval_seconds.is_finite()
        || minimum_interval_seconds < 0.0
    {
        return Vec::new();
    }
    let window = (sample_rate as usize / 100).max(1);
    let energies: Vec<f64> = samples
        .chunks(window)
        .map(|chunk| {
            chunk
                .iter()
                .map(|sample| {
                    if sample.is_finite() {
                        f64::from(*sample) * f64::from(*sample)
                    } else {
                        0.0
                    }
                })
                .sum::<f64>()
                / chunk.len() as f64
        })
        .collect();
    if energies.len() < 3 {
        return Vec::new();
    }
    let onsets: Vec<f64> = energies
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).max(0.0))
        .collect();
    let mean = onsets.iter().sum::<f64>() / onsets.len() as f64;
    let variance = onsets
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / onsets.len() as f64;
    let sensitivity = if sensitivity.is_finite() {
        f64::from(sensitivity.clamp(0.0, 4.0))
    } else {
        1.5f64
    };
    let threshold = mean + variance.sqrt() * sensitivity;
    let minimum_samples = (minimum_interval_seconds * sample_rate as f64) as usize;
    let mut last_sample = None;
    let mut beats = Vec::new();
    for index in 1..onsets.len().saturating_sub(1) {
        let onset = onsets[index];
        if onset > threshold && onset >= onsets[index - 1] && onset >= onsets[index + 1] {
            let sample = (index + 1) * window;
            if last_sample.is_none_or(|last| sample.saturating_sub(last) >= minimum_samples) {
                beats.push(sample as f64 / sample_rate as f64);
                last_sample = Some(sample);
            }
        }
    }
    beats
}

/// Computes a safe gain toward a target RMS level without exceeding a peak ceiling.
pub fn normalization_gain_db(samples: &[f32], target_rms_db: f32, peak_ceiling_db: f32) -> f32 {
    if samples.is_empty() || !target_rms_db.is_finite() || !peak_ceiling_db.is_finite() {
        return 0.0;
    }
    let finite: Vec<f32> = samples
        .iter()
        .copied()
        .filter(|sample| sample.is_finite())
        .collect();
    if finite.is_empty() {
        return 0.0;
    }
    let sum_squares = finite
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (sum_squares / finite.len() as f64).sqrt();
    let peak = finite
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0, f64::max);
    if !rms.is_finite() || !peak.is_finite() || rms <= f64::EPSILON || peak <= f64::EPSILON {
        return 0.0;
    }
    let rms_db = 20.0 * rms.log10();
    let peak_db = 20.0 * peak.log10();
    ((f64::from(target_rms_db) - rms_db)
        .min(f64::from(peak_ceiling_db) - peak_db)
        .clamp(-60.0, 60.0)) as f32
}

/// Returns ranges where audio remains at or above a clipping threshold.
pub fn detect_audio_clipping(
    samples: &[f32],
    sample_rate: u32,
    threshold: f32,
    minimum_samples: usize,
) -> Vec<TimeRange> {
    if samples.is_empty() || sample_rate == 0 || !threshold.is_finite() || minimum_samples == 0 {
        return Vec::new();
    }
    let threshold = threshold.abs().clamp(0.0, 1.0);
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, sample) in samples.iter().enumerate() {
        let clipped = sample.is_finite() && sample.abs() >= threshold;
        if clipped {
            start.get_or_insert(index);
        } else if let Some(range_start) = start.take() {
            if index - range_start >= minimum_samples {
                ranges.push(TimeRange {
                    start_seconds: range_start as f64 / sample_rate as f64,
                    end_seconds: index as f64 / sample_rate as f64,
                });
            }
        }
    }
    if let Some(range_start) = start {
        if samples.len() - range_start >= minimum_samples {
            ranges.push(TimeRange {
                start_seconds: range_start as f64 / sample_rate as f64,
                end_seconds: samples.len() as f64 / sample_rate as f64,
            });
        }
    }
    ranges
}
