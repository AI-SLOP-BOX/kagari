use crate::core::timeline::Composition;

pub(crate) fn inject_audio_reactive_data(comp: &Composition, frame: u32) {
    // If there are no audio-bearing layers (Audio, Video, or PreComp) in the composition,
    // skip the heavy audio mixing and FFT spectrum analysis entirely.
    let has_audio_bearing_layers = comp.layers.iter().any(|l| {
        matches!(
            l.layer_type,
            crate::core::timeline::LayerType::Audio { .. }
                | crate::core::timeline::LayerType::Video { .. }
                | crate::core::timeline::LayerType::PreComp { .. }
        )
    });
    let needs_visualizer_spectrum = comp.layers.iter().any(|layer| {
        layer.is_active(frame)
            && layer.visible
            && layer.effects.iter().any(|effect| {
                effect.enabled
                    && matches!(
                        &effect.effect_type,
                        crate::core::timeline::EffectType::AudioSpectrum { enabled, .. }
                            if enabled.evaluate(frame) > 0.5
                    )
            })
    });

    if !has_audio_bearing_layers {
        crate::core::expression_engine::set_audio_expr_data(
            crate::core::expression_engine::AudioExprData::default(),
        );
        crate::core::expression_engine::set_audio_spectrum_bands(Vec::new());
        return;
    }

    // Mix audio for the current frame and compute spectrum bands for expressions.
    {
        use crate::core::audio_engine::mix_audio_for_frame;
        use crate::core::audio_spectrum::SpectrumAnalyzer;
        let sample_rate = 44100u32;
        let buffer_size = 2048u32;
        let (pcm, meter) = mix_audio_for_frame(
            comp,
            frame,
            sample_rate,
            buffer_size as usize,
            &crate::core::audio_engine::MasterDspParams::default(),
        );
        let peak = meter.peak_db_left.max(meter.peak_db_right);
        let amplitude = peak.clamp(-60.0, 0.0) / 60.0;

        // Compute 5 frequency bands using the spectrum analyzer
        let mut analyzer = SpectrumAnalyzer::new(5);
        let options = crate::core::audio_spectrum::AudioSpectrumOptions {
            fft_size: 2048,
            frequency_bands: 5,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
            ..Default::default()
        };
        let bands_raw = analyzer.analyze(&pcm, sample_rate, &options);
        let mut bands = [0.0f32; 5];
        for (i, b) in bands_raw.iter().enumerate().take(5) {
            bands[i] = *b;
        }

        crate::core::expression_engine::set_audio_expr_data(
            crate::core::expression_engine::AudioExprData { amplitude, bands },
        );

        if needs_visualizer_spectrum {
            let spectrum_options = crate::core::audio_spectrum::AudioSpectrumOptions {
                fft_size: 2048,
                frequency_bands: 512,
                start_frequency: 20.0,
                end_frequency: 20_000.0,
                smoothing: 0.0,
                ..Default::default()
            };
            let spectrum = crate::core::audio_spectrum::generate_audio_spectrum_bands(
                &pcm,
                sample_rate,
                &spectrum_options,
            );
            crate::core::expression_engine::set_audio_spectrum_bands(spectrum);
        } else {
            crate::core::expression_engine::set_audio_spectrum_bands(Vec::new());
        }
    }
}
