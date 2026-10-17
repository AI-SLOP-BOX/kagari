use super::*;

// ─── Tracker Point ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPoint {
    pub id: String,
    pub name: String,
    pub position: Animatable<[f32; 2]>,
    pub search_size: f32,
    pub feature_size: f32,
    pub reference_pattern: Option<Vec<f32>>,
}

impl TrackerPoint {
    pub fn new(id: String, name: String, initial_pos: [f32; 2]) -> Self {
        Self {
            id,
            name,
            position: Animatable::new_constant(initial_pos),
            search_size: 25.0,
            feature_size: 10.0,
            reference_pattern: None,
        }
    }
}

// ─── Serde default helpers for LayerType::Text new fields ────────────────

fn default_font_family() -> String {
    "Inter".to_string()
}
fn default_leading() -> f32 {
    1.2
}
fn default_stroke_color() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}
fn default_video_speed() -> f32 {
    1.0
}

// ─── Layer Type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    Solid {
        color: [f32; 4],
    },
    Image {
        path: String,
    },
    /// Wavefront OBJ mesh rendered through the software 3D preview path.
    Model3D {
        path: String,
    },
    /// Video layer: plays a pre-extracted frame sequence (see video_import).
    /// `frames_dir` holds frame_%05d.webp files decoded at import time (older
    /// projects may still contain PNG frames); rendering samples the sequence
    /// by the layer's effective frame.
    Video {
        source: String,
        frames_dir: String,
        frame_count: u32,
        #[serde(default)]
        audio_wav: Option<String>,
        /// Playback speed multiplier: 1.0 = normal, 2.0 = double speed
        #[serde(default = "default_video_speed")]
        speed: f32,
    },
    Text {
        text: String,
        font_size: u32,
        color: [f32; 4],
        /// Font family name, e.g. "Inter"
        #[serde(default = "default_font_family")]
        font_family: String,
        /// Tracking (letter-spacing) in virtual units
        #[serde(default)]
        tracking: f32,
        /// Leading (line-height) multiplier
        #[serde(default = "default_leading")]
        leading: f32,
        /// Paragraph alignment: 0=Left, 1=Center, 2=Right
        #[serde(default)]
        align: usize,
        /// Stroke color \[r,g,b,a]
        #[serde(default = "default_stroke_color")]
        stroke_color: [f32; 4],
        /// Stroke width in pixels (0 = no stroke)
        #[serde(default)]
        stroke_width: f32,
        /// Text on Path: use first mask as text path
        #[serde(default)]
        text_on_path: bool,
    },
    Shape {
        shape_type: ShapeType,
        color: [f32; 4],
        stroke_color: [f32; 4],
        stroke_width: f32,
        #[serde(default)]
        fill_type: ShapeFillType,
        #[serde(default)]
        extrusion_depth: f32,
        #[serde(default)]
        bevel_depth: f32,
    },
    Null,
    PreComp {
        comp_id: String,
    },
    Audio {
        path: String,
        volume: Animatable<f32>,
    },
    /// Adjustment Layer: applies layer effects to the composited buffer of lower layers
    AdjustmentLayer,
    /// Particle emitter layer: procedural particle simulation
    Particle {
        #[serde(default)]
        emitter: crate::core::particle_system::ParticleEmitter,
    },
}

// ─── Trim Paths Animator ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimPaths {
    /// Start percentage (0.0 .. 100.0)
    pub start: Animatable<f32>,
    /// End percentage (0.0 .. 100.0)
    pub end: Animatable<f32>,
    /// Offset degrees (0.0 .. 360.0)
    pub offset: Animatable<f32>,
}

impl Default for TrimPaths {
    fn default() -> Self {
        Self {
            start: Animatable::new_constant(0.0),
            end: Animatable::new_constant(100.0),
            offset: Animatable::new_constant(0.0),
        }
    }
}

