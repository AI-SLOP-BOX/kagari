use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BezierControlPoint {
    /// Time influence, typically between 0.0 and 1.0 (X axis in AE velocity graph)
    pub influence: f32,
    /// Value speed, change per second (Y axis in AE velocity graph)
    pub speed: f32,
}

impl Default for BezierControlPoint {
    fn default() -> Self {
        Self {
            influence: 0.333,
            speed: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum InterpolationType {
    #[default]
    Linear,
    Hold,
    Bezier {
        /// Control point outgoing from the current keyframe (influence/speed)
        outgoing: BezierControlPoint,
        /// Control point incoming to the next keyframe (influence/speed)
        incoming: BezierControlPoint,
        /// Easing parameters mapped to CSS-like control points (x1, y1, x2, y2)
        /// if computed, otherwise default to a standard ease.
        #[serde(default)]
        custom_bezier: Option<[f32; 4]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphEditorMode {
    #[default]
    ValueGraph,
    SpeedGraph,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub frame: u32,
    pub value: T,
    pub interpolation: InterpolationType,
}

impl<T> Keyframe<T> {
    pub fn new(frame: u32, value: T, interpolation: InterpolationType) -> Self {
        Self {
            frame,
            value,
            interpolation,
        }
    }
}
