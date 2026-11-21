use super::*;

#[path = "mixing/export.rs"]
mod export;
#[path = "mixing/source_timing.rs"]
mod source_timing;
pub use export::mix_composition_to_wav;
pub use export::mix_composition_to_wav_cancelable;
use source_timing::{audio_source_timing, video_audio_source_timing};

/// Read an audio frame at a fractional time using linear interpolation.
/// Integer truncation here produces audible stepping whenever playback speed
/// or time remapping moves the read head between source samples.
fn sample_audio_at_time(buffer: &AudioBuffer, time_seconds: f64) -> (f32, f32) {
    let channels = usize::from(buffer.channels);
    if channels == 0 || buffer.sample_rate == 0 || !time_seconds.is_finite() || time_seconds < 0.0 {
        return (0.0, 0.0);
    }

    let frame_count = buffer.samples.len() / channels;
    if frame_count == 0 {
        return (0.0, 0.0);
    }
    let position = time_seconds * f64::from(buffer.sample_rate);
    if !position.is_finite() || position >= frame_count as f64 {
        return (0.0, 0.0);
    }
    let first_frame = position.floor() as usize;
    let second_frame = (first_frame + 1).min(frame_count - 1);
    let fraction = (position - first_frame as f64) as f32;
    let sample_channel = |channel: usize| {
        let channel = channel.min(channels - 1);
        let first = buffer.samples[first_frame * channels + channel];
        let second = buffer.samples[second_frame * channels + channel];
        first + (second - first) * fraction
    };
    (sample_channel(0), sample_channel(1))
}

/// Apply per-layer audio effects (BassTreble) to stereo interleaved samples.
/// Called on each layer's audio samples BEFORE mixing them together.
pub fn apply_layer_audio_effects(samples: &mut [f32], layer: &Layer, sample_rate: f32, frame: u32) {
    let mut states = std::collections::HashMap::new();
    apply_layer_audio_effects_with_state(
        samples,
        layer,
        sample_rate,
        frame,
        frame,
        "",
        &mut states,
    );
}

fn apply_layer_audio_effects_with_state(
    samples: &mut [f32],
    layer: &Layer,
    sample_rate: f32,
    frame: u32,
    state_frame: u32,
    instance_id: &str,
    states: &mut std::collections::HashMap<
        (String, String, usize),
        (u32, crate::core::audio_dsp::EqState),
    >,
) {
    if !layer.effects_enabled || samples.is_empty() {
        return;
    }
    use crate::core::audio_dsp;
    for (effect_index, effect) in layer.effects.iter().enumerate() {
        if !effect.enabled {
            continue;
        }
        if let EffectType::BassTreble {
            bass_gain,
            treble_gain,
            crossover_freq,
        } = &effect.effect_type
        {
            let bass_db = bass_gain.evaluate(frame);
            let treble_db = treble_gain.evaluate(frame);
            let crossover = crossover_freq.evaluate(frame).clamp(80.0, 12000.0);
            let bands = vec![
                audio_dsp::EqBand {
                    freq: crossover,
                    gain_db: bass_db,
                    q: 0.707,
                    band_type: audio_dsp::EqBandType::LowShelf,
                },
                audio_dsp::EqBand {
                    freq: crossover,
                    gain_db: treble_db,
                    q: 0.707,
                    band_type: audio_dsp::EqBandType::HighShelf,
                },
            ];
            let key = (instance_id.to_owned(), layer.id.clone(), effect_index);
            let (last_frame, state) = states
                .entry(key)
                .or_insert_with(|| (state_frame.saturating_sub(1), audio_dsp::EqState::default()));
            if last_frame.checked_add(1) != Some(state_frame) {
                state.reset();
            }
            audio_dsp::apply_eq_with_state(samples, &bands, sample_rate as u32, state);
            *last_frame = state_frame;
        }
    }
}

// ── Multi-track mixing: real audio sources ──────────────────────────────────

