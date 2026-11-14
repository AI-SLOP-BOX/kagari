#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderScale {
    pub x: f32,
    pub y: f32,
}

impl RenderScale {
    pub(crate) const FULL: Self = Self { x: 1.0, y: 1.0 };

    pub(crate) fn from_dimensions(source: [u32; 2], target: [u32; 2]) -> Option<Self> {
        if source.contains(&0) || target.contains(&0) {
            return None;
        }
        Some(Self {
            x: target[0] as f32 / source[0] as f32,
            y: target[1] as f32 / source[1] as f32,
        })
    }

    pub(crate) fn point(self, point: [f32; 2]) -> [f32; 2] {
        [point[0] * self.x, point[1] * self.y]
    }

    pub(crate) fn radius(self) -> f32 {
        ((self.x.abs() * self.y.abs()).sqrt()).max(0.0001)
    }
}
