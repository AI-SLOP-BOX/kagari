use ab_glyph::FontRef;
use std::collections::HashMap;

pub const MAX_TEXT_BITMAP_BYTES: usize = 256 * 1024 * 1024;

pub(super) fn checked_text_bitmap_len(width: u32, height: u32) -> Option<usize> {
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= MAX_TEXT_BITMAP_BYTES).then_some(bytes)
}

pub(super) fn checked_text_dimensions(width: f32, height: f32, padding: u32) -> Option<(u32, u32)> {
    if !width.is_finite() || !height.is_finite() || width < 0.0 || height < 0.0 {
        return None;
    }
    let padding = padding as f32 * 2.0;
    let width = width.ceil() + padding;
    let height = height.ceil() + padding;
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return None;
    }
    let dimensions = (width.max(1.0) as u32, height.max(1.0) as u32);
    if dimensions.0 > crate::core::software_renderer::MAX_RENDER_DIMENSION
        || dimensions.1 > crate::core::software_renderer::MAX_RENDER_DIMENSION
    {
        return None;
    }
    checked_text_bitmap_len(dimensions.0, dimensions.1).map(|_| dimensions)
}

pub(super) fn zeroed_text_bitmap(width: u32, height: u32) -> Option<Vec<u8>> {
    let byte_len = checked_text_bitmap_len(width, height)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(byte_len).ok()?;
    pixels.resize(byte_len, 0);
    Some(pixels)
}

pub(super) fn checked_animated_glyph_pixels(width: usize, height: usize) -> Option<usize> {
    let pixel_count = width.checked_mul(height)?;
    let byte_count = pixel_count.checked_mul(4)?;
    (byte_count <= super::glyph::MAX_GLYPH_BITMAP_BYTES).then_some(pixel_count)
}

pub(super) fn zeroed_glyph_coverage(pixel_count: usize) -> Option<Vec<f32>> {
    let bytes = pixel_count.checked_mul(std::mem::size_of::<f32>())?;
    if bytes > super::glyph::MAX_GLYPH_BITMAP_BYTES {
        return None;
    }
    let mut coverage = Vec::new();
    coverage.try_reserve_exact(pixel_count).ok()?;
    coverage.resize(pixel_count, 0.0);
    Some(coverage)
}

pub(super) fn zeroed_glyph_bitmap(width: usize, height: usize) -> Option<Vec<u8>> {
    let pixels = checked_animated_glyph_pixels(width, height)?;
    let byte_count = pixels.checked_mul(4)?;
    let mut bitmap = Vec::new();
    bitmap.try_reserve_exact(byte_count).ok()?;
    bitmap.resize(byte_count, 0);
    Some(bitmap)
}

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
    /// RGBA pixel data with straight (unpremultiplied) alpha
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
    let Some(input_len) = checked_text_bitmap_len(width, height) else {
        return (width, height, pixels);
    };
    if pixels.len() < input_len {
        return (width, height, pixels);
    }
    pixels.truncate(input_len);

    if style.small_caps && width > 0 && height > 0 {
        let new_width = ((width as f32) * 0.84).round().max(1.0) as u32;
        let new_height = ((height as f32) * 0.84).round().max(1.0) as u32;
        if checked_text_bitmap_len(new_width, new_height).is_some() {
            let Some(mut scaled) = zeroed_text_bitmap(new_width, new_height) else {
                return (width, height, pixels);
            };
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
    }

    if style.bold {
        let mut source = Vec::new();
        if source.try_reserve_exact(pixels.len()).is_err() {
            return (width, height, pixels);
        }
        source.extend_from_slice(&pixels);
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
        if let Some(new_width) = width.checked_add(shear) {
            if checked_text_bitmap_len(new_width, height).is_some() {
                let Some(mut slanted) = zeroed_text_bitmap(new_width, height) else {
                    return (width, height, pixels);
                };
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
        }
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
