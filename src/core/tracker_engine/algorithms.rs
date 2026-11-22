use super::*;

mod matching;
mod stabilization;
pub use matching::*;
pub use stabilization::*;
mod quad;
pub use quad::*;

// ── Four-point perspective tracking (AE Perspective Corner Pin workflow) ──

/// Smooths 2D tracker keyframes using a Gaussian / weighted moving average window.
pub fn smooth_tracker_keyframes(
    kfs: &[crate::core::keyframe::Keyframe<[f32; 2]>],
    window_radius: usize,
) -> Vec<crate::core::keyframe::Keyframe<[f32; 2]>> {
    if kfs.len() <= 2 || window_radius == 0 {
        return kfs.to_vec();
    }

    let n = kfs.len();
    let mut smoothed = Vec::with_capacity(n);

    for i in 0..n {
        let min_idx = i.saturating_sub(window_radius);
        let max_idx = (i + window_radius).min(n - 1);

        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut weight_sum = 0.0f32;

        for (j, keyframe) in kfs.iter().enumerate().take(max_idx + 1).skip(min_idx) {
            let dist = (i as isize - j as isize).abs() as f32;
            let sigma = (window_radius as f32 * 0.5).max(1.0);
            let weight = (-dist * dist / (2.0 * sigma * sigma)).exp();

            sum_x += keyframe.value[0] * weight;
            sum_y += keyframe.value[1] * weight;
            weight_sum += weight;
        }

        let avg_x = if weight_sum > 0.0 {
            sum_x / weight_sum
        } else {
            kfs[i].value[0]
        };
        let avg_y = if weight_sum > 0.0 {
            sum_y / weight_sum
        } else {
            kfs[i].value[1]
        };

        smoothed.push(crate::core::keyframe::Keyframe::new(
            kfs[i].frame,
            [avg_x, avg_y],
            kfs[i].interpolation,
        ));
    }

    smoothed
}

/// Per-frame tracking quality in 0..1: how well the tracked quad preserves the
/// source rectangle's area and convexity. Flipped or collapsed quads (failed
/// SAD matches) score near zero.
pub fn quad_track_confidence(track: &QuadTrackData, source_rect: [[f32; 2]; 4]) -> Vec<f32> {
    fn signed_area(quad: &[[f32; 2]; 4]) -> f32 {
        let mut sum = 0.0;
        for i in 0..4 {
            let a = quad[i];
            let b = quad[(i + 1) % 4];
            sum += a[0] * b[1] - b[0] * a[1];
        }
        sum * 0.5
    }

    let src_area = signed_area(&source_rect).abs().max(1e-6);
    track
        .corners
        .iter()
        .map(|quad| {
            let area = signed_area(quad);
            if area <= 0.0 {
                return 0.0; // flipped / self-intersecting → lost lock
            }
            let ratio = area / src_area;
            let area_score = if ratio >= 1.0 { 1.0 / ratio } else { ratio };
            let e01 = (quad[1][0] - quad[0][0]).hypot(quad[1][1] - quad[0][1]);
            let e32 = (quad[2][0] - quad[3][0]).hypot(quad[2][1] - quad[3][1]);
            let e12 = (quad[2][0] - quad[1][0]).hypot(quad[2][1] - quad[1][1]);
            let e03 = (quad[3][0] - quad[0][0]).hypot(quad[3][1] - quad[0][1]);
            let r1 = if e01.max(e32) > 1e-6 {
                e01.min(e32) / e01.max(e32)
            } else {
                0.0
            };
            let r2 = if e12.max(e03) > 1e-6 {
                e12.min(e03) / e12.max(e03)
            } else {
                0.0
            };
            let shape_score = (r1 + r2) * 0.5;
            (area_score * 0.7 + shape_score * 0.3).clamp(0.0, 1.0)
        })
        .collect()
}

/// Post-analysis cleanup: frames whose lock confidence falls below
/// `conf_threshold` are replaced by their spike-smoothed counterparts while
/// healthy frames stay bit-identical. Combines [`quad_track_confidence`] and
/// [`smooth_quad_track`] into the standard refinement pass.
pub fn refine_quad_track(
    track: &QuadTrackData,
    source_rect: [[f32; 2]; 4],
    conf_threshold: f32,
    max_jump: f32,
) -> QuadTrackData {
    let smoothed = smooth_quad_track(track, max_jump);
    let conf = quad_track_confidence(track, source_rect);
    let mut out = track.clone();
    for i in 0..out.frames.len() {
        if conf.get(i).copied().unwrap_or(1.0) < conf_threshold {
            if let Some(s) = smoothed.corners.get(i) {
                out.corners[i] = *s;
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "algorithms/subpixel_tests.rs"]
mod subpixel_tests;

#[cfg(test)]
#[path = "algorithms/pixel_tracking_tests.rs"]
mod pixel_tracking_tests;

#[cfg(test)]
#[path = "algorithms/quad_track_tests.rs"]
mod quad_track_tests;
