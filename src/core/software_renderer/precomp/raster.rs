#[derive(Clone, Copy)]
pub(crate) struct RasterizationRegion {
    pub(crate) min_x: u32,
    pub(crate) min_y: u32,
    pub(crate) max_x: u32,
    pub(crate) max_y: u32,
    pub(crate) buffer_width: u32,
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) cos_rotation: f32,
    pub(crate) sin_rotation: f32,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(crate) bounds_x: f32,
    pub(crate) bounds_y: f32,
}

pub(crate) fn rasterize_texture_layer(
    layer_buf: &mut [u8],
    path: &str,
    next_path: Option<&str>,
    blend_t: f32,
    region: RasterizationRegion,
) {
    let Some(img) = crate::core::image_cache::load_image_shared(path) else {
        return;
    };
    let next_img = next_path
        .filter(|candidate| *candidate != path && blend_t > 0.0)
        .and_then(crate::core::image_cache::load_image_shared)
        .filter(|next| next.width == img.width && next.height == img.height);
    for py in region.min_y..region.max_y {
        if super::super::render_cancelled() {
            return;
        }
        for px in region.min_x..region.max_x {
            let dx = px as f32 + 0.5 - region.center_x;
            let dy = py as f32 + 0.5 - region.center_y;
            let lx = dx * region.cos_rotation + dy * region.sin_rotation;
            let ly = -dx * region.sin_rotation + dy * region.cos_rotation;
            let u = (lx / region.bounds_x + 1.0) * 0.5;
            let v = (ly / region.bounds_y + 1.0) * 0.5;
            let u = if region.flip_x { 1.0 - u } else { u };
            let v = if region.flip_y { 1.0 - v } else { v };
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }

            let Some(sample_a) = super::super::sampling::sample_rgba8_linear(
                &img.pixels,
                img.width,
                img.height,
                u,
                v,
            ) else {
                continue;
            };
            let sample_b = next_img
                .as_ref()
                .and_then(|next| {
                    super::super::sampling::sample_rgba8_linear(
                        &next.pixels,
                        next.width,
                        next.height,
                        u,
                        v,
                    )
                })
                .unwrap_or(sample_a);
            let lidx =
                (((py - region.min_y) * region.buffer_width + (px - region.min_x)) * 4) as usize;
            if lidx + 3 >= layer_buf.len() {
                continue;
            }

            let blend = blend_t.clamp(0.0, 1.0);
            for channel in 0..4 {
                let value = sample_a[channel] + (sample_b[channel] - sample_a[channel]) * blend;
                layer_buf[lidx + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

pub(crate) fn rasterize_pixel_layer(
    layer_buf: &mut [u8],
    pixels: &[u8],
    source_width: u32,
    source_height: u32,
    region: RasterizationRegion,
) {
    if source_width == 0 || source_height == 0 {
        return;
    }
    let source_size = (source_width as usize)
        .checked_mul(source_height as usize)
        .and_then(|size| size.checked_mul(4));
    if source_size != Some(pixels.len()) {
        return;
    }

    for py in region.min_y..region.max_y {
        if super::super::render_cancelled() {
            return;
        }
        for px in region.min_x..region.max_x {
            let dx = px as f32 + 0.5 - region.center_x;
            let dy = py as f32 + 0.5 - region.center_y;
            let lx = dx * region.cos_rotation + dy * region.sin_rotation;
            let ly = -dx * region.sin_rotation + dy * region.cos_rotation;
            let u = (lx / region.bounds_x + 1.0) * 0.5;
            let v = (ly / region.bounds_y + 1.0) * 0.5;
            let u = if region.flip_x { 1.0 - u } else { u };
            let v = if region.flip_y { 1.0 - v } else { v };
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }

            let layer_idx =
                (((py - region.min_y) * region.buffer_width + (px - region.min_x)) * 4) as usize;
            if let Some(sample) = super::super::sampling::sample_rgba8_linear(
                pixels,
                source_width,
                source_height,
                u,
                v,
            ) {
                if layer_idx + 3 < layer_buf.len() {
                    for channel in 0..4 {
                        layer_buf[layer_idx + channel] =
                            sample[channel].round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}
