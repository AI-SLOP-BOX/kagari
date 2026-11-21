//! PCM audio analysis across frame sequences for audio-reactive motion graphics.

use super::types::MultiBandAudioKeyframes;

/// Analyzes PCM audio buffer across entire frame range to generate multi-band keyframe animation data.
pub fn analyze_audio_multiband_keyframes(
    samples: &[f32],
    sample_rate: u32,
    fps: f32,
    total_frames: u32,
) -> Vec<MultiBandAudioKeyframes> {
    if samples.is_empty() || sample_rate == 0 || fps <= 0.0 || total_frames == 0 {
        return vec![];
    }

    let samples_per_frame = (sample_rate as f32 / fps.max(1.0)) as usize;
    let window_size = samples_per_frame.max(512);
    let mut keyframes = Vec::with_capacity(total_frames as usize);

    for f in 0..total_frames {
        let start_sample = (f as usize * samples_per_frame).min(samples.len());
        let end_sample = (start_sample + window_size).min(samples.len());

        if start_sample >= end_sample {
            keyframes.push(MultiBandAudioKeyframes {
                frame: f,
                bass_amplitude: 0.0,
                mid_amplitude: 0.0,
                treble_amplitude: 0.0,
                master_amplitude: 0.0,
            });
            continue;
        }

        let slice = &samples[start_sample..end_sample];

        // Overall Master RMS amplitude
        let mut sum_sq = 0.0f32;
        let mut bass_sum = 0.0f32;
        let mut treble_sum = 0.0f32;

        let mut prev = 0.0f32;
        for &s in slice {
            sum_sq += s * s;
            // High frequency differentiator approximation
            let diff = s - prev;
            treble_sum += diff * diff;
            // Low frequency integrator approximation
            let avg = (s + prev) * 0.5;
            bass_sum += avg * avg;
            prev = s;
        }

        let n = slice.len() as f32;
        let master = (sum_sq / n).sqrt().clamp(0.0, 1.0);
        let bass = (bass_sum / n).sqrt().clamp(0.0, 1.0);
        let treble = (treble_sum / n).sqrt().clamp(0.0, 1.0);
        let mid = (master * 1.5 - bass * 0.5 - treble * 0.5).clamp(0.0, 1.0);

        keyframes.push(MultiBandAudioKeyframes {
            frame: f,
            bass_amplitude: bass,
            mid_amplitude: mid,
            treble_amplitude: treble,
            master_amplitude: master,
        });
    }

    keyframes
}
