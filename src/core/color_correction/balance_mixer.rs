const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

pub(crate) fn rec709_luma(r: f32, g: f32, b: f32) -> f32 {
    r * LUMA_R + g * LUMA_G + b * LUMA_B
}

/// Three-way color balance (shadows / midtones / highlights color shifts).
///
/// Shifts are in -100..100 per channel; `preserve_luminosity` keeps the
/// Rec.709 luma constant after grading (AE "Preserve Luminosity").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorBalance {
    pub shadows: [f32; 3],
    pub midtones: [f32; 3],
    pub highlights: [f32; 3],
    pub preserve_luminosity: bool,
}

impl Default for ColorBalance {
    fn default() -> Self {
        Self {
            shadows: [0.0; 3],
            midtones: [0.0; 3],
            highlights: [0.0; 3],
            preserve_luminosity: true,
        }
    }
}

/// Apply three-way Color Balance to an RGBA8 buffer.
pub fn apply_color_balance(pixels: &mut [u8], cb: &ColorBalance) {
    if pixels.is_empty() {
        return;
    }
    let scale = 127.0 / 100.0; // slider ±100 → ±~half channel range

    for px in pixels.as_chunks_mut::<4>().0 {
        let rf = px[0] as f32;
        let gf = px[1] as f32;
        let bf = px[2] as f32;
        let luma01 = rec709_luma(rf, gf, bf) / 255.0;

        // Quadratic Bernstein weights: sum to 1 across tonal ranges.
        let w_shadow = (1.0 - luma01) * (1.0 - luma01);
        let w_high = luma01 * luma01;
        let w_mid = 2.0 * luma01 * (1.0 - luma01);

        let shift = |i: usize| -> f32 {
            (cb.shadows[i] * w_shadow + cb.midtones[i] * w_mid + cb.highlights[i] * w_high) * scale
        };

        let mut out_r = (rf + shift(0)).clamp(0.0, 255.0);
        let mut out_g = (gf + shift(1)).clamp(0.0, 255.0);
        let mut out_b = (bf + shift(2)).clamp(0.0, 255.0);

        if cb.preserve_luminosity {
            let luma_in = rec709_luma(rf, gf, bf);
            let luma_out = rec709_luma(out_r, out_g, out_b);
            let correction = luma_in - luma_out;
            out_r = (out_r + correction).clamp(0.0, 255.0);
            out_g = (out_g + correction).clamp(0.0, 255.0);
            out_b = (out_b + correction).clamp(0.0, 255.0);
        }

        px[0] = out_r.round() as u8;
        px[1] = out_g.round() as u8;
        px[2] = out_b.round() as u8;
    }
}

/// Channel Mixer: each output channel is a weighted mix of input channels.
///
/// Weights are percentages (100 = full channel). `monochrome` collapses the
/// image using the red row weights (Photoshop behaviour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelMixer {
    /// Output red = row 0 · [r, g, b], etc. Percent units.
    pub matrix: [[f32; 3]; 3],
    pub monochrome: bool,
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self {
            matrix: [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0]],
            monochrome: false,
        }
    }
}

/// 256-bin luma histogram (Rec.709) for scopes / Lumetri displays.
pub fn compute_luma_histogram(pixels: &[u8]) -> [u32; 256] {
    let mut bins = [0u32; 256];
    for px in pixels.as_chunks::<4>().0 {
        let l = rec709_luma(px[0] as f32, px[1] as f32, px[2] as f32);
        let bin = l.round().clamp(0.0, 255.0) as usize;
        bins[bin] += 1;
    }
    bins
}

/// Per-channel RGB histograms `[r_bins, g_bins, b_bins]` for parade scopes.
pub fn compute_rgb_histograms(pixels: &[u8]) -> [[u32; 256]; 3] {
    let mut out = [[0u32; 256]; 3];
    for px in pixels.as_chunks::<4>().0 {
        for c in 0..3 {
            out[c][px[c] as usize] += 1;
        }
    }
    out
}

/// Apply the Channel Mixer to an RGBA8 buffer.
pub fn apply_channel_mixer(pixels: &mut [u8], mixer: &ChannelMixer) {
    if pixels.is_empty() {
        return;
    }
    let m = &mixer.matrix;

    for px in pixels.as_chunks_mut::<4>().0 {
        let r = px[0] as f32;
        let g = px[1] as f32;
        let b = px[2] as f32;

        if mixer.monochrome {
            let gray = (r * m[0][0] + g * m[0][1] + b * m[0][2]) / 100.0;
            let v = gray.clamp(0.0, 255.0).round() as u8;
            px[0] = v;
            px[1] = v;
            px[2] = v;
        } else {
            let or_ = (r * m[0][0] + g * m[0][1] + b * m[0][2]) / 100.0;
            let og = (r * m[1][0] + g * m[1][1] + b * m[1][2]) / 100.0;
            let ob = (r * m[2][0] + g * m[2][1] + b * m[2][2]) / 100.0;
            px[0] = or_.clamp(0.0, 255.0).round() as u8;
            px[1] = og.clamp(0.0, 255.0).round() as u8;
            px[2] = ob.clamp(0.0, 255.0).round() as u8;
        }
    }
}
