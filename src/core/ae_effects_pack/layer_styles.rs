use super::*;

// ── Inner Shadow / Inner Glow (layer styles) ──
/// Inner Shadow: darkens the inside edges of the layer's alpha, offset in
/// `angle_deg` (compass convention matching apply_drop_shadow) by `distance`,
/// softened by `softness`. Composites `shadow_color` under existing pixels.
pub fn apply_inner_shadow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    distance: f32,
    angle_deg: f32,
    softness: u32,
    shadow_color: [u8; 4],
) {
    if width == 0 || height == 0 {
        return;
    }
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(pixel_bytes) = pixel_count.checked_mul(4) else {
        return;
    };
    if pixels.len() != pixel_bytes {
        return;
    }
    let rad = angle_deg.to_radians();
    let dx = (rad.sin() * distance).round() as i32;
    let dy = (-rad.cos() * distance).round() as i32;

    // Mask M = A(x,y) × (1 − A(x−dx, y−dy)): the band of opaque pixels whose
    // back-shifted sample falls outside the shape.
    let mut mask = vec![0u8; pixel_count];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let a = pixels[idx + 3] as f32 / 255.0;
            if a <= 0.003 {
                continue;
            }
            let sx = (x as i32 - dx).clamp(0, width as i32 - 1) as u32;
            let sy = (y as i32 - dy).clamp(0, height as i32 - 1) as u32;
            let s_a = pixels[((sy * width + sx) * 4) as usize + 3] as f32 / 255.0;
            mask[(y * width + x) as usize] = (a * (1.0 - s_a) * 255.0).round() as u8;
        }
    }

    // Soften the band into an RGBA buffer so gaussian blur can run on alpha.
    let mut soft = vec![shadow_color[0]; pixel_bytes];
    for i in (0..soft.len()).step_by(4) {
        soft[i] = shadow_color[0];
        soft[i + 1] = shadow_color[1];
        soft[i + 2] = shadow_color[2];
        soft[i + 3] = mask[i / 4];
    }
    if softness > 0 {
        apply_gaussian_blur(&mut soft, width, height, softness);
    }

    // Under-composite: shadow sits beneath the original pixels but only where
    // the original is opaque (inner region), scaled by its own alpha.
    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        let sh_a = (soft[i + 3] as f32 / 255.0) * (shadow_color[3] as f32 / 255.0);
        let m = sh_a.min(1.0);
        if m <= 0.003 || base_a <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32 / 255.0;
            let sc = soft[i + c] as f32 / 255.0;
            // Multiply-ish darkening toward shadow color, preserving base alpha
            let blended = base * (1.0 - m) + sc * m;
            pixels[i + c] = (blended * 255.0).round() as u8;
        }
    }
}

/// Inner Glow: brightens the interior edges of the layer's alpha with
/// `glow_color`, falloff controlled by `size`, strength by `opacity` (0..100).
pub fn apply_inner_glow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    size: u32,
    glow_color: [u8; 4],
    opacity: f32,
) {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(pixel_bytes) = pixel_count.checked_mul(4) else {
        return;
    };
    if width == 0
        || height == 0
        || pixels.len() != pixel_bytes
        || !opacity.is_finite()
        || opacity <= 0.0
    {
        return;
    }
    let strength = (opacity / 100.0).clamp(0.0, 1.0);

    // Invert alpha, blur it, then re-mask by original alpha: brightest just
    // inside the edge, fading toward the interior.
    let mut inv = vec![0u8; pixel_bytes];
    for i in (0..inv.len()).step_by(4) {
        inv[i + 3] = 255 - pixels[i + 3];
    }
    if size > 0 {
        apply_gaussian_blur(&mut inv, width, height, size);
    }

    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        if base_a <= 0.003 {
            continue;
        }
        let edge = (inv[i + 3] as f32 / 255.0).min(1.0);
        let g = edge * strength * (glow_color[3] as f32 / 255.0);
        if g <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32;
            let gc = glow_color[c] as f32;
            // Additive-leaning screen blend keeps interiors luminous
            let screened = 255.0 - ((255.0 - base) * (255.0 - gc) / 255.0);
            pixels[i + c] = (base + (screened - base) * g).round() as u8;
        }
    }
}

