//! Shadow mapping and 3D light helpers for the software renderer.
//!
//! Covers orthographic shadow-map accumulation, light attenuation
//! models, and spot cone factors.

use crate::core::mask::point_in_polygon;
use crate::core::timeline::{Composition, Layer, LayerType, Light3D, LightType, ShapeType};

/// Direction from a surface toward the fixed parallel-light source.
pub(crate) const PARALLEL_LIGHT_DIRECTION: [f32; 3] = [-1.0, -1.0, 1.0];

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

/// Project a point along the renderer's fixed parallel-light ray direction.
/// Surface lighting uses light direction `[-1,-1,1]`, so shadow rays travel
/// toward `[1,1,-1]` and shift one composition unit per unit of caster height.
fn project_parallel_point_to_plane_z0(point: [f32; 3]) -> Option<[f32; 2]> {
    let ray = PARALLEL_LIGHT_DIRECTION.map(|component| -component);
    if point[2] <= 0.0 || ray[2].abs() < f32::EPSILON {
        return None;
    }
    let t = -point[2] / ray[2];
    Some([point[0] + t * ray[0], point[1] + t * ray[1]])
}

/// Fill a convex polygon into the density buffer (scanline-free point test
/// over the bbox — quads are tiny relative to frame, this is plenty fast).
#[allow(clippy::too_many_arguments)]
fn accumulate_polygon_density(
    density: &mut [f32],
    width: u32,
    height: u32,
    pts: &[[f32; 2]],
    light: &Light3D,
    light_pos: [f32; 3],
    frame: u32,
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
        if super::render_cancelled() {
            return;
        }
        for px in min_x..max_x.min(width) {
            if point_in_polygon(px as f32 + 0.5, py as f32 + 0.5, pts) {
                let idx = (py * width + px) as usize;
                if idx < density.len() {
                    let receiver = [px as f32 + 0.5, py as f32 + 0.5, 0.0];
                    let (attenuation, cone) = if matches!(&light.light_type, LightType::Parallel) {
                        (1.0, 1.0)
                    } else {
                        (
                            light_attenuation(light, light_pos, receiver, frame),
                            spot_cone_factor(light, light_pos, receiver, frame),
                        )
                    };
                    density[idx] = (density[idx] + amount * attenuation * cone).min(1.0);
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
        if super::render_cancelled() {
            return;
        }
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
        if super::render_cancelled() {
            return;
        }
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
                let Some(mut points) = super::try_vec_with_capacity(24) else {
                    return Vec::new();
                };
                for i in 0..24 {
                    let a = i as f32 / 24.0 * std::f32::consts::TAU;
                    points.push([w * a.cos(), h * a.sin()]);
                }
                Some(points)
            }
            ShapeType::Polygon { sides, radius } => {
                let n = sides.evaluate(frame).round().max(3.0) as usize;
                let r = radius.evaluate(frame).max(1.0);
                let Some(mut points) = super::try_vec_with_capacity(n) else {
                    return Vec::new();
                };
                for i in 0..n {
                    let a =
                        i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                    points.push([r * a.cos(), r * a.sin()]);
                }
                Some(points)
            }
            ShapeType::Star {
                points,
                inner_radius,
                outer_radius,
            } => {
                let n = points.evaluate(frame).round().max(3.0) as usize;
                let ri = inner_radius.evaluate(frame).max(1.0);
                let ro = outer_radius.evaluate(frame).max(ri);
                let Some(point_count) = n.checked_mul(2) else {
                    super::mark_render_allocation_failed();
                    return Vec::new();
                };
                let Some(mut outline) = super::try_vec_with_capacity(point_count) else {
                    return Vec::new();
                };
                for i in 0..point_count {
                    let a = i as f32 / point_count as f32 * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { ro } else { ri };
                    outline.push([r * a.cos(), r * a.sin()]);
                }
                Some(outline)
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

/// Transform a centered layer-space caster point with the same Z*Y*X
/// convention used by `Composition::resolve_world_transform_3d`.
fn transform_3d_caster_point(layer: &Layer, frame: u32, point: [f32; 2]) -> [f32; 3] {
    let position = layer.transform_3d.position.evaluate(frame);
    let rotation = layer.transform_3d.rotation.evaluate(frame);
    let scale = layer.transform_3d.scale.evaluate(frame);
    let [rx, ry, rz] = rotation.map(f32::to_radians);
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let [x, y] = [point[0] * scale[0] / 100.0, point[1] * scale[1] / 100.0];
    [
        position[0] + cy * cz * x + (sx * sy * cz - cx * sz) * y,
        position[1] + cy * sz * x + (sx * sy * sz + cx * cz) * y,
        position[2] - sy * x + sx * cy * y,
    ]
}

/// Build the shadow density map (0=lit, 1=fully shadowed) at comp resolution:
/// every shadow-casting light projects every casting layer's world quad onto
/// the z=0 plane; densities accumulate and are softened by a small blur.
pub fn build_shadow_map(comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<f32> {
    build_shadow_map_scaled(
        comp,
        frame,
        width,
        height,
        super::resolution::RenderScale::FULL,
    )
}

pub(super) fn build_shadow_map_scaled(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    render_scale: super::resolution::RenderScale,
) -> Vec<f32> {
    let n = (width.max(1) as usize) * (height.max(1) as usize);
    let mut density = vec![0.0f32; n];

    for light in &comp.lights {
        if super::render_cancelled() {
            return density;
        }
        let intensity = light.intensity_at(frame);
        if !light.casts_shadows || intensity <= 0.0 {
            continue;
        }
        let is_parallel = matches!(&light.light_type, LightType::Parallel);
        let mut lpos = comp.light_position_at(light, frame);
        lpos[0] *= render_scale.x;
        lpos[1] *= render_scale.y;
        let strength = ((light.shadow_darkness / 100.0) * (intensity / 100.0)).clamp(0.0, 1.0);
        if strength <= 0.003 {
            continue;
        }
        for layer in &comp.layers {
            if super::render_cancelled() {
                return density;
            }
            if !layer.is_active(frame) || !layer.visible || !layer.material.cast_shadows {
                continue;
            }
            let transmission = (layer.material.light_transmission / 100.0).clamp(0.0, 1.0);
            let caster_strength = strength * (1.0 - transmission);
            if caster_strength <= 0.003 {
                continue;
            }
            // Time remapping changes source pixels, not the 3D transform that
            // casts the shadow. Use the same 3D position, scale, and Euler
            // rotation as the rendered plane; its independent 2D transform
            // must not move or resize the shadow.
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
            let Some(mut caster_points) = super::try_vec_with_capacity(local_pts.len()) else {
                continue;
            };
            caster_points.extend(
                local_pts
                    .iter()
                    .map(|point| transform_3d_caster_point(layer, frame, *point)),
            );
            // Coplanar with the receiver plane: its "shadow" lands exactly on
            // itself and would blanket the frame — skip.
            if caster_points.iter().all(|point| point[2].abs() < 1.0) {
                continue;
            }
            let Some(mut projected) = super::try_vec_with_capacity(caster_points.len()) else {
                continue;
            };
            for point in caster_points {
                let projection = if is_parallel {
                    project_parallel_point_to_plane_z0(point)
                } else {
                    project_point_to_plane_z0(lpos, point)
                };
                match projection {
                    Some(mut p) => {
                        p[0] *= render_scale.x;
                        p[1] *= render_scale.y;
                        projected.push(p);
                    }
                    None => {
                        projected.clear();
                        break;
                    }
                }
            }
            if projected.len() == local_pts.len() {
                accumulate_polygon_density(
                    &mut density,
                    width,
                    height,
                    &projected,
                    light,
                    lpos,
                    frame,
                    caster_strength,
                );
            }
        }
    }

    // Soft penumbra
    if super::render_cancelled() {
        return density;
    }
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
