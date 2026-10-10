use std::f32::consts::TAU;
use super::wave_warp::sample_bilinear;

/// Parameters for [`apply_cc_lens_pro`].
#[derive(Debug, Clone, Copy)]
pub struct CcLensParams {
    /// -100 (fisheye out) .. 100 (bulge in), AE-style.
    pub convergence: f32,
    /// Output zoom factor (1.0 = neutral).
    pub zoom: f32,
}

impl Default for CcLensParams {
    fn default() -> Self {
        Self {
            convergence: 50.0,
            zoom: 1.0,
        }
    }
}

/// True radial fisheye: source radius follows `rn^(1 + k)` so the centre is
/// magnified for positive convergence and compressed for negative. Uses
/// bilinear sampling; the exact centre pixel is invariant.
pub fn apply_cc_lens_pro(pixels: &mut [u8], width: u32, height: u32, p: &CcLensParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = (width - 1) as f32 * 0.5;
    let cy = (height - 1) as f32 * 0.5;
    let rmax = (cx * cx + cy * cy).sqrt().max(1.0);
    let k = (p.convergence / 100.0).clamp(-0.95, 0.95);
    let zoom = p.zoom.max(0.01);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let rn = ((rx * rx + ry * ry).sqrt() / rmax * zoom).clamp(0.0, 1.0);
            let src_rn = rn.powf(1.0 + k);
            let scale = if rn > 1e-6 { src_rn / rn } else { 1.0 };
            let mut rgba = [0u8; 4];
            sample_bilinear(
                &temp,
                width,
                height,
                cx + rx * scale,
                cy + ry * scale,
                &mut rgba,
            );
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

/// Conversion direction for [`apply_polar_coordinates_pro`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarMode {
    RectToPolar,
    PolarToRect,
}

/// AE-style Polar Coordinates with an interpolation blend between the
/// original image and the fully converted result.
///
/// * `RectToPolar`: the vertical axis becomes the radius, the horizontal
///   axis sweeps a full circle around the image centre.
/// * `PolarToRect`: the exact inverse mapping.
pub fn apply_polar_coordinates_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    mode: PolarMode,
    interpolation: f32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let fw = width as f32;
    let fh = height as f32;
    let cx = fw * 0.5;
    let cy = fh * 0.5;
    let mix = interpolation.clamp(0.0, 1.0);

    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = match mode {
                PolarMode::RectToPolar => {
                    let a = (x as f32 / fw) * TAU;
                    let r = cy - y as f32; // vertical distance from centre = radius
                    (cx + a.cos() * r, cy + a.sin() * r)
                }
                PolarMode::PolarToRect => {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let r = (dx * dx + dy * dy).sqrt();
                    let a = dy.atan2(dx).rem_euclid(TAU);
                    (a / TAU * fw, cy - r)
                }
            };
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, sx, sy, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            if mix < 1.0 {
                for c in 0..4 {
                    let orig = temp[idx + c] as f32;
                    rgba[c] = (orig + (rgba[c] as f32 - orig) * mix)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

/// Parameters for [`apply_optics_compensation`].
#[derive(Debug, Clone, Copy)]
pub struct OpticsCompensationParams {
    /// Field of view in degrees. Positive = barrel distortion removal
    /// (edge squeeze), negative = pincushion. 0 = identity.
    pub field_of_view_deg: f32,
    /// Reverse the distortion direction (AE "Reverse Lens Distortion").
    pub reverse: bool,
    /// Output zoom factor (1.0 = neutral).
    pub zoom: f32,
}

impl Default for OpticsCompensationParams {
    fn default() -> Self {
        Self {
            field_of_view_deg: 0.0,
            reverse: false,
            zoom: 1.0,
        }
    }
}

/// Radial lens distortion model `src_rn = rn * (1 + c * rn^2)` where the
/// curvature is derived from the field of view. Bilinear sampled.
pub fn apply_optics_compensation(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p: &OpticsCompensationParams,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = (width - 1) as f32 * 0.5;
    let cy = (height - 1) as f32 * 0.5;
    let rmax = (cx * cx + cy * cy).sqrt().max(1.0);
    let fov = p.field_of_view_deg.clamp(-179.0, 179.0);
    let mut c = (fov.abs() / 180.0).powi(2) * 0.9 * fov.signum();
    if p.reverse {
        c = -c;
    }
    let zoom = p.zoom.max(0.01);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let rn = ((rx * rx + ry * ry).sqrt() / rmax * zoom).clamp(0.0, 1.0);
            let src_rn = (rn * (1.0 + c * rn * rn)).clamp(0.0, 1.0);
            let scale = if rn > 1e-6 { src_rn / rn } else { 1.0 };
            let mut rgba = [0u8; 4];
            sample_bilinear(
                &temp,
                width,
                height,
                cx + rx * scale,
                cy + ry * scale,
                &mut rgba,
            );
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}
