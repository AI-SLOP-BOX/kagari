/// Sample top-left-origin RGBA8 pixels with the linear filtering used by the
/// WGPU layer sampler. Coordinates are normalized texture coordinates.
pub(crate) fn sample_rgba8_linear(
    pixels: &[u8],
    width: u32,
    height: u32,
    u: f32,
    v: f32,
) -> Option<[f32; 4]> {
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || pixels.len() < expected_len || !u.is_finite() || !v.is_finite()
    {
        return None;
    }

    // WGPU maps normalized coordinates to texel centers at (n + 0.5) / size.
    // Clamp the taps independently to match ClampToEdge at image boundaries.
    let x = u.clamp(0.0, 1.0) * width as f32 - 0.5;
    let y = v.clamp(0.0, 1.0) * height as f32 - 0.5;
    let x_floor = x.floor();
    let y_floor = y.floor();
    let tx = x - x_floor;
    let ty = y - y_floor;
    let x0 = (x_floor as i64).clamp(0, width as i64 - 1) as u32;
    let y0 = (y_floor as i64).clamp(0, height as i64 - 1) as u32;
    let x1 = (x_floor as i64 + 1).clamp(0, width as i64 - 1) as u32;
    let y1 = (y_floor as i64 + 1).clamp(0, height as i64 - 1) as u32;
    let index = |px: u32, py: u32| ((py * width + px) * 4) as usize;
    let top_left = index(x0, y0);
    let top_right = index(x1, y0);
    let bottom_left = index(x0, y1);
    let bottom_right = index(x1, y1);

    let interpolate = |sample: [usize; 4]| {
        let top = f32::from(pixels[sample[0]]) * (1.0 - tx) + f32::from(pixels[sample[1]]) * tx;
        let bottom = f32::from(pixels[sample[2]]) * (1.0 - tx) + f32::from(pixels[sample[3]]) * tx;
        top * (1.0 - ty) + bottom * ty
    };
    let alpha = interpolate([
        top_left + 3,
        top_right + 3,
        bottom_left + 3,
        bottom_right + 3,
    ]);
    if alpha <= f32::EPSILON {
        return Some([0.0; 4]);
    }
    let unpremultiplied = |channel: usize| {
        let premultiplied = [top_left, top_right, bottom_left, bottom_right]
            .map(|index| f32::from(pixels[index + channel]) * f32::from(pixels[index + 3]) / 255.0);
        let top = premultiplied[0] * (1.0 - tx) + premultiplied[1] * tx;
        let bottom = premultiplied[2] * (1.0 - tx) + premultiplied[3] * tx;
        (top * (1.0 - ty) + bottom * ty) * 255.0 / alpha
    };
    Some([
        unpremultiplied(0),
        unpremultiplied(1),
        unpremultiplied(2),
        alpha,
    ])
}
