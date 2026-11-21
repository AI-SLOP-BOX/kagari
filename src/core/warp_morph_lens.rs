//! Mesh Warp, 2-Frame Image Morphing, and Auto Lens Distortion Analysis (Brown-Conrady `k1, k2`).

/// Normalized 2D control-point grid (`cols x rows` in `0.0..=1.0` UV space).
#[derive(Debug, Clone)]
pub struct WarpGrid {
    pub cols: usize,
    pub rows: usize,
    pub points: Vec<[f32; 2]>,
}

impl WarpGrid {
    /// Creates an identity uniform grid in `0.0..=1.0`.
    pub fn identity(cols: usize, rows: usize) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(2);
        let mut points = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let v = r as f32 / (rows - 1) as f32;
            for c in 0..cols {
                let u = c as f32 / (cols - 1) as f32;
                points.push([u, v]);
            }
        }
        Self { cols, rows, points }
    }

    #[inline]
    pub fn sample_uv(&self, u: f32, v: f32) -> [f32; 2] {
        let gx = u.clamp(0.0, 1.0) * (self.cols - 1) as f32;
        let gy = v.clamp(0.0, 1.0) * (self.rows - 1) as f32;
        let c0 = (gx.floor() as usize).min(self.cols - 1);
        let r0 = (gy.floor() as usize).min(self.rows - 1);
        let c1 = (c0 + 1).min(self.cols - 1);
        let r1 = (r0 + 1).min(self.rows - 1);
        let fx = gx - c0 as f32;
        let fy = gy - r0 as f32;

        let p00 = self.points[r0 * self.cols + c0];
        let p10 = self.points[r0 * self.cols + c1];
        let p01 = self.points[r1 * self.cols + c0];
        let p11 = self.points[r1 * self.cols + c1];

        let tx = [p00[0] * (1.0 - fx) + p10[0] * fx, p00[1] * (1.0 - fx) + p10[1] * fx];
        let bx = [p01[0] * (1.0 - fx) + p11[0] * fx, p01[1] * (1.0 - fx) + p11[1] * fx];
        [tx[0] * (1.0 - fy) + bx[0] * fy, tx[1] * (1.0 - fy) + bx[1] * fy]
    }
}

