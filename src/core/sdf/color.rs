use crate::core::timeline::ShapeFillType;

/// Convert RGB (each 0..1) to HSB (H: 0..360, S: 0..1, B: 0..1).
pub fn rgb_to_hsb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta < 0.001 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max < 0.001 { 0.0 } else { delta / max };
    (h, s, max)
}

/// Convert HSB (H: 0..360, S: 0..1, B: 0..1) to RGB (each 0..1).
pub fn hsb_to_rgb(h: f32, s: f32, b: f32) -> (f32, f32, f32) {
    let c = b * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = b - c;
    let (r, g, bl) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r + m, g + m, bl + m)
}

pub fn sample_gradient(colors: &[[f32; 4]], stops: &[f32], t: f32) -> [f32; 4] {
    if colors.is_empty() {
        return [0.5, 0.5, 0.5, 1.0];
    }
    if colors.len() == 1 || stops.len() <= 1 {
        return colors[0];
    }
    let t = t.clamp(0.0, 1.0);
    for i in 0..stops.len().saturating_sub(1) {
        if t >= stops[i] && t <= stops[i + 1] {
            let range = stops[i + 1] - stops[i];
            let local_t = if range.abs() < 0.001 {
                0.0
            } else {
                (t - stops[i]) / range
            };
            let c0 = colors[i.min(colors.len() - 1)];
            let c1 = colors[(i + 1).min(colors.len() - 1)];
            return [
                c0[0] + (c1[0] - c0[0]) * local_t,
                c0[1] + (c1[1] - c0[1]) * local_t,
                c0[2] + (c1[2] - c0[2]) * local_t,
                c0[3] + (c1[3] - c0[3]) * local_t,
            ];
        }
    }
    colors[colors.len() - 1]
}

pub fn resolve_fill_color(
    fill: &ShapeFillType,
    fallback: [f32; 4],
    px: f32,
    py: f32,
    cx: f32,
    cy: f32,
) -> [f32; 4] {
    match fill {
        ShapeFillType::Solid => fallback,
        ShapeFillType::LinearGradient {
            start,
            end,
            colors,
            stops,
        } => {
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len_sq = dx * dx + dy * dy;
            if len_sq > 0.001 {
                let t = ((px - cx - start[0]) * dx + (py - cy - start[1]) * dy) / len_sq;
                sample_gradient(colors, stops, t)
            } else {
                fallback
            }
        }
        ShapeFillType::RadialGradient {
            center,
            radius,
            colors,
            stops,
        } => {
            let dx = px - cx - center[0];
            let dy = py - cy - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (dist / radius.max(0.001)).clamp(0.0, 1.0);
            sample_gradient(colors, stops, t)
        }
    }
}
