use super::color_spaces::{linear_to_srgb, srgb_to_linear};

/// Project / Composition color bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum BitDepth {
    #[default]
    EightBit, // 8bpc (0..255 integer)
    SixteenBit,        // 16bpc Half Float (unclamped HDR)
    ThirtyTwoBitFloat, // 32bpc Full Float (infinite dynamic range, scene-linear)
}

impl BitDepth {
    pub fn is_hdr(&self) -> bool {
        matches!(self, BitDepth::SixteenBit | BitDepth::ThirtyTwoBitFloat)
    }

    pub fn is_32bpc(&self) -> bool {
        matches!(self, BitDepth::ThirtyTwoBitFloat)
    }

    pub fn label(&self) -> &'static str {
        match self {
            BitDepth::EightBit => "8 bpc",
            BitDepth::SixteenBit => "16 bpc",
            BitDepth::ThirtyTwoBitFloat => "32 bpc (float)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            BitDepth::EightBit => "8bpc",
            BitDepth::SixteenBit => "16bpc",
            BitDepth::ThirtyTwoBitFloat => "32bpc",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            BitDepth::EightBit => BitDepth::SixteenBit,
            BitDepth::SixteenBit => BitDepth::ThirtyTwoBitFloat,
            BitDepth::ThirtyTwoBitFloat => BitDepth::EightBit,
        }
    }
}

/// High Dynamic Range (HDR) 32bpc / 16bpc scene-linear float pixel buffer.
/// Stores RGBA components as 32-bit floats with unclamped range (-inf, +inf).
#[derive(Debug, Clone)]
pub struct HdrF32Buffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>, // 4 floats per pixel: [R, G, B, A]
}

