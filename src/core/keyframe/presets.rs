use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EasePreset {
    Standard,   // Easy Ease [0.25, 0.1, 0.25, 1.0]
    FastIn,     // Acceleration [0.42, 0.0, 1.0, 1.0]
    SmoothOut,  // Deceleration [0.0, 0.0, 0.58, 1.0]
    Overshoot,  // Bounce Spring [0.68, -0.55, 0.265, 1.55]
    Sine,       // Ultra Smooth Sine [0.37, 0.0, 0.63, 1.0]
    FastOut,    // Fast Deceleration [0.0, 0.0, 0.35, 1.0]
    SlowIn,     // Slow Acceleration [0.43, 0.0, 0.9, 1.0]
    CustomEase, // Custom bezier defined via custom_bezier
    MirrorEase, // Mirror the first half [0.5, 0, 1, 1]
    // Legacy AE presets (backward compat)
    EaseIn,  // Quadratic ease-in [0.5, 0, 1, 1]
    EaseOut, // Quadratic ease-out [0, 0, 0.58, 1]
    // New presets
    Elastic,     // Elastic bounce [0.5, 0, 0.8, 0.3]
    Bounce,      // Single bounce [0.5, 0.3, 0.7, 0.1]
    Cycle,       // Cycling [0.5, 0.5, 0.5, 0.5]
    MirrorEase2, // Mirror back and forth [0.5, 0, 1, 1]
    Custom0,     // [0.0, 0, 1, 1] - default
    Custom1,     // [0.25, 0.1, 0.25, 1] - slightly eased
    Custom2,     // [0.42, 0, 1, 1] - fast start
    Custom3,     // [0, 0, 0.58, 1] - slow start
}

impl EasePreset {
    pub fn control_points(self) -> [f32; 4] {
        match self {
            EasePreset::Standard => [0.25, 0.1, 0.25, 1.0],
            EasePreset::FastIn => [0.42, 0.0, 1.0, 1.0],
            EasePreset::SmoothOut => [0.0, 0.0, 0.58, 1.0],
            EasePreset::Overshoot => [0.68, -0.4, 0.265, 1.4],
            EasePreset::Sine => [0.37, 0.0, 0.63, 1.0],
            EasePreset::EaseIn => [0.5, 0.0, 1.0, 1.0],
            EasePreset::EaseOut => [0.0, 0.0, 0.58, 1.0],
            EasePreset::FastOut => [0.0, 0.0, 0.35, 1.0],
            EasePreset::SlowIn => [0.43, 0.0, 0.9, 1.0],
            EasePreset::CustomEase => [0.5, 0.0, 0.5, 1.0],
            EasePreset::MirrorEase => [0.5, 0.0, 1.0, 1.0],
            EasePreset::Elastic => [0.5, 0.2, 0.6, 0.8],
            EasePreset::Bounce => [0.5, 0.3, 0.7, 0.1],
            EasePreset::Cycle => [0.5, 0.5, 0.5, 0.5],
            EasePreset::MirrorEase2 => [0.5, 0.0, 1.0, 1.0],
            //
            EasePreset::Custom0 => [0.0, 0.0, 1.0, 1.0],
            EasePreset::Custom1 => [0.25, 0.1, 0.25, 1.0],
            EasePreset::Custom2 => [0.42, 0.0, 1.0, 1.0],
            EasePreset::Custom3 => [0.0, 0.0, 0.58, 1.0],
        }
    }
}