/// Returns whether a composition or any nested precomposition contains an
/// audio-bearing layer. This is a structural check; it does not decode media.
pub fn composition_contains_audio_sources(comp: &Composition) -> bool {
    fn visit(comp: &Composition, depth: u32) -> bool {
        if depth > 32 {
            return false;
        }
        comp.layers.iter().any(|layer| match &layer.layer_type {
            LayerType::Audio { .. } => true,
            LayerType::Video {
                audio_wav: Some(_), ..
            } => true,
            LayerType::PreComp { comp_id, .. } => comp
                .find_sub_comp(comp_id)
                .is_some_and(|sub| visit(sub, depth + 1)),
            _ => false,
        })
    }

    visit(comp, 0)
}

/// Recursively mix audio from a sub-composition (PreComp) into the stereo output.
#[allow(clippy::too_many_arguments)]
fn mix_precomp_audio(
    sub_comp: &crate::core::timeline::Composition,
    sub_frame: f32,
    depth: u32,
    stereo_output: &mut [f32],
    sample_rate: u32,
    buffer_size: usize,
    gain: f32,
    pan: f32,
    fps: f32,
    source_frame_rate: f32,
    output_frame: u32,
    instance_id: &str,
    layer_eq: &mut std::collections::HashMap<
        (String, String, usize),
        (u32, crate::core::audio_dsp::EqState),
    >,
    source_errors: &mut Vec<AudioSourceError>,
    loaded_sources: &mut usize,
) {
    if depth >= crate::core::software_renderer::MAX_PRECOMP_DEPTH {
        return;
    }
    for layer in &sub_comp.layers {
        let evaluated_frame = sub_frame.floor().max(0.0) as u32;
        if !layer.is_active(evaluated_frame) || !layer.visible || layer.audio_muted {
            continue;
        }
        match &layer.layer_type {
            crate::core::timeline::LayerType::PreComp { comp_id, .. } => {
                if let Some(sub) = sub_comp.find_sub_comp(comp_id) {
                    let mapped_start = layer.remap_frame_at_time(sub_frame);
                    let mapped_end = layer.remap_frame_at_time(sub_frame + 1.0);
                    let nested_instance_id = format!("{instance_id}/{}/{}", sub.id, layer.id);
                    let mut nested_output = vec![0.0f32; buffer_size * 2];
                    mix_precomp_audio(
                        sub,
                        mapped_start,
                        depth + 1,
                        &mut nested_output,
                        sample_rate,
                        buffer_size,
                        1.0,
                        0.0,
                        fps,
                        source_frame_rate * (mapped_end - mapped_start),
                        output_frame,
                        &nested_instance_id,
                        layer_eq,
                        source_errors,
                        loaded_sources,
                    );
                    apply_layer_audio_effects_with_state(
                        &mut nested_output,
                        layer,
                        sample_rate as f32,
                        evaluated_frame,
                        output_frame,
                        instance_id,
                        layer_eq,
                    );
                    let gl = gain * (1.0 - pan.max(0.0));
                    let gr = gain * (1.0 - (-pan).max(0.0));
                    for i in 0..buffer_size {
                        let l_idx = i * 2;
                        stereo_output[l_idx] += nested_output[l_idx] * gl;
                        stereo_output[l_idx + 1] += nested_output[l_idx + 1] * gr;
                    }
                }
            }
            crate::core::timeline::LayerType::Audio { path, volume } => {
                let vol_db = volume.value_at_f32(sub_frame);
                let layer_gain = gain * 10.0f32.powf(vol_db / 20.0);
                let (time_start, sample_rate_scale) = audio_source_timing(
                    layer,
                    sub_frame,
                    sub_comp.fps.max(1) as f32,
                    fps,
                    source_frame_rate,
                );
                let source = load_cached_audio(path, sample_rate);
                if let Ok(buf) = source {
                    *loaded_sources = loaded_sources.saturating_add(1);
                    let mut layer_samples = vec![0.0f32; buffer_size * 2];
                    for i in 0..buffer_size {
                        let t = time_start + i as f64 / f64::from(sample_rate) * sample_rate_scale;
                        let (l, r) = sample_audio_at_time(&buf, t);
                        let l_idx = i * 2;
                        layer_samples[l_idx] = l;
                        layer_samples[l_idx + 1] = r;
                    }
                    apply_layer_audio_effects_with_state(
                        &mut layer_samples,
                        layer,
                        sample_rate as f32,
                        evaluated_frame,
                        output_frame,
                        instance_id,
                        layer_eq,
                    );
                    let gl = layer_gain * (1.0 - pan.max(0.0));
                    let gr = layer_gain * (1.0 - (-pan).max(0.0));
                    for i in 0..buffer_size {
                        let l_idx = i * 2;
                        stereo_output[l_idx] += layer_samples[l_idx] * gl;
                        stereo_output[l_idx + 1] += layer_samples[l_idx + 1] * gr;
                    }
                } else if let Err(reason) = source {
                    source_errors.push(AudioSourceError {
                        layer: layer.name.clone(),
                        path: path.clone(),
                        reason,
                    });
                }
            }
            crate::core::timeline::LayerType::Video {
                frames_dir,
                audio_wav: Some(w),
                audio_volume,
                speed,
                ..
            } => {
                let (time_start, sample_rate_scale) = video_audio_source_timing(
                    layer,
                    sub_frame,
                    frames_dir,
                    sub_comp.fps.max(1) as f32,
                    fps,
                    source_frame_rate,
                    *speed,
                );
                let layer_gain = gain * 10.0f32.powf(audio_volume.value_at_f32(sub_frame) / 20.0);
                let source = load_cached_audio(w, sample_rate);
                if let Ok(buf) = source {
                    *loaded_sources = loaded_sources.saturating_add(1);
                    let mut layer_samples = vec![0.0f32; buffer_size * 2];
                    for i in 0..buffer_size {
                        let t = time_start + i as f64 / f64::from(sample_rate) * sample_rate_scale;
                        let (l, r) = sample_audio_at_time(&buf, t);
                        let l_idx = i * 2;
                        layer_samples[l_idx] = l;
                        layer_samples[l_idx + 1] = r;
                    }
                    apply_layer_audio_effects_with_state(
                        &mut layer_samples,
                        layer,
                        sample_rate as f32,
                        evaluated_frame,
                        output_frame,
                        instance_id,
                        layer_eq,
                    );
                    let gl = layer_gain * (1.0 - pan.max(0.0));
                    let gr = layer_gain * (1.0 - (-pan).max(0.0));
                    for i in 0..buffer_size {
                        let l_idx = i * 2;
                        stereo_output[l_idx] += layer_samples[l_idx] * gl;
                        stereo_output[l_idx + 1] += layer_samples[l_idx + 1] * gr;
                    }
                } else if let Err(reason) = source {
                    source_errors.push(AudioSourceError {
                        layer: layer.name.clone(),
                        path: w.clone(),
                        reason,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Mixes ALL audio-carrying layers (Audio layers + Video layers with extracted audio)
/// at the given frame into a stereo buffer, respecting per-layer volume and
/// active ranges. This is the real backing for the audio mixer UI.
///
/// Audio files are decoded once and cached per path in a process-local map.
pub fn mix_audio_sources_for_frame(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
    // Optional per-layer mixer overrides indexed by layer order
    mixer: Option<&[crate::core::audio_types::MixerChannel]>,
    dsp: &MasterDspParams,
) -> (Vec<f32>, AudioFrameMeter) {
    let mut state = MasterDspState::default();
    mix_audio_sources_for_frame_with_state(
        comp,
        frame,
        sample_rate,
        buffer_size,
        mixer,
        dsp,
        &mut state,
    )
}

pub fn mix_audio_sources_for_frame_with_state(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
    mixer: Option<&[crate::core::audio_types::MixerChannel]>,
    dsp: &MasterDspParams,
    dsp_state: &mut MasterDspState,
) -> (Vec<f32>, AudioFrameMeter) {
    if dsp_state
        .last_frame
        .is_some_and(|last_frame| last_frame.checked_add(1) != Some(frame))
        || dsp_state
            .composition_id
            .as_deref()
            .is_some_and(|composition_id| composition_id != comp.id)
        || dsp_state
            .sample_rate
            .is_some_and(|previous_rate| previous_rate != sample_rate)
    {
        *dsp_state = MasterDspState::default();
    }
    dsp_state.last_frame = Some(frame);
    dsp_state.composition_id = Some(comp.id.clone());
    dsp_state.sample_rate = Some(sample_rate);

    let mut stereo_output = vec![0.0f32; buffer_size * 2];
    let mut sum_sq_l = 0.0f32;
    let mut sum_sq_r = 0.0f32;
    let mut max_peak_l = 0.0f32;
    let mut max_peak_r = 0.0f32;
    let mut track_peaks = vec![(0.0f32, 0.0f32); comp.layers.len()];
    let mut source_errors = Vec::new();
    let mut loaded_sources = 0usize;
    let fps = comp.fps.max(1) as f32;

    for (layer_idx, layer) in comp.layers.iter().enumerate() {
        if !layer.is_active(frame) || !layer.visible || layer.audio_muted {
            continue;
        }
        // Handle PreComp layers: recursively mix sub-composition audio
        if let LayerType::PreComp { comp_id, .. } = &layer.layer_type {
            if let Some(sub) = comp.find_sub_comp(comp_id) {
                let mapped_start = layer.remap_frame_at_time(frame as f32);
                let mapped_end = layer.remap_frame_at_time(frame as f32 + 1.0);
                let mut sub_gain = 1.0f32;
                let mut sub_pan = 0.0f32;
                if let Some(mix) = mixer {
                    if let Some(ch) = mix.get(layer_idx) {
                        sub_gain *= 10.0f32.powf(ch.gain_db / 20.0);
                        sub_pan = (ch.pan / 100.0).clamp(-1.0, 1.0);
                        if ch.mute {
                            sub_gain = 0.0;
                        }
                    }
                }
                if sub_gain > 0.0 {
                    let mut sub_output = vec![0.0f32; buffer_size * 2];
                    let sub_instance_id = format!("{}/{}/{}", comp.id, sub.id, layer.id);
                    mix_precomp_audio(
                        sub,
                        mapped_start,
                        0,
                        &mut sub_output,
                        sample_rate,
                        buffer_size,
                        1.0,
                        0.0,
                        fps,
                        mapped_end - mapped_start,
                        frame,
                        &sub_instance_id,
                        &mut dsp_state.layer_eq,
                        &mut source_errors,
                        &mut loaded_sources,
                    );
                    apply_layer_audio_effects_with_state(
                        &mut sub_output,
                        layer,
                        sample_rate as f32,
                        frame,
                        frame,
                        &comp.id,
                        &mut dsp_state.layer_eq,
                    );
                    let gl = sub_gain * (1.0 - sub_pan.max(0.0));
                    let gr = sub_gain * (1.0 - (-sub_pan).max(0.0));
                    for i in 0..buffer_size {
                        let l_idx = i * 2;
                        stereo_output[l_idx] += sub_output[l_idx] * gl;
                        stereo_output[l_idx + 1] += sub_output[l_idx + 1] * gr;
                    }
                }
            }
            continue;
        }
        // Resolve the WAV path + gain for this layer
        let (wav_path, gain_db, playback_speed, video_frames_dir) = match &layer.layer_type {
            LayerType::Audio { path, volume } => {
                (Some(path.clone()), volume.evaluate(frame), 1.0, None)
            }
            LayerType::Video {
                frames_dir,
                audio_wav: Some(w),
                audio_volume,
                speed,
                ..
            } => (
                Some(w.clone()),
                audio_volume.evaluate(frame),
                if speed.is_finite() {
                    speed.max(0.0)
                } else {
                    0.0
                },
                Some(frames_dir.as_str()),
            ),
            _ => continue,
        };
        let mut gain = 10.0f32.powf(gain_db / 20.0);
        let mut pan = 0.0f32;
        let mut is_muted = false;
        if let Some(mix) = mixer {
            if let Some(ch) = mix.get(layer_idx) {
                gain *= 10.0f32.powf(ch.gain_db / 20.0);
                pan = (ch.pan / 100.0).clamp(-1.0, 1.0);
                is_muted = ch.mute;
            }
        }
        // Solo logic: if any channel in the mixer is soloed, mute non-soloed channels
        if let Some(mix) = mixer {
            let any_soloed = mix.iter().any(|ch| ch.solo);
            if any_soloed {
                let is_soloed = mix.get(layer_idx).is_some_and(|ch| ch.solo);
                if !is_soloed {
                    is_muted = true;
                }
            }
        }
        if is_muted {
            gain = 0.0;
        }
        let (time_start, sample_rate_scale) = if let Some(frames_dir) = video_frames_dir {
            video_audio_source_timing(
                layer,
                frame as f32,
                frames_dir,
                fps,
                fps,
                1.0,
                playback_speed,
            )
        } else {
            audio_source_timing(layer, frame as f32, fps, fps, 1.0)
        };

        // Source samples: decoded audio if present, otherwise silent placeholder
        let source: Option<std::sync::Arc<AudioBuffer>> = match &wav_path {
            Some(path) => match load_cached_audio(path, sample_rate) {
                Ok(buffer) => {
                    loaded_sources = loaded_sources.saturating_add(1);
                    Some(buffer)
                }
                Err(reason) => {
                    source_errors.push(AudioSourceError {
                        layer: layer.name.clone(),
                        path: path.clone(),
                        reason,
                    });
                    None
                }
            },
            None => None,
        };

        let mut layer_samples = vec![0.0f32; buffer_size * 2];
        for i in 0..buffer_size {
            let t = time_start + i as f64 / f64::from(sample_rate) * sample_rate_scale;
            let (sample_l, sample_r) = match &source {
                Some(buf) => sample_audio_at_time(buf, t),
                None => (0.0, 0.0),
            };
            let l_idx = i * 2;
            let r_idx = i * 2 + 1;
            layer_samples[l_idx] = sample_l;
            layer_samples[r_idx] = sample_r;
        }
        apply_layer_audio_effects_with_state(
            &mut layer_samples,
            layer,
            sample_rate as f32,
            frame,
            frame,
            &comp.id,
            &mut dsp_state.layer_eq,
        );
        let mut track_peak_l = 0.0f32;
        let mut track_peak_r = 0.0f32;
        for i in 0..buffer_size {
            let l_idx = i * 2;
            let r_idx = i * 2 + 1;
            let gl = gain * (1.0 - pan.max(0.0));
            let gr = gain * (1.0 - (-pan).max(0.0));
            let layer_l = layer_samples[l_idx] * gl;
            let layer_r = layer_samples[r_idx] * gr;
            track_peak_l = track_peak_l.max(layer_l.abs());
            track_peak_r = track_peak_r.max(layer_r.abs());
            stereo_output[l_idx] += layer_l;
            stereo_output[r_idx] += layer_r;
        }
        if let Some(track_peak) = track_peaks.get_mut(layer_idx) {
            *track_peak = (track_peak_l, track_peak_r);
        }
    }

    // ── Master DSP processing (auto gain → EQ → compressor → limiter) ──
    {
        use crate::core::audio_dsp;
        let wet_dry = if dsp.wet_dry.is_finite() {
            dsp.wet_dry.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let dry_output = (wet_dry < 1.0).then(|| stereo_output.clone());
        if dsp.auto_gain_enabled {
            audio_dsp::apply_auto_gain_with_state(
                &mut stereo_output,
                dsp.auto_gain_target_db,
                sample_rate,
                &mut dsp_state.auto_gain,
            );
        }
        let master_eq = vec![
            audio_dsp::EqBand {
                freq: dsp.eq_highpass,
                gain_db: 0.0,
                q: 0.707,
                band_type: audio_dsp::EqBandType::HighPass,
            },
            audio_dsp::EqBand {
                freq: dsp.eq_lowpass,
                gain_db: 0.0,
                q: 0.707,
                band_type: audio_dsp::EqBandType::LowPass,
            },
            audio_dsp::EqBand {
                freq: dsp.eq_mid_freq,
                gain_db: dsp.eq_mid_gain,
                q: 1.0,
                band_type: audio_dsp::EqBandType::Bell,
            },
        ];
        audio_dsp::apply_eq_with_state(
            &mut stereo_output,
            &master_eq,
            sample_rate,
            &mut dsp_state.eq,
        );

        let comp_params = audio_dsp::CompressorParams {
            threshold_db: dsp.comp_threshold,
            ratio: dsp.comp_ratio,
            attack_ms: dsp.comp_attack,
            release_ms: dsp.comp_release,
            knee_db: 6.0,
            makeup_gain_db: dsp.comp_makeup,
        };
        audio_dsp::apply_compressor(
            &mut stereo_output,
            &comp_params,
            &mut dsp_state.compressor,
            sample_rate,
        );

        if let Some(dry_output) = dry_output {
            for (processed, dry) in stereo_output.iter_mut().zip(dry_output) {
                *processed = *processed * wet_dry + dry * (1.0 - wet_dry);
            }
        }

        if dsp.limiter_enabled {
            audio_dsp::apply_limiter(
                &mut stereo_output,
                dsp.limiter_ceiling_db,
                50.0,
                &mut dsp_state.limiter,
                sample_rate,
            );
        } else {
            for sample in &mut stereo_output {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
            }
        }
    }

    // Meter the MIXED output, not per-layer contributions — otherwise the peak
    // reflects the loudest single source instead of the actual sum.
    for chunk in stereo_output.as_chunks::<2>().0 {
        let l = chunk[0];
        let r = chunk[1];
        sum_sq_l += l * l;
        sum_sq_r += r * r;
        max_peak_l = max_peak_l.max(l.abs());
        max_peak_r = max_peak_r.max(r.abs());
    }

    let rms_l = (sum_sq_l / buffer_size.max(1) as f32).sqrt();
    let rms_r = (sum_sq_r / buffer_size.max(1) as f32).sqrt();

    // Silence clamps to -120 dBFS (20·log10(1e-6)); the keyframe mapper and
    // meter displays treat anything below -90 dB as digital silence.
    let to_db = |v: f32| 20.0 * v.max(1e-6).log10();
    (
        stereo_output,
        AudioFrameMeter {
            peak_db_left: to_db(max_peak_l),
            peak_db_right: to_db(max_peak_r),
            rms_db_left: to_db(rms_l),
            rms_db_right: to_db(rms_r),
            track_peaks,
            source_errors,
            loaded_sources,
        },
    )
}

/// Energy-based onset (beat) detection over a supported audio file.
/// Returns comp-frame indices where transients occur, sorted & deduped.
pub fn detect_beat_frames(path: &std::path::Path, total_frames: u32, fps: f32) -> Vec<u32> {
    let Ok(buf) = AudioBuffer::load_audio(path) else {
        return Vec::new();
    };
    if buf.samples.is_empty() || total_frames == 0 || fps <= 0.0 {
        return Vec::new();
    }
    // ~10 ms hop energy envelope (mono mix)
    let hop = ((buf.sample_rate as f32 * 0.01) as usize).max(1);
    let ch = buf.channels.max(1) as usize;
    let mut energies: Vec<f32> = Vec::with_capacity(buf.samples.len() / hop + 1);
    let mut i = 0;
    while i < buf.samples.len() {
        let end = (i + hop * ch).min(buf.samples.len());
        let samples = &buf.samples[i..end];
        let mean_square = samples
            .iter()
            .map(|sample| {
                let sample = if sample.is_finite() {
                    f64::from(*sample)
                } else {
                    0.0
                };
                sample * sample
            })
            .sum::<f64>()
            / samples.len().max(1) as f64;
        energies.push(mean_square.sqrt() as f32);
        i += hop * ch;
    }
    if energies.len() < 3 {
        return Vec::new();
    }
    // Onset = positive energy flux; adaptive threshold from local mean
    let win = 30usize; // ~0.3 s
    let mut beats: Vec<u32> = Vec::new();
    for h in 1..energies.len() {
        let flux = energies[h] - energies[h - 1];
        if flux <= 0.0 {
            continue;
        }
        let lo = h.saturating_sub(win);
        let hi = (h + win).min(energies.len());
        let local_mean: f32 = energies[lo..hi].iter().sum::<f32>() / (hi - lo) as f32;
        if flux > local_mean * 1.5 && flux > 0.01 {
            let sec = h as f32 * 0.01;
            let frame = (sec * fps).round() as u32;
            if frame < total_frames && beats.last() != Some(&frame) {
                beats.push(frame);
            }
        }
    }
    beats
}
