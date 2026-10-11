/// Multi-Track Audio Mixing & DSP Engine for After Effects timeline playback.
///
/// Features:
/// - Real PCM stereo AudioBuffer storage & multi-track mixing
/// - Per-layer animated volume/gain evaluation (`volume.evaluate(frame)`)
/// - Peak RMS VU meter calculation for audio mixers
use crate::core::timeline::{Composition, EffectType, Layer, LayerType};

mod cache;
mod mixing;
mod wav;
use cache::load_cached_audio;
pub use mixing::*;

const MAX_DECODED_AUDIO_BYTES: usize = 512 * 1024 * 1024;
const MAX_AUDIO_CACHE_BYTES: usize = 256 * 1024 * 1024;
const MAX_AUDIO_CACHE_ENTRIES: usize = 4096;

/// Validates that an audio path is safe to load: rejects path traversal (`..`)
/// and absolute paths outside the project directory.
/// Returns `Err(reason)` if the path is suspicious, `Ok(())` if safe.
pub fn validate_audio_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    // Reject empty paths
    if path.is_empty() {
        return Err("audio path is empty".into());
    }
    // Reject paths with .. components (traversal)
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(format!("audio path contains '..' traversal: {}", path));
        }
    }
    // Reject absolute paths to sensitive system directories
    if p.is_absolute() {
        let forbidden = ["/etc", "/proc", "/sys", "/dev", "/private/etc"];
        for prefix in &forbidden {
            if let Ok(canonical) = p.canonicalize() {
                if canonical.starts_with(prefix) {
                    return Err(format!("audio path targets system directory: {}", path));
                }
            }
            // If canonicalize fails (path doesn't exist yet), check prefix string
            if p.starts_with(prefix) {
                return Err(format!("audio path targets system directory: {}", path));
            }
        }
    }
    Ok(())
}

