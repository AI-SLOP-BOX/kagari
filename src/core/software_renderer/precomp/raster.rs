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
    use crate::core::image_cache::with_image_cache;

    let Some((img, next_img)) = with_image_cache(|cache| {
        let img = cache.load_image(path).cloned()?;
        let next_img = next_path
            .filter(|candidate| *candidate != path && blend_t > 0.0)
            .and_then(|candidate| cache.load_image(candidate).cloned())
            .filter(|next| next.width == img.width && next.height == img.height);
        Some((img, next_img))
    }) else {
        return;
    };
    let img_w = img.width as f32;
    let img_h = img.height as f32;

    for py in region.min_y..region.max_y {
        for px in region.min_x..region.max_x {
            let dx = px as f32 - region.center_x;
            let dy = py as f32 - region.center_y;
            let lx = dx * region.cos_rotation + dy * region.sin_rotation;
            let ly = -dx * region.sin_rotation + dy * region.cos_rotation;
            let u = (lx / region.bounds_x + 1.0) * 0.5;
            let v = (ly / region.bounds_y + 1.0) * 0.5;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }

            let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img.width - 1);
            let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img.height - 1);
            let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
            if tidx + 3 >= img.pixels.len() {
                continue;
            }
            let lidx =
                (((py - region.min_y) * region.buffer_width + (px - region.min_x)) * 4) as usize;
            if lidx + 3 >= layer_buf.len() {
                continue;
            }

            let sample = |channel: usize| -> u8 {
                let a = img.pixels[tidx + channel] as f32;
                let b = next_img
                    .as_ref()
                    .and_then(|next| {
                        let next_idx = ((tex_y * next.width + tex_x) * 4) as usize;
                        next.pixels.get(next_idx + channel).copied()
                    })
                    .map(f32::from)
                    .unwrap_or(a);
                (a + (b - a) * blend_t).round().clamp(0.0, 255.0) as u8
            };
            layer_buf[lidx] = sample(0);
            layer_buf[lidx + 1] = sample(1);
            layer_buf[lidx + 2] = sample(2);
            layer_buf[lidx + 3] = sample(3);
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

    let source_w = source_width as f32;
    let source_h = source_height as f32;
    for py in region.min_y..region.max_y {
        for px in region.min_x..region.max_x {
            let dx = px as f32 - region.center_x;
            let dy = py as f32 - region.center_y;
            let lx = dx * region.cos_rotation + dy * region.sin_rotation;
            let ly = -dx * region.sin_rotation + dy * region.cos_rotation;
            let u = (lx / region.bounds_x + 1.0) * 0.5;
            let v = (ly / region.bounds_y + 1.0) * 0.5;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }

            let sx = ((u * (source_w - 1.0)).round() as u32).min(source_width - 1);
            let sy = ((v * (source_h - 1.0)).round() as u32).min(source_height - 1);
            let source_idx = ((sy * source_width + sx) * 4) as usize;
            let layer_idx =
                (((py - region.min_y) * region.buffer_width + (px - region.min_x)) * 4) as usize;
            if source_idx + 3 < pixels.len() && layer_idx + 3 < layer_buf.len() {
                layer_buf[layer_idx..layer_idx + 4]
                    .copy_from_slice(&pixels[source_idx..source_idx + 4]);
            }
        }
    }
}
