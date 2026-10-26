use ab_glyph::FontRef;
use std::collections::HashMap;

/// Parses raw font bytes, supporting TrueType Collections (.ttc) via index fallback.
pub fn parse_font(data: &[u8]) -> Option<FontRef<'_>> {
    if let Ok(f) = FontRef::try_from_slice(data) {
        return Some(f);
    }
    for idx in 0..8u32 {
        if let Ok(f) = FontRef::try_from_slice_and_index(data, idx) {
            return Some(f);
        }
    }
    None
}

/// A rasterized glyph with its position and pixel data.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Left offset in pixels from the origin
    pub left: i32,
    /// Top offset in pixels from the baseline
    pub top: i32,
    /// Width of the glyph bitmap in pixels
    pub width: u32,
    /// Height of the glyph bitmap in pixels
    pub height: u32,
    /// RGBA pixel data (premultiplied alpha)
    pub pixels: Vec<u8>,
}

/// Synthetic text styles used when a font family does not expose a matching
/// face.  These are deliberately kept in the rasterizer so CPU and GPU text
/// paths can share exactly the same fallback behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct FauxTextStyle {
    pub bold: bool,
    pub italic: bool,
    pub all_caps: bool,
    pub small_caps: bool,
}

pub fn styled_text_input(text: &str, style: FauxTextStyle) -> String {
    if !style.all_caps && !style.small_caps {
        return text.to_string();
    }
    text.chars()
        .map(|ch| {
            if ch.is_alphabetic() {
                ch.to_uppercase().next().unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

pub fn apply_faux_bitmap_style(
    (mut width, mut height, mut pixels): (u32, u32, Vec<u8>),
    style: FauxTextStyle,
) -> (u32, u32, Vec<u8>) {
    if style.small_caps && width > 0 && height > 0 {
        let new_width = ((width as f32) * 0.84).round().max(1.0) as u32;
        let new_height = ((height as f32) * 0.84).round().max(1.0) as u32;
        let mut scaled = vec![0u8; (new_width * new_height * 4) as usize];
        for y in 0..new_height {
            for x in 0..new_width {
                let sx = ((x as f32 / new_width as f32) * width as f32)
                    .floor()
                    .min(width.saturating_sub(1) as f32) as u32;
                let sy = ((y as f32 / new_height as f32) * height as f32)
                    .floor()
                    .min(height.saturating_sub(1) as f32) as u32;
                let src = ((sy * width + sx) * 4) as usize;
                let dst = ((y * new_width + x) * 4) as usize;
                scaled[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
            }
        }
        width = new_width;
        height = new_height;
        pixels = scaled;
    }

    if style.bold {
        let source = pixels.clone();
        for y in 0..height {
            for x in 0..width {
                let mut best = 0u8;
                let mut best_src = 0usize;
                for oy in -1i32..=1 {
                    for ox in -1i32..=1 {
                        let sx = x as i32 + ox;
                        let sy = y as i32 + oy;
                        if sx >= 0 && sy >= 0 && sx < width as i32 && sy < height as i32 {
                            let idx = ((sy as u32 * width + sx as u32) * 4) as usize;
                            if source[idx + 3] > best {
                                best = source[idx + 3];
                                best_src = idx;
                            }
                        }
                    }
                }
                let dst = ((y * width + x) * 4) as usize;
                if best > 0 {
                    pixels[dst..dst + 3].copy_from_slice(&source[best_src..best_src + 3]);
                    pixels[dst + 3] = best;
                }
            }
        }
    }

    if style.italic && width > 0 && height > 0 {
        let shear = (height as f32 * 0.22).ceil() as u32;
        let new_width = width.saturating_add(shear).max(1);
        let mut slanted = vec![0u8; (new_width * height * 4) as usize];
        for y in 0..height {
            let shift = ((height - 1 - y) as f32 * 0.22).round() as u32;
            for x in 0..width {
                let dst_x = x.saturating_add(shift);
                let src = ((y * width + x) * 4) as usize;
                let dst = ((y * new_width + dst_x) * 4) as usize;
                slanted[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
            }
        }
        width = new_width;
        pixels = slanted;
    }

    (width, height, pixels)
}

pub struct GlyphCache {
    pub entries: HashMap<(String, char, u32), RasterizedGlyph>,
    pub bytes: usize,
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
        }
    }
}
