#[cfg(test)]
mod tests {
    use crate::core::audio_spectrum::*;

    #[test]
    fn test_audio_analysis_ignores_nonfinite_samples_and_magnitudes() {
        let samples = [f32::NAN, f32::INFINITY, 0.5, -0.5];
        let options = AudioSpectrumOptions {
            frequency_bands: 2,
            ..Default::default()
        };
        let bands = generate_audio_spectrum_bands(&samples, 48_000, &options);
        assert_eq!(bands.len(), 2);
        assert!(bands.iter().all(|value| value.is_finite()));

        let mut buffer = vec![11u8; 16];
        render_spectrum(
            &mut buffer,
            2,
            2,
            &[f32::NAN, f32::INFINITY],
            &options,
            [255, 0, 0],
            [0, 0, 255],
        );
    }

    #[test]
    fn test_audio_spectrum_generation_linear() {
        let samples = vec![0.5f32; 1024];
        let opts = AudioSpectrumOptions {
            frequency_bands: 16,
            is_polar: false,
            max_height: 100.0,
            ..Default::default()
        };

        let bars = generate_audio_spectrum(&samples, &opts);
        assert_eq!(bars.len(), 16);
        assert!(bars[0].magnitude > 0.0);
        let h = ((bars[0].end[0] - bars[0].start[0]).powi(2)
            + (bars[0].end[1] - bars[0].start[1]).powi(2))
        .sqrt();
        assert!(h > 0.0);
    }

    #[test]
    fn test_audio_spectrum_generation_polar() {
        let samples = vec![0.8f32; 512];
        let center = [200.0f32, 540.0];
        let opts = AudioSpectrumOptions {
            frequency_bands: 8,
            is_polar: true,
            polar_radius: 50.0,
            max_height: 50.0,
            start_point: center,
            ..Default::default()
        };

        let bars = generate_audio_spectrum(&samples, &opts);
        assert_eq!(bars.len(), 8);
        // Polar branch: every bar starts exactly on the base circle, proving
        // radial layout ran (linear layout would scatter start points).
        for (i, bar) in bars.iter().enumerate() {
            let r_start =
                ((bar.start[0] - center[0]).powi(2) + (bar.start[1] - center[1]).powi(2)).sqrt();
            let r_end =
                ((bar.end[0] - center[0]).powi(2) + (bar.end[1] - center[1]).powi(2)).sqrt();
            assert!(
                (r_start - 50.0).abs() < 1e-3,
                "bar {} starts off the base circle: r={}",
                i,
                r_start
            );
            assert!(
                r_end + 1e-3 >= r_start,
                "bar {} extends inward: {} < {}",
                i,
                r_end,
                r_start
            );
            assert!(
                bar.magnitude.is_finite() && (0.0..=1.0).contains(&bar.magnitude),
                "bar {} magnitude out of normalized range: {}",
                i,
                bar.magnitude
            );
        }
        // DC-heavy input carries energy: at least one bar must be non-trivial.
        assert!(
            bars.iter().any(|b| b.magnitude > 0.01),
            "constant signal produced only silence"
        );
    }

    #[test]
    fn test_audio_waveform_generation_and_render() {
        let samples = vec![0.3f32; 512];
        let opts = AudioWaveformOptions {
            start_point: [10.0, 32.0],
            end_point: [54.0, 32.0],
            max_height: 20.0,
            ..Default::default()
        };
        let pts = generate_waveform_points(&samples, &opts);
        assert_eq!(pts.len(), opts.sample_count);

        let mut buf = vec![0u8; 64 * 64 * 4];
        render_audio_waveform(&mut buf, 64, 64, &samples, &opts, [255, 255, 255, 255]);
        // Buffer should contain some drawn waveform pixels
        let non_zero = buf.iter().any(|&b| b > 0);
        assert!(non_zero);
    }

    #[test]
    fn test_multiband_audio_keyframes() {
        let sine_wave: Vec<f32> = (0..44100)
            .map(|i| ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 44100.0).sin() * 0.5)
            .collect();
        let kfs = analyze_audio_multiband_keyframes(&sine_wave, 44100, 30.0, 30);
        assert_eq!(kfs.len(), 30);
        assert!(kfs[0].master_amplitude > 0.0);
    }
}
