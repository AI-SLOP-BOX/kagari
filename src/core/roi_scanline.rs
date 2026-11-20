//! Nuke-style ROI / Data Window Bounding Box & Scanline Row Evaluator.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DataWindow {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DataWindow {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn right(&self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = self.right().max(other.right());
        let y1 = self.bottom().max(other.bottom());
        Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        if x1 <= x0 || y1 <= y0 {
            return Self::empty();
        }
        Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    pub fn expand(&self, pad: u32, max_w: u32, max_h: u32) -> Self {
        if self.is_empty() {
            return *self;
        }
        let x0 = self.x.saturating_sub(pad);
        let y0 = self.y.saturating_sub(pad);
        let x1 = self.right().saturating_add(pad).min(max_w);
        let y1 = self.bottom().saturating_add(pad).min(max_h);
        Self {
            x: x0,
            y: y0,
            width: x1.saturating_sub(x0),
            height: y1.saturating_sub(y0),
        }
    }
}

/// Computes the tightest non-zero [`DataWindow`] (Bounding Box) of `rgba` (`RGBA32F`).
pub fn compute_nonzero_bbox(
    rgba: &[f32],
    width: u32,
    height: u32,
    alpha_threshold: f32,
) -> DataWindow {
    if width == 0 || height == 0 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return DataWindow::empty();
    }
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            if rgba[idx + 3] > alpha_threshold
                || rgba[idx].abs() > alpha_threshold
                || rgba[idx + 1].abs() > alpha_threshold
                || rgba[idx + 2].abs() > alpha_threshold
            {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !any {
        DataWindow::empty()
    } else {
        DataWindow {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        }
    }
}

/// Evaluates a scanline kernel only across rows & columns inside `roi` (`DataWindow`).
/// Returns the number of pixels actually processed.
pub fn evaluate_scanline_roi(
    out_rgba: &mut [f32],
    width: u32,
    height: u32,
    roi: DataWindow,
    mut pixel_fn: impl FnMut(u32, u32) -> [f32; 4],
) -> usize {
    let clamped = roi.intersect(&DataWindow {
        x: 0,
        y: 0,
        width,
        height,
    });
    if clamped.is_empty() || out_rgba.len() != (width as usize) * (height as usize) * 4 {
        return 0;
    }
    let mut processed = 0usize;
    for y in clamped.y..clamped.bottom() {
        let row_start = y as usize * width as usize;
        for x in clamped.x..clamped.right() {
            let px = pixel_fn(x, y);
            let dst = (row_start + x as usize) * 4;
            out_rgba[dst..dst + 4].copy_from_slice(&px);
            processed += 1;
        }
    }
    processed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_and_scanline_roi_skip_empty_regions() {
        let mut buf = vec![0.0_f32; 16 * 16 * 4];
        // Active pixel at (5, 7) and (8, 9)
        for &(x, y) in &[(5u32, 7u32), (8u32, 9u32)] {
            let i = (y * 16 + x) as usize * 4;
            buf[i] = 1.0;
            buf[i + 3] = 1.0;
        }
        let bbox = compute_nonzero_bbox(&buf, 16, 16, 1e-4);
        assert_eq!(
            bbox,
            DataWindow {
                x: 5,
                y: 7,
                width: 4,
                height: 3
            }
        );

        let mut out = vec![0.0_f32; 16 * 16 * 4];
        let processed = evaluate_scanline_roi(&mut out, 16, 16, bbox, |_, _| [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(processed, 12); // Only 4x3 = 12 pixels evaluated out of 256!
    }
}
