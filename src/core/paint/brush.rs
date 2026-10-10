/// Draw one stroke into `buf` (w×h RGBA8). Points are in buffer pixel space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushStyle {
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub flow: f32,
}

pub fn draw_stroke(
    buf: &mut [u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    color: [f32; 4],
    size: f32,
) {
    draw_stroke_with_style(
        buf,
        w,
        h,
        points,
        color,
        BrushStyle {
            size,
            hardness: 1.0,
            opacity: 1.0,
            flow: 1.0,
        },
    );
}

/// Draw a stroke with the brush controls exposed by the paint UI.
#[allow(clippy::too_many_arguments)]
pub fn draw_stroke_with_brush(
    buf: &mut [u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    color: [f32; 4],
    size: f32,
    hardness: f32,
    opacity: f32,
    flow: f32,
) {
    draw_stroke_with_style(
        buf,
        w,
        h,
        points,
        color,
        BrushStyle {
            size,
            hardness,
            opacity,
            flow,
        },
    );
}

pub fn draw_stroke_with_style(
    buf: &mut [u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    color: [f32; 4],
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
    if points.is_empty()
        || !size.is_finite()
        || size <= 0.0
        || w == 0
        || h == 0
        || buf.len() < expected_len
        || !color.iter().all(|value| value.is_finite())
    {
        return;
    }
    let radius = (size.clamp(0.0, 8192.0) * 0.5).max(0.5);
    let r2 = radius * radius;
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
    let src_a = color[3].clamp(0.0, 1.0) * opacity * flow;

    let mut stamp = |cx: f32, cy: f32| {
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
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let dist = d2.sqrt();
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
                let idx = ((py * w + px) * 4) as usize;
                if idx + 3 >= buf.len() {
                    continue;
                }
                let dst = [
                    buf[idx] as f32 / 255.0,
                    buf[idx + 1] as f32 / 255.0,
                    buf[idx + 2] as f32 / 255.0,
                    buf[idx + 3] as f32 / 255.0,
                ];
                let sa = src_a * cov;
                let out_a = sa + dst[3] * (1.0 - sa);
                if out_a <= 0.0001 {
                    continue;
                }
                for ch in 0..3 {
                    let sc = color[ch];
                    let v = (sc * sa + dst[ch] * dst[3] * (1.0 - sa)) / out_a;
                    buf[idx + ch] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                buf[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
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
