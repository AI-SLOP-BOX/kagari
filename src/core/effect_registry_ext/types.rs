use serde::{Deserialize, Serialize};

/// Serializable extended effect. All parameters are plain values; animation
/// is driven by the caller evaluating them per frame before dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtEffect {
    /// AE Wave Warp (Pro rebuild).
    WaveWarp {
        #[serde(default = "default_pct_50")]
        wave_height: f32,
        #[serde(default = "default_pct_100")]
        wave_width: f32,
        #[serde(default)]
        speed: f32,
        #[serde(default = "default_deg_90")]
        direction_deg: f32,
        /// 0=Sine 1=Triangle 2=Square 3=Sawtooth
        #[serde(default)]
        wave_type: u8,
        /// 0=All 1=LeftRight 2=TopBottom 3=None
        #[serde(default)]
        pinning: u8,
    },
    /// CC Lens fisheye.
    CcLens {
        #[serde(default = "default_pct_50")]
        convergence: f32,
        #[serde(default = "default_unit")]
        zoom: f32,
    },
    /// Polar Coordinates conversion.
    PolarCoordinates {
        #[serde(default)]
        to_polar: bool,
        #[serde(default = "default_pct_100")]
        interpolation: f32,
    },
    /// Lens distortion compensation.
    OpticsCompensation {
        #[serde(default)]
        field_of_view_deg: f32,
        #[serde(default)]
        reverse: bool,
        #[serde(default = "default_unit")]
        zoom: f32,
    },
    /// Spline tone curves. Empty vec = channel untouched.
    Curves {
        #[serde(default)]
        master: Vec<[f32; 2]>,
        #[serde(default)]
        red: Vec<[f32; 2]>,
        #[serde(default)]
        green: Vec<[f32; 2]>,
        #[serde(default)]
        blue: Vec<[f32; 2]>,
    },
    /// Three-way color balance (-100..100 per slider).
    ColorBalance {
        #[serde(default)]
        shadows: [f32; 3],
        #[serde(default)]
        midtones: [f32; 3],
        #[serde(default)]
        highlights: [f32; 3],
        #[serde(default = "default_enabled")]
        preserve_luminosity: bool,
    },
    /// Channel mixer matrix in percent units.
    ChannelMixer {
        #[serde(default = "identity_matrix")]
        matrix: [[f32; 3]; 3],
        #[serde(default)]
        monochrome: bool,
    },
    /// CC Light Sweep specular band.
    LightSweep {
        #[serde(default)]
        direction_deg: f32,
        #[serde(default = "default_half")]
        center: f32,
        #[serde(default = "default_quarter")]
        width: f32,
        #[serde(default = "default_sixty_pct")]
        sweep_intensity: f32,
        #[serde(default = "default_thirty_pct")]
        edge_intensity: f32,
    },
    /// Radial (zoom) fast blur.
    RadialFastBlur {
        #[serde(default = "default_half")]
        amount: f32,
        #[serde(default = "default_sample_count")]
        samples: u32,
    },
    /// Vertical-axis bend between top/bottom offsets (px).
    BendIt {
        #[serde(default)]
        top_offset: f32,
        #[serde(default)]
        bottom_offset: f32,
    },
    /// Tiling with edge mode (0=Repeat 1=Mirror).
    Tiler {
        #[serde(default = "default_pct_200")]
        scale_percent: f32,
        #[serde(default)]
        mirror: bool,
    },
    /// Threshold bloom glow.
    GlowPro {
        #[serde(default = "default_seventy_pct")]
        threshold: f32,
        #[serde(default = "default_count_four")]
        radius: u32,
        #[serde(default = "default_unit")]
        intensity: f32,
    },
    /// Saturation boost protecting skin tones (−100..100).
    Vibrance {
        #[serde(default = "default_half")]
        amount: f32,
    },
    /// Temperature/Tint white balance (−100..100 each).
    WhiteBalance {
        #[serde(default)]
        temperature: f32,
        #[serde(default)]
        tint: f32,
    },
    /// Hue rotation / saturation / lightness three-way.
    HslAdjust {
        #[serde(default)]
        hue_deg: f32,
        #[serde(default)]
        saturation: f32,
        #[serde(default)]
        lightness: f32,
    },
}

