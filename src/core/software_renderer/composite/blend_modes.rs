use crate::core::sdf::{hsb_to_rgb, rgb_to_hsb};
use crate::core::timeline::BlendMode;

#[inline(always)]
pub fn overlay(source: f32, destination: f32) -> f32 {
    if destination < 0.5 {
        2.0 * source * destination
    } else {
        1.0 - 2.0 * (1.0 - source) * (1.0 - destination)
    }
}

#[inline(always)]
pub fn soft_light(source: f32, destination: f32) -> f32 {
    if source <= 0.5 {
        destination - (1.0 - 2.0 * source) * destination * (1.0 - destination)
    } else {
        let curve = if destination <= 0.25 {
            ((16.0 * destination - 12.0) * destination + 4.0) * destination
        } else {
            destination.sqrt()
        };
        destination + (2.0 * source - 1.0) * (curve - destination)
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_blend_mode(
    blend_mode: BlendMode,
    src_r: f32,
    src_g: f32,
    src_b: f32,
    src_a: f32,
    dst_r: f32,
    dst_g: f32,
    dst_b: f32,
    dst_a: f32,
) -> (f32, f32, f32) {
    match blend_mode {
        BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
        BlendMode::Screen => (
            1.0 - (1.0 - src_r) * (1.0 - dst_r),
            1.0 - (1.0 - src_g) * (1.0 - dst_g),
            1.0 - (1.0 - src_b) * (1.0 - dst_b),
        ),
        BlendMode::Overlay => (
            overlay(src_r, dst_r),
            overlay(src_g, dst_g),
            overlay(src_b, dst_b),
        ),
        BlendMode::Add => (
            (src_r + dst_r).min(1.0),
            (src_g + dst_g).min(1.0),
            (src_b + dst_b).min(1.0),
        ),
        BlendMode::Darken => (src_r.min(dst_r), src_g.min(dst_g), src_b.min(dst_b)),
        BlendMode::Lighten => (src_r.max(dst_r), src_g.max(dst_g), src_b.max(dst_b)),
        BlendMode::SoftLight => (
            soft_light(src_r, dst_r),
            soft_light(src_g, dst_g),
            soft_light(src_b, dst_b),
        ),
        BlendMode::HardLight => {
            let f = |s: f32, d: f32| {
                if s <= 0.5 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                }
            };
            (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
        }
        BlendMode::Difference => (
            (dst_r - src_r).abs(),
            (dst_g - src_g).abs(),
            (dst_b - src_b).abs(),
        ),
        BlendMode::Exclusion => (
            src_r + dst_r - 2.0 * src_r * dst_r,
            src_g + dst_g - 2.0 * src_g * dst_g,
            src_b + dst_b - 2.0 * src_b * dst_b,
        ),
        BlendMode::Divide => (
            (src_r / dst_r.max(1e-6)).clamp(0.0, 1.0),
            (src_g / dst_g.max(1e-6)).clamp(0.0, 1.0),
            (src_b / dst_b.max(1e-6)).clamp(0.0, 1.0),
        ),
        BlendMode::Subtract => (
            (src_r - dst_r).max(0.0),
            (src_g - dst_g).max(0.0),
            (src_b - dst_b).max(0.0),
        ),
        BlendMode::ColorBurn => {
            let f = |s: f32, d: f32| {
                if s <= 0.0 {
                    0.0
                } else {
                    (1.0 - ((1.0 - d) / s)).clamp(0.0, 1.0)
                }
            };
            (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
        }
        BlendMode::LinearBurn => (
            (src_r + dst_r - 1.0).clamp(0.0, 1.0),
            (src_g + dst_g - 1.0).clamp(0.0, 1.0),
            (src_b + dst_b - 1.0).clamp(0.0, 1.0),
        ),
        BlendMode::VividLight => {
            let f = |s: f32, d: f32| {
                if s <= 0.5 {
                    if s <= 0.0 {
                        0.0
                    } else {
                        (1.0 - ((1.0 - d) / (2.0 * s))).clamp(0.0, 1.0)
                    }
                } else if s >= 1.0 {
                    1.0
                } else {
                    (d / (2.0 * (1.0 - s))).clamp(0.0, 1.0)
                }
            };
            (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
        }
        BlendMode::ColorDodge => {
            let f = |s: f32, d: f32| {
                if s >= 1.0 {
                    1.0
                } else {
                    (d / (1.0 - s)).clamp(0.0, 1.0)
                }
            };
            (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
        }
        BlendMode::LinearDodge => (
            (src_r + dst_r).min(1.0),
            (src_g + dst_g).min(1.0),
            (src_b + dst_b).min(1.0),
        ),
        BlendMode::Color => {
            let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, s, _) = rgb_to_hsb(sh, ss, sb);
                let (_, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).0
            };
            let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, s, _) = rgb_to_hsb(sh, ss, sb);
                let (_, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).1
            };
            let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, s, _) = rgb_to_hsb(sh, ss, sb);
                let (_, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).2
            };
            (
                f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fb(src_r, dst_r, src_g, dst_g, src_b, dst_b),
            )
        }
        BlendMode::Hue => {
            let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                let (_, s, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).0
            };
            let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                let (_, s, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).1
            };
            let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                let (_, s, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).2
            };
            (
                f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fb(src_r, dst_r, src_g, dst_g, src_b, dst_b),
            )
        }
        BlendMode::Saturation => {
            let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                let (h, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).0
            };
            let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                let (h, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).1
            };
            let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                let (h, _, b) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).2
            };
            (
                f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fb(src_r, dst_r, src_g, dst_g, src_b, dst_b),
            )
        }
        BlendMode::Luminosity => {
            let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                let (h, s, _) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).0
            };
            let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                let (h, s, _) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).1
            };
            let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                let (h, s, _) = rgb_to_hsb(dh, ds, db);
                hsb_to_rgb(h, s, b).2
            };
            (
                f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                fb(src_r, dst_r, src_g, dst_g, src_b, dst_b),
            )
        }
        BlendMode::StencilAlpha => {
            let mask = src_a;
            (dst_r * mask, dst_g * mask, dst_b * mask)
        }
        BlendMode::StencilLuma => {
            let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
            (dst_r * luma, dst_g * luma, dst_b * luma)
        }
        BlendMode::SilhouetteAlpha => {
            let mask = 1.0 - src_a;
            (dst_r * mask, dst_g * mask, dst_b * mask)
        }
        BlendMode::SilhouetteLuma => {
            let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
            let mask = 1.0 - luma;
            (dst_r * mask, dst_g * mask, dst_b * mask)
        }
        BlendMode::Behind => {
            let mask = 1.0 - dst_a;
            (
                src_r * mask + dst_r * (1.0 - mask),
                src_g * mask + dst_g * (1.0 - mask),
                src_b * mask + dst_b * (1.0 - mask),
            )
        }
        BlendMode::AlphaAdd => (
            src_r * src_a + dst_r * dst_a,
            src_g * src_a + dst_g * dst_a,
            src_b * src_a + dst_b * dst_a,
        ),
        BlendMode::LinearLight => {
            let f = |s: f32, d: f32| -> f32 { (s + d - 0.5).clamp(0.0, 1.0) };
            (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
        }
        BlendMode::Normal => (src_r, src_g, src_b),
    }
}
