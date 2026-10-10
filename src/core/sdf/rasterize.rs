use crate::core::timeline::{ShapeFillType, ShapeType, TrimPaths};
use super::color::resolve_fill_color;
use super::primitives::*;

/// Rasterize a shape layer into the layer buffer using SDF.
/// Returns true if any pixels were written.
#[allow(clippy::too_many_arguments)]
pub fn rasterize_shape_sdf(
    layer_buf: &mut [u8],
    bw: u32,
    bh: u32,
    min_x: u32,
    min_y: u32,
    cx: f32,
    cy: f32,
    bounds_x: f32,
    bounds_y: f32,
    base_color: [f32; 4],
    fill_type: &ShapeFillType,
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
) -> bool {
    rasterize_shape_sdf_with_rotation(
        layer_buf,
        bw,
        bh,
        min_x,
        min_y,
        cx,
        cy,
        bounds_x,
        bounds_y,
        base_color,
        fill_type,
        stroke_color,
        stroke_width,
        l_opacity,
        shape_type,
        frame,
        trim_paths,
        0.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_shape_sdf_with_rotation(
    layer_buf: &mut [u8],
    bw: u32,
    bh: u32,
    min_x: u32,
    min_y: u32,
    cx: f32,
    cy: f32,
    bounds_x: f32,
    bounds_y: f32,
    base_color: [f32; 4],
    fill_type: &ShapeFillType,
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
    rotation_deg: f32,
) -> bool {
    let rotation = (-rotation_deg).to_radians();
    let (sin_rotation, cos_rotation) = rotation.sin_cos();
    let mut any_written = false;
    // Hoist all frame-invariant work out of the pixel loop: scalar property
    // evaluation, bezier tessellation, and trim-path evaluation used to run
    // per pixel (tessellation per pixel cost seconds per layer at 1080p).
    enum SdfPrep {
        Rect { hx: f32, hy: f32, cr: f32 },
        Ellipse { hx: f32, hy: f32 },
        Star { pts: u32, or: f32, ir: f32 },
        Polygon { s: u32, r: f32 },
        Free { pts: Vec<(f32, f32)> },
        Empty,
    }
    let prep = match shape_type {
        ShapeType::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            let w = width.evaluate(frame) / 100.0;
            let h = height.evaluate(frame) / 100.0;
            let cr = corner_radius.evaluate(frame) / 100.0;
            SdfPrep::Rect {
                hx: w * 0.5,
                hy: h * 0.5,
                cr,
            }
        }
        ShapeType::Ellipse { width, height } => {
            let w = width.evaluate(frame) / 100.0;
            let h = height.evaluate(frame) / 100.0;
            SdfPrep::Ellipse {
                hx: w * 0.5,
                hy: h * 0.5,
            }
        }
        ShapeType::Star {
            points,
            inner_radius,
            outer_radius,
        } => {
            let pts = (points.evaluate(frame) as u32).max(3);
            let ir = inner_radius.evaluate(frame) / 100.0;
            let or = outer_radius.evaluate(frame) / 100.0;
            SdfPrep::Star { pts, or, ir }
        }
        ShapeType::Polygon { sides, radius } => {
            let s = (sides.evaluate(frame) as u32).max(3);
            let r = radius.evaluate(frame) / 100.0;
            SdfPrep::Polygon { s, r }
        }
        ShapeType::FreeformBezier {
            points,
            tangents,
            closed,
        } => {
            if points.len() < 3 {
                SdfPrep::Empty
            } else {
                let tessellated = tessellate_bezier_path(points, tangents, *closed, 8);
                let scale = 100.0;
                let pts: Vec<(f32, f32)> = tessellated
                    .iter()
                    .map(|p| (p[0] / scale, p[1] / scale))
                    .collect();
                SdfPrep::Free { pts }
            }
        }
    };
    let (trim_s, trim_e, trim_on) = match trim_paths {
        None => (0.0, 0.0, false),
        Some(tp) => {
            let start_pct = tp.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
            let end_pct = tp.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
            let offset_pct = (tp.offset.evaluate(frame) / 360.0).fract();
            let s = (start_pct + offset_pct).fract();
            let e = (end_pct + offset_pct).fract();
            (s, e, true)
        }
    };
    // Bounding-box early-out reach (stroke halo + AA fringe, normalized
    // units; degenerate bounds yield +inf, i.e. never skip).
    let reach_x = (stroke_width.max(0.0) + 4.0) / bounds_x;
    let reach_y = (stroke_width.max(0.0) + 4.0) / bounds_y;
    for py in 0..bh {
        for px in 0..bw {
            let world_x = min_x + px;
            let world_y = min_y + py;
            let raw_x = world_x as f32 - cx;
            let raw_y = world_y as f32 - cy;
            let local_x = raw_x * cos_rotation - raw_y * sin_rotation;
            let local_y = raw_x * sin_rotation + raw_y * cos_rotation;
            let nx = local_x / bounds_x;
            let ny = local_y / bounds_y;

            // The shape cannot cover pixels outside its normalized bounds.
            if nx.abs() > 1.0 + reach_x || ny.abs() > 1.0 + reach_y {
                continue;
            }

            let dist = match &prep {
                SdfPrep::Rect { hx, hy, cr } => {
                    if *cr > 0.01 {
                        let dx = nx.abs() - hx + cr;
                        let dy = ny.abs() - hy + cr;
                        let outside = (dx.max(0.0), dy.max(0.0));
                        let inside = dx.min(0.0).max(dy.min(0.0));
                        (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside - cr
                    } else {
                        sdf_rectangle(nx, ny, *hx, *hy)
                    }
                }
                SdfPrep::Ellipse { hx, hy } => sdf_ellipse(nx, ny, *hx, *hy),
                SdfPrep::Star { pts, or, ir } => sdf_star(nx, ny, *pts, *or, *ir),
                SdfPrep::Polygon { s, r } => sdf_polygon(nx, ny, *s, *r),
                SdfPrep::Free { pts } => sdf_polygon_points(nx, ny, pts),
                SdfPrep::Empty => 1.0,
            };

            let pixel_width = 4.0 / bounds_x;
            let mut alpha = (1.0 - (dist / pixel_width).clamp(0.0, 1.0)) * l_opacity;

            if alpha > 0.001 && trim_on {
                let angle = ny.atan2(nx);
                let angle_norm = (angle / (2.0 * std::f32::consts::PI) + 1.0).fract();
                let s = trim_s;
                let e = trim_e;
                let in_trim = if (s - e).abs() < f32::EPSILON {
                    false
                } else if s < e {
                    angle_norm >= s && angle_norm <= e
                } else {
                    angle_norm >= s || angle_norm <= e
                };
                if !in_trim {
                    alpha = 0.0;
                }
            }

            if alpha > 0.001 {
                let lidx = ((py * bw + px) * 4) as usize;
                if lidx + 3 < layer_buf.len() {
                    let (r, g, b, a) = if stroke_width > 0.5 && dist.abs() < stroke_width / bounds_x
                    {
                        (
                            stroke_color[0],
                            stroke_color[1],
                            stroke_color[2],
                            stroke_color[3] * alpha,
                        )
                    } else {
                        let fc = resolve_fill_color(
                            fill_type,
                            base_color,
                            world_x as f32,
                            world_y as f32,
                            cx,
                            cy,
                        );
                        (fc[0], fc[1], fc[2], fc[3] * alpha)
                    };
                    layer_buf[lidx] = (r * 255.0) as u8;
                    layer_buf[lidx + 1] = (g * 255.0) as u8;
                    layer_buf[lidx + 2] = (b * 255.0) as u8;
                    layer_buf[lidx + 3] = (a * 255.0) as u8;
                    any_written = true;
                }
            }
        }
    }
    any_written
}
