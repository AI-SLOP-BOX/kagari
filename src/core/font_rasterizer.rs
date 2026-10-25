//! Font rasterizer using ab_glyph for text layer rendering.
//!
//! Loads system fonts (or bundled fonts) and rasterizes individual glyphs
//! into RGBA pixel buffers that can be composited by the software renderer.

pub mod animator;
pub mod discovery;
pub mod global;
pub mod glyph;
pub mod rasterizer;
pub mod types;

pub use animator::{rasterize_text_animated, rasterize_text_animated_stack};
pub use discovery::{discover_system_fonts, system_font_paths};
pub use global::{
    init_font_rasterizer, with_font_rasterizer, with_font_rasterizer_mut, GLOBAL_FONT_RASTERIZER,
};
pub use glyph::{rasterize_glyph_uncached, GLYPH_CACHE_BUDGET_BYTES, MAX_GLYPH_BITMAP_BYTES};
pub use rasterizer::FontRasterizer;
pub use types::{
    apply_faux_bitmap_style, parse_font, styled_text_input, FauxTextStyle, GlyphCache,
    RasterizedGlyph,
};

/// Clip a rasterized text bitmap to a centered paragraph box height.
/// A height of zero or less leaves the bitmap unchanged.
pub(crate) fn clip_text_bitmap_height(
    (width, height, pixels): (u32, u32, Vec<u8>),
    box_height: f32,
) -> (u32, u32, Vec<u8>) {
    if !box_height.is_finite() || box_height <= 0.0 || width == 0 || height == 0 {
        return (width, height, pixels);
    }
    let clipped_height = (box_height.ceil() as u32).clamp(1, height);
    if clipped_height == height {
        return (width, height, pixels);
    }
    let first_row = (height - clipped_height) / 2;
    let start = (first_row as usize)
        .saturating_mul(width as usize)
        .saturating_mul(4)
        .min(pixels.len());
    let byte_count = (clipped_height as usize)
        .saturating_mul(width as usize)
        .saturating_mul(4)
        .min(pixels.len().saturating_sub(start));
    let cropped = pixels[start..start + byte_count].to_vec();
    (width, clipped_height, cropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_rasterizer_creation() {
        let r = FontRasterizer::new();
        assert!(r.available_families().is_empty());
    }

    #[test]
    fn test_load_nonexistent_font() {
        let mut r = FontRasterizer::new();
        assert!(!r.load_system_font("NonExistentFont12345"));
    }

    #[test]
    fn load_font_rejects_invalid_data_without_replacing_loaded_font() {
        let mut rasterizer = FontRasterizer::new();
        let valid = include_bytes!("../../assets/fonts/Inter.ttf").to_vec();
        assert!(rasterizer.load_font("Inter", valid.clone()));
        assert!(!rasterizer.load_font("Inter", vec![0, 1, 2, 3]));
        assert!(rasterizer.has_font("Inter"));
        assert!(rasterizer.load_font("Inter", valid));
    }

    #[test]
    fn faux_text_style_transforms_content_without_losing_layout_markers() {
        let styled = styled_text_input(
            "Kagari vfx 42\nα",
            FauxTextStyle {
                all_caps: true,
                ..Default::default()
            },
        );
        assert_eq!(styled, "KAGARI VFX 42\nΑ");
    }

    #[test]
    fn faux_bitmap_style_changes_only_requested_dimensions_and_alpha() {
        let pixels = vec![
            255, 255, 255, 0, 255, 255, 255, 0, // transparent row
            255, 255, 255, 255, 255, 255, 255, 0,
        ];
        let (width, height, bold) = apply_faux_bitmap_style(
            (2, 2, pixels.clone()),
            FauxTextStyle {
                bold: true,
                ..Default::default()
            },
        );
        assert_eq!((width, height), (2, 2));
        assert!(bold.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 255));

        let (italic_width, italic_height, _) = apply_faux_bitmap_style(
            (2, 2, pixels),
            FauxTextStyle {
                italic: true,
                ..Default::default()
            },
        );
        assert!(italic_width > 2);
        assert_eq!(italic_height, 2);
    }
}
