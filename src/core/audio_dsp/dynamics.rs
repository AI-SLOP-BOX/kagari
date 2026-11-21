pub fn finite_audio_sample(value: f64) -> f32 {
    if value.is_finite() {
        value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    } else if value.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

#[derive(Debug, Clone)]
pub struct AutoGainState {
    pub gain_db: f32,
    initialized: bool,
}

impl Default for AutoGainState {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            initialized: false,
        }
    }
}

/// Apply a conservative, smoothed block loudness correction.
pub fn apply_auto_gain_with_state(
    buf: &mut [f32],
    target_db: f32,
    sample_rate: u32,
    state: &mut AutoGainState,
) {
    if buf.is_empty() {
        return;
    }
    let mut sum_sq = 0.0f64;
    let mut count = 0usize;
    for sample in buf.iter().copied() {
        if sample.is_finite() {
            sum_sq += f64::from(sample) * f64::from(sample);
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let rms = (sum_sq / count as f64).sqrt();
    if rms <= 1.0e-8 {
        return;
    }
    let current_db = 20.0 * rms.log10();
    let target_db = if target_db.is_finite() {
        target_db.clamp(-60.0, 0.0)
    } else {
        -18.0
    };
    let desired_gain_db = (target_db as f64 - current_db).clamp(-12.0, 18.0) as f32;
    if !state.gain_db.is_finite() {
        state.gain_db = 0.0;
        state.initialized = false;
    }
    if !state.initialized {
        state.gain_db = desired_gain_db;
        state.initialized = true;
    } else {
        let duration_sec = if sample_rate == 0 {
            0.0
        } else {
            (buf.len() / 2) as f32 / sample_rate as f32
        };
        let time_constant = if desired_gain_db < state.gain_db {
            0.08
        } else {
            0.4
        };
        let alpha = 1.0 - (-duration_sec / time_constant).exp();
        state.gain_db += (desired_gain_db - state.gain_db) * alpha;
    }
    let gain = 10.0f64.powf(f64::from(state.gain_db) / 20.0);
    for sample in buf.iter_mut() {
        *sample = finite_audio_sample(f64::from(*sample) * gain);
    }
}

/// Stateless convenience wrapper used by isolated DSP tests and one-shot callers.
pub fn apply_auto_gain(buf: &mut [f32], target_db: f32) {
    let mut state = AutoGainState::default();
    apply_auto_gain_with_state(buf, target_db, 48_000, &mut state);
}

/// Compressor state (keeps running envelope for smooth gain reduction).
#[derive(Debug, Clone)]
pub struct CompressorState {
    pub(crate) envelope: f64,
}

impl Default for CompressorState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Compressor parameters.
#[derive(Debug, Clone)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_gain_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        }
    }
}

/// Apply compressor to a stereo interleaved buffer.
pub fn apply_compressor(
    buf: &mut [f32],
    params: &CompressorParams,
    state: &mut CompressorState,
    sample_rate: u32,
) {
    if buf.is_empty() || sample_rate == 0 {
        return;
    }
    let fs = sample_rate as f64;
    let attack_ms = if params.attack_ms.is_finite() {
        params.attack_ms.max(0.001)
    } else {
        5.0
    };
    let release_ms = if params.release_ms.is_finite() {
        params.release_ms.max(0.001)
    } else {
        50.0
    };
    let attack_coeff = (-1.0 / (f64::from(attack_ms) * 0.001 * fs)).exp();
    let release_coeff = (-1.0 / (f64::from(release_ms) * 0.001 * fs)).exp();
    let threshold = if params.threshold_db.is_finite() {
        f64::from(params.threshold_db)
    } else {
        -20.0
    };
    let ratio = if params.ratio.is_finite() {
        f64::from(params.ratio.max(1.0))
    } else {
        4.0
    };
    let knee = if params.knee_db.is_finite() {
        f64::from(params.knee_db.max(0.0))
    } else {
        6.0
    };
    let makeup_db = if params.makeup_gain_db.is_finite() {
        f64::from(params.makeup_gain_db.clamp(-120.0, 120.0))
    } else {
        0.0
    };
    let makeup = 10.0_f64.powf(makeup_db / 20.0);
    if !state.envelope.is_finite() {
        state.envelope = 0.0;
    }

    for sample in buf.as_chunks_mut::<2>().0 {
        let peak_l = if sample[0].is_finite() {
            f64::from(sample[0])
        } else {
            0.0
        };
        let peak_r = if sample[1].is_finite() {
            f64::from(sample[1])
        } else {
            0.0
        };
        let peak = peak_l.abs().max(peak_r.abs());

        let level_db = if peak > 1e-10 {
            20.0 * peak.log10()
        } else {
            -120.0
        };

        let gr_db = if knee <= f64::EPSILON {
            if level_db > threshold {
                threshold + (level_db - threshold) / ratio - level_db
            } else {
                0.0
            }
        } else if level_db < threshold - knee / 2.0 {
            0.0
        } else if level_db > threshold + knee / 2.0 {
            threshold + (level_db - threshold) / ratio - level_db
        } else {
            let x = level_db - threshold + knee / 2.0;
            (1.0 / ratio - 1.0) * x * x / (2.0 * knee)
        };

        let coeff = if gr_db < state.envelope {
            attack_coeff
        } else {
            release_coeff
        };
        state.envelope = coeff * state.envelope + (1.0 - coeff) * gr_db;

        let gain = 10.0_f64.powf(state.envelope / 20.0) * makeup;
        sample[0] = finite_audio_sample(peak_l * gain);
        sample[1] = finite_audio_sample(peak_r * gain);
    }
}

/// Apply a simple limiter (hard-knee compressor with very high ratio).
pub fn apply_limiter(
    buf: &mut [f32],
    ceiling_db: f32,
    release_ms: f32,
    state: &mut CompressorState,
    sample_rate: u32,
) {
    let params = CompressorParams {
        threshold_db: ceiling_db,
        ratio: 20.0,
        attack_ms: 0.1,
        release_ms,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    apply_compressor(buf, &params, state, sample_rate);

    for sample in buf.iter_mut() {
        *sample = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }
}