pub fn default_pct_50() -> f32 {
    50.0
}
pub fn default_pct_100() -> f32 {
    100.0
}
pub fn default_deg_90() -> f32 {
    90.0
}
pub fn default_unit() -> f32 {
    1.0
}
pub fn default_half() -> f32 {
    0.5
}
pub fn default_quarter() -> f32 {
    0.25
}
pub fn default_sixty_pct() -> f32 {
    0.6
}
pub fn default_thirty_pct() -> f32 {
    0.3
}
pub fn default_sample_count() -> u32 {
    12
}
pub fn default_pct_200() -> f32 {
    200.0
}
pub fn default_seventy_pct() -> f32 {
    0.7
}
pub fn default_count_four() -> u32 {
    4
}
pub fn default_enabled() -> bool {
    true
}
pub fn identity_matrix() -> [[f32; 3]; 3] {
    [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0]]
}

impl ExtEffect {
    /// Stable identifier used by project files and UI catalogs.
    pub fn type_id(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. } => "wave_warp",
            ExtEffect::CcLens { .. } => "cc_lens",
            ExtEffect::PolarCoordinates { .. } => "polar_coordinates",
            ExtEffect::OpticsCompensation { .. } => "optics_compensation",
            ExtEffect::Curves { .. } => "curves",
            ExtEffect::ColorBalance { .. } => "color_balance",
            ExtEffect::ChannelMixer { .. } => "channel_mixer",
            ExtEffect::LightSweep { .. } => "light_sweep",
            ExtEffect::RadialFastBlur { .. } => "radial_fast_blur",
            ExtEffect::BendIt { .. } => "bend_it",
            ExtEffect::Tiler { .. } => "tiler",
            ExtEffect::GlowPro { .. } => "glow_pro",
            ExtEffect::Vibrance { .. } => "vibrance",
            ExtEffect::WhiteBalance { .. } => "white_balance",
            ExtEffect::HslAdjust { .. } => "hsl_adjust",
        }
    }

    /// Human-readable display name for effect browsers.
    pub fn display_name(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. } => "Wave Warp",
            ExtEffect::CcLens { .. } => "CC Lens",
            ExtEffect::PolarCoordinates { .. } => "Polar Coordinates",
            ExtEffect::OpticsCompensation { .. } => "Optics Compensation",
            ExtEffect::Curves { .. } => "Curves",
            ExtEffect::ColorBalance { .. } => "Color Balance",
            ExtEffect::ChannelMixer { .. } => "Channel Mixer",
            ExtEffect::LightSweep { .. } => "CC Light Sweep",
            ExtEffect::RadialFastBlur { .. } => "CC Radial Fast Blur",
            ExtEffect::BendIt { .. } => "CC Bend It",
            ExtEffect::Tiler { .. } => "CC Tiler",
            ExtEffect::GlowPro { .. } => "Glow",
            ExtEffect::Vibrance { .. } => "Vibrance",
            ExtEffect::WhiteBalance { .. } => "White Balance",
            ExtEffect::HslAdjust { .. } => "HSL Adjust",
        }
    }

    /// Category for the effects library browser.
    pub fn category(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. }
            | ExtEffect::CcLens { .. }
            | ExtEffect::PolarCoordinates { .. }
            | ExtEffect::OpticsCompensation { .. }
            | ExtEffect::BendIt { .. }
            | ExtEffect::Tiler { .. } => "Distort",
            ExtEffect::LightSweep { .. } | ExtEffect::GlowPro { .. } => "Stylize",
            ExtEffect::RadialFastBlur { .. } => "Blur & Sharpen",
            ExtEffect::Curves { .. }
            | ExtEffect::ColorBalance { .. }
            | ExtEffect::ChannelMixer { .. }
            | ExtEffect::Vibrance { .. }
            | ExtEffect::WhiteBalance { .. }
            | ExtEffect::HslAdjust { .. } => "Color Correction",
        }
    }
}
