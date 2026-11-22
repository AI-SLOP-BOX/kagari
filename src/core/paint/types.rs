use serde::{Deserialize, Serialize};

/// A single paint stroke with pressure-sensitive width and hardness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintStroke {
    pub id: u64,
    pub color: [f32; 4],
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub points: Vec<StrokePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

/// Axis-aligned bounding box for dirty rect tracking.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DirtyRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl DirtyRect {
    pub fn new() -> Self {
        Self {
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    pub fn expand_point(&mut self, x: f32, y: f32, radius: f32) {
        self.min_x = self.min_x.min(x - radius);
        self.min_y = self.min_y.min(y - radius);
        self.max_x = self.max_x.max(x + radius);
        self.max_y = self.max_y.max(y + radius);
    }

    pub fn expand_rect(&mut self, other: &DirtyRect) {
        if !other.is_empty() {
            self.min_x = self.min_x.min(other.min_x);
            self.min_y = self.min_y.min(other.min_y);
            self.max_x = self.max_x.max(other.max_x);
            self.max_y = self.max_y.max(other.max_y);
        }
    }

    pub fn to_pixels(&self, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let x0 = (self.min_x.floor() as i32).max(0) as u32;
        let y0 = (self.min_y.floor() as i32).max(0) as u32;
        let x1 = (self.max_x.ceil() as i32).min(w as i32) as u32;
        let y1 = (self.max_y.ceil() as i32).min(h as i32) as u32;
        (x0, y0, x1, y1)
    }
}

/// A paint layer holding all brush strokes for a frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaintLayer {
    pub strokes: Vec<PaintStroke>,
    #[serde(skip)]
    pub dirty_rect: DirtyRect,
}

impl PaintLayer {
    pub fn new() -> Self {
        Self {
            strokes: Vec::new(),
            dirty_rect: DirtyRect::new(),
        }
    }

    pub fn add_stroke(&mut self, stroke: PaintStroke) {
        self.dirty_rect_for_stroke(&stroke);
        self.strokes.push(stroke);
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.dirty_rect = DirtyRect::new();
    }

    fn dirty_rect_for_stroke(&mut self, stroke: &PaintStroke) {
        for point in &stroke.points {
            self.dirty_rect
                .expand_point(point.x, point.y, stroke.size * 0.5);
        }
    }

    /// Render only the dirty rect region.
    pub fn render_dirty(&self, pixels: &mut [u8], width: u32, height: u32) {
        if self.dirty_rect.is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = self.dirty_rect.to_pixels(width, height);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        for stroke in &self.strokes {
            let points: Vec<[f32; 2]> = stroke.points.iter().map(|p| [p.x, p.y]).collect();
            super::brush::draw_stroke(pixels, width, height, &points, stroke.color, stroke.size);
        }
    }

    /// Render all paint strokes into an RGBA buffer.
    pub fn render(&self, pixels: &mut [u8], width: u32, height: u32) {
        for stroke in &self.strokes {
            let points: Vec<[f32; 2]> = stroke.points.iter().map(|p| [p.x, p.y]).collect();
            super::brush::draw_stroke(pixels, width, height, &points, stroke.color, stroke.size);
        }
    }
}