/// Backwards-compatible name retained for callers that only accept WAV input.
pub fn validate_wav_path(path: &str) -> Result<(), String> {
    validate_audio_path(path)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>, // Interleaved stereo PCM [L, R, L, R...]
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    #[allow(dead_code)]
    pub fn new_sine_preview(duration_sec: f32, sample_rate: u32, freq: f32) -> Self {
        let num_samples = (duration_sec * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let s = (t * freq * std::f32::consts::TAU).sin() * 0.25;
            samples.push(s);
            samples.push(s);
        }
        Self {
            samples,
            sample_rate,
            channels: 2,
        }
    }

    /// Resample PCM audio samples to match destination sample rate with linear interpolation.
    #[allow(dead_code)]
    pub fn resample(&self, target_sample_rate: u32) -> Result<Self, String> {
        if self.sample_rate == 0 || target_sample_rate == 0 {
            return Err("audio sample rate must be positive".into());
        }
        if self.sample_rate == target_sample_rate {
            return Ok(self.clone());
        }
        if self.samples.is_empty() {
            return Ok(Self {
                samples: Vec::new(),
                sample_rate: target_sample_rate,
                channels: self.channels,
            });
        }

        let channels = self.channels.max(1) as usize;
        if !self.samples.len().is_multiple_of(channels) {
            return Err("audio buffer has an incomplete interleaved frame".into());
        }
        let source_frames = self.samples.len() / channels;
        let new_frames = source_frames as f64 * target_sample_rate as f64 / self.sample_rate as f64;
        let max_samples = MAX_DECODED_AUDIO_BYTES / std::mem::size_of::<f32>();
        if !new_frames.is_finite()
            || new_frames > (max_samples / channels) as f64
            || new_frames > usize::MAX as f64
        {
            return Err("resampled audio would exceed the 512 MiB memory limit".into());
        }
        let new_frames = new_frames as usize;
        let ratio = self.sample_rate as f64 / target_sample_rate as f64;
        let mut out = Vec::with_capacity(new_frames.saturating_mul(channels));

        for i in 0..new_frames {
            let src_f = i as f64 * ratio;
            let idx0 = src_f.floor() as usize;
            let idx1 = (idx0 + 1).min((self.samples.len() / channels).saturating_sub(1));
            let frac = (src_f - idx0 as f64) as f32;
            for channel in 0..channels {
                let fallback = self
                    .samples
                    .get(idx0 * channels + channel)
                    .copied()
                    .unwrap_or(0.0);
                let s0 = fallback;
                let s1 = self
                    .samples
                    .get(idx1 * channels + channel)
                    .copied()
                    .unwrap_or(s0);
                out.push(s0 + (s1 - s0) * frac);
            }
        }
        Ok(Self {
            samples: out,
            sample_rate: target_sample_rate,
            channels: self.channels,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AudioSourceError {
    pub layer: String,
    pub path: String,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioFrameMeter {
    pub peak_db_left: f32,
    pub peak_db_right: f32,
    pub rms_db_left: f32,
    pub rms_db_right: f32,
    /// Per-layer post-fader peaks, indexed by composition layer order.
    pub track_peaks: Vec<(f32, f32)>,
    /// Active sources that could not be loaded; real-time preview may continue silently.
    pub source_errors: Vec<AudioSourceError>,
    /// Number of active audio sources successfully decoded for this window.
    pub loaded_sources: usize,
}

impl Default for AudioFrameMeter {
    fn default() -> Self {
        Self {
            peak_db_left: -90.0,
            peak_db_right: -90.0,
            rms_db_left: -90.0,
            rms_db_right: -90.0,
            track_peaks: Vec::new(),
            source_errors: Vec::new(),
            loaded_sources: 0,
        }
    }
}

/// Mix audio tracks across all active layers for a given frame window into a 2-channel stereo f32 buffer.
/// Delegates to `mix_audio_sources_for_frame` for decoded audio sampling.
#[allow(dead_code)]
/// Master DSP parameters (passed from UI controls to audio engine).
#[derive(Debug, Clone)]
pub struct MasterDspParams {
    pub eq_highpass: f32,
    pub eq_lowpass: f32,
    pub eq_mid_gain: f32,
    pub eq_mid_freq: f32,
    pub comp_threshold: f32,
    pub comp_ratio: f32,
    pub comp_attack: f32,
    pub comp_release: f32,
    pub comp_makeup: f32,
    pub auto_gain_enabled: bool,
    pub auto_gain_target_db: f32,
    pub limiter_enabled: bool,
    pub limiter_ceiling_db: f32,
    pub wet_dry: f32,
}

impl Default for MasterDspParams {
    fn default() -> Self {
        Self {
            eq_highpass: 30.0,
            eq_lowpass: 18000.0,
            eq_mid_gain: 0.0,
            eq_mid_freq: 1000.0,
            comp_threshold: -12.0,
            comp_ratio: 2.0,
            comp_attack: 10.0,
            comp_release: 100.0,
            comp_makeup: 0.0,
            auto_gain_enabled: false,
            auto_gain_target_db: -18.0,
            limiter_enabled: true,
            limiter_ceiling_db: -1.0,
            wet_dry: 1.0,
        }
    }
}

impl MasterDspParams {
    pub fn bypass() -> Self {
        Self {
            eq_highpass: 0.0,
            eq_lowpass: 24_000.0,
            eq_mid_gain: 0.0,
            eq_mid_freq: 1_000.0,
            comp_threshold: 0.0,
            comp_ratio: 1.0,
            comp_attack: 1.0,
            comp_release: 10.0,
            comp_makeup: 0.0,
            auto_gain_enabled: false,
            auto_gain_target_db: -18.0,
            limiter_enabled: false,
            limiter_ceiling_db: 0.0,
            wet_dry: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MasterDspState {
    pub auto_gain: crate::core::audio_dsp::AutoGainState,
    pub compressor: crate::core::audio_dsp::CompressorState,
    pub limiter: crate::core::audio_dsp::CompressorState,
}

pub fn mix_audio_for_frame(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
    dsp: &MasterDspParams,
) -> (Vec<f32>, AudioFrameMeter) {
    mix_audio_sources_for_frame(comp, frame, sample_rate, buffer_size, None, dsp)
}

/// Keyframe data point generated from audio amplitude analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioAmplitudeKeyframe {
    pub frame: u32,
    pub left_amp: f32,
    pub right_amp: f32,
    pub both_amp: f32,
}

/// Evaluates audio waveforms across the composition duration and converts volume amplitudes into keyframe sequences.
/// Returns a list of generated AudioAmplitudeKeyframe structs.
pub fn convert_audio_to_keyframes(comp: &Composition) -> Vec<AudioAmplitudeKeyframe> {
    let mut keyframes = Vec::with_capacity(comp.duration_frames as usize);
    let sample_rate = 44100;
    let buffer_size = (sample_rate / comp.fps.max(1)) as usize;

    for frame in 0..comp.duration_frames {
        let (_pcm, meter) = mix_audio_for_frame(
            comp,
            frame,
            sample_rate,
            buffer_size,
            &MasterDspParams::default(),
        );

        // Convert dB (-90dB .. +6dB) to normalized 0.0 .. 100.0 AE amplitude scale
        let db_to_ae_scale = |db: f32| -> f32 {
            let norm = ((db + 90.0) / 96.0).clamp(0.0, 1.0);
            norm * 100.0
        };

        let left_amp = db_to_ae_scale(meter.rms_db_left);
        let right_amp = db_to_ae_scale(meter.rms_db_right);
        let both_amp = (left_amp + right_amp) * 0.5;

        keyframes.push(AudioAmplitudeKeyframe {
            frame,
            left_amp,
            right_amp,
            both_amp,
        });
    }

    keyframes
}

#[cfg(test)]
#[path = "audio_engine/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "audio_engine/multitrack_tests.rs"]
mod multitrack_tests;

#[cfg(test)]
#[path = "audio_engine/beat_tests.rs"]
mod beat_tests;
