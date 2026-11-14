use super::*;

/// Precomputed 0..=255 values as little-endian f16 bytes.
static F16_LUT: std::sync::OnceLock<[[u8; 2]; 256]> = std::sync::OnceLock::new();
static PREMULTIPLIED_F16_LUT: std::sync::OnceLock<[[[u8; 2]; 256]; 256]> =
    std::sync::OnceLock::new();

fn get_f16_lut() -> &'static [[u8; 2]; 256] {
    F16_LUT.get_or_init(|| {
        let mut lut = [[0u8; 2]; 256];
        for (i, entry) in lut.iter_mut().enumerate() {
            let f = f16::from_f32(i as f32 / 255.0);
            *entry = f.to_le_bytes();
        }
        lut
    })
}

/// Convert straight-alpha RGBA8 pixels to premultiplied RGBA16Float bytes.
/// Each pixel: 4 channels × 2 bytes (f16) = 8 bytes.
pub(super) fn rgba8_to_rgba16f_bytes(pixels: &[u8]) -> Option<Vec<u8>> {
    let alpha_lut = get_f16_lut();
    let premultiplied_lut = PREMULTIPLIED_F16_LUT.get_or_init(|| {
        let mut lut = [[[0u8; 2]; 256]; 256];
        for (alpha, row) in lut.iter_mut().enumerate() {
            let alpha = alpha as f32 / 255.0;
            for (channel, bytes) in row.iter_mut().enumerate() {
                *bytes = f16::from_f32(channel as f32 / 255.0 * alpha).to_le_bytes();
            }
        }
        lut
    });
    let num_pixels = pixels.len().checked_div(4)?;
    if !pixels.len().is_multiple_of(4) {
        return None;
    }
    let output_len = num_pixels.checked_mul(8)?;
    let mut out = Vec::new();
    out.try_reserve_exact(output_len).ok()?;
    for pixel in pixels.as_chunks::<4>().0 {
        for channel in 0..3 {
            out.extend_from_slice(&premultiplied_lut[pixel[3] as usize][pixel[channel] as usize]);
        }
        out.extend_from_slice(&alpha_lut[pixel[3] as usize]);
    }
    Some(out)
}

/// Convert a single RGBA8 color to RGBA16Float bytes.
/// Returns 8 bytes (4 × f16).
pub(super) fn rgba8_color_to_rgba16f_bytes(r: u8, g: u8, b: u8, a: u8) -> [u8; 8] {
    let mut out = [0u8; 8];
    let rf = f16::from_f32(r as f32 / 255.0);
    let gf = f16::from_f32(g as f32 / 255.0);
    let bf = f16::from_f32(b as f32 / 255.0);
    let af = f16::from_f32(a as f32 / 255.0);
    out[0..2].copy_from_slice(&rf.to_le_bytes());
    out[2..4].copy_from_slice(&gf.to_le_bytes());
    out[4..6].copy_from_slice(&bf.to_le_bytes());
    out[6..8].copy_from_slice(&af.to_le_bytes());
    out
}

// Helper matrix functions
#[allow(dead_code)]
pub(super) fn mat4_identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
pub(super) fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            out[column][row] = a[0][row] * b[column][0]
                + a[1][row] * b[column][1]
                + a[2][row] * b[column][2]
                + a[3][row] * b[column][3];
        }
    }
    out
}

pub(super) fn effective_render_frame(layer: &Layer, frame: u32, fps: u32) -> u32 {
    layer.effective_render_frame(frame, fps)
}

pub(super) fn gpu_layer_draw_order(
    comp: &Composition,
    layers: &[&Layer],
    frame: u32,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..layers.len()).collect();
    if !comp.layers.iter().any(|layer| layer.is_3d) {
        return order;
    }
    order.sort_by(|&a, &b| {
        let depth = |index: usize| {
            let layer = layers[index];
            if layer.is_3d {
                layer.transform_3d.position.evaluate(frame)[2]
            } else {
                0.0
            }
        };
        depth(a)
            .partial_cmp(&depth(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(&b))
    });
    order
}

pub(super) fn layer_position_at_render_frame(
    comp: &Composition,
    layer: &Layer,
    frame: u32,
) -> [f32; 2] {
    let has_exprs = layer.transform.position_expression.is_some()
        || layer.transform.rotation_expression.is_some()
        || layer.transform.scale_expression.is_some()
        || layer.transform.opacity_expression.is_some();
    if layer.parent_id.is_some() || has_exprs {
        comp.resolve_world_transform(layer, frame).0
    } else {
        layer.transform.position.evaluate(frame)
    }
}

pub(super) fn layer_motion_velocity(comp: &Composition, layer: &Layer, frame: u32) -> [f32; 2] {
    let previous = layer_position_at_render_frame(comp, layer, frame.saturating_sub(1));
    let next = layer_position_at_render_frame(comp, layer, frame.saturating_add(1));
    [
        ((next[0] - previous[0]) * 0.5).clamp(-100_000.0, 100_000.0),
        ((next[1] - previous[1]) * 0.5).clamp(-100_000.0, 100_000.0),
    ]
    .map(|value| if value.is_finite() { value } else { 0.0 })
}
