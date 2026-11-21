use super::*;

// ─── Layer Style ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DropShadowStyle {
    pub enabled: bool,
    pub blend_mode: BlendMode,
    pub opacity: f32,
    pub angle: f32,
    pub distance: f32,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OuterGlowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub spread: f32,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrokeStyle {
    pub enabled: bool,
    pub size: f32,
    pub position: u32,
    pub color: [f32; 4],
}

/// Gradient Overlay layer style: linear gradient blend across the layer
/// (angle in degrees, scale = gradient extent % of layer diagonal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientOverlayStyle {
    pub enabled: bool,
    pub opacity: f32,
    /// Gradient direction in degrees (0 = left→right, 90 = bottom→top)
    pub angle: f32,
    /// Gradient length as % of layer diagonal
    pub scale: f32,
    /// Gradient start / end colors
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

impl Default for GradientOverlayStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 100.0,
            angle: 90.0,
            scale: 100.0,
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Solid Color Overlay layer style.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorOverlayStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub color: [f32; 4],
}

/// Inner Shadow layer style (compass angle matching DropShadowStyle).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InnerShadowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub angle: f32,
    pub distance: f32,
    pub size: f32,
    pub color: [f32; 4],
}

/// Inner Glow layer style.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InnerGlowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub size: f32,
    pub color: [f32; 4],
}

/// Satin layer style: soft interior sheen band along the shape's contour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatinStyle {
    pub enabled: bool,
    pub opacity: f32,
    /// Sheen direction (compass degrees, matching DropShadowStyle)
    pub angle: f32,
    /// Offset of the sheen source from the edge
    pub distance: f32,
    /// Softness of the band
    pub size: f32,
    pub color: [f32; 4],
}

impl Default for SatinStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 50.0,
            angle: 90.0,
            distance: 12.0,
            size: 16.0,
            color: [0.2, 0.2, 0.35, 1.0],
        }
    }
}

/// Bevel/Emboss layer style: directional highlight + shadow along alpha edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BevelEmbossStyle {
    pub enabled: bool,
    /// Light direction (compass degrees matching DropShadowStyle)
    pub angle: f32,
    /// Edge detection offset (px)
    pub depth: f32,
    /// Softness of the bevel band (px)
    pub size: f32,
    /// Highlight strength (0..100)
    pub highlight: f32,
    /// Shadow strength (0..100)
    pub shadow: f32,
    /// Highlight color
    pub color_light: [f32; 4],
    /// Shadow color
    pub color_dark: [f32; 4],
}

impl Default for BevelEmbossStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            angle: 135.0,
            depth: 3.0,
            size: 3.0,
            highlight: 70.0,
            shadow: 50.0,
            color_light: [1.0, 1.0, 1.0, 1.0],
            color_dark: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerStyle {
    pub drop_shadow: DropShadowStyle,
    pub outer_glow: OuterGlowStyle,
    pub stroke: StrokeStyle,
    #[serde(default)]
    pub gradient_overlay: GradientOverlayStyle,
    #[serde(default)]
    pub color_overlay: ColorOverlayStyle,
    #[serde(default)]
    pub inner_shadow: InnerShadowStyle,
    #[serde(default)]
    pub inner_glow: InnerGlowStyle,
    #[serde(default)]
    pub satin: SatinStyle,
    #[serde(default)]
    pub bevel_emboss: BevelEmbossStyle,
}
