use serde::{Deserialize, Serialize};

/// Per-layer text animator settings (AE Text > Animators).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnimatorSettings {
    pub enabled: bool,
    pub selector: RangeSelector,
    pub position_offset: [f32; 2],
    pub scale: [f32; 2],
    pub opacity: f32,
    pub tracking: f32,
    pub rotation: f32,
    /// Per-character blur amount at full selector amount (in px). Rendered value
    /// is `blur_amount * amount` per character.
    #[serde(default)]
    pub blur_amount: f32,
}

impl Default for TextAnimatorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            selector: RangeSelector::default(),
            position_offset: [0.0, -40.0],
            scale: [1.0, 1.0],
            opacity: 0.0,
            tracking: 0.0,
            rotation: 0.0,
            blur_amount: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorShape {
    Square,
    RampUp,
    RampDown,
    Triangle,
    Round,
    Smooth,
    /// Per-character amount oscillates deterministically by character index.
    Wobble,
    /// Per-character amount is a deterministic pseudo-random value in [0, 1).
    Random,
    /// Per-character amount is driven by a Rhai expression. The expression
    /// receives `index` (0-based char index), `total` (char count), and
    /// `time` (seconds), and should return a value in 0..1.
    Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeSelector {
    pub start: f32,  // 0.0 to 100.0%
    pub end: f32,    // 0.0 to 100.0%
    pub offset: f32, // -100.0 to 100.0%
    pub shape: SelectorShape,
    pub ease_high: f32, // 0.0 to 100.0%
    pub ease_low: f32,  // 0.0 to 100.0%
    pub random_order: bool,
    /// Inclusive character index range (0-based). -1 means unbounded on that side.
    #[serde(default = "default_unbounded")]
    pub char_start: i32,
    #[serde(default = "default_unbounded")]
    pub char_end: i32,
    /// When present, overrides `offset` each frame (for typewriter sweeps etc.).
    #[serde(default)]
    pub offset_anim: Option<crate::core::property::Animatable<f32>>,
    /// Rhai expression for `SelectorShape::Expression`. Receives `index`,
    /// `total`, and `time`; returns amount in 0..1.
    #[serde(default)]
    pub expression: Option<String>,
}

fn default_unbounded() -> i32 {
    -1
}

impl Default for RangeSelector {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: SelectorShape::Square,
            ease_high: 0.0,
            ease_low: 0.0,
            random_order: false,
            char_start: -1,
            char_end: -1,
            offset_anim: None,
            expression: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterTransform {
    pub position_offset: [f32; 2],
    pub scale_multiplier: [f32; 2],
    pub opacity_multiplier: f32,
    pub tracking_offset: f32,
    pub rotation_deg: f32,
    /// Blur amount in px for this character (0.0 = no blur).
    pub blur: f32,
}

impl Default for CharacterTransform {
    fn default() -> Self {
        Self {
            position_offset: [0.0, 0.0],
            scale_multiplier: [1.0, 1.0],
            opacity_multiplier: 1.0,
            tracking_offset: 0.0,
            rotation_deg: 0.0,
            blur: 0.0,
        }
    }
}
