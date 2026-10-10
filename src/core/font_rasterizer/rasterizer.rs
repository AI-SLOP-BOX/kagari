use super::animator::{rasterize_text_animated, rasterize_text_animated_stack};
use super::discovery::{discover_system_fonts, system_font_paths};
use super::glyph::{rasterize_glyph_uncached, GLYPH_CACHE_BUDGET_BYTES};
use super::types::{
    apply_faux_bitmap_style, parse_font, styled_text_input, FauxTextStyle, GlyphCache,
    RasterizedGlyph,
};
use ab_glyph::{Font, PxScale, ScaleFont};
use std::collections::HashMap;

/// Font rasterizer that caches loaded fonts and rasterized glyphs.
pub struct FontRasterizer {
    /// Loaded fonts keyed by family name
    fonts: HashMap<String, Vec<u8>>,
    /// Glyph coverage bitmaps keyed by (family, char, font-size bits).
    glyph_cache: std::cell::RefCell<GlyphCache>,
}

impl Default for FontRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl FontRasterizer {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            glyph_cache: std::cell::RefCell::new(GlyphCache::new()),
        }
    }

    /// Load a font from bytes. Returns true if successful.
    pub fn load_font(&mut self, family_name: &str, font_data: Vec<u8>) -> bool {
        if family_name.is_empty() || parse_font(&font_data).is_none() {
            return false;
        }
        self.fonts.insert(family_name.to_string(), font_data);
        let mut cache = self.glyph_cache.borrow_mut();
        cache.entries.clear();
        cache.bytes = 0;
        true
    }

    /// Load a system font by name. Searches common system font paths.
    pub fn load_system_font(&mut self, family_name: &str) -> bool {
        if self.fonts.contains_key(family_name) {
            return true;
        }

        let paths = system_font_paths(family_name);
        for path in &paths {
            if path.exists() {
                if let Ok(data) = std::fs::read(path) {
                    if parse_font(&data).is_some() {
                        self.fonts.insert(family_name.to_string(), data);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a font is loaded.
    pub fn has_font(&self, family_name: &str) -> bool {
        self.fonts.contains_key(family_name)
    }

    /// Returns the requested family if available, otherwise a deterministic
    /// fallback (preferred list, then lexicographically smallest loaded family).
    pub fn resolve_family(&self, family_name: &str) -> String {
        if self.fonts.contains_key(family_name) {
            return family_name.to_string();
        }
        const PREFERRED: &[&str] = &[
            "Helvetica",
            "Helvetica Neue",
            "Arial",
            "Inter",
            "Roboto",
            "DejaVu Sans",
            "Liberation Sans",
            "Verdana",
            "Tahoma",
            "Menlo",
        ];
        if let Some(name) = PREFERRED
            .iter()
            .find(|name| self.fonts.contains_key(**name))
        {
            return name.to_string();
        }
        if let Some(first) = self.fonts.keys().min() {
            return first.clone();
        }
        "Helvetica".to_string()
    }

    /// Sorted list of successfully loaded font families.
    pub fn available_families(&self) -> Vec<String> {
        let mut names: Vec<String> = self.fonts.keys().cloned().collect();
        names.sort();
        names
    }

    /// Dynamically discover installed system fonts by scanning font directories.
    pub fn discover_system_fonts() -> Vec<(String, String)> {
        discover_system_fonts()
    }

    /// Try to load every plausible system font family (best effort).
    pub fn load_all_system_fonts(&mut self) {
        let discovered = Self::discover_system_fonts();
        for (name, path) in discovered {
            if let Ok(data) = std::fs::read(&path) {
                if parse_font(&data).is_some() {
                    self.fonts.insert(name, data);
                }
            }
        }

        const CANDIDATES: &[&str] = &[
            "Helvetica",
            "Helvetica Neue",
            "Arial",
            "Inter",
            "Roboto",
            "DejaVu Sans",
            "Liberation Sans",
            "Times New Roman",
            "Georgia",
            "Courier New",
            "Menlo",
            "Monaco",
            "Verdana",
            "Tahoma",
            "Trebuchet MS",
            "Futura",
            "Gill Sans",
            "Avenir",
            "Optima",
            "Hiragino Sans",
            "Yu Gothic",
            "Noto Sans CJK JP",
            "Osaka",
        ];
        for name in CANDIDATES {
            if !self.fonts.contains_key(*name) {
                self.load_system_font(name);
            }
        }
    }

    /// Get the default font (first available system font, or embed a fallback).
    pub fn get_default_font_data(&self) -> &[u8] {
        for name in &[
            "Helvetica",
            "Arial",
            "DejaVu Sans",
            "Liberation Sans",
            "sans-serif",
        ] {
            if let Some(data) = self.fonts.get(*name) {
                return data;
            }
        }
        &[]
    }

    /// Rasterize a single glyph at the given font size.
    pub fn rasterize_glyph(
        &self,
        family_name: &str,
        ch: char,
        font_size: f32,
    ) -> Option<RasterizedGlyph> {
        if !font_size.is_finite() || !(0.1..=8192.0).contains(&font_size) {
            return None;
        }
        let key = (family_name.to_string(), ch, font_size.to_bits());
        if let Some(hit) = self.glyph_cache.borrow().entries.get(&key) {
            return Some(hit.clone());
        }
        let font_data = self.fonts.get(family_name)?;
        let glyph = rasterize_glyph_uncached(font_data, ch, font_size)?;
        let bytes = glyph.pixels.len();
        if bytes <= GLYPH_CACHE_BUDGET_BYTES {
            let mut cache = self.glyph_cache.borrow_mut();
            if cache.bytes.saturating_add(bytes) > GLYPH_CACHE_BUDGET_BYTES {
                cache.entries.clear();
                cache.bytes = 0;
            }
            cache.bytes = cache.bytes.saturating_add(bytes);
            cache.entries.insert(key, glyph.clone());
        }
        Some(glyph)
    }

    /// Rasterize a full text string into a single RGBA buffer.
    pub fn rasterize_text(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        if !font_size.is_finite()
            || !(0.1..=8192.0).contains(&font_size)
            || !tracking.is_finite()
            || !color.iter().all(|value| value.is_finite())
        {
            return None;
        }
        self.rasterize_text_formatted(
            family_name,
            text,
            font_size,
            color,
            tracking,
            1.2,
            0.0,
            crate::core::text_layout::TextAlign::Left,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_formatted_styled(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        style: FauxTextStyle,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let styled_text = styled_text_input(text, style);
        let rasterized = self.rasterize_text_formatted(
            family_name,
            &styled_text,
            font_size,
            color,
            tracking,
            leading,
            box_width,
            alignment,
        )?;
        Some(apply_faux_bitmap_style(rasterized, style))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_formatted(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
    ) -> Option<(u32, u32, Vec<u8>)> {
        if !font_size.is_finite()
            || !(0.1..=8192.0).contains(&font_size)
            || !tracking.is_finite()
            || !leading.is_finite()
            || leading <= 0.0
            || leading > 100.0
            || !box_width.is_finite()
            || box_width < 0.0
            || !color.iter().all(|value| value.is_finite())
        {
            return None;
        }
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scale = PxScale::from(font_size);
        let scaled_font = font.as_scaled(scale);

        let layout = crate::core::text_layout::layout_text(
            text, font_size, tracking, leading, box_width, alignment,
        );
        if layout.lines.is_empty() {
            return None;
        }

        let actual_width: f32 = layout
            .lines
            .iter()
            .map(|line| {
                let mut w = 0.0f32;
                for ch in line.text.chars() {
                    let glyph_id = font.glyph_id(ch);
                    w += scaled_font.h_advance(glyph_id);
                    w += tracking;
                }
                if line.text.ends_with(' ') {
                    w -= tracking;
                }
                w.max(line.width)
            })
            .fold(0.0f32, f32::max);

        let buf_w = (actual_width.ceil() as u32)
            .max(layout.total_width.ceil() as u32)
            .max(1);
        let line_height = font_size * leading;
        let buf_h = (layout.total_height.ceil() as u32)
            .max(line_height.ceil() as u32)
            .max(1);

        let mut pixels = vec![0u8; (buf_w * buf_h * 4) as usize];
        let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
        let max_top = font_size * 0.8;

        for line in &layout.lines {
            let align_x =
                crate::core::text_layout::get_alignment_offset(line.width, box_width, alignment);
            let mut cursor_x = align_x;
            let cursor_y = line.y_offset;

            for ch in line.text.chars() {
                let glyph_id = font.glyph_id(ch);
                let h_advance = scaled_font.h_advance(glyph_id);

                if ch != ' ' {
                    if let Some(rg) = self.rasterize_glyph(family_name, ch, font_size) {
                        for gy in 0..rg.height {
                            for gx in 0..rg.width {
                                let dest_x = cursor_x as i32 + rg.left + gx as i32;
                                let dest_y = cursor_y as i32 + max_top as i32 + rg.top + gy as i32;
                                if dest_x >= 0
                                    && dest_y >= 0
                                    && (dest_x as u32) < buf_w
                                    && (dest_y as u32) < buf_h
                                {
                                    let sidx = ((gy * rg.width + gx) * 4) as usize;
                                    let didx =
                                        ((dest_y as u32 * buf_w + dest_x as u32) * 4) as usize;
                                    if sidx + 3 < rg.pixels.len() && didx + 3 < pixels.len() {
                                        pixels[didx] = r;
                                        pixels[didx + 1] = g;
                                        pixels[didx + 2] = b;
                                        pixels[didx + 3] = rg.pixels[sidx + 3];
                                    }
                                }
                            }
                        }
                    }
                }

                cursor_x += h_advance + tracking;
            }
        }

        Some((buf_w, buf_h, pixels))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_animated(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        animator: &crate::core::text_animator::TextAnimatorSettings,
        time: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let font_data = self.fonts.get(family_name)?;
        rasterize_text_animated(
            font_data, text, font_size, color, tracking, leading, box_width, alignment, animator,
            time,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_animated_styled(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        animator: &crate::core::text_animator::TextAnimatorSettings,
        time: f32,
        style: FauxTextStyle,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let styled_text = styled_text_input(text, style);
        let rasterized = self.rasterize_text_animated(
            family_name,
            &styled_text,
            font_size,
            color,
            tracking,
            leading,
            box_width,
            alignment,
            animator,
            time,
        )?;
        Some(apply_faux_bitmap_style(rasterized, style))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_animated_stack(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        stack: &crate::core::text_animator_advanced::AnimatorStack,
        time: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let font_data = self.fonts.get(family_name)?;
        rasterize_text_animated_stack(
            font_data, text, font_size, color, tracking, leading, box_width, alignment, stack, time,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_animated_stack_styled(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        stack: &crate::core::text_animator_advanced::AnimatorStack,
        time: f32,
        style: FauxTextStyle,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let styled_text = styled_text_input(text, style);
        let rasterized = self.rasterize_text_animated_stack(
            family_name,
            &styled_text,
            font_size,
            color,
            tracking,
            leading,
            box_width,
            alignment,
            stack,
            time,
        )?;
        Some(apply_faux_bitmap_style(rasterized, style))
    }
}
