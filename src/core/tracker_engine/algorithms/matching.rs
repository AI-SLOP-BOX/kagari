#[cfg(test)]
use super::*;

/// Compute real Sum of Absolute Differences (SAD) template matching between reference RGBA buffer and target RGBA buffer.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn compute_sad_match(
    ref_patch: &[u8],
    patch_w: usize,
    patch_h: usize,
    target_img: &[u8],
    img_w: usize,
    img_h: usize,
    search_center_x: i32,
    search_center_y: i32,
    search_radius: i32,
) -> [f32; 2] {
    let mut min_sad = f32::MAX;
    let mut best_x = search_center_x as f32;
    let mut best_y = search_center_y as f32;

    let half_pw = (patch_w / 2) as i32;
    let half_ph = (patch_h / 2) as i32;

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let cx = search_center_x + dx;
            let cy = search_center_y + dy;

            if cx - half_pw < 0
                || cx + half_pw >= img_w as i32
                || cy - half_ph < 0
                || cy + half_ph >= img_h as i32
            {
                continue;
            }

            let mut sad = 0.0f32;
            for py in 0..patch_h {
                for px in 0..patch_w {
                    let rx = px;
                    let ry = py;
                    let ref_idx = (ry * patch_w + rx) * 4;

                    let tx = (cx - half_pw) as usize + px;
                    let ty = (cy - half_ph) as usize + py;
                    let tgt_idx = (ty * img_w + tx) * 4;

                    if ref_idx + 3 < ref_patch.len() && tgt_idx + 3 < target_img.len() {
                        let diff_r = (ref_patch[ref_idx] as f32 - target_img[tgt_idx] as f32).abs();
                        let diff_g =
                            (ref_patch[ref_idx + 1] as f32 - target_img[tgt_idx + 1] as f32).abs();
                        let diff_b =
                            (ref_patch[ref_idx + 2] as f32 - target_img[tgt_idx + 2] as f32).abs();
                        sad += diff_r + diff_g + diff_b;
                    }
                }
            }

            // Tie-break toward the search center: flat regions (uniform areas)
            // have many equal-SAD offsets, and without this the first-scanned
            // corner always won, causing spurious drift.
            let is_better = sad < min_sad
                || ((sad - min_sad).abs() <= 1e-3
                    && (dx.abs() + dy.abs())
                        < (best_x as i32 - search_center_x).abs()
                            + (best_y as i32 - search_center_y).abs());
            if is_better {
                min_sad = sad;
                best_x = cx as f32;
                best_y = cy as f32;
            }
        }
    }

    [best_x, best_y]
}

/// Subpixel refinement: parabola fit through the SAD cost at (x-1, x, x+1)
/// along each axis. Returns the refined offset in [-0.5, 0.5] per axis.
/// Ported concept from NextVFX's dense optical flow post-processing.
pub fn subpixel_refine(sad_at: &dyn Fn(i32, i32) -> f32, best_x: i32, best_y: i32) -> [f32; 2] {
    let fit = |minus: f32, center: f32, plus: f32| -> f32 {
        let denom = (minus - 2.0 * center + plus).abs();
        if denom < 1e-9 {
            return 0.0;
        }
        // Vertex of the parabola through the three samples
        (0.5 * (minus - plus) / denom).clamp(-0.5, 0.5)
    };
    let dx = fit(
        sad_at(best_x - 1, best_y),
        sad_at(best_x, best_y),
        sad_at(best_x + 1, best_y),
    );
    let dy = fit(
        sad_at(best_x, best_y - 1),
        sad_at(best_x, best_y),
        sad_at(best_x, best_y + 1),
    );
    [dx, dy]
}

/// Match confidence from the two lowest SAD values: ratio near 0 means a
/// distinctive peak (good), near 1 means a flat valley (ambiguous match).
pub fn match_confidence(min_sad: f32, second_min_sad: f32) -> f32 {
    if second_min_sad <= 1e-9 {
        if min_sad <= 1e-9 {
            return 1.0;
        } else {
            return 0.0;
        }
    }
    (min_sad / second_min_sad).clamp(0.0, 1.0)
}

/// Extracts a `feature`×`feature` RGBA patch centred at integer (cx,cy) —
/// used to seed a tracker's persistent reference pattern
/// ([`crate::core::timeline::TrackerPoint::reference_pattern`]).
/// Out-of-bounds centres yield None.
pub fn extract_patch(
    frame: &[u8],
    img_w: u32,
    img_h: u32,
    cx: i32,
    cy: i32,
    feature: usize,
) -> Option<Vec<u8>> {
    let half = (feature / 2) as i32;
    if feature == 0 || half > cx || half > cy {
        return None;
    }
    let max_x = cx + (feature as i32 - 1 - half);
    let max_y = cy + (feature as i32 - 1 - half);
    if max_x >= img_w as i32
        || max_y >= img_h as i32
        || frame.len() < img_w as usize * img_h as usize * 4
    {
        return None;
    }
    let mut patch = vec![0u8; feature * feature * 4];
    for py in 0..feature {
        for px in 0..feature {
            let sx = cx - half + px as i32;
            let sy = cy - half + py as i32;
            let src = ((sy * img_w as i32 + sx) * 4) as usize;
            let dst = (py * feature + px) * 4;
            patch[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
        }
    }
    Some(patch)
}

/// Blends the current appearance into a persistent template by `blend`
/// (0 = frozen, 1 = fully adopt current). Mismatched lengths are ignored.
/// Slow blends (e.g. 0.05/frame) track gradual appearance drift while
/// resisting one-frame occlusions.
pub fn blend_template(template: &mut [u8], current: &[u8], blend: f32) {
    let b = blend.clamp(0.0, 1.0);
    if b <= 0.0 || template.len() != current.len() {
        return;
    }
    for (t, &c) in template.iter_mut().zip(current.iter()) {
        *t = (*t as f32 * (1.0 - b) + c as f32 * b)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}
#[cfg(test)]
include!("matching/tests.rs");
