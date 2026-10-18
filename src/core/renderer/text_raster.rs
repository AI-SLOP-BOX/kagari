/// Bakes a text stroke into a rasterized text bitmap: dilates the fill alpha by the
/// stroke radius, colors it with stroke_color, and composites it behind the fill.
/// Returns padded (width, height, pixels) so the stroke is not clipped.
pub(crate) fn bake_text_stroke(
    pixels: &[u8],
    width: u32,
    height: u32,
    stroke_color: [f32; 4],
    stroke_width: f32,
) -> (u32, u32, Vec<u8>) {
    let radius = (stroke_width * 0.5).ceil().max(1.0) as i32;
    let pad = radius + 1;
    let (nw, nh) = (width + (pad * 2) as u32, height + (pad * 2) as u32);
    let mut out = vec![0u8; (nw * nh * 4) as usize];

    let sr = (stroke_color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let sg = (stroke_color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let sb = (stroke_color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let stroke_a = stroke_color[3];

    let w = width as i32;
    let h = height as i32;
    for py in 0..nh as i32 {
        for px in 0..nw as i32 {
            let tx = px - pad;
            let ty = py - pad;
            let oidx = ((py as u32 * nw + px as u32) * 4) as usize;

            // Fill sample (offset back by pad)
            let fill_alpha = if tx >= 0 && ty >= 0 && tx < w && ty < h {
                let idx = ((ty * w + tx) * 4) as usize;
                pixels[idx + 3] as f32 / 255.0
            } else {
                0.0
            };

            // Stroke: max over neighbors within radius of fill alpha, feathered by distance
            let mut stroke_alpha = 0.0f32;
            if fill_alpha < 0.999 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = tx + dx;
                        let ny = ty + dy;
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                            if dist <= stroke_width * 0.5 {
                                let nidx = ((ny * w + nx) * 4) as usize;
                                let n_alpha = pixels[nidx + 3] as f32 / 255.0;
                                if n_alpha > 0.001 {
                                    let edge = (stroke_width * 0.5 - dist)
                                        / (stroke_width * 0.25).max(0.5);
                                    stroke_alpha = stroke_alpha.max(edge.clamp(0.0, 1.0));
                                }
                            }
                        }
                    }
                }
            }

            // Composite stroke behind fill (premultiplied-style over)
            let stroke_a_px = stroke_alpha * stroke_a;
            let out_a = fill_alpha + stroke_a_px * (1.0 - fill_alpha);
            if out_a > 0.001 {
                let fr = pixels
                    .get(((ty.max(0) * w + tx.max(0)) * 4) as usize)
                    .copied()
                    .unwrap_or(0);
                let fg = pixels
                    .get(((ty.max(0) * w + tx.max(0)) * 4 + 1) as usize)
                    .copied()
                    .unwrap_or(0);
                let fb = pixels
                    .get(((ty.max(0) * w + tx.max(0)) * 4 + 2) as usize)
                    .copied()
                    .unwrap_or(0);
                // Stroke color behind fill color
                let mix_r = (sr as f32 * (1.0 - fill_alpha) + fr as f32 * fill_alpha) as u8;
                let mix_g = (sg as f32 * (1.0 - fill_alpha) + fg as f32 * fill_alpha) as u8;
                let mix_b = (sb as f32 * (1.0 - fill_alpha) + fb as f32 * fill_alpha) as u8;
                out[oidx] = mix_r;
                out[oidx + 1] = mix_g;
                out[oidx + 2] = mix_b;
                out[oidx + 3] = (out_a * 255.0) as u8;
            }
        }
    }
    (nw, nh, out)
}

pub(crate) struct TextRasterParams {
    pub(crate) text: String,
    pub(crate) font_size: u32,
    pub(crate) color: [f32; 4],
    pub(crate) font_family: String,
    pub(crate) tracking: f32,
    pub(crate) leading: f32,
    pub(crate) align: usize,
    pub(crate) box_width: f32,
    pub(crate) stroke_color: [f32; 4],
    pub(crate) stroke_width: f32,
    pub(crate) faux_style: crate::core::font_rasterizer::FauxTextStyle,
}