impl TrimPaths {
    /// Trims a list of 2D polygon points based on start/end/offset percentages.
    pub fn trim_polygon(&self, points: &[[f32; 2]], frame: u32) -> Vec<[f32; 2]> {
        if points.len() < 2 {
            return points.to_vec();
        }

        let mut start_pct = self.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
        let mut end_pct = self.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
        let offset_norm = (self.offset.evaluate(frame) / 360.0).fract();

        start_pct = (start_pct + offset_norm).fract();
        end_pct = (end_pct + offset_norm).fract();
        if start_pct < 0.0 {
            start_pct += 1.0;
        }
        if end_pct < 0.0 {
            end_pct += 1.0;
        }

        if (start_pct - end_pct).abs() < 0.001 {
            return Vec::new();
        }

        let mut segment_lengths = Vec::with_capacity(points.len() - 1);
        let mut total_len = 0.0f32;

        for w in points.windows(2) {
            let dx = w[1][0] - w[0][0];
            let dy = w[1][1] - w[0][1];
            let len = (dx * dx + dy * dy).sqrt();
            segment_lengths.push(len);
            total_len += len;
        }

        if total_len <= 0.0001 {
            return points.to_vec();
        }

        let target_start = start_pct.min(end_pct) * total_len;
        let target_end = start_pct.max(end_pct) * total_len;

        let mut current_accum = 0.0f32;
        let mut trimmed = Vec::new();

        for i in 0..segment_lengths.len() {
            let seg_len = segment_lengths[i];
            let seg_start = current_accum;
            let seg_end = current_accum + seg_len;

            if seg_end >= target_start && seg_start <= target_end {
                let p0 = points[i];
                let p1 = points[i + 1];

                // Interpolate start point
                let t_start = if target_start > seg_start {
                    ((target_start - seg_start) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Interpolate end point
                let t_end = if target_end < seg_end {
                    ((target_end - seg_start) / seg_len).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let sub_p0 = [
                    p0[0] + (p1[0] - p0[0]) * t_start,
                    p0[1] + (p1[1] - p0[1]) * t_start,
                ];
                let sub_p1 = [
                    p0[0] + (p1[0] - p0[0]) * t_end,
                    p0[1] + (p1[1] - p0[1]) * t_end,
                ];

                if trimmed.is_empty() {
                    trimmed.push(sub_p0);
                }
                trimmed.push(sub_p1);
            }

            current_accum = seg_end;
        }

        trimmed
    }
}

impl LayerType {
    /// Convenience constructor for a new text layer with sensible defaults.
    pub fn new_text(text: impl Into<String>, font_size: u32, color: [f32; 4]) -> Self {
        LayerType::Text {
            text: text.into(),
            font_size,
            color,
            font_family: "Inter".to_string(),
            tracking: 0.0,
            leading: 1.2,
            align: 0,
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            text_on_path: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ShapeFillType {
    #[default]
    Solid,
    LinearGradient {
        start: [f32; 2],
        end: [f32; 2],
        colors: Vec<[f32; 4]>,
        stops: Vec<f32>,
    },
    RadialGradient {
        center: [f32; 2],
        radius: f32,
        colors: Vec<[f32; 4]>,
        stops: Vec<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeType {
    /// Freeform Bezier path with control points.
    FreeformBezier {
        /// Flat list of [x, y] coordinates (closed path if first == last).
        points: Vec<[f32; 2]>,
        /// Tangent handles: pairs of (in_tangent, out_tangent) per point.
        tangents: Vec<([f32; 2], [f32; 2])>,
        closed: bool,
    },
    Rectangle {
        width: Animatable<f32>,
        height: Animatable<f32>,
        corner_radius: Animatable<f32>,
    },
    Ellipse {
        width: Animatable<f32>,
        height: Animatable<f32>,
    },
    Star {
        points: Animatable<f32>,
        inner_radius: Animatable<f32>,
        outer_radius: Animatable<f32>,
    },
    Polygon {
        sides: Animatable<f32>,
        radius: Animatable<f32>,
    },
}

// ─── Track Matte ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrackMatteMode {
    #[default]
    None,
    AlphaMatte,
    AlphaMatteInverted,
    LumaMatte,
    LumaMatteInverted,
}

// ─── AE Blend Modes ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Darken,
    Lighten,
    SoftLight,
    HardLight,
    Difference,
    Exclusion,
    Divide,
    Subtract,
    ColorBurn,
    LinearBurn,
    VividLight,
    ColorDodge,
    LinearDodge,
    Color,
    Hue,
    Saturation,
    Luminosity,
    StencilAlpha,
    StencilLuma,
    SilhouetteAlpha,
    SilhouetteLuma,
    Behind,
    AlphaAdd,
    LinearLight,
}

// ─── Label Colors ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LabelColor {
    #[default]
    None,
    Red,
    Yellow,
    Aqua,
    Pink,
    Lavender,
    Peach,
    Sea,
    Blue,
    Purple,
}

impl LabelColor {
    pub const ALL: [LabelColor; 10] = [
        LabelColor::None,
        LabelColor::Red,
        LabelColor::Yellow,
        LabelColor::Aqua,
        LabelColor::Pink,
        LabelColor::Lavender,
        LabelColor::Peach,
        LabelColor::Sea,
        LabelColor::Blue,
        LabelColor::Purple,
    ];

    pub fn name(self) -> &'static str {
        match self {
            LabelColor::None => "None",
            LabelColor::Red => "Red",
            LabelColor::Yellow => "Yellow",
            LabelColor::Aqua => "Aqua",
            LabelColor::Pink => "Pink",
            LabelColor::Lavender => "Lavender",
            LabelColor::Peach => "Peach",
            LabelColor::Sea => "Sea",
            LabelColor::Blue => "Blue",
            LabelColor::Purple => "Purple",
        }
    }

    pub fn to_rgb(self) -> [f32; 3] {
        match self {
            LabelColor::None => [0.35, 0.35, 0.35],
            LabelColor::Red => [0.90, 0.25, 0.25],
            LabelColor::Yellow => [0.95, 0.85, 0.10],
            LabelColor::Aqua => [0.20, 0.85, 0.80],
            LabelColor::Pink => [0.95, 0.45, 0.70],
            LabelColor::Lavender => [0.65, 0.55, 0.90],
            LabelColor::Peach => [0.95, 0.70, 0.45],
            LabelColor::Sea => [0.30, 0.70, 0.55],
            LabelColor::Blue => [0.35, 0.55, 0.95],
            LabelColor::Purple => [0.70, 0.35, 0.90],
        }
    }

    pub fn color32(self) -> eframe::egui::Color32 {
        let [r, g, b] = self.to_rgb();
        eframe::egui::Color32::from_rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    }
}

// ─── Expression Engine ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Wiggle { frequency: f32, amplitude: f32 },
    LoopOut,
    PingPong,
    TimeDriver { multiplier: f32, offset: f32 },
    Raw(String),
}

impl Expression {
    pub fn evaluate_f32(&self, base: f32, frame: u32, fps: u32) -> f32 {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle {
                frequency,
                amplitude,
            } => {
                let seed = (frame as f32 * frequency * 1.618_034) % std::f32::consts::TAU;
                let noise = seed.sin() * 0.7 + (seed * 2.1).sin() * 0.2 + (seed * 5.3).sin() * 0.1;
                base + noise * amplitude
            }
            Expression::TimeDriver { multiplier, offset } => time * multiplier + offset,
            // Raw Rhai script — evaluated by the Rhai expression engine.
            // Supports full AE-compatible API: time, frame, fps, value, wiggle(), etc.
            Expression::Raw(script) => RHAI_ENGINE
                .with(|engine| expression_engine::eval_f32(engine, script, base, frame, fps)),
            _ => base,
        }
    }

    pub fn evaluate_f32_with_property(
        &self,
        base: f32,
        frame: u32,
        fps: u32,
        source: &Animatable<f32>,
    ) -> f32 {
        match self {
            Expression::Raw(script) => RHAI_ENGINE.with(|engine| {
                expression_engine::eval_f32_with_property(engine, script, base, frame, fps, source)
            }),
            _ => self.evaluate_f32(base, frame, fps),
        }
    }

    pub fn evaluate_v2(&self, base: [f32; 2], frame: u32, fps: u32) -> [f32; 2] {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle {
                frequency,
                amplitude,
            } => {
                let t = frame as f32 * frequency * 1.618_034;
                let nx = t.sin() * 0.7 + (t * 2.1).sin() * 0.2 + (t * 5.3).sin() * 0.1;
                let ny = (t + 100.0).sin() * 0.7 + ((t + 100.0) * 2.1).sin() * 0.2;
                [base[0] + nx * amplitude, base[1] + ny * amplitude]
            }
            Expression::TimeDriver { multiplier, offset } => {
                [base[0] + time * multiplier + offset, base[1]]
            }
            // Raw Rhai script — evaluated by the Rhai expression engine.
            Expression::Raw(script) => RHAI_ENGINE
                .with(|engine| expression_engine::eval_v2(engine, script, base, frame, fps)),
            _ => base,
        }
    }

    pub fn evaluate_v2_with_property(
        &self,
        base: [f32; 2],
        frame: u32,
        fps: u32,
        source: &Animatable<[f32; 2]>,
    ) -> [f32; 2] {
        match self {
            Expression::Raw(script) => RHAI_ENGINE.with(|engine| {
                expression_engine::eval_v2_with_property(engine, script, base, frame, fps, source)
            }),
            _ => self.evaluate_v2(base, frame, fps),
        }
    }
}
