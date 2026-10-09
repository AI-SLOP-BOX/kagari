use super::types::{parse_font, RasterizedGlyph};
use ab_glyph::{Font, PxScale};

pub const MAX_GLYPH_BITMAP_BYTES: usize = 64 * 1024 * 1024;
pub const GLYPH_CACHE_BUDGET_BYTES: usize = 32 * 1024 * 1024;

/// Rasterize a single glyph uncached from raw font data.
pub fn rasterize_glyph_uncached(
    font_data: &[u8],
    ch: char,
    font_size: f32,
) -> Option<RasterizedGlyph> {
    let font = parse_font(font_data)?;

    let glyph_id = font.glyph_id(ch);
    let glyph =
        glyph_id.with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
    let outlined = font.outline_glyph(glyph)?;

    let bounds = outlined.px_bounds();
    let width = (bounds.max.x - bounds.min.x).max(0.0) as u32;
    let height = (bounds.max.y - bounds.min.y).max(0.0) as u32;

    if width == 0 || height == 0 {
        return Some(RasterizedGlyph {
            left: bounds.min.x as i32,
            top: bounds.min.y as i32,
            width: 0,
            height: 0,
            pixels: vec![],
        });
    }

    let pixel_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))?;
    if pixel_len > MAX_GLYPH_BITMAP_BYTES {
        return None;
    }
    let mut pixels = vec![0u8; pixel_len];
    outlined.draw(|x, y, coverage| {
        let idx = ((y * width + x) * 4) as usize;
        if idx + 3 < pixels.len() {
            let a = (coverage * 255.0) as u8;
            pixels[idx] = 255; // R
            pixels[idx + 1] = 255; // G
            pixels[idx + 2] = 255; // B
            pixels[idx + 3] = a; // A
        }
    });

    Some(RasterizedGlyph {
        left: bounds.min.x as i32,
        top: bounds.min.y as i32,
        width,
        height,
        pixels,
    })
}