// ── Bevel/Emboss (layer style) ──
#[derive(Debug, Clone, PartialEq)]
pub struct BevelEmbossParams {
    /// Light direction (degrees; 90 = light from the left per renderer convention)
    pub angle_deg: f32,
    pub depth_px: f32,
    pub size_px: u32,
    pub color_light: [u8; 4],
    pub color_dark: [u8; 4],
    pub highlight_strength: f32,
    pub shadow_strength: f32,
}

/// Directional edge lighting: samples alpha at ±depth along the light axis;
/// edges facing the light get the highlight tint, edges facing away get the
/// shadow tint. Gaussian-softened bands, alpha-weighted compositing.
pub fn apply_bevel_emboss(pixels: &mut [u8], width: u32, height: u32, p: &BevelEmbossParams) {
    let BevelEmbossParams {
        angle_deg,
        depth_px,
        size_px,
        color_light,
        color_dark,
        highlight_strength,
        shadow_strength,
    } = p;
    let depth_px = *depth_px;
    let size_px = *size_px;
    let highlight_strength = *highlight_strength;
    let shadow_strength = *shadow_strength;
    if width == 0 || height == 0 || pixels.is_empty() {
        return;
    }
    let hl = (highlight_strength / 100.0).clamp(0.0, 1.0);
    let sh = (shadow_strength / 100.0).clamp(0.0, 1.0);
    if hl <= 0.0 && sh <= 0.0 {
        return;
    }
    let rad = angle_deg.to_radians();
    // Light comes FROM this direction (compass): offset points back toward light
    let dx = (-rad.sin() * depth_px).round() as i32;
    let dy = (rad.cos() * depth_px).round() as i32;

    let sample_a = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return 0.0;
        }
        pixels[((y as u32 * width + x as u32) * 4) as usize + 3] as f32 / 255.0
    };

    // Two masks: edges whose toward-light sample is empty → lit side;
    // edges whose away-from-light sample is empty → shaded side.
    let mut mask_lit = vec![0u8; (width * height) as usize];
    let mut mask_shade = vec![0u8; (width * height) as usize];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let a = sample_a(x, y);
            if a <= 0.003 {
                continue;
            }
            let idx = (y as u32 * width + x as u32) as usize;
            let a_light = sample_a(x + dx, y + dy); // toward the light
            let a_away = sample_a(x - dx, y - dy); // away from the light
            mask_lit[idx] = (a * (1.0 - a_light) * 255.0).round() as u8;
            mask_shade[idx] = (a * (1.0 - a_away) * 255.0).round() as u8;
        }
    }

    let soften = |mask: &[u8], color: [u8; 4]| -> Vec<u8> {
        let mut soft = vec![0u8; (width * height * 4) as usize];
        for i in (0..soft.len()).step_by(4) {
            soft[i] = color[0];
            soft[i + 1] = color[1];
            soft[i + 2] = color[2];
            soft[i + 3] = mask[i / 4];
        }
        if size_px > 0 {
            apply_gaussian_blur(&mut soft, width, height, size_px);
        }
        soft
    };
    let lit_buf = soften(&mask_lit, *color_light);
    let shade_buf = soften(&mask_shade, *color_dark);

    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        if base_a <= 0.003 {
            continue;
        }
        let m_hl = ((lit_buf[i + 3] as f32 / 255.0) * hl).min(1.0);
        let m_sh = ((shade_buf[i + 3] as f32 / 255.0) * sh).min(1.0);
        // Shade first, then highlight wins where both overlap
        for c in 0..3 {
            let base = pixels[i + c] as f32 / 255.0;
            let dark_c = shade_buf[i + c] as f32 / 255.0;
            let light_c = lit_buf[i + c] as f32 / 255.0;
            let v = base * (1.0 - m_sh) + dark_c * m_sh;
            let v = v * (1.0 - m_hl) + light_c * m_hl;
            pixels[i + c] = (v * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

// ── Satin (layer style) ──
/// Satin parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct SatinParams {
    /// Offset of the sheen source (px)
    pub distance: f32,
    /// Sheen direction in degrees (compass convention: 0 = up, 90 = right)
    pub angle_deg: f32,
    /// Softness of the band (gaussian radius px)
    pub size: u32,
    /// Tint color RGBA
    pub color: [u8; 4],
    /// Blend strength (0..100)
    pub opacity: f32,
}

/// Satin: interior sheen band. Shifts the inverted alpha by the compass
/// offset, intersects it with the original shape, blurs, then tints.
pub fn apply_satin(pixels: &mut [u8], width: u32, height: u32, p: &SatinParams) {
    if width == 0 || height == 0 || p.opacity <= 0.0 {
        return;
    }
    let strength = (p.opacity / 100.0).clamp(0.0, 1.0);
    let rad = p.angle_deg.to_radians();
    let dx = (rad.sin() * p.distance).round() as i32;
    let dy = (-rad.cos() * p.distance).round() as i32;

    // Band M(x,y) = A(x,y) × Ainv_shifted(x,y): opaque pixels whose
    // forward-shifted sample falls outside the shape.
    let mut mask = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let a = pixels[idx + 3] as f32 / 255.0;
            if a <= 0.003 {
                continue;
            }
            let sx = (x as i32 - dx).clamp(0, width as i32 - 1) as u32;
            let sy = (y as i32 - dy).clamp(0, height as i32 - 1) as u32;
            let s_a = pixels[((sy * width + sx) * 4) as usize + 3] as f32 / 255.0;
            // Inverted shifted alpha intersected with the shape
            mask[(y * width + x) as usize] = (a * (1.0 - s_a) * 255.0).round() as u8;
        }
    }

    let mut soft = vec![0u8; (width * height * 4) as usize];
    for i in (0..soft.len()).step_by(4) {
        soft[i] = p.color[0];
        soft[i + 1] = p.color[1];
        soft[i + 2] = p.color[2];
        soft[i + 3] = mask[i / 4];
    }
    if p.size > 0 {
        apply_gaussian_blur(&mut soft, width, height, p.size);
    }

    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        let m = ((soft[i + 3] as f32 / 255.0) * (p.color[3] as f32 / 255.0)).min(1.0) * strength;
        if m <= 0.003 || base_a <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32 / 255.0;
            let sc = soft[i + c] as f32 / 255.0;
            pixels[i + c] = ((base * (1.0 - m) + sc * m) * 255.0).round() as u8;
        }
    }
}