impl HdrF32Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![0.0; len],
        }
    }

    /// Creates an HDR float buffer from an 8-bit sRGB RGBA slice.
    pub fn from_rgba8(pixels: &[u8], width: u32, height: u32) -> Self {
        let mut buffer = Self::new(width, height);
        for (i, chunk) in pixels.as_chunks::<4>().0.iter().enumerate() {
            let base = i * 4;
            if base + 3 < buffer.data.len() {
                buffer.data[base] = srgb_to_linear(chunk[0] as f32 / 255.0);
                buffer.data[base + 1] = srgb_to_linear(chunk[1] as f32 / 255.0);
                buffer.data[base + 2] = srgb_to_linear(chunk[2] as f32 / 255.0);
                buffer.data[base + 3] = chunk[3] as f32 / 255.0;
            }
        }
        buffer
    }

    /// Converts HDR float buffer back to 8bpc sRGB for display / 8-bit export.
    /// Supports exposure EV offset and optional ACES filmic tonemapping.
    pub fn to_rgba8(&self, tonemap: bool, exposure: f32) -> Vec<u8> {
        self.to_rgba8_dithered(tonemap, exposure, false)
    }

    /// Converts HDR float buffer to 8bpc sRGB with optional TPDF dithering.
    pub fn to_rgba8_dithered(&self, tonemap: bool, exposure: f32, dither: bool) -> Vec<u8> {
        let num_pixels = (self.width as usize) * (self.height as usize);
        let mut out = vec![0u8; num_pixels * 4];
        let exp_factor = 2.0f32.powf(exposure);

        for (i, chunk) in self.data.as_chunks::<4>().0.iter().enumerate() {
            let base = i * 4;
            let mut r = chunk[0] * exp_factor;
            let mut g = chunk[1] * exp_factor;
            let mut b = chunk[2] * exp_factor;
            let a = chunk[3].clamp(0.0, 1.0);

            if tonemap {
                // ACES / Reinhard filmic tonemapping for HDR highlights
                r = r / (1.0 + r.max(0.0));
                g = g / (1.0 + g.max(0.0));
                b = b / (1.0 + b.max(0.0));
            }

            let sr = linear_to_srgb(r);
            let sg = linear_to_srgb(g);
            let sb = linear_to_srgb(b);

            if dither {
                let seed = i as f32 * 0.618_034;
                let t1 = (seed * 7.13).fract();
                let t2 = (seed * 3.71).fract();
                let noise = (t1 - t2) / 255.0;
                out[base] = ((sr + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 1] = ((sg + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 2] = ((sb + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
            } else {
                out[base] = (sr.clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 1] = (sg.clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 2] = (sb.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            out[base + 3] = (a * 255.0).round() as u8;
        }
        out
    }

    /// Alpha blend source layer over this buffer in scene-linear float space.
    pub fn blend_over(&mut self, src: &HdrF32Buffer, opacity: f32) {
        if self.width != src.width || self.height != src.height {
            return;
        }
        let op = opacity.clamp(0.0, 1.0);
        for i in 0..(self.data.len() / 4) {
            let base = i * 4;
            let src_a = src.data[base + 3] * op;
            let dst_a = self.data[base + 3];
            let out_a = src_a + dst_a * (1.0 - src_a);

            if out_a > 1e-6 {
                for c in 0..3 {
                    let src_c = src.data[base + c];
                    let dst_c = self.data[base + c];
                    self.data[base + c] = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
                }
                self.data[base + 3] = out_a;
            }
        }
    }

    /// Full 32bpc floating-point blend mode compositor.
    /// Blends `src` over `self` with arbitrary blend modes in unbounded float space.
    pub fn blend_layer_mode(
        &mut self,
        src: &HdrF32Buffer,
        opacity: f32,
        mode: crate::core::timeline::BlendMode,
    ) {
        if self.width != src.width || self.height != src.height {
            return;
        }
        let op = opacity.clamp(0.0, 1.0);
        for i in 0..(self.data.len() / 4) {
            let base = i * 4;
            let src_a = (src.data[base + 3] * op).clamp(0.0, 1.0);
            if src_a <= 1e-6 {
                continue;
            }
            let dst_a = self.data[base + 3].clamp(0.0, 1.0);

            for c in 0..3 {
                let s = src.data[base + c];
                let d = self.data[base + c];
                let blended = match mode {
                    crate::core::timeline::BlendMode::Normal => s,
                    crate::core::timeline::BlendMode::Add
                    | crate::core::timeline::BlendMode::LinearDodge => s + d,
                    crate::core::timeline::BlendMode::Multiply => s * d,
                    crate::core::timeline::BlendMode::Screen => s + d - s * d,
                    crate::core::timeline::BlendMode::Overlay => {
                        if d < 0.5 {
                            2.0 * s * d
                        } else {
                            1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                        }
                    }
                    crate::core::timeline::BlendMode::HardLight => {
                        if s < 0.5 {
                            2.0 * s * d
                        } else {
                            1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                        }
                    }
                    crate::core::timeline::BlendMode::SoftLight => {
                        if s <= 0.5 {
                            d - (1.0 - 2.0 * s) * d * (1.0 - d)
                        } else {
                            let root_d = if d > 0.0 { d.sqrt() } else { 0.0 };
                            d + (2.0 * s - 1.0) * (root_d - d)
                        }
                    }
                    crate::core::timeline::BlendMode::Difference => (s - d).abs(),
                    crate::core::timeline::BlendMode::Exclusion => s + d - 2.0 * s * d,
                    crate::core::timeline::BlendMode::Subtract => (d - s).max(0.0),
                    crate::core::timeline::BlendMode::Darken => s.min(d),
                    crate::core::timeline::BlendMode::Lighten => s.max(d),
                    crate::core::timeline::BlendMode::ColorDodge => {
                        if (1.0 - s).abs() < 1e-5 {
                            10.0
                        } else {
                            d / (1.0 - s).max(1e-5)
                        }
                    }
                    crate::core::timeline::BlendMode::ColorBurn => {
                        if s.abs() < 1e-5 {
                            0.0
                        } else {
                            1.0 - (1.0 - d) / s.max(1e-5)
                        }
                    }
                    _ => s,
                };

                let out_a = src_a + dst_a * (1.0 - src_a);
                if out_a > 1e-6 {
                    self.data[base + c] = (blended * src_a + d * dst_a * (1.0 - src_a)) / out_a;
                }
            }
            self.data[base + 3] = (src_a + dst_a * (1.0 - src_a)).clamp(0.0, 1.0);
        }
    }
}
