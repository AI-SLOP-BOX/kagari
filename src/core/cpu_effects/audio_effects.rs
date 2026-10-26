use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::BassTreble { .. }
            | EffectType::Flanger { .. }
            | EffectType::Chorus { .. }
            | EffectType::ParametricEQ { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext { frame, .. } = context;

    match effect_type {
        EffectType::BassTreble {
            bass_gain,
            treble_gain,
            crossover_freq,
        } => {
            let bass = bass_gain.evaluate(frame);
            let treble = treble_gain.evaluate(frame);
            let cross = crossover_freq.evaluate(frame).clamp(20.0, 5000.0);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.as_chunks::<4>().0 {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::audio_effects_dsp::apply_bass_treble(
                    &mut luma, 44100.0, bass, treble, cross,
                );
                for (px, &lv) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(luma.iter()) {
                    let ratio =
                        if (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32)
                            > 0.5
                        {
                            lv / 128.0
                        } else {
                            1.0
                        };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::Flanger {
            max_delay_ms,
            lfo_rate,
            feedback,
            wet_dry,
        } => {
            let md = max_delay_ms.evaluate(frame);
            let rate = lfo_rate.evaluate(frame);
            let fb = feedback.evaluate(frame).clamp(0.0, 0.95);
            let wd = wet_dry.evaluate(frame);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.as_chunks::<4>().0 {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::audio_effects_dsp::apply_flanger(&mut luma, 44100.0, md, rate, fb, wd);
                for (px, &lv) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::Chorus {
            delay_ms,
            depth_ms,
            rate_hz,
            voices,
            feedback,
        } => {
            let dm = delay_ms.evaluate(frame);
            let dp = depth_ms.evaluate(frame);
            let rt = rate_hz.evaluate(frame);
            let vc = voices.evaluate(frame);
            let fb = feedback.evaluate(frame).clamp(0.0, 0.9);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.as_chunks::<4>().0 {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::audio_effects_dsp::apply_chorus(
                    &mut luma, 44100.0, dm, dp, rt, vc, fb,
                );
                for (px, &lv) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::ParametricEQ {
            freq_hz,
            gain_db,
            q_factor,
        } => {
            let freq = freq_hz.evaluate(frame).clamp(20.0, 20000.0);
            let gain = gain_db.evaluate(frame);
            let q = q_factor.evaluate(frame).clamp(0.5, 20.0);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.as_chunks::<4>().0 {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::audio_effects_dsp::apply_parametric_eq_bell(
                    &mut luma, 44100.0, freq, gain, q,
                );
                for (px, &lv) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        _ => unreachable!(),
    }
    true
}
