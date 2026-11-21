use super::brush::BrushStyle;

/// Draw a clone-stamp stroke by sampling `source` at the corresponding
/// offset position for every brush stamp.
#[allow(clippy::too_many_arguments)]
pub fn draw_clone_stroke_with_brush(
    buf: &mut [u8],
    source: &[u8],
    w: u32,
    h: u32,
    target_points: &[[f32; 2]],
    source_points: &[[f32; 2]],
    size: f32,
    hardness: f32,
    opacity: f32,
    flow: f32,
) {
    draw_clone_stroke_with_style(
        buf,
        source,
        w,
        h,
        target_points,
        source_points,
        BrushStyle {
            size,
            hardness,
            opacity,
            flow,
        },
    );
}

pub fn draw_clone_stroke_with_style(
    buf: &mut [u8],
    source: &[u8],
    w: u32,
    h: u32,
    target_points: &[[f32; 2]],
    source_points: &[[f32; 2]],
    style: BrushStyle,
) {
    let BrushStyle {
        size,
        hardness,
        opacity,
        flow,
    } = style;
    let Some(expected_len) = (w as usize)
        .checked_mul(h as usize)
        .and_then(|count| count.checked_mul(4))
    else {
        return;
    };
    if target_points.is_empty()
        || target_points.len() != source_points.len()
        || !size.is_finite()
        || size <= 0.0
        || w == 0
        || h == 0
        || buf.len() < expected_len
        || source.len() < expected_len
    {
        return;
    }
    let radius = (size.clamp(0.0, 8192.0) * 0.5).max(0.5);
    let hardness = if hardness.is_finite() {
        hardness.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let flow = if flow.is_finite() {
        flow.clamp(0.0, 1.0)
    } else {
        1.0
    };

    let mut stamp = |target: [f32; 2], sample: [f32; 2]| {
        let (cx, cy) = (target[0], target[1]);
        if !cx.is_finite() || !cy.is_finite() {
            return;
        }
        let lo_x = ((cx - radius).floor().max(0.0)) as u32;
        let hi_x = ((cx + radius).ceil().min(w as f32 - 1.0)) as u32;
        let lo_y = ((cy - radius).floor().max(0.0)) as u32;
        let hi_y = ((cy + radius).ceil().min(h as f32 - 1.0)) as u32;
        for py in lo_y..=hi_y {
            for px in lo_x..=hi_x {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > radius {
                    continue;
                }
                let cov = if hardness >= 1.0 {
                    (radius - dist).clamp(0.0, 1.0)
                } else if dist <= radius * hardness {
                    1.0
                } else {
                    let inner_radius = radius * hardness;
                    ((radius - dist) / (radius - inner_radius).max(1.0)).clamp(0.0, 1.0)
                };
                let sx = (sample[0] + dx).round().clamp(0.0, w as f32 - 1.0) as u32;
                let sy = (sample[1] + dy).round().clamp(0.0, h as f32 - 1.0) as u32;
                let src_idx = ((sy * w + sx) * 4) as usize;
                let dst_idx = ((py * w + px) * 4) as usize;
                let sa = source[src_idx + 3] as f32 / 255.0 * opacity * flow * cov;
                if sa <= 0.0001 {
                    continue;
                }
                let dst = [
                    buf[dst_idx] as f32 / 255.0,
                    buf[dst_idx + 1] as f32 / 255.0,
                    buf[dst_idx + 2] as f32 / 255.0,
                    buf[dst_idx + 3] as f32 / 255.0,
                ];
                let out_a = sa + dst[3] * (1.0 - sa);
                for ch in 0..3 {
                    let sc = source[src_idx + ch] as f32 / 255.0;
                    let value = (sc * sa + dst[ch] * dst[3] * (1.0 - sa)) / out_a;
                    buf[dst_idx + ch] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                buf[dst_idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    };

    if target_points.len() == 1 {
        stamp(target_points[0], source_points[0]);
        return;
    }
    for (target_seg, source_seg) in target_points.windows(2).zip(source_points.windows(2)) {
        let target_a = target_seg[0];
        let target_b = target_seg[1];
        let source_a = source_seg[0];
        let source_b = source_seg[1];
        let dx = target_b[0] - target_a[0];
        let dy = target_b[1] - target_a[1];
        let len = (dx * dx + dy * dy).sqrt();
        let steps = ((len / (radius * 0.5)).ceil() as usize).clamp(1, 4096);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            stamp(
                [
                    target_a[0] + (target_b[0] - target_a[0]) * t,
                    target_a[1] + (target_b[1] - target_a[1]) * t,
                ],
                [
                    source_a[0] + (source_b[0] - source_a[0]) * t,
                    source_a[1] + (source_b[1] - source_a[1]) * t,
                ],
            );
        }
    }
}

/// Configuration for the clone stamp tool.
#[derive(Debug, Clone)]
pub struct CloneStampConfig {
    pub src_offset: [f32; 2],
    pub size: f32,
    pub opacity: f32,
    pub hardness: f32,
    pub rotation_deg: f32,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl Default for CloneStampConfig {
    fn default() -> Self {
        Self {
            src_offset: [0.0, 0.0],
            size: 10.0,
            opacity: 1.0,
            hardness: 0.5,
            rotation_deg: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }
}

/// Transform source coordinates with rotation, flip, and offset.
#[inline]
fn clone_transform(
    dx: f32,
    dy: f32,
    offset: [f32; 2],
    rot_rad: f32,
    flip_h: bool,
    flip_v: bool,
) -> (f32, f32) {
    let mut x = dx;
    let mut y = dy;
    if flip_h {
        x = -x;
    }
    if flip_v {
        y = -y;
    }
    let cos = rot_rad.cos();
    let sin = rot_rad.sin();
    let rx = x * cos - y * sin;
    let ry = x * sin + y * cos;
    (rx + offset[0], ry + offset[1])
}

/// Clone stamp: copies pixels from a source buffer at an offset into `dst`.
pub fn draw_clone_stamp(
    dst: &mut [u8],
    src: &[u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    config: &CloneStampConfig,
) {
    if points.is_empty() || config.size <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let radius = (config.size * 0.5).max(0.5);
    let r2 = radius * radius;
    let soft = (1.0 - config.hardness).max(0.0);

    let rot_rad = config.rotation_deg.to_radians();
    let mut stamp = |cx: f32, cy: f32| {
        let lo_x = ((cx - radius).floor().max(0.0)) as u32;
        let hi_x = ((cx + radius).ceil().min(w as f32 - 1.0)) as u32;
        let lo_y = ((cy - radius).floor().max(0.0)) as u32;
        let hi_y = ((cy + radius).ceil().min(h as f32 - 1.0)) as u32;
        for py in lo_y..=hi_y {
            for px in lo_x..=hi_x {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let dist = d2.sqrt();
                let alpha = if soft <= 0.001 {
                    if dist <= radius {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    let t = dist / radius;
                    if t < 1.0 - soft {
                        1.0
                    } else if t < 1.0 {
                        1.0 - (t - (1.0 - soft)) / soft
                    } else {
                        0.0
                    }
                };
                let a = alpha * config.opacity;
                if a <= 0.001 {
                    continue;
                }
                let (sx_f, sy_f) = clone_transform(
                    dx,
                    dy,
                    config.src_offset,
                    rot_rad,
                    config.flip_h,
                    config.flip_v,
                );
                let sx = sx_f as i32;
                let sy = sy_f as i32;
                if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
                    continue;
                }
                let s_idx = ((sy as u32 * w + sx as u32) * 4) as usize;
                let d_idx = ((py * w + px) * 4) as usize;
                if s_idx + 3 >= src.len() || d_idx + 3 >= dst.len() {
                    continue;
                }
                let out_a = a + dst[d_idx + 3] as f32 / 255.0 * (1.0 - a);
                if out_a > 0.0001 {
                    let inv_a = 1.0 / out_a;
                    for ch in 0..3 {
                        let s_val = src[s_idx + ch] as f32 / 255.0;
                        let d_val = dst[d_idx + ch] as f32 / 255.0;
                        dst[d_idx + ch] = ((s_val * a
                            + d_val * dst[d_idx + 3] as f32 / 255.0 * (1.0 - a))
                            * inv_a
                            * 255.0)
                            .round()
                            .min(255.0) as u8;
                    }
                    dst[d_idx + 3] = (out_a * 255.0).round().min(255.0) as u8;
                }
            }
        }
    };

    if points.len() == 1 {
        stamp(points[0][0], points[0][1]);
        return;
    }
    for seg in points.windows(2) {
        let (a, b) = (&seg[0], &seg[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        let steps = ((len / (radius * 0.5)).ceil() as usize).clamp(1, 4096);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            stamp(a[0] + dx * t, a[1] + dy * t);
        }
    }
}