// ── Layer Style Overlays ──
/// Solid Color Overlay: blends every pixel toward `color` by `opacity` (0..100),
/// weighted by existing alpha so transparent areas stay transparent.
pub fn apply_color_overlay(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    color: [u8; 4],
    opacity: f32,
) {
    if opacity <= 0.0 {
        return;
    }
    let mix = (opacity / 100.0).clamp(0.0, 1.0);
    for px in pixels.as_chunks_mut::<4>().0 {
        let a = px[3] as f32 / 255.0;
        if a < 0.003 {
            continue;
        }
        px[0] = (px[0] as f32 + (color[0] as f32 - px[0] as f32) * mix) as u8;
        px[1] = (px[1] as f32 + (color[1] as f32 - px[1] as f32) * mix) as u8;
        px[2] = (px[2] as f32 + (color[2] as f32 - px[2] as f32) * mix) as u8;
    }
}

/// Parameters for [`apply_gradient_overlay`].
#[derive(Debug, Clone, PartialEq)]
pub struct GradientOverlayParams {
    /// Gradient direction in degrees (0 = left→right, 90 = bottom→top)
    pub angle_deg: f32,
    /// Gradient length as % of layer bounding extent
    pub scale_pct: f32,
    /// Gradient start / end colors
    pub start: [u8; 4],
    pub end: [u8; 4],
    /// Blend strength (0..100)
    pub opacity: f32,
}

/// Linear Gradient Overlay: interpolates start→end along `angle_deg`
/// (0 = left→right, 90 = bottom→top), spanning `scale_pct` of the layer
/// extent, blended at `opacity` over existing alpha.
pub fn apply_gradient_overlay(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p: &GradientOverlayParams,
) {
    if width == 0 || height == 0 || p.opacity <= 0.0 {
        return;
    }
    let mix_max = (p.opacity / 100.0).clamp(0.0, 1.0);
    let rad = p.angle_deg.to_radians();
    // Screen-space direction: 0° → +x, 90° → up (-y)
    let dx = rad.cos();
    let dy = -rad.sin();
    let w = width as f32;
    let h = height as f32;

    // Project the four corners to find the gradient extent
    let mut t_min = f32::INFINITY;
    let mut t_max = f32::NEG_INFINITY;
    for (cx, cy) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
        let t = cx * dx + cy * dy;
        t_min = t_min.min(t);
        t_max = t_max.max(t);
    }
    let span = ((t_max - t_min) * (p.scale_pct / 100.0).clamp(0.05, 4.0)).max(1e-3);
    let mid = (t_min + t_max) * 0.5;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let a = pixels[idx + 3] as f32 / 255.0;
            if a < 0.003 {
                continue;
            }
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = (((px * dx + py * dy) - mid) / span + 0.5).clamp(0.0, 1.0);
            let or = p.start[0] as f32 + (p.end[0] as f32 - p.start[0] as f32) * t;
            let og = p.start[1] as f32 + (p.end[1] as f32 - p.start[1] as f32) * t;
            let ob = p.start[2] as f32 + (p.end[2] as f32 - p.start[2] as f32) * t;
            let oa = p.start[3] as f32 + (p.end[3] as f32 - p.start[3] as f32) * t;
            // Blend overlay color at (mix × overlay alpha)
            let m = mix_max * (oa / 255.0);
            pixels[idx] = (pixels[idx] as f32 + (or - pixels[idx] as f32) * m) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 + (og - pixels[idx + 1] as f32) * m) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 + (ob - pixels[idx + 2] as f32) * m) as u8;
        }
    }
}