#[inline]
fn sample_bilinear_rgba(src: &[f32], width: u32, height: u32, u: f32, v: f32) -> [f32; 4] {
    if u < 0.0 || v < 0.0 || u > 1.0 || v > 1.0 || width == 0 || height == 0 {
        return [0.0; 4];
    }
    let w = width as usize;
    let h = height as usize;
    let fx = u * (w - 1) as f32;
    let fy = v * (h - 1) as f32;
    let x0 = (fx.floor() as usize).min(w - 1);
    let y0 = (fy.floor() as usize).min(h - 1);
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let i00 = (y0 * w + x0) * 4;
    let i10 = (y0 * w + x1) * 4;
    let i01 = (y1 * w + x0) * 4;
    let i11 = (y1 * w + x1) * 4;
    let mut out = [0.0_f32; 4];
    for c in 0..4 {
        let top = src[i00 + c] * (1.0 - tx) + src[i10 + c] * tx;
        let bot = src[i01 + c] * (1.0 - tx) + src[i11 + c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Warps `src_rgba` (`RGBA32F`) using a [`WarpGrid`] mapping output UV -> source UV.
pub fn warp_mesh_f32(src_rgba: &[f32], width: u32, height: u32, grid: &WarpGrid) -> Vec<f32> {
    let count = (width as usize).saturating_mul(height as usize);
    if count == 0 || src_rgba.len() != count * 4 {
        return Vec::new();
    }
    let mut out = vec![0.0_f32; count * 4];
    for y in 0..height {
        let v = if height > 1 {
            y as f32 / (height - 1) as f32
        } else {
            0.0
        };
        for x in 0..width {
            let u = if width > 1 {
                x as f32 / (width - 1) as f32
            } else {
                0.0
            };
            let suv = grid.sample_uv(u, v);
            let px = sample_bilinear_rgba(src_rgba, width, height, suv[0], suv[1]);
            let dst = (y as usize * width as usize + x as usize) * 4;
            out[dst..dst + 4].copy_from_slice(&px);
        }
    }
    out
}

/// Morphs between `frame_a` (with `grid_a`) and `frame_b` (with `grid_b`) at blend `t` (`0.0..=1.0`).
pub fn morph_frames_f32(
    frame_a: &[f32],
    frame_b: &[f32],
    width: u32,
    height: u32,
    grid_a: &WarpGrid,
    grid_b: &WarpGrid,
    t: f32,
) -> Vec<f32> {
    let t = t.clamp(0.0, 1.0);
    if grid_a.cols != grid_b.cols || grid_a.rows != grid_b.rows {
        return Vec::new();
    }
    // Interpolate intermediate morph grids for A and B
    let mut g_a_to_t = WarpGrid::identity(grid_a.cols, grid_a.rows);
    let mut g_b_to_t = WarpGrid::identity(grid_a.cols, grid_a.rows);
    for i in 0..grid_a.points.len() {
        let da = [
            grid_a.points[i][0] - g_a_to_t.points[i][0],
            grid_a.points[i][1] - g_a_to_t.points[i][1],
        ];
        let db = [
            grid_b.points[i][0] - g_b_to_t.points[i][0],
            grid_b.points[i][1] - g_b_to_t.points[i][1],
        ];
        g_a_to_t.points[i][0] += da[0] * t;
        g_a_to_t.points[i][1] += da[1] * t;
        g_b_to_t.points[i][0] += db[0] * (1.0 - t);
        g_b_to_t.points[i][1] += db[1] * (1.0 - t);
    }

    let wa = warp_mesh_f32(frame_a, width, height, &g_a_to_t);
    let wb = warp_mesh_f32(frame_b, width, height, &g_b_to_t);
    wa.iter()
        .zip(wb.iter())
        .map(|(&a, &b)| a * (1.0 - t) + b * t)
        .collect()
}

/// Brown-Conrady radial lens distortion parameters (`k1`, `k2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LensDistortionModel {
    pub k1: f32,
    pub k2: f32,
    pub center: [f32; 2], // normalized [0.5, 0.5]
}

impl Default for LensDistortionModel {
    fn default() -> Self {
        Self {
            k1: 0.0,
            k2: 0.0,
            center: [0.5, 0.5],
        }
    }
}

/// Automatically estimates radial lens distortion `(k1, k2)` from observed curved
/// edge polylines (each polyline should represent a straight real-world line in normalized `0..1` coords).
pub fn estimate_lens_distortion_k1_k2(lines: &[Vec<[f32; 2]>]) -> LensDistortionModel {
    let mut best_k1 = 0.0_f32;
    let mut best_k2 = 0.0_f32;
    let mut best_err = f32::INFINITY;

    let eval_err = |k1: f32, k2: f32| -> f32 {
        let mut total = 0.0_f32;
        for line in lines {
            if line.len() < 3 {
                continue;
            }
            let undist = |p: [f32; 2]| -> [f32; 2] {
                let dx = p[0] - 0.5;
                let dy = p[1] - 0.5;
                let r2 = dx * dx + dy * dy;
                let scale = 1.0 + k1 * r2 + k2 * r2 * r2;
                [0.5 + dx * scale, 0.5 + dy * scale]
            };
            let a = undist(line[0]);
            let b = undist(line[line.len() - 1]);
            let abx = b[0] - a[0];
            let aby = b[1] - a[1];
            let len = (abx * abx + aby * aby).sqrt().max(1e-6);
            for &pt in &line[1..line.len() - 1] {
                let u = undist(pt);
                let cross = ((u[0] - a[0]) * aby - (u[1] - a[1]) * abx).abs();
                total += cross / len;
            }
        }
        total
    };

    // Coarse-to-fine grid search over physical k1 in [-0.6, 0.6], k2 in [-0.3, 0.3]
    for step_i in -24..=24 {
        let k1 = step_i as f32 * 0.025;
        for step_j in -12..=12 {
            let k2 = step_j as f32 * 0.025;
            let err = eval_err(k1, k2);
            if err < best_err {
                best_err = err;
                best_k1 = k1;
                best_k2 = k2;
            }
        }
    }
    for step_i in -10..=10 {
        let k1 = best_k1 + step_i as f32 * 0.0025;
        for step_j in -10..=10 {
            let k2 = best_k2 + step_j as f32 * 0.0025;
            let err = eval_err(k1, k2);
            if err < best_err {
                best_err = err;
                best_k1 = k1;
                best_k2 = k2;
            }
        }
    }

    LensDistortionModel {
        k1: best_k1,
        k2: best_k2,
        center: [0.5, 0.5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_lens_distortion_solver_recovers_radial_barrel_k1() {
        // Synthesize straight horizontal lines warped by barrel distortion (distorted = p_u / (1 + 0.25*r^2))
        let true_k1 = 0.25_f32;
        let mut lines = Vec::new();
        for &y in &[0.15_f32, 0.85_f32] {
            let mut poly = Vec::new();
            for i in 0..=8 {
                let x = 0.1 + 0.8 * (i as f32 / 8.0);
                let dx = x - 0.5;
                let dy = y - 0.5;
                let r2 = dx * dx + dy * dy;
                let inv = 1.0 / (1.0 + true_k1 * r2);
                poly.push([0.5 + dx * inv, 0.5 + dy * inv]);
            }
            lines.push(poly);
        }
        let est = estimate_lens_distortion_k1_k2(&lines);
        assert!(
            (est.k1 - true_k1).abs() < 0.03,
            "expected k1 ~ {true_k1}, got {}",
            est.k1
        );
    }

    #[test]
    fn mesh_morph_blends_warped_frames_smoothly() {
        let a = [1.0_f32, 0.0, 0.0, 1.0].repeat(4);
        let b = [0.0_f32, 0.0, 1.0, 1.0].repeat(4);
        let ga = WarpGrid::identity(3, 3);
        let gb = WarpGrid::identity(3, 3);
        let mid = morph_frames_f32(&a, &b, 2, 2, &ga, &gb, 0.5);
        assert!((mid[0] - 0.5).abs() < 1e-5 && (mid[2] - 0.5).abs() < 1e-5);
    }
}
