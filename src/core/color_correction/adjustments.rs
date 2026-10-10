// ── Vibrance ───────────────────────────────────────────────────────────────

/// Vibrance: boosts saturation of muted pixels far more than saturated ones
/// and partially protects warm skin-tone hues. `amount` spans −100..100
/// (negative desaturates).
pub fn apply_vibrance(pixels: &mut [u8], amount: f32) {
    let amt = if amount.is_finite() {
        (amount / 100.0).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    if amt == 0.0 || pixels.is_empty() {
        return;
    }
    for px in pixels.as_chunks_mut::<4>().0 {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        // 0..1 saturation proxy (chroma relative to intensity).
        let sat = (max - min) / (max + 1e-6);
        // Skin-tone guard applies to boosts only; desaturation is uniform so
        // negative amounts behave like a straightforward saturation pull.
        let boost = if amt >= 0.0 {
            amt * (1.0 - sat) * (1.0 - 0.5 * if r > g && g >= b { 1.0f32 } else { 0.0 })
        } else {
            amt
        };
        if boost.abs() < 1e-6 {
            continue;
        }
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let k = 1.0 + boost;
        for (c, slot) in [r, g, b].iter().zip(px.iter_mut().take(3)) {
            let v = luma + (c - luma) * k;
            *slot = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

// ── White Balance ──────────────────────────────────────────────────────────

/// Temperature/Tint white-balance sliders (−100..100 each).
/// Positive temperature warms (R↑ B↓); positive tint shifts magenta (G↓).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    pub temperature: f32,
    pub tint: f32,
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
        }
    }
}

pub fn apply_white_balance(pixels: &mut [u8], wb: &WhiteBalance) {
    let temperature = if wb.temperature.is_finite() {
        wb.temperature
    } else {
        0.0
    };
    let tint = if wb.tint.is_finite() { wb.tint } else { 0.0 };
    let t = (temperature / 100.0).clamp(-1.0, 1.0) * 0.25;
    let gshift = -(tint / 100.0).clamp(-1.0, 1.0) * 0.20;
    if (t == 0.0 && gshift == 0.0) || pixels.is_empty() {
        return;
    }
    let gains = [1.0 + t, 1.0 + gshift, 1.0 - t];
    for px in pixels.as_chunks_mut::<4>().0 {
        for (c, gain) in gains.iter().enumerate() {
            px[c] = ((px[c] as f32 * gain).clamp(0.0, 255.0)) as u8;
        }
    }
}

// ── HSL Adjust ─────────────────────────────────────────────────────────────

/// Lumetri-style three-way adjustment: hue rotation in degrees, saturation
/// gain (−100..100 mapped onto ×0..×2 around 1.0) and lightness shift
/// (−100..100 → ±0.5 additive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HslAdjust {
    /// Hue rotation in degrees (wraps).
    pub hue_deg: f32,
    /// Saturation slider −100..100.
    pub saturation: f32,
    /// Lightness slider −100..100.
    pub lightness: f32,
}

impl Default for HslAdjust {
    fn default() -> Self {
        Self {
            hue_deg: 0.0,
            saturation: 0.0,
            lightness: 0.0,
        }
    }
}

/// RGB (0..1) → HSL (h in degrees 0..360, s/l in 0..1).
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d).rem_euclid(6.0)
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// HSL → RGB (writes 0..1 triple).
fn hsl_to_rgb(h: f32, s: f32, l: f32, out: &mut [f32; 3]) {
    if s <= 1e-6 {
        *out = [l, l, l];
        return;
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let sector = (hp.floor() as i32).rem_euclid(6);
    let (r1, g1, b1) = match sector {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    *out = [r1 + m, g1 + m, b1 + m];
}

pub fn apply_hsl_adjust(pixels: &mut [u8], adj: &HslAdjust) {
    let hue = if adj.hue_deg.is_finite() {
        adj.hue_deg
    } else {
        0.0
    };
    let saturation = if adj.saturation.is_finite() {
        adj.saturation
    } else {
        0.0
    };
    let lightness = if adj.lightness.is_finite() {
        adj.lightness
    } else {
        0.0
    };
    let sat_mul = 1.0 + (saturation / 100.0).clamp(-1.0, 1.0);
    let l_shift = (lightness / 100.0).clamp(-1.0, 1.0) * 0.5;
    if hue.abs() < 1e-3 && (sat_mul - 1.0).abs() < 1e-6 && l_shift.abs() < 1e-6 {
        return;
    }
    for px in pixels.as_chunks_mut::<4>().0 {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;
        let (mut h, mut s, mut l) = rgb_to_hsl(r, g, b);
        h = (h + hue).rem_euclid(360.0);
        s = (s * sat_mul).clamp(0.0, 1.0);
        l = (l + l_shift).clamp(0.0, 1.0);
        let mut rgb = [0.0f32; 3];
        hsl_to_rgb(h, s, l, &mut rgb);
        for (c, slot) in rgb.iter().zip(px.iter_mut().take(3)) {
            *slot = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}