// ── Lens Flare (CPU mirror of the WGSL screen-space flare) ──
/// Screen-space optical flare parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct LensFlareParams {
    /// Normalized source position (0..1)
    pub pos_x: f32,
    pub pos_y: f32,
    /// Overall brightness multiplier
    pub intensity: f32,
    /// Output gain applied after element summation
    pub threshold: f32,
    /// Tint color (0-255 RGB)
    pub color: [u8; 3],
}

/// Screen-space optical flare: Gaussian core + concentric ring artifacts +
/// 4-point star streaks, additively tinted onto the buffer.
/// Mirrors `shader.wgsl` fs_main flare pass for GPU/CPU export parity.
pub fn apply_lens_flare(pixels: &mut [u8], width: u32, height: u32, p: &LensFlareParams) {
    if width == 0 || height == 0 || p.intensity <= 0.0 {
        return;
    }
    let w = width as f32;
    let h = height as f32;
    let cx = p.pos_x.clamp(0.0, 1.0);
    let cy = p.pos_y.clamp(0.0, 1.0);
    let intensity = p.intensity;
    let threshold = p.threshold;
    let cr = p.color[0] as f32 / 255.0;
    let cg = p.color[1] as f32 / 255.0;
    let cb = p.color[2] as f32 / 255.0;

    for y in 0..height {
        let ny = (y as f32 + 0.5) / h;
        for x in 0..width {
            let nx = (x as f32 + 0.5) / w;

            let dx = nx - cx;
            let dy = ny - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let d_norm = d * 2.0;

            // Core glow
            let core_radius = 0.02;
            let core = (-d_norm * d_norm / (core_radius * core_radius)).exp() * intensity;

            // Concentric rings fading with distance
            let ring_phase = d_norm * 12.0;
            let ring = (ring_phase.sin() * 0.3) + 0.5;
            let ring_mask = (-d_norm * 3.0).exp();
            let rings = ring * ring_mask * intensity * 0.3;

            // Horizontal + vertical star streaks
            let streak_h = (-dy.abs() * 80.0).exp() * (-dx.abs() * 8.0).exp();
            let streak_v = (-dx.abs() * 80.0).exp() * (-dy.abs() * 8.0).exp();
            let streaks = (streak_h + streak_v) * intensity * 0.4;

            let flare = ((core + rings + streaks) * threshold).min(4.0);
            if flare < 0.002 {
                continue;
            }

            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let r = pixels[idx] as f32 + cr * flare * 255.0;
            let g = pixels[idx + 1] as f32 + cg * flare * 255.0;
            let b = pixels[idx + 2] as f32 + cb * flare * 255.0;
            pixels[idx] = r.min(255.0) as u8;
            pixels[idx + 1] = g.min(255.0) as u8;
            pixels[idx + 2] = b.min(255.0) as u8;
        }
    }
}

// ── Camera Shake ──
/// Applies procedural camera shake to a position value.
/// Returns offset [dx, dy] for the given time.
pub fn camera_shake(time_sec: f32, intensity: f32, speed_hz: f32, seed: u64) -> [f32; 2] {
    let t = time_sec * speed_hz;
    let s = seed as f32;
    let dx = (t * 1.7 + s).sin() * 0.6 + (t * 3.3 + s * 1.7).sin() * 0.4;
    let dy = (t * 2.1 + s * 2.3).sin() * 0.5 + (t * 4.7 + s * 0.9).sin() * 0.5;
    [dx * intensity, dy * intensity]
}
