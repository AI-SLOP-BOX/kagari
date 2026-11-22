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

            // Solve the inverse warp at the source coordinate. Sampling flow
            // only at the output coordinate is inaccurate when motion varies
            // spatially, as it does under scaling and deformation.
            let (src_a_x, src_a_y) = inverse_warp_position(flow_a_to_b, px, py, t, width, height);
            // `flow_b_to_a` maps B source positions back to A, so the same
            // inverse equation applies with the remaining time fraction.
            let (src_b_x, src_b_y) =
                inverse_warp_position(flow_b_to_a, px, py, 1.0 - t, width, height);

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

fn inverse_warp_position(
    flow: &DenseFlowField,
    output_x: f32,
    output_y: f32,
    time_weight: f32,
    frame_width: u32,
    frame_height: u32,
) -> (f32, f32) {
    let mut source_x = output_x;
    let mut source_y = output_y;
    for _ in 0..4 {
        let vector = sample_flow_bilinear(flow, source_x, source_y, frame_width, frame_height);
        let next_x = output_x - vector[0] * time_weight;
        let next_y = output_y - vector[1] * time_weight;
        if (next_x - source_x).abs().max((next_y - source_y).abs()) < 0.01 {
            source_x = next_x;
            source_y = next_y;
            break;
        }
        source_x = next_x;
        source_y = next_y;
    }
    (source_x, source_y)
}

fn sample_flow_bilinear(
    flow: &DenseFlowField,
    x: f32,
    y: f32,
    frame_width: u32,
    frame_height: u32,
) -> [f32; 2] {
    if flow.width == 0 || flow.height == 0 || frame_width == 0 || frame_height == 0 {
        return [0.0; 2];
    }
    let fx = (x * flow.width as f32 / frame_width as f32)
        .clamp(0.0, flow.width.saturating_sub(1) as f32);
    let fy = (y * flow.height as f32 / frame_height as f32)
        .clamp(0.0, flow.height.saturating_sub(1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(flow.width - 1);
    let y1 = (y0 + 1).min(flow.height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let a = flow.get(x0, y0);
    let b = flow.get(x1, y0);
    let c = flow.get(x0, y1);
    let d = flow.get(x1, y1);
    [
        (a[0] * (1.0 - tx) + b[0] * tx) * (1.0 - ty) + (c[0] * (1.0 - tx) + d[0] * tx) * ty,
        (a[1] * (1.0 - tx) + b[1] * tx) * (1.0 - ty) + (c[1] * (1.0 - tx) + d[1] * tx) * ty,
    ]
}

#[cfg(test)]
mod tests {
    use super::inverse_warp_position;
    use crate::core::optical_flow_timewarp::DenseFlowField;

    #[test]
    fn inverse_warp_solves_spatially_varying_forward_flow() {
        let mut flow = DenseFlowField::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                flow.vectors[(y * 64 + x) as usize] = [x as f32 * 0.1, 0.0];
            }
        }

        let (source_x, source_y) = inverse_warp_position(&flow, 15.0, 20.0, 0.5, 64, 64);

        assert!((source_x - 15.0 / 1.05).abs() < 0.01);
        assert_eq!(source_y, 20.0);
    }
}
