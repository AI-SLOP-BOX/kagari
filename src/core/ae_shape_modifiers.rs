//! After Effects Shape Layer Modifiers:
//! - **Pucker & Bloat**: pinches corners inward / bloats midpoints outward.
//! - **Twist**: swirls vertices around a center by angle proportional to distance.
//! - **Wiggle Transform**: deterministic per-copy/temporal wiggle for Position, Scale, and Rotation.

/// Applies AE-style Pucker & Bloat to a polygon/polyline (`amount_pct` in `-100.0..=100.0`).
///
/// Positive values bloat segment midpoints outward and pull corners inward;
/// negative values pucker midpoints inward and spike corners outward.
pub fn apply_pucker_bloat(points: &[[f32; 2]], amount_pct: f32, closed: bool) -> Vec<[f32; 2]> {
    if points.len() < 2 || amount_pct.abs() < 1e-4 {
        return points.to_vec();
    }
    let amt = (amount_pct / 100.0).clamp(-1.5, 1.5);
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    for p in points {
        cx += p[0];
        cy += p[1];
    }
    cx /= points.len() as f32;
    cy /= points.len() as f32;

    let seg_count = if closed { points.len() } else { points.len() - 1 };
    let mut out = Vec::with_capacity(seg_count * 2 + 1);

    for i in 0..seg_count {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let corner = [
            a[0] + (cx - a[0]) * (amt * 0.35),
            a[1] + (cy - a[1]) * (amt * 0.35),
        ];
        let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        let bloated_mid = [
            mid[0] + (mid[0] - cx) * (amt * 0.65),
            mid[1] + (mid[1] - cy) * (amt * 0.65),
        ];
        out.push(corner);
        out.push(bloated_mid);
    }
    if !closed {
        let last = points[points.len() - 1];
        out.push([
            last[0] + (cx - last[0]) * (amt * 0.35),
            last[1] + (cy - last[1]) * (amt * 0.35),
        ]);
    }
    out
}

/// Applies AE-style Twist modifier around `center`, rotating vertices by `angle_deg`
/// scaled by `distance / radius.max(1.0)`.
pub fn apply_twist(
    points: &[[f32; 2]],
    angle_deg: f32,
    center: [f32; 2],
    radius: f32,
) -> Vec<[f32; 2]> {
    if points.is_empty() || angle_deg.abs() < 1e-4 {
        return points.to_vec();
    }
    let r_ref = radius.max(1.0);
    let max_rad = angle_deg.to_radians();
    points
        .iter()
        .map(|p| {
            let dx = p[0] - center[0];
            let dy = p[1] - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let theta = max_rad * (dist / r_ref);
            let (s, c) = theta.sin_cos();
            [center[0] + dx * c - dy * s, center[1] + dx * s + dy * c]
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WiggleTransformParams {
    pub wiggles_per_second: f32,
    pub correlation: f32, // 0.0 = independent per copy, 1.0 = locked together
    pub position_amp: [f32; 2],
    pub scale_amp: [f32; 2],
    pub rotation_amp_deg: f32,
    pub seed: u64,
}

impl Default for WiggleTransformParams {
    fn default() -> Self {
        Self {
            wiggles_per_second: 2.0,
            correlation: 0.5,
            position_amp: [15.0, 15.0],
            scale_amp: [0.15, 0.15],
            rotation_amp_deg: 12.0,
            seed: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WiggleTransformDelta {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation_deg: f32,
}

#[inline]
fn smooth_noise1(t: f32, seed: u64) -> f32 {
    let i0 = t.floor() as i64;
    let f = t - t.floor();
    let u = f * f * (3.0 - 2.0 * f);
    let h = |k: i64| -> f32 {
        let mut x = (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ seed;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        ((x >> 40) as f32 / ((1u64 << 24) as f32)) * 2.0 - 1.0
    };
    h(i0) * (1.0 - u) + h(i0 + 1) * u
}

/// Evaluates AE-style Wiggle Transform for shape copy `copy_index` at `time_sec`.
pub fn evaluate_wiggle_transform(
    copy_index: u32,
    time_sec: f32,
    params: &WiggleTransformParams,
) -> WiggleTransformDelta {
    let t = time_sec * params.wiggles_per_second.max(0.0);
    let corr = params.correlation.clamp(0.0, 1.0);
    let base_seed = params.seed.wrapping_add(17);
    let copy_seed = params
        .seed
        .wrapping_add((copy_index as u64 + 1).wrapping_mul(0x517C_C1B7_2722_0A95));

    let sample = |ch: u64| -> f32 {
        let g = smooth_noise1(t, base_seed.wrapping_add(ch * 101));
        let l = smooth_noise1(t, copy_seed.wrapping_add(ch * 101));
        g * corr + l * (1.0 - corr)
    };

    WiggleTransformDelta {
        position: [
            sample(0) * params.position_amp[0],
            sample(1) * params.position_amp[1],
        ],
        scale: [
            1.0 + sample(2) * params.scale_amp[0],
            1.0 + sample(3) * params.scale_amp[1],
        ],
        rotation_deg: sample(4) * params.rotation_amp_deg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pucker_bloat_subdivides_and_bloats_midpoints() {
        let square = [[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0]];
        let bloated = apply_pucker_bloat(&square, 50.0, true);
        assert_eq!(bloated.len(), 8);
        // Bloated top midpoint (index 1) has y < -10.0
        assert!(bloated[1][1] < -12.0);
    }

    #[test]
    fn twist_rotates_outer_points_more_than_center() {
        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let twisted = apply_twist(&pts, 90.0, [0.0, 0.0], 100.0);
        assert!(twisted[0][0].abs() < 1e-4 && twisted[0][1].abs() < 1e-4);
        assert!(twisted[1][0].abs() < 1e-4 && (twisted[1][1] - 100.0).abs() < 1e-4);
    }

    #[test]
    fn wiggle_transform_correlation_controls_per_copy_divergence() {
        let locked = WiggleTransformParams {
            correlation: 1.0,
            ..Default::default()
        };
        let a = evaluate_wiggle_transform(0, 1.25, &locked);
        let b = evaluate_wiggle_transform(5, 1.25, &locked);
        assert_eq!(a, b);

        let ind = WiggleTransformParams {
            correlation: 0.0,
            ..Default::default()
        };
        let c = evaluate_wiggle_transform(0, 1.25, &ind);
        let d = evaluate_wiggle_transform(5, 1.25, &ind);
        assert_ne!(c, d);
    }
}
