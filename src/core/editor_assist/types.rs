#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeRange {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReframePoint {
    pub frame: usize,
    pub center: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LetterboxInsets {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopCandidate {
    pub start_frame: usize,
    pub end_frame: usize,
    pub error: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureReport {
    pub shadow_clipped_fraction: f32,
    pub highlight_clipped_fraction: f32,
}
