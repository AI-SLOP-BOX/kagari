//! Types and configuration options for audio spectrum & waveform visualization.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioSpectrumType {
    #[default]
    DigitalBands,
    AnalogLines,
    AnalogDots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioSpectrumDisplayMode {
    #[default]
    DigitalLines,
    AnalogLines,
    Dots,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioSpectrumOptions {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub frequency_bands: usize,
    pub max_height: f32,
    pub is_polar: bool,
    pub polar_radius: f32,
    pub display_mode: AudioSpectrumDisplayMode,
    pub spectrum_type: AudioSpectrumType,
    pub smoothing: f32,
    #[serde(default = "default_fft_size")]
    pub fft_size: usize,
    #[serde(default = "default_start_freq")]
    pub start_frequency: f32,
    #[serde(default = "default_end_freq")]
    pub end_frequency: f32,
    #[serde(default = "default_db_floor")]
    pub db_floor: f32,
    #[serde(default = "default_release")]
    pub release: f32,
    #[serde(default = "default_peak_decay")]
    pub peak_decay: f32,
}

fn default_fft_size() -> usize {
    2048
}
fn default_start_freq() -> f32 {
    20.0
}
fn default_end_freq() -> f32 {
    20000.0
}
fn default_db_floor() -> f32 {
    -60.0
}
fn default_release() -> f32 {
    0.25
}
fn default_peak_decay() -> f32 {
    0.02
}

impl Default for AudioSpectrumOptions {
    fn default() -> Self {
        Self {
            start_point: [200.0, 540.0],
            end_point: [1720.0, 540.0],
            frequency_bands: 64,
            max_height: 150.0,
            is_polar: false,
            polar_radius: 100.0,
            display_mode: AudioSpectrumDisplayMode::DigitalLines,
            spectrum_type: AudioSpectrumType::DigitalBands,
            smoothing: 0.5,
            fft_size: 2048,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpectrumBar {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub magnitude: f32,
}

/// Options for Audio Waveform visualizer (AE Parity: Generate > Audio Waveform).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioWaveformOptions {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub sample_count: usize,
    pub max_height: f32,
    pub stroke_width: f32,
    pub is_polar: bool,
    pub polar_radius: f32,
    pub softness: f32,
}

impl Default for AudioWaveformOptions {
    fn default() -> Self {
        Self {
            start_point: [200.0, 540.0],
            end_point: [1720.0, 540.0],
            sample_count: 256,
            max_height: 120.0,
            stroke_width: 2.5,
            is_polar: false,
            polar_radius: 120.0,
            softness: 1.0,
        }
    }
}

/// Multi-band frequency amplitude keyframes for advanced audio-reactive motion graphics.
#[derive(Debug, Clone, Default)]
pub struct MultiBandAudioKeyframes {
    pub frame: u32,
    pub bass_amplitude: f32,
    pub mid_amplitude: f32,
    pub treble_amplitude: f32,
    pub master_amplitude: f32,
}
