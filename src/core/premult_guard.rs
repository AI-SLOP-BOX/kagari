//! Alpha Premultiplication Guard & Automatic State Management.
//!
//! Prevents dark fringes, white halos, and double-premultiplication bugs by
//! tagging buffers with [`AlphaMode`] (`Straight` vs `Premultiplied`) and
//! automatically unpremultiplying around color grading operations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    Straight,
    Premultiplied,
}

#[derive(Debug, Clone)]
pub struct TaggedRgbaBuffer {
    pub rgba: Vec<f32>,
    pub alpha_mode: AlphaMode,
}

impl TaggedRgbaBuffer {
    pub fn new(rgba: Vec<f32>, alpha_mode: AlphaMode) -> Self {
        Self { rgba, alpha_mode }
    }

    /// Converts buffer in-place to `target` alpha mode (no-op if already matching).
    pub fn ensure_mode(&mut self, target: AlphaMode) {
        if self.alpha_mode == target {
            return;
        }
        match (self.alpha_mode, target) {
            (AlphaMode::Straight, AlphaMode::Premultiplied) => {
                for px in self.rgba.chunks_exact_mut(4) {
                    let a = px[3].clamp(0.0, 1.0);
                    px[0] *= a;
                    px[1] *= a;
                    px[2] *= a;
                }
            }
            (AlphaMode::Premultiplied, AlphaMode::Straight) => {
                for px in self.rgba.chunks_exact_mut(4) {
                    let a = px[3];
                    if a > 1e-6 {
                        px[0] /= a;
                        px[1] /= a;
                        px[2] /= a;
                    } else {
                        px[0] = 0.0;
                        px[1] = 0.0;
                        px[2] = 0.0;
                    }
                }
            }
            _ => {}
        }
        self.alpha_mode = target;
    }

    /// Safely executes a color-grading closure in `Straight` alpha space and restores
    /// the original [`AlphaMode`] afterward, eliminating dark fringe artifacts.
    pub fn with_safe_color_grade(&mut self, grade_fn: impl FnOnce(&mut [f32])) {
        let orig = self.alpha_mode;
        self.ensure_mode(AlphaMode::Straight);
        grade_fn(&mut self.rgba);
        self.ensure_mode(orig);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premult_guard_prevents_fringing_when_adding_offset_to_semitransparent_edge() {
        // Straight red at 50% alpha -> premultiplied [0.5, 0.0, 0.0, 0.5]
        let mut buf = TaggedRgbaBuffer::new(vec![0.5, 0.0, 0.0, 0.5], AlphaMode::Premultiplied);
        // Add +0.2 brightness offset safely in Straight space
        buf.with_safe_color_grade(|px| {
            px[0] += 0.2;
            px[1] += 0.2;
            px[2] += 0.2;
        });
        assert_eq!(buf.alpha_mode, AlphaMode::Premultiplied);
        // Expected straight [1.2, 0.2, 0.2, 0.5] -> premultiplied [0.6, 0.1, 0.1, 0.5]
        assert!((buf.rgba[0] - 0.6).abs() < 1e-5);
        assert!((buf.rgba[1] - 0.1).abs() < 1e-5);
        assert!((buf.rgba[3] - 0.5).abs() < 1e-5);
    }
}
