//! Shadow mapping and 3D light helpers for the software renderer.
//!
//! Covers orthographic shadow-map accumulation, light attenuation
//! models, and spot cone factors.

use crate::core::mask::point_in_polygon;
use crate::core::timeline::{
    Composition, Layer, LayerType, Light3D, LightType, ShapeType,
};

/// Project a world point through a light onto the z=0 receiver plane.
/// Returns None when the ray is parallel to the plane or points away.
fn project_point_to_plane_z0(light: [f32; 3], point: [f32; 3]) -> Option<[f32; 2]> {
    let dz = point[2] - light[2];
    if dz.abs() < 0.001 {
        return None;
    }
    let t = -light[2] / dz;
    if t <= 0.0 {
        return None;
    }
    Some([
        light[0] + t * (point[0] - light[0]),
        light[1] + t * (point[1] - light[1]),
    ])
}

/// Fill a convex polygon into the density buffer (scanline-free point test
/// over the bbox — quads are tiny relative to frame, this is plenty fast).
fn accumulate_polygon_density(
    density: &mut [f32],
    width: u32,
    height: u32,
    pts: &[[f32; 2]],
    amount: f32,
) {
    if pts.len() < 3 || amount <= 0.0 {
        return;
    }
    let min_x = pts
        .iter()
        .map(|p| p[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_x = pts
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(width as f32) as u32;
    let min_y = pts
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_y = pts
        .iter()
        .map(|p| p[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(height as f32) as u32;
    for py in min_y..max_y.min(height) {
        for px in min_x..max_x.min(width) {
            if point_in_polygon(px as f32 + 0.5, py as f32 + 0.5, pts) {
                let idx = (py * width + px) as usize;
                if idx < density.len() {
                    density[idx] = (density[idx] + amount).min(1.0);
                }
            }
        }
    }
}

fn box_blur_f32(buf: &mut [f32], width: u32, height: u32, radius: u32) {
    if radius == 0 || buf.is_empty() || width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    let r = radius as usize;
    let mut tmp = vec![0.0f32; buf.len()];
    // Horizontal
    for y in 0..h {
        let row = &buf[y * w..(y + 1) * w];
        let out = &mut tmp[y * w..(y + 1) * w];
        for (x, slot) in out.iter_mut().enumerate() {
            let lo = x.saturating_sub(r);
            let hi = (x + r + 1).min(w);
            let s: f32 = row[lo..hi].iter().sum();
            *slot = s / (hi - lo) as f32;
        }
    }
    // Vertical
    for x in 0..w {
        for y in 0..h {
            let lo = y.saturating_sub(r);
            let hi = (y + r + 1).min(h);
            let mut s = 0.0;
            for yy in lo..hi {
                s += tmp[yy * w + x];
            }
            buf[y * w + x] = s / (hi - lo) as f32;
        }
    }
}

/// Build the caster's outline points in LOCAL layer space (centered origin),
/// honoring the actual shape instead of its bounding quad. Falls back to the
/// full-size rect for raster/content types.
fn caster_outline_points(layer: &Layer, base_w: f32, base_h: f32, frame: u32) -> Vec<[f32; 2]> {
    let shape_pts: Option<Vec<[f32; 2]>> = match &layer.layer_type {
        LayerType::Shape { shape_type, .. } => match shape_type {
            ShapeType::Ellipse { width, height } => {
                let w = width.evaluate(frame).max(1.0) / 2.0;
                let h = height.evaluate(frame).max(1.0) / 2.0;
                Some(
                    (0..24)
                        .map(|i| {
                            let a = i as f32 / 24.0 * std::f32::consts::TAU;
                            [w * a.cos(), h * a.sin()]
                        })
                        .collect(),
                )
            }
            ShapeType::Polygon { sides, radius } => {
                let n = sides.evaluate(frame).round().max(3.0) as usize;
                let r = radius.evaluate(frame).max(1.0);
                Some(
                    (0..n)
                        .map(|i| {
                            let a = i as f32 / n as f32 * std::f32::consts::TAU
                                - std::f32::consts::FRAC_PI_2;
                            [r * a.cos(), r * a.sin()]
                        })
                        .collect(),
                )
            }
            ShapeType::Star {
                points,
                inner_radius,
                outer_radius,
            } => {
                let n = points.evaluate(frame).round().max(3.0) as usize;
                let ri = inner_radius.evaluate(frame).max(1.0);
                let ro = outer_radius.evaluate(frame).max(ri);
                Some(
                    (0..n * 2)
                        .map(|i| {
                            let a = i as f32 / (n * 2) as f32 * std::f32::consts::TAU
                                - std::f32::consts::FRAC_PI_2;
                            let r = if i % 2 == 0 { ro } else { ri };
                            [r * a.cos(), r * a.sin()]
                        })
                        .collect(),
                )
            }
            ShapeType::Rectangle { width, height, .. } => {
                let hw = width.evaluate(frame).max(1.0) / 2.0;
                let hh = height.evaluate(frame).max(1.0) / 2.0;
                Some(vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]])
            }
            _ => None,
        },
        _ => None,
    };
    // Shape-local coordinates are already in comp pixels; layer scale is
    // applied by the caller via resolve transform values.
    let _ = (base_w, base_h);
    shape_pts.unwrap_or_default()
}

/// Build the shadow density map (0=lit, 1=fully shadowed) at comp resolution:
/// every shadow-casting light projects every casting layer's world quad onto
/// the z=0 plane; densities accumulate and are softened by a small blur.
pub fn build_shadow_map(comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<f32> {
    let n = (width.max(1) as usize) * (height.max(1) as usize);
    let mut density = vec![0.0f32; n];

    for light in &comp.lights {
        if !light.casts_shadows || light.intensity <= 0.0 {
            continue;
        }
        let lpos = light.position.evaluate(frame);
        let strength =
            ((light.shadow_darkness / 100.0) * (light.intensity / 100.0)).clamp(0.0, 1.0);
        if strength <= 0.003 {
            continue;
        }
        for layer in &comp.layers {
            if !layer.is_active(frame) || !layer.visible || !layer.material.cast_shadows {
                continue;
            }
            // Both the light and the caster geometry are evaluated at
            // composition time. Time remapping changes source pixels, not
            // the layer transform that casts the shadow.
            let (pos, scale, rot, _op) = comp.resolve_world_transform(layer, frame);
            // Outline points: real shape geometry when available (ellipse,
            // polygon, star, rect), else the full-size rect quad.
            let (base_w, base_h) = match &layer.layer_type {
                LayerType::Solid { .. }
                | LayerType::Shape { .. }
                | LayerType::Image { .. }
                | LayerType::Video { .. }
                | LayerType::Text { .. }
                | LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
                _ => continue,
            };
            let mut local_pts = caster_outline_points(layer, base_w, base_h, frame);
            if local_pts.is_empty() {
                let hw = base_w / 2.0;
                let hh = base_h / 2.0;
                local_pts = vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];
            }
            if local_pts.len() < 3 {
                continue;
            }
            let lz = if layer.is_3d {
                layer.transform_3d.position.evaluate(frame)[2]
            } else {
                0.0
            };
            // Coplanar with the receiver plane: its "shadow" lands exactly on
            // itself and would blanket the frame — skip.
            if lz.abs() < 1.0 {
                continue;
            }
            let rad = rot.to_radians();
            let (c, s) = (rad.cos(), rad.sin());
            // Layer scale applies to shape-local coordinates too
            let sx_mul = scale[0].abs() / 100.0;
            let sy_mul = scale[1].abs() / 100.0;
            let mut projected: Vec<[f32; 2]> = Vec::with_capacity(local_pts.len());
            for cl in &local_pts {
                let lx2 = cl[0] * sx_mul;
                let ly2 = cl[1] * sy_mul;
                let wx = pos[0] + lx2 * c - ly2 * s;
                let wy = pos[1] + lx2 * s + ly2 * c;
                match project_point_to_plane_z0(lpos, [wx, wy, lz]) {
                    Some(p) => projected.push(p),
                    None => {
                        projected.clear();
                        break;
                    }
                }
            }
            if projected.len() == local_pts.len() {
                // Apply distance attenuation per shadow-casting pixel region

                let atten = light_attenuation(
                    light,
                    lpos,
                    [lpos[0], lpos[1], 0.0],
                    frame,
                );
                accumulate_polygon_density(
                    &mut density,
                    width,
                    height,
                    &projected,
                    strength * atten,
                );
            }
        }
    }

    // Soft penumbra
    box_blur_f32(
        &mut density,
        width,
        height,
        ((width.max(height)) / 64).clamp(1, 8),
    );
    density
}

/// Compute light attenuation factor based on distance from light to a point.
/// `falloff`: 0 = no falloff, 1 = linear, 2 = inverse-square (realistic).
/// `max_radius`: 0 = unlimited, otherwise hard cutoff.
pub fn light_attenuation(
    light: &Light3D,
    light_pos: [f32; 3],
    point_pos: [f32; 3],
    _frame: u32,
) -> f32 {
    let dx = point_pos[0] - light_pos[0];
    let dy = point_pos[1] - light_pos[1];
    let dz = point_pos[2] - light_pos[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();

    if light.max_radius > 0.0 && dist > light.max_radius {
        return 0.0;
    }

    if light.falloff <= 0.001 {
        return 1.0;
    }

    let atten = match light.falloff as u32 {
        0 => 1.0,
        1 => {
            // Linear falloff: 1 - dist/max_dist
            if light.max_radius > 0.0 {
                (1.0 - dist / light.max_radius).max(0.0)
            } else {
                1.0 / (1.0 + dist * 0.001)
            }
        }
        _ => {
            // Inverse-square (physically correct): 1/(1 + dist^2 * k)
            let k = 0.0001; // scale factor
            1.0 / (1.0 + dist * dist * k)
        }
    };

    atten.clamp(0.0, 1.0)
}

/// Compute spot light cone factor. Returns 1.0 for point lights.
pub fn spot_cone_factor(
    light: &Light3D,
    light_pos: [f32; 3],
    point_pos: [f32; 3],
    _frame: u32,
) -> f32 {
    match light.light_type {
        LightType::Spot {
            cone_angle_deg,
            cone_feather_pct,
        } => {
            let dx = point_pos[0] - light_pos[0];
            let dy = point_pos[1] - light_pos[1];
            let dz = point_pos[2] - light_pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < 0.001 {
                return 1.0;
            }
            let dir_z = dz / dist;
            let cone_half = (cone_angle_deg * 0.5).to_radians();
            let cos_cone = cone_half.cos();
            let cos_dir = -dir_z;

            if cos_dir < cos_cone {
                0.0
            } else {
                let feather = (cone_feather_pct / 100.0).clamp(0.0, 1.0);
                let edge = cos_cone + (1.0 - cos_cone) * feather;
                if cos_dir > edge {
                    1.0
                } else {
                    (cos_dir - cos_cone) / (edge - cos_cone)
                }
            }
        }
        _ => 1.0,
    }
}
