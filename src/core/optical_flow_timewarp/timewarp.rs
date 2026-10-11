use super::*;

/// Interpolates an intermediate frame at fractional position `t` (0.0 .. 1.0)
/// using bidirectional forward and backward flow fields.
#[allow(clippy::too_many_arguments)]
pub fn interpolate_timewarp_frame(
    frame_a: &[u8],
    frame_b: &[u8],
    width: u32,
    height: u32,
    flow_a_to_b: &DenseFlowField,
    flow_b_to_a: &DenseFlowField,
    t: f32,
    out_rgba: &mut [u8],
) {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return;
    };
    if width == 0
        || height == 0
        || frame_a.len() != size
        || frame_b.len() != size
        || out_rgba.len() != size
    {
        return;
    }

    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let w = width as f32;
    let h = height as f32;

    let sample_bilinear = |buf: &[u8], sx: f32, sy: f32| -> [f32; 4] {
        let x = sx.clamp(0.0, (w - 1.0).max(0.0));
        let y = sy.clamp(0.0, (h - 1.0).max(0.0));

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(width as usize - 1);
        let y1 = (y0 + 1).min(height as usize - 1);

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let idx00 = (y0 * width as usize + x0) * 4;
        let idx10 = (y0 * width as usize + x1) * 4;
        let idx01 = (y1 * width as usize + x0) * 4;
        let idx11 = (y1 * width as usize + x1) * 4;

        let mut res = [0.0f32; 4];
        for c in 0..4 {
            let v00 = buf[idx00 + c] as f32;
            let v10 = buf[idx10 + c] as f32;
            let v01 = buf[idx01 + c] as f32;
            let v11 = buf[idx11 + c] as f32;
            res[c] = (1.0 - fx) * (1.0 - fy) * v00
                + fx * (1.0 - fy) * v10
                + (1.0 - fx) * fy * v01
                + fx * fy * v11;
        }
        res
    };

    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;

            // Flow vectors
            let v_ab = flow_a_to_b.get(x, y);
            let v_ba = flow_b_to_a.get(x, y);

            // Backward-warp positions
            let src_a_x = px - v_ab[0] * t;
            let src_a_y = py - v_ab[1] * t;

            let src_b_x = px + v_ba[0] * (1.0 - t);
            let src_b_y = py + v_ba[1] * (1.0 - t);

            let col_a = sample_bilinear(frame_a, src_a_x, src_a_y);
            let col_b = sample_bilinear(frame_b, src_b_x, src_b_y);

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            for c in 0..4 {
                let blended = col_a[c] * (1.0 - t) + col_b[c] * t;
                out_rgba[dst_idx + c] = blended.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}
