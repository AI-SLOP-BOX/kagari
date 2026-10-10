//! Types and settings for RotoBrush strokes and configuration.

/// RotoBrush stroke type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum RotoStrokeType {
    #[default]
    Foreground, // Green stroke: marks subject
    Background, // Red stroke: marks background
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoStroke {
    pub stroke_type: RotoStrokeType,
    pub points: Vec<[f32; 2]>,
    pub radius: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u32>,
}

pub fn strokes_for_frame(strokes: &[RotoStroke], frame: u32) -> Vec<RotoStroke> {
    strokes
        .iter()
        .filter(|stroke| stroke.frame.is_none() || stroke.frame == Some(frame))
        .cloned()
        .collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoBrushSettings {
    pub feather_radius: f32,
    pub contrast: f32,
    pub edge_detection_sensitivity: f32,
    pub propagation_quality: usize, // Subdivisions
    pub temporal_smoothing: f32,
}

impl Default for RotoBrushSettings {
    fn default() -> Self {
        Self {
            feather_radius: 3.0,
            contrast: 1.0,
            edge_detection_sensitivity: 0.8,
            propagation_quality: 3,
            temporal_smoothing: 0.5,
        }
    }
}
