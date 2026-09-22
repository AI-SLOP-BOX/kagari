//! Phase-3 compositing for the software renderer.
//!
//! Blends one effect-processed layer buffer over the frame with full
//! blend-mode support, track mattes, composite-time opacity, and
//! optional linear-light blending.

use crate::core::sdf::{hsb_to_rgb, rgb_to_hsb};
use crate::core::timeline::{BlendMode, Layer, TrackMatteMode};

/// Everything [`composite_layer_buffer`] needs that is not the two
/// pixel buffers.
pub(crate) struct CompositeCtx<'a> {
    pub layer: &'a Layer,
    pub matte: Option<&'a [u8]>,
    pub min_x: u32,
    pub min_y: u32,
    pub bw: u32,
    pub bh: u32,
    pub width: u32,
    pub height: u32,
    pub blend_linear: bool,
    pub l_opacity: f32,
}

/// Composite `layer_buf` over `buffer` (both RGBA8) with the layer's
/// blend mode, track matte, and composite-time opacity.
pub(crate) fn composite_layer_buffer(buffer: &mut [u8], layer_buf: &[u8], ctx: &CompositeCtx<'_>) {
    let CompositeCtx {
        layer,
        matte,
        min_x,
        min_y,
        bw,
        bh,
        width,
        height,
        blend_linear,
        l_opacity,
    } = ctx;
    let (min_x, min_y, bw, bh, width, height) = (*min_x, *min_y, *bw, *bh, *width, *height);
    let (blend_linear, l_opacity) = (*blend_linear, *l_opacity);
    for ly in 0..bh {
        for lx in 0..bw {
            let lidx = ((ly * bw + lx) * 4) as usize;
            // Composite-time opacity (see precomp path above): applied
            // once here so effect stacks cannot destroy it, uniformly
            // across all blend modes via the src_a lerp below.
            let mut src_a = layer_buf[lidx + 3] as f32 / 255.0 * l_opacity;
            if src_a <= 0.001 {
                continue;
            }
            let px = min_x + lx;
            let py = min_y + ly;
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 >= buffer.len() {
                continue;
            }

            // Apply track matte masking
            if let Some(matte_buf) = matte {
                let mx = px.min(width - 1);
                let my = py.min(height - 1);
                let midx = ((my * width + mx) * 4) as usize;
                if midx + 3 < matte_buf.len() {
                    let matte_a = matte_buf[midx + 3] as f32 / 255.0;
                    let matte_luma = (matte_buf[midx] as f32 * 0.299
                        + matte_buf[midx + 1] as f32 * 0.587
                        + matte_buf[midx + 2] as f32 * 0.114)
                        / 255.0;
                    src_a = match layer.track_matte {
                        TrackMatteMode::AlphaMatte => src_a * matte_a,
                        TrackMatteMode::AlphaMatteInverted => src_a * (1.0 - matte_a),
                        TrackMatteMode::LumaMatte => src_a * matte_luma,
                        TrackMatteMode::LumaMatteInverted => src_a * (1.0 - matte_luma),
                        _ => src_a,
                    };
                    if src_a <= 0.001 {
                        continue;
                    }
                }
            }

            let mut src_r = layer_buf[lidx] as f32 / 255.0;
            let mut src_g = layer_buf[lidx + 1] as f32 / 255.0;
            let mut src_b = layer_buf[lidx + 2] as f32 / 255.0;

            let mut dst_r = buffer[idx] as f32 / 255.0;
            let mut dst_g = buffer[idx + 1] as f32 / 255.0;
            let mut dst_b = buffer[idx + 2] as f32 / 255.0;
            let dst_a = buffer[idx + 3] as f32 / 255.0;

            // Linear-light mode: decode both sides with the exact IEC
            // sRGB piecewise EOTF so Add/Screen/Glow blends behave
            // physically and match GPU sRGB hardware encoders.
            if blend_linear {
                use crate::core::color::srgb_to_linear_piecewise as to_lin;
                src_r = to_lin(src_r);
                src_g = to_lin(src_g);
                src_b = to_lin(src_b);
                dst_r = to_lin(dst_r);
                dst_g = to_lin(dst_g);
                dst_b = to_lin(dst_b);
            }

            // Compute BlendMode calculations
            let (blended_r, blended_g, blended_b) = match layer.blend_mode {
                BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                BlendMode::Screen => (
                    1.0 - (1.0 - src_r) * (1.0 - dst_r),
                    1.0 - (1.0 - src_g) * (1.0 - dst_g),
                    1.0 - (1.0 - src_b) * (1.0 - dst_b),
                ),
                BlendMode::Overlay => (
                    if dst_r < 0.5 {
                        2.0 * src_r * dst_r
                    } else {
                        1.0 - 2.0 * (1.0 - src_r) * (1.0 - dst_r)
                    },
                    if dst_g < 0.5 {
                        2.0 * src_g * dst_g
                    } else {
                        1.0 - 2.0 * (1.0 - src_g) * (1.0 - dst_g)
                    },
                    if dst_b < 0.5 {
                        2.0 * src_b * dst_b
                    } else {
                        1.0 - 2.0 * (1.0 - src_b) * (1.0 - dst_b)
                    },
                ),
                BlendMode::Add => (
                    (src_r + dst_r).min(1.0),
                    (src_g + dst_g).min(1.0),
                    (src_b + dst_b).min(1.0),
                ),
                BlendMode::Darken => (src_r.min(dst_r), src_g.min(dst_g), src_b.min(dst_b)),
                BlendMode::Lighten => (src_r.max(dst_r), src_g.max(dst_g), src_b.max(dst_b)),
                BlendMode::SoftLight => {
                    let f = |s: f32, d: f32| {
                        if s <= 0.5 {
                            d - (1.0 - 2.0 * s) * d * (1.0 - d)
                        } else {
                            let a = if d <= 0.25 {
                                ((16.0 * d - 12.0) * d + 4.0) * d
                            } else {
                                d.sqrt()
                            };
                            d + (2.0 * s - 1.0) * (a - d)
                        }
                    };
                    (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                }
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
                            if s == 0.0 {
                                0.0
                            } else {
                                (1.0 - (1.0 - d) / (2.0 * s)).clamp(0.0, 1.0)
                            }
                        } else if s == 1.0 {
                            1.0
                        } else {
                            (d / (2.0 * (1.0 - s))).clamp(0.0, 1.0)
                        }
                    };
                    (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                }
                BlendMode::ColorDodge => {
                    let f = |s: f32, d: f32| {
                        if d == 0.0 {
                            0.0
                        } else if s >= 1.0 {
                            1.0
                        } else {
                            (d / (1.0 - s)).clamp(0.0, 1.0)
                        }
                    };
                    (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                }
                BlendMode::LinearDodge => {
                    let f = |s: f32, d: f32| (s + d).clamp(0.0, 1.0);
                    (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                }
                BlendMode::Color => {
                    let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                        let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                        let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                        hsb_to_rgb(h, s, db2).0
                    };
                    let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                        let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                        let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                        hsb_to_rgb(h, s, db2).1
                    };
                    let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                        let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                        let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                        hsb_to_rgb(h, s, db2).2
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
                // Stencil: source alpha as mask for destination
                BlendMode::StencilAlpha => {
                    let mask = src_a;
                    (dst_r * mask, dst_g * mask, dst_b * mask)
                }
                BlendMode::StencilLuma => {
                    let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
                    (dst_r * luma, dst_g * luma, dst_b * luma)
                }
                // Silhouette: inverse stencil (punch hole)
                BlendMode::SilhouetteAlpha => {
                    let mask = 1.0 - src_a;
                    (dst_r * mask, dst_g * mask, dst_b * mask)
                }
                BlendMode::SilhouetteLuma => {
                    let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
                    let mask = 1.0 - luma;
                    (dst_r * mask, dst_g * mask, dst_b * mask)
                }
                // Behind: paint behind (source only shows where destination is transparent)
                BlendMode::Behind => {
                    let mask = 1.0 - dst_a;
                    (
                        src_r * mask + dst_r * (1.0 - mask),
                        src_g * mask + dst_g * (1.0 - mask),
                        src_b * mask + dst_b * (1.0 - mask),
                    )
                }
                // Alpha Add: additive blend using alpha
                BlendMode::AlphaAdd => (
                    src_r * src_a + dst_r * dst_a,
                    src_g * src_a + dst_g * dst_a,
                    src_b * src_a + dst_b * dst_a,
                ),
                // Linear Light: combination of Linear Burn and Linear Dodge
                BlendMode::LinearLight => {
                    let f = |s: f32, d: f32| -> f32 { (s + d - 0.5).clamp(0.0, 1.0) };
                    (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                }
                BlendMode::Normal => (src_r, src_g, src_b),
            };

            // Preserve Underlying Transparency (AE 'T' switch)
            if layer.preserve_transparency {
                src_a *= dst_a;
                if src_a <= 0.001 {
                    continue;
                }
            }

            // Alpha blending formula: Standard Source-Over or Transparency Preservation
            let out_a = if layer.preserve_transparency {
                dst_a
            } else {
                src_a + dst_a * (1.0 - src_a)
            };
            let out_r = if out_a > 0.0 {
                (blended_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a
            } else {
                0.0
            };
            let out_g = if out_a > 0.0 {
                (blended_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a
            } else {
                0.0
            };
            let out_b = if out_a > 0.0 {
                (blended_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a
            } else {
                0.0
            };

            // Encode back to display space when in linear-light mode.
            let (or_, og_, ob_) = if blend_linear {
                use crate::core::color::linear_to_srgb_piecewise as to_srgb;
                (
                    to_srgb(out_r.max(0.0)),
                    to_srgb(out_g.max(0.0)),
                    to_srgb(out_b.max(0.0)),
                )
            } else {
                (out_r, out_g, out_b)
            };
            buffer[idx] = (or_ * 255.0) as u8;
            buffer[idx + 1] = (og_ * 255.0) as u8;
            buffer[idx + 2] = (ob_ * 255.0) as u8;
            buffer[idx + 3] = (out_a * 255.0) as u8;
        }
    }
}
