//! Mocha Pro-style Temporal Object Removal & Cleanplate Synthesis.
//!
//! Given a target `RGBA32F` frame, a `[0..1]` removal mask (`>0.5` = object to erase),
//! and a list of aligned/stabilized neighbor frames `(rgba, neighbor_mask)`,
//! synthesizes a clean background plate using per-pixel temporal median over
//! unmasked samples, falling back to spatial neighbor diffusion where occluded
//! across all frames, with smooth boundary feathering.

pub fn remove_object_temporal_f32(
    target_rgba: &[f32],
    target_mask: &[f32],
    neighbors: &[(&[f32], &[f32])],
    width: u32,
    height: u32,
) -> Option<Vec<f32>> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || target_rgba.len() != count * 4 || target_mask.len() != count {
        return None;
    }
    for (n_rgba, n_mask) in neighbors {
        if n_rgba.len() != count * 4 || n_mask.len() != count {
            return None;
        }
    }

    let mut out = target_rgba.to_vec();
    let mut unresolved = vec![false; count];

    for idx in 0..count {
        let m = target_mask[idx].clamp(0.0, 1.0);
        if m <= 1e-4 {
            continue;
        }
        let mut rs = Vec::with_capacity(neighbors.len());
        let mut gs = Vec::with_capacity(neighbors.len());
        let mut bs = Vec::with_capacity(neighbors.len());
        for (n_rgba, n_mask) in neighbors {
            if n_mask[idx] < 0.25 {
                let c = idx * 4;
                rs.push(n_rgba[c]);
                gs.push(n_rgba[c + 1]);
                bs.push(n_rgba[c + 2]);
            }
        }

        if !rs.is_empty() {
            rs.sort_by(|a, b| a.total_cmp(b));
            gs.sort_by(|a, b| a.total_cmp(b));
            bs.sort_by(|a, b| a.total_cmp(b));
            let mid = rs.len() / 2;
            let c = idx * 4;
            out[c] = target_rgba[c] * (1.0 - m) + rs[mid] * m;
            out[c + 1] = target_rgba[c + 1] * (1.0 - m) + gs[mid] * m;
            out[c + 2] = target_rgba[c + 2] * (1.0 - m) + bs[mid] * m;
        } else {
            unresolved[idx] = true;
        }
    }

    // Spatial fallback diffusion for pixels occluded in every neighbor frame
    if unresolved.iter().any(|&u| u) {
        let w = width as i32;
        let h = height as i32;
        let mut next = out.clone();
        for _ in 0..8 {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    if !unresolved[idx] {
                        continue;
                    }
                    let mut acc = [0.0_f32; 3];
                    let mut cnt = 0.0_f32;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < w && ny >= 0 && ny < h {
                            let n_idx = (ny * w + nx) as usize;
                            if !unresolved[n_idx] || target_mask[n_idx] < 0.25 {
                                let c = n_idx * 4;
                                acc[0] += out[c];
                                acc[1] += out[c + 1];
                                acc[2] += out[c + 2];
                                cnt += 1.0;
                            }
                        }
                    }
                    if cnt > 0.0 {
                        let c = idx * 4;
                        next[c] = acc[0] / cnt;
                        next[c + 1] = acc[1] / cnt;
                        next[c + 2] = acc[2] / cnt;
                        unresolved[idx] = false;
                    }
                }
            }
            out.copy_from_slice(&next);
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_cleanplate_removes_moving_foreground_wire_using_neighbor_median() {
        // 3x1 pixels: true background is [0.2, 0.4, 0.8]; target has white wire at pixel 1
        let target = vec![
            0.2, 0.4, 0.8, 1.0,
            1.0, 1.0, 1.0, 1.0,
            0.2, 0.4, 0.8, 1.0,
        ];
        let t_mask = vec![0.0, 1.0, 0.0];

        // Neighbor 1 has wire at pixel 0 (pixel 1 is clean)
        let n1 = vec![
            1.0, 1.0, 1.0, 1.0,
            0.2, 0.4, 0.8, 1.0,
            0.2, 0.4, 0.8, 1.0,
        ];
        let n1_mask = vec![1.0, 0.0, 0.0];

        // Neighbor 2 has wire at pixel 2 (pixel 1 is clean)
        let n2 = vec![
            0.2, 0.4, 0.8, 1.0,
            0.2, 0.4, 0.8, 1.0,
            1.0, 1.0, 1.0, 1.0,
        ];
        let n2_mask = vec![0.0, 0.0, 1.0];

        let clean = remove_object_temporal_f32(&target, &t_mask, &[(&n1, &n1_mask), (&n2, &n2_mask)], 3, 1)
            .expect("cleanplate");

        assert!((clean[4] - 0.2).abs() < 1e-5);
        assert!((clean[5] - 0.4).abs() < 1e-5);
        assert!((clean[6] - 0.8).abs() < 1e-5);
    }
}
