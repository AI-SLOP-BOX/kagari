//! Audio DSP effects: Parametric EQ, Compressor, Limiter, and High/Low-pass filters.
//! All processing is done on f32 stereo interleaved buffers.

pub mod dynamics;
pub mod eq;
pub mod extraction;

pub use dynamics::*;
pub use eq::*;
pub use extraction::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_lowpass_stability() {
        let bands = vec![EqBand {
            freq: 1000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        }];
        let mut buf = vec![0.0f32; 1024];
        buf[0] = 1.0;
        buf[1] = 1.0;
        apply_eq(&mut buf, &bands, 44100);
        for s in &buf {
            assert!(s.abs() < 10.0, "Low-pass filter went unstable: {}", s);
        }
    }

    #[test]
    fn test_eq_with_invalid_sample_rate_is_a_safe_bypass() {
        let mut buf = vec![0.25f32, -0.5, 0.75, -1.0];
        let original = buf.clone();
        apply_eq(&mut buf, &[EqBand::default()], 0);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_eq_skips_invalid_band_parameters() {
        let mut buf = vec![0.25f32, -0.5, 0.75, -1.0];
        let original = buf.clone();
        apply_eq(
            &mut buf,
            &[
                EqBand {
                    q: 0.0,
                    ..EqBand::default()
                },
                EqBand {
                    freq: f32::NAN,
                    ..EqBand::default()
                },
            ],
            48_000,
        );
        assert_eq!(buf, original);
    }

    #[test]
    fn test_eq_sanitizes_nonfinite_input_samples() {
        let mut buf = vec![f32::NAN, f32::INFINITY, 0.25, -0.25];
        apply_eq(
            &mut buf,
            &[EqBand {
                band_type: EqBandType::LowPass,
                freq: 1_000.0,
                ..EqBand::default()
            }],
            48_000,
        );
        assert!(buf.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn test_eq_skips_extreme_finite_band_parameters() {
        let mut buf = vec![0.25f32, -0.5, 0.75, -1.0];
        let original = buf.clone();
        apply_eq(
            &mut buf,
            &[EqBand {
                gain_db: f32::MAX,
                ..EqBand::default()
            }],
            48_000,
        );
        assert_eq!(buf, original);
    }

    #[test]
    fn test_auto_gain_moves_rms_toward_target_without_unbounded_boost() {
        let mut buf = vec![0.1f32; 4096];
        apply_auto_gain(&mut buf, -18.0);
        let rms =
            (buf.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / buf.len() as f64).sqrt();
        assert!((20.0 * rms.log10() + 18.0).abs() < 0.1);

        let mut silence = vec![0.0f32; 64];
        apply_auto_gain(&mut silence, -18.0);
        assert!(silence.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn test_eq_bell_boost() {
        let bands = vec![EqBand {
            freq: 1000.0,
            gain_db: 12.0,
            q: 1.0,
            band_type: EqBandType::Bell,
        }];
        let sr = 44100u32;
        let len = 1024usize;
        let mut buf = Vec::with_capacity(len * 2);
        for i in 0..len {
            let t = i as f32 / sr as f32;
            let v = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5;
            buf.push(v);
            buf.push(v);
        }
        let rms_before: f64 =
            buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        apply_eq(&mut buf, &bands, sr);
        let rms_after: f64 =
            buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        assert!(
            rms_after > rms_before * 1.5,
            "Bell boost did not increase level: before={:.4} after={:.4}",
            rms_before,
            rms_after
        );
    }

    #[test]
    fn test_compressor_reduces_dynamic_range() {
        let params = CompressorParams {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        };
        let mut state = CompressorState::default();
        let mut buf_loud = vec![0.0f32; 4096];
        for s in buf_loud.as_chunks_mut::<2>().0 {
            s[0] = 0.8;
            s[1] = 0.8;
        }
        let rms_before: f64 = 0.8;
        apply_compressor(&mut buf_loud, &params, &mut state, 44100);
        let rms_after: f64 = buf_loud
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum::<f64>()
            .sqrt()
            / 2048.0;
        assert!(
            rms_after < rms_before,
            "Compressor did not reduce loud signal: before={:.4} after={:.4}",
            rms_before,
            rms_after
        );
    }

    #[test]
    fn test_limiter_clips() {
        let mut state = CompressorState::default();
        let mut buf = vec![0.0f32; 1024];
        for s in buf.as_chunks_mut::<2>().0 {
            s[0] = 1.5;
            s[1] = 1.5;
        }
        apply_limiter(&mut buf, -0.1, 50.0, &mut state, 44100);
        for s in &buf {
            assert!(s.abs() <= 1.001, "Limiter failed to clip: {}", s);
        }
    }

    #[test]
    fn test_compressor_invalid_parameters_do_not_create_nonfinite_audio() {
        let params = CompressorParams {
            threshold_db: f32::NAN,
            ratio: 0.0,
            attack_ms: f32::INFINITY,
            release_ms: 0.0,
            knee_db: 0.0,
            makeup_gain_db: f32::INFINITY,
        };
        let mut state = CompressorState::default();
        let mut buf = vec![f32::NAN, 0.5, f32::INFINITY, -0.5];
        apply_compressor(&mut buf, &params, &mut state, 48_000);
        assert!(buf.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn test_compressor_recovers_invalid_state_and_extreme_output() {
        let params = CompressorParams::default();
        let mut state = CompressorState {
            envelope: f32::NAN.into(),
        };
        let mut buf = vec![f32::MAX, f32::MAX];
        apply_compressor(&mut buf, &params, &mut state, 48_000);
        assert!(state.envelope.is_finite());
        assert!(buf.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn test_limiter_cleans_nonfinite_odd_tail() {
        let mut state = CompressorState::default();
        let mut buf = vec![0.0, 0.0, f32::NAN];
        apply_limiter(&mut buf, -1.0, 50.0, &mut state, 48_000);
        assert!(buf.iter().all(|sample| sample.is_finite()));
        assert_eq!(buf[2], 0.0);
    }

    #[test]
    fn test_extract_multiband_audio_keyframes() {
        let sr = 44100u32;
        let fps = 30u32;
        let total_frames = 10u32;
        let num_samples = (sr as usize / fps as usize) * (total_frames as usize);
        let mut pcm = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sr as f32;
            let v = (2.0 * std::f32::consts::PI * 100.0 * t).sin() * 0.8;
            pcm.push(v);
            pcm.push(v);
        }

        let options = AudioKeyframeOptions::default();
        let kfs = extract_multiband_audio_keyframes(&pcm, sr, fps, total_frames, &options);

        assert_eq!(kfs.master.len(), total_frames as usize);
        assert_eq!(kfs.bass.len(), total_frames as usize);
        assert_eq!(kfs.treble.len(), total_frames as usize);

        let mid_frame = (total_frames / 2) as usize;
        let bass_val = kfs.bass[mid_frame].value;
        let treble_val = kfs.treble[mid_frame].value;
        assert!(
            bass_val > treble_val * 3.0,
            "Bass {} must be much higher than Treble {}",
            bass_val,
            treble_val
        );
    }

    #[test]
    fn test_extract_multiband_audio_keyframes_low_sample_rate() {
        let sr = 20u32;
        let fps = 60u32;
        let total_frames = 30u32;
        let pcm = vec![0.8f32; 100];
        let options = AudioKeyframeOptions::default();
        let kfs = extract_multiband_audio_keyframes(&pcm, sr, fps, total_frames, &options);

        assert_eq!(kfs.master.len(), total_frames as usize);
        assert!(kfs.master.iter().any(|kf| kf.value > 0.0));
    }

    #[test]
    fn test_extract_multiband_sanitizes_nonfinite_options_and_pcm() {
        let options = AudioKeyframeOptions {
            attack_ms: f32::NAN,
            release_ms: f32::INFINITY,
            min_db: f32::NAN,
            max_db: f32::NEG_INFINITY,
            multiplier: f32::NAN,
            ..AudioKeyframeOptions::default()
        };
        let pcm = vec![f32::NAN, f32::INFINITY, 0.5, -0.5];
        let result = extract_multiband_audio_keyframes(&pcm, 48_000, 30, 4, &options);
        assert_eq!(result.master.len(), 4);
        assert!(result
            .master
            .iter()
            .chain(&result.bass)
            .chain(&result.mid)
            .chain(&result.treble)
            .all(|kf| kf.value.is_finite()));
    }

    #[test]
    fn test_extract_multiband_audio_is_silent_after_eof() {
        let options = AudioKeyframeOptions {
            min_db: -60.0,
            max_db: 0.0,
            multiplier: 100.0,
            ..Default::default()
        };
        let pcm = vec![1.0f32, 1.0f32];
        let kfs = extract_multiband_audio_keyframes(&pcm, 48_000, 24, 3, &options);

        assert!(kfs.master[0].value > 0.0);
        assert_eq!(kfs.master[1].value, 0.0);
        assert_eq!(kfs.master[2].value, 0.0);
    }
}
