use super::eq::{design_biquad, EqBand, EqBandType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioExtractBand {
    Master,
    Bass,
    Mid,
    Treble,
}

#[derive(Debug, Clone)]
pub struct AudioKeyframeOptions {
    pub band: AudioExtractBand,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub min_db: f32,
    pub max_db: f32,
    pub multiplier: f32,
}

impl Default for AudioKeyframeOptions {
    fn default() -> Self {
        Self {
            band: AudioExtractBand::Master,
            attack_ms: 10.0,
            release_ms: 100.0,
            min_db: -48.0,
            max_db: 0.0,
            multiplier: 100.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MultiBandAudioKeyframes {
    pub master: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub bass: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub mid: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub treble: Vec<crate::core::keyframe::Keyframe<f32>>,
}

/// Convert an audio buffer into multi-band animated keyframes per frame (AE Parity).
pub fn extract_multiband_audio_keyframes(
    pcm_stereo: &[f32],
    sample_rate: u32,
    fps: u32,
    total_frames: u32,
    options: &AudioKeyframeOptions,
) -> MultiBandAudioKeyframes {
    if pcm_stereo.is_empty() || sample_rate == 0 || fps == 0 || total_frames == 0 {
        return MultiBandAudioKeyframes::default();
    }

    let _total_stereo_samples = pcm_stereo.len() / 2;

    // Filter banks for frequency crossover:
    // Bass: LowPass 250Hz
    let mut bq_bass = design_biquad(
        &EqBand {
            freq: 250.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        },
        sample_rate as f64,
    );
    // Treble: HighPass 4000Hz
    let mut bq_treble = design_biquad(
        &EqBand {
            freq: 4000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::HighPass,
        },
        sample_rate as f64,
    );
    // Mid: BandPass (LowPass 4000Hz + HighPass 250Hz)
    let mut bq_mid_lp = design_biquad(
        &EqBand {
            freq: 4000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        },
        sample_rate as f64,
    );
    let mut bq_mid_hp = design_biquad(
        &EqBand {
            freq: 250.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::HighPass,
        },
        sample_rate as f64,
    );

    // Envelope followers per band
    let mut env_master = 0.0f32;
    let mut env_bass = 0.0f32;
    let mut env_mid = 0.0f32;
    let mut env_treble = 0.0f32;

    let attack_ms = if options.attack_ms.is_finite() {
        options.attack_ms.max(1.0)
    } else {
        10.0
    };
    let release_ms = if options.release_ms.is_finite() {
        options.release_ms.max(1.0)
    } else {
        100.0
    };
    let min_db = if options.min_db.is_finite() {
        options.min_db
    } else {
        -48.0
    };
    let max_db = if options.max_db.is_finite() && options.max_db > min_db {
        options.max_db
    } else {
        0.0
    };
    let multiplier = if options.multiplier.is_finite() {
        options.multiplier.max(0.0)
    } else {
        100.0
    };
    let dt = 1.0f32 / fps.max(1) as f32;
    let att_coef = (-dt / (attack_ms * 0.001)).exp();
    let rel_coef = (-dt / (release_ms * 0.001)).exp();

    let mut res = MultiBandAudioKeyframes::default();
    let db_to_value = |amplitude: f32| {
        let db = 20.0 * amplitude.max(1.0e-8).log10();
        let span = (max_db - min_db).max(f32::EPSILON);
        ((db - min_db) / span).clamp(0.0, 1.0) * multiplier
    };

    for f in 0..total_frames {
        let start_sample_idx =
            ((f as f64 * sample_rate as f64) / fps.max(1) as f64).floor() as usize;
        let mut end_sample_idx =
            (((f as u64 + 1) as f64 * sample_rate as f64) / fps.max(1) as f64).ceil() as usize;
        if end_sample_idx == start_sample_idx {
            end_sample_idx = start_sample_idx.saturating_add(1);
        }
        let start_sample = (start_sample_idx.saturating_mul(2)).min(pcm_stereo.len());
        let end_sample = (end_sample_idx.saturating_mul(2)).min(pcm_stereo.len());

        let frame_buf = if start_sample < pcm_stereo.len() && start_sample < end_sample {
            pcm_stereo[start_sample..end_sample]
                .iter()
                .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
                .collect()
        } else {
            Vec::new()
        };

        if frame_buf.is_empty() {
            let kf = |val: f32| {
                crate::core::keyframe::Keyframe::new(
                    f,
                    val,
                    crate::core::keyframe::InterpolationType::Linear,
                )
            };
            res.master.push(kf(0.0));
            res.bass.push(kf(0.0));
            res.mid.push(kf(0.0));
            res.treble.push(kf(0.0));
            continue;
        }

        // Master RMS
        let mut master_sq = 0.0f64;
        for s in frame_buf.as_chunks::<2>().0 {
            master_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_master = (master_sq / (frame_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Bass filtered RMS
        let mut bass_buf = frame_buf.clone();
        bq_bass.process_stereo(&mut bass_buf);
        let mut bass_sq = 0.0f64;
        for s in bass_buf.as_chunks::<2>().0 {
            bass_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_bass = (bass_sq / (bass_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Mid filtered RMS
        let mut mid_buf = frame_buf.clone();
        bq_mid_lp.process_stereo(&mut mid_buf);
        bq_mid_hp.process_stereo(&mut mid_buf);
        let mut mid_sq = 0.0f64;
        for s in mid_buf.as_chunks::<2>().0 {
            mid_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_mid = (mid_sq / (mid_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Treble filtered RMS
        let mut treble_buf = frame_buf.clone();
        bq_treble.process_stereo(&mut treble_buf);
        let mut treble_sq = 0.0f64;
        for s in treble_buf.as_chunks::<2>().0 {
            treble_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_treble = (treble_sq / (treble_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Update envelope followers
        let update_env = |env: &mut f32, target: f32| {
            if target > *env {
                *env = target + att_coef * (*env - target);
            } else {
                *env = target + rel_coef * (*env - target);
            }
            (*env * multiplier).clamp(0.0, multiplier)
        };

        let v_master = db_to_value(update_env(&mut env_master, peak_master) / multiplier.max(1.0));
        let v_bass = db_to_value(update_env(&mut env_bass, peak_bass) / multiplier.max(1.0));
        let v_mid = db_to_value(update_env(&mut env_mid, peak_mid) / multiplier.max(1.0));
        let v_treble = db_to_value(update_env(&mut env_treble, peak_treble) / multiplier.max(1.0));

        let kf = |val: f32| {
            crate::core::keyframe::Keyframe::new(
                f,
                val,
                crate::core::keyframe::InterpolationType::Linear,
            )
        };

        res.master.push(kf(v_master));
        res.bass.push(kf(v_bass));
        res.mid.push(kf(v_mid));
        res.treble.push(kf(v_treble));
    }

    res
}
