//! Audio-domain value types shared by the mixer, renderer, and UI adapters.

use serde::{Deserialize, Serialize};

/// Non-destructive, CPU-only voice enhancement settings stored with a
/// production document. The defaults are intentionally conservative so old
/// projects keep their existing sound when opened.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioCorrectionSettings {
    pub enabled: bool,
    pub auto_gain: bool,
    pub target_level_db: f32,
    pub highpass_hz: f32,
    pub lowpass_hz: f32,
    pub presence_gain_db: f32,
    pub presence_freq_hz: f32,
    pub compressor_threshold_db: f32,
    pub compressor_ratio: f32,
    pub compressor_attack_ms: f32,
    pub compressor_release_ms: f32,
    pub compressor_makeup_db: f32,
    pub limiter_enabled: bool,
    pub limiter_ceiling_db: f32,
    pub wet_dry: f32,
}

impl Default for AudioCorrectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_gain: false,
            target_level_db: -18.0,
            highpass_hz: 30.0,
            lowpass_hz: 18_000.0,
            presence_gain_db: 0.0,
            presence_freq_hz: 1_000.0,
            compressor_threshold_db: -12.0,
            compressor_ratio: 2.0,
            compressor_attack_ms: 10.0,
            compressor_release_ms: 100.0,
            compressor_makeup_db: 0.0,
            limiter_enabled: true,
            limiter_ceiling_db: -1.0,
            wet_dry: 1.0,
        }
    }
}

impl AudioCorrectionSettings {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.target_level_db.is_finite() || !(-60.0..=0.0).contains(&self.target_level_db) {
            return Err("audio correction target level is invalid");
        }
        if !self.highpass_hz.is_finite() || !(0.0..=20_000.0).contains(&self.highpass_hz) {
            return Err("audio correction high-pass frequency is invalid");
        }
        if !self.lowpass_hz.is_finite() || !(20.0..=24_000.0).contains(&self.lowpass_hz) {
            return Err("audio correction low-pass frequency is invalid");
        }
        if self.lowpass_hz <= self.highpass_hz {
            return Err("audio correction low-pass must be above high-pass");
        }
        if !self.presence_gain_db.is_finite() || !(-24.0..=24.0).contains(&self.presence_gain_db) {
            return Err("audio correction presence gain is invalid");
        }
        if !self.presence_freq_hz.is_finite() || !(60.0..=18_000.0).contains(&self.presence_freq_hz)
        {
            return Err("audio correction presence frequency is invalid");
        }
        if !self.compressor_threshold_db.is_finite()
            || !(-60.0..=0.0).contains(&self.compressor_threshold_db)
        {
            return Err("audio correction compressor threshold is invalid");
        }
        if !self.compressor_ratio.is_finite() || !(1.0..=20.0).contains(&self.compressor_ratio) {
            return Err("audio correction compressor ratio is invalid");
        }
        if !self.compressor_attack_ms.is_finite()
            || !(0.1..=200.0).contains(&self.compressor_attack_ms)
        {
            return Err("audio correction compressor attack is invalid");
        }
        if !self.compressor_release_ms.is_finite()
            || !(1.0..=2_000.0).contains(&self.compressor_release_ms)
        {
            return Err("audio correction compressor release is invalid");
        }
        if !self.compressor_makeup_db.is_finite()
            || !(-24.0..=24.0).contains(&self.compressor_makeup_db)
        {
            return Err("audio correction compressor makeup is invalid");
        }
        if !self.limiter_ceiling_db.is_finite() || !(-12.0..=0.0).contains(&self.limiter_ceiling_db)
        {
            return Err("audio correction limiter ceiling is invalid");
        }
        if !self.wet_dry.is_finite() || !(0.0..=1.0).contains(&self.wet_dry) {
            return Err("audio correction mix is invalid");
        }
        Ok(())
    }
}

/// Per-channel mixer controls. This type intentionally lives in Core so
/// headless rendering does not depend on application state or egui.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MixerChannel {
    pub gain_db: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

impl MixerChannel {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.gain_db.is_finite() || self.gain_db < -144.0 || self.gain_db > 24.0 {
            return Err("mixer gain must be finite and within -144..=24 dB");
        }
        if !self.pan.is_finite() || !(-100.0..=100.0).contains(&self.pan) {
            return Err("mixer pan must be finite and within -100..=100");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_channel_accepts_normal_values() {
        assert!(MixerChannel::default().validate().is_ok());
        assert!(MixerChannel {
            gain_db: 6.0,
            pan: -1.0,
            ..Default::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn mixer_channel_rejects_non_finite_and_extreme_values() {
        for gain_db in [f32::NAN, f32::INFINITY, -145.0, 25.0] {
            assert!(MixerChannel {
                gain_db,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for pan in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY, -100.1, 100.1] {
            assert!(MixerChannel {
                pan,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }

    #[test]
    fn audio_correction_defaults_are_safe_and_valid() {
        assert!(AudioCorrectionSettings::default().validate().is_ok());
        assert!(AudioCorrectionSettings {
            target_level_db: 2.0,
            ..Default::default()
        }
        .validate()
        .is_err());

        let partial: AudioCorrectionSettings = serde_json::from_str(r#"{"auto_gain":true}"#)
            .expect("partial correction settings should use defaults");
        assert!(partial.auto_gain);
        assert_eq!(partial.target_level_db, -18.0);
    }
}
