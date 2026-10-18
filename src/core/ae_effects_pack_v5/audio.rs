#![allow(dead_code)]

// 141. Audio Spectrum FX
pub fn apply_audio_spectrum_fx(pixels: &mut [u8], width: u32, height: u32) {
    let opts = crate::core::audio_spectrum::AudioSpectrumOptions::default();
    let bands = crate::core::audio_spectrum::generate_audio_spectrum_bands(
        &[0.1, 0.5, 0.8, 0.3, 0.9, 0.4],
        44100,
        &opts,
    );
    crate::core::ae_effects_pack_v2::apply_grid(
        pixels,
        width,
        height,
        10,
        bands.len() as u32,
        [0, 255, 200, 255],
    );
}

// 142. Audio Waveform Display
pub fn apply_audio_waveform_fx(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v4::apply_audio_waveforms(
        pixels,
        width,
        height,
        [0, 255, 150, 255],
    );
}

// 143. Tone Generator
pub fn apply_tone_generator(samples: &mut [f32], sample_rate: u32, freq_hz: f32) {
    let dt = 1.0 / sample_rate as f32;
    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 * dt;
        *sample = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.5;
    }
}

// 144. Stereo Mixer
pub fn apply_stereo_mixer(left: &mut [f32], right: &mut [f32], pan: f32) {
    let p = pan.clamp(-1.0, 1.0);
    let l_gain = (1.0 - p) * 0.5;
    let r_gain = (1.0 + p) * 0.5;
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
        *l *= l_gain;
        *r *= r_gain;
    }
}

// 145. High Pass Filter
pub fn apply_high_pass_filter(samples: &mut [f32]) {
    let mut prev = 0.0f32;
    for s in samples.iter_mut() {
        let curr = *s;
        *s = curr - prev;
        prev = curr;
    }
}

// 146. Low Pass Filter
pub fn apply_low_pass_filter(samples: &mut [f32]) {
    let mut prev = 0.0f32;
    for s in samples.iter_mut() {
        let curr = *s;
        *s = (curr + prev) * 0.5;
        prev = curr;
    }
}

// 147. Parametric EQ
pub fn apply_parametric_eq(samples: &mut [f32], gain: f32) {
    for s in samples.iter_mut() {
        *s *= gain;
    }
}

// 148. Reverb Effect
pub fn apply_reverb_effect(samples: &mut [f32], delay_samples: usize, decay: f32) {
    if delay_samples == 0 || delay_samples >= samples.len() {
        return;
    }
    for i in delay_samples..samples.len() {
        samples[i] += samples[i - delay_samples] * decay;
    }
}

// 149. Delay Effect
pub fn apply_delay_effect(samples: &mut [f32], delay_samples: usize, feedback: f32) {
    apply_reverb_effect(samples, delay_samples, feedback);
}

// 150. Pitch Shifter
pub fn apply_pitch_shifter(samples: &mut [f32], pitch_ratio: f32) {
    if pitch_ratio <= 0.01 {
        return;
    }
    let temp = samples.to_vec();
    for (i, sample) in samples.iter_mut().enumerate() {
        let src_idx = ((i as f32 * pitch_ratio) as usize).min(temp.len() - 1);
        *sample = temp[src_idx];
    }
}

// 161. Bass/Treble two-band EQ
pub fn apply_bass_treble(
    samples: &mut [f32],
    sample_rate: f32,
    bass_db: f32,
    treble_db: f32,
    crossover_hz: f32,
) {
    if samples.len() < 2 {
        return;
    }
    let bass_linear = 10.0f32.powf(bass_db * 0.05);
    let treble_linear = 10.0f32.powf(treble_db * 0.05);
    let rc = 1.0 / (std::f32::consts::PI * crossover_hz);
    let dt = 1.0 / sample_rate;
    let alpha = dt / (rc + dt);
    let mut prev_low = samples[0];
    for s in samples.iter_mut() {
        let low = prev_low + alpha * (*s - prev_low);
        let high = *s - low;
        prev_low = low;
        *s = low * bass_linear + high * treble_linear;
    }
}

// 162. Flanger (modulated delay with LFO)
pub fn apply_flanger(
    samples: &mut [f32],
    sample_rate: f32,
    max_delay_ms: f32,
    rate_hz: f32,
    feedback: f32,
    wet_dry: f32,
) {
    let max_delay_samples = (max_delay_ms * 0.001 * sample_rate) as usize;
    if max_delay_samples == 0 || samples.len() <= max_delay_samples {
        return;
    }
    let mut delay_buf = vec![0.0f32; max_delay_samples + 1];
    let wet = wet_dry.clamp(0.0, 1.0);
    let dry = 1.0 - wet;
    for (i, s) in samples.iter_mut().enumerate() {
        let lfo =
            ((2.0 * std::f32::consts::PI * rate_hz * i as f32 / sample_rate).sin() + 1.0) * 0.5;
        let delay_idx = (lfo * max_delay_samples as f32) as usize;
        let delay_idx = delay_idx.min(max_delay_samples);
        let delayed = if i >= delay_idx {
            delay_buf[(i - delay_idx) % (max_delay_samples + 1)]
        } else {
            0.0
        };
        let output = *s * dry + delayed * wet;
        delay_buf[i % (max_delay_samples + 1)] = *s + delayed * feedback;
        *s = output;
    }
}

// 163. Chorus (multiple detuned delays)
pub fn apply_chorus(
    samples: &mut [f32],
    sample_rate: f32,
    delay_ms: f32,
    depth_ms: f32,
    rate_hz: f32,
    voices: f32,
    feedback: f32,
) {
    let base_delay = (delay_ms * 0.001 * sample_rate) as usize;
    let depth = (depth_ms * 0.001 * sample_rate) as usize;
    let n_voices = (voices as u32).clamp(2, 8);
    if base_delay + depth == 0 || samples.len() <= base_delay + depth {
        return;
    }
    let buf_size = base_delay + depth + 1;
    let mut delay_buf = vec![0.0f32; buf_size];
    let wet = 1.0 / n_voices as f32;
    let dry = 1.0 - wet * 0.5;
    for (i, s) in samples.iter_mut().enumerate() {
        let mut out = *s * dry;
        for v in 0..n_voices {
            let phase = v as f32 / n_voices as f32;
            let lfo = ((2.0 * std::f32::consts::PI * rate_hz * i as f32 / sample_rate
                + phase * 2.0 * std::f32::consts::PI)
                .sin()
                + 1.0)
                * 0.5;
            let d = base_delay + (lfo * depth as f32) as usize;
            let d = d.min(buf_size - 1);
            let delayed = if i >= d {
                delay_buf[(i - d) % buf_size]
            } else {
                0.0
            };
            out += delayed * wet;
        }
        delay_buf[i % buf_size] = *s + *s * feedback;
        *s = out;
    }
}

// 164. Parametric EQ (single-band bell)
pub fn apply_parametric_eq_bell(
    samples: &mut [f32],
    sample_rate: f32,
    freq_hz: f32,
    gain_db: f32,
    q: f32,
) {
    if samples.len() < 3 {
        return;
    }
    let a = 10.0f32.powf(gain_db * 0.05);
    let w0 = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * w0.cos();
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha / a;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for s in samples.iter_mut() {
        let x0 = *s;
        let y0 = (b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
        *s = y0;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
    }
}
